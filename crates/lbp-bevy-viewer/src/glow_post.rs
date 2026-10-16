//! D292 (docs/level-scripts.md part 43): the glow on the Pod dissolve's green line, the
//! game's post chain (FOUND, frame 5247): the tent's shader writes alpha = t (0 on the
//! line), a half-resolution pass turns `0.7 x (1 - alpha) x colour` into a glow source,
//! two 9-tap blur passes at quarter resolution spread it, and the final pass adds it to
//! the scene.
//!
//! D294 (part 45): that final pass's own scale and vignette (FOUND, PS 14857),
//! `out = (scene x 1.2998 + glow) x (0.2 + 0.8 x smoothstep(0, 1, saturate(1.2 - r)))`, under
//! `--final-pass` on the level-lighting route only (off by default). With it the add pass
//! runs on every frame: with no dissolve the glow term is 0 and only the scale and the
//! vignette apply.
//!
//! D300 (part 51): the pass ends in the game's `lerp(c, scene, scene.a)` (FOUND, PS 14857 and
//! the Moon's eid 5403): where the scene's alpha is 1 it returns the scene. In the Pod's
//! frames the scene reaching it has alpha 0 on every pixel of the 3-D image: after the depth
//! of field and the glow's source pass the game copies the frame to main memory, an SPU
//! anti-aliasing job (the EBOOT's "EDGE MLAA" task) works on it there and the result comes
//! back with alpha 0 (MEASURED in frames 5247 and 14790). In a level's frames the pass is not
//! seen (part 50), so there the scene keeps its own alpha (1 on opaque surfaces). On by
//! default on the level-lighting route: the Pod picture (`--pod-camera`) mixes back by alpha
//! 0 ([`clear_scene_alpha`]), every other level by the scene's alpha, a no-op where that is
//! 1. `--no-final-pass` turns it off.
//!
//! D322 (part 73): in the Pod the scripts' main menu is drawn as the game draws it, into the
//! scene after the copy back and before this pass (FOUND, frames 8980 / 9601): its draws blend
//! premultiplied colour over the scene in encoded values (One, OneMinusSrcAlpha) and add their
//! alpha (One, One), so the pass mixes back to the menu by the menu's own alpha. The viewer
//! rasterizes the menu into a layer ([`MenuLayer`], `pod_menu.rs`) that the final entries
//! compose under the shader define `MENU_LAYER`; without the layer the entries are unchanged.
//!
//! One full-screen chain on the main 3D camera ([`EdgeGlow`]), after the main passes and
//! before egui/UI (`Core3dSystems::PostProcess`). The glow runs while a `ViewerMaterial`
//! dissolve is on and (D303, part 54) with the final pass on every frame, as the game's chain
//! runs in the Pod and in a level; its sources are the dissolve's alpha and the glowing
//! materials' (`dual.rs`). Not under `--no-edge-glow`; without the glow or the final pass the
//! frame is untouched.
use crate::dual::{self, ViewerMaterial};
use bevy::{
    core_pipeline::{tonemapping::tonemapping, Core3d, Core3dSystems, FullscreenShader},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{sampler, texture_2d},
            AddressMode, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FilterMode,
            FragmentState, Operations, PipelineCache, RenderPassColorAttachment,
            RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat,
            TextureSampleType, TextureUsages, TextureView,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        sync_component::SyncComponent,
        texture::{CachedTexture, GpuImage, TextureCache},
        view::ViewTarget,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::Shader,
};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

/// D292 FOUND (frame 5247, PS 14831, fc4): the glow source's strength,
/// `glow_src = GLOW_STRENGTH x (1 - scene.a) x scene.rgb` (encoded values).
/// D297 (docs/level-scripts.md part 48): FOUND the same in frame 14790 (the Puter open; every
/// constant of the four post passes is equal in both frames), and MEASURED against `open-t1`
/// with the final pass and the culled tent (D294, D296): on the left post, where the
/// viewer's background is flat, the halo at 2 / 3 / 9 game px is the game's within 0.7
/// levels. Part 43's 0.4-0.45 for t1 came from the doubled line and is withdrawn.
pub const GLOW_STRENGTH: f32 = 0.7;
/// D292 FOUND (PS 14839, passes 3281/3298): the blur's tap offsets in full-resolution
/// pixels (centre, then each side): 0, +-2, +-6, +-10, +-14 (+-1/3/5/7 half-res texels).
pub const BLUR_OFFSETS: [f32; 5] = [0., 2., 6., 10., 14.];
/// D292 FOUND (PS 14839): the blur's weights per tap (each side tap twice); not
/// normalised: one pass sums to 1.38.
pub const BLUR_WEIGHTS: [f32; 5] = [0.3, 0.225, 0.15, 0.105, 0.06];
/// D294 FOUND (frame 5247, final pass PS 14857, fc13): the whole frame's scale before the
/// glow is added, `c = scene x FINAL_SCALE + glow` (encoded values).
pub const FINAL_SCALE: f32 = 1.2998;
/// D294 FOUND (PS 14857, fc15): the vignette's radius axes, `r = length(((u - 0.5) x 1.0996,
/// (v - 0.5) x 1.7002))` over the viewport's uv.
pub const VIGNETTE_AXES: [f32; 2] = [1.0996, 1.7002];
/// D294 FOUND (PS 14857, fc18.w): where the vignette starts, `smoothstep(0, 1,
/// saturate(VIGNETTE_START - r))` (fc26 = 3, the smoothstep's 3 - 2x).
pub const VIGNETTE_START: f32 = 1.2;
/// D294 FOUND (PS 14857, fc12 = fc2 = fc8 = 0.2): the vignette's floor, `FLOOR + (1 - FLOOR)
/// x smoothstep(..)` (the colour matrix fc14..25 is the identity, fc16/fc24 0).
pub const VIGNETTE_FLOOR: f32 = 0.2;

/// D292 `--no-edge-glow`: the glow chain never runs (A/B captures). `--no-glow` is P3h's.
static NO_EDGE_GLOW: AtomicBool = AtomicBool::new(false);
pub fn disable() {
    NO_EDGE_GLOW.store(true, Ordering::Relaxed);
}
/// D294 `--final-pass` (level-lighting route only, `main.rs`): the final pass's scale and
/// vignette on every frame. D300: on by default on that route (`--no-final-pass` off).
static FINAL_PASS: AtomicBool = AtomicBool::new(false);
pub fn enable_final_pass() {
    FINAL_PASS.store(true, Ordering::Relaxed);
}
/// Whether the final pass runs (D322: the menu is then composed in it).
pub fn final_pass_on() -> bool {
    FINAL_PASS.load(Ordering::Relaxed)
}

