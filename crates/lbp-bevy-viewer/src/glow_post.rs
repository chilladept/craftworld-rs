//! D292 (docs/level-scripts.md part 43): the glow on the Pod dissolve's green line, the
//! game's post chain (FOUND, frame 5247): the tent's shader writes alpha = t (0 on the
//! line), a half-resolution pass turns `0.7 x (1 - alpha) x colour` into a glow source,
//! two 9-tap blur passes at quarter resolution spread it, and the final pass adds it to
//! the scene. The final pass's own x1.2998 on the whole frame is not copied (part 42).
//!
//! One full-screen chain on the main 3D camera ([`EdgeGlow`]), after the main passes and
//! before egui/UI (`Core3dSystems::PostProcess`). It runs only while a `ViewerMaterial`
//! dissolve is on (other draws keep alpha 1 but are not relied on) and not under
//! `--no-edge-glow`; otherwise the frame is untouched.
use crate::dual::{self, ViewerMaterial};
use bevy::{
    core_pipeline::{tonemapping::tonemapping, Core3d, Core3dSystems, FullscreenShader},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
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

/// D292 FOUND (frame 5247, PS 14831, fc4): the glow source's strength,
/// `glow_src = GLOW_STRENGTH x (1 - scene.a) x scene.rgb` (encoded values). The menu frame
/// t1 fits about 0.4-0.45 (INFERRED, its own settings were not captured).
pub const GLOW_STRENGTH: f32 = 0.7;
/// D292 FOUND (PS 14839, passes 3281/3298): the blur's tap offsets in full-resolution
/// pixels (centre, then each side): 0, +-2, +-6, +-10, +-14 (+-1/3/5/7 half-res texels).
pub const BLUR_OFFSETS: [f32; 5] = [0., 2., 6., 10., 14.];
/// D292 FOUND (PS 14839): the blur's weights per tap (each side tap twice); not
/// normalised: one pass sums to 1.38.
pub const BLUR_WEIGHTS: [f32; 5] = [0.3, 0.225, 0.15, 0.105, 0.06];

/// D292 `--no-edge-glow`: the glow chain never runs (A/B captures). `--no-glow` is P3h's.
static NO_EDGE_GLOW: AtomicBool = AtomicBool::new(false);
pub fn disable() {
    NO_EDGE_GLOW.store(true, Ordering::Relaxed);
}

/// D292: on the camera the glow chain runs for; `active` while a dissolve is on.
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct EdgeGlow {
    pub active: bool,
}
impl SyncComponent for EdgeGlow {
    type Target = Self;
}
/// Extracted only while active: off removes it from the render view, so the chain skips.
impl ExtractComponent for EdgeGlow {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(item: &Self) -> Option<Self> {
        item.active.then_some(*item)
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
        app.add_plugins(ExtractComponentPlugin::<EdgeGlow>::default())
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

/// D292: the chain is on while any viewer material dissolves (and not `--no-edge-glow`).
fn update_active(materials: Res<Assets<ViewerMaterial>>, mut cameras: Query<&mut EdgeGlow>) {
    let on = !NO_EDGE_GLOW.load(Ordering::Relaxed)
        && materials.iter().any(|(_, m)| dual::dissolve_on(m));
    for mut glow in &mut cameras {
        if glow.active != on {
            glow.active = on;
        }
    }
}

#[derive(Resource)]
struct GlowPipelines {
    /// One texture and the sampler (source and blur passes).
    single: BindGroupLayoutDescriptor,
    /// The scene, the sampler and the blurred glow (the final add).
    composite: BindGroupLayoutDescriptor,
    sampler: Sampler,
    source: HashMap<TextureFormat, CachedRenderPipelineId>,
    composite_ids: HashMap<TextureFormat, CachedRenderPipelineId>,
    blur_h: CachedRenderPipelineId,
    blur_v: CachedRenderPipelineId,
    vertex: bevy::render::render_resource::VertexState,
}

/// Whether the main texture holds linear values that the hardware encodes (sRGB views),
/// encoded values as they are (plain 8-bit), or neither (float/HDR: not supported).
fn target_space(format: TextureFormat) -> Option<bool> {
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
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: Some(format!("d292_{entry}").into()),
        layout: vec![layout.clone()],
        vertex: vertex.clone(),
        fragment: Some(FragmentState {
            shader: SHADER,
            shader_defs: if srgb {
                vec!["SRGB_TARGET".into()]
            } else {
                Vec::new()
            },
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
    ));
    let blur_v = cache.queue_render_pipeline(descriptor(
        &vertex,
        &single,
        "glow_blur_v",
        GLOW_FORMAT,
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
struct GlowView {
    half: CachedTexture,
    quarter_h: CachedTexture,
    quarter_v: CachedTexture,
    source: CachedRenderPipelineId,
    composite: CachedRenderPipelineId,
}

fn prepare_glow(
    mut commands: Commands,
    mut pipelines: ResMut<GlowPipelines>,
    cache: Res<PipelineCache>,
    mut textures: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    views: Query<(Entity, &ViewTarget), With<EdgeGlow>>,
    stale: Query<Entity, (With<GlowView>, Without<EdgeGlow>)>,
) {
    for entity in &stale {
        commands.entity(entity).remove::<GlowView>();
    }
    for (entity, target) in &views {
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
            ))
        });
        let composite = *p.composite_ids.entry(format).or_insert_with(|| {
            cache.queue_render_pipeline(descriptor(
                &p.vertex,
                &p.composite,
                "glow_composite",
                format,
                srgb,
            ))
        });
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
        };
        commands.entity(entity).insert(view);
    }
}

/// D292: the four passes: source (half res), horizontal and vertical blur (quarter res),
/// and the add into the main texture (alpha 1).
fn edge_glow(
    view: ViewQuery<(&ViewTarget, &GlowView), With<EdgeGlow>>,
    pipelines: Res<GlowPipelines>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, glow) = view.into_inner();
    let (Some(source), Some(blur_h), Some(blur_v), Some(composite)) = (
        cache.get_render_pipeline(glow.source),
        cache.get_render_pipeline(pipelines.blur_h),
        cache.get_render_pipeline(pipelines.blur_v),
        cache.get_render_pipeline(glow.composite),
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
                    post.source,
                    &pipelines.sampler,
                    &glow.quarter_v.default_view,
                )),
            ),
            post.destination,
        ),
    ];
    let encoder = ctx.command_encoder();
    for (pipeline, bind_group, out) in &steps {
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
        "const GLOW_STRENGTH: f32 = {:?};\nconst BLUR_OFFSETS = array<f32, 5>({});\nconst BLUR_WEIGHTS = array<f32, 5>({});\n{BODY}",
        GLOW_STRENGTH,
        list(BLUR_OFFSETS),
        list(BLUR_WEIGHTS)
    )
}

