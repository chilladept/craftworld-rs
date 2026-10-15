//! D302 (docs/level-scripts.md part 53): the game's anti-aliasing job in the Pod picture, the
//! EBOOT's "EDGE MLAA" SPU task (part 51). In the Pod's frames the post chain copies the scene
//! to main memory after the depth of field, the glow's source pass reads the scene as it was,
//! the SPU job filters the copy and the copy comes back over the scene before the final pass
//! (FOUND, frame 5247: copy out 3194/3195, glow source 3256, copy back 3363/3364, final pass
//! 3454). Here, in two full-screen passes after the depth of field (`dof_post.rs`):
//!
//! 1. Edges: between each pixel and the ones right of and below it, an edge where any colour
//!    channel differs by at least the larger of the two pixels' thresholds, a pixel's
//!    threshold being `max(MLAA_ABS_THRESHOLD, floor(max channel x MLAA_REL_THRESHOLD / 256))`
//!    in 8-bit encoded values (FOUND in the job's SPU program: the per-pixel threshold from the
//!    largest channel times a 16-bit factor, shifted down by 8 and floored by a second value,
//!    kept in the pixel's alpha byte; the edge test takes the larger alpha of the pair and
//!    compares every colour byte's absolute difference with it).
//! 2. Blend: Reshetov's morphological anti-aliasing (2009). A run of edges along a row (or a
//!    column) ends in a step up, a step down or none at each end; from the middle of each step
//!    a line runs to the middle of the run, and each pixel beside the run takes the colour
//!    across it by the area that line cuts from it (Z, U and L shapes alike). MEASURED in 5247
//!    (static parts of the copy back): the weights along Z and U runs are these areas (fit
//!    scale 0.97, a 16-pixel run 0.41, 0.35, 0.29, 0.22, 0.16, 0.09, 0.03 from its end), and a
//!    pixel with edges on two sides takes only the larger of its weights (45-degree stairs
//!    move by 1/8 of the contrast, not 1/4). Runs longer than [`MLAA_MAX_EDGE_LENGTH`] are left
//!    alone (INFERRED, best fit 36 to 40; a 36-pixel run is blended, 42 and 101 are not).
//!
//! The job's own code is not followed beyond its edge test (the blend is INFERRED as the
//! published algorithm with the measured weights). The game's copy back has alpha 0 (part
//! 51); here the scene's alpha is kept and the final pass's `SCENE_ALPHA_CLEARED` applies the
//! 0 in the Pod. By default (D304; `--no-mlaa` turns it off), in the Pod picture (`--pod-camera`) on the level-lighting
//! route only, with the camera's MSAA off (the game's scene target has one sample, MEASURED
//! in the capture). When the glow chain's composite runs (`glow_post.rs`), the result goes to
//! a texture of its own that the composite reads as the scene, so the glow's source still reads
//! the scene before the job, as the game's does; otherwise it replaces the main texture.
use bevy::{
    core_pipeline::{tonemapping::tonemapping, Core3d, Core3dSystems, FullscreenShader},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            binding_types::texture_2d, BindGroupEntries, BindGroupLayoutDescriptor,
            BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            Extent3d, FragmentState, Operations, PipelineCache, RenderPassColorAttachment,
            RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages, TextureDescriptor,
            TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        sync_component::SyncComponent,
        texture::{CachedTexture, TextureCache},
        view::ViewTarget,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::Shader,
};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

/// D302 FOUND (EBOOT 01.33, the kick at 0x31e330 / 0x31e334, stored by 0xc0bca8 at +0x24 of
/// each SPU task's record): the job's absolute threshold, 10 in 8-bit encoded values. Which of
/// the kick's two values is the absolute one is INFERRED (the other order, 89, would leave the
/// blended edges of contrast 50 in 5247 undetected).
pub const MLAA_ABS_THRESHOLD: u32 = 10;
/// D302 FOUND (the same kick, 0x59 at +0x26): the relative threshold's factor, applied as
/// `max channel x 89 >> 8` (0.348 of the brighter pixel's largest channel; the SPU program
/// multiplies by a 16-bit value and keeps the high byte). MEASURED: 0.348 fits the copy back
/// better than 0.25 or 0.45 (residual 0.618 of the stale copy's against 0.679 and 0.684).
pub const MLAA_REL_THRESHOLD: u32 = 0x59;
/// D302 INFERRED (frame 5247): runs of more edges than this are not blended. A 36-pixel Z run
/// matches the areas (fit scale 1.04 / 0.95), 42- and 101-pixel runs are untouched; over the
/// static part of the stale copy the residual is lowest at 36 to 40 (0.618; 0.621 at 32, 0.637
/// unlimited).
pub const MLAA_MAX_EDGE_LENGTH: u32 = 40;

