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
//! D312 (part 63): the game runs these steps twice a frame (FOUND, part 62, frames 8348,
//! 8980 and 5247). DoF 1 runs between its opaque and translucent loops, without the stencil:
//! the same source, rings and one composite over the opaque scene where it lies beyond the
//! plane. The translucent surfaces are then drawn over it (the glass writing its depth,
//! `dual.rs`, and every translucent draw marking the stencil bit 0x80). DoF 2 is the post-chain
//! form (part 57): on the marked pixels the composite runs without the plane test (so on the
//! glass, whose own depth gives a radius of 0, nothing changes), on the others where the scene
//! lies beyond the plane. The viewer's marked pixels are those whose depth changed between a
//! copy of the depth taken after the opaque pass and the final depth.
//!
//! Where DoF 1 runs in the viewer: the main camera has no HDR and `Tonemapping::None`, so its
//! main texture after the opaque pass already holds the encoded colours the post chain sees
//! (the same `Rgba8UnormSrgb` / `Rgba8Unorm` target, nothing between it and the post pass but
//! the transparent pass); DoF 1 reads it as DoF 2 does (`post_process_write`, the same
//! encoded-space mix rather than the game's hardware SrcAlpha blend, which on an sRGB target
//! would mix in linear light). Under MSAA its composite is written into the multisampled
//! texture (every sample) and resolved, so the transparent pass, which loads the samples,
//! draws over the blurred scene.
//!
//! The range is the scripts' `Cam.SetDOFAndFog` start and end (level units along the
//! view), the same call whose third value is the fog bias. On the level-lighting route
//! only: by default in the Pod picture (`--pod-camera`, D304; `--no-depth-of-field` turns it
//! off), elsewhere under `--depth-of-field`; and only while the scripts have passed a usable
//! range.
use bevy::{
    core_pipeline::{
        core_3d::{main_opaque_pass_3d, main_transparent_pass_3d},
        tonemapping::tonemapping,
        Core3d, Core3dSystems, FullscreenShader,
    },
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
            ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState, Operations,
            PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
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
/// of a regular pentagon (72 degrees apart). Both passes use the same five. D313 (part 64):
/// +y is down the picture (MEASURED on the replayed buffers read in their stored row order,
/// whose first row is the top of the picture: all eight ring passes of 8348, 8980 and 14790,
/// DoF 1's and DoF 2's, fit within 0.02 of a level with y down against 0.05 to 0.27 with y
/// up; FOUND: every program holds the same five directions). D298's y up came from buffers
/// read bottom row first (INFERRED: RenderDoc's saved images are upside down).
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
/// D312 FOUND (part 63, frames 8348 and 8980: fc1 of DoF 1's rings (0.0025005, 0.0044441),
/// then twice that): DoF 1's second ring takes taps at twice [`DOF_TAP_STEP`]; DoF 2's rings
/// both at the step.
pub const DOF_1_SECOND_RING_SCALE: f32 = 2.0;

/// D298 `--depth-of-field` (level-lighting route only, `main.rs`); D304 on by default in the Pod.
static ENABLED: AtomicBool = AtomicBool::new(false);
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
/// D304: whether the depth of field runs: on the level-lighting route only; by default in the
/// Pod picture (`--pod-camera`) and off elsewhere. `flag` is `Some(true)` under
/// `--depth-of-field` (on in any level there) and `Some(false)` under `--no-depth-of-field`.
pub fn dof_route(level_route: bool, pod: bool, flag: Option<bool>) -> bool {
    level_route && flag.unwrap_or(pod)
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
                (
                    // D312: DoF 1 between the opaque and the transparent pass, after a copy
                    // of the depth the opaque pass left (DoF 2 tells the translucent pixels
                    // by it).
                    (copy_opaque_depth, depth_of_field_opaque)
                        .chain()
                        .in_set(Core3dSystems::MainPass)
                        .after(main_opaque_pass_3d)
                        .before(main_transparent_pass_3d),
                    // D298 / D312: DoF 2, the post-chain form.
                    depth_of_field
                        .in_set(Core3dSystems::PostProcess)
                        .after(tonemapping)
                        .before(crate::glow_post::edge_glow)
                        .before(crate::mlaa_post::mlaa),
                ),
            );
    }
}

