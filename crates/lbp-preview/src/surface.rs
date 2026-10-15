//! P2q opt-in surface extras beside the diffuse route: bump (output port 3) and gloss
//! (output port 2) for the LBP-shaped preview lighting. Evidence and limits:
//! docs/p2q-bump-gloss.md. Only two shapes are taken: port 3 wired straight from one
//! static texture sampler, and port 2 wired straight from one static COLOR. Anything
//! else stays unrendered and is named in `skipped`.
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};
use lbp_resource::material::{GraphInspection, GraphMetadata};

/// Output-node port numbers (CONFIRMED_FROM_COMMUNITY_CODE, docs/material-graph.md).
const SPECULAR_PORT: u8 = 2;
const BUMP_PORT: u8 = 3;
/// P3h: glow (INFERRED from the decoded shaders: added unlit, oracles/results/2026-10-09-shading-fpdec.md).
pub const GLOW_PORT: u8 = 4;
/// P3i: reflection (CONFIRMED port 6 by d104-uv-channel E5; math from the fpdec oracles
/// 2026-10-10-node-types-10-12.md and 2026-10-10-reflection-uv.md).
pub const REFLECTION_PORT: u8 = 6;
/// Most reflection textures and steps the viewer binds (graph slots 0..=3, rows 0..=30).
pub const MAX_REFLECTION_TEXTURES: usize = 4;
pub const MAX_REFLECTION_STEPS: usize = 31;
/// Upper bounds for untrusted material scalars before they reach a shader.
const MAX_BUMP_LEVEL: f32 = 16.;
const MAX_COSINE_POWER: f32 = 4096.;
const MAX_REFLECTION_BLUR: f32 = 16.;
/// The shader constant is a fixed transform of the stored GfxMaterial scalar (PC fpdec/CGB
/// patch lists, 644 gmats, 0 exceptions; oracles 2026-10-10-glass-scalars): BumpLevel = 2 x
/// stored, CosinePower = 22 x stored (reflection blur = stored - 1, see P3i).
const BUMP_LEVEL_SCALE: f32 = 2.;
const COSINE_POWER_SCALE: f32 = 22.;

