//! D298 (docs/level-scripts.md part 49): the game's depth of field, the first passes of its
//! post chain (FOUND, frames 5247 and 14790: passes 3084 / 3110 / 3128 / 3159 / 3172 and
//! 2976 / 3002 / 3020 / 3051 / 3064), run before the glow and the final pass
//! (`glow_post.rs`):
//!
//! 1. Half size: each texel is the 2x2 mean of the scene (the game's one bilinear tap) with,
//!    in its alpha, a depth gate `g` that is 7/8 at the range's start and 1 at its end
//!    (8-bit buffers, as the game's).
//! 2. Twice: a ring of five taps around the texel, on the corners of a regular pentagon,
//!    at a radius `r = saturate(8 g - 7)` (0 at the start, 1 at the end) times
//!    [`DOF_TAP_STEP`]. A tap counts only when its own `g` is not below the centre's by
//!    more than [`DOF_SAME_LAYER`] (so nearer things do not bleed into the blur); the
//!    centre always counts; the colour is the mean of the counted ones. The first pass
//!    keeps `g` in the alpha, the second writes `r`.
//! 3. Full size: where the scene's depth is beyond the range's start, the blurred image
//!    (bilinear) is blended over the scene by its alpha `r`; the scene's alpha is kept.
//!
//! The range is the scripts' `Cam.SetDOFAndFog` start and end (level units along the
//! view), the same call whose third value is the fog bias. Under `--depth-of-field` on the
//! level-lighting route only (off by default), and only while the scripts have passed a
//! usable range.
use bevy::{
    core_pipeline::{tonemapping::tonemapping, Core3d, Core3dSystems, FullscreenShader},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            binding_types::{
                sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled,
                uniform_buffer_sized,
            },
            AddressMode, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            Buffer, BufferInitDescriptor, BufferUsages, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, Extent3d, FilterMode, FragmentState, Operations, PipelineCache,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
            SamplerBindingType, SamplerDescriptor, ShaderStages, TextureDescriptor,
            TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        sync_component::SyncComponent,
        texture::{CachedTexture, TextureCache},
        view::{ViewDepthTexture, ViewTarget},
        Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::Shader,
};
use std::{
    collections::HashMap,
    num::NonZeroU64,
    sync::atomic::{AtomicBool, Ordering},
};

/// D298 FOUND (PS 14810 and 14815, fc4 / fc3 = (8, -7) in both frames): the blur's radius
/// from the depth gate, `r = saturate(DOF_RAMP x g - (DOF_RAMP - 1))`: the blur starts at
/// `g` = 7/8 and is whole at 1. With pass 3084's gate constants (frame 5247 fc1 = (4993.22,
/// -4992.22), frame 14790 (599.87, -598.56)) and the Pod camera's planes (10 and 100,000,
/// part 34), `g` is 7/8 at the depth of the first `SetDOFAndFog` value and 1 at the depth of
/// the second: (80,000, 100,000) in 5247 (the idle view, 2 x (40,000, 50,000)) and
/// (12,000, 16,000) in 14790 (the main menu's view, 2 x (6,000, 8,000)), within 0.13 %.
/// So `r` is linear in the depth buffer from 0 at the start to 1 at the end.
pub const DOF_RAMP: f32 = 8.0;
/// D298 FOUND (PS 14810 / 14815, fc0 = (0.0025, 0.00444) in both frames): one unit of the
/// ring's radius in uv: 1.6 half-size texels on the game's 640 x 360 buffers (3.2 scene
/// pixels). The y value is printed with five decimals; 1.6 / 360 is taken.
pub const DOF_TAP_STEP: [f32; 2] = [0.0025, 1.6 / 360.];
/// D298 FOUND (PS 14810 / 14815, fc3, fc5, fc7..fc11): the five tap directions, the corners
/// of a regular pentagon (72 degrees apart); +y is up the image (MEASURED: the mirror of the
/// replayed buffers matches only with y up, to within 0.6 of a level, against 13 with y
/// down). Both passes use the same five.
pub const DOF_TAP_DIRS: [[f32; 2]; 5] = [
    [0., 1.],
    [-0.587_78, -0.809_02],
    [-0.951_06, 0.309_01],
    [0.951_06, 0.309_02],
    [0.587_79, -0.809_02],
];
/// D298 FOUND (PS 14810 fc1 / 14815 fc2 = -0.01): a tap counts when its gate is above the
/// centre's minus this.
pub const DOF_SAME_LAYER: f32 = 0.01;
/// D298 FOUND (frames 5247 and 14790): two ring passes between the source and the
/// composite.
pub const DOF_RING_PASSES: usize = 2;

