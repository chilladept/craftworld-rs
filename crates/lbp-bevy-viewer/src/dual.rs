//! Authored unlit two/three/four-sample extension. Evidence: docs/dual-preview.md,
//! docs/nested-preview.md, docs/four-product-preview.md, docs/third-sample-preview.md.
//! Third samples second (or primary, D65) coordinates through a CPU-derived
//! scale/offset; fourth shares primary coordinates after exact CPU validation, or
//! (D101, docs/d101-multiply-rest.md) primary/second coordinates × xy + zw.
//! D99: `enabled.w` selects the two-texture alpha MIX mode (docs/d99-product-trees.md).
//! D121: opt-in preview lighting multiplies the finished colour by a Lambert factor
//! (docs/d121-preview-lighting.md); off (`light_dir.w` = 0) leaves the colour untouched.
//! P2l: `light_dir.w` = 2 selects the opt-in LBP-shaped factor (docs/p2l-lbp-lighting.md).
//! P2q: under that factor, an optional bump map on UV1 tilts the normal and a static
//! gloss colour adds a specular term (docs/p2q-bump-gloss.md).
//! D189: `graph_info.x` > 0 evaluates a D100 graph per fragment over raw UV0/UV1
//! (docs/d189-mixed-uv-graphs.md); graph textures 0..2 reuse bindings 100/103/105.
//! D278: a non-zero `dissolve` plane cuts the fragments above a line and tints a band
//! under it (the Pod tent's change-over; docs/level-scripts.md part 29, `pod_dissolve.rs`).
//! D290: the glass draws through dual-source blending where the device has it (one
//! transmittance per channel); without it, the D287 blend with one T stays.
//! D292: while the dissolve is on, opaque surfaces write the kept share as their alpha (the
//! game's alpha = t), which the edge glow (`glow_post.rs`) reads.
//! D303: on the level-lighting route an opaque surface with material glow (port 4) writes
//! the game's alpha `1 / (1 + 4 |g|^2)` (FOUND, frame 5247: the lamp bulb and the stars),
//! which the glow's source pass and the final pass read.
use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline},
    prelude::*,
    render::{
        render_resource::{
            AsBindGroup, BlendComponent, BlendFactor, BlendOperation, Face,
            RenderPipelineDescriptor, SpecializedMeshPipelineError,
        },
        renderer::RenderDevice,
        settings::WgpuFeatures,
        RenderApp, RenderStartup,
    },
    shader::{Shader, ShaderRef},
};
use std::sync::atomic::{AtomicBool, Ordering};
pub type ViewerMaterial = ExtendedMaterial<StandardMaterial, SecondTexture>;
const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("c1f79fb1-32b7-4f6f-bef7-71a8c677f05c");
/// D290: the same shader with `enable dual_source_blending;`, used by the glass's pipeline
/// only, and only when the device has the feature.
const GLASS_DUAL_SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("5b0d6f4e-2c1a-4e8b-9f3d-7a6c2e1b0d90");
/// D290: whether the render device has `DUAL_SOURCE_BLENDING` (set at render start-up).
static DUAL_SOURCE: AtomicBool = AtomicBool::new(false);
/// D290: the pipeline key of the extension: whether the material is glass.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SecondTextureKey {
    glass: bool,
}
impl From<&SecondTexture> for SecondTextureKey {
    fn from(material: &SecondTexture) -> Self {
        Self {
            glass: material.glass.x > 0.,
        }
    }
}
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
#[bind_group_data(SecondTextureKey)]
pub struct SecondTexture {
    #[texture(100)]
    #[sampler(101)]
    pub image: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    pub third_image: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    pub fourth_image: Option<Handle<Image>>,
    #[uniform(102)]
    enabled: Vec4,
    /// Third sample: base coordinates × xy + zw.
    #[uniform(107)]
    third_transform: Vec4,
    /// D101 fourth sample: base coordinates × xy + zw.
    #[uniform(108)]
    fourth_transform: Vec4,
    /// D121: direction toward the key light (xyz), w = 1 when this material is lit.
    #[uniform(109)]
    light_dir: Vec4,
    /// D121: key light colour (rgb).
    #[uniform(110)]
    light_key: Vec4,
    /// D121: ambient colour (rgb).
    #[uniform(111)]
    light_ambient: Vec4,
    /// P2l: lower rim colour (rgb), used toward N.y = -1.
    #[uniform(112)]
    rim_low: Vec4,
    /// P2l: upper rim colour (rgb), used toward N.y = +1.
    #[uniform(113)]
    rim_high: Vec4,
    /// P2q: bump map (linear, UV1).
    #[texture(114)]
    #[sampler(115)]
    pub bump_image: Option<Handle<Image>>,
    /// P2q: x = BumpLevel, y = 1 when the bump map is bound.
    #[uniform(116)]
    bump_params: Vec4,
    /// P2q: gloss colour (rgb) and CosinePower (w; 0 = no gloss).
    #[uniform(117)]
    gloss: Vec4,
    /// P2t: game fog: start, slope (display units), luma CC, on (w).
    #[uniform(118)]
    lbp_fog: Vec4,
    /// P2t: game fog colour (output space).
    #[uniform(119)]
    lbp_fog_color: Vec4,
    /// D189: graph textures 3..5 (0..2 are `image`, `third_image`, `fourth_image`).
    #[texture(120)]
    #[sampler(121)]
    pub graph_image3: Option<Handle<Image>>,
    #[texture(122)]
    #[sampler(123)]
    pub graph_image4: Option<Handle<Image>>,
    #[texture(124)]
    #[sampler(125)]
    pub graph_image5: Option<Handle<Image>>,
    /// D189: x = step count (0 = off).
    #[uniform(126)]
    graph_info: UVec4,
    /// D189: `lbp_preview::FragmentStep::code` per step.
    #[uniform(127)]
    graph_code: [UVec4; 32],
    /// D189: `lbp_preview::FragmentStep::param` per step.
    #[uniform(128)]
    graph_param: [Vec4; 32],
    /// P3l: the reflection graph's cube map (sample steps with channel 2).
    #[texture(140, dimension = "cube")]
    #[sampler(141)]
    pub cube_image: Option<Handle<Image>>,
    /// P3k: backdrop baked-AO lightmap (R8, UV1).
    #[texture(129)]
    #[sampler(130)]
    pub lightmap: Option<Handle<Image>>,
    /// P3k: x = bakedAOScale, y = bakedAOBias, z = 1 when a lightmap is bound and lit.
    #[uniform(131)]
    ao_params: Vec4,
    /// D268: a tint a level script gave the Thing's render mesh (rgb), w = 1 when set.
    #[uniform(132)]
    thing_tint: Vec4,
    /// D278: the dissolve line as a plane in Bevy world space: `dot(xyz, world position)
    /// + w` is the height above the line in the level's own units. Zero = off.
    #[uniform(133)]
    dissolve: Vec4,
    /// D278: y = 1 when the mask is bound. D284: x = the input that holds raw UV0 (1 =
    /// `in.uv`, 2 = `in.uv_b`, 0 = none: the mask goes over the position), z = 1 when
    /// `pod_mask` is bound, w = 1 at value 1 (the rule's last term).
    #[uniform(134)]
    dissolve_params: Vec4,
    /// D278: the transition mask (linear, repeating).
    #[texture(135)]
    #[sampler(136)]
    dissolve_mask: Option<Handle<Image>>,
    /// D278: the flat colour the band under the line goes to (rgb, linear).
    #[uniform(137)]
    dissolve_colour: Vec4,
    /// D278: x = units the line rises per unit of the surface colour's green, y = the
    /// band's height, z = the edge value above which the band is all flat colour, w = mask
    /// tiles per unit of UV (`pod_dissolve::consts`).
    #[uniform(138)]
    dissolve_consts: Vec4,
    /// D278: mask coordinates from the world position, in tiles, for a bucket without
    /// usable UV: `dot(xyz, world position) + w` each.
    #[uniform(139)]
    dissolve_mask_u: Vec4,
    #[uniform(142)]
    dissolve_mask_v: Vec4,
    /// D284: `pod_mask` (linear, clamped), sampled at raw UV0: where its red is over one
    /// half the line stands one band higher.
    #[texture(143)]
    #[sampler(144)]
    dissolve_island: Option<Handle<Image>>,
    /// D286: clear glass. x = the output alpha (0 = not glass: alpha 1, colour as is), y =
    /// the green the dissolve's edge takes in place of the surface colour's. D287: z = the
    /// scale of the specular added after the alpha (the game's fc45).
    #[uniform(145)]
    glass: Vec4,
    /// D121: generated normals passed `adapter::normal_trust`; otherwise stays unlit.
    pub normals_trusted: bool,
    /// D284: the input of this bucket that holds the mesh's raw UV0
    /// (`pod_dissolve::uv0_source`): 1 = `in.uv`, 2 = `in.uv_b`, 0 = none.
    pub raw_uv0: u8,
}
impl MaterialExtension for SecondTexture {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    /// D290: the glass's main-pass pipeline takes the dual-source shader and blends the
    /// colour as One + OneMinusSrc1 (a transmittance per channel); everything else, and
    /// the glass on a device without the feature, keeps the pipeline as it was. D307: the
    /// glass's alpha keeps the destination's (Zero + One) on both.
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if !key.bind_group_data.glass {
            return Ok(());
        }
        if let Some(blend) = descriptor
            .fragment
            .as_mut()
            .filter(|f| f.shader == SHADER && f.targets.len() == 1)
            .and_then(|f| f.targets.first_mut())
            .and_then(|t| t.as_mut())
            .and_then(|t| t.blend.as_mut())
        {
            blend.alpha = glass_alpha_blend();
        }
        if !DUAL_SOURCE.load(Ordering::Relaxed) {
            return Ok(());
        }
        let Some(fragment) = descriptor.fragment.as_mut() else {
            return Ok(());
        };
        if fragment.shader != SHADER || fragment.targets.len() != 1 {
            return Ok(());
        }
        let Some(Some(target)) = fragment.targets.first_mut() else {
            return Ok(());
        };
        let Some(blend) = target.blend.as_mut() else {
            return Ok(());
        };
        blend.color = glass_dual_color_blend();
        fragment.shader = GLASS_DUAL_SHADER;
        fragment.shader_defs.push("GLASS_DUAL_SOURCE".into());
        Ok(())
    }
}
/// D307 FOUND (frames 5247 and 14790, the blend state of every enabled blend): the glass
/// (5247's 3033 and 2732) blends its alpha as Zero + One, so the scene keeps the alpha
/// beneath it (a glowing surface behind the glass keeps its glow alpha). The translucent
/// list's draws (the planets, 2910 to 3015; the Moon's 3903 to 4068) blend alpha as
/// InvDstAlpha + One, `a_d + (1 - a_d) a_s`, which is Bevy's One + OneMinusSrcAlpha (`a_s +
/// (1 - a_s) a_d`) for its blended pipelines; only the glass needs its own.
fn glass_alpha_blend() -> BlendComponent {
    BlendComponent {
        src_factor: BlendFactor::Zero,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    }
}
/// D290: the glass's colour blend with dual sources: out = S + (1 - src1) D, src1 = 1 - t.
fn glass_dual_color_blend() -> BlendComponent {
    BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrc1,
        operation: BlendOperation::Add,
    }
}
/// D290: notes whether the device can blend with two sources.
fn note_dual_source(device: Res<RenderDevice>) {
    let on = device
        .features()
        .contains(WgpuFeatures::DUAL_SOURCE_BLENDING);
    DUAL_SOURCE.store(on, Ordering::Relaxed);
    info!(
        "D290: glass dual-source blending {}",
        if on { "on" } else { "off (one T)" }
    );
}
pub fn material(base: StandardMaterial) -> ViewerMaterial {
    ViewerMaterial {
        base,
        extension: SecondTexture::default(),
    }
}
pub fn enable(material: &mut ViewerMaterial, image: Handle<Image>) {
    material.extension.image = Some(image);
    material.extension.enabled = Vec4::X;
}
/// D99: off-grid two-texture alpha MIX (docs/d99-product-trees.md): the primary and
/// second samples are mixed by the second sample's alpha instead of multiplied.
pub fn enable_alpha_mix(material: &mut ViewerMaterial) {
    material.extension.enabled.w = 1.;
}
pub fn enable_third(
    material: &mut ViewerMaterial,
    image: Handle<Image>,
    relative: lbp_preview::RelativeSampling,
) {
    material.extension.third_image = Some(image);
    material.extension.enabled.y = if relative.from_primary { 2. } else { 1. };
    material.extension.third_transform = Vec4::new(
        relative.scale[0],
        relative.scale[1],
        relative.offset[0],
        relative.offset[1],
    );
}
/// `enabled.z`: 1 = primary coordinates, 2 = second-sample coordinates (D101).
pub fn enable_fourth(
    material: &mut ViewerMaterial,
    image: Handle<Image>,
    relative: lbp_preview::RelativeSampling,
) {
    material.extension.fourth_image = Some(image);
    material.extension.enabled.z = if relative.from_primary { 1. } else { 2. };
    material.extension.fourth_transform = Vec4::new(
        relative.scale[0],
        relative.scale[1],
        relative.offset[0],
        relative.offset[1],
    );
}
/// D189: per-fragment graph over raw UV0 (`uv`) and UV1 (`uv_b`); `images[k]` is graph
/// texture k (at most 6), `steps` the encoded D100 program (root last, at most 32).
pub fn enable_graph(
    material: &mut ViewerMaterial,
    images: Vec<Handle<Image>>,
    steps: &[lbp_preview::FragmentStep],
) -> Result<(), String> {
    if images.is_empty()
        || images.len() > lbp_preview::MAX_FRAGMENT_TEXTURES
        || steps.is_empty()
        || steps.len() > lbp_preview::MAX_FRAGMENT_STEPS
    {
        return Err("Invalid: per-fragment graph budget".into());
    }
    let ext = &mut material.extension;
    let mut images = images.into_iter();
    ext.image = images.next();
    ext.third_image = images.next();
    ext.fourth_image = images.next();
    ext.graph_image3 = images.next();
    ext.graph_image4 = images.next();
    ext.graph_image5 = images.next();
    for (k, step) in steps.iter().enumerate() {
        ext.graph_code[k] = UVec4::from_array(step.code);
        ext.graph_param[k] = Vec4::from_array(step.param);
    }
    ext.graph_info = UVec4::new(steps.len() as u32, 0, 0, 0);
    Ok(())
}
/// P2q: bump map sampled on UV1, offset (.yw - 0.502) × `level` (docs/p2q-bump-gloss.md).
pub fn enable_bump(material: &mut ViewerMaterial, image: Handle<Image>, level: f32) {
    material.extension.bump_image = Some(image);
    material.extension.bump_params = Vec4::new(level, 1., 0., 0.);
}
/// P3d: gloss colour map on `graph_image5` (free unless a D189 graph is bound) with the
/// static gloss white; `uv_mode` 1 = primary UV, 2 = UV1. Call only without a graph.
pub fn enable_spec_map(material: &mut ViewerMaterial, image: Handle<Image>, uv_mode: u32) {
    material.extension.graph_image5 = Some(image);
    material.extension.graph_info.y = uv_mode;
}
/// P3a: alpha cut-out: fragments whose colour alpha is below `level` are discarded (LBP
/// mode materials with a stored AlphaTestLevel, docs/p3a-alpha-test.md).
pub fn enable_alpha_test(material: &mut ViewerMaterial, level: f32) {
    material.extension.bump_params.z = level;
}
/// P3h: unlit glow added to the lit colour. `color` multiplies the optional map (graph slot
/// 4, sRGB like the diffuse); `mode` 2 = primary UV, 3 = UV1. Rides graph_info.w and the
/// last graph_param row, so it needs no per-fragment graph (slots shared). D307: `fogged`
/// says whether the material's program mixes its glow alpha towards 1 by the fog
/// ([`glow_alpha_fogged`]); when it does not, [`GLOW_ALPHA_UNFOGGED_BIT`] rides graph_info.w.
pub fn enable_glow(
    material: &mut ViewerMaterial,
    color: [f32; 3],
    map: Option<(Handle<Image>, u32)>,
    fogged: bool,
) {
    let ext = &mut material.extension;
    let blur = ext.graph_param[31].w;
    ext.graph_param[31] = Vec4::new(color[0], color[1], color[2], blur);
    let mode = match map {
        Some((image, mode)) => {
            ext.graph_image4 = Some(image);
            mode
        }
        None => 1,
    };
    ext.graph_info.w = if fogged {
        mode
    } else {
        mode | GLOW_ALPHA_UNFOGGED_BIT
    };
}
/// D307: graph_info.w's bit for a glow whose alpha the fog leaves alone (the glow mode is
/// the low four bits).
pub const GLOW_ALPHA_UNFOGGED_BIT: u32 = 16;
/// D307 FOUND (the 7,980 decoded gmat programs, variant 8, the alpha's last write): 524
/// programs write the glow alpha as `tintcol.w x (a + f (1 - a))` with `a = 1 / (1 + 4
/// |g|^2)` and `f = sat(TC3.z)` the fog (RCP, the fog MAD, MUL by tintcol), so the fog
/// lifts the alpha towards 1 as it lerps the colour; these 23 write `tintcol.w / (1 + 4
/// |g|^2)` (DIV by the sum, or `burn`'s RCP) and the fog leaves the alpha alone. Among them
/// the lamp bulb and the stars of frame 5247 (part 54, the replay) and the checkpoint ring
/// (`neon_lifebar`). Nothing in the material's stored graph or scalars tells the two apart
/// (the same graph, flags and revision occur in both), so the list is by GUID.
pub const GLOW_ALPHA_UNFOGGED: [u32; 23] = [
    0x000175a3, // burn
    0x0002c0b3, // cats_eyes_fi
    0x00006a49, // cosmos_starc (the Pod's stars)
    0x0005d405, // dcc_lanternpower
    0x0001ea04, // evil_eye
    0x00009888, // eye_01_base_colour
    0x0002b27b, // killzone3_hazmat_to_metal
    0x0002b283, // killzone3_rico_to_metal
    0x0000d6cc, // light_bulb (the Pod's lamp bulb)
    0x0000be15, // neon_lifebar (the checkpoint ring)
    0x00018401, // proc_base_hologram
    0x00024e3e, // toy_buzz_pack
    0x00024e74, // toy_lounge_vcr
    0x00025fb5, // toy_woodys_roundup
    0x0002b365, // toy_zurg_badge_illum
    0x0002ab42, // tron_abraxas_to
    0x0002ab45, // tron_anon_he
    0x0002894b, // tron_clu_hd
    0x0002894c, // tron_clu_hd_disk
    0x0002894d, // tron_clu_he_helmet
    0x0002894f, // tron_clu_lg
    0x00028950, // tron_clu_to
    0x0001d1fd, // victoria_fire
];
/// D307: whether a glowing material's program mixes its alpha towards 1 by the fog: every
/// program but [`GLOW_ALPHA_UNFOGGED`]'s (a material of unknown GUID takes the common form).
pub fn glow_alpha_fogged(material: Option<u32>) -> bool {
    material.is_none_or(|m| !GLOW_ALPHA_UNFOGGED.contains(&m))
}
/// P3i: reflection graph sampled at R.xy with LOD bias `blur` (docs/p3i-reflection.md).
/// `images[k]` is graph texture k (at most 4, graph slots 0..=3), `steps` at most 31 rows
/// from row 0; the count rides graph_info.z >> 8 and the blur graph_param[31].w. Needs no
/// diffuse graph or second/third/fourth sample (the slots are shared).
pub fn enable_reflection(
    material: &mut ViewerMaterial,
    images: Vec<Option<Handle<Image>>>,
    cube: Option<Handle<Image>>,
    steps: &[lbp_preview::FragmentStep],
    blur: f32,
) -> Result<(), String> {
    if images.is_empty() || images.len() > 4 || steps.is_empty() || steps.len() > 31 {
        return Err("Invalid: reflection graph budget".into());
    }
    let ext = &mut material.extension;
    let mut images = images.into_iter();
    ext.image = images.next().flatten();
    ext.third_image = images.next().flatten();
    ext.fourth_image = images.next().flatten();
    ext.graph_image3 = images.next().flatten();
    ext.cube_image = cube;
    for (k, step) in steps.iter().enumerate() {
        ext.graph_code[k] = UVec4::from_array(step.code);
        ext.graph_param[k] = Vec4::from_array(step.param);
    }
    ext.graph_param[31].w = blur;
    ext.graph_info.z = (ext.graph_info.z & 1) | ((steps.len() as u32) << 8);
    Ok(())
}
/// P2q: static gloss colour with exponent `power` (CosinePower).
pub fn enable_gloss(material: &mut ViewerMaterial, color: [f32; 3], power: f32) {
    material.extension.gloss = Vec3::from(color).extend(power.max(1.));
}
/// D121: applies (Some) or clears (None) preview lighting; returns whether the material
/// is lit. Untrusted normals keep the unlit colour (counted by the caller).
pub fn set_lighting(
    material: &mut ViewerMaterial,
    light: Option<&crate::lighting::PreviewLight>,
) -> bool {
    let ext = &mut material.extension;
    // P2s: LBP mode lights every draw; untrusted smooth normals use the face normal (3).
    match light.filter(|l| ext.normals_trusted || l.lbp) {
        Some(l) => {
            let mode = match (l.lbp, ext.normals_trusted) {
                (false, _) => 1.,
                (true, true) => 2.,
                (true, false) => 3.,
            };
            ext.light_dir = l.toward_light.extend(mode);
            // P2q: .w carries the game's scalar per light (brdf.cg `light.w`).
            ext.light_key = l.key.extend(l.scales.x);
            ext.light_ambient = l.ambient.extend(l.scales.y);
            ext.rim_low = l.rim[0].extend(l.scales.z);
            ext.rim_high = l.rim[1].extend(l.scales.w);
            ext.lbp_fog = l.lbp_fog();
            ext.graph_info.z = (ext.graph_info.z & !1) | u32::from(l.shadows);
            ext.ao_params = Vec4::new(
                l.baked_ao[0],
                l.baked_ao[1],
                f32::from(u8::from(l.lbp && ext.lightmap.is_some())),
                0.,
            );
            ext.lbp_fog_color = l.lbp_fog_color().extend(1.);
            true
        }
        None => {
            ext.light_dir = Vec4::ZERO;
            ext.light_key = Vec4::ZERO;
            ext.light_ambient = Vec4::ZERO;
            ext.rim_low = Vec4::ZERO;
            ext.rim_high = Vec4::ZERO;
            ext.lbp_fog = Vec4::ZERO;
            ext.graph_info.z &= !1;
            ext.ao_params = Vec4::ZERO;
            false
        }
    }
}
/// P3k: binds the backdrop lightmap; the AO scale/bias and the on flag come from
/// `set_lighting` (LBP mode only).
pub fn enable_lightmap(material: &mut ViewerMaterial, image: Handle<Image>) {
    material.extension.lightmap = Some(image);
}
/// D268: multiplies the surface colour by a script's render-mesh tint before lighting
/// (`None`: no tint). INFERRED: how the game's shader uses the tint was not read.
pub fn set_thing_tint(material: &mut ViewerMaterial, tint: Option<[f32; 3]>) {
    material.extension.thing_tint = match tint {
        Some([r, g, b]) => Vec4::new(r, g, b, 1.),
        None => Vec4::ZERO,
    };
}
/// D278: whether a script's tint is on the material ([`set_thing_tint`]).
pub fn tinted(material: &ViewerMaterial) -> bool {
    material.extension.thing_tint.w > 0.5
}
/// D286: draws the material as the game's glass (`window_glassy`): the fragment's alpha
/// is `opacity` times the dissolve's kept share; the dissolve's edge takes `green` in
/// place of the surface colour's green. D287: the colour is that alpha times the encoded
/// lit colour plus `specular` times the gloss (added after the alpha, as the game does),
/// written for `AlphaMode::Premultiplied` through [`glass_blend`].
pub fn enable_glass(material: &mut ViewerMaterial, opacity: f32, green: f32, specular: f32) {
    material.extension.glass = Vec4::new(opacity, green, specular, 0.);
}
/// D287: the two encoded destination values the glass's blend is exact at
/// ([`glass_blend`]); chosen on the captured frame's and the viewer's room pixels. The
/// shader writes the same numbers.
#[cfg(test)]
pub const GLASS_SECANT: [f32; 2] = [0.35, 0.75];
/// D287: sRGB encoding of a linear value (the viewer's target), clamped to 0..1.
#[cfg(test)]
pub fn srgb_encode(c: f32) -> f32 {
    let v = c.clamp(0., 1.);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
/// D287: sRGB decoding of an encoded value, clamped to 0..1.
#[cfg(test)]
pub fn srgb_decode(v: f32) -> f32 {
    let v = v.clamp(0., 1.);
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
/// D287 (mirrors the shader's `glass_blend`): the game blends the glass in its encoded
/// output, `out = s + (1 - a) d` (8-bit target, no sRGB). The viewer's target blends
/// linear light, `out = S + T D` with one T for the three channels (One /
/// OneMinusSrcAlpha). S and T are the secant of the game's blend, in linear light,
/// through the destinations [`GLASS_SECANT`]: T the mean of the three slopes, S the mean
/// of the two points' residuals. Returns (S, 1 - T), the premultiplied colour and alpha.
/// D290: the fallback, used only where the device has no dual-source blending.
#[cfg(test)]
pub fn glass_blend(s: [f32; 3], a: f32) -> ([f32; 3], f32) {
    let [d1, d2] = GLASS_SECANT;
    let (l1, l2) = (srgb_decode(d1), srgb_decode(d2));
    let y = |d: f32| s.map(|c| srgb_decode(c + (1. - a) * d));
    let (y1, y2) = (y(d1), y(d2));
    let t = ((0..3).map(|i| (y2[i] - y1[i]) / (l2 - l1)).sum::<f32>() / 3.).clamp(0., 1.);
    let c = std::array::from_fn(|i| (0.5 * ((y1[i] - t * l1) + (y2[i] - t * l2))).max(0.));
    (c, 1. - t)
}
/// D290 (mirrors the shader's `glass_secant`): the same secant per channel, through
/// dual-source blending, `out = S + (1 - src1) D`: `t_c = clamp((y2_c - y1_c) / (l2 -
/// l1), 0, 1)` and `S_c = max(0.5 ((y1_c - t_c l1) + (y2_c - t_c l2)), 0)`. Returns (S,
/// 1 - t), the colour (source 0) and the per-channel second source.
#[cfg(test)]
pub fn glass_blend_per_channel(s: [f32; 3], a: f32) -> ([f32; 3], [f32; 3]) {
    let [d1, d2] = GLASS_SECANT;
    let (l1, l2) = (srgb_decode(d1), srgb_decode(d2));
    let y = |d: f32| s.map(|c| srgb_decode(c + (1. - a) * d));
    let (y1, y2) = (y(d1), y(d2));
    let t: [f32; 3] = std::array::from_fn(|i| ((y2[i] - y1[i]) / (l2 - l1)).clamp(0., 1.));
    let c = std::array::from_fn(|i| (0.5 * ((y1[i] - t[i] * l1) + (y2[i] - t[i] * l2))).max(0.));
    (c, t.map(|t| 1. - t))
}
/// D291 (mirrors the shader's `dissolve_mix` on the routes without the game's fog): the
/// game mixes the flat colour and the lit colour in its encoded output, `out = mix(COL0,
/// lit, t)`; the viewer's encoded value is the sRGB encoding of its linear output. Takes
/// and returns linear colours: `decode(mix(encode(flat), encode(lit), keep))`, the lit
/// colour itself where it is kept whole.
#[cfg(test)]
pub fn dissolve_mix(flat: [f32; 3], lit: [f32; 3], keep: f32) -> [f32; 3] {
    if keep >= 1. {
        return lit;
    }
    std::array::from_fn(|i| {
        let (f, l) = (srgb_encode(flat[i]), srgb_encode(lit[i]));
        srgb_decode(f + (l - f) * keep)
    })
}
/// D303 FOUND (frame 5247; mirrors the shader's `lbp_glow_alpha`): the alpha an opaque
/// surface with glow `g` writes on the level-lighting route, `1 / (1 + 4 |g|^2)`.
#[cfg(test)]
pub fn glow_alpha(g: [f32; 3]) -> f32 {
    glow_alpha_under_fog(g, 0., false)
}
/// D307 FOUND (mirrors the shader): the glow alpha with the fog's share `f` (0 to 1): `a +
/// f (1 - a)` for the common program, `a` for [`GLOW_ALPHA_UNFOGGED`]'s.
#[cfg(test)]
pub fn glow_alpha_under_fog(g: [f32; 3], f: f32, fogged: bool) -> f32 {
    let a = 1. / (1. + 4. * (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]));
    if fogged {
        a + f.clamp(0., 1.) * (1. - a)
    } else {
        a
    }
}
/// D286: the glass's opacity, none when the material is not glass.
#[cfg(test)]
pub fn glass(material: &ViewerMaterial) -> Option<f32> {
    let opacity = material.extension.glass.x;
    (opacity > 0.).then_some(opacity)
}
/// D287: the glass's specular scale (0 when the material is not glass).
#[cfg(test)]
pub fn glass_specular_scale(material: &ViewerMaterial) -> f32 {
    material.extension.glass.z
}
/// D278: what [`set_dissolve`] writes. See the fields of the same names above.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dissolve {
    pub plane: Vec4,
    pub colour: Vec3,
    pub consts: Vec4,
    pub mask_u: Vec4,
    pub mask_v: Vec4,
    pub mask: Option<Handle<Image>>,
    /// D284: `pod_mask`, for the card materials only.
    pub island: Option<Handle<Image>>,
    /// D284: value 1, the rule's last term on.
    pub at_one: bool,
}
/// D278: the dissolve a material draws with now; none when it is off.
pub fn dissolve(material: &ViewerMaterial) -> Option<Dissolve> {
    let ext = &material.extension;
    (ext.dissolve.truncate() != Vec3::ZERO).then(|| Dissolve {
        plane: ext.dissolve,
        colour: ext.dissolve_colour.truncate(),
        consts: ext.dissolve_consts,
        mask_u: ext.dissolve_mask_u,
        mask_v: ext.dissolve_mask_v,
        mask: ext.dissolve_mask.clone(),
        island: ext.dissolve_island.clone(),
        at_one: ext.dissolve_params.w > 0.5,
    })
}
/// D292: whether the material draws with a dissolve now (cheap form of [`dissolve`]).
pub fn dissolve_on(material: &ViewerMaterial) -> bool {
    material.extension.dissolve.truncate() != Vec3::ZERO
}
/// D284: the input of the material's bucket that holds raw UV0 (see `raw_uv0`).
pub fn raw_uv0(material: &ViewerMaterial) -> u8 {
    material.extension.raw_uv0
}
/// D278: sets (Some) or takes off (None) the top-down dissolve. D284: both masks are
/// sampled at the bucket's raw UV0 (`raw_uv0`, set when the bucket is built); a bucket
/// without it lays the transition mask over the position (`mask_u`, `mask_v`) and has no
/// `pod_mask`. Without a mask the band under the line is not tinted. The alpha mode is
/// not touched: the cut is a `discard` in the opaque pass, as the alpha test's.
/// D296: while it is on, back faces are culled, as the game's tent and platform draws do
/// (FOUND: cull back on all four captured pod draws). Double-sided, the inside of a post
/// shows through the cut with a second line on its far wall.
pub fn set_dissolve(material: &mut ViewerMaterial, dissolve: Option<&Dissolve>) {
    material.base.cull_mode = dissolve.map(|_| Face::Back);
    let ext = &mut material.extension;
    match dissolve {
        Some(d) => {
            ext.dissolve = d.plane;
            ext.dissolve_colour = d.colour.extend(1.);
            ext.dissolve_consts = d.consts;
            ext.dissolve_mask_u = d.mask_u;
            ext.dissolve_mask_v = d.mask_v;
            ext.dissolve_mask.clone_from(&d.mask);
            ext.dissolve_island.clone_from(&d.island);
            let flag = |on: bool| f32::from(u8::from(on));
            ext.dissolve_params = Vec4::new(
                f32::from(ext.raw_uv0.min(2)),
                flag(d.mask.is_some()),
                flag(d.island.is_some()),
                flag(d.at_one),
            );
        }
        None => {
            ext.dissolve = Vec4::ZERO;
            ext.dissolve_colour = Vec4::ZERO;
            ext.dissolve_consts = Vec4::ZERO;
            ext.dissolve_mask_u = Vec4::ZERO;
            ext.dissolve_mask_v = Vec4::ZERO;
            ext.dissolve_mask = None;
            ext.dissolve_island = None;
            ext.dissolve_params = Vec4::ZERO;
        }
    }
}
pub struct DualPlugin;
impl Plugin for DualPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ViewerMaterial>::default());
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                SHADER.id(),
                Shader::from_wgsl(SOURCE, "authored-dual-preview.wgsl"),
            )
            .unwrap();
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                GLASS_DUAL_SHADER.id(),
                Shader::from_wgsl(
                    format!("enable dual_source_blending;\n{SOURCE}"),
                    "authored-dual-preview-glass.wgsl",
                ),
            )
            .unwrap();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(RenderStartup, note_dual_source);
        }
    }
}
// Independently authored preview shader; imported Bevy interfaces are pinned by Cargo.lock.
const SOURCE: &str = r#"
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::{forward_io::{VertexOutput, FragmentOutput}, pbr_fragment::pbr_input_from_standard_material, pbr_functions::main_pass_post_lighting_processing, shadows::fetch_directional_shadow}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var second_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var second_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> enabled: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var third_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var third_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var fourth_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var fourth_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var<uniform> third_transform: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var<uniform> fourth_transform: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var<uniform> light_dir: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var<uniform> light_key: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var<uniform> light_ambient: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var<uniform> rim_low: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var<uniform> rim_high: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(114) var bump_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(115) var bump_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(116) var<uniform> bump_params: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(117) var<uniform> gloss: vec4<f32>;
// P2q: screen-space derivatives for the bump frame, taken in uniform control flow.
struct BumpInput {
    dp1: vec3<f32>,
    dp2: vec3<f32>,
    duv1: vec2<f32>,
    duv2: vec2<f32>,
    offset: vec2<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(118) var<uniform> lbp_fog: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(119) var<uniform> lbp_fog_color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(120) var graph_image3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(121) var graph_sampler3: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(122) var graph_image4: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(123) var graph_sampler4: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(124) var graph_image5: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(125) var graph_sampler5: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(126) var<uniform> graph_info: vec4<u32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(127) var<uniform> graph_code: array<vec4<u32>, 32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(128) var<uniform> graph_param: array<vec4<f32>, 32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(140) var cube_image: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(141) var cube_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(129) var lightmap_image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(130) var lightmap_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(131) var<uniform> ao_params: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(132) var<uniform> thing_tint: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(133) var<uniform> dissolve: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(134) var<uniform> dissolve_params: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(135) var dissolve_mask: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(136) var dissolve_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(137) var<uniform> dissolve_colour: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(138) var<uniform> dissolve_consts: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(139) var<uniform> dissolve_mask_u: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(142) var<uniform> dissolve_mask_v: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(143) var dissolve_island: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(144) var dissolve_island_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(145) var<uniform> glass: vec4<f32>;
// D292: Bevy's alpha-mode bits of StandardMaterial flags (pbr_types.wgsl, pinned 0.19.1).
const ALPHA_MODE_BITS: u32 = 7u << 29u;
const ALPHA_MODE_OPAQUE: u32 = 0u;
// D189 graph texture k at uv with explicit gradients (non-uniform control flow).
fn graph_texture(k: u32, uv: vec2<f32>, gx: vec2<f32>, gy: vec2<f32>) -> vec4<f32> {
    switch k {
        case 0u: { return textureSampleGrad(second_image, second_sampler, uv, gx, gy); }
        case 1u: { return textureSampleGrad(third_image, third_sampler, uv, gx, gy); }
        case 2u: { return textureSampleGrad(fourth_image, fourth_sampler, uv, gx, gy); }
        case 3u: { return textureSampleGrad(graph_image3, graph_sampler3, uv, gx, gy); }
        case 4u: { return textureSampleGrad(graph_image4, graph_sampler4, uv, gx, gy); }
        default: { return textureSampleGrad(graph_image5, graph_sampler5, uv, gx, gy); }
    }
}
// D189 input: step (code >> 3), lane 0 = all four, 1..4 = broadcast lane - 1.
fn graph_lane(v: vec4<f32>, code: u32) -> vec4<f32> {
    let lane = code & 7u;
    if lane == 0u {
        return v;
    }
    return vec4<f32>(v[min(lane - 1u, 3u)]);
}
// D189 per-fragment D100 program (mirrors lbp_preview::evaluate_fragment): linear texels,
// MULTIPLY/ADD per lane, MIX (r - l) * f + l, BLEND l + (r - l) * clamp((m.x - p0)/(p1 - p0)).
// P3l: `dir` and its gradients feed sample steps on channel 2 (the cube map); other callers pass zero.
fn graph_color(count_in: u32, uv0: vec2<f32>, uv1: vec2<f32>, d0: vec4<f32>, d1: vec4<f32>, dir: vec3<f32>, ddx3: vec3<f32>, ddy3: vec3<f32>) -> vec4<f32> {
    var values: array<vec4<f32>, 32>;
    let count = min(count_in, 32u);
    for (var k = 0u; k < count; k++) {
        let code = graph_code[k];
        let p = graph_param[k];
        let a = graph_lane(values[min(code.y >> 3u, 31u)], code.y);
        let b = graph_lane(values[min(code.z >> 3u, 31u)], code.z);
        let c = graph_lane(values[min(code.w >> 3u, 31u)], code.w);
        var v = vec4<f32>(0.0);
        switch code.x {
            case 1u: {
                if code.z == 2u {
                    v = textureSampleGrad(cube_image, cube_sampler, dir, ddx3, ddy3);
                } else {
                    var base = uv0;
                    var d = d0;
                    if code.z == 1u {
                        base = uv1;
                        d = d1;
                    }
                    v = graph_texture(code.y, base * p.xy + p.zw, d.xy * p.xy, d.zw * p.xy);
                }
            }
            case 2u: { v = p; }
            case 3u: { v = a * b; }
            case 4u: { v = a + b; }
            case 5u: {
                var f = c;
                if code.w == 0xffffffffu {
                    f = vec4<f32>(p.x);
                }
                v = (b - a) * f + a;
            }
            case 6u: { v = (b - a) * clamp((c.x - p.x) / (p.y - p.x), 0.0, 1.0) + a; }
            default: {}
        }
        values[k] = v;
    }
    return values[max(count, 1u) - 1u];
}
// D121 preview Lambert factor (mirrors lighting::PreviewLight::factor/facing_normal).
// Double-sided: the smooth normal is turned into the hemisphere of the face normal that
// points at the viewer, so back faces and mirrored instances light like front faces.
fn preview_light(n: vec3<f32>, to_view: vec3<f32>, shadow: f32, ao: f32, baked: f32) -> vec3<f32> {
    if dot(n, n) <= 0.0 {
        return vec3<f32>(1.0);
    }
    if light_dir.w > 1.5 {
        // P2l (mirrors lighting::PreviewLight::lbp_factor): hemisphere ambient, sun,
        // grazing rim lerped by N.y (P2n: P2m's eye-facing rim reverted, D203); squared
        // for the game's square-then-sqrt combine.
        let rim = pow(clamp(dot(n, -to_view) + 0.9, 0.0, 1.0), 2.0);
        // P3k (fpdec v0/v3, CONFIRMED arithmetic): ambient x ao, sun x shadowmap.x x ao,
        // rim x shadowmap.x. `baked` is the lightmap texel as shadowmap.x (INFERRED binding).
        let f = light_ambient.rgb * (n.y * 0.15 + 0.5) * ao
            + light_key.rgb * (max(dot(n, light_dir.xyz), 0.0) * shadow * baked * ao)
            + mix(rim_low.rgb, rim_high.rgb, (n.y + 1.0) * 0.5) * rim * baked;
        return f * f;
    }
    return light_ambient.rgb + light_key.rgb * max(dot(n, light_dir.xyz), 0.0);
}
// P2q gloss (LBP mode only), brdf.cg combine `2 * light.w * light^2 * spec`: light.w
// sums the game's per-light scalars with the same weights as the colours, light^2 is
// the squared factor, spec = gloss colour x pow(saturate(R.L), CosinePower) with R the
// mirrored view ray.
fn gloss_term(n: vec3<f32>, to_view: vec3<f32>, factor: vec3<f32>, tint: vec3<f32>, shadow: f32) -> vec3<f32> {
    if light_dir.w < 1.5 || gloss.w <= 0.0 || dot(n, n) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let rim = pow(clamp(dot(n, -to_view) + 0.9, 0.0, 1.0), 2.0);
    let w = light_ambient.w * (n.y * 0.15 + 0.5)
        + light_key.w * (max(dot(n, light_dir.xyz), 0.0) * shadow)
        + mix(rim_low.w, rim_high.w, (n.y + 1.0) * 0.5) * rim;
    let r = reflect(-to_view, n);
    let spec = gloss.rgb * tint * (pow(clamp(dot(r, light_dir.xyz), 0.0, 1.0), gloss.w) * shadow);
    return 2.0 * w * factor * spec;
}
// D287 (part 38): the glass's specular (the captured program, LBP mode only): light^2 x
// (pow(saturate(R.L), CosinePower) + saturate(rim / 2)) x gloss colour x glass.z, added
// to the encoded output after the alpha; no shadow (none bound in the game's draw).
fn glass_specular(n: vec3<f32>, to_view: vec3<f32>, factor: vec3<f32>) -> vec3<f32> {
    if light_dir.w < 1.5 || gloss.w <= 0.0 || dot(n, n) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let rim = pow(clamp(dot(n, -to_view) + 0.9, 0.0, 1.0), 2.0);
    let r = reflect(-to_view, n);
    let spec = pow(clamp(dot(r, light_dir.xyz), 0.0, 1.0), gloss.w) + clamp(rim * 0.5, 0.0, 1.0);
    return glass.z * gloss.rgb * factor * spec;
}
// D287: sRGB transfer of the viewer's target, clamped to 0..1.
fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let high = 1.055 * pow(v, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, v * 12.92, v <= vec3<f32>(0.0031308));
}
fn srgb_decode(e: vec3<f32>) -> vec3<f32> {
    let v = clamp(e, vec3<f32>(0.0), vec3<f32>(1.0));
    let high = pow((v + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, v / 12.92, v <= vec3<f32>(0.04045));
}
// D287: the game blends the glass in its encoded output, out = s + (1 - a) d. This
// target blends linear light, out = S + T D (One, OneMinusSrcAlpha, one T), so S and T
// are the secant of the game's blend through the encoded destinations 0.35 and 0.75:
// T the mean of the three slopes, S the mean residual. Returns (S, 1 - T). D290: the
// fallback, used only where the device has no dual-source blending.
fn glass_blend(s: vec3<f32>, a: f32) -> vec4<f32> {
    let l1 = srgb_decode(vec3<f32>(0.35));
    let l2 = srgb_decode(vec3<f32>(0.75));
    let y1 = srgb_decode(s + (1.0 - a) * 0.35);
    let y2 = srgb_decode(s + (1.0 - a) * 0.75);
    let slope = (y2 - y1) / (l2 - l1);
    let t = clamp((slope.x + slope.y + slope.z) / 3.0, 0.0, 1.0);
    let c = max(0.5 * ((y1 - t * l1) + (y2 - t * l2)), vec3<f32>(0.0));
    return vec4<f32>(c, 1.0 - t);
}
// D290: the same secant per channel, for dual-source blending (out = S + (1 - src1) D):
// t the three slopes, each clamped to 0..1, S the residuals. Returns S and the cover 1 - t.
struct GlassSecant {
    color: vec3<f32>,
    cover: vec3<f32>,
}
fn glass_secant(s: vec3<f32>, a: f32) -> GlassSecant {
    let l1 = srgb_decode(vec3<f32>(0.35));
    let l2 = srgb_decode(vec3<f32>(0.75));
    let y1 = srgb_decode(s + (1.0 - a) * 0.35);
    let y2 = srgb_decode(s + (1.0 - a) * 0.75);
    let t = clamp((y2 - y1) / (l2 - l1), vec3<f32>(0.0), vec3<f32>(1.0));
    let c = max(0.5 * ((y1 - t * l1) + (y2 - t * l2)), vec3<f32>(0.0));
    return GlassSecant(c, vec3<f32>(1.0) - t);
}
#ifdef GLASS_DUAL_SOURCE
// D290: the glass's two blend sources at location 0: the colour and the cover 1 - t
// per channel (1 = opaque).
struct GlassOutput {
    @location(0) @blend_src(0) color: vec4<f32>,
    @location(0) @blend_src(1) cover: vec4<f32>,
}
#endif
// Facing normal, tilted by the bump offset in a cotangent frame built from the
// position and UV1 derivatives (LBP mode with a bound bump map only).
fn lit_normal(shading: vec3<f32>, face: vec3<f32>, to_view: vec3<f32>, bump: BumpInput) -> vec3<f32> {
    if dot(shading, shading) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n = facing_normal(shading, face, to_view);
    if light_dir.w < 1.5 || bump_params.y < 0.5 {
        return n;
    }
    let dp2perp = cross(bump.dp2, n);
    let dp1perp = cross(n, bump.dp1);
    let t = dp2perp * bump.duv1.x + dp1perp * bump.duv2.x;
    let b = dp2perp * bump.duv1.y + dp1perp * bump.duv2.y;
    let size = max(dot(t, t), dot(b, b));
    if size <= 1e-20 {
        return n;
    }
    let scale = inverseSqrt(size);
    return normalize(n + (t * bump.offset.x + b * bump.offset.y) * scale);
}
// P2r (RSX capture, all lit Pod draws): the game writes sqrt(colour x tintcol) with
// tintcol (4,4,4) into a plain RGBA8 target, so that sqrt is the only output gamma.
// Bevy sRGB-encodes our linear output, so return the linear value whose encoding is it.
const GAME_TINT: f32 = 4.0;
// D291: the game's encoded colour (its 8-bit target's value) before the fog.
fn lbp_encode(c: vec3<f32>) -> vec3<f32> {
    return clamp(sqrt(max(c * GAME_TINT, vec3<f32>(0.0))), vec3<f32>(0.0), vec3<f32>(1.0));
}
// D307: the fog's share at a view depth (the game's sat(TC3.z)), 0 without fog.
fn lbp_fog_share(depth: f32) -> f32 {
    if lbp_fog.w > 0.5 {
        return clamp((depth - lbp_fog.x) * lbp_fog.y, 0.0, 1.0);
    }
    return 0.0;
}
// P2t: then the game's fog in that output space, by view depth (lbp_fog).
fn lbp_fog_encoded(e: vec3<f32>, depth: f32) -> vec3<f32> {
    var v = e;
    if lbp_fog.w > 0.5 {
        let k = lbp_fog_share(depth);
        let fogged = (dot(v, vec3<f32>(0.3, 0.6, 0.1)) * lbp_fog.z + 1.0) * lbp_fog_color.rgb;
        v = clamp(mix(v, fogged, k), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return v;
}
// D303 FOUND (frame 5247, the lamp bulb's and the stars' program): a glowing material's
// alpha is tintcol.w / (4 dot(g, g) + 1), g the glow added before the x4 and the square
// root (tintcol.w = 1). D307 FOUND (the decoded programs): most programs then mix it
// towards 1 by the fog's share, a + f (1 - a); the bulb's and the stars' do not.
fn lbp_glow_alpha(g: vec3<f32>) -> f32 {
    return 1.0 / (1.0 + 4.0 * dot(g, g));
}
fn lbp_output(c: vec3<f32>, depth: f32) -> vec3<f32> {
    return srgb_decode(lbp_fog_encoded(lbp_encode(c), depth));
}
// D291: the game's pod_card_edge mixes the flat colour and the lit colour in its encoded
// output, out = mix(COL0, lit, t), and fogs after the mix. The viewer's encoded value is
// the sRGB encoding of its linear output (P2r, D287), and the flat colour's encoding is
// the game's COL0 (D285), so the mix is taken on the encoded pair. A whole kept share
// returns the lit colour exactly.
fn dissolve_mix(lit_encoded: vec3<f32>, keep: f32) -> vec3<f32> {
    if keep >= 1.0 {
        return lit_encoded;
    }
    return mix(srgb_encode(dissolve_colour.rgb), lit_encoded, keep);
}
fn facing_normal(shading: vec3<f32>, face: vec3<f32>, to_view: vec3<f32>) -> vec3<f32> {
    var n = normalize(shading);
    if dot(face, face) > 0.0 {
        var f = normalize(face);
        if dot(f, to_view) < 0.0 {
            f = -f;
        }
        if dot(n, f) < 0.0 {
            n = -n;
        }
    }
    return n;
}
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool)
#ifdef GLASS_DUAL_SOURCE
    -> GlassOutput {
#else
    -> FragmentOutput {
#endif
    let pbr = pbr_input_from_standard_material(in, front);
    // Screen-space derivatives stay in uniform control flow.
    let dp1 = dpdx(in.world_position.xyz);
    let dp2 = dpdy(in.world_position.xyz);
    let face = cross(dp1, dp2);
    var bump = BumpInput(dp1, dp2, vec2<f32>(0.0), vec2<f32>(0.0), vec2<f32>(0.0));
#ifdef VERTEX_UVS_B
    bump.duv1 = dpdx(in.uv_b);
    bump.duv2 = dpdy(in.uv_b);
    if bump_params.y > 0.5 {
        let texel = textureSample(bump_image, bump_sampler, in.uv_b);
        bump.offset = (texel.yw - vec2<f32>(0.502)) * bump_params.x;
    }
#endif
    var color = pbr.material.base_color;
    // D278: the dissolve mask's coordinates (in tiles) and their gradients, taken here in
    // uniform control flow: from the position, or (D284) from the input that holds the
    // mesh's raw UV0, which `pod_mask` is sampled at too.
    var dissolve_uv = vec2<f32>(
        dot(dissolve_mask_u.xyz, in.world_position.xyz) + dissolve_mask_u.w,
        dot(dissolve_mask_v.xyz, in.world_position.xyz) + dissolve_mask_v.w);
    var raw_uv0 = vec2<f32>(0.0);
    var has_raw_uv0 = false;
#ifdef VERTEX_UVS_A
    if dissolve_params.x > 0.5 && dissolve_params.x < 1.5 {
        raw_uv0 = in.uv;
        has_raw_uv0 = true;
    }
#endif
#ifdef VERTEX_UVS_B
    if dissolve_params.x > 1.5 {
        raw_uv0 = in.uv_b;
        has_raw_uv0 = true;
    }
#endif
    if has_raw_uv0 {
        dissolve_uv = raw_uv0 * dissolve_consts.w;
    }
    let dissolve_dx = dpdx(dissolve_uv);
    let dissolve_dy = dpdy(dissolve_uv);
    let raw_uv0_dx = dpdx(raw_uv0);
    let raw_uv0_dy = dpdy(raw_uv0);
#ifdef VERTEX_UVS_A
#ifdef VERTEX_UVS_B
    // D189: gradients taken here, in uniform control flow, for textureSampleGrad.
    let graph_d0 = vec4<f32>(dpdx(in.uv), dpdy(in.uv));
    let graph_d1 = vec4<f32>(dpdx(in.uv_b), dpdy(in.uv_b));
    if graph_info.x > 0u {
        let g = graph_color(graph_info.x, in.uv, in.uv_b, graph_d0, graph_d1, vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
        color = vec4<f32>(clamp(g.rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
    }
#endif
#endif
#ifdef VERTEX_UVS_B
    if enabled.x > 0.0 {
        let second = textureSample(second_image, second_sampler, in.uv_b);
        if enabled.w > 0.5 {
            // Toolkit MIX: (port1 - port0) * port1.a + port0, on linear values.
            color = mix(color, second, second.a);
        } else {
            color *= second;
        }
        if enabled.y > 0.0 {
            var third_base = in.uv_b;
#ifdef VERTEX_UVS_A
            if enabled.y > 1.5 {
                third_base = in.uv;
            }
#endif
            color *= textureSample(third_image, third_sampler, third_base * third_transform.xy + third_transform.zw);
        }
#ifdef VERTEX_UVS_A
        if enabled.z > 0.0 {
            var fourth_base = in.uv;
            if enabled.z > 1.5 {
                fourth_base = in.uv_b;
            }
            color *= textureSample(fourth_image, fourth_sampler, fourth_base * fourth_transform.xy + fourth_transform.zw);
        }
#endif
    }
#endif
    if bump_params.z > 0.0 && color.a < bump_params.z {
        discard;
    }
    // D278: the top-down dissolve. `edge` is 1 on the line and 0 one band below it; the
    // line stands higher where the surface colour is greener. Above the line the fragment
    // is cut; in the band under it the colour goes toward the flat colour where the mask
    // is dark (`dissolve_keep` is the share of the lit colour kept). D284: where `pod_mask`
    // is white (m6 = 1) the line stands one band higher, and at value 1 (w) a band is
    // added where it is black: `pod_dissolve::edge`.
    let dissolve_on = dot(dissolve.xyz, dissolve.xyz) > 0.0;
    var dissolve_keep = 1.0;
    if dissolve_on {
        let above = dot(dissolve.xyz, in.world_position.xyz) + dissolve.w;
        var m6 = 0.0;
        if dissolve_params.z > 0.5 && has_raw_uv0 {
            m6 = step(0.5, textureSampleGrad(dissolve_island, dissolve_island_sampler, raw_uv0, raw_uv0_dx, raw_uv0_dy).r);
        }
        // D286: the glass takes a constant in place of the surface green.
        let green = select(color.g, glass.y, glass.x > 0.0);
        let edge = (above - dissolve_consts.x * green) / dissolve_consts.y + 1.0 - m6 + dissolve_params.w * (1.0 - m6);
        if edge > 1.0 {
            discard;
        }
        var mask = 1.0;
        if dissolve_params.y > 0.5 {
            mask = textureSampleGrad(dissolve_mask, dissolve_sampler, dissolve_uv, dissolve_dx, dissolve_dy).r;
        }
        dissolve_keep = clamp(1.0 - edge + mask, 0.0, 1.0);
        if edge > dissolve_consts.z {
            dissolve_keep = 0.0;
        }
    }
    var rgb = color.rgb;
    var glass_spec = vec3<f32>(0.0);
    // D303: the alpha a glowing surface writes (1 without glow).
    var glow_alpha = 1.0;
    var dissolve_mixed = false;
    // D268: a level script's render-mesh tint (INFERRED: a multiply before lighting).
    if thing_tint.w > 0.5 {
        rgb *= thing_tint.rgb;
    }
    if light_dir.w > 0.5 {
        // P2s: mode 3 (untrusted smooth normals) shades with the face normal.
        let shading = select(in.world_normal, face, light_dir.w > 2.5);
        let n = lit_normal(shading, face, pbr.V, bump);
        // P3g: Bevy's cascaded sun-shadow map (one directional light, set up by the viewer),
        // LBP-lit level meshes only; 1 = lit.
        var shadow = 1.0;
        if light_dir.w > 1.5 && (graph_info.z & 1u) > 0u {
            let view_z = (view.view_from_world * vec4<f32>(in.world_position.xyz, 1.0)).z;
            shadow = fetch_directional_shadow(0u, in.world_position, in.world_normal, view_z, in.position.xy);
        }
        // P3k: backdrop lightmap (B8, UV1). A B8 texel reads as A = one in RPCS3, so aomap.w is
        // 1 and ao = bakedAOScale + bakedAOBias for the whole mesh; the texel reaches the
        // light as shadowmap.x (sun and rim). Which unit the lightmap binds to is INFERRED
        // (PC review of #117: reading the texel as aomap.w darkened Bravery to x0.2-0.45).
        var ao = 1.0;
        var baked = 1.0;
#ifdef VERTEX_UVS_B
        if ao_params.z > 0.5 && light_dir.w > 1.5 {
            ao = ao_params.x + ao_params.y;
            baked = textureSample(lightmap_image, lightmap_sampler, in.uv_b).r;
        }
#endif
        let factor = preview_light(n, pbr.V, shadow, ao, baked);
        // P3d: gloss colour map (port 2 sampler) on graph slot 5, unused without a graph.
        var spec_tint = vec3<f32>(1.0);
#ifdef VERTEX_UVS_A
        if graph_info.y == 1u {
            spec_tint = textureSample(graph_image5, graph_sampler5, in.uv).rgb;
        }
#endif
#ifdef VERTEX_UVS_B
        if graph_info.y == 2u {
            spec_tint = textureSample(graph_image5, graph_sampler5, in.uv_b).rgb;
        }
#endif
        if glass.x > 0.0 {
            // D287: the glass's specular goes in after its alpha (below), not here.
            glass_spec = glass_specular(n, pbr.V, factor);
            rgb = rgb * factor;
        } else {
            rgb = rgb * factor + gloss_term(n, pbr.V, factor, spec_tint, shadow * baked);
        }
        // P3i: reflection graph sampled at the world-orientation reflection vector (the game's
        // TC4 is the camera-translated world position, not view-rotated; world axes INFERRED to
        // match ours), scaled by the luminance of the lit light.
        let refl_count = graph_info.z >> 8u;
        if light_dir.w > 1.5 && refl_count > 0u && graph_info.x == 0u && dot(n, n) > 0.0 {
            let rv = reflect(-pbr.V, n);
            let bias = exp2(graph_param[31].w);
            let gx = dpdx(rv.xy) * bias;
            let gy = dpdy(rv.xy) * bias;
            let d = vec4<f32>(gx.x, gx.y, gy.x, gy.y);
            let refl = graph_color(refl_count, rv.xy, rv.xy, d, d, rv, dpdx(rv) * bias, dpdy(rv) * bias);
            rgb += refl.rgb * dot(factor, vec3<f32>(0.299, 0.587, 0.114));
        }
        // P3h: glow is added unlit before the square-root output (INFERRED scale 1).
        let glow_mode = graph_info.w & 15u;
        if light_dir.w > 1.5 && glow_mode > 0u && graph_info.x == 0u {
            var glow = graph_param[31].rgb;
#ifdef VERTEX_UVS_A
            if glow_mode == 2u {
                glow *= textureSample(graph_image4, graph_sampler4, in.uv).rgb;
            }
#endif
#ifdef VERTEX_UVS_B
            if glow_mode == 3u {
                glow *= textureSample(graph_image4, graph_sampler4, in.uv_b).rgb;
            }
#endif
            rgb += glow;
            glow_alpha = lbp_glow_alpha(glow);
        }
        if light_dir.w > 1.5 {
            let depth = -(view.view_from_world * vec4<f32>(in.world_position.xyz, 1.0)).z;
            // D307: the common glow program lifts its alpha towards 1 by the fog.
            if glow_mode > 0u && (graph_info.w & 16u) == 0u {
                glow_alpha = mix(glow_alpha, 1.0, lbp_fog_share(depth));
            }
            if dissolve_on {
                // D291: the mix in the encoded output, then the fog, as the game does.
                rgb = srgb_decode(lbp_fog_encoded(dissolve_mix(lbp_encode(rgb), dissolve_keep), depth));
                dissolve_mixed = true;
            } else {
                rgb = lbp_output(rgb, depth);
            }
        }
    }
    // D291: the other routes (no fog of the game's) mix the encoded pair too; where the
    // lit colour is kept whole it is left as it is.
    if dissolve_on && !dissolve_mixed && dissolve_keep < 1.0 {
        rgb = srgb_decode(dissolve_mix(srgb_encode(rgb), dissolve_keep));
    }
    // D286: the glass's alpha = opacity x the dissolve's kept share; the rest alpha 1.
    // D287: as the game, the encoded colour times that alpha plus the specular, blended
    // as in encoded space (glass_blend), written premultiplied.
    // D290: with dual sources (the glass's pipeline only) a transmittance per channel.
    var alpha = 1.0;
    // D303: an opaque glowing surface writes its glow alpha (the game's tintcol.w / (1 +
    // 4 |g|^2)); the glow post pass and the final pass read it (glow_post.rs).
    if (pbr.material.flags & ALPHA_MODE_BITS) == ALPHA_MODE_OPAQUE {
        alpha = glow_alpha;
    }
    // D292: an opaque surface under the dissolve writes the kept share as its alpha (the
    // game's pod_card_edge: alpha = t x texalpha; 0 on the line). The opaque pipeline
    // has no blend, so the target stores it as is; the edge glow reads it (glow_post.rs).
    // Blended draws (decals) keep alpha 1, the glass its own below.
    if dissolve_on && (pbr.material.flags & ALPHA_MODE_BITS) == ALPHA_MODE_OPAQUE {
        alpha = dissolve_keep * glow_alpha;
    }
#ifdef GLASS_DUAL_SOURCE
    var cover = vec3<f32>(1.0);
#endif
    if glass.x > 0.0 {
        alpha = glass.x * dissolve_keep;
#ifdef GLASS_DUAL_SOURCE
        let secant = glass_secant(alpha * srgb_encode(rgb) + glass_spec, alpha);
        rgb = secant.color;
        cover = secant.cover;
        alpha = 1.0;
#else
        let blended = glass_blend(alpha * srgb_encode(rgb) + glass_spec, alpha);
        rgb = blended.rgb;
        alpha = blended.a;
#endif
    }
#ifdef GLASS_DUAL_SOURCE
    var out: GlassOutput;
    out.color = main_pass_post_lighting_processing(pbr, vec4<f32>(rgb, alpha));
    out.cover = vec4<f32>(cover, 1.0);
#else
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr, vec4<f32>(rgb, alpha));
#endif
    return out;
}
"#;
#[cfg(test)]
mod tests {
    /// Stand-ins for the Bevy imports (same names and the fields this shader uses).
    const STUBS: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_b: vec2<f32>,
}
struct ViewStub {
    view_from_world: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> view: ViewStub;
struct FragmentOutput {
    @location(0) color: vec4<f32>,
}
struct StandardMaterialStub {
    base_color: vec4<f32>,
    flags: u32,
}
struct PbrInputStub {
    material: StandardMaterialStub,
    V: vec3<f32>,
}
fn pbr_input_from_standard_material(in: VertexOutput, front: bool) -> PbrInputStub {
    var p: PbrInputStub;
    p.material.base_color = vec4<f32>(1.0);
    p.material.flags = 0u;
    p.V = vec3<f32>(0.0, 0.0, 1.0);
    return p;
}
fn fetch_directional_shadow(id: u32, p: vec4<f32>, n: vec3<f32>, z: f32, xy: vec2<f32>) -> f32 {
    return 1.0;
}
fn main_pass_post_lighting_processing(p: PbrInputStub, c: vec4<f32>) -> vec4<f32> {
    return c;
}
"#;
    /// P3h: glow rides graph_info.w (1 = colour, 2/3 = map on UV0/UV1) and the last param row.
    #[test]
    fn glow_uses_the_free_graph_slots() {
        use bevy::prelude::*;
        let mut m = super::material(StandardMaterial::default());
        super::enable_glow(&mut m, [0.2, 0.4, 0.6], None, true);
        assert_eq!(m.extension.graph_info.w, 1);
        assert_eq!(m.extension.graph_param[31], Vec4::new(0.2, 0.4, 0.6, 0.));
        assert!(m.extension.graph_image4.is_none());
        let h = Handle::<Image>::default();
        super::enable_glow(&mut m, [1.; 3], Some((h, 3)), true);
        assert_eq!(m.extension.graph_info.w, 3);
        assert!(m.extension.graph_image4.is_some());
    }
    /// P3i: reflection packs its step count above the shadow bit and its blur next to glow.
    #[test]
    fn reflection_packs_beside_shadow_and_glow() {
        use bevy::prelude::*;
        let mut m = super::material(StandardMaterial::default());
        m.extension.graph_info.z = 1; // shadow bit
        let step = lbp_preview::FragmentStep {
            code: [2, 0, 0, 0],
            param: [1.; 4],
        };
        let h = Handle::<Image>::default();
        super::enable_reflection(&mut m, vec![Some(h.clone())], None, &[step, step], 0.45).unwrap();
        assert_eq!(m.extension.graph_info.z, 1 | (2 << 8));
        assert_eq!(m.extension.graph_param[31].w, 0.45);
        super::enable_glow(&mut m, [0.5; 3], None, true);
        assert_eq!(m.extension.graph_param[31].w, 0.45, "glow keeps the blur");
        assert!(
            super::enable_reflection(&mut m, vec![Some(h.clone()); 5], None, &[step], 0.).is_err()
        );
        // A cube map rides its own binding and leaves a placeholder in its graph slot.
        super::enable_reflection(&mut m, vec![None], Some(h), &[step], 0.).unwrap();
        assert!(m.extension.cube_image.is_some() && m.extension.image.is_none());
        // Lighting changes only the shadow bit.
        assert!(super::enable_reflection(&mut m, vec![], None, &[step], 0.).is_err());
    }
    /// D278: the dissolve is off on a new material; the setter writes the plane, the
    /// colour, the constants and the mask, says whether the bucket's UV is usable, and
    /// takes all of it off again. Nothing else of the material changes.
    /// D303: the glow alpha gives the game's bytes in frame 5247: the lamp bulb (glow
    /// (0.6627, 0.5608, 0)) 64, the stars (glow 1) 20; no glow 255.
    #[test]
    fn d303_glow_alpha_matches_the_game_bytes() {
        let byte = |g| (super::glow_alpha(g) * 255.).round() as u8;
        assert_eq!(byte([0.6627, 0.5608, 0.]), 64);
        assert_eq!(byte([1.; 3]), 20);
        assert_eq!(byte([0.; 3]), 255);
        // Monotonic in the glow's size, never above 1.
        let mut last = 1.;
        for k in 0..=20 {
            let a = super::glow_alpha([k as f32 * 0.05; 3]);
            assert!(a <= last && a > 0.);
            last = a;
        }
    }
    /// D307: the glow alpha under fog. The common program lifts it towards 1 by the fog's
    /// share (`a + f (1 - a)`); the 23 listed programs (the bulb, the stars, the checkpoint
    /// ring among them) keep `a`; without fog both are D303's.
    #[test]
    fn d307_glow_alpha_under_fog() {
        let ring = [1.; 3];
        let a = super::glow_alpha(ring);
        assert!((a - 1. / 13.).abs() < 1e-6);
        assert_eq!(super::glow_alpha_under_fog(ring, 0., true), a);
        assert_eq!(super::glow_alpha_under_fog(ring, 0.6, false), a);
        let half = super::glow_alpha_under_fog(ring, 0.5, true);
        assert!((half - (a + 0.5 * (1. - a))).abs() < 1e-6);
        assert_eq!(super::glow_alpha_under_fog(ring, 1., true), 1.);
        assert_eq!(super::glow_alpha_under_fog(ring, 2., true), 1.);
        // Monotonic in the fog, never below the unfogged alpha.
        let mut last = a;
        for k in 0..=10 {
            let v = super::glow_alpha_under_fog([0.6627, 0.5608, 0.], k as f32 * 0.1, true);
            assert!(v >= last - 1e-6 && v <= 1.);
            last = v;
        }
        for m in [0x0000_d6cc, 0x0000_6a49, 0x0000_be15] {
            assert!(!super::glow_alpha_fogged(Some(m)), "{m:08x}");
        }
        // proc_da_vinci_glow and an unknown material take the common form.
        assert!(super::glow_alpha_fogged(Some(0x0001_45b9)));
        assert!(super::glow_alpha_fogged(None));
        let mut sorted = super::GLOW_ALPHA_UNFOGGED;
        sorted.sort_unstable();
        assert!(sorted.windows(2).all(|w| w[0] < w[1]), "no duplicates");
    }
    /// D307: the unfogged bit rides graph_info.w above the glow mode.
    #[test]
    fn d307_glow_mode_keeps_the_unfogged_bit_apart() {
        use bevy::prelude::*;
        let mut m = super::material(StandardMaterial::default());
        super::enable_glow(&mut m, [1.; 3], None, false);
        assert_eq!(m.extension.graph_info.w, 1 | super::GLOW_ALPHA_UNFOGGED_BIT);
        assert_eq!(m.extension.graph_info.w & 15, 1);
        let h = Handle::<Image>::default();
        super::enable_glow(&mut m, [1.; 3], Some((h, 3)), true);
        assert_eq!(m.extension.graph_info.w, 3);
        // The glass keeps the destination's alpha.
        use bevy::render::render_resource::{BlendFactor, BlendOperation};
        let b = super::glass_alpha_blend();
        assert_eq!(
            (b.src_factor, b.dst_factor, b.operation),
            (BlendFactor::Zero, BlendFactor::One, BlendOperation::Add)
        );
    }
    #[test]
    fn d278_the_dissolve_setter_writes_and_clears() {
        use bevy::prelude::*;
        let mut m = super::material(StandardMaterial::default());
        super::set_thing_tint(&mut m, Some([0.5, 0.25, 1.]));
        assert!(super::dissolve(&m).is_none());
        assert_eq!(m.extension.dissolve, Vec4::ZERO);
        let mut d = super::Dissolve {
            plane: Vec4::new(0., 2., 0., -3.),
            colour: Vec3::new(0.1, 0.2, 0.3),
            consts: Vec4::new(100., 400., 0.99, 40.),
            mask_u: Vec4::new(1., 0., 1., 0.),
            mask_v: Vec4::new(0., 1., 0., 0.),
            mask: None,
            island: None,
            at_one: false,
        };
        // No raw UV0 and no mask: position coordinates, no tint.
        super::set_dissolve(&mut m, Some(&d));
        assert_eq!(super::dissolve(&m).as_ref(), Some(&d));
        assert_eq!(m.extension.dissolve_params, Vec4::ZERO);
        assert_eq!(m.extension.dissolve_colour, Vec4::new(0.1, 0.2, 0.3, 1.));
        // Raw UV0 in `in.uv` (D284; D278 took a diffuse texture for it) and a mask.
        m.extension.raw_uv0 = 1;
        d.mask = Some(Handle::default());
        super::set_dissolve(&mut m, Some(&d));
        assert_eq!(m.extension.dissolve_params, Vec4::new(1., 1., 0., 0.));
        assert_eq!(super::dissolve(&m).as_ref(), Some(&d));
        assert_eq!(m.base.alpha_mode, AlphaMode::Opaque);
        assert_eq!(m.base.cull_mode, Some(super::Face::Back));
        super::set_dissolve(&mut m, None);
        assert!(super::dissolve(&m).is_none() && m.extension.dissolve_mask.is_none());
        assert_eq!(m.base.cull_mode, None);
        assert_eq!(m.extension.dissolve_params, Vec4::ZERO);
        assert_eq!(m.extension.thing_tint, Vec4::new(0.5, 0.25, 1., 1.));
        assert!(super::tinted(&m));
        super::set_thing_tint(&mut m, None);
        assert!(!super::tinted(&m));
    }
    /// D284: the input that holds raw UV0 goes to the shader as x, `pod_mask` as z and
    /// value 1 as w; the dissolve read back has them, and taking it off clears them. A
    /// diffuse texture alone no longer says where the mask's coordinates are.
    #[test]
    fn d284_the_dissolve_carries_the_island_mask_and_the_uv_source() {
        use bevy::prelude::*;
        let mut m = super::material(StandardMaterial::default());
        m.base.base_color_texture = Some(Handle::default());
        let d = super::Dissolve {
            plane: Vec4::Y,
            mask: Some(Handle::default()),
            island: Some(Handle::default()),
            at_one: true,
            ..Default::default()
        };
        super::set_dissolve(&mut m, Some(&d));
        assert_eq!(m.extension.dissolve_params, Vec4::new(0., 1., 1., 1.));
        m.extension.raw_uv0 = 2;
        assert_eq!(super::raw_uv0(&m), 2);
        super::set_dissolve(&mut m, Some(&d));
        assert_eq!(m.extension.dissolve_params, Vec4::new(2., 1., 1., 1.));
        assert_eq!(super::dissolve(&m).as_ref(), Some(&d));
        let plain = super::Dissolve {
            island: None,
            at_one: false,
            ..d.clone()
        };
        assert_ne!(super::dissolve(&m).as_ref(), Some(&plain));
        super::set_dissolve(&mut m, Some(&plain));
        assert_eq!(m.extension.dissolve_params, Vec4::new(2., 1., 0., 0.));
        super::set_dissolve(&mut m, None);
        assert!(m.extension.dissolve_island.is_none());
        assert_eq!(m.extension.dissolve_params, Vec4::ZERO);
        assert_eq!(super::raw_uv0(&m), 2);
    }
    /// D286: a new material is not glass; `enable_glass` gives the shader the opacity and
    /// the dissolve's green, and leaves the rest of the material (its dissolve) alone.
    #[test]
    fn d286_glass_carries_its_opacity_and_green() {
        use bevy::prelude::*;
        let mut m = super::material(StandardMaterial::default());
        assert_eq!(super::glass(&m), None);
        assert_eq!(m.extension.glass, Vec4::ZERO);
        m.extension.raw_uv0 = 1;
        let d = super::Dissolve {
            plane: Vec4::Y,
            ..Default::default()
        };
        super::set_dissolve(&mut m, Some(&d));
        super::enable_glass(&mut m, 0.2, 0.117_65, 6.);
        assert_eq!(super::glass(&m), Some(0.2));
        assert_eq!(m.extension.glass, Vec4::new(0.2, 0.117_65, 6., 0.));
        assert_eq!(super::dissolve(&m).as_ref(), Some(&d));
        assert_eq!(m.extension.dissolve_params.x, 1.);
    }
    /// D287: the game's blend in its encoded output, out = s + (1 - a) d, saturating.
    fn d287_game(s: f32, a: f32, d: f32) -> f32 {
        (s + (1. - a) * d).min(1.)
    }
    /// D287: what the viewer's linear blend of `glass_blend`'s output gives, encoded.
    fn d287_viewer(s: [f32; 3], a: f32, d: [f32; 3]) -> [f32; 3] {
        let (c, alpha) = super::glass_blend(s, a);
        std::array::from_fn(|i| super::srgb_encode(c[i] + (1. - alpha) * super::srgb_decode(d[i])))
    }
    /// D287: the sRGB pair round-trips and is the standard curve.
    #[test]
    fn d287_srgb_pair_round_trips() {
        for i in 0..=255 {
            let v = i as f32 / 255.;
            assert!(
                (super::srgb_encode(super::srgb_decode(v)) - v).abs() < 1e-5,
                "{v}"
            );
        }
        assert!((super::srgb_decode(0.5) - 0.214_04).abs() < 1e-4);
        assert_eq!(super::srgb_decode(2.), 1.);
        assert_eq!(super::srgb_encode(-1.), 0.);
    }
    /// D287: the blend is exact at the two secant destinations for a grey glass colour;
    /// over the destinations 0.2 to 1 and glass colours like the measured ones (encoded s
    /// up to 0.15, alpha 0.2) it is on average within 2 / 255 of the game's encoded blend,
    /// where the linear blend D286 used is off by more than 7 / 255.
    #[test]
    fn d287_glass_blend_follows_the_encoded_blend() {
        let a = 0.2;
        for s in [0.0, 0.05, 0.1] {
            for d in super::GLASS_SECANT {
                let v = d287_viewer([s; 3], a, [d; 3]);
                for c in v {
                    assert!((c - d287_game(s, a, d)).abs() < 1e-4, "s {s} d {d}: {c}");
                }
            }
        }
        let (mut sum, mut d286_sum, mut n, mut worst) = (0., 0., 0., 0f32);
        for s in [
            [0.07, 0.09, 0.09],
            [0.03, 0.06, 0.08],
            [0.0, 0.0, 0.0],
            [0.15, 0.15, 0.15],
        ] {
            for k in 0..=40 {
                let d = 0.2 + 0.8 * k as f32 / 40.;
                let v = d287_viewer(s, a, [d; 3]);
                for i in 0..3 {
                    let err = (v[i] - d287_game(s[i], a, d)).abs();
                    worst = worst.max(err);
                    sum += err;
                    // D286: colour s / a premultiplied in linear light, alpha a.
                    let lin = a * super::srgb_decode(s[i] / a) + (1. - a) * super::srgb_decode(d);
                    d286_sum += (super::srgb_encode(lin) - d287_game(s[i], a, d)).abs();
                    n += 1.;
                }
            }
        }
        // Mean 1.7 / 255 (D286's 7.4); the worst, 12.5, is a tinted colour over a dark
        // destination (one alpha for the three channels).
        assert!(sum / n < 2. / 255., "mean {}", sum / n * 255.);
        assert!(
            d286_sum / n > 7. / 255.,
            "d286 mean {}",
            d286_sum / n * 255.
        );
        assert!(worst < 13. / 255., "worst {}", worst * 255.);
    }
    /// D287: the blend's output is a valid premultiplied pair: colour at least 0, alpha
    /// in 0..1, and a fully dissolved glass (alpha 0, no specular) leaves the room as is.
    #[test]
    fn d287_glass_blend_output_is_valid() {
        for s in [0.0, 0.1, 0.5, 1.0, 1.5] {
            for a in [0.0, 0.1, 0.2, 1.0] {
                let (c, alpha) = super::glass_blend([s; 3], a);
                assert!((0. ..=1.).contains(&alpha), "s {s} a {a}: {alpha}");
                assert!(c.iter().all(|&v| v >= 0.), "s {s} a {a}: {c:?}");
            }
        }
        let (c, alpha) = super::glass_blend([0.; 3], 0.);
        assert!(
            alpha.abs() < 1e-5 && c.iter().all(|v| v.abs() < 1e-5),
            "{c:?} {alpha}"
        );
    }
    /// D290: per channel the blend is exact at both secant destinations, also for a tinted
    /// glass colour, where one T for the three channels is not.
    #[test]
    fn d290_per_channel_secant_is_exact_at_both_points() {
        let a = 0.2;
        for s in [
            [0.0; 3],
            [0.03, 0.06, 0.08],
            [0.0, 0.05, 0.15],
            [0.07, 0.09, 0.09],
        ] {
            for d in super::GLASS_SECANT {
                let v = d290_viewer(s, a, [d; 3]);
                for i in 0..3 {
                    let game = d287_game(s[i], a, d);
                    assert!((v[i] - game).abs() < 1e-4, "s {s:?} d {d}: {v:?}");
                }
            }
        }
        // One T misses a tinted colour at the same points.
        let s = [0.0, 0.05, 0.15];
        let d = super::GLASS_SECANT[0];
        let v = d287_viewer(s, a, [d; 3]);
        assert!((v[0] - d287_game(s[0], a, d)).abs() > 3. / 255., "{v:?}");
    }
    /// D290: on a grey glass colour the three slopes are equal, so the per-channel blend
    /// is the D287 one: the same colour, and the cover equal to the alpha.
    #[test]
    fn d290_grey_glass_matches_the_mean_t_blend() {
        for s in [0.0, 0.05, 0.1, 0.3] {
            for a in [0.1, 0.2, 0.5] {
                let (c1, alpha) = super::glass_blend([s; 3], a);
                let (c2, cover) = super::glass_blend_per_channel([s; 3], a);
                for i in 0..3 {
                    assert!((c1[i] - c2[i]).abs() < 1e-6, "s {s} a {a}");
                    assert!((cover[i] - alpha).abs() < 1e-6, "s {s} a {a}");
                }
            }
        }
    }
    /// D290: the Pod's closed right panel (pod-glass notes 2, game share and room on the
    /// game's mask, of 255): per channel within 1 level of the game's s + 0.8 d, where one
    /// T is 4 to 5 off in red and blue; the left panel within 1.5 either way.
    #[test]
    fn d290_pod_panels_follow_the_game_per_channel() {
        let to = |v: [f32; 3]| v.map(|c| c / 255.);
        for (s, d, one_t_off) in [
            (to([7.3, 14.7, 20.8]), to([73.2, 86.0, 86.9]), true),
            (to([17.3, 21.9, 22.9]), to([145.1, 144.5, 110.7]), false),
        ] {
            let v = d290_viewer(s, 0.2, d);
            let one = d287_viewer(s, 0.2, d);
            for i in 0..3 {
                let game = d287_game(s[i], 0.2, d[i]);
                assert!((v[i] - game).abs() < 1.5 / 255., "{i}: {v:?}");
            }
            if one_t_off {
                assert!(
                    (one[0] - d287_game(s[0], 0.2, d[0])).abs() > 4. / 255.,
                    "{one:?}"
                );
                assert!(
                    (one[2] - d287_game(s[2], 0.2, d[2])).abs() > 4. / 255.,
                    "{one:?}"
                );
            }
        }
    }
    /// D290: over the destinations 0.2 to 1 and tinted glass colours the per-channel blend
    /// is on average within 1.5 / 255 of the game's (one T: 2.6) and at worst 9 (one T: 25).
    #[test]
    fn d290_per_channel_blend_beats_one_t_over_the_room() {
        let a = 0.2;
        let (mut sum, mut one_sum, mut n, mut worst) = (0., 0., 0., 0f32);
        for s in [
            [0.07, 0.09, 0.09],
            [0.03, 0.06, 0.08],
            [0.0, 0.0, 0.0],
            [0.15, 0.15, 0.15],
            [0.0, 0.05, 0.15],
        ] {
            for k in 0..=40 {
                let d = 0.2 + 0.8 * k as f32 / 40.;
                let v = d290_viewer(s, a, [d; 3]);
                let one = d287_viewer(s, a, [d; 3]);
                for i in 0..3 {
                    let err = (v[i] - d287_game(s[i], a, d)).abs();
                    worst = worst.max(err);
                    sum += err;
                    one_sum += (one[i] - d287_game(s[i], a, d)).abs();
                    n += 1.;
                }
            }
        }
        assert!(sum / n < 1.5 / 255., "mean {}", sum / n * 255.);
        assert!(
            one_sum / n > 2.5 / 255.,
            "one T mean {}",
            one_sum / n * 255.
        );
        assert!(worst < 9. / 255., "worst {}", worst * 255.);
    }
    /// D290: the per-channel output is a valid pair (colour at least 0, cover in 0..1),
    /// and a fully dissolved glass leaves the room as is.
    #[test]
    fn d290_per_channel_output_is_valid() {
        for s in [[0.0, 0.1, 0.5], [1.0, 1.5, 0.0], [0.2; 3]] {
            for a in [0.0, 0.1, 0.2, 1.0] {
                let (c, cover) = super::glass_blend_per_channel(s, a);
                assert!(cover.iter().all(|v| (0. ..=1.).contains(v)), "{s:?} {a}");
                assert!(c.iter().all(|&v| v >= 0.), "{s:?} {a}: {c:?}");
            }
        }
        let (c, cover) = super::glass_blend_per_channel([0.; 3], 0.);
        assert!(
            c.iter().chain(&cover).all(|v| v.abs() < 1e-5),
            "{c:?} {cover:?}"
        );
    }
    /// D290: only glass keys the dual-source pipeline, and its colour blend is One +
    /// OneMinusSrc1 (out = S + t D).
    #[test]
    fn d290_glass_keys_the_dual_source_pipeline() {
        use bevy::prelude::*;
        use bevy::render::render_resource::{BlendFactor, BlendOperation};
        let mut m = super::material(StandardMaterial::default());
        assert!(!super::SecondTextureKey::from(&m.extension).glass);
        super::enable_glass(&mut m, 0.2, 0.117_65, 6.);
        assert!(super::SecondTextureKey::from(&m.extension).glass);
        let blend = super::glass_dual_color_blend();
        assert_eq!(blend.src_factor, BlendFactor::One);
        assert_eq!(blend.dst_factor, BlendFactor::OneMinusSrc1);
        assert_eq!(blend.operation, BlendOperation::Add);
    }
    /// D290: what the viewer's dual-source blend of the per-channel output gives, encoded.
    fn d290_viewer(s: [f32; 3], a: f32, d: [f32; 3]) -> [f32; 3] {
        let (c, cover) = super::glass_blend_per_channel(s, a);
        std::array::from_fn(|i| {
            super::srgb_encode(c[i] + (1. - cover[i]) * super::srgb_decode(d[i]))
        })
    }
    /// D291: on the line (keep 0) the output is the flat colour, shown as the game's COL0
    /// bytes (88, 192, 14), whatever the lit colour.
    #[test]
    fn d291_the_line_shows_the_flat_colour() {
        let vc4 = [0.346_020_8, 0.751_111_2, 0.055_363_3];
        let flat = vc4.map(super::srgb_decode);
        for lit in [[0.0; 3], [0.2, 0.5, 0.9], [1.0; 3]] {
            let out = super::dissolve_mix(flat, lit, 0.);
            let bytes = out.map(|c| (super::srgb_encode(c) * 255.).round() as u8);
            assert_eq!(bytes, [88, 192, 14], "{lit:?}");
        }
    }
    /// D291: in the band the mix is the game's, in its encoded output: lit 0.2 and flat
    /// 0.75 (encoded) at keep 0.5 give 0.475, where the linear mix gave about 0.56.
    #[test]
    fn d291_the_band_mixes_in_the_encoded_output() {
        let (flat, lit) = (super::srgb_decode(0.75), super::srgb_decode(0.2));
        let out = super::dissolve_mix([flat; 3], [lit; 3], 0.5);
        for c in out {
            assert!((super::srgb_encode(c) - 0.475).abs() < 1e-4, "{c}");
        }
        let linear = super::srgb_encode(0.5 * (flat + lit));
        assert!((linear - 0.564).abs() < 1e-3, "{linear}");
        // Along the band, encoded output = the game's mix of the encoded pair.
        for k in 0..=10 {
            let keep = k as f32 / 10.;
            let e = super::srgb_encode(super::dissolve_mix([flat; 3], [lit; 3], keep)[0]);
            assert!(
                (e - (0.75 + (0.2 - 0.75) * keep)).abs() < 1e-4,
                "{keep}: {e}"
            );
        }
    }
    /// D291: where the lit colour is kept whole it is returned as it is (also over 1).
    #[test]
    fn d291_a_whole_kept_share_is_the_lit_colour() {
        let lit = [1.5, 0.3, 0.0];
        assert_eq!(super::dissolve_mix([0.1, 0.5, 0.0], lit, 1.), lit);
    }
    /// D290: the shader with `#ifdef`s resolved: every define on but `GLASS_DUAL_SOURCE`,
    /// which follows `dual`; `#import` lines dropped, the bind group fixed at 2.
    fn preprocess(dual: bool) -> String {
        let mut active = vec![true];
        let mut body = String::new();
        for line in super::SOURCE.lines() {
            let t = line.trim_start();
            if let Some(def) = t.strip_prefix("#ifdef ") {
                let on = def.trim() != "GLASS_DUAL_SOURCE" || dual;
                let parent = *active.last().unwrap();
                active.push(parent && on);
            } else if t.starts_with("#else") {
                let parent = active[active.len() - 2];
                let now = active.last_mut().unwrap();
                *now = parent && !*now;
            } else if t.starts_with("#endif") {
                active.pop();
            } else if t.starts_with('#') {
                assert!(t.starts_with("#import"), "{t}");
            } else if *active.last().unwrap() {
                body += &(line.replace("#{MATERIAL_BIND_GROUP}", "2") + "\n");
            }
        }
        assert_eq!(active.len(), 1);
        body
    }
    /// D290: the glass's dual-source variant parses and validates with the extension on.
    #[test]
    fn d290_dual_source_variant_validates() {
        let body = preprocess(true);
        assert!(body.contains("@location(0) @blend_src(1) cover: vec4<f32>,"));
        assert!(body.contains("glass_secant(alpha * srgb_encode(rgb) + glass_spec, alpha)"));
        assert!(body.contains("out.cover = vec4<f32>(cover, 1.0);"));
        assert!(!body.contains("glass_blend(alpha * srgb_encode(rgb)"));
        let source = format!("enable dual_source_blending;\n{STUBS}{body}");
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
    /// D121: the authored shader (all `#ifdef` blocks enabled but D290's dual source,
    /// imports stubbed) parses and validates headlessly. Bevy's own composition is
    /// exercised only at run time.
    #[test]
    fn authored_shader_validates_with_every_branch_enabled() {
        let body = preprocess(false);
        assert!(!body.contains("@blend_src"));
        assert!(body.contains("lit_normal(shading, face, pbr.V, bump)"));
        assert!(body.contains("preview_light(n, pbr.V, shadow, ao, baked)"));
        assert!(body.contains("gloss_term(n, pbr.V, factor, spec_tint, shadow * baked)"));
        assert!(body.contains("rgb = lbp_output(rgb, depth);"));
        assert!(body.contains("rgb += glow;"));
        assert!(body.contains("graph_color(refl_count, rv.xy, rv.xy, d, d, rv,"));
        assert!(body.contains("graph_color(graph_info.x, in.uv, in.uv_b, graph_d0, graph_d1, "));
        // D278: the dissolve's cut, its mask sample with explicit gradients and its tint.
        assert!(body.contains("dissolve_uv = raw_uv0 * dissolve_consts.w;"));
        // D284: the raw UV0 selector, pod_mask at raw UV0 with gradients, and the edge
        // with the island and value-1 terms.
        assert!(body.contains("raw_uv0 = in.uv;") && body.contains("raw_uv0 = in.uv_b;"));
        assert!(body.contains(
            "textureSampleGrad(dissolve_island, dissolve_island_sampler, raw_uv0, raw_uv0_dx, raw_uv0_dy).r"
        ));
        assert!(body.contains("+ 1.0 - m6 + dissolve_params.w * (1.0 - m6);"));
        assert!(body.contains(
            "textureSampleGrad(dissolve_mask, dissolve_sampler, dissolve_uv, dissolve_dx, dissolve_dy).r"
        ));
        // D291: the mix in the encoded output, before the fog on the LBP route.
        assert!(body.contains("return mix(srgb_encode(dissolve_colour.rgb), lit_encoded, keep);"));
        assert!(body.contains(
            "rgb = srgb_decode(lbp_fog_encoded(dissolve_mix(lbp_encode(rgb), dissolve_keep), depth));"
        ));
        assert!(body.contains("rgb = srgb_decode(dissolve_mix(srgb_encode(rgb), dissolve_keep));"));
        assert!(body.contains("return srgb_decode(lbp_fog_encoded(lbp_encode(c), depth));"));
        assert!(!body.contains("mix(dissolve_colour.rgb, rgb, dissolve_keep)"));
        // D286: the glass's green in the edge, its alpha and its premultiplied colour.
        assert!(body.contains("(above - dissolve_consts.x * green)"));
        assert!(body.contains("alpha = glass.x * dissolve_keep;"));
        // D292: opaque surfaces under the dissolve write the kept share as alpha, before
        // the glass sets its own.
        let keep = body
            .find(
                "if dissolve_on && (pbr.material.flags & ALPHA_MODE_BITS) == ALPHA_MODE_OPAQUE {
        alpha = dissolve_keep * glow_alpha;",
            )
            .expect("D292 alpha write");
        // D303: the glow alpha, set where the glow is added and written for opaque draws
        // before the dissolve's.
        assert!(body.contains("glow_alpha = lbp_glow_alpha(glow);"));
        assert!(body.contains("return 1.0 / (1.0 + 4.0 * dot(g, g));"));
        // D307: the glow mode in the low bits, the fog's lift of the alpha unless bit 16,
        // the same fog share as the colour's.
        assert!(body.contains("let glow_mode = graph_info.w & 15u;"));
        assert!(body.contains("if glow_mode > 0u && (graph_info.w & 16u) == 0u {"));
        assert!(body.contains("glow_alpha = mix(glow_alpha, 1.0, lbp_fog_share(depth));"));
        assert!(body.contains("let k = lbp_fog_share(depth);"));
        assert!(
            body.find("glow_alpha = mix(glow_alpha, 1.0, lbp_fog_share(depth));")
                < body.find("alpha = glow_alpha;")
        );
        let glow_write = body
            .find(
                "if (pbr.material.flags & ALPHA_MODE_BITS) == ALPHA_MODE_OPAQUE {
        alpha = glow_alpha;",
            )
            .expect("D303 alpha write");
        assert!(glow_write < keep);
        assert!(keep < body.find("alpha = glass.x * dissolve_keep;").unwrap());
        assert!(body.contains("const ALPHA_MODE_BITS: u32 = 7u << 29u;"));
        assert!(body.contains("vec4<f32>(rgb, alpha)"));
        // D287: the encoded colour times the alpha plus the specular, through the blend,
        // and the glass's lit colour without the gloss term.
        assert!(body.contains("glass_blend(alpha * srgb_encode(rgb) + glass_spec, alpha)"));
        assert!(body.contains("glass_spec = glass_specular(n, pbr.V, factor);"));
        assert!(!body.contains("rgb *= alpha;"));
        let [d1, d2] = super::GLASS_SECANT;
        for d in [d1, d2] {
            assert!(body.contains(&format!("srgb_decode(vec3<f32>({d}))")));
            assert!(body.contains(&format!("(1.0 - a) * {d});")));
        }
        assert_eq!(body.matches("discard;").count(), 2);
        let source = format!("{STUBS}{body}");
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