const BODY: &str = r#"
// D292: the game's glow chain (PS 14831, 14839, 14857), in its encoded output space.
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var glow_texture: texture_2d<f32>;
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
"#;

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
    /// D292: opaque everywhere (alpha 1) gives no glow at all, whatever the colour.
    #[test]
    fn d292_opaque_scene_has_no_glow() {
        let (w, h) = (8, 8);
        let rgb: Vec<[f32; 3]> = (0..w * h).map(|i| [i as f32 / 64., 1., 0.5]).collect();
        let glow = glow_cpu(&rgb, &vec![1.; w * h], w, h);
        assert!(glow.iter().flatten().all(|&v| v == 0.));
    }
    /// Resolves `#ifdef SRGB_TARGET` / `#else` / `#endif` for the given choice.
    fn preprocess(srgb: bool) -> String {
        let mut active = vec![true];
        let mut body = String::new();
        for line in shader_source().lines() {
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
            assert!(!source.contains("1.2998"));
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
    /// D292: the camera's marker reaches the render world only while active.
    #[test]
    fn d292_extracted_only_while_active() {
        assert!(EdgeGlow::extract_component(&EdgeGlow { active: false }).is_none());
        assert!(EdgeGlow::extract_component(&EdgeGlow { active: true }).is_some());
    }
}