/// P3i: a reflection graph sampled at the reflection vector instead of a UV: a 2D sampler
/// reads R.xy, a cube sampler (P3l, one per graph) reads R.xyz, with LOD bias `blur`
/// (GfxMaterial ReflectionBlur).
pub struct Reflection {
    pub textures: Vec<ReflectionMap>,
    /// Encoded D100 steps (root last); sample steps use identity scale/offset, channel 0.
    pub steps: Vec<crate::FragmentStep>,
    pub blur: f32,
}
/// P3l: one decoded reflection sampler. A cube map keeps its six top-mip faces (BC1/BC3 GTF,
/// face order +X, -X, +Y, -Y, +Z, -Z) and sample steps on it carry channel `CUBE_CHANNEL`.
pub enum ReflectionMap {
    Flat(lbp_texture::Texture, crate::TextureWrapping),
    Cube(Box<lbp_texture::LayeredTexture>),
}
impl ReflectionMap {
    fn rgba_bytes(&self) -> usize {
        match self {
            Self::Flat(t, _) => t.rgba.len(),
            Self::Cube(c) => {
                c.texture.rgba.len() + c.extra_faces.iter().map(Vec::len).sum::<usize>()
            }
        }
    }
}
/// P3l: the UV-channel field of a reflection sample step that reads a cube map.
pub const CUBE_CHANNEL: u32 = 2;
/// Sampler slots in texture order and the encoded steps.
pub(crate) type ReflectionProgram = (Vec<usize>, Vec<crate::FragmentStep>);
pub struct BumpMap {
    pub texture: lbp_texture::Texture,
    pub texture_key: lbp_catalog::attachment::LocationKey,
    /// Sampler coordinates per vertex (raw channel through the sampler's transform).
    pub sampling_uv: Vec<[f32; 2]>,
    pub wrap: crate::TextureWrapping,
    pub uv_channel: usize,
    /// D286: the sampler's transform from the raw channel to `sampling_uv`.
    pub uv_transform: crate::SamplingTransform,
}
#[derive(Default)]
pub struct Surface {
    pub bump: Option<Box<BumpMap>>,
    /// GfxMaterial BumpLevel (finite, clamped to 0..=16).
    pub bump_level: f32,
    /// Static COLOR on the specular port (linear rgb), when wired that way.
    pub specular: Option<[f32; 3]>,
    /// P3d: gloss colour map on port 2 (same layout as `BumpMap`; rgb is the gloss colour).
    pub spec_map: Option<Box<BumpMap>>,
    /// GfxMaterial CosinePower (finite, clamped to 1..=4096).
    pub cosine_power: f32,
    pub alpha_mode_raw: u8,
    /// GfxMaterial AlphaTestLevel (finite, 0..=1; 0 = no cut-out). The decoded shaders write
    /// alpha 0 below it (oracles/results/2026-10-09-shading-fpdec.md); the viewer discards.
    pub alpha_test: f32,
    /// P3i: what output port 6 evaluates to when it is a graph of 2D samplers (port 6 sampled
    /// at the reflection vector, added to the diffuse times the luminance of the light).
    pub reflection: Option<Box<Reflection>>,
    /// P3h: what output port 4 (glow) evaluates to, loaded by the diffuse route with the glow
    /// graph standing in for port 0 (a solid colour or a baked texture), when wired.
    pub glow: Option<Box<crate::MaterialAsset>>,
    /// D309: how the material's own program writes the glow alpha under fog
    /// (`lbp_resource::material::glow_alpha_form`; `Unknown` when the program is not read).
    pub glow_alpha: lbp_resource::material::GlowAlphaForm,
    /// Wired surface ports this preview does not draw, with the reason.
    pub skipped: Vec<String>,
}
impl Surface {
    pub fn retained_bytes(&self) -> usize {
        [&self.bump, &self.spec_map]
            .into_iter()
            .flatten()
            .map(|b| b.texture.rgba.len() + b.sampling_uv.len() * std::mem::size_of::<[f32; 2]>())
            .sum::<usize>()
            + self.reflection.as_deref().map_or(0, |r| {
                r.textures
                    .iter()
                    .map(ReflectionMap::rgba_bytes)
                    .sum::<usize>()
            })
            + match self.glow.as_deref() {
                Some(crate::MaterialAsset::Texture(a)) => {
                    a.texture.rgba.len() + a.sampling_uv.len() * std::mem::size_of::<[f32; 2]>()
                }
                _ => 0,
            }
    }
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if let Some(b) = &self.bump {
            parts.push(format!(
                "bump UV{} × {:.2} ({}×{})",
                b.uv_channel, self.bump_level, b.texture.header.width, b.texture.header.height
            ));
        }
        if let Some(m) = &self.spec_map {
            parts.push(format!(
                "gloss map UV{} ^ {:.0} ({}×{})",
                m.uv_channel, self.cosine_power, m.texture.header.width, m.texture.header.height
            ));
        }
        if let Some(r) = &self.reflection {
            parts.push(format!(
                "reflection {} map{} blur {:.2}",
                r.textures.len(),
                if r.textures.len() == 1 { "" } else { "s" },
                r.blur
            ));
        }
        match self.glow.as_deref() {
            Some(crate::MaterialAsset::Solid(a)) => {
                let [r, g, b, _] = a.selection.color.values();
                parts.push(format!("glow {r:.2}/{g:.2}/{b:.2}"));
            }
            Some(crate::MaterialAsset::Texture(a)) => parts.push(format!(
                "glow map ({}×{})",
                a.texture.header.width, a.texture.header.height
            )),
            None => {}
        }
        if let Some(s) = self.specular {
            parts.push(format!(
                "gloss {:.2}/{:.2}/{:.2} ^ {:.0}",
                s[0], s[1], s[2], self.cosine_power
            ));
        }
        parts.extend(self.skipped.iter().cloned());
        parts.push(format!("alpha mode {}", self.alpha_mode_raw));
        if self.alpha_test > 0. {
            parts.push(format!("alpha test {:.2}", self.alpha_test));
        }
        format!("P2q surface: {}", parts.join(" • "))
    }
}