/// D322: the menu layer the final pass composes (FOUND, frames 8980 / 9601: the menu is drawn
/// into the scene after the copy back, before the pass): `colour` holds the menu's
/// premultiplied colour in encoded values and 1 - its transmittance (`Rgba8Unorm`, the size
/// of the window), `alpha` the sum of its draws' alphas (`R8Unorm`, the game's alpha target).
/// Composed only while `on`.
#[derive(Resource, Clone, Default)]
pub struct MenuLayer {
    pub colour: Handle<Image>,
    pub alpha: Handle<Image>,
    pub on: bool,
}
impl ExtractResource for MenuLayer {
    type Source = Self;
    fn extract_resource(source: &Self) -> Self {
        source.clone()
    }
}
/// D300: the scene's alpha as the final pass sees it in the Pod's frames: 0 on every pixel
/// (the anti-aliasing job's copy back, FOUND frames 5247 / 14790). Set for the Pod picture
/// (`--pod-camera`); otherwise the pass mixes back by the scene's own alpha.
static SCENE_ALPHA_CLEARED: AtomicBool = AtomicBool::new(false);
pub fn clear_scene_alpha() {
    SCENE_ALPHA_CLEARED.store(true, Ordering::Relaxed);
}
/// D300: whether the final pass runs, and whether it mixes back by alpha 0 (the Pod): on the
/// level-lighting route unless `--no-final-pass` (`flag` `Some(false)`; `--final-pass` is
/// `Some(true)` and changes nothing there), never elsewhere.
pub fn final_pass_route(level_route: bool, flag: Option<bool>, pod: bool) -> (bool, bool) {
    let on = level_route && flag != Some(false);
    (on, on && pod)
}

/// D292: on the camera the glow chain runs for; `active` while a dissolve is on (the glow),
/// D294 `final_pass` under `--final-pass` (the scale and vignette, every frame), D300
/// `alpha_cleared` when the pass mixes back by alpha 0 (the Pod) instead of the scene's.
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct EdgeGlow {
    pub active: bool,
    pub final_pass: bool,
    pub alpha_cleared: bool,
}
impl SyncComponent for EdgeGlow {
    type Target = Self;
}
/// Extracted only while the glow or the final pass is on: otherwise the chain skips.
impl ExtractComponent for EdgeGlow {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(item: &Self) -> Option<Self> {
        (item.active || item.final_pass).then_some(*item)
    }
}

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("8e3b6a2d-41f7-4c5e-9a0b-d29200000001");
/// The glow source and the blurs are stored as plain floats (encoded-space numbers).
const GLOW_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

pub struct GlowPostPlugin;
impl Plugin for GlowPostPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                SHADER.id(),
                Shader::from_wgsl(shader_source(), "d292-edge-glow.wgsl"),
            )
            .unwrap();
        app.add_plugins((
            ExtractComponentPlugin::<EdgeGlow>::default(),
            ExtractResourcePlugin::<MenuLayer>::default(),
        ))
        .add_systems(PostUpdate, update_active);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(Render, prepare_glow.in_set(RenderSystems::PrepareResources))
            .add_systems(
                Core3d,
                edge_glow
                    .in_set(Core3dSystems::PostProcess)
                    .after(tonemapping),
            );
    }
}

/// D303: the glow's passes run with the final pass on every frame (FOUND: the game runs the
/// chain in the Pod's frames and in the Moon's, part 50), and otherwise while a dissolve is on
/// (D292); never under `--no-edge-glow`.
pub fn glow_runs(no_edge_glow: bool, final_pass: bool, dissolving: bool) -> bool {
    !no_edge_glow && (final_pass || dissolving)
}

/// D292: the glow is on while any viewer material dissolves (and not `--no-edge-glow`);
/// D294: the final pass while `--final-pass` is on. D303: the glow with the final pass too.
fn update_active(materials: Res<Assets<ViewerMaterial>>, mut cameras: Query<&mut EdgeGlow>) {
    let final_pass = FINAL_PASS.load(Ordering::Relaxed);
    let dissolving = materials.iter().any(|(_, m)| dual::dissolve_on(m));
    let on = glow_runs(NO_EDGE_GLOW.load(Ordering::Relaxed), final_pass, dissolving);
    let alpha_cleared = SCENE_ALPHA_CLEARED.load(Ordering::Relaxed);
    for mut glow in &mut cameras {
        let next = EdgeGlow {
            active: on,
            final_pass,
            alpha_cleared,
        };
        if (glow.active, glow.final_pass, glow.alpha_cleared) != (on, final_pass, alpha_cleared) {
            *glow = next;
        }
    }
}

/// D294: the add pass's entry point: the glow alone (D292), the glow and the final pass, or
/// the final pass with no glow (its glow term 0).
fn composite_entry(glow: bool, final_pass: bool) -> &'static str {
    match (glow, final_pass) {
        (true, false) => "glow_composite",
        (true, true) => "final_composite",
        (false, _) => "final_only",
    }
}

