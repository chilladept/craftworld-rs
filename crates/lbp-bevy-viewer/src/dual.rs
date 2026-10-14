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
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::{Shader, ShaderRef},
};
pub type ViewerMaterial = ExtendedMaterial<StandardMaterial, SecondTexture>;
const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("c1f79fb1-32b7-4f6f-bef7-71a8c677f05c");
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
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
    /// the green the dissolve's edge takes in place of the surface colour's.
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
/// last graph_param row, so it needs no per-fragment graph (slots shared).
pub fn enable_glow(
    material: &mut ViewerMaterial,
    color: [f32; 3],
    map: Option<(Handle<Image>, u32)>,
) {
    let ext = &mut material.extension;
    let blur = ext.graph_param[31].w;
    ext.graph_param[31] = Vec4::new(color[0], color[1], color[2], blur);
    ext.graph_info.w = match map {
        Some((image, mode)) => {
            ext.graph_image4 = Some(image);
            mode
        }
        None => 1,
    };
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
/// is `opacity` times the dissolve's kept share and its colour is multiplied by that
/// alpha (for `AlphaMode::Premultiplied`); the dissolve's edge takes `green` in place of
/// the surface colour's green.
pub fn enable_glass(material: &mut ViewerMaterial, opacity: f32, green: f32) {
    material.extension.glass = Vec4::new(opacity, green, 0., 0.);
}
/// D286: the glass's opacity, none when the material is not glass.
#[cfg(test)]
pub fn glass(material: &ViewerMaterial) -> Option<f32> {
    let opacity = material.extension.glass.x;
    (opacity > 0.).then_some(opacity)
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
/// D284: the input of the material's bucket that holds raw UV0 (see `raw_uv0`).
pub fn raw_uv0(material: &ViewerMaterial) -> u8 {
    material.extension.raw_uv0
}
/// D278: sets (Some) or takes off (None) the top-down dissolve. D284: both masks are
/// sampled at the bucket's raw UV0 (`raw_uv0`, set when the bucket is built); a bucket
/// without it lays the transition mask over the position (`mask_u`, `mask_v`) and has no
/// `pod_mask`. Without a mask the band under the line is not tinted. The alpha mode is
/// not touched: the cut is a `discard` in the opaque pass, as the alpha test's.
pub fn set_dissolve(material: &mut ViewerMaterial, dissolve: Option<&Dissolve>) {
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
// P2t: then the game's fog in that output space, by view depth (lbp_fog).
fn lbp_output(c: vec3<f32>, depth: f32) -> vec3<f32> {
    var v = clamp(sqrt(max(c * GAME_TINT, vec3<f32>(0.0))), vec3<f32>(0.0), vec3<f32>(1.0));
    if lbp_fog.w > 0.5 {
        let k = clamp((depth - lbp_fog.x) * lbp_fog.y, 0.0, 1.0);
        let fogged = (dot(v, vec3<f32>(0.3, 0.6, 0.1)) * lbp_fog.z + 1.0) * lbp_fog_color.rgb;
        v = clamp(mix(v, fogged, k), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    let low = v / 12.92;
    let high = pow((v + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, v <= vec3<f32>(0.04045));
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
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
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
        rgb = rgb * factor + gloss_term(n, pbr.V, factor, spec_tint, shadow * baked);
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
        if light_dir.w > 1.5 && graph_info.w > 0u && graph_info.x == 0u {
            var glow = graph_param[31].rgb;
#ifdef VERTEX_UVS_A
            if graph_info.w == 2u {
                glow *= textureSample(graph_image4, graph_sampler4, in.uv).rgb;
            }
#endif
#ifdef VERTEX_UVS_B
            if graph_info.w == 3u {
                glow *= textureSample(graph_image4, graph_sampler4, in.uv_b).rgb;
            }
#endif
            rgb += glow;
        }
        if light_dir.w > 1.5 {
            let depth = -(view.view_from_world * vec4<f32>(in.world_position.xyz, 1.0)).z;
            rgb = lbp_output(rgb, depth);
        }
    }
    if dissolve_on {
        rgb = mix(dissolve_colour.rgb, rgb, dissolve_keep);
    }
    // D286: the glass writes alpha = opacity x the dissolve's kept share and its colour
    // multiplied by it (premultiplied, for One / OneMinusSrcAlpha); the rest alpha 1.
    var alpha = 1.0;
    if glass.x > 0.0 {
        alpha = glass.x * dissolve_keep;
        rgb *= alpha;
    }
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr, vec4<f32>(rgb, alpha));
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
}
struct PbrInputStub {
    material: StandardMaterialStub,
    V: vec3<f32>,
}
fn pbr_input_from_standard_material(in: VertexOutput, front: bool) -> PbrInputStub {
    var p: PbrInputStub;
    p.material.base_color = vec4<f32>(1.0);
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
        super::enable_glow(&mut m, [0.2, 0.4, 0.6], None);
        assert_eq!(m.extension.graph_info.w, 1);
        assert_eq!(m.extension.graph_param[31], Vec4::new(0.2, 0.4, 0.6, 0.));
        assert!(m.extension.graph_image4.is_none());
        let h = Handle::<Image>::default();
        super::enable_glow(&mut m, [1.; 3], Some((h, 3)));
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
        super::enable_glow(&mut m, [0.5; 3], None);
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
        super::set_dissolve(&mut m, None);
        assert!(super::dissolve(&m).is_none() && m.extension.dissolve_mask.is_none());
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
        super::enable_glass(&mut m, 0.2, 0.117_65);
        assert_eq!(super::glass(&m), Some(0.2));
        assert_eq!(m.extension.glass, Vec4::new(0.2, 0.117_65, 0., 0.));
        assert_eq!(super::dissolve(&m).as_ref(), Some(&d));
        assert_eq!(m.extension.dissolve_params.x, 1.);
    }
    /// D121: the authored shader (all `#ifdef` blocks enabled, imports stubbed) parses
    /// and validates headlessly. Bevy's own composition is exercised only at run time.
    #[test]
    fn authored_shader_validates_with_every_branch_enabled() {
        let body: String = super::SOURCE
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .map(|l| l.replace("#{MATERIAL_BIND_GROUP}", "2") + "\n")
            .collect();
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
        assert!(body.contains("rgb = mix(dissolve_colour.rgb, rgb, dissolve_keep);"));
        // D286: the glass's green in the edge, its alpha and its premultiplied colour.
        assert!(body.contains("(above - dissolve_consts.x * green)"));
        assert!(body.contains("alpha = glass.x * dissolve_keep;"));
        assert!(body.contains("rgb *= alpha;"));
        assert!(body.contains("vec4<f32>(rgb, alpha)"));
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