/// Where the static ports of the single output node come from (no I/O).
#[derive(Debug, Default, PartialEq)]
pub struct SurfaceRoutes {
    pub bump: Option<crate::linked::SamplerSelection>,
    pub specular: Option<[f32; 3]>,
    /// P3d: port 2 wired straight from one sampler (a gloss colour map).
    pub spec_map: Option<crate::linked::SamplerSelection>,
    pub skipped: Vec<String>,
}
/// P3h: a copy of `graph` whose output node reads `port` as its port 0, so the diffuse route
/// can evaluate the glow graph. `Ok(None)` when `port` is unwired; one wire only.
pub fn rewire_output(graph: &GraphMetadata, port: u8) -> Result<Option<GraphMetadata>, String> {
    let outputs: Vec<usize> = (0..graph.boxes.len())
        .filter(|&i| graph.boxes[i].type_raw == 0)
        .collect();
    let [output] = outputs[..] else {
        return Err("Unsupported: surface needs one output node".into());
    };
    let into = |p: u8| {
        graph
            .wires
            .iter()
            .enumerate()
            .filter(move |(_, w)| w.box_to == output as i32 && w.port_to_raw == p)
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    };
    let wired = into(port);
    let [wire] = wired[..] else {
        return if wired.is_empty() {
            Ok(None)
        } else {
            Err(format!("several port {port} wires"))
        };
    };
    let mut derived = graph.clone();
    let diffuse = into(0);
    derived.wires[wire].port_to_raw = 0;
    let mut index = 0;
    derived.wires.retain(|_| {
        let keep = !diffuse.contains(&index);
        index += 1;
        keep
    });
    derived.wire_count = derived.wires.len();
    Ok(Some(derived))
}
/// P3h: whether the compiled program has the glow add. A colour box (type 3) feeding port 4 is
/// bound to the fragment program as constant `ta<k>` (k = its box index, CONFIRMED over 8,043
/// gmats, oracles 2026-10-10-glow-gate); programs that never name it (e.g. the frozen cube
/// 00005407) have no glow add. Raw scan of the shader blob for `\0ta<k>\0` (never the bare
/// prefix: `ta1` is inside `ta10`); named in any variant counts. Texture, mix and other source
/// kinds are not decided here and pass.
pub fn glow_in_program(graph: &GraphMetadata, blob: &[u8]) -> bool {
    let Some(output) = graph.boxes.iter().position(|b| b.type_raw == 0) else {
        return true;
    };
    let source = graph
        .wires
        .iter()
        .find(|w| w.box_to == output as i32 && w.port_to_raw == GLOW_PORT)
        .map(|w| w.box_from);
    let Some(k) = source.and_then(|f| usize::try_from(f).ok()) else {
        return true;
    };
    if graph.boxes.get(k).map(|b| b.type_raw) != Some(3) {
        return true;
    }
    let name = format!("\0ta{k}\0").into_bytes();
    blob.windows(name.len()).any(|w| w == name.as_slice())
}
/// D307: whether the compiled program cuts out by AlphaTestLevel. The 885 programs that name
/// the constant write alpha 0 below it (P3a's reading); the others never read the colour's
/// alpha and write their own (1, or a glow's `1 / (1 + 4 |g|^2)`, which the game draws: the
/// checkpoint ring's `neon_lifebar`, drawn at a colour alpha of 0.5, stores AlphaTestLevel
/// 0.5 and names no such constant). So the stored scalar cuts out only where the program
/// names it. Raw scan for `\0AlphaTestLevel\0` as [`glow_in_program`]; an empty blob (not
/// read) keeps the stored scalar.
pub fn alpha_test_in_program(blob: &[u8]) -> bool {
    const NAME: &[u8] = b"\0AlphaTestLevel\0";
    blob.is_empty() || blob.windows(NAME.len()).any(|w| w == NAME)
}
fn is_static(node: &lbp_resource::material::BoxMetadata) -> bool {
    node.subtype_raw == 0
        && node
            .animations
            .iter()
            .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|v| v == b'0'))
}
pub fn select_routes(graph: &GraphMetadata) -> Result<SurfaceRoutes, String> {
    let boxes = &graph.boxes;
    let wires = &graph.wires;
    if boxes.len() > 1024 || wires.len() > 4096 {
        return Err("Unsupported: surface graph budget".into());
    }
    let outputs: Vec<usize> = (0..boxes.len())
        .filter(|&i| boxes[i].type_raw == 0)
        .collect();
    let [output] = outputs[..] else {
        return Err("Unsupported: surface needs one output node".into());
    };
    let mut routes = SurfaceRoutes::default();
    let port = |p: u8| {
        wires
            .iter()
            .filter(|w| w.box_to == output as i32 && w.port_to_raw == p)
            .collect::<Vec<_>>()
    };
    let endpoint = |w: &lbp_resource::material::WireMetadata| {
        usize::try_from(w.box_from)
            .ok()
            .filter(|&i| i < boxes.len() && w.port_from_raw == 0 && w.swizzle_raw == [0; 5])
    };
    match port(BUMP_PORT)[..] {
        [] => {}
        [w] => match endpoint(w).filter(|&i| boxes[i].type_raw == 1) {
            Some(i) => match crate::linked::sampler(graph, i) {
                Ok(s) => routes.bump = Some(s),
                Err(e) => routes.skipped.push(format!("bump not drawn ({e})")),
            },
            None => routes
                .skipped
                .push("bump not drawn (port 3 is not one unswizzled sampler)".into()),
        },
        _ => routes
            .skipped
            .push("bump not drawn (several port 3 wires)".into()),
    }
    match port(SPECULAR_PORT)[..] {
        [] => {}
        [w] => match endpoint(w).filter(|&i| {
            boxes[i].type_raw == 3
                && is_static(&boxes[i])
                && !wires.iter().any(|w| w.box_to == i as i32)
        }) {
            Some(i) => {
                let tint = crate::DiffuseTint {
                    bits: boxes[i].parameters_raw[..4].try_into().unwrap(),
                };
                tint.validate()?;
                let [r, g, b, _] = tint.values();
                routes.specular = Some([r, g, b]);
            }
            None => match endpoint(w).filter(|&i| boxes[i].type_raw == 1) {
                Some(i) => match crate::linked::sampler(graph, i) {
                    Ok(s) => routes.spec_map = Some(s),
                    Err(e) => routes.skipped.push(format!("gloss map not drawn ({e})")),
                },
                None => routes
                    .skipped
                    .push("gloss not drawn (port 2 is not one static COLOR or one sampler)".into()),
            },
        },
        _ => routes
            .skipped
            .push("gloss not drawn (several port 2 wires)".into()),
    }
    Ok(routes)
}
/// Shader ReflectionBlur = stored - 1 (PC CGB patch lists, 124 gmats, 0 exceptions; stored
/// 1.45 -> 0.45), clamped to 0..=MAX_REFLECTION_BLUR; 0 when unusable.
fn reflection_blur(bits: u32) -> f32 {
    (scalar(bits, 1., MAX_REFLECTION_BLUR + 1., 1.) - 1.).max(0.)
}
/// Shader BumpLevel from the stored scalar (finite, 0..=MAX_BUMP_LEVEL; 0 when unusable).
fn bump_level(bits: u32) -> f32 {
    scalar(bits, 0., MAX_BUMP_LEVEL / BUMP_LEVEL_SCALE, 0.) * BUMP_LEVEL_SCALE
}
/// Shader CosinePower from the stored scalar (1..=MAX_COSINE_POWER; 22 when unusable).
fn cosine_power(bits: u32) -> f32 {
    scalar(
        bits,
        1. / COSINE_POWER_SCALE,
        MAX_COSINE_POWER / COSINE_POWER_SCALE,
        1.,
    ) * COSINE_POWER_SCALE
}
fn scalar(bits: u32, lo: f32, hi: f32, fallback: f32) -> f32 {
    let v = f32::from_bits(bits);
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        fallback
    }
}
/// Builds the surface extras for one material from its inspected graph. `uv` gives the
/// vertex coordinates (the same input the diffuse route used).
pub(crate) fn load(
    source: &AttachedSource,
    catalog: &Catalog,
    graph: &GraphInspection,
    vertices: usize,
    uv: crate::linked::UvInput,
    control: &LoadControl,
) -> Result<Surface, String> {
    control.check()?;
    let scalars = &graph.material.prefix.scalars;
    let routes = select_routes(&graph.graph)?;
    let mut surface = Surface {
        bump_level: bump_level(scalars.bump_level_bits),
        // CosinePower 22 is the decoded shaders' default (pod-fidelity README, D196).
        cosine_power: cosine_power(scalars.cosine_power_bits),
        alpha_mode_raw: scalars.alpha_mode_raw,
        alpha_test: scalar(scalars.alpha_test_bits, 0., 1., 0.),
        specular: routes.specular,
        spec_map: None,
        glow: None,
        glow_alpha: Default::default(),
        reflection: None,
        skipped: routes.skipped,
        bump: None,
    };
    if let Some(sampler) = routes.bump.filter(|_| surface.bump_level > 0.) {
        match load_bump(source, catalog, graph, sampler, vertices, uv, control) {
            Ok(b) => surface.bump = Some(Box::new(b)),
            Err(e) => surface.skipped.push(format!("bump not drawn ({e})")),
        }
    }
    if let Some(sampler) = routes.spec_map {
        match load_bump(source, catalog, graph, sampler, vertices, uv, control) {
            Ok(b) => surface.spec_map = Some(Box::new(b)),
            Err(e) => surface.skipped.push(format!("gloss map not drawn ({e})")),
        }
    }
    match load_reflection(source, catalog, graph, control) {
        Ok(None) => {}
        Ok(Some(r)) => surface.reflection = Some(Box::new(r)),
        Err(e) => surface.skipped.push(format!("reflection not drawn ({e})")),
    }
    Ok(surface)
}
/// P3i: the port 6 graph as per-fragment steps whose samplers read the reflection vector.
/// Pure (no I/O): returns the sampler slots in texture order and the encoded steps.
pub(crate) fn reflection_program(
    graph: &GraphMetadata,
) -> Result<Option<ReflectionProgram>, String> {
    let outputs: Vec<usize> = (0..graph.boxes.len())
        .filter(|&i| graph.boxes[i].type_raw == 0)
        .collect();
    let [output] = outputs[..] else {
        return Err("Unsupported: surface needs one output node".into());
    };
    let wires: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| w.box_to == output as i32 && w.port_to_raw == REFLECTION_PORT)
        .collect();
    let [wire] = wires[..] else {
        return if wires.is_empty() {
            Ok(None)
        } else {
            Err("several port 6 wires".into())
        };
    };
    let root = usize::try_from(wire.box_from)
        .ok()
        .filter(|&i| i < graph.boxes.len() && wire.port_from_raw == 0 && wire.swizzle_raw == [0; 5])
        .ok_or("port 6 wire is swizzled or leaves a non-zero port")?;
    let program = crate::grid_graph_program(graph, root)?;
    let sources = program.sources();
    let slots = crate::fragment::slots(&sources);
    if slots.len() > MAX_REFLECTION_TEXTURES {
        return Err(format!(
            "{} textures, at most {MAX_REFLECTION_TEXTURES}",
            slots.len()
        ));
    }
    let mut steps =
        crate::fragment::encode(&crate::grid_graph::steps(&program, &sources)?, &sources)?;
    if steps.len() > MAX_REFLECTION_STEPS {
        return Err(format!(
            "{} steps, at most {MAX_REFLECTION_STEPS}",
            steps.len()
        ));
    }
    for step in &mut steps {
        if step.code[0] == crate::fragment::FRAGMENT_SAMPLE {
            // The TXB coordinate is the plain reflection vector (reflection-uv oracle): no UV
            // channel, no sampler scale/offset.
            step.code[2] = 0;
            step.param = [1., 1., 0., 0.];
        }
    }
    Ok(Some((slots, steps)))
}
fn load_reflection(
    source: &AttachedSource,
    catalog: &Catalog,
    graph: &GraphInspection,
    control: &LoadControl,
) -> Result<Option<Reflection>, String> {
    let Some((slots, steps)) = reflection_program(&graph.graph)? else {
        return Ok(None);
    };
    let textures = slots
        .iter()
        .map(|&slot| load_slot(source, catalog, graph, slot, control))
        .collect::<Result<Vec<_>, _>>()?;
    let mut steps = steps;
    let mut maps = Vec::new();
    let mut cubes = 0;
    for (k, (layered, _, wrap)) in textures.into_iter().enumerate() {
        match layered.layout {
            "2d" => maps.push(ReflectionMap::Flat(layered.texture, wrap)),
            "cube" => {
                cubes += 1;
                if cubes > 1 {
                    return Err("Unsupported: several cube maps in one reflection graph".into());
                }
                let n = layered.texture.rgba.len();
                if layered.extra_faces.len() != lbp_texture::CUBE_FACES - 1
                    || layered.extra_faces.iter().any(|f| f.len() != n)
                    || layered.texture.header.width != layered.texture.header.height
                {
                    return Err("Invalid: cube map faces".into());
                }
                for step in &mut steps {
                    if step.code[0] == crate::fragment::FRAGMENT_SAMPLE
                        && step.code[1] as usize == k
                    {
                        step.code[2] = CUBE_CHANNEL;
                    }
                }
                maps.push(ReflectionMap::Cube(Box::new(layered)));
            }
            other => {
                return Err(format!(
                    "Unsupported: {other} texture in a reflection graph"
                ))
            }
        }
    }
    Ok(Some(Reflection {
        textures: maps,
        steps,
        blur: reflection_blur(graph.material.prefix.scalars.reflection_blur_bits),
    }))
}
/// Decodes the texture bound to material slot `slot`: 2D, or (P3l) a BC1/BC3 cube map.
fn load_slot(
    source: &AttachedSource,
    catalog: &Catalog,
    graph: &GraphInspection,
    slot: usize,
    control: &LoadControl,
) -> Result<
    (
        lbp_texture::LayeredTexture,
        lbp_catalog::attachment::LocationKey,
        crate::TextureWrapping,
    ),
    String,