/// D298 `--depth-of-field` (level-lighting route only, `main.rs`).
static ENABLED: AtomicBool = AtomicBool::new(false);
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// D298: on the main 3D camera: the scripts' depth-of-field range in display units
/// (start, end along the view), `None` before a usable `SetDOFAndFog`.
#[derive(Component, Clone, Copy, Default, Debug, PartialEq)]
pub struct DepthOfField {
    pub range: Option<[f32; 2]>,
}
impl SyncComponent for DepthOfField {
    type Target = DofRange;
}
/// Extracted only under the flag, with a range and a perspective lens.
impl ExtractComponent for DepthOfField {
    type QueryData = (&'static DepthOfField, &'static Projection);
    type QueryFilter = ();
    type Out = DofRange;
    fn extract_component((dof, projection): (&DepthOfField, &Projection)) -> Option<DofRange> {
        if !enabled() {
            return None;
        }
        let Projection::Perspective(lens) = projection else {
            return None;
        };
        let [start, end] = dof.range?;
        DofRange::new(start, end, lens.near)
    }
}

/// D298, render world: the ramp `r = c - k x depth` over Bevy's depth buffer (reverse z,
/// infinite far: `depth = near / distance`), 0 at the range's start and 1 at its end.
/// The same ramp as the game's (linear in its depth buffer), since both buffers are affine
/// in 1 / distance.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct DofRange {
    pub c: f32,
    pub k: f32,
}
impl DofRange {
    /// `start` and `end` along the view and the lens's `near`, all in one unit; `None`
    /// unless 0 < start < end and near > 0, all finite.
    pub fn new(start: f32, end: f32, near: f32) -> Option<Self> {
        let ok = start.is_finite() && end.is_finite() && near.is_finite();
        if !ok || start <= 0. || end <= start || near <= 0. {
            return None;
        }
        let span = 1. / start - 1. / end;
        Some(Self {
            c: (1. / start) / span,
            k: (1. / near) / span,
        })
    }
    /// The ramp at a depth-buffer value (unclamped).
    #[cfg(test)]
    pub fn ramp(&self, depth: f32) -> f32 {
        self.c - self.k * depth
    }
}

/// D298: the ramp at a distance along the view: `(1/start - 1/z) / (1/start - 1/end)`.
#[cfg(test)]
pub fn ramp_at(start: f32, end: f32, z: f32) -> f32 {
    (1. / start - 1. / z) / (1. / start - 1. / end)
}
/// D298: the gate the source pass stores for a ramp value: 7/8 at the start, 1 at the end.
#[cfg(test)]
pub fn gate(ramp: f32) -> f32 {
    ((DOF_RAMP - 1. + ramp) / DOF_RAMP).clamp(0., 1.)
}
/// D298: the blur's radius (in [`DOF_TAP_STEP`] units) for a stored gate.
#[cfg(test)]
pub fn radius(gate: f32) -> f32 {
    (DOF_RAMP * gate - (DOF_RAMP - 1.)).clamp(0., 1.)
}

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("8e3b6a2d-41f7-4c5e-9a0b-d29800000001");
/// The game's depth-of-field buffers are 8-bit (R8G8B8A8_UNORM, FOUND): the encoded colour
/// and the gate as stored.
const HALF_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

pub struct DofPostPlugin;
impl Plugin for DofPostPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                SHADER.id(),
                Shader::from_wgsl(shader_source(), "d298-depth-of-field.wgsl"),
            )
            .unwrap();
        app.add_plugins(ExtractComponentPlugin::<DepthOfField>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(Render, prepare_dof.in_set(RenderSystems::PrepareResources))
            .add_systems(
                Core3d,
                depth_of_field
                    .in_set(Core3dSystems::PostProcess)
                    .after(tonemapping)
                    .before(crate::glow_post::edge_glow)
                    .before(crate::mlaa_post::mlaa),
            );
    }
}