#[derive(Resource)]
struct DofPipelines {
    /// Keyed by whether the depth buffer is multisampled.
    layouts: [BindGroupLayoutDescriptor; 2],
    sampler: Sampler,
    /// Keyed by the output format, the output's sample count, whether the depth buffer is
    /// multisampled, and the entry point.
    ids: HashMap<(TextureFormat, u32, bool, &'static str), CachedRenderPipelineId>,
    vertex: bevy::render::render_resource::VertexState,
}

fn layout(multisampled: bool) -> BindGroupLayoutDescriptor {
    let depth = || {
        if multisampled {
            texture_depth_2d_multisampled()
        } else {
            texture_depth_2d()
        }
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
                depth(),
                uniform_buffer_sized(false, NonZeroU64::new(16)),
                texture_2d(TextureSampleType::Float { filterable: true }),
                // D312: the depth after the opaque pass, to tell the translucent pixels.
                depth(),
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

/// The entry points: the source, the rings ([`DOF_RING_PASSES`]), the composites.
const ENTRIES: [&str; 6] = [
    "dof_source",
    "dof_ring_gate",
    "dof_ring_radius",
    "dof_ring_gate_wide",
    "dof_composite",
    "dof_composite_marked",
];
/// D312: DoF 1's passes in [`ENTRIES`]: its second ring at twice the step, keeping the gate,
/// and its composite (beyond the plane, alpha `8 a - 7`).
const DOF_1: [usize; 4] = [0, 1, 3, 4];
/// D312: DoF 2's: both rings at the step, the second writing the radius, and the marked
/// composite (marked pixels without the plane test, the others nearer than the plane).
const DOF_2: [usize; 4] = [0, 1, 2, 5];

#[derive(Component)]
struct DofView {
    a: CachedTexture,
    b: CachedTexture,
    /// D312: the depth buffer as the opaque pass left it.
    opaque_depth: CachedTexture,
    range: Buffer,
    multisampled: bool,
    pipelines: [CachedRenderPipelineId; 6],
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
            .contains(TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC)
        {
            warn_once!("D298: depth of field skipped: the depth buffer cannot be read");
            commands.entity(entity).remove::<DofView>();
            continue;
        }
        let samples = depth.texture.sample_count();
        let multisampled = samples > 1;
        // D312: DoF 1's composite is drawn into the main pass's (multisampled) texture.
        let main_samples = target
            .sampled_main_texture()
            .map_or(1, |texture| texture.sample_count());
        let p = &mut *pipelines;
        let pipelines = ENTRIES.map(|entry| {
            let (out, count) = match entry {
                "dof_composite" => (format, main_samples),
                "dof_composite_marked" => (format, 1),
                _ => (HALF_FORMAT, 1),
            };
            *p.ids
                .entry((out, count, multisampled, entry))
                .or_insert_with(|| {
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
                        multisample: MultisampleState { count, ..default() },
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
        let opaque_depth = textures.get(
            &device,
            TextureDescriptor {
                label: Some("d312_dof_opaque_depth"),
                size: depth.texture.size(),
                mip_level_count: 1,
                sample_count: samples,
                dimension: TextureDimension::D2,
                format: depth.texture.format(),
                // (A multisampled texture must also be a render attachment.)
                usage: TextureUsages::COPY_DST
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        );
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
            opaque_depth,
            range,
            multisampled,
            pipelines,
        });
    }
}

/// D312: between the opaque and the transparent pass, a copy of the depth buffer: where a
/// translucent draw (the glass) then writes its depth, DoF 2 treats the pixel as the game's
/// stencil-marked one.
fn copy_opaque_depth(view: ViewQuery<(&ViewDepthTexture, &DofView)>, mut ctx: RenderContext) {
    let (depth, dof) = view.into_inner();
    if depth.texture.size() != dof.opaque_depth.texture.size() {
        return;
    }
    ctx.command_encoder().copy_texture_to_texture(
        depth.texture.as_image_copy(),
        dof.opaque_depth.texture.as_image_copy(),
        depth.texture.size(),
    );
}

/// D312: DoF 1, between the opaque and the transparent pass: the four steps over the opaque
/// scene (the second ring at twice the step), composited where it lies beyond the plane (no
/// stencil, FOUND parts 62 and 63).
fn depth_of_field_opaque(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture, &DofView)>,
    pipelines: Res<DofPipelines>,
    cache: Res<PipelineCache>,
    ctx: RenderContext,
) {
    let (target, depth, dof) = view.into_inner();
    run_passes((target, depth, dof), &pipelines, &cache, ctx, DOF_1);
}

/// D298 / D312: DoF 2, in the post chain: the four steps over the whole scene, composited on
/// the translucent (marked) pixels without the plane test and on the others nearer than the
/// plane (FOUND part 63: GL_GREATER against the plane).
fn depth_of_field(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture, &DofView)>,
    pipelines: Res<DofPipelines>,
    cache: Res<PipelineCache>,
    ctx: RenderContext,
) {
    let (target, depth, dof) = view.into_inner();
    run_passes((target, depth, dof), &pipelines, &cache, ctx, DOF_2);
}

/// D298: source (scene -> a), ring with the gate kept (a -> b), second ring (b -> a),
/// composite (scene and a -> the main texture), with the entries `steps` of [`ENTRIES`].
/// D312: DoF 1's composite goes through the main pass's multisampled texture when there is
/// one.
fn run_passes(
    (target, depth, dof): (&ViewTarget, &ViewDepthTexture, &DofView),
    pipelines: &DofPipelines,
    cache: &PipelineCache,
    mut ctx: RenderContext,
    steps: [usize; 4],
) {
    let mut ready = Vec::with_capacity(4);
    for id in steps.map(|k| dof.pipelines[k]) {
        let Some(pipeline) = cache.get_render_pipeline(id) else {
            return;
        };
        ready.push(pipeline);
    }
    let device = ctx.render_device().clone();
    let layout = cache.get_bind_group_layout(&pipelines.layouts[usize::from(dof.multisampled)]);
    let main_pass = steps == DOF_1;
    let sampled = target.sampled_main_texture_view().filter(|_| main_pass);
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
    for (k, (pipeline, input, out)) in steps.into_iter().enumerate() {
        let bind_group = device.create_bind_group(
            "d298_dof",
            &layout,
            &BindGroupEntries::sequential((
                post.source,
                &pipelines.sampler,
                depth.view(),
                dof.range.as_entire_binding(),
                input,
                &dof.opaque_depth.default_view,
            )),
        );
        // DoF 1's composite under MSAA: every sample of the main pass's texture, resolved
        // into the new main texture (the transparent pass loads the samples).
        let (view, resolve_target) = match sampled {
            Some(msaa) if k == 3 => (msaa, Some(out)),
            _ => (out, None),
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(if main_pass {
                "d312_depth_of_field_1"
            } else {
                "d298_depth_of_field"
            }),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: resolve_target.map(|v| &**v),
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
        "const DOF_RAMP: f32 = {:?};\nconst DOF_TAP_STEP = vec2<f32>({:?}, {:?});\nconst DOF_TAP_DIRS = array<vec2<f32>, 5>({dirs});\nconst DOF_SAME_LAYER: f32 = {:?};\nconst DOF_1_SECOND_RING_SCALE: f32 = {:?};\n{BODY}",
        DOF_RAMP, DOF_TAP_STEP[0], DOF_TAP_STEP[1], DOF_SAME_LAYER, DOF_1_SECOND_RING_SCALE
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
// D312: the depth buffer after the opaque pass (before the translucent draws).
#ifdef DEPTH_MSAA
@group(0) @binding(5) var opaque_depth: texture_depth_multisampled_2d;
#else
@group(0) @binding(5) var opaque_depth: texture_depth_2d;
#endif
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
// gate) x DOF_TAP_STEP, +y down the picture (D313, as uv); a tap counts when its gate is
// above the centre's minus DOF_SAME_LAYER. Returns the mean colour and the centre's gate.
fn ring(uv: vec2<f32>, scale: f32) -> vec4<f32> {
    let centre = textureSampleLevel(half_texture, linear_sampler, uv, 0.0);
    let r = saturate(DOF_RAMP * centre.a - (DOF_RAMP - 1.0));
    var dirs = DOF_TAP_DIRS;
    var sum = centre.rgb;
    var count = 1.0;
    for (var k = 0; k < 5; k++) {
        let d = dirs[k] * DOF_TAP_STEP * (r * scale);
        let tap = textureSampleLevel(half_texture, linear_sampler, uv + d, 0.0);
        let keep = select(0.0, 1.0, tap.a > centre.a - DOF_SAME_LAYER);
        sum += tap.rgb * keep;
        count += keep;
    }
    return vec4<f32>(sum / count, centre.a);
}
@fragment
fn dof_ring_gate(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return ring(in.uv, 1.0);
}
// D312: DoF 1's second ring: twice the step, the gate kept (its composite takes 8 a - 7).
@fragment
fn dof_ring_gate_wide(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return ring(in.uv, DOF_1_SECOND_RING_SCALE);
}
@fragment
fn dof_ring_radius(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = ring(in.uv, 1.0);
    return vec4<f32>(c.rgb, saturate(DOF_RAMP * c.a - (DOF_RAMP - 1.0)));
}
// Pass 4, full size: beyond the range's start the blurred image (bilinear from half size)
// over the scene by its radius; the scene's alpha is kept. D312: DoF 1's composite (between
// the opaque and the transparent pass), its alpha 8 a - 7 from the gate read back.
@fragment
fn dof_composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.position.xy));
    let scene = textureLoad(scene_texture, p, 0);
    if (ramp_at(p) <= 0.0) {
        return scene;
    }
    let blurred = textureSampleLevel(half_texture, linear_sampler, in.uv, 0.0);
    let alpha = saturate(DOF_RAMP * blurred.a - (DOF_RAMP - 1.0));
    let e = mix(to_encoded(scene.rgb), blurred.rgb, alpha);
    return vec4<f32>(from_encoded(e), scene.a);
}
// D312: a pixel a translucent draw (the glass) wrote: the game's stencil bit 0x80.
fn translucent_at(p: vec2<i32>) -> bool {
    return textureLoad(depth_texture, p, 0) != textureLoad(opaque_depth, p, 0);
}
// D312: DoF 2's composite (part 57's post-chain form): on the translucent pixels without the
// plane test (the game's EQUAL 0x80 composite; no change where the radius is 0), on the others
// nearer than the range's start (its NOTEQUAL one, GL_GREATER against the plane, part 63).
@fragment
fn dof_composite_marked(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.position.xy));
    let scene = textureLoad(scene_texture, p, 0);
    let marked = translucent_at(p);
    if (!marked && ramp_at(p) >= 0.0) {
        return scene;
    }
    let blurred = textureSampleLevel(half_texture, linear_sampler, in.uv, 0.0);
    if (marked && blurred.a <= 0.0) {
        return scene;
    }
    let e = mix(to_encoded(scene.rgb), blurred.rgb, blurred.a);
    return vec4<f32>(from_encoded(e), scene.a);
}
"#;

/// D298: a CPU mirror of the four passes on a scene of encoded colours and ramps
/// (row-major, `w` x `h`, both even): the half-size source with its 8-bit gate, the two
/// rings (bilinear taps, clamped), and the composite (bilinear from half size where the
/// ramp is above 0). Returns the encoded colours. D312: in DoF 1's form (part 63): the second
/// ring at twice the step keeping the gate, the composite's alpha `8 a - 7` of the gate read
/// back.
#[cfg(test)]
pub fn dof_cpu(rgb: &[[f32; 3]], ramp: &[f32], w: usize, h: usize) -> Vec<[f32; 3]> {
    dof_cpu_form(rgb, ramp, None, DOF_1_SECOND_RING_SCALE, w, h)
}

/// D312: one translucent surface's pixel over the opaque scene: its colour, its coverage
/// (the scene below is mixed towards the colour by it) and its own ramp (it writes its depth).
#[cfg(test)]
#[derive(Clone, Copy)]
pub struct Translucent {
    pub rgb: [f32; 3],
    pub cover: f32,
    pub ramp: f32,
}

/// D312: a CPU mirror of the game's two passes: DoF 1 ([`dof_cpu`]) over the opaque scene,
/// then the translucent surfaces drawn over it (each writing its ramp and marking its pixels),
/// then DoF 2 ([`dof_cpu_marked`]) over the whole scene with those marks.
#[cfg(test)]
pub fn dof_two_pass_cpu(
    rgb: &[[f32; 3]],
    ramp: &[f32],
    translucent: &[Option<Translucent>],
    w: usize,
    h: usize,
) -> Vec<[f32; 3]> {
    let first = dof_cpu(rgb, ramp, w, h);
    let mut scene = first.clone();
    let mut depth = ramp.to_vec();
    let mut marked = vec![false; w * h];
    for (p, t) in translucent.iter().enumerate() {
        if let Some(t) = t {
            scene[p] = std::array::from_fn(|k| first[p][k] * (1. - t.cover) + t.rgb[k] * t.cover);
            depth[p] = t.ramp;
            marked[p] = true;
        }
    }
    dof_cpu_marked(&scene, &depth, &marked, w, h)
}

/// D312: DoF 2's form: both rings at the step, the second writing the radius; the translucent
/// pixels (the game's stencil bit 0x80) composited without the plane test and left as they are
/// where the radius read back is 0; the others only where they are nearer than the plane
/// (GL_GREATER, part 63), so beyond it nothing changes.
#[cfg(test)]
pub fn dof_cpu_marked(
    rgb: &[[f32; 3]],
    ramp: &[f32],
    marked: &[bool],
    w: usize,
    h: usize,
) -> Vec<[f32; 3]> {
    dof_cpu_form(rgb, ramp, Some(marked), 1.0, w, h)
}

/// D298 / D312: the passes in either form: `marked` `None` is DoF 1's (the second ring keeps
/// the gate, the composite takes `8 a - 7` beyond the plane), `Some` DoF 2's (the second ring
/// writes the radius; see [`dof_cpu_marked`]); `second_scale` multiplies the second ring's
/// step.
#[cfg(test)]
fn dof_cpu_form(
    rgb: &[[f32; 3]],
    ramp: &[f32],
    marked: Option<&[bool]>,
    second_scale: f32,
    w: usize,
    h: usize,
) -> Vec<[f32; 3]> {
    let bilinear = |img: &[[f32; 4]], u: f32, v: f32| bilinear_cpu(img, w / 2, h / 2, u, v);
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
    let mut img = half;
    for pass in 0..DOF_RING_PASSES {
        let last = pass + 1 == DOF_RING_PASSES;
        let scale = if last { second_scale } else { 1. };
        let mut out = dof_ring_cpu(&img, hw, hh, scale);
        if last && marked.is_some() {
            for (o, c) in out.iter_mut().zip(&img) {
                o[3] = byte(radius(c[3]));
            }
        }
        img = out;
    }
    (0..w * h)
        .map(|p| {
            let (u, v) = (
                ((p % w) as f32 + 0.5) / w as f32,
                ((p / w) as f32 + 0.5) / h as f32,
            );
            let b = match marked {
                None => {
                    if ramp[p] <= 0. {
                        return rgb[p];
                    }
                    let b = bilinear(&img, u, v);
                    [b[0], b[1], b[2], radius(b[3])]
                }
                Some(marked) => {
                    if !marked[p] && ramp[p] >= 0. {
                        return rgb[p];
                    }
                    let b = bilinear(&img, u, v);
                    if b[3] <= 0. {
                        return rgb[p];
                    }
                    b
                }
            };
            std::array::from_fn(|k| rgb[p][k] * (1. - b[3]) + b[k] * b[3])
        })
        .collect()
}

/// The 8-bit store of the half-size buffers.
#[cfg(test)]
fn byte(v: f32) -> f32 {
    (v.clamp(0., 1.) * 255.).round() / 255.
}

/// A bilinear read of a row-major `iw` x `ih` image at uv (texel centres at +0.5, clamped
/// to the edge).
#[cfg(test)]
fn bilinear_cpu(data: &[[f32; 4]], iw: usize, ih: usize, u: f32, v: f32) -> [f32; 4] {
    let (x, y) = (u * iw as f32 - 0.5, v * ih as f32 - 0.5);
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
}

/// D298 / D313: one ring over a half-size buffer (row-major, the first row the top of the
/// picture): the centre and the five taps at `radius x scale x` [`DOF_TAP_STEP`], +y down the
/// picture, counted when their gate is above the centre's minus [`DOF_SAME_LAYER`]; the mean
/// colour and the centre's gate, stored in 8 bits.
#[cfg(test)]
fn dof_ring_cpu(img: &[[f32; 4]], hw: usize, hh: usize, scale: f32) -> Vec<[f32; 4]> {
    let mut out = vec![[0.; 4]; hw * hh];
    for j in 0..hh {
        for i in 0..hw {
            let (u, v) = ((i as f32 + 0.5) / hw as f32, (j as f32 + 0.5) / hh as f32);
            let centre = img[j * hw + i];
            let r = radius(centre[3]) * scale;
            let mut sum = [centre[0], centre[1], centre[2]];
            let mut count = 1.;
            for [dx, dy] in DOF_TAP_DIRS {
                let tap = bilinear_cpu(
                    img,
                    hw,
                    hh,
                    u + dx * DOF_TAP_STEP[0] * r,
                    v + dy * DOF_TAP_STEP[1] * r,
                );
                if tap[3] > centre[3] - DOF_SAME_LAYER {
                    (0..3).for_each(|k| sum[k] += tap[k]);
                    count += 1.;
                }
            }
            out[j * hw + i] = [sum[0] / count, sum[1] / count, sum[2] / count, centre[3]].map(byte);
        }
    }
    out
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
                    // D312: DoF 1's alpha is 8 a - 7 of the gate read back, so next to the
                    // square (a lower gate) the bleed is smaller and may be none.
                    assert!(
                        out[p][0] > 0.3 - 1.5 / 255. && out[p][0] < 0.65,
                        "{x} {y} {:?}",
                        out[p]
                    );
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
                assert!(source.contains("let d = dirs[k] * DOF_TAP_STEP * (r * scale);"));
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

    /// D312 (part 62 FOUND): the glass drawn between the two passes. A near glass panel with
    /// a fine pattern (writing its depth, marked) over a far room: the room around it and
    /// under it is blurred by DoF 1, the panel's own pattern stays exactly as drawn (DoF 2
    /// finds a radius of 0 on it). The one post-chain pass (D298) blurs the pattern with the
    /// room; part 57's stencil form alone (D306) keeps both sharp.
    #[test]
    fn d312_glass_sharp_over_a_blurred_room() {
        let (w, h) = (32, 32);
        let room = checker(w, h);
        let ramp = vec![2.; w * h];
        let panel = |x: usize, y: usize| (8..24).contains(&x) && (8..24).contains(&y);
        let glass: Vec<Option<Translucent>> = (0..w * h)
            .map(|p| {
                let (x, y) = (p % w, p / w);
                panel(x, y).then(|| Translucent {
                    rgb: if x % 2 == 0 { [1.; 3] } else { [0.; 3] },
                    cover: 0.5,
                    ramp: -3.,
                })
            })
            .collect();
        let over = |base: &[[f32; 3]]| -> Vec<[f32; 3]> {
            (0..w * h)
                .map(|p| match glass[p] {
                    Some(t) => {
                        std::array::from_fn(|k| base[p][k] * (1. - t.cover) + t.rgb[k] * t.cover)
                    }
                    None => base[p],
                })
                .collect()
        };
        let two = dof_two_pass_cpu(&room, &ramp, &glass, w, h);
        // The room alone through DoF 1, then the glass over it as drawn.
        let want = over(&dof_cpu(&room, &ramp, w, h));
        let inner = |x: usize, y: usize| (10..22).contains(&x) && (10..22).contains(&y);
        for y in 0..h {
            for x in 0..w {
                let p = y * w + x;
                if inner(x, y) {
                    assert_eq!(two[p], want[p], "{x} {y}");
                }
            }
        }
        // The room beside the panel is blurred (rows 0..6 hold no glass).
        let base = gradient(&room, w, h, 1..6);
        assert!(gradient(&two, w, h, 1..6) < 0.5 * base);
        // The pattern on the panel: per-row gradient over its inner columns.
        let panel_gradient = |img: &[[f32; 3]]| {
            let mut sum = 0.;
            for y in 10..22 {
                for x in 10..21 {
                    sum += (img[y * w + x + 1][0] - img[y * w + x][0]).abs();
                }
            }
            sum / (12. * 11.)
        };
        let drawn = over(&room);
        // D298: one pass after the glass, which writes no depth: the pattern is blurred.
        let one = dof_cpu(&drawn, &ramp, w, h);
        // D306: the stencil form alone: the room under the panel stays sharp.
        let marked: Vec<bool> = glass.iter().map(Option::is_some).collect();
        let mut panel_ramp = ramp.clone();
        for (p, g) in glass.iter().enumerate() {
            if let Some(t) = g {
                panel_ramp[p] = t.ramp;
            }
        }
        let stencil = dof_cpu_marked(&drawn, &panel_ramp, &marked, w, h);
        let (g_two, g_one) = (panel_gradient(&two), panel_gradient(&one));
        assert!(g_two > 1.5 * g_one, "{g_two} {g_one}");
        for y in 10..22 {
            for x in 10..22 {
                assert_eq!(stencil[y * w + x], drawn[y * w + x], "{x} {y}");
            }
        }
        assert_ne!(two, stencil);
    }

    /// D312: with nothing beyond the start, the two passes leave the opaque scene and the
    /// glass over it exactly as drawn (the closed Pod, part 49).
    #[test]
    fn d312_nothing_in_range_unchanged() {
        let (w, h) = (16, 16);
        let room = checker(w, h);
        let glass: Vec<Option<Translucent>> = (0..w * h)
            .map(|p| {
                (p % w < 8).then_some(Translucent {
                    rgb: [0.9, 0.1, 0.4],
                    cover: 0.3,
                    ramp: -5.,
                })
            })
            .collect();
        let out = dof_two_pass_cpu(&room, &vec![-1.; w * h], &glass, w, h);
        for p in 0..w * h {
            let want = match glass[p] {
                Some(t) => std::array::from_fn(|k| room[p][k] * 0.7 + t.rgb[k] * 0.3),
                None => room[p],
            };
            assert_eq!(out[p], want, "{p}");
        }
        // Without translucent surfaces DoF 2 leaves the room beyond the plane as DoF 1 left it
        // (its unmarked composite passes only nearer than the plane, part 63): no second blur.
        let none = vec![None; w * h];
        let ramp = vec![2.; w * h];
        let once = dof_cpu(&room, &ramp, w, h);
        assert_ne!(once, room);
        assert_eq!(dof_two_pass_cpu(&room, &ramp, &none, w, h), once);
    }

    /// D312: DoF 2's composite reads the depth after the opaque pass and composites the pixels
    /// whose depth changed since then without the plane test; DoF 1's keeps the plane test.
    #[test]
    fn d312_composite_shaders() {
        let raw = shader_source();
        for msaa in [true, false] {
            let defs: Vec<&str> = if msaa { vec!["DEPTH_MSAA"] } else { vec![] };
            let source = preprocess(&raw, &defs);
            assert!(source.contains("@group(0) @binding(5) var opaque_depth"));
            assert!(source
                .contains("textureLoad(depth_texture, p, 0) != textureLoad(opaque_depth, p, 0)"));
            assert!(source.contains("if (!marked && ramp_at(p) >= 0.0)"));
            assert!(
                source.contains("let alpha = saturate(DOF_RAMP * blurred.a - (DOF_RAMP - 1.0));")
            );
            assert!(source.contains("return ring(in.uv, DOF_1_SECOND_RING_SCALE);"));
            assert!(source.contains("const DOF_1_SECOND_RING_SCALE: f32 = 2.0;"));
            assert!(source.contains("if (marked && blurred.a <= 0.0)"));
            assert_eq!(
                source.matches("texture_depth_multisampled_2d").count(),
                if msaa { 2 } else { 0 }
            );
        }
        let names = |steps: [usize; 4]| steps.map(|k| ENTRIES[k]);
        assert_eq!(
            names(DOF_1),
            [
                "dof_source",
                "dof_ring_gate",
                "dof_ring_gate_wide",
                "dof_composite"
            ]
        );
        assert_eq!(
            names(DOF_2),
            [
                "dof_source",
                "dof_ring_gate",
                "dof_ring_radius",
                "dof_composite_marked"
            ]
        );
    }

    /// D312 FOUND (part 63, 8348 / 8980): DoF 1's second ring at twice the step blurs a far
    /// pattern more than at the step; DoF 2's unmarked composite (GL_GREATER) changes nothing
    /// beyond the plane and, nearer than it, only the outline next to a blurred background
    /// (the pixels' own radius is 0; the read-back is bilinear).
    #[test]
    fn d312_dof1_wide_ring_and_dof2_nearer() {
        assert_eq!(DOF_1_SECOND_RING_SCALE, 2.);
        let (w, h) = (32, 8);
        let rgb = checker(w, h);
        let far = vec![1.; w * h];
        let wide = dof_cpu(&rgb, &far, w, h);
        let narrow = dof_cpu_form(&rgb, &far, None, 1., w, h);
        assert!(gradient(&wide, w, h, 2..6) < gradient(&narrow, w, h, 2..6));
        // A near square (ramp -3, unmarked) on a far checker (ramp 2).
        let (w, h) = (32, 32);
        let mut rgb = checker(w, h);
        let mut ramp = vec![2.; w * h];
        for y in 8..24 {
            for x in 8..24 {
                rgb[y * w + x] = [1.; 3];
                ramp[y * w + x] = -3.;
            }
        }
        let out = dof_cpu_marked(&rgb, &ramp, &vec![false; w * h], w, h);
        let mut outline = 0;
        for y in 0..h {
            for x in 0..w {
                let p = y * w + x;
                let inner = (10..22).contains(&x) && (10..22).contains(&y);
                if ramp[p] > 0. || inner {
                    assert_eq!(out[p], rgb[p], "{x} {y}");
                } else if out[p] != rgb[p] {
                    outline += 1;
                }
            }
        }
        assert!(outline > 0);
    }

    /// D313 MEASURED (part 64): +y of the taps points down the picture. On the game's
    /// 640 x 360 buffer at radius 1, one bright texel at (320, 180) reaches the centres up and
    /// to the side at (318, 179) and (322, 179) through the taps (+-0.951, +0.309) x 1.6
    /// texels (bilinear weight 0.52 x 0.51, over the six counted samples), and no centre
    /// below at (318, 181) or (322, 181); with y up it is the other way round.
    #[test]
    fn d313_ring_taps_point_down_the_picture() {
        let (hw, hh) = (640, 360);
        let mut half = vec![[0., 0., 0., 1.]; hw * hh];
        half[180 * hw + 320] = [1.; 4];
        let out = dof_ring_cpu(&half, hw, hh, 1.);
        let at = |x: usize, y: usize| out[y * hw + x][0];
        for x in [318, 322] {
            assert!((at(x, 179) - 11. / 255.).abs() < 1e-6, "{x} {}", at(x, 179));
            assert_eq!(at(x, 181), 0., "{x}");
        }
        // The gate is kept; the bright texel itself keeps a sixth (its taps all miss it).
        assert_eq!(out[180 * hw + 320][3], 1.);
        assert!(
            (at(320, 180) - 1. / 6.).abs() < 1.5 / 255.,
            "{}",
            at(320, 180)
        );
    }

    /// D313: the shader's taps and the CPU mirror's agree: +y of [`DOF_TAP_DIRS`] is +v (down
    /// the picture), with no flip of the y axis.
    #[test]
    fn d313_shader_taps_without_a_y_flip() {
        let source = preprocess(&shader_source(), &[]);
        assert!(source.contains("let d = dirs[k] * DOF_TAP_STEP * (r * scale);"));
        assert!(!source.contains("vec2<f32>(1.0, -1.0)"));
        assert!(source.contains("+y down the picture"));
    }

    /// D312: the glass writes its depth while the depth of field runs, not otherwise.
    #[test]
    fn d312_glass_writes_depth_with_the_dof() {
        assert!(crate::dual::glass_writes_depth(true));
        assert!(!crate::dual::glass_writes_depth(false));
    }

    /// D304: on by default in the Pod picture on the level-lighting route, off by default in
    /// any other level; `--depth-of-field` turns it on there, `--no-depth-of-field` off in the
    /// Pod; never off the level-lighting route.
    #[test]
    fn d304_dof_on_by_default_in_the_pod() {
        // (level_route, pod, flag) -> runs
        assert!(dof_route(true, true, None));
        assert!(dof_route(true, true, Some(true)));
        assert!(!dof_route(true, true, Some(false)));
        assert!(!dof_route(true, false, None));
        assert!(dof_route(true, false, Some(true)));
        assert!(!dof_route(true, false, Some(false)));
        for pod in [false, true] {
            for flag in [None, Some(true), Some(false)] {
                assert!(!dof_route(false, pod, flag));
            }
        }
    }
}