/// D302 `--mlaa` (the Pod picture on the level-lighting route only, `main.rs`).
static ENABLED: AtomicBool = AtomicBool::new(false);
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
/// D302: whether the job runs: on the level-lighting route, in the Pod picture (where the game
/// runs it, part 51); its camera then renders without MSAA. D304: on by default there; `flag`
/// is `Some(true)` under `--mlaa` (no change there) and `Some(false)` under `--no-mlaa`.
pub fn mlaa_route(level_route: bool, pod: bool, flag: Option<bool>) -> bool {
    level_route && pod && flag != Some(false)
}

/// D302: on the main 3D camera; extracted only while the job is on.
#[derive(Component, Clone, Copy, Default, Debug, PartialEq)]
pub struct Mlaa;
impl SyncComponent for Mlaa {
    type Target = Self;
}
impl ExtractComponent for Mlaa {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(item: &Self) -> Option<Self> {
        enabled().then_some(*item)
    }
}

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("8e3b6a2d-41f7-4c5e-9a0b-d30200000001");
/// The edge flags: red, the edge below the pixel; green, the edge right of it.
const EDGE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

pub struct MlaaPostPlugin;
impl Plugin for MlaaPostPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                SHADER.id(),
                Shader::from_wgsl(shader_source(), "d302-mlaa.wgsl"),
            )
            .unwrap();
        app.add_plugins(ExtractComponentPlugin::<Mlaa>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(Render, prepare_mlaa.in_set(RenderSystems::PrepareResources))
            .add_systems(
                Core3d,
                mlaa.in_set(Core3dSystems::PostProcess)
                    .after(tonemapping)
                    .before(crate::glow_post::edge_glow),
            );
    }
}

#[derive(Resource)]
pub(crate) struct MlaaPipelines {
    layout: BindGroupLayoutDescriptor,
    ids: HashMap<(TextureFormat, &'static str), CachedRenderPipelineId>,
    vertex: bevy::render::render_resource::VertexState,
}

fn init_pipelines(mut commands: Commands, fullscreen: Res<FullscreenShader>) {
    let layout = BindGroupLayoutDescriptor::new(
        "d302_mlaa",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        ),
    );
    commands.insert_resource(MlaaPipelines {
        layout,
        ids: HashMap::new(),
        vertex: fullscreen.to_vertex_state(),
    });
}

/// D302, render world: the edge flags, the blended picture when the glow chain's composite
/// reads it (`into_glow`), and the two pipelines.
#[derive(Component)]
pub(crate) struct MlaaView {
    edges: CachedTexture,
    out: CachedTexture,
    pub(crate) into_glow: bool,
    pipelines: [CachedRenderPipelineId; 2],
}
impl MlaaView {
    /// The blended scene for the glow chain's composite (`into_glow` only).
    pub(crate) fn scene(&self) -> &TextureView {
        &self.out.default_view
    }
}