#[derive(Resource)]
pub(crate) struct GlowPipelines {
    /// One texture and the sampler (source and blur passes).
    single: BindGroupLayoutDescriptor,
    /// The scene, the sampler, the blurred glow (the final add) and D322's menu layer
    /// (colour and alpha; the scene in their place when there is no layer, unread).
    composite: BindGroupLayoutDescriptor,
    sampler: Sampler,
    source: HashMap<TextureFormat, CachedRenderPipelineId>,
    composite_ids: HashMap<(TextureFormat, &'static str, bool, bool), CachedRenderPipelineId>,
    blur_h: CachedRenderPipelineId,
    blur_v: CachedRenderPipelineId,
    vertex: bevy::render::render_resource::VertexState,
}

/// Whether the main texture holds linear values that the hardware encodes (sRGB views),
/// encoded values as they are (plain 8-bit), or neither (float/HDR: not supported).
pub(crate) fn target_space(format: TextureFormat) -> Option<bool> {
    match format {
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb => Some(true),
        TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm => Some(false),
        _ => None,
    }
}

fn descriptor(
    vertex: &bevy::render::render_resource::VertexState,
    layout: &BindGroupLayoutDescriptor,
    entry: &'static str,
    format: TextureFormat,
    srgb: bool,
    alpha_cleared: bool,
    menu: bool,
) -> RenderPipelineDescriptor {
    let mut shader_defs = Vec::new();
    if srgb {
        shader_defs.push("SRGB_TARGET".into());
    }
    if alpha_cleared {
        shader_defs.push("SCENE_ALPHA_CLEARED".into());
    }
    if menu {
        shader_defs.push("MENU_LAYER".into());
    }
    RenderPipelineDescriptor {
        label: Some(format!("d292_{entry}").into()),
        layout: vec![layout.clone()],
        vertex: vertex.clone(),
        fragment: Some(FragmentState {
            shader: SHADER,
            shader_defs,
            entry_point: Some(entry.into()),
            targets: vec![Some(ColorTargetState {
                format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        ..default()
    }
}

fn init_pipelines(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
) {
    let single = BindGroupLayoutDescriptor::new(
        "d292_glow_single",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    let composite = BindGroupLayoutDescriptor::new(
        "d292_glow_composite",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        ),
    );
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("d292_glow_sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    let vertex = fullscreen.to_vertex_state();
    let blur_h = cache.queue_render_pipeline(descriptor(
        &vertex,
        &single,
        "glow_blur_h",
        GLOW_FORMAT,
        false,
        false,
        false,
    ));
    let blur_v = cache.queue_render_pipeline(descriptor(
        &vertex,
        &single,
        "glow_blur_v",
        GLOW_FORMAT,
        false,
        false,
        false,
    ));
    commands.insert_resource(GlowPipelines {
        single,
        composite,
        sampler,
        source: HashMap::new(),
        composite_ids: HashMap::new(),
        blur_h,
        blur_v,
        vertex,
    });
}

/// D292: the per-view pieces: the intermediate textures and the pipelines for the
/// view's main texture format.
#[derive(Component)]
pub(crate) struct GlowView {
    half: CachedTexture,
    quarter_h: CachedTexture,
    quarter_v: CachedTexture,
    source: CachedRenderPipelineId,
    composite: CachedRenderPipelineId,
    /// D322: the same with the menu layer composed.
    composite_menu: CachedRenderPipelineId,
    /// D294: whether the glow's three passes run (else only the final pass, glow 0).
    glow: bool,
}

fn prepare_glow(
    mut commands: Commands,
    mut pipelines: ResMut<GlowPipelines>,
    cache: Res<PipelineCache>,
    mut textures: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    views: Query<(Entity, &ViewTarget, &EdgeGlow)>,
    stale: Query<Entity, (With<GlowView>, Without<EdgeGlow>)>,
) {
    for entity in &stale {
        commands.entity(entity).remove::<GlowView>();
    }
    for (entity, target, marker) in &views {
        let format = target.main_texture_format();
        let Some(srgb) = target_space(format) else {
            warn_once!("D292: edge glow skipped: main texture format {format:?}");
            commands.entity(entity).remove::<GlowView>();
            continue;
        };
        let p = &mut *pipelines;
        let source = *p.source.entry(format).or_insert_with(|| {
            cache.queue_render_pipeline(descriptor(
                &p.vertex,
                &p.single,
                "glow_source",
                GLOW_FORMAT,
                srgb,
                false,
                false,
            ))
        });
        let entry = composite_entry(marker.active, marker.final_pass);
        let cleared = marker.alpha_cleared;
        let mut composite = |menu: bool| {
            *p.composite_ids
                .entry((format, entry, cleared, menu))
                .or_insert_with(|| {
                    cache.queue_render_pipeline(descriptor(
                        &p.vertex,
                        &p.composite,
                        entry,
                        format,
                        srgb,
                        cleared,
                        menu,
                    ))
                })
        };
        let (composite, composite_menu) = (composite(false), composite(true));
        let size = target.main_texture().size();
        let mut texture = |label: &'static str, div: u32| {
            textures.get(
                &device,
                TextureDescriptor {
                    label: Some(label),
                    size: Extent3d {
                        width: size.width.div_ceil(div).max(1),
                        height: size.height.div_ceil(div).max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: GLOW_FORMAT,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        let view = GlowView {
            half: texture("d292_glow_half", 2),
            quarter_h: texture("d292_glow_quarter_h", 4),
            quarter_v: texture("d292_glow_quarter_v", 4),
            source,
            composite,
            composite_menu,
            glow: marker.active,
        };
        commands.entity(entity).insert(view);
    }
}

/// D292: the four passes: source (half res), horizontal and vertical blur (quarter res),
/// and the add into the main texture (alpha 1). D294: with the glow off only the add pass
/// runs (the final pass, glow 0; the scene is bound in the glow's slot, unread).
/// D302: under `--mlaa` the add pass reads the anti-aliased scene (`mlaa_post.rs`), the
/// source pass the scene before it, as the game's (part 53).
/// D322: with a [`MenuLayer`] that is on and uploaded, the add pass composes the menu
/// (`MENU_LAYER`; the final entries read it, `pod_menu` turns it on only with the final pass).
pub(crate) fn edge_glow(
    view: ViewQuery<(&ViewTarget, &GlowView, Option<&crate::mlaa_post::MlaaView>), With<EdgeGlow>>,
    pipelines: Res<GlowPipelines>,
    cache: Res<PipelineCache>,
    (layer, gpu_images): (Option<Res<MenuLayer>>, Res<RenderAssets<GpuImage>>),
    mut ctx: RenderContext,
) {
    let (target, glow, mlaa) = view.into_inner();
    let menu = layer
        .as_deref()
        .filter(|l| l.on)
        .and_then(|l| Some((gpu_images.get(&l.colour)?, gpu_images.get(&l.alpha)?)))
        .filter(|_| cache.get_render_pipeline(glow.composite_menu).is_some());
    let composite_id = if menu.is_some() {
        glow.composite_menu
    } else {
        glow.composite
    };
    let (Some(source), Some(blur_h), Some(blur_v), Some(composite)) = (
        cache.get_render_pipeline(glow.source),
        cache.get_render_pipeline(pipelines.blur_h),
        cache.get_render_pipeline(pipelines.blur_v),
        cache.get_render_pipeline(composite_id),
    ) else {
        return;
    };
    let device = ctx.render_device().clone();
    let single = cache.get_bind_group_layout(&pipelines.single);
    let bind_single = |texture: &TextureView| {
        device.create_bind_group(
            "d292_glow_single",
            &single,
            &BindGroupEntries::sequential((texture, &pipelines.sampler)),
        )
    };
    let post = target.post_process_write();
    let scene = match mlaa {
        Some(m) if m.into_glow => m.scene(),
        _ => post.source,
    };
    let glow_view = if glow.glow {
        &glow.quarter_v.default_view
    } else {
        post.source
    };
    let (menu_colour, menu_alpha) = match menu {
        Some((c, a)) => (&c.texture_view, &a.texture_view),
        None => (post.source, post.source),
    };
    let steps = [
        (source, bind_single(post.source), &glow.half.default_view),
        (
            blur_h,
            bind_single(&glow.half.default_view),
            &glow.quarter_h.default_view,
        ),
        (
            blur_v,
            bind_single(&glow.quarter_h.default_view),
            &glow.quarter_v.default_view,
        ),
        (
            composite,
            device.create_bind_group(
                "d292_glow_composite",
                &cache.get_bind_group_layout(&pipelines.composite),
                &BindGroupEntries::sequential((
                    scene,
                    &pipelines.sampler,
                    glow_view,
                    menu_colour,
                    menu_alpha,
                )),
            ),
            post.destination,
        ),
    ];
    let encoder = ctx.command_encoder();
    let skip = if glow.glow { 0 } else { 3 };
    for (pipeline, bind_group, out) in &steps[skip..] {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("d292_edge_glow"),
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
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The WGSL of the chain, with the FOUND constants written in from the consts above.
fn shader_source() -> String {
    let list = |v: [f32; 5]| {
        v.iter()
            .map(|x| format!("{x:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "const GLOW_STRENGTH: f32 = {:?};\nconst BLUR_OFFSETS = array<f32, 5>({});\nconst BLUR_WEIGHTS = array<f32, 5>({});\nconst FINAL_SCALE: f32 = {:?};\nconst VIGNETTE_AXES = vec2<f32>({:?}, {:?});\nconst VIGNETTE_START: f32 = {:?};\nconst VIGNETTE_FLOOR: f32 = {:?};\n{BODY}",
        GLOW_STRENGTH,
        list(BLUR_OFFSETS),
        list(BLUR_WEIGHTS),
        FINAL_SCALE,
        VIGNETTE_AXES[0],
        VIGNETTE_AXES[1],
        VIGNETTE_START,
        VIGNETTE_FLOOR
    )
}

const BODY: &str = r#"
// D292: the game's glow chain (PS 14831, 14839, 14857), in its encoded output space.
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var glow_texture: texture_2d<f32>;
#ifdef MENU_LAYER
// D322: the menu layer (premultiplied encoded colour, 1 - transmittance) and its summed alpha.
@group(0) @binding(3) var menu_texture: texture_2d<f32>;
@group(0) @binding(4) var menu_alpha: texture_2d<f32>;
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
// The main texture's value as the game's 8-bit number: an sRGB view samples linear light.
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
// Pass 3256: one texel per 2x2 scene pixels (the game's one bilinear tap), then
// GLOW_STRENGTH x (1 - alpha) x colour. Alpha out 0.
@fragment
fn glow_source(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(source_texture));
    let base = vec2<i32>(floor(in.position.xy)) * 2;
    var rgb = vec3<f32>(0.0);
    var a = 0.0;
    for (var k = 0; k < 4; k++) {
        let p = min(base + vec2<i32>(k & 1, k >> 1u), size - vec2<i32>(1));
        let t = textureLoad(source_texture, p, 0);
        rgb += to_encoded(t.rgb);
        a += t.a;
    }
    rgb *= 0.25;
    a = clamp(a * 0.25, 0.0, 1.0);
    return vec4<f32>(GLOW_STRENGTH * (1.0 - a) * rgb, 0.0);
}
// Passes 3281/3298: nine bilinear taps at full-resolution offsets `step` x BLUR_OFFSETS,
// weights BLUR_WEIGHTS (sum 1.38, not normalised).
fn blur(uv: vec2<f32>, step: vec2<f32>) -> vec4<f32> {
    var offsets = BLUR_OFFSETS;
    var weights = BLUR_WEIGHTS;
    var sum = textureSampleLevel(source_texture, source_sampler, uv, 0.0).rgb * weights[0];
    for (var k = 1; k < 5; k++) {
        let d = step * offsets[k];
        let pair = textureSampleLevel(source_texture, source_sampler, uv + d, 0.0).rgb
            + textureSampleLevel(source_texture, source_sampler, uv - d, 0.0).rgb;
        sum += pair * weights[k];
    }
    return vec4<f32>(sum, 0.0);
}
// Horizontal, reading the half-res source (one full-res pixel = half a texel).
@fragment
fn glow_blur_h(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(source_texture));
    return blur(in.uv, vec2<f32>(1.0 / (2.0 * size.x), 0.0));
}
// Vertical, reading the quarter-res horizontal blur (one full-res pixel = a quarter texel).
@fragment
fn glow_blur_v(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(source_texture));
    return blur(in.uv, vec2<f32>(0.0, 1.0 / (4.0 * size.y)));
}
// Pass 3454 without its whole-frame scale (part 42): scene + glow (bilinear from quarter res), alpha 1.
@fragment
fn glow_composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let scene = textureLoad(source_texture, vec2<i32>(floor(in.position.xy)), 0);
    let glow = textureSampleLevel(glow_texture, source_sampler, in.uv, 0.0).rgb;
    let e = min(to_encoded(scene.rgb) + glow, vec3<f32>(1.0));
    return vec4<f32>(from_encoded(e), 1.0);
}
// D294, pass 3454 (PS 14857) itself: the vignette over the viewport's uv (pixel centres),
// VIGNETTE_FLOOR + (1 - VIGNETTE_FLOOR) x smoothstep(0, 1, saturate(VIGNETTE_START - r)).
fn vignette(position: vec2<f32>) -> f32 {
    let uv = position / vec2<f32>(textureDimensions(source_texture));
    let r = length((uv - vec2<f32>(0.5)) * VIGNETTE_AXES);
    let s = smoothstep(0.0, 1.0, saturate(VIGNETTE_START - r));
    return VIGNETTE_FLOOR + (1.0 - VIGNETTE_FLOOR) * s;
}
// D300: the pass's last step, lerp(c, scene, a) in encoded values, saturated at the end
// (the 8-bit target), alpha 1. `a` is 0 in the Pod (the anti-aliasing job's copy back), else
// the scene's own alpha; where that is 1 the scene comes back as it is.
fn final_mix(scene: vec4<f32>, c: vec3<f32>) -> vec4<f32> {
#ifdef SCENE_ALPHA_CLEARED
    let a = 0.0;
#else
    let a = clamp(scene.a, 0.0, 1.0);
#endif
    if a >= 1.0 {
        return vec4<f32>(scene.rgb, 1.0);
    }
    let e = mix(c, to_encoded(scene.rgb), a);
    return vec4<f32>(from_encoded(min(e, vec3<f32>(1.0))), 1.0);
}
#ifdef MENU_LAYER
// D322: the scene with the menu drawn into it as the game draws it after the copy back
// (FOUND, frames 8980 / 9601): premultiplied colour over the scene in encoded values
// (One, OneMinusSrcAlpha), xyz; the menu's alpha added to the scene's (One, One), w.
fn with_menu(scene: vec4<f32>, position: vec2<f32>) -> vec4<f32> {
    let p = min(vec2<i32>(floor(position)), vec2<i32>(textureDimensions(menu_texture)) - vec2<i32>(1));
    let m = textureLoad(menu_texture, p, 0);
    let add = textureLoad(menu_alpha, p, 0).r;
#ifdef SCENE_ALPHA_CLEARED
    let a = clamp(add, 0.0, 1.0);
#else
    let a = clamp(scene.a + add, 0.0, 1.0);
#endif
    return vec4<f32>(m.rgb + (1.0 - m.a) * to_encoded(scene.rgb), a);
}
// The pass's last step on the scene with the menu (encoded), saturated at the end.
fn final_mix_menu(e: vec4<f32>, c: vec3<f32>) -> vec4<f32> {
    let mixed = mix(c, e.rgb, e.a);
    return vec4<f32>(from_encoded(min(mixed, vec3<f32>(1.0))), 1.0);
}
#endif
// c = (scene x FINAL_SCALE + glow) x vignette, then the mix back by the scene's alpha.
@fragment
fn final_composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let scene = textureLoad(source_texture, vec2<i32>(floor(in.position.xy)), 0);
    let glow = textureSampleLevel(glow_texture, source_sampler, in.uv, 0.0).rgb;
#ifdef MENU_LAYER
    let e = with_menu(scene, in.position.xy);
    let c = (e.rgb * FINAL_SCALE + glow) * vignette(in.position.xy);
    return final_mix_menu(e, c);
#else
    let c = (to_encoded(scene.rgb) * FINAL_SCALE + glow) * vignette(in.position.xy);
    return final_mix(scene, c);
#endif
}
// The same with no dissolve (the glow term 0).
@fragment
fn final_only(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let scene = textureLoad(source_texture, vec2<i32>(floor(in.position.xy)), 0);
#ifdef MENU_LAYER
    let e = with_menu(scene, in.position.xy);
    let c = e.rgb * FINAL_SCALE * vignette(in.position.xy);
    return final_mix_menu(e, c);
#else
    let c = to_encoded(scene.rgb) * FINAL_SCALE * vignette(in.position.xy);
    return final_mix(scene, c);
#endif
}
"#;

/// D294: a CPU mirror of the final pass's vignette at radius `r`.
#[cfg(test)]
pub fn vignette_at(r: f32) -> f32 {
    let s = (VIGNETTE_START - r).clamp(0., 1.);
    VIGNETTE_FLOOR + (1. - VIGNETTE_FLOOR) * s * s * (3. - 2. * s)
}
/// D294: the vignette at pixel (x, y) (centres at +0.5) of a `w` x `h` frame.
#[cfg(test)]
pub fn vignette_cpu(x: u32, y: u32, w: u32, h: u32) -> f32 {
    let u = (x as f32 + 0.5) / w as f32 - 0.5;
    let v = (y as f32 + 0.5) / h as f32 - 0.5;
    vignette_at((u * VIGNETTE_AXES[0]).hypot(v * VIGNETTE_AXES[1]))
}
/// D294: the final pass on one encoded colour with its glow, saturated at the end (the
/// scene's alpha 0, as in the Pod's frames).
#[cfg(test)]
pub fn final_pass_cpu(scene: [f32; 3], glow: [f32; 3], vignette: f32) -> [f32; 3] {
    final_pass_mix_cpu(scene, 0., glow, vignette)
}
/// D300: the same with the game's last step, `lerp(c, scene, alpha)` before the saturate.
#[cfg(test)]
pub fn final_pass_mix_cpu(scene: [f32; 3], alpha: f32, glow: [f32; 3], vignette: f32) -> [f32; 3] {
    let a = alpha.clamp(0., 1.);
    std::array::from_fn(|k| {
        let c = (scene[k] * FINAL_SCALE + glow[k]) * vignette;
        (c + (scene[k] - c) * a).min(1.)
    })
}

/// D292: a CPU mirror of the chain on a scene of encoded colours and alphas (row-major,
/// `w` x `h`, both multiples of 4): the half-res source, the two blurs with bilinear
/// taps (clamped), and the glow at each full-res pixel (bilinear from quarter res).
#[cfg(test)]
pub fn glow_cpu(rgb: &[[f32; 3]], alpha: &[f32], w: usize, h: usize) -> Vec<[f32; 3]> {
    type Img = (Vec<[f32; 3]>, usize, usize);
    let bilinear = |img: &Img, x: f32, y: f32| -> [f32; 3] {
        let (data, iw, ih) = img;
        let (x, y) = (x - 0.5, y - 0.5);
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
    let (hw, hh) = (w / 2, h / 2);
    let mut half = vec![[0.; 3]; hw * hh];
    for j in 0..hh {
        for i in 0..hw {
            let (mut c, mut a) = ([0.; 3], 0.);
            for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let p = (2 * j + dj) * w + 2 * i + di;
                (0..3).for_each(|k| c[k] += rgb[p][k] * 0.25);
                a += alpha[p] * 0.25;
            }
            half[j * hw + i] = c.map(|v| GLOW_STRENGTH * (1. - a) * v);
        }
    }
    let half: Img = (half, hw, hh);
    let (qw, qh) = (w / 4, h / 4);
    // Texel coordinates of quarter texel (i, j) in an image with `div` full-res px/texel.
    let blur = |img: &Img, div: f32, dir: (f32, f32)| -> Img {
        let mut out = vec![[0.; 3]; qw * qh];
        for j in 0..qh {
            for i in 0..qw {
                let (cx, cy) = ((4 * i + 2) as f32, (4 * j + 2) as f32);
                let mut sum = [0.; 3];
                for (k, (&o, &wt)) in BLUR_OFFSETS.iter().zip(&BLUR_WEIGHTS).enumerate() {
                    let sides: &[f32] = if k == 0 { &[0.] } else { &[1., -1.] };
                    for s in sides {
                        let v =
                            bilinear(img, (cx + s * o * dir.0) / div, (cy + s * o * dir.1) / div);
                        (0..3).for_each(|c| sum[c] += v[c] * wt);
                    }
                }
                out[j * qw + i] = sum;
            }
        }
        (out, qw, qh)
    };
    let quarter_h = blur(&half, 2., (1., 0.));
    let quarter_v = blur(&quarter_h, 4., (0., 1.));
    (0..w * h)
        .map(|p| {
            let (x, y) = ((p % w) as f32 + 0.5, (p / w) as f32 + 0.5);
            bilinear(&quarter_v, x / 4., y / 4.)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    /// D292: the blur's weights are the game's: 1.38 per pass (not normalised).
    #[test]
    fn d292_blur_weights_sum_to_1_38() {
        let sum = BLUR_WEIGHTS[0] + 2. * BLUR_WEIGHTS[1..].iter().sum::<f32>();
        assert!((sum - 1.38).abs() < 1e-6, "{sum}");
        assert_eq!(BLUR_OFFSETS, [0., 2., 6., 10., 14.]);
        assert_eq!(GLOW_STRENGTH, 0.7);
    }
    /// The glow of a 3 px alpha-0 line of COL0 over black opaque ground, per phase of the
    /// line against the 4 px grid: on the line (peak) and at d px above and below it.
    fn line_profile(top: usize) -> (f32, Vec<f32>, Vec<f32>) {
        let (w, h) = (16, 96);
        let col0 = [88. / 255., 192. / 255., 14. / 255.];
        let mut rgb = vec![[0.; 3]; w * h];
        let mut alpha = vec![1.; w * h];
        for y in top..top + 3 {
            for x in 0..w {
                rgb[y * w + x] = col0;
                alpha[y * w + x] = 0.;
            }
        }
        let glow = glow_cpu(&rgb, &alpha, w, h);
        let g = |y: usize| glow[y * w + w / 2][1] / col0[1];
        // Red and blue in the same ratio as green: the glow is the colour scaled.
        let p = (top + 1) * w + w / 2;
        assert!((glow[p][0] / col0[0] - glow[p][1] / col0[1]).abs() < 1e-4);
        let peak = (top..top + 3).map(g).fold(0f32, f32::max);
        let at = [1, 2, 3, 6, 9];
        (
            peak,
            at.iter().map(|&d| g(top - d)).collect(),
            at.iter().map(|&d| g(top + 2 + d)).collect(),
        )
    }
    /// D292: the line's glow against the notes' model (glow/COL0 about 0.35 on the line,
    /// 0.23 / 0.17 / 0.13 / 0.10 / 0.07 at 1 / 2 / 3 / 6 / 9 px): averaged over the four
    /// phases within 0.02 off the line; on the line 0.24-0.30 here (the notes' 0.35 is
    /// from a session model not kept; INFERRED: its sampling of the half-res pass).
    #[test]
    fn d292_line_glow_follows_the_model() {
        let notes = [0.23, 0.17, 0.13, 0.10, 0.07];
        let mut mean = [0f32; 5];
        let mut peaks = Vec::new();
        for top in 40..44 {
            let (peak, above, below) = line_profile(top);
            peaks.push(peak);
            for k in 0..5 {
                mean[k] += (above[k] + below[k]) / 8.;
            }
            // Falls off away from the line on both sides, gone by the blur's reach.
            for side in [&above, &below] {
                assert!(side.windows(2).all(|p| p[1] <= p[0] + 1e-6), "{side:?}");
            }
        }
        for k in 0..5 {
            assert!((mean[k] - notes[k]).abs() < 0.02, "{k}: {mean:?}");
        }
        for p in &peaks {
            assert!((0.24..0.31).contains(p), "{peaks:?}");
        }
        // Gone well past the reach (14 px + the half/quarter resampling).
        let (_, _, below) = line_profile(40);
        let far = {
            let (w, h) = (16, 96);
            let mut rgb = vec![[0.; 3]; w * h];
            let mut alpha = vec![1.; w * h];
            for y in 40..43 {
                for x in 0..w {
                    rgb[y * w + x] = [1.; 3];
                    alpha[y * w + x] = 0.;
                }
            }
            glow_cpu(&rgb, &alpha, w, h)[(42 + 24) * w + 8][1]
        };
        assert!(below[4] > 0.03 && far == 0., "{below:?} {far}");
    }
    /// D297: an alpha-0 line of COL0 about 2.3 viewer px thick (the game's ~2 px line at the
    /// change-over's 0.877 scale; the mean of a 2 and a 3 px line) through the chain at
    /// GLOW_STRENGTH, times the vignette at the post tops (0.83), as green excess (8-bit)
    /// above the line: within 20% of `open-t1`'s left post (MEASURED, part 48) at 2 and
    /// 3 px (18.8 / 12.3), and inside the blur's reach still above 0 at 9 px.
    #[test]
    fn d297_line_halo_matches_t1_at_0_7() {
        let (w, h) = (16, 96);
        let col0 = [88. / 255., 192. / 255., 14. / 255.];
        let vig = 0.83;
        let mut halo = [0f32; 3];
        for thick in [2, 3] {
            for top in 40..44 {
                let mut rgb = vec![[0.; 3]; w * h];
                let mut alpha = vec![1.; w * h];
                for y in top..top + thick {
                    for x in 0..w {
                        rgb[y * w + x] = col0;
                        alpha[y * w + x] = 0.;
                    }
                }
                let glow = glow_cpu(&rgb, &alpha, w, h);
                for (k, d) in [2, 3, 9].into_iter().enumerate() {
                    let g = glow[(top - d) * w + w / 2];
                    halo[k] += vig * 255. * (g[1] - (g[0] + g[2]) / 2.) / 8.;
                }
            }
        }
        for (got, game) in halo[..2].iter().zip([18.8f32, 12.3]) {
            assert!((got / game - 1.).abs() < 0.2, "{halo:?}");
        }
        assert!(halo[2] > 0. && halo[2] < halo[1], "{halo:?}");
    }
    /// D292: opaque everywhere (alpha 1) gives no glow at all, whatever the colour.
    #[test]
    fn d292_opaque_scene_has_no_glow() {
        let (w, h) = (8, 8);
        let rgb: Vec<[f32; 3]> = (0..w * h).map(|i| [i as f32 / 64., 1., 0.5]).collect();
        let glow = glow_cpu(&rgb, &vec![1.; w * h], w, h);
        assert!(glow.iter().flatten().all(|&v| v == 0.));
    }
    /// Resolves `#ifdef SRGB_TARGET` / `#else` / `#endif` for the given choice (D300: and
    /// `SCENE_ALPHA_CLEARED`, off here; [`preprocess_with`] sets it).
    fn preprocess(srgb: bool) -> String {
        preprocess_with(srgb, false)
    }
    fn preprocess_with(srgb: bool, cleared: bool) -> String {
        preprocess_all(srgb, cleared, false)
    }
    fn preprocess_all(srgb: bool, cleared: bool, menu: bool) -> String {
        let mut active = vec![true];
        let mut body = String::new();
        for line in shader_source().lines() {
            let t = line.trim_start();
            if let Some(def) = t.strip_prefix("#ifdef ") {
                let on = match def.trim() {
                    "SRGB_TARGET" => srgb,
                    "SCENE_ALPHA_CLEARED" => cleared,
                    "MENU_LAYER" => menu,
                    other => panic!("{other}"),
                };
                let parent = *active.last().unwrap();
                active.push(parent && on);
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
    /// D292: the chain's shader carries the FOUND constants, adds (no x1.2998) with alpha
    /// 1, and validates with naga for both target kinds.
    #[test]
    fn d292_glow_shader_validates() {
        for srgb in [true, false] {
            let source = preprocess(srgb);
            assert!(source.contains("const GLOW_STRENGTH: f32 = 0.7;"));
            assert!(
                source.contains("const BLUR_OFFSETS = array<f32, 5>(0.0, 2.0, 6.0, 10.0, 14.0);")
            );
            assert!(source
                .contains("const BLUR_WEIGHTS = array<f32, 5>(0.3, 0.225, 0.15, 0.105, 0.06);"));
            assert!(source.contains("GLOW_STRENGTH * (1.0 - a) * rgb"));
            assert!(source.contains("min(to_encoded(scene.rgb) + glow, vec3<f32>(1.0))"));
            assert!(source.contains("vec4<f32>(from_encoded(e), 1.0)"));
            // D292's add has no scale; D294's is a named const in the final entries.
            let d292 = &source[source.find("fn glow_composite").unwrap()..];
            let d292 = &d292[..d292.find("fn vignette").unwrap()];
            assert!(!d292.contains("1.2998") && !d292.contains("FINAL_SCALE"));
            assert_eq!(source.contains("return srgb_encode(c);"), srgb);
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
            let _ = info;
            for entry in [
                "glow_source",
                "glow_blur_h",
                "glow_blur_v",
                "glow_composite",
                "final_composite",
                "final_only",
            ] {
                assert!(
                    module.entry_points.iter().any(|e| e.name == entry),
                    "{entry}"
                );
            }
        }
    }
    /// D292: only plain and sRGB 8-bit main textures run the chain.
    #[test]
    fn d292_target_formats() {
        assert_eq!(target_space(TextureFormat::Rgba8UnormSrgb), Some(true));
        assert_eq!(target_space(TextureFormat::Bgra8Unorm), Some(false));
        assert_eq!(target_space(TextureFormat::Rgba16Float), None);
    }
    /// D292: the camera's marker reaches the render world only while active (D294: or
    /// while the final pass is on).
    #[test]
    fn d292_extracted_only_while_active() {
        let m = |active, final_pass| EdgeGlow {
            active,
            final_pass,
            alpha_cleared: false,
        };
        assert!(EdgeGlow::extract_component(&m(false, false)).is_none());
        assert!(EdgeGlow::extract_component(&m(true, false)).is_some());
        assert!(EdgeGlow::extract_component(&m(false, true)).is_some());
    }
    /// D294: the FOUND constants of the final pass (PS 14857) and the add pass's entries.
    #[test]
    fn d294_final_pass_constants() {
        assert_eq!(FINAL_SCALE, 1.2998);
        assert_eq!(VIGNETTE_AXES, [1.0996, 1.7002]);
        assert_eq!(VIGNETTE_START, 1.2);
        assert_eq!(VIGNETTE_FLOOR, 0.2);
        assert_eq!(composite_entry(true, false), "glow_composite");
        assert_eq!(composite_entry(true, true), "final_composite");
        assert_eq!(composite_entry(false, true), "final_only");
    }
    /// D294: the vignette is 1 over the centre (r <= 0.2), about 0.27 in a 16:9 frame's
    /// corner, and the 0.2 floor from r = 1.2 out (never reached inside the frame: the
    /// corner's r is about 1.012); the notes' 0.83 / 0.47 / 0.44 in the game's viewport.
    #[test]
    fn d294_vignette_centre_corner_and_floor() {
        let (w, h) = (1280, 720);
        assert_eq!(vignette_cpu(639, 359, w, h), 1.);
        assert_eq!(vignette_at(0.), 1.);
        assert_eq!(vignette_at(0.2), 1.);
        // r = 0.2 is still the full 1: 0.2 / 1.7002 of the height below the centre.
        assert_eq!(vignette_cpu(639, 359 + 80, w, h), 1.);
        let r = (0.5f32 * 1.0996).hypot(0.5 * 1.7002);
        assert!((r - 1.0124).abs() < 1e-3, "{r}");
        let corner = vignette_cpu(0, 0, w, h);
        assert!((0.27..0.28).contains(&corner), "{corner}");
        assert_eq!(corner, vignette_cpu(w - 1, h - 1, w, h));
        assert_eq!(corner, vignette_cpu(w - 1, 0, w, h));
        // The floor: 0.2 at r 1.2 and beyond, never below it.
        assert!((vignette_at(1.2) - 0.2).abs() < 1e-6);
        assert!((vignette_at(3.) - 0.2).abs() < 1e-6);
        assert!((vignette_at(1.1) - (0.2 + 0.8 * 0.028)).abs() < 1e-6);
        for k in 0..100 {
            let v = vignette_at(k as f32 * 0.02);
            assert!((VIGNETTE_FLOOR..=1.).contains(&v), "{k} {v}");
        }
        // The notes' values (vig.py, game viewport 1120 x 630 at (80, 45)): the post tops
        // (390, 520), the bottom centre (640, 660), (100, 600).
        let at = |x: u32, y: u32| vignette_cpu(x - 80, y - 45, 1120, 630);
        assert!((at(390, 520) - 0.83).abs() < 0.01);
        assert!((at(640, 660) - 0.47).abs() < 0.01);
        assert!((at(100, 600) - 0.44).abs() < 0.01);
    }
    /// D294: the final pass scales the encoded scene by 1.2998, adds the glow, then the
    /// vignette, saturated at the end: COL0 at the centre is 1.3 x COL0 bytes; with the glow
    /// 0 only the scale and vignette; white at the floor is 0.26 (not saturated first).
    #[test]
    fn d294_final_pass_scale_and_glow() {
        let c = final_pass_cpu([0.5; 3], [0.; 3], 1.);
        assert!((c[0] - 0.6499).abs() < 1e-6, "{c:?}");
        let c = final_pass_cpu([0.5, 0.2, 0.], [0.1, 0.2, 0.3], 0.5);
        assert!(
            (c[0] - 0.37495).abs() < 1e-6 && (c[2] - 0.15).abs() < 1e-6,
            "{c:?}"
        );
        let col0 = [88. / 255., 192. / 255., 14. / 255.];
        let b = final_pass_cpu(col0, [0.; 3], 1.).map(|v| (v * 255.).round());
        assert_eq!(b, [114., 250., 18.]);
        let white = final_pass_cpu([1.; 3], [0.; 3], VIGNETTE_FLOOR);
        assert!((white[0] - 0.25996).abs() < 1e-6, "{white:?}");
        assert_eq!(final_pass_cpu([0.9; 3], [0.5; 3], 1.), [1.; 3]);
    }
    /// D294: the final entries carry the FOUND constants, smoothstep(0, 1, saturate(..)), the
    /// scale before the glow's add, saturate at the end and write alpha 1, and validate with
    /// naga for both target kinds.
    #[test]
    fn d294_final_pass_shader_validates() {
        for srgb in [true, false] {
            let source = preprocess(srgb);
            assert!(source.contains("const FINAL_SCALE: f32 = 1.2998;"));
            assert!(source.contains("const VIGNETTE_AXES = vec2<f32>(1.0996, 1.7002);"));
            assert!(source.contains("const VIGNETTE_START: f32 = 1.2;"));
            assert!(source.contains("const VIGNETTE_FLOOR: f32 = 0.2;"));
            assert!(source.contains("smoothstep(0.0, 1.0, saturate(VIGNETTE_START - r))"));
            assert!(source.contains("VIGNETTE_FLOOR + (1.0 - VIGNETTE_FLOOR) * s"));
            assert!(source.contains(
                "(to_encoded(scene.rgb) * FINAL_SCALE + glow) * vignette(in.position.xy)"
            ));
            assert!(
                source.contains("to_encoded(scene.rgb) * FINAL_SCALE * vignette(in.position.xy)")
            );
            // D300: both end in the mix back by the scene's alpha, saturated after it.
            assert_eq!(source.matches("return final_mix(scene, c);").count(), 2);
            assert!(source.contains("vec4<f32>(from_encoded(min(e, vec3<f32>(1.0))), 1.0)"));
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
            for entry in ["final_composite", "final_only"] {
                assert!(
                    module.entry_points.iter().any(|e| e.name == entry),
                    "{entry}"
                );
            }
        }
    }
    /// D300: the game's last step, `lerp(c, scene, a)` before the 8-bit saturate: alpha 1
    /// gives the scene as it is (a level's opaque surfaces), alpha 0 the D294 pass (the
    /// Pod), between them the straight mix; the saturate comes after the mix.
    #[test]
    fn d300_final_pass_mixes_back_by_alpha() {
        let scene = [0.5, 0.2, 0.8];
        let glow = [0.1, 0.0, 0.3];
        assert_eq!(final_pass_mix_cpu(scene, 1., glow, 0.4), scene);
        assert_eq!(
            final_pass_mix_cpu(scene, 0., glow, 0.4),
            final_pass_cpu(scene, glow, 0.4)
        );
        let half = final_pass_mix_cpu(scene, 0.5, [0.; 3], 1.);
        assert!((half[0] - (0.5 + 0.6499) / 2.).abs() < 1e-6, "{half:?}");
        // c = 0.8 x 1.2998 = 1.04 is mixed before it is saturated: (1.04 + 0.8) / 2.
        assert!(
            (half[2] - (0.8 * 1.2998 + 0.8) / 2.).abs() < 1e-6,
            "{half:?}"
        );
        assert_eq!(final_pass_mix_cpu(scene, 0., [0.; 3], 1.)[2], 1.);
        let mixed = final_pass_mix_cpu(scene, 0.5, glow, 1.);
        assert_eq!(mixed[2], 1.);
        assert_eq!(final_pass_mix_cpu([0.9; 3], 0.25, [0.5; 3], 1.), [1.; 3]);
        // Out-of-range alphas clamp, as the shader's.
        assert_eq!(final_pass_mix_cpu(scene, 2., glow, 0.4), scene);
        assert_eq!(
            final_pass_mix_cpu(scene, -1., glow, 0.4),
            final_pass_cpu(scene, glow, 0.4)
        );
    }
    /// D300: the frame a level gives (every pixel alpha 1, as the viewer's opaque surfaces,
    /// its clear and its blended draws leave it) comes back unchanged from the pass, edge to
    /// edge; the same frame in the Pod (alpha 0) gets x1.2998 at the centre and the
    /// vignette's 0.27 in the corner.
    #[test]
    fn d300_level_frame_unchanged_pod_frame_scaled() {
        let (w, h) = (1280, 720);
        let grey = [0.4; 3];
        for (x, y) in [(0, 0), (639, 359), (w - 1, h - 1), (100, 600)] {
            let v = vignette_cpu(x, y, w, h);
            assert_eq!(final_pass_mix_cpu(grey, 1., [0.; 3], v), grey);
        }
        let centre = final_pass_mix_cpu(grey, 0., [0.; 3], vignette_cpu(639, 359, w, h));
        assert!((centre[0] - 0.4 * 1.2998).abs() < 1e-6, "{centre:?}");
        let corner = final_pass_mix_cpu(grey, 0., [0.; 3], vignette_cpu(0, 0, w, h));
        assert!(
            (corner[0] / (0.4 * 1.2998) - 0.27).abs() < 0.01,
            "{corner:?}"
        );
    }
    /// D300: the route: on by default on the level-lighting route (and under
    /// `--final-pass`), off under `--no-final-pass` and on every other route; alpha 0 only
    /// for the Pod picture and only while the pass is on.
    #[test]
    fn d300_final_pass_default_route() {
        assert_eq!(final_pass_route(true, None, false), (true, false));
        assert_eq!(final_pass_route(true, Some(true), false), (true, false));
        assert_eq!(final_pass_route(true, None, true), (true, true));
        assert_eq!(final_pass_route(true, Some(false), true), (false, false));
        assert_eq!(final_pass_route(true, Some(false), false), (false, false));
        assert_eq!(final_pass_route(false, None, true), (false, false));
        assert_eq!(final_pass_route(false, Some(true), true), (false, false));
    }
    /// D300: the shader's mix: the scene's alpha (clamped), returned as it is at 1, and the
    /// Pod's define puts 0 in its place; the glow's source still reads the scene's own alpha
    /// (`1 - a`, D292) under either define, and every entry validates with naga in all four
    /// combinations.
    #[test]
    fn d300_final_mix_shader_validates() {
        for srgb in [true, false] {
            for cleared in [false, true] {
                let source = preprocess_with(srgb, cleared);
                assert!(source.contains("fn final_mix(scene: vec4<f32>, c: vec3<f32>)"));
                assert!(source.contains("let e = mix(c, to_encoded(scene.rgb), a);"));
                assert!(source.contains("if a >= 1.0 {"));
                assert!(source.contains("return vec4<f32>(scene.rgb, 1.0);"));
                assert_eq!(source.contains("let a = 0.0;"), cleared);
                assert_eq!(
                    source.contains("let a = clamp(scene.a, 0.0, 1.0);"),
                    !cleared
                );
                let src = &source[source.find("fn glow_source").unwrap()..];
                let src = &src[..src.find("fn blur").unwrap()];
                assert!(src.contains("a += t.a;") && src.contains("(1.0 - a)"));
                // D292's own add (no final pass) does not mix: alpha 1 as before.
                let d292 = &source[source.find("fn glow_composite").unwrap()..];
                let d292 = &d292[..d292.find("fn vignette").unwrap()];
                assert!(!d292.contains("final_mix"));
                let module = naga::front::wgsl::parse_str(&source)
                    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap();
                for entry in ["glow_source", "final_composite", "final_only"] {
                    assert!(module.entry_points.iter().any(|e| e.name == entry));
                }
            }
        }
    }
    /// D322: under `MENU_LAYER` the final entries compose the menu layer over the encoded
    /// scene (premultiplied over), add its alpha to the scene's (0 in the Pod) and mix back
    /// by that; without it the entries are D300's text. naga validates every combination.
    #[test]
    fn d322_menu_layer_shader_validates() {
        for srgb in [true, false] {
            for cleared in [false, true] {
                let plain = preprocess_all(srgb, cleared, false);
                assert!(!plain.contains("menu_texture") && !plain.contains("with_menu"));
                assert_eq!(plain, preprocess_with(srgb, cleared));
                let source = preprocess_all(srgb, cleared, true);
                assert!(source.contains("@group(0) @binding(3) var menu_texture"));
                assert!(source.contains("@group(0) @binding(4) var menu_alpha"));
                assert!(source.contains("m.rgb + (1.0 - m.a) * to_encoded(scene.rgb)"));
                assert_eq!(source.contains("let a = clamp(add, 0.0, 1.0);"), cleared);
                assert_eq!(
                    source.contains("let a = clamp(scene.a + add, 0.0, 1.0);"),
                    !cleared
                );
                assert!(source.contains("let c = (e.rgb * FINAL_SCALE + glow) * vignette"));
                assert!(source.contains("let mixed = mix(c, e.rgb, e.a);"));
                // D292's own add does not read the layer.
                let d292 = &source[source.find("fn glow_composite").unwrap()..];
                let d292 = &d292[..d292.find("fn vignette").unwrap()];
                assert!(!d292.contains("menu"));
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
    }
    /// D300: the marker carries the Pod's alpha choice into the render world.
    #[test]
    fn d300_extracted_with_the_alpha_choice() {
        let m = EdgeGlow {
            active: false,
            final_pass: true,
            alpha_cleared: true,
        };
        let out = EdgeGlow::extract_component(&m).unwrap();
        assert!(out.final_pass && out.alpha_cleared && !out.active);
    }
    /// D303: the glow runs with the final pass (every frame on the level-lighting route) or
    /// a dissolve, and never under `--no-edge-glow`.
    #[test]
    fn d303_glow_runs_with_the_final_pass() {
        assert!(glow_runs(false, true, false));
        assert!(glow_runs(false, false, true));
        assert!(glow_runs(false, true, true));
        assert!(!glow_runs(false, false, false));
        assert!(!glow_runs(true, true, true));
        let m = EdgeGlow {
            active: glow_runs(false, true, false),
            final_pass: true,
            alpha_cleared: false,
        };
        assert_eq!(composite_entry(m.active, m.final_pass), "final_composite");
    }
    /// D303: a glowing surface (alpha 1/13, the game's stars) in an opaque level frame: the
    /// glow chain makes a halo from it, the final pass shows it only where the alpha is below
    /// 1 (the surface itself, mixed by its alpha) and returns the scene elsewhere; in the Pod
    /// (alpha 0 at the pass) the halo shows around it.
    #[test]
    fn d303_glowing_surface_through_the_chain() {
        let (w, h) = (64, 64);
        let mut rgb = vec![[0.3f32; 3]; w * h];
        let mut alpha = vec![1f32; w * h];
        let star = crate::dual::glow_alpha([1.; 3]);
        for y in 30..34 {
            for x in 30..34 {
                rgb[y * w + x] = [0.6; 3];
                alpha[y * w + x] = star;
            }
        }
        let glow = glow_cpu(&rgb, &alpha, w, h);
        let at = |x: usize, y: usize| y * w + x;
        assert!(glow[at(32, 32)][0] > 0.04, "{:?}", glow[at(32, 32)]);
        let side = glow[at(36, 32)][0];
        assert!(side > 0.01 && side < glow[at(32, 32)][0], "{side}");
        assert!(glow[at(2, 2)][0] < 1e-6);
        let v = vignette_cpu(32, 32, w as u32, h as u32);
        // Level frame: the neighbour (alpha 1) is the scene, the surface brighter.
        let side = final_pass_mix_cpu(rgb[at(36, 32)], 1., glow[at(36, 32)], v);
        assert_eq!(side, rgb[at(36, 32)]);
        let on = final_pass_mix_cpu(rgb[at(32, 32)], star, glow[at(32, 32)], v);
        assert!(on[0] > 0.6 * FINAL_SCALE, "{on:?}");
        // Pod frame: the halo shows on the neighbour.
        let pod = final_pass_mix_cpu(rgb[at(36, 32)], 0., glow[at(36, 32)], v);
        assert!(
            pod[0] > rgb[at(36, 32)][0] * FINAL_SCALE * v + 0.02,
            "{pod:?}"
        );
    }
    /// D307: the fog's lift of the glow alpha weakens the glow in step: a fully fogged
    /// glowing surface of the common program writes alpha 1 and feeds the source nothing;
    /// the listed programs (the checkpoint ring's) keep their halo under the same fog.
    #[test]
    fn d307_fogged_glow_alpha_through_the_chain() {
        let (w, h) = (64, 64);
        let ring = [1.; 3];
        let halo = |a: f32| {
            let rgb = vec![[0.6f32; 3]; w * h];
            let mut alpha = vec![1f32; w * h];
            for y in 30..34 {
                for x in 30..34 {
                    alpha[y * w + x] = a;
                }
            }
            glow_cpu(&rgb, &alpha, w, h)[36 * w + 32][0]
        };
        let clear = halo(crate::dual::glow_alpha_under_fog(ring, 0., true));
        let half = halo(crate::dual::glow_alpha_under_fog(ring, 0.5, true));
        let full = halo(crate::dual::glow_alpha_under_fog(ring, 1., true));
        let listed = halo(crate::dual::glow_alpha_under_fog(ring, 1., false));
        assert!(clear > half && half > full, "{clear} {half} {full}");
        assert!(full.abs() < 1e-6);
        assert_eq!(listed, clear);
    }
}