#[derive(Resource)]
struct DofPipelines {
    /// Keyed by whether the depth buffer is multisampled.
    layouts: [BindGroupLayoutDescriptor; 2],
    sampler: Sampler,
    ids: HashMap<(TextureFormat, bool, &'static str), CachedRenderPipelineId>,
    vertex: bevy::render::render_resource::VertexState,
}

fn layout(multisampled: bool) -> BindGroupLayoutDescriptor {
    let depth = if multisampled {
        texture_depth_2d_multisampled()
    } else {
        texture_depth_2d()
    };
    BindGroupLayoutDescriptor::new(
        if multisampled {
            "d298_dof_msaa"
        } else {
            "d298_dof"
        },
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                depth,
                uniform_buffer_sized(false, NonZeroU64::new(16)),
                texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        ),
    )
}

fn init_pipelines(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
) {
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("d298_dof_sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    commands.insert_resource(DofPipelines {
        layouts: [layout(false), layout(true)],
        sampler,
        ids: HashMap::new(),
        vertex: fullscreen.to_vertex_state(),
    });
}

/// The entry points in the order they run; the ring runs [`DOF_RING_PASSES`] times.
const ENTRIES: [&str; 4] = [
    "dof_source",
    "dof_ring_gate",
    "dof_ring_radius",
    "dof_composite",
];

#[derive(Component)]
struct DofView {
    a: CachedTexture,
    b: CachedTexture,
    range: Buffer,
    multisampled: bool,
    pipelines: [CachedRenderPipelineId; 4],
}