> {
    let prefix = &graph.material.prefix;
    let descriptor = prefix
        .textures
        .get(slot)
        .ok_or("Unsupported: texture slot")?;
    if descriptor.kind == 0 || descriptor.flags != 0 {
        return Err("Unsupported: absent texture or descriptor flags".into());
    }
    let wrap = crate::TextureWrapping::from_raw(
        prefix.wrap_s_raw.get(slot).copied(),
        prefix.wrap_t_raw.get(slot).copied(),
    )?;
    let texture_key = crate::resolve(catalog, descriptor.guid, descriptor.sha1.as_deref())?;
    let texture = lbp_texture::decode_reader_layered(
        texture_key.size,
        |offset, count| {
            source
                .read_hex_checked(&texture_key, offset, count, control)
                .map(|p| p.bytes)
                .map_err(lbp_texture::Error::Input)
        },
        || control.check().is_err(),
    )
    .map_err(|e| e.to_string())?;
    Ok((texture, texture_key, wrap))
}
fn load_bump(
    source: &AttachedSource,
    catalog: &Catalog,
    graph: &GraphInspection,
    sampler: crate::linked::SamplerSelection,
    vertices: usize,
    uv: crate::linked::UvInput,
    control: &LoadControl,
) -> Result<BumpMap, String> {
    let prefix = &graph.material.prefix;
    let descriptor = prefix
        .textures
        .get(sampler.slot)
        .ok_or("Unsupported: bump texture slot")?;
    if descriptor.kind == 0 || descriptor.flags != 0 {
        return Err("Unsupported: absent bump texture or descriptor flags".into());
    }
    let wrap = crate::TextureWrapping::from_raw(
        prefix.wrap_s_raw.get(sampler.slot).copied(),
        prefix.wrap_t_raw.get(sampler.slot).copied(),
    )?;
    let coordinates = uv.decode(control)?;
    let raw = crate::linked::selected_uv(&coordinates, sampler.uv_channel, vertices)?;
    let sampling_uv = sampler
        .uv_transform
        .sampling(&raw, || control.check().is_err())?;
    let texture_key = crate::resolve(catalog, descriptor.guid, descriptor.sha1.as_deref())?;
    let texture = lbp_texture::decode_reader(
        texture_key.size,
        |offset, count| {
            source
                .read_hex_checked(&texture_key, offset, count, control)
                .map(|p| p.bytes)
                .map_err(lbp_texture::Error::Input)
        },
        || control.check().is_err(),
    )
    .map_err(|e| e.to_string())?;
    Ok(BumpMap {
        texture,
        texture_key,
        sampling_uv,
        wrap,
        uv_channel: sampler.uv_channel,
        uv_transform: sampler.uv_transform,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};

    fn node(type_raw: u32, parameters_raw: [u32; 8]) -> BoxMetadata {
        BoxMetadata {
            type_raw,
            declared_parameter_count: 8,
            parameters_raw,
            position_bits: [0; 4],
            subtype_raw: 0,
            animations: std::array::from_fn(|_| AnimationMetadata {
                base_value_bits: [0; 4],
                component_value_count: 0,
                component_bytes_sha256: String::new(),
                name_hex: "000000".into(),
                components_raw: 0,
                derived_key_count: 0,
            }),
            texture_slot_raw: None,
            texture_slot_in_range: None,
        }
    }
    fn graph(boxes: Vec<BoxMetadata>, wires: Vec<WireMetadata>) -> GraphMetadata {
        GraphMetadata {
            box_count: boxes.len(),
            wire_count: wires.len(),
            boxes,
            wires,
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        }
    }
    fn wire(from: i32, to: i32, port_to_raw: u8) -> WireMetadata {
        WireMetadata {
            box_from: from,
            box_to: to,
            port_from_raw: 0,
            port_to_raw,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        }
    }
    fn sampler_params(slot: u32) -> [u32; 8] {
        [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, slot, 0, 0]
    }

    #[test]
    fn p3i_reflection_samplers_read_the_reflection_vector() {
        // output <- port 6 <- MIX(COLOR, sampler slot 2) with the unwired weight 0.25
        let mut mix_params = [0; 8];
        mix_params[0] = 0.25f32.to_bits();
        let color = [0.1f32, 0.2, 0.3, 1.0].map(f32::to_bits);
        let g = graph(
            vec![
                node(0, [0; 8]),
                node(12, mix_params),
                node(3, [color[0], color[1], color[2], color[3], 0, 0, 0, 0]),
                node(1, [2f32.to_bits(), 3f32.to_bits(), 0, 0, 0, 2, 0, 0]),
            ],
            vec![wire(1, 0, REFLECTION_PORT), wire(2, 1, 0), wire(3, 1, 1)],
        );
        let (slots, steps) = reflection_program(&g).unwrap().unwrap();
        assert_eq!(slots, vec![2]);
        let sample = steps
            .iter()
            .find(|s| s.code[0] == crate::fragment::FRAGMENT_SAMPLE)
            .unwrap();
        assert_eq!(sample.code[2], 0, "no UV channel");
        assert_eq!(sample.param, [1., 1., 0., 0.], "scale/offset ignored");
        assert!(steps.len() <= MAX_REFLECTION_STEPS);
        // Unwired port 6: nothing. Two wires: refused.
        let plain = graph(vec![node(0, [0; 8])], vec![]);
        assert!(reflection_program(&plain).unwrap().is_none());
        let two = graph(
            vec![node(0, [0; 8]), node(1, sampler_params(0))],
            vec![wire(1, 0, REFLECTION_PORT), wire(1, 0, REFLECTION_PORT)],
        );
        assert!(reflection_program(&two).is_err());
    }

    #[test]
    fn p3h_glow_graph_replaces_the_diffuse_wire_on_a_copy() {
        // output 0; sampler 1 -> port 0 (diffuse); sampler 2 -> port 4 (glow)
        let g = graph(
            vec![
                node(0, [0; 8]),
                node(1, sampler_params(0)),
                node(1, sampler_params(1)),
            ],
            vec![wire(1, 0, 0), wire(2, 0, GLOW_PORT)],
        );
        let d = rewire_output(&g, GLOW_PORT).unwrap().unwrap();
        assert_eq!(d.wires.len(), 1);
        assert_eq!((d.wires[0].box_from, d.wires[0].port_to_raw), (2, 0));
        assert_eq!(d.wire_count, 1);
        // The original is untouched; an unwired port is None; two wires are refused.
        assert_eq!(g.wires.len(), 2);
        assert!(rewire_output(&g, BUMP_PORT).unwrap().is_none());
        let two = graph(
            vec![node(0, [0; 8]), node(1, sampler_params(0))],
            vec![wire(1, 0, GLOW_PORT), wire(1, 0, GLOW_PORT)],
        );
        assert!(rewire_output(&two, GLOW_PORT).is_err());
    }

    #[test]
    fn p2q_routes_take_one_bump_sampler_and_one_static_gloss_color() {
        let gloss = [0.6f32, 0.6, 0.6, 1.].map(f32::to_bits);
        let mut color = [0; 8];
        color[..4].copy_from_slice(&gloss);
        let full = graph(
            vec![node(0, [0; 8]), node(1, sampler_params(2)), node(3, color)],
            vec![wire(1, 0, BUMP_PORT), wire(2, 0, SPECULAR_PORT)],
        );
        let routes = select_routes(&full).unwrap();
        assert_eq!(routes.bump.unwrap().slot, 2);
        assert_eq!(routes.specular, Some([0.6, 0.6, 0.6]));
        assert!(routes.skipped.is_empty());

        // A swizzled bump wire is named, not drawn; a sampler on the gloss port is a gloss map.
        let mut swizzled = full.clone();
        swizzled.wires[0].swizzle_raw = [b'a', 0, 0, 0, 0];
        swizzled.wires[1].box_from = 1;
        let routes = select_routes(&swizzled).unwrap();
        assert!(routes.bump.is_none() && routes.specular.is_none());
        assert_eq!(routes.spec_map.unwrap().slot, 2);
        assert_eq!(routes.skipped.len(), 1);

        // No surface wires: nothing drawn, nothing skipped.
        let plain = graph(full.boxes, vec![wire(1, 0, 0)]);
        assert_eq!(select_routes(&plain).unwrap(), SurfaceRoutes::default());
    }

    #[test]
    fn p2q_untrusted_scalars_are_clamped_or_defaulted() {
        assert_eq!(scalar(f32::NAN.to_bits(), 1., 4096., 22.), 22.);
        assert_eq!(scalar(1e9f32.to_bits(), 1., 4096., 22.), 4096.);
        assert_eq!(scalar(0.6f32.to_bits(), 0., 16., 0.), 0.6);
        assert_eq!(scalar((-1f32).to_bits(), 0., 16., 0.), 0.);
    }

    /// D307: the stored AlphaTestLevel cuts out only where the program names the constant.
    #[test]
    fn d307_alpha_test_needs_the_program_to_name_it() {
        assert!(alpha_test_in_program(
            b"CGB\0tb1\0AlphaTestLevel\0tintcol\0"
        ));
        assert!(!alpha_test_in_program(b"CGB\0tb1\0tintcol\0fogcol\0"));
        assert!(!alpha_test_in_program(b"\0AlphaTestLevelX\0"));
        assert!(alpha_test_in_program(b""));
    }
    #[test]
    fn p3h_glow_needs_the_program_to_name_the_colour_box() {
        // output <- port 4 <- colour box 1; the blob names ta1 or only ta10
        let g = graph(
            vec![node(0, [0; 8]), node(3, [0; 8])],
            vec![wire(1, 0, GLOW_PORT)],
        );
        assert!(glow_in_program(&g, b"CGB\0ta1\0ta1\0\0ta1\0x"));
        assert!(!glow_in_program(&g, b"\0ta10\0\0ta2\0"));
        assert!(!glow_in_program(&g, b""));
        // texture sources are not decided here
        let t = graph(
            vec![node(0, [0; 8]), node(1, sampler_params(0))],
            vec![wire(1, 0, GLOW_PORT)],
        );
        assert!(glow_in_program(&t, b""));
    }

    #[test]
    fn reflection_blur_is_the_stored_scalar_minus_one() {
        assert!((reflection_blur(1.45f32.to_bits()) - 0.45).abs() < 1e-6);
        assert_eq!(reflection_blur(0.5f32.to_bits()), 0.);
        assert_eq!(reflection_blur(f32::NAN.to_bits()), 0.);
    }

    #[test]
    fn shader_scalars_are_transforms_of_the_stored_ones() {
        // glass: stored BumpLevel 0.12 -> listing 0.24; default stored CosinePower 1 -> 22
        assert!((bump_level(0.12f32.to_bits()) - 0.24).abs() < 1e-6);
        assert_eq!(cosine_power(1f32.to_bits()), 22.);
        assert_eq!(cosine_power(2f32.to_bits()), 44.);
        assert_eq!(cosine_power(f32::NAN.to_bits()), 22.);
        assert_eq!(cosine_power(1e9f32.to_bits()), MAX_COSINE_POWER);
        assert_eq!(bump_level(f32::NAN.to_bits()), 0.);
    }
}