fn prepare_mlaa(
    mut commands: Commands,
    mut pipelines: ResMut<MlaaPipelines>,
    cache: Res<PipelineCache>,
    mut textures: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    views: Query<(Entity, &ViewTarget, Has<crate::glow_post::EdgeGlow>), With<Mlaa>>,
    stale: Query<Entity, (With<MlaaView>, Without<Mlaa>)>,
) {
    for entity in &stale {
        commands.entity(entity).remove::<MlaaView>();
    }
    for (entity, target, into_glow) in &views {
        let format = target.main_texture_format();
        let Some(srgb) = crate::glow_post::target_space(format) else {
            warn_once!("D302: MLAA skipped: main texture format {format:?}");
            commands.entity(entity).remove::<MlaaView>();
            continue;
        };
        let p = &mut *pipelines;
        let pipelines = ["mlaa_edges", "mlaa_blend"].map(|entry| {
            let out = if entry == "mlaa_edges" {
                EDGE_FORMAT
            } else {
                format
            };
            *p.ids.entry((out, entry)).or_insert_with(|| {
                let shader_defs = if srgb {
                    vec!["SRGB_TARGET".into()]
                } else {
                    Vec::new()
                };
                cache.queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some(format!("d302_{entry}").into()),
                    layout: vec![p.layout.clone()],
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
        let mut texture = |label: &'static str, format: TextureFormat| {
            textures.get(
                &device,
                TextureDescriptor {
                    label: Some(label),
                    size: Extent3d {
                        width: size.width.max(1),
                        height: size.height.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        let view = MlaaView {
            edges: texture("d302_mlaa_edges", EDGE_FORMAT),
            out: texture("d302_mlaa_out", format),
            into_glow,
            pipelines,
        };
        commands.entity(entity).insert(view);
    }
}

/// D302: edges (scene -> edge flags), blend (scene and edge flags -> the blended picture),
/// into the glow chain's own texture when its composite runs, else into the main texture.
pub(crate) fn mlaa(
    view: ViewQuery<(&ViewTarget, &MlaaView)>,
    pipelines: Res<MlaaPipelines>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, mlaa) = view.into_inner();
    let (Some(edges), Some(blend)) = (
        cache.get_render_pipeline(mlaa.pipelines[0]),
        cache.get_render_pipeline(mlaa.pipelines[1]),
    ) else {
        return;
    };
    let device = ctx.render_device().clone();
    let layout = cache.get_bind_group_layout(&pipelines.layout);
    let (scene, out) = if mlaa.into_glow {
        (target.main_texture_view(), &mlaa.out.default_view)
    } else {
        let post = target.post_process_write();
        (post.source, post.destination)
    };
    let edge_view = &mlaa.edges.default_view;
    // The edge pass binds the scene twice (its second slot is unread).
    let steps = [(edges, scene, edge_view), (blend, edge_view, out)];
    let encoder = ctx.command_encoder();
    for (pipeline, second, target) in steps {
        let bind_group = device.create_bind_group(
            "d302_mlaa",
            &layout,
            &BindGroupEntries::sequential((scene, second)),
        );
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("d302_mlaa"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
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

/// The WGSL of the two passes, with the FOUND / INFERRED constants written in.
fn shader_source() -> String {
    format!(
        "const MLAA_ABS_THRESHOLD: f32 = {:?};\nconst MLAA_REL_THRESHOLD: f32 = {:?};\nconst MLAA_MAX_EDGE_LENGTH: i32 = {};\n{BODY}",
        MLAA_ABS_THRESHOLD as f32, MLAA_REL_THRESHOLD as f32, MLAA_MAX_EDGE_LENGTH
    )
}

const BODY: &str = r#"
// D302: the game's EDGE MLAA job (part 53), in its 8-bit encoded output space.
@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var edge_texture: texture_2d<f32>;
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
// The pixel as the game's bytes (0..255).
fn scene_bytes(p: vec2<i32>) -> vec3<f32> {
    return round(to_encoded(textureLoad(scene_texture, p, 0).rgb) * 255.0);
}
// The job's per-pixel threshold: max(absolute, floor(max channel x relative / 256)).
fn threshold(c: vec3<f32>) -> f32 {
    return max(MLAA_ABS_THRESHOLD, floor(max(c.r, max(c.g, c.b)) * MLAA_REL_THRESHOLD / 256.0));
}
// An edge where any channel differs by at least the larger threshold of the two.
fn is_edge(a: vec3<f32>, b: vec3<f32>) -> bool {
    let d = abs(a - b);
    return max(d.r, max(d.g, d.b)) >= max(threshold(a), threshold(b));
}
// Pass 1: red, the edge between the pixel and the one below; green, the one to its right.
@fragment
fn mlaa_edges(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(scene_texture));
    let p = vec2<i32>(floor(in.position.xy));
    let c = scene_bytes(p);
    var below = 0.0;
    var right = 0.0;
    if (p.y + 1 < size.y && is_edge(c, scene_bytes(p + vec2<i32>(0, 1)))) {
        below = 1.0;
    }
    if (p.x + 1 < size.x && is_edge(c, scene_bytes(p + vec2<i32>(1, 0)))) {
        right = 1.0;
    }
    return vec4<f32>(below, right, 0.0, 0.0);
}
// Edge flag k (0 below, 1 right) at p; false outside the picture.
fn flag(p: vec2<i32>, k: i32) -> bool {
    let size = vec2<i32>(textureDimensions(edge_texture));
    if (p.x < 0 || p.y < 0 || p.x >= size.x || p.y >= size.y) {
        return false;
    }
    return textureLoad(edge_texture, p, 0)[k] > 0.5;
}
// A run's end: +0.5 when the step is on the run's first side (the row above a horizontal run,
// the column left of a vertical one), -0.5 on the second side, 0 for none or both.
fn step_height(first: bool, second: bool) -> f32 {
    if (first && !second) {
        return 0.5;
    }
    if (second && !first) {
        return -0.5;
    }
    return 0.0;
}
// The signed area over [i, i + 1] under the line from (0, hl) to (L/2, 0) and the one from
// (L/2, 0) to (L, hr); x: the first side's share (> 0), y: the second side's (< 0, negated).
fn areas(i: f32, len: f32, hl: f32, hr: f32) -> vec2<f32> {
    let m = len * 0.5;
    var out = vec2<f32>(0.0);
    var parts = array<f32, 2>(0.0, 0.0);
    let lo_l = i;
    let hi_l = min(i + 1.0, m);
    if (hl != 0.0 && hi_l > lo_l) {
        parts[0] = hl * ((1.0 - lo_l / m) + (1.0 - hi_l / m)) * 0.5 * (hi_l - lo_l);
    }
    let lo_r = max(i, m);
    let hi_r = min(i + 1.0, len);
    if (hr != 0.0 && hi_r > lo_r) {
        parts[1] = hr * ((lo_r - m) / m + (hi_r - m) / m) * 0.5 * (hi_r - lo_r);
    }
    for (var k = 0; k < 2; k++) {
        if (parts[k] > 0.0) {
            out.x += parts[k];
        } else {
            out.y -= parts[k];
        }
    }
    return out;
}
// The run of edges flag `k` through p, along `along` (the edge between p and p + across):
// the weights of p (x) and of p + across (y), 0 off an edge or past the longest run.
fn run(p: vec2<i32>, k: i32, along: vec2<i32>, across: vec2<i32>) -> vec2<f32> {
    if (!flag(p, k)) {
        return vec2<f32>(0.0);
    }
    var dl = 0;
    while (dl < MLAA_MAX_EDGE_LENGTH && flag(p - along * (dl + 1), k)) {
        dl += 1;
    }
    var dr = 0;
    while (dr < MLAA_MAX_EDGE_LENGTH && flag(p + along * (dr + 1), k)) {
        dr += 1;
    }
    let len = dl + dr + 1;
    if (len > MLAA_MAX_EDGE_LENGTH) {
        return vec2<f32>(0.0);
    }
    // The steps are the other kind of edge, just outside each end, on either side.
    let other = 1 - k;
    let before = p - along * (dl + 1);
    let after = p + along * dr;
    let hl = step_height(flag(before, other), flag(before + across, other));
    let hr = step_height(flag(after, other), flag(after + across, other));
    return areas(f32(dl), f32(len), hl, hr);
}
// Pass 2: each pixel takes the colour across its strongest edge by that edge's weight.
@fragment
fn mlaa_blend(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.position.xy));
    let scene = textureLoad(scene_texture, p, 0);
    let x = vec2<i32>(1, 0);
    let y = vec2<i32>(0, 1);
    // Below, above, right, left: the order ties are broken in.
    var w = array<f32, 4>(
        run(p, 0, x, y).x,
        run(p - y, 0, x, y).y,
        run(p, 1, y, x).x,
        run(p - x, 1, y, x).y,
    );
    var n = array<vec2<i32>, 4>(y, -y, x, -x);
    var best = 0;
    for (var k = 1; k < 4; k++) {
        if (w[k] > w[best]) {
            best = k;
        }
    }
    if (w[best] <= 0.0) {
        return scene;
    }
    let c = scene_bytes(p);
    let o = scene_bytes(p + n[best]);
    let e = mix(c, o, w[best]) / 255.0;
    return vec4<f32>(from_encoded(e), scene.a);
}
"#;

/// D302: a CPU mirror of both passes on 8-bit encoded colours (row-major, `w` x `h`).
/// Returns the blended colours (0..255, unrounded).
#[cfg(test)]
pub fn mlaa_cpu(rgb: &[[u8; 3]], w: usize, h: usize) -> Vec<[f32; 3]> {
    let (_, weights) = mlaa_weights(rgb, w, h);
    (0..w * h)
        .map(|p| {
            let c = rgb[p].map(f32::from);
            let ws = weights[p];
            let mut best = 0;
            for k in 1..4 {
                if ws[k] > ws[best] {
                    best = k;
                }
            }
            if ws[best] <= 0. {
                return c;
            }
            let q = match best {
                0 => p + w,
                1 => p - w,
                2 => p + 1,
                _ => p - 1,
            };
            let o = rgb[q].map(f32::from);
            std::array::from_fn(|k| c[k] + (o[k] - c[k]) * ws[best])
        })
        .collect()
}

/// D302: the job's per-pixel threshold in 8-bit values.
#[cfg(test)]
pub fn threshold_cpu(c: [u8; 3]) -> u32 {
    let max = u32::from(c[0].max(c[1]).max(c[2]));
    MLAA_ABS_THRESHOLD.max((max * MLAA_REL_THRESHOLD) >> 8)
}
/// D302: the edge test between two pixels.
#[cfg(test)]
pub fn is_edge_cpu(a: [u8; 3], b: [u8; 3]) -> bool {
    let d = (0..3).map(|k| a[k].abs_diff(b[k])).max().unwrap();
    u32::from(d) >= threshold_cpu(a).max(threshold_cpu(b))
}
/// D302: the signed areas of the shader's `areas` (first side, second side).
#[cfg(test)]
pub fn areas_cpu(i: f32, len: f32, hl: f32, hr: f32) -> [f32; 2] {
    let m = len * 0.5;
    let mut parts = [0f32; 2];
    let (lo, hi) = (i, (i + 1.).min(m));
    if hl != 0. && hi > lo {
        parts[0] = hl * ((1. - lo / m) + (1. - hi / m)) * 0.5 * (hi - lo);
    }
    let (lo, hi) = (i.max(m), (i + 1.).min(len));
    if hr != 0. && hi > lo {
        parts[1] = hr * ((lo - m) / m + (hi - m) / m) * 0.5 * (hi - lo);
    }
    let mut out = [0.; 2];
    for a in parts {
        if a > 0. {
            out[0] += a;
        } else {
            out[1] -= a;
        }
    }
    out
}

/// D302: the edge flags (`[below, right]` per pixel) and each pixel's four weights (from
/// the pixel below, above, right, left), as the shader's passes compute them.
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub fn mlaa_weights(rgb: &[[u8; 3]], w: usize, h: usize) -> (Vec<[bool; 2]>, Vec<[f32; 4]>) {
    let mut edges = vec![[false; 2]; w * h];
    for y in 0..h {
        for x in 0..w {
            let p = y * w + x;
            edges[p][0] = y + 1 < h && is_edge_cpu(rgb[p], rgb[p + w]);
            edges[p][1] = x + 1 < w && is_edge_cpu(rgb[p], rgb[p + 1]);
        }
    }
    let flag = |x: i64, y: i64, k: usize| -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < w
            && (y as usize) < h
            && edges[y as usize * w + x as usize][k]
    };
    let step = |a: bool, b: bool| match (a, b) {
        (true, false) => 0.5,
        (false, true) => -0.5,
        _ => 0.,
    };
    let max = i64::from(MLAA_MAX_EDGE_LENGTH);
    let run = |x: i64, y: i64, k: usize, along: (i64, i64), across: (i64, i64)| -> [f32; 2] {
        if !flag(x, y, k) {
            return [0.; 2];
        }
        let at = |d: i64| (x + along.0 * d, y + along.1 * d);
        let mut dl = 0;
        while dl < max && {
            let (a, b) = at(-(dl + 1));
            flag(a, b, k)
        } {
            dl += 1;
        }
        let mut dr = 0;
        while dr < max && {
            let (a, b) = at(dr + 1);
            flag(a, b, k)
        } {
            dr += 1;
        }
        let len = dl + dr + 1;
        if len > max {
            return [0.; 2];
        }
        let other = 1 - k;
        let (bx, by) = at(-(dl + 1));
        let (ax, ay) = at(dr);
        let hl = step(
            flag(bx, by, other),
            flag(bx + across.0, by + across.1, other),
        );
        let hr = step(
            flag(ax, ay, other),
            flag(ax + across.0, ay + across.1, other),
        );
        areas_cpu(dl as f32, len as f32, hl, hr)
    };
    let weights = (0..w * h)
        .map(|p| {
            let (x, y) = ((p % w) as i64, (p / w) as i64);
            [
                run(x, y, 0, (1, 0), (0, 1))[0],
                run(x, y - 1, 0, (1, 0), (0, 1))[1],
                run(x, y, 1, (0, 1), (1, 0))[0],
                run(x - 1, y, 1, (0, 1), (1, 0))[1],
            ]
        })
        .collect();
    (edges, weights)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D302: the FOUND kick values and what the threshold makes of them: 10 below a
    /// largest channel of 29, 0.348 of it above (88 for white).
    #[test]
    fn d302_constants_and_threshold() {
        assert_eq!(MLAA_ABS_THRESHOLD, 10);
        assert_eq!(MLAA_REL_THRESHOLD, 89);
        assert_eq!(MLAA_MAX_EDGE_LENGTH, 40);
        assert_eq!(threshold_cpu([0, 0, 0]), 10);
        assert_eq!(threshold_cpu([28, 0, 0]), 10);
        assert_eq!(threshold_cpu([0, 255, 0]), 88);
        assert_eq!(threshold_cpu([10, 20, 100]), 34);
        // Any channel, against the larger threshold of the pair.
        assert!(is_edge_cpu([20, 20, 20], [20, 30, 20]));
        assert!(!is_edge_cpu([20, 20, 20], [20, 29, 20]));
        assert!(!is_edge_cpu([200, 200, 200], [200, 200, 132]));
        assert!(is_edge_cpu([200, 200, 200], [200, 200, 131]));
        // The brighter pixel's threshold counts: 20 for 60, 31 for 90.
        assert!(is_edge_cpu([0, 0, 0], [20, 20, 20]));
        assert!(!is_edge_cpu([45, 45, 45], [64, 64, 64]));
        assert!(is_edge_cpu([59, 59, 59], [90, 90, 90]));
        assert!(!is_edge_cpu([60, 60, 60], [90, 90, 90]));
    }

    /// A two-colour picture: `a` where `inside(x, y)`, else `b`.
    fn picture(
        w: usize,
        h: usize,
        a: [u8; 3],
        b: [u8; 3],
        inside: impl Fn(usize, usize) -> bool,
    ) -> Vec<[u8; 3]> {
        (0..w * h)
            .map(|p| if inside(p % w, p / w) { a } else { b })
            .collect()
    }
    /// The share of `b` in an output colour of the `a` / `b` picture (first channel).
    fn share(out: [f32; 3], a: [u8; 3], b: [u8; 3]) -> f32 {
        (out[0] - f32::from(a[0])) / (f32::from(b[0]) - f32::from(a[0]))
    }

    /// D302: a straight edge across the picture has no steps: untouched; so is a contrast
    /// under the threshold.
    #[test]
    fn d302_straight_edge_and_low_contrast_untouched() {
        let (w, h) = (48, 16);
        let (dark, light) = ([30, 30, 30], [200, 190, 180]);
        let rgb = picture(w, h, light, dark, |_, y| y < 8);
        let out = mlaa_cpu(&rgb, w, h);
        for p in 0..w * h {
            assert_eq!(out[p], rgb[p].map(f32::from), "{p}");
        }
        // A staircase whose contrast is under the bright side's threshold (0.348 x 200 = 69).
        let stairs = picture(w, h, [200, 200, 200], [200, 200, 140], |x, y| y * 4 < x);
        let out = mlaa_cpu(&stairs, w, h);
        for p in 0..w * h {
            assert_eq!(out[p], stairs[p].map(f32::from), "{p}");
        }
    }

    /// D302: a shallow staircase (steps 16 pixels long): along each run the pixels take the
    /// colour across by the areas under the line from each step's middle to the run's
    /// middle; the 5247 copy back's 16-pixel run (MEASURED) within 0.02.
    #[test]
    fn d302_staircase_follows_the_areas_and_the_game() {
        let (w, h) = (96, 12);
        let (a, b) = ([40, 40, 40], [220, 200, 180]);
        // Row y is `a` left of x = 16 (y - 2): steps 16 long, going right one row down.
        let rgb = picture(w, h, a, b, |x, y| (x as i64) < 16 * (y as i64 - 2));
        let out = mlaa_cpu(&rgb, w, h);
        // The run on the edge between rows 3 and 4, x 16..31 (row 3 is b there, row 4 a).
        let measured = [0.41, 0.35, 0.29, 0.22, 0.16, 0.09, 0.03];
        for i in 0..16usize {
            let x = 16 + i;
            let [first, second] = areas_cpu(i as f32, 16., 0.5, -0.5);
            let up = share(out[3 * w + x], b, a);
            let down = share(out[4 * w + x], a, b);
            assert!((up - first).abs() < 1e-3, "{i} {up} {first}");
            assert!((down - second).abs() < 1e-3, "{i} {down} {second}");
            if (1..8).contains(&i) {
                assert!((first - measured[i - 1]).abs() < 0.021, "{i} {first}");
            }
            if (8..15).contains(&i) {
                assert!((second - measured[14 - i]).abs() < 0.021, "{i} {second}");
            }
        }
        // The areas: 0.5 - 1/32 at the ends, 0 at the run's middle, Z symmetric.
        assert!((areas_cpu(0., 16., 0.5, -0.5)[0] - 0.46875).abs() < 1e-6);
        assert_eq!(areas_cpu(7., 16., 0.5, -0.5), [0.03125, 0.]);
        assert_eq!(areas_cpu(8., 16., 0.5, -0.5), [0., 0.03125]);
        assert_eq!(
            areas_cpu(3., 16., 0.5, -0.5)[0],
            areas_cpu(12., 16., 0.5, -0.5)[1]
        );
    }

    /// D302: a 45-degree edge: each pixel beside it takes 1/8 of the colour across (one
    /// neighbour, the larger weight), not 1/4 (MEASURED in 5247: 0.48 of the summed model).
    #[test]
    fn d302_diagonal_takes_one_neighbour() {
        let (w, h) = (24, 24);
        let (a, b) = ([20, 20, 20], [180, 180, 180]);
        let rgb = picture(w, h, a, b, |x, y| x > y);
        let out = mlaa_cpu(&rgb, w, h);
        for y in 4..20 {
            // (x y + 1, row y) is a with b below and left of it; (y + 1, y + 1) is b with a
            // above and right of it.
            let s = share(out[y * w + y + 1], a, b);
            assert!((s - 0.125).abs() < 1e-4, "{y} {s}");
            let s = share(out[(y + 1) * w + y + 1], b, a);
            assert!((s - 0.125).abs() < 1e-4, "{y} {s}");
        }
    }

    /// D302: runs longer than the longest edge are left alone; one at the limit is blended.
    #[test]
    fn d302_long_runs_untouched() {
        let max = MLAA_MAX_EDGE_LENGTH as usize;
        let (a, b) = ([30, 30, 30], [200, 200, 200]);
        for (len, blended) in [(max, true), (max + 1, false)] {
            let w = 3 * len + 4;
            let h = 8;
            // `a` below row 4 and in row 3 from x = len + 2 to 2 len + 1: a U run of `len`
            // under row 2, both steps in the lower row.
            let rgb = picture(w, h, a, b, |x, y| {
                y >= 4 || (y == 3 && (len + 2..2 * len + 2).contains(&x))
            });
            let out = mlaa_cpu(&rgb, w, h);
            // Row 3 inside the run (its two end pixels also lie on the steps' own edges).
            let inside = len + 3..2 * len + 1;
            let changed = inside
                .clone()
                .any(|x| out[3 * w + x] != rgb[3 * w + x].map(f32::from));
            assert_eq!(changed, blended, "{len}");
            if blended {
                let s = share(out[3 * w + len + 3], a, b);
                let want = areas_cpu(1., len as f32, -0.5, -0.5)[1];
                assert!((s - want).abs() < 1e-4, "{s} {want}");
            }
        }
    }

    /// Resolves `#ifdef NAME` / `#else` / `#endif` for the given definitions.
    fn preprocess(source: &str, srgb: bool) -> String {
        let mut active = vec![true];
        let mut body = String::new();
        for line in source.lines() {
            let t = line.trim_start();
            if let Some(def) = t.strip_prefix("#ifdef ") {
                assert_eq!(def.trim(), "SRGB_TARGET");
                let parent = *active.last().unwrap();
                active.push(parent && srgb);
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

    /// D302: the passes' shader carries the constants and the mirror's rules, and validates
    /// with naga for both target kinds.
    #[test]
    fn d302_mlaa_shader_validates() {
        let raw = shader_source();
        for srgb in [true, false] {
            let source = preprocess(&raw, srgb);
            assert!(source.contains("const MLAA_ABS_THRESHOLD: f32 = 10.0;"));
            assert!(source.contains("const MLAA_REL_THRESHOLD: f32 = 89.0;"));
            assert!(source.contains("const MLAA_MAX_EDGE_LENGTH: i32 = 40;"));
            assert!(source.contains("floor(max(c.r, max(c.g, c.b)) * MLAA_REL_THRESHOLD / 256.0)"));
            assert!(source.contains(">= max(threshold(a), threshold(b))"));
            assert!(source.contains("if (len > MLAA_MAX_EDGE_LENGTH)"));
            assert!(source.contains("if (w[k] > w[best])"));
            assert!(source.contains("vec4<f32>(from_encoded(e), scene.a)"));
            assert_eq!(source.contains("return srgb_encode(c);"), srgb);
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
            for entry in ["mlaa_edges", "mlaa_blend"] {
                assert!(
                    module.entry_points.iter().any(|e| e.name == entry),
                    "{entry}"
                );
            }
        }
    }

    /// D302: the job runs only in the Pod picture on the level-lighting route; the component
    /// reaches the render world only while it is on.
    #[test]
    fn d302_route_and_extract() {
        assert!(mlaa_route(true, true, Some(true)));
        assert!(!mlaa_route(true, false, Some(true)));
        assert!(!mlaa_route(false, true, Some(true)));
        if !enabled() {
            assert!(Mlaa::extract_component(&Mlaa).is_none());
        }
        enable();
        assert_eq!(Mlaa::extract_component(&Mlaa), Some(Mlaa));
    }

    /// D304: on by default in the Pod picture on the level-lighting route, off there under
    /// `--no-mlaa`; never in another level or off the route, with or without `--mlaa`.
    #[test]
    fn d304_mlaa_on_by_default_in_the_pod() {
        // (level_route, pod, flag) -> runs
        assert!(mlaa_route(true, true, None));
        assert!(mlaa_route(true, true, Some(true)));
        assert!(!mlaa_route(true, true, Some(false)));
        for flag in [None, Some(true), Some(false)] {
            assert!(!mlaa_route(true, false, flag));
            assert!(!mlaa_route(false, true, flag));
            assert!(!mlaa_route(false, false, flag));
        }
    }
}