fn prepare_dof(
    mut commands: Commands,
    mut pipelines: ResMut<DofPipelines>,
    cache: Res<PipelineCache>,
    mut textures: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    views: Query<(Entity, &ViewTarget, &ViewDepthTexture, &DofRange)>,
    stale: Query<Entity, (With<DofView>, Without<DofRange>)>,
) {
    for entity in &stale {
        commands.entity(entity).remove::<DofView>();
    }
    for (entity, target, depth, range) in &views {
        let format = target.main_texture_format();
        let Some(srgb) = crate::glow_post::target_space(format) else {
            warn_once!("D298: depth of field skipped: main texture format {format:?}");
            commands.entity(entity).remove::<DofView>();
            continue;
        };
        if !depth
            .texture
            .usage()
            .contains(TextureUsages::TEXTURE_BINDING)
        {
            warn_once!("D298: depth of field skipped: the depth buffer cannot be read");
            commands.entity(entity).remove::<DofView>();
            continue;
        }
        let multisampled = depth.texture.sample_count() > 1;
        let p = &mut *pipelines;
        let pipelines = ENTRIES.map(|entry| {
            let out = if matches!(entry, "dof_composite") {
                format
            } else {
                HALF_FORMAT
            };
            *p.ids.entry((out, multisampled, entry)).or_insert_with(|| {
                let mut shader_defs = Vec::new();
                if srgb {
                    shader_defs.push("SRGB_TARGET".into());
                }
                if multisampled {
                    shader_defs.push("DEPTH_MSAA".into());
                }
                cache.queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some(format!("d298_{entry}").into()),
                    layout: vec![p.layouts[usize::from(multisampled)].clone()],
                    vertex: p.vertex.clone(),
                    fragment: Some(FragmentState {
                        shader: SHADER,
                        shader_defs,
                        entry_point: Some(entry.into()),
                        targets: vec![Some(ColorTargetState {
                            format: out,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    ..default()
                })
            })
        });
        let size = target.main_texture().size();
        let mut half = |label: &'static str| {
            textures.get(
                &device,
                TextureDescriptor {
                    label: Some(label),
                    size: Extent3d {
                        width: size.width.div_ceil(2).max(1),
                        height: size.height.div_ceil(2).max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: HALF_FORMAT,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        let (a, b) = (half("d298_dof_a"), half("d298_dof_b"));
        let words = [range.c, range.k, 0., 0.];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let range = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("d298_dof_range"),
            contents: &bytes,
            usage: BufferUsages::UNIFORM,
        });
        commands.entity(entity).insert(DofView {
            a,
            b,
            range,
            multisampled,
            pipelines,
        });
    }
}

/// D298: source (scene -> a), ring with the gate kept (a -> b), ring writing the radius
/// (b -> a), composite (scene and a -> the main texture).
fn depth_of_field(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture, &DofView)>,
    pipelines: Res<DofPipelines>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, depth, dof) = view.into_inner();
    let mut ready = Vec::with_capacity(4);
    for id in dof.pipelines {
        let Some(pipeline) = cache.get_render_pipeline(id) else {
            return;
        };
        ready.push(pipeline);
    }
    let device = ctx.render_device().clone();
    let layout = cache.get_bind_group_layout(&pipelines.layouts[usize::from(dof.multisampled)]);
    let post = target.post_process_write();
    let (a, b) = (&dof.a.default_view, &dof.b.default_view);
    // (pipeline, the half-size input, the output); the scene is bound in every pass.
    let steps = [
        (ready[0], b, a),
        (ready[1], a, b),
        (ready[2], b, a),
        (ready[3], a, post.destination),
    ];
    debug_assert_eq!(DOF_RING_PASSES, 2);
    let encoder = ctx.command_encoder();
    for (pipeline, input, out) in steps {
        let bind_group = device.create_bind_group(
            "d298_dof",
            &layout,
            &BindGroupEntries::sequential((
                post.source,
                &pipelines.sampler,
                depth.view(),
                dof.range.as_entire_binding(),
                input,
            )),
        );
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("d298_depth_of_field"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: out,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The WGSL of the passes, with the FOUND constants written in from the consts above.
fn shader_source() -> String {
    let dirs = DOF_TAP_DIRS
        .iter()
        .map(|[x, y]| format!("vec2<f32>({x:?}, {y:?})"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "const DOF_RAMP: f32 = {:?};\nconst DOF_TAP_STEP = vec2<f32>({:?}, {:?});\nconst DOF_TAP_DIRS = array<vec2<f32>, 5>({dirs});\nconst DOF_SAME_LAYER: f32 = {:?};\n{BODY}",
        DOF_RAMP, DOF_TAP_STEP[0], DOF_TAP_STEP[1], DOF_SAME_LAYER
    )
}

const BODY: &str = r#"
// D298: the game's depth of field (frames 5247 and 14790), in its encoded output space.
@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var linear_sampler: sampler;
#ifdef DEPTH_MSAA
@group(0) @binding(2) var depth_texture: texture_depth_multisampled_2d;
#else
@group(0) @binding(2) var depth_texture: texture_depth_2d;
#endif
// (c, k, 0, 0): the ramp is c - k x depth, 0 at the range's start and 1 at its end.
@group(0) @binding(3) var<uniform> range: vec4<f32>;
@group(0) @binding(4) var half_texture: texture_2d<f32>;
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
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
fn to_encoded(c: vec3<f32>) -> vec3<f32> {
#ifdef SRGB_TARGET
    return srgb_encode(c);
#else
    return clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
#endif
}
fn from_encoded(e: vec3<f32>) -> vec3<f32> {
#ifdef SRGB_TARGET
    return srgb_decode(e);
#else
    return e;
#endif
}
// Sample 0 of the depth buffer (the first sample of a multisampled one).
fn ramp_at(p: vec2<i32>) -> f32 {
    let depth = textureLoad(depth_texture, p, 0);
    return range.x - range.y * depth;
}
// Pass 1, half size: the 2x2 mean of the encoded scene, and the gate (7/8 at the start,
// 1 at the end) from the depth under the block's first pixel.
@fragment
fn dof_source(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(scene_texture));
    let base = vec2<i32>(floor(in.position.xy)) * 2;
    var rgb = vec3<f32>(0.0);
    for (var k = 0; k < 4; k++) {
        let p = min(base + vec2<i32>(k & 1, k >> 1u), size - vec2<i32>(1));
        rgb += to_encoded(textureLoad(scene_texture, p, 0).rgb);
    }
    let gate = saturate((DOF_RAMP - 1.0 + ramp_at(min(base, size - vec2<i32>(1)))) / DOF_RAMP);
    return vec4<f32>(rgb * 0.25, gate);
}
// Passes 2 and 3: the centre and five taps on the pentagon at radius r (from the centre's
// gate) x DOF_TAP_STEP, +y up the image; a tap counts when its gate is above the centre's
// minus DOF_SAME_LAYER. Returns the mean colour and the centre's gate.
fn ring(uv: vec2<f32>) -> vec4<f32> {
    let centre = textureSampleLevel(half_texture, linear_sampler, uv, 0.0);
    let r = saturate(DOF_RAMP * centre.a - (DOF_RAMP - 1.0));
    var dirs = DOF_TAP_DIRS;
    var sum = centre.rgb;
    var count = 1.0;
    for (var k = 0; k < 5; k++) {
        let d = dirs[k] * vec2<f32>(1.0, -1.0) * DOF_TAP_STEP * r;
        let tap = textureSampleLevel(half_texture, linear_sampler, uv + d, 0.0);
        let keep = select(0.0, 1.0, tap.a > centre.a - DOF_SAME_LAYER);
        sum += tap.rgb * keep;
        count += keep;
    }
    return vec4<f32>(sum / count, centre.a);
}
@fragment
fn dof_ring_gate(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return ring(in.uv);
}
@fragment
fn dof_ring_radius(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = ring(in.uv);
    return vec4<f32>(c.rgb, saturate(DOF_RAMP * c.a - (DOF_RAMP - 1.0)));
}
// Pass 4, full size: beyond the range's start the blurred image (bilinear from half size)
// over the scene by its radius; the scene's alpha is kept.
@fragment
fn dof_composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.position.xy));
    let scene = textureLoad(scene_texture, p, 0);
    if (ramp_at(p) <= 0.0) {
        return scene;
    }
    let blurred = textureSampleLevel(half_texture, linear_sampler, in.uv, 0.0);
    let e = mix(to_encoded(scene.rgb), blurred.rgb, blurred.a);
    return vec4<f32>(from_encoded(e), scene.a);
}
"#;

/// D298: a CPU mirror of the four passes on a scene of encoded colours and ramps
/// (row-major, `w` x `h`, both even): the half-size source with its 8-bit gate, the two
/// rings (bilinear taps, clamped), and the composite (bilinear from half size where the
/// ramp is above 0). Returns the encoded colours.
#[cfg(test)]
pub fn dof_cpu(rgb: &[[f32; 3]], ramp: &[f32], w: usize, h: usize) -> Vec<[f32; 3]> {
    type Img = (Vec<[f32; 4]>, usize, usize);
    let bilinear = |img: &Img, u: f32, v: f32| -> [f32; 4] {
        let (data, iw, ih) = img;
        let (x, y) = (u * *iw as f32 - 0.5, v * *ih as f32 - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |i: f32, j: f32| {
            let i = (i.max(0.) as usize).min(iw - 1);
            let j = (j.max(0.) as usize).min(ih - 1);
            data[j * iw + i]
        };
        let (a, b, c, d) = (
            at(x0, y0),
            at(x0 + 1., y0),
            at(x0, y0 + 1.),
            at(x0 + 1., y0 + 1.),
        );
        std::array::from_fn(|k| {
            (a[k] * (1. - fx) + b[k] * fx) * (1. - fy) + (c[k] * (1. - fx) + d[k] * fx) * fy
        })
    };
    let byte = |v: f32| (v.clamp(0., 1.) * 255.).round() / 255.;
    let (hw, hh) = (w / 2, h / 2);
    let mut half = vec![[0.; 4]; hw * hh];
    for j in 0..hh {
        for i in 0..hw {
            let mut c = [0.; 4];
            for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let p = (2 * j + dj) * w + 2 * i + di;
                (0..3).for_each(|k| c[k] += rgb[p][k] * 0.25);
            }
            c[3] = gate(ramp[2 * j * w + 2 * i]);
            half[j * hw + i] = c.map(byte);
        }
    }
    let mut img: Img = (half, hw, hh);
    for pass in 0..DOF_RING_PASSES {
        let mut out = vec![[0.; 4]; hw * hh];
        for j in 0..hh {
            for i in 0..hw {
                let (u, v) = ((i as f32 + 0.5) / hw as f32, (j as f32 + 0.5) / hh as f32);
                let centre = img.0[j * hw + i];
                let r = radius(centre[3]);
                let mut sum = [centre[0], centre[1], centre[2]];
                let mut count = 1.;
                for [dx, dy] in DOF_TAP_DIRS {
                    let tap = bilinear(
                        &img,
                        u + dx * DOF_TAP_STEP[0] * r,
                        v - dy * DOF_TAP_STEP[1] * r,
                    );
                    if tap[3] > centre[3] - DOF_SAME_LAYER {
                        (0..3).for_each(|k| sum[k] += tap[k]);
                        count += 1.;
                    }
                }
                let a = if pass + 1 == DOF_RING_PASSES {
                    radius(centre[3])
                } else {
                    centre[3]
                };
                out[j * hw + i] = [sum[0] / count, sum[1] / count, sum[2] / count, a].map(byte);
            }
        }
        img = (out, hw, hh);
    }
    (0..w * h)
        .map(|p| {
            if ramp[p] <= 0. {
                return rgb[p];
            }
            let (u, v) = (
                ((p % w) as f32 + 0.5) / w as f32,
                ((p / w) as f32 + 0.5) / h as f32,
            );
            let b = bilinear(&img, u, v);
            std::array::from_fn(|k| rgb[p][k] * (1. - b[3]) + b[k] * b[3])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D298: the FOUND constants: the ramp (8, -7), the tap step of 1.6 half-size texels on
    /// the game's 640 x 360 buffers, five unit directions 72 degrees apart, two rings.
    #[test]
    fn d298_constants() {
        assert_eq!(DOF_RAMP, 8.);
        assert_eq!(DOF_SAME_LAYER, 0.01);
        assert_eq!(DOF_RING_PASSES, 2);
        assert!((DOF_TAP_STEP[0] * 640. - 1.6).abs() < 1e-5);
        assert!((DOF_TAP_STEP[1] * 360. - 1.6).abs() < 1e-5);
        assert!((DOF_TAP_STEP[1] - 0.00444).abs() < 5e-6);
        let mut angles: Vec<f32> = DOF_TAP_DIRS
            .iter()
            .map(|[x, y]| {
                assert!((x.hypot(*y) - 1.).abs() < 1e-4, "{x} {y}");
                y.atan2(*x).to_degrees().rem_euclid(360.)
            })
            .collect();
        angles.sort_by(f32::total_cmp);
        for pair in angles.windows(2) {
            assert!((pair[1] - pair[0] - 72.).abs() < 0.01, "{angles:?}");
        }
        assert_eq!(radius(gate(0.)), 0.);
        assert_eq!(radius(gate(1.)), 1.);
        assert!((gate(0.) - 0.875).abs() < 1e-6);
    }

    /// D298: pass 3084's gate constants of both frames come back from the scripts' range
    /// and the Pod camera's planes (10 and 100,000): the gate is 7/8 at the depth of the
    /// first value and 1 at the depth of the second, so the radius is linear in the depth
    /// buffer between them; the viewer's ramp over distance is the same one.
    #[test]
    fn d298_gate_constants_from_the_range() {
        let (n, f) = (10f64, 100_000f64);
        let depth = |z: f64| f / (f - n) * (1. - n / z);
        let gate_constants = |start: f64, end: f64| {
            let a = 1. / (8. * (depth(end) - depth(start)));
            (a, 1. - a * depth(end))
        };
        // FOUND: frame 5247 fc1 = (4993.21924, -4992.21924), 14790 (599.87183, -598.55688).
        // Within 0.13 %: the game's depths near 1 are 32-bit floats (2.5e-5 from 1 in 5247).
        for ((start, end), (fa, fb)) in [
            ((80_000., 100_000.), (4993.21924, -4992.21924)),
            ((12_000., 16_000.), (599.87183, -598.55688)),
        ] {
            let (a, b) = gate_constants(start, end);
            assert!((a / fa - 1.).abs() < 2e-3, "{a} {fa}");
            assert!((b / fb - 1.).abs() < 2e-3, "{b} {fb}");
            // The game's radius at a few distances against the viewer's ramp.
            for z in [start * 0.5, start, (start + end) / 2., end, end * 3.] {
                let g = (a * depth(z) + b).clamp(0., 1.);
                let game = (8. * g - 7.).clamp(0., 1.) as f32;
                let viewer = radius(gate(ramp_at(start as f32, end as f32, z as f32)));
                assert!((game - viewer).abs() < 2e-3, "{z} {game} {viewer}");
            }
        }
        // The ramp over Bevy's reverse-z buffer (depth = near / z) equals the one over
        // distance, in display units as in level units.
        let (start, end, near) = (12_000. * 1e-3, 16_000. * 1e-3, 10. * 1e-3);
        let range = DofRange::new(start, end, near).unwrap();
        for z in [6., 12., 14., 16., 40.] {
            let want = ramp_at(start, end, z);
            assert!((range.ramp(near / z) - want).abs() < 1e-4, "{z}");
        }
        assert!(range.ramp(near / start).abs() < 1e-4);
        assert!((range.ramp(near / end) - 1.).abs() < 1e-4);
        // The cleared background (depth 0) is beyond the end: whole blur.
        assert!(range.ramp(0.) > 1.);
        assert_eq!(DofRange::new(0., 1., 1.), None);
        assert_eq!(DofRange::new(2., 1., 1.), None);
        assert_eq!(DofRange::new(1., 2., 0.), None);
        assert_eq!(DofRange::new(1., f32::NAN, 1.), None);
    }

    fn checker(w: usize, h: usize) -> Vec<[f32; 3]> {
        (0..w * h)
            .map(|p| {
                let v = if ((p % w) / 2 + (p / w) / 2).is_multiple_of(2) {
                    0.8
                } else {
                    0.2
                };
                [v, v * 0.5, 0.1]
            })
            .collect()
    }
    fn gradient(rgb: &[[f32; 3]], w: usize, h: usize, rows: std::ops::Range<usize>) -> f32 {
        let mut sum = 0.;
        let mut n = 0.;
        for y in rows {
            for x in 0..w - 1 {
                sum += (rgb[y * w + x + 1][0] - rgb[y * w + x][0]).abs();
                n += 1.;
            }
        }
        let _ = h;
        sum / n
    }

    /// D298: a fine pattern beyond the range is softened (the gradient falls), the same
    /// pattern nearer than its start is untouched pixel for pixel, and a near object's
    /// colour does not bleed into the blurred background next to it.
    #[test]
    fn d298_far_blurs_near_stays() {
        let (w, h) = (32, 32);
        let rgb = checker(w, h);
        // Top half beyond the end (ramp 2), bottom half nearer than the start (-3).
        let ramp: Vec<f32> = (0..w * h)
            .map(|p| if p / w < 16 { 2. } else { -3. })
            .collect();
        let out = dof_cpu(&rgb, &ramp, w, h);
        let before = gradient(&rgb, w, h, 2..12);
        let after = gradient(&out, w, h, 2..12);
        assert!(after < 0.5 * before, "{before} {after}");
        for p in 16 * w..w * h {
            assert_eq!(out[p], rgb[p], "{p}");
        }
        // Nothing in range: the frame is unchanged.
        let none = dof_cpu(&rgb, &vec![-1.; w * h], w, h);
        assert_eq!(none, rgb);
        // A near white square (ramp -3) on a far flat grey (ramp 2): the rings leave the
        // white out (its gate is below the grey's), so the grey keeps its value but for the
        // two pixels next to the square, where the half-size image is read back bilinear
        // (as the game's composite does); the square itself is untouched.
        let mut rgb = vec![[0.3; 3]; w * h];
        let mut ramp = vec![2.; w * h];
        for y in 12..20 {
            for x in 12..20 {
                rgb[y * w + x] = [1.; 3];
                ramp[y * w + x] = -3.;
            }
        }
        let out = dof_cpu(&rgb, &ramp, w, h);
        for y in 8..24 {
            for x in 8..24 {
                let p = y * w + x;
                let near_edge = (10..22).contains(&x) && (10..22).contains(&y);
                if ramp[p] > 0. && !near_edge {
                    assert!((out[p][0] - 0.3).abs() < 1.5 / 255., "{x} {y} {:?}", out[p]);
                } else if ramp[p] > 0. {
                    assert!(out[p][0] > 0.3 && out[p][0] < 0.65, "{x} {y} {:?}", out[p]);
                } else {
                    assert_eq!(out[p], [1.; 3]);
                }
            }
        }
    }

    /// D298: just past the start the blur is light (radius near 0: about the half-size
    /// resampling alone), at the end whole; a ramp of exactly 0 is not composited.
    #[test]
    fn d298_radius_follows_the_ramp() {
        let (w, h) = (32, 8);
        let rgb = checker(w, h);
        let at = |r: f32| {
            let out = dof_cpu(&rgb, &vec![r; w * h], w, h);
            gradient(&out, w, h, 2..6)
        };
        let base = gradient(&rgb, w, h, 2..6);
        assert_eq!(at(0.), base);
        let (light, whole) = (at(0.05), at(1.));
        assert!(light < base && whole < light, "{base} {light} {whole}");
    }

    /// Resolves `#ifdef NAME` / `#else` / `#endif` for the given definitions.
    fn preprocess(source: &str, defs: &[&str]) -> String {
        let mut active = vec![true];
        let mut body = String::new();
        for line in source.lines() {
            let t = line.trim_start();
            if let Some(def) = t.strip_prefix("#ifdef ") {
                let def = def.trim();
                assert!(["SRGB_TARGET", "DEPTH_MSAA"].contains(&def), "{def}");
                let parent = *active.last().unwrap();
                active.push(parent && defs.contains(&def));
            } else if t.starts_with("#else") {
                let parent = active[active.len() - 2];
                let now = active.last_mut().unwrap();
                *now = parent && !*now;
            } else if t.starts_with("#endif") {
                active.pop();
            } else {
                assert!(!t.starts_with('#'), "{t}");
                if *active.last().unwrap() {
                    body += line;
                    body.push('\n');
                }
            }
        }
        assert_eq!(active.len(), 1);
        body
    }

    /// D298: the passes' shader carries the FOUND constants and validates with naga for
    /// both target kinds and both depth buffers.
    #[test]
    fn d298_dof_shader_validates() {
        let raw = shader_source();
        for srgb in [true, false] {
            for msaa in [true, false] {
                let mut defs = Vec::new();
                if srgb {
                    defs.push("SRGB_TARGET");
                }
                if msaa {
                    defs.push("DEPTH_MSAA");
                }
                let source = preprocess(&raw, &defs);
                assert!(source.contains("const DOF_RAMP: f32 = 8.0;"));
                assert!(source.contains("const DOF_TAP_STEP = vec2<f32>(0.0025, "));
                assert!(source.contains("const DOF_SAME_LAYER: f32 = 0.01;"));
                assert!(source.contains("vec2<f32>(-0.95106, 0.30901)"));
                assert!(source.contains("tap.a > centre.a - DOF_SAME_LAYER"));
                assert!(source.contains("dirs[k] * vec2<f32>(1.0, -1.0) * DOF_TAP_STEP * r"));
                assert!(source.contains("if (ramp_at(p) <= 0.0)"));
                assert!(source.contains("vec4<f32>(from_encoded(e), scene.a)"));
                assert_eq!(source.contains("texture_depth_multisampled_2d"), msaa);
                assert_eq!(source.contains("return srgb_encode(c);"), srgb);
                let module = naga::front::wgsl::parse_str(&source)
                    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap();
                for entry in ENTRIES {
                    assert!(
                        module.entry_points.iter().any(|e| e.name == entry),
                        "{entry}"
                    );
                }
            }
        }
    }

    /// D298: the range reaches the render world only under the flag, with a usable range
    /// and a perspective lens.
    #[test]
    fn d298_extracted_only_with_flag_and_range() {
        let lens = Projection::Perspective(PerspectiveProjection {
            near: 0.01,
            ..default()
        });
        let dof = DepthOfField {
            range: Some([12., 16.]),
        };
        // The flag is process-wide: off until enabled.
        if !enabled() {
            assert!(DepthOfField::extract_component((&dof, &lens)).is_none());
        }
        enable();
        let got = DepthOfField::extract_component((&dof, &lens)).unwrap();
        assert_eq!(Some(got), DofRange::new(12., 16., 0.01));
        assert!(DepthOfField::extract_component((&DepthOfField::default(), &lens)).is_none());
        let ortho = Projection::Orthographic(OrthographicProjection::default_3d());
        assert!(DepthOfField::extract_component((&dof, &ortho)).is_none());
    }
}
