mod adapter;
mod dual;
use dual::ViewerMaterial;
mod browser;
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use lbp_catalog::{
    attachment::AttachedSource,
    census_summary::{census_flag, CorpusCounts, LineKind, CENSUS_FLAGS_HELP},
    Catalog, LoadControl,
};
use std::{
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
};

type Context = (Arc<Catalog>, Arc<AttachedSource>);
type OwnedScene = (
    Entity,
    Handle<Mesh>,
    Handle<ViewerMaterial>,
    Option<Handle<Image>>,
);
struct Completed {
    context: Context,
    preview: Result<Option<lbp_preview::GroupedMesh>, String>,
    identity: String,
    guid: String,
    clip: Option<Result<lbp_preview::skinned::SkinnedClip, String>>,
}
/// Rd: CPU-skinned playback of one animation on the displayed mesh.
struct Playing {
    clip: lbp_preview::skinned::SkinnedClip,
    frame: adapter::DisplayFrame,
    started: f32,
    shown: Option<i64>,
    gpu: Option<GpuRig>,
}
/// Re: joint entities under a root carrying the display normalisation; the standard
/// vertex shader skins. Mesh positions stay in display space (see `gpu_rig`).
struct GpuRig {
    root: Entity,
    joints: Vec<Entity>,
    /// Morph weights are applied on the CPU into positions; skip when there are none.
    morphs: bool,
}
struct Job {
    control: LoadControl,
    receiver: Mutex<mpsc::Receiver<Result<Completed, String>>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Resource)]
struct Viewer {
    instance_input_enabled: bool,
    instance_rgba: [f32; 4],
    animation_base: bool,
    animation_guid: String,
    animation_frame: Option<i64>,
    cpu_skinning: bool,
    playing: Option<Playing>,
    snapshot: String,
    game: String,
    guid: String,
    status: String,
    context: Option<Context>,
    context_paths: (String, String),
    job: Option<Job>,
    requested: bool,
    scene: Vec<OwnedScene>,
    capture: Option<PathBuf>,
    qa: bool,
    ready_frames: u32,
    synthetic: bool,
    browser: browser::Browser,
    census: CorpusCounts,
}
impl Viewer {
    fn start(&mut self) {
        self.browser.displayed_guid = None;
        self.job = None;
        self.status = "Loading snapshot / validating read-only source / preparing preview…".into();
        self.ready_frames = 0;
        let paths = (self.snapshot.clone(), self.game.clone());
        if paths != self.context_paths {
            self.context = None;
            self.browser.clear_source();
        }
        let cached = (paths == self.context_paths)
            .then(|| self.context.clone())
            .flatten();
        let guid = self.guid.clone();
        let animation_guid = self.animation_guid.trim().to_ascii_lowercase();
        self.playing = None;
        let options = lbp_preview::PreviewOptions {
            animation_base: self.animation_base,
            instance_color: self
                .instance_input_enabled
                .then_some(lbp_preview::DiffuseTint {
                    bits: self.instance_rgba.map(f32::to_bits),
                }),
        };
        let control = LoadControl::default();
        let worker = control.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let run = || {
                let (catalog, source) = match cached {
                    Some(context) => context,
                    None => {
                        let catalog =
                            Arc::new(lbp_catalog::load(&PathBuf::from(&paths.0), &worker)?);
                        let source = Arc::new(AttachedSource::attach(
                            &PathBuf::from(&paths.1),
                            &catalog,
                            &worker,
                        )?);
                        (catalog, source)
                    }
                };
                // Attachment succeeds independently of a particular preview failure.
                // The catalog remains browsable even when the initial GUID is unsupported.
                let mut identity = String::new();
                let mut mesh_key = None;
                let preview = (|| {
                    if guid.trim().is_empty() {
                        return Ok(None);
                    }
                    let number = u32::from_str_radix(guid.trim(), 16)
                        .map_err(|_| "GUID must be eight hexadecimal digits")?;
                    if guid.trim().len() != 8 {
                        return Err("GUID must be eight hexadecimal digits".into());
                    }
                    let key = lbp_preview::resolve(&catalog, Some(number), None)?;
                    mesh_key = Some(key.clone());
                    identity = catalog.by_guid[&format!("{number:08x}")]
                        .iter()
                        .map(|&i| catalog.snapshot.resources[i].map_row.path.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ");
                    lbp_preview::load_mesh_groups_with_options(
                        &source, &catalog, &key, &worker, options,
                    )
                    .map(Some)
                })();
                let clip = (preview.as_ref().is_ok_and(Option::is_some)
                    && !animation_guid.is_empty())
                .then(|| {
                    let number = u32::from_str_radix(&animation_guid, 16)
                        .ok()
                        .filter(|_| animation_guid.len() == 8)
                        .ok_or("Animation GUID must be eight hexadecimal digits")?;
                    let key = lbp_preview::skinned::resolve_animation(&catalog, number)?;
                    let mesh_key = mesh_key.as_ref().ok_or("No mesh key")?;
                    lbp_preview::skinned::load_clip(&source, mesh_key, &key, &worker)
                });
                worker.check()?;
                Ok(Completed {
                    context: (catalog, source),
                    preview,
                    identity,
                    guid,
                    clip,
                })
            };
            let _ = tx.send(run());
        });
        self.context_paths = (self.snapshot.clone(), self.game.clone());
        self.job = Some(Job {
            control,
            receiver: Mutex::new(rx),
        });
    }
}
#[derive(Resource)]
struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
}
impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: 0.65,
            distance: 2.,
            target: Vec3::ZERO,
        }
    }
}
fn options() -> Result<Viewer, String> {
    let mut v = Viewer {
        instance_input_enabled: false,
        instance_rgba: [1.; 4],
        animation_base: false,
        animation_guid: String::new(),
        animation_frame: None,
        cpu_skinning: false,
        playing: None,
        snapshot: String::new(),
        game: String::new(),
        guid: String::new(),
        status: String::new(),
        context: None,
        context_paths: default(),
        job: None,
        requested: true,
        scene: Vec::new(),
        capture: None,
        qa: false,
        ready_frames: 0,
        synthetic: false,
        browser: browser::Browser::default(),
        census: CorpusCounts::default(),
    };
    let mut print_census = false;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--instance-rgba" {
            let value = args
                .next()
                .ok_or("--instance-rgba requires linear R,G,B,A")?;
            let parts = value
                .split(',')
                .map(str::parse::<f32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid instance RGBA")?;
            v.instance_rgba = parts
                .try_into()
                .map_err(|_| "instance RGBA requires four values")?;
            lbp_preview::DiffuseTint {
                bits: v.instance_rgba.map(f32::to_bits),
            }
            .validate()?;
            v.instance_input_enabled = true;
            continue;
        }
        if flag == "--animation-base" {
            v.animation_base = true;
            continue;
        }
        if flag == "--show-unavailable" {
            v.browser.show_unavailable = true;
            continue;
        }
        if flag == "--synthetic-affine" {
            v.synthetic = true;
            v.guid = "synthetic-affine".into();
            continue;
        }
        if flag == "--synthetic-groups" {
            v.synthetic = true;
            v.guid = "synthetic-groups".into();
            continue;
        }
        if flag == "--synthetic" {
            v.synthetic = true;
            continue;
        }
        if flag == "--print-census" {
            print_census = true;
            continue;
        }
        if let Some(role) = census_flag(&flag) {
            let path = args
                .next()
                .ok_or_else(|| format!("{flag} requires a FILE"))?;
            v.census.load(role, std::path::Path::new(&path))?;
            continue;
        }
        if flag == "--help" {
            println!("lbp-bevy-viewer --snapshot FILE --game-dir DIRECTORY [--select-guid HEX]\n--play-animation HEX: CPU-skinned playback of an animation on the selected mesh\n--animation-frame N: hold that running frame instead of playing (QA captures)\n--skinning gpu|cpu: Bevy joint skinning (default) or the CPU reference path\n--browse-query TEXT; --show-unavailable: browser filters\n--animation-base: opt-in local animation base-row snapshot (not game time)\n--synthetic: authored depth/UV/repeat/clamp scene\n--synthetic-groups: authored mixed material groups\n--synthetic-affine: authored UV scale/offset/reflection\n--qa-capture NEW_PNG_UNDER_LOCAL_ANALYSIS: capture then exit\n{CENSUS_FLAGS_HELP}\n--print-census: print the corpus-count panel text and exit (no window)\nLeft drag orbit; right/middle drag pan; wheel zoom; R reset; F frame.");
            std::process::exit(0);
        }
        let value = args.next().ok_or("Missing option value")?;
        match flag.as_str() {
            "--snapshot" => v.snapshot = value,
            "--game-dir" => v.game = value,
            "--select-guid" => v.guid = value,
            "--play-animation" => v.animation_guid = value,
            "--skinning" => {
                v.cpu_skinning = match value.as_str() {
                    "cpu" => true,
                    "gpu" => false,
                    _ => return Err("--skinning takes cpu or gpu".into()),
                }
            }
            "--animation-frame" => {
                v.animation_frame = Some(value.parse().map_err(|_| "invalid animation frame")?)
            }
            "--browse-query" => v.browser.query = value,
            "--qa-capture" => {
                let base = std::fs::canonicalize("local-analysis").map_err(|e| e.to_string())?;
                let path = PathBuf::from(value);
                let parent = std::fs::canonicalize(path.parent().ok_or("Capture parent required")?)
                    .map_err(|e| e.to_string())?;
                if !parent.starts_with(base)
                    || path.exists()
                    || path.extension().is_none_or(|v| v != "png")
                {
                    return Err("Capture must be a new PNG under local-analysis".into());
                }
                v.capture = Some(parent.join(path.file_name().ok_or("Capture filename required")?));
                v.qa = true;
            }
            _ => return Err(format!("Unknown option {flag}")),
        }
    }
    if print_census {
        for (_, line) in v.census.lines() {
            println!("{line}");
        }
        std::process::exit(0);
    }
    if !v.census.is_empty() {
        println!(
            "Corpus counts: {} mesh census file(s), texture census {}",
            v.census.mesh.len(),
            if v.census.texture.is_some() {
                "given"
            } else {
                "not given"
            }
        );
    }
    if !v.synthetic && (v.snapshot.is_empty() || v.game.is_empty()) {
        return Err(
            "Provide --snapshot, --game-dir or --synthetic (--select-guid is optional)".into(),
        );
    }
    Ok(v)
}
fn main() {
    let viewer = options().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    App::new()
        .insert_resource(viewer)
        .init_resource::<Orbit>()
        .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.045)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LBP2 Asset Explorer".into(),
                resolution: (1280, 800).into(),
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(dual::DualPlugin)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (place_on_second_monitor, load, animate, camera, capture).chain(),
        )
        .add_systems(EguiPrimaryContextPass, panel)
        .run();
}
// Keep test/launcher windows hidden until positioned on a non-primary display.
fn place_on_second_monitor(
    monitors: Query<(Entity, Option<&bevy::window::PrimaryMonitor>), With<bevy::window::Monitor>>,
    mut windows: Query<&mut Window>,
) {
    let second = monitors.iter().find(|(_, primary)| primary.is_none());
    if let Some((monitor, _)) = second {
        for mut window in &mut windows {
            if !window.visible {
                window.position = bevy::window::WindowPosition::Centered(
                    bevy::window::MonitorSelection::Entity(monitor),
                );
                println!(
                    "Preview window positioned on non-primary monitor {monitor:?} before showing"
                );
                window.visible = true;
            }
        }
    }
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 0.01,
            far: 100.,
            ..default()
        }),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::default(),
    ));
}
fn clear(
    v: &mut Viewer,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
) {
    if let Some(gpu) = v.playing.take().and_then(|p| p.gpu) {
        commands.entity(gpu.root).despawn();
    }
    for (entity, mesh, material, image) in v.scene.drain(..) {
        commands.entity(entity).despawn();
        meshes.remove(mesh.id());
        if let Some(second) = materials
            .get(material.id())
            .and_then(|m| m.extension.image.as_ref())
        {
            images.remove(second.id());
        }
        if let Some(third) = materials
            .get(material.id())
            .and_then(|m| m.extension.third_image.as_ref())
        {
            images.remove(third.id());
        }
        if let Some(fourth) = materials
            .get(material.id())
            .and_then(|m| m.extension.fourth_image.as_ref())
        {
            images.remove(fourth.id());
        }
        materials.remove(material.id());
        if let Some(image) = image {
            images.remove(image.id());
        }
    }
}
fn load(
    mut commands: Commands,
    mut v: ResMut<Viewer>,
    mut orbit: ResMut<Orbit>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ViewerMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    if v.requested {
        v.requested = false;
        clear(
            &mut v,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
        );
        if v.synthetic {
            if v.guid == "synthetic-groups" {
                v.scene = synthetic_groups(&mut commands, &mut meshes, &mut materials, &mut images);
                *orbit = Orbit {
                    yaw: 0.,
                    pitch: 0.,
                    distance: 2.,
                    ..default()
                };
                v.status = "Authored primitive groups: red left; green upper right; neutral unsupported lower right. Repeated red binding shares one draw. Shared whole-mesh framing.".into();
                return;
            }
            synthetic(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                v.guid == "synthetic-affine",
            );
            *orbit = Orbit {
                yaw: 0.,
                pitch: 0.,
                distance: 3.,
                ..default()
            };
            v.status="Authored fixture: left UV0 quadrants (red/green above blue/yellow); middle repeat UV 0..2; right clamp UV 0..2. Red front square occludes blue back square.".into();
            if v.guid == "synthetic-affine" {
                v.status = "Authored affine fixture: left raw UV quadrants; middle repeat UV × (2,2) + (0.25,0.25); right clamp UV × (-1,1) + (1,0), horizontally reflected. Raw UV arrays preserved.".into();
            }
            return;
        }
        v.start();
    }
    let result = v
        .job
        .as_ref()
        .and_then(|job| match job.receiver.lock().unwrap().try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Worker disconnected".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
    let Some(result) = result else { return };
    let job = v.job.take().unwrap();
    if job.control.check().is_err() {
        return;
    }
    let attempted_guid = result
        .as_ref()
        .ok()
        .map(|done| done.guid.trim().to_ascii_lowercase());
    let mut evidence = browser::PreviewEvidence::default();
    let rendered = (|| -> Result<(), String> {
        let done = result?;
        v.context = Some(done.context);
        let Some(preview) = done.preview? else {
            v.status = "Source attached. Choose an asset in the browser.".into();
            return Ok(());
        };
        let g = &preview.geometry;
        let buckets = adapter::buckets(&g.triangles, &g.primitive_triangles, &preview.bindings)?;
        for (range, binding) in g.primitive_triangles.iter().zip(&preview.bindings) {
            if !range.is_empty() {
                evidence.total_groups += 1;
                if let Ok(asset) = binding.as_deref() {
                    evidence.unrendered_texture_groups +=
                        usize::from(!asset.additional_texture_slots().is_empty());
                    evidence.instance_input |= asset.instance_projection().is_some();
                    evidence.animation_snapshot |= asset
                        .texture()
                        .is_some_and(|a| a.selection.animation_base.is_some());
                }
                match binding.as_deref() {
                    Ok(lbp_preview::MaterialAsset::Texture(_)) => evidence.textured_groups += 1,
                    Ok(lbp_preview::MaterialAsset::Solid(_)) => evidence.solid_groups += 1,
                    Err(_) => {}
                }
            }
        }
        let mut details = Vec::new();
        for (index, (range, binding)) in g
            .primitive_triangles
            .iter()
            .zip(&preview.bindings)
            .enumerate()
        {
            let label = match binding {
                Ok(a) => a.label(),
                Err(e) => format!("Neutral: {e}"),
            };
            details.push(format!(
                "Primitive {index}: {} triangles • {label}",
                range.len()
            ));
        }
        // Prepare all adapters before allocating GPU assets. Shared full-mesh bounds
        // keep different groups aligned; supported groups share one draw per binding.
        let mut prepared = Vec::new();
        for (asset, triangles) in buckets {
            if triangles.is_empty() {
                continue;
            }
            let mut mesh = match adapter::mesh(
                &g.positions,
                &triangles,
                asset.as_ref().and_then(|a| a.sampling_uv()),
            ) {
                Ok(mesh) => mesh,
                Err(e) if e == "Unsupported: no nondegenerate faces" => {
                    evidence.skipped_geometry = true;
                    details.push(e);
                    continue;
                }
                Err(e) => return Err(e),
            };
            let alpha_mix = asset
                .as_ref()
                .and_then(|a| a.texture())
                .is_some_and(|a| a.dual.is_some() && a.selection.texture_alpha_mix.is_some());
            let second_image = if let Some(second) = asset
                .as_ref()
                .and_then(|a| a.texture())
                .and_then(|a| a.dual.as_ref())
            {
                adapter::secondary_uv(&mut mesh, &second.sampling_uv)?;
                Some(adapter::image_axes(
                    second.texture.header.width,
                    second.texture.header.height,
                    second.texture.rgba.clone(),
                    second.wrap,
                )?)
            } else {
                None
            };
            let image = asset
                .as_ref()
                .and_then(|a| a.texture())
                .map(|a| {
                    let [width, height] = a.display_extent();
                    let mut image =
                        adapter::image_axes(width, height, a.display_pixels().to_vec(), a.wrap)?;
                    if a.nearest_sampling() {
                        if let bevy::image::ImageSampler::Descriptor(s) = &mut image.sampler {
                            s.mag_filter = bevy::image::ImageFilterMode::Nearest;
                            s.min_filter = bevy::image::ImageFilterMode::Nearest;
                        }
                    }
                    Ok::<_, String>(image)
                })
                .transpose()?;
            let color = match asset.as_deref() {
                Some(lbp_preview::MaterialAsset::Solid(a)) => {
                    let [r, g, b, _] = a.selection.color.values();
                    Color::linear_rgba(r, g, b, 1.)
                }
                Some(lbp_preview::MaterialAsset::Texture(_)) => Color::WHITE,
                None => Color::srgb(0.65, 0.68, 0.72),
            };
            let third_image = asset
                .as_ref()
                .and_then(|a| a.texture())
                .and_then(|a| a.third.as_ref())
                .map(|third| {
                    let relative = third.relative.ok_or("Missing third-sample coordinates")?;
                    adapter::image(
                        third.texture.header.width,
                        third.texture.header.height,
                        third.texture.rgba.clone(),
                        true,
                    )
                    .map(|image| (image, relative))
                })
                .transpose()?;
            let fourth_image = asset
                .as_ref()
                .and_then(|a| a.texture())
                .and_then(|a| a.fourth.as_ref())
                .map(|fourth| {
                    let relative = fourth.relative.ok_or("Missing fourth-sample coordinates")?;
                    adapter::image(
                        fourth.texture.header.width,
                        fourth.texture.header.height,
                        fourth.texture.rgba.clone(),
                        true,
                    )
                    .map(|image| (image, relative))
                })
                .transpose()?;
            prepared.push((
                mesh,
                image,
                second_image,
                third_image,
                fourth_image,
                color,
                alpha_mix,
            ));
        }
        if prepared.is_empty() {
            return Err("Unavailable: no nondegenerate faces to display".into());
        }
        for (mesh, image, second_image, third_image, fourth_image, color, alpha_mix) in prepared {
            let image = image.map(|i| images.add(i));
            let mut surface = dual::material(StandardMaterial {
                base_color: color,
                base_color_texture: image.clone(),
                unlit: true,
                cull_mode: None,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            });
            if let Some(second) = second_image {
                dual::enable(&mut surface, images.add(second));
                if alpha_mix {
                    dual::enable_alpha_mix(&mut surface);
                }
            }
            if let Some((third, relative)) = third_image {
                dual::enable_third(&mut surface, images.add(third), relative);
            }
            if let Some((fourth, relative)) = fourth_image {
                dual::enable_fourth(&mut surface, images.add(fourth), relative);
            }
            let material = materials.add(surface);
            let mesh = meshes.add(mesh);
            let entity = commands
                .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())))
                .id();
            v.scene.push((entity, mesh, material, image));
        }
        *orbit = Orbit::default();
        let clip_status = match done.clip {
            None => String::new(),
            Some(Err(e)) => format!("\nAnimation not played: {e}"),
            Some(Ok(clip)) => {
                let r = &clip.report;
                let text = format!(
                    "\nAnimation: {} frames at {} fps{} • {}/{} mesh bones driven ({} animation bones unmatched) • morphs {} mesh / {} animation",
                    r.frames,
                    r.fps,
                    if r.looped { ", looped" } else { "" },
                    r.matched_bones,
                    r.mesh_bones,
                    r.unmatched_animation_bones,
                    r.mesh_morphs,
                    r.animation_morphs
                );
                let frame = adapter::display_frame(&g.positions)
                    .ok_or("Unsupported: zero-size geometry")?;
                let gpu = if v.cpu_skinning {
                    None
                } else {
                    let scene: Vec<_> = v.scene.iter().map(|s| (s.0, s.1.clone())).collect();
                    let mut rig = gpu_rig(
                        &mut commands,
                        &mut meshes,
                        &mut bindposes,
                        &clip.rig,
                        (&clip.skin.joints, &clip.skin.weights),
                        frame,
                        &scene,
                    )?;
                    rig.morphs = !clip.skin.morphs.is_empty() && clip.report.animation_morphs > 0;
                    Some(rig)
                };
                let path = if gpu.is_some() {
                    "GPU joints"
                } else {
                    "CPU reference"
                };
                v.playing = Some(Playing {
                    clip,
                    frame,
                    started: -1.,
                    shown: None,
                    gpu,
                });
                format!("{text} • skinning: {path}")
            }
        };
        v.status = format!(
            "Loaded {}\n{}\n{} vertices • {} triangles • {} primitives{}{}",
            done.guid,
            done.identity,
            g.positions.len(),
            g.triangles.len(),
            g.metadata.primitives.len(),
            clip_status + "\n",
            details.join("\n")
        );
        println!("{}", v.status);
        Ok(())
    })();
    if let Some(guid) = attempted_guid.filter(|guid| !guid.is_empty()) {
        v.browser.record(guid, rendered.clone().map(|()| evidence));
        let (viewable, complete, failed) = v.browser.counts();
        println!("Compatibility (source session, distinct GUIDs): {viewable} viewable; {complete} complete; {failed} failed");
    }
    if let Err(e) = rendered {
        v.status = format!("Load failed: {e}");
        eprintln!("{}", v.status);
    }
}
/// Advances the playing clip one integer frame at a time (no interpolation) and
/// rewrites every scene mesh's positions in the bind-pose display frame.
/// Spawns the joint hierarchy and turns every scene mesh into a skinned mesh.
fn gpu_rig(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    rig: &lbp_preview::skinned::Rig,
    (skin_joints, skin_weights): (&[[u8; 4]], &[[f32; 4]]),
    (centre, span): adapter::DisplayFrame,
    scene: &[(Entity, Handle<Mesh>)],
) -> Result<GpuRig, String> {
    let bones = rig.bone_count();
    // One extra static joint holds vertices without a usable weight in place.
    if bones + 1 > bevy::pbr::MAX_JOINTS {
        return Err(format!(
            "Unsupported: {bones} bones exceed the GPU joint limit; use --skinning cpu"
        ));
    }
    // display = N × raw, N = scale(1/span) × translate(−centre). Joint globals become
    // N × world, so inverse binds take N⁻¹ to cancel it on display-space vertices.
    let centre = Vec3::from_array(centre.map(|c| c as f32));
    let span = span as f32;
    let n = Mat4::from_scale(Vec3::splat(1. / span)) * Mat4::from_translation(-centre);
    let n_inv = Mat4::from_translation(centre) * Mat4::from_scale(Vec3::splat(span));
    let mut inverse: Vec<Mat4> = rig
        .inverse_binds()
        .iter()
        .map(|m| Mat4::from_cols_array(m) * n_inv)
        .collect();
    inverse.push(n_inv);
    let root = commands
        .spawn((Transform::from_matrix(n), Visibility::default()))
        .id();
    let local = rig.local_matrices(None);
    let mut joints: Vec<Entity> = local
        .iter()
        .map(|m| {
            commands
                .spawn(Transform::from_matrix(Mat4::from_cols_array(m)))
                .id()
        })
        .collect();
    for (i, parent) in rig.parents().into_iter().enumerate() {
        let parent = usize::try_from(parent).map_or(root, |p| joints[p]);
        commands.entity(joints[i]).insert(ChildOf(parent));
    }
    joints.push(commands.spawn((Transform::IDENTITY, ChildOf(root))).id());
    let inverse_bindposes = bindposes.add(SkinnedMeshInverseBindposes::from(inverse));
    let (joint_index, joint_weight) =
        lbp_preview::skinned::gpu_skin_attributes(skin_joints, skin_weights, bones);
    for (entity, handle) in scene {
        let mut mesh = meshes.get_mut(handle).ok_or("Missing scene mesh")?;
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            bevy::mesh::VertexAttributeValues::Uint16x4(joint_index.clone()),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weight.clone());
        commands.entity(*entity).insert((
            SkinnedMesh {
                // Strong handles: the asset is freed with the scene entities.
                inverse_bindposes: inverse_bindposes.clone(),
                joints: joints.clone(),
            },
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    Ok(GpuRig {
        root,
        joints,
        morphs: false,
    })
}
fn animate(
    time: Res<Time>,
    mut v: ResMut<Viewer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut transforms: Query<&mut Transform>,
) {
    let held = v.animation_frame;
    let handles: Vec<Handle<Mesh>> = v.scene.iter().map(|s| s.1.clone()).collect();
    let Some(playing) = v.playing.as_mut() else {
        return;
    };
    let now = time.elapsed_secs();
    if playing.started < 0. {
        playing.started = now;
    }
    let fps = f32::from(playing.clip.report.fps.max(1));
    let frame = held.unwrap_or(((now - playing.started) * fps) as i64);
    if playing.shown == Some(frame) {
        return;
    }
    playing.shown = Some(frame);
    let posed = if let Some(gpu) = &playing.gpu {
        let Some(pose) = playing.clip.frame_pose(frame) else {
            return;
        };
        for (joint, m) in gpu
            .joints
            .iter()
            .zip(playing.clip.rig.local_matrices(Some(&pose)))
        {
            if let Ok(mut t) = transforms.get_mut(*joint) {
                *t = Transform::from_matrix(Mat4::from_cols_array(&m));
            }
        }
        if !gpu.morphs {
            return;
        }
        playing.clip.morphed_bind(&pose)
    } else {
        let Some(posed) = playing.clip.frame_positions(frame) else {
            return;
        };
        posed
    };
    let display = adapter::to_display(&posed, playing.frame);
    for handle in handles {
        if let Some(mut mesh) = meshes.get_mut(&handle) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, display.clone());
        }
    }
}
fn panel(mut contexts: EguiContexts, mut v: ResMut<Viewer>, mut orbit: ResMut<Orbit>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if !v.synthetic {
        let mut browser_ui = egui::Ui::new(
            ctx.clone(),
            "browser_root".into(),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(ctx.viewport_rect()),
        );
        egui::Panel::left("asset_browser")
            .default_size(390.)
            .min_size(260.)
            .show(&mut browser_ui, |ui| {
                ui.heading("Browse assets");
                let context = v.context.clone();
                if let Some((catalog, _)) = context {
                    let selected = v.guid.clone();
                    if let Some(guid) = v.browser.panel(ui, &catalog, &selected) {
                        v.guid = guid;
                        v.requested = true;
                    }
                } else {
                    ui.label("Attach a snapshot and game directory to browse.");
                }
            });
    }
    egui::Window::new("Asset preview").default_pos(if v.synthetic { [10., 10.] } else { [900., 10.] }).default_width(310.).show(ctx, |ui| {
        if !v.synthetic {
            ui.label("Snapshot");
            ui.text_edit_singleline(&mut v.snapshot);
            ui.label("Extracted game directory");
            ui.text_edit_singleline(&mut v.game);
            ui.label("Mesh GUID (blank: attach and browse)");
            ui.text_edit_singleline(&mut v.guid);
            ui.label("Play animation GUID (blank: bind pose)");
            ui.text_edit_singleline(&mut v.animation_guid);
            if ui.button("Load / reload").clicked() { v.requested = true; }
            if v.job.is_some() && ui.button("Cancel").clicked() {
                v.job = None;
                v.status = "Cancelled".into();
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() { *orbit = Orbit::default(); }
            if ui.button("Frame selected").clicked() {
                orbit.target = Vec3::ZERO;
                orbit.distance = 2.;
            }
        });
        ui.separator();
        if !v.synthetic {
            if ui.checkbox(&mut v.animation_base,"Preview stored material animation base values").changed() {
                v.browser.clear_source();v.requested=true;
            }
            if v.animation_base {ui.small("Stored base rows matched to global declarations; no game time or motion inferred. Reload clears verification.");}
            let mut input_changed = ui.checkbox(&mut v.instance_input_enabled,"Supply instance color for preview").changed();
            if v.instance_input_enabled {
                ui.small("Supplied linear RGBA; actual game instance values are unverified.");
                ui.horizontal(|ui| {
                    for (name, value) in ["R", "G", "B", "A"].into_iter().zip(v.instance_rgba.iter_mut()) {
                        ui.label(name);
                        input_changed |= ui.add(egui::DragValue::new(value).range(0.0..=1.0).speed(0.01)).changed();
                    }
                });
            }
            if input_changed { v.browser.clear_source(); v.requested = true; }
            v.browser.review_panel(ui);
        }
        egui::ScrollArea::vertical().max_height(260.).show(ui, |ui| { ui.label(&v.status); });
        ui.separator();
        ui.label("Left drag: orbit\nRight / middle drag: pan\nWheel: zoom • R: reset • F: frame");
        ui.small("Unlit • opaque • sRGB • double-sided\nGenerated diagnostic normals. Animation: CPU skinning, positions only.\nTop mip only. Original game lighting, alpha and axis conventions unverified.");
    });
    if !v.synthetic || !v.census.is_empty() {
        census_window(ctx, &v.census);
    }
    Ok(())
}
/// Corpus-count window; text and validation come from `lbp_catalog::census_summary`.
fn census_window(ctx: &egui::Context, census: &CorpusCounts) {
    egui::Window::new("Corpus counts (census files)")
        .default_pos([420., 10.])
        .default_width(380.)
        .default_open(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(420.)
                .show(ui, |ui| {
                    for (kind, text) in census.lines() {
                        match kind {
                            LineKind::Heading => {
                                ui.separator();
                                ui.strong(text);
                            }
                            LineKind::Value => {
                                ui.label(text);
                            }
                            LineKind::Note => {
                                ui.small(text);
                            }
                        }
                    }
                });
        });
}
// Bevy injects independent input resources and the camera query into this system.
#[allow(clippy::too_many_arguments)]
fn camera(
    mut contexts: EguiContexts,
    v: Res<Viewer>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut orbit: ResMut<Orbit>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let blocked = v.qa
        || contexts
            .ctx_mut()
            .is_ok_and(|ctx| ctx.egui_wants_pointer_input() || ctx.egui_wants_keyboard_input());
    if !blocked {
        if keys.just_pressed(KeyCode::KeyR) {
            *orbit = Orbit::default();
        }
        if keys.just_pressed(KeyCode::KeyF) {
            orbit.target = Vec3::ZERO;
            orbit.distance = 2.;
        }
        if buttons.pressed(MouseButton::Left) {
            orbit.yaw -= motion.delta.x * 0.008;
            orbit.pitch = (orbit.pitch + motion.delta.y * 0.008).clamp(-1.5, 1.5);
        }
        let rotation = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(-orbit.pitch);
        if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
            let distance = orbit.distance;
            orbit.target +=
                rotation * Vec3::new(-motion.delta.x, motion.delta.y, 0.) * distance * 0.0015;
            orbit.target = orbit.target.clamp(Vec3::splat(-10.), Vec3::splat(10.));
        }
        orbit.distance = (orbit.distance * (-scroll.delta.y * 0.12).exp()).clamp(0.15, 30.);
    }
    let rotation = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(-orbit.pitch);
    for mut transform in &mut cameras {
        *transform =
            Transform::from_translation(orbit.target + rotation * Vec3::Z * orbit.distance)
                .looking_at(orbit.target, Vec3::Y);
    }
}
fn capture(mut commands: Commands, mut v: ResMut<Viewer>, time: Res<Time>) {
    if v.job.is_none() && !v.requested {
        v.ready_frames += 1;
    }
    if v.ready_frames > 120 && time.elapsed_secs() > 10. {
        if let Some(path) = v.capture.take() {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path))
                .observe(
                    |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                        exit.write(AppExit::Success);
                    },
                );
        }
    }
}
fn synthetic(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
    affine: bool,
) {
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 64,
    ];
    for (x, extent, repeat) in [(-0.75, 1., false), (0., 2., true), (0.75, 2., false)] {
        let raw_uv = if affine {
            vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
        } else {
            vec![[0., 0.], [extent, 0.], [extent, extent], [0., extent]]
        };
        let transform = lbp_preview::UvTransform {
            bits: if !affine || x < 0. {
                [1., 1., 0., 0.]
            } else if x == 0. {
                [2., 2., 0.25, 0.25]
            } else {
                [-1., 1., 1., 0.]
            }
            .map(f32::to_bits),
        };
        let sampling_uv = transform.sampling(&raw_uv, || false).unwrap();
        let mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-0.3, 0.3, 0.],
                [0.3, 0.3, 0.],
                [0.3, -0.3, 0.],
                [-0.3, -0.3, 0.],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, sampling_uv)
        .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 2, 1, 0, 3, 2]));
        let mut image = adapter::image(2, 2, rgba.clone(), repeat).unwrap();
        if let bevy::image::ImageSampler::Descriptor(ref mut s) = image.sampler {
            s.mag_filter = bevy::image::ImageFilterMode::Nearest;
            s.min_filter = bevy::image::ImageFilterMode::Nearest;
        }
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(dual::material(StandardMaterial {
                base_color_texture: Some(images.add(image)),
                unlit: true,
                cull_mode: None,
                ..default()
            }))),
            Transform::from_xyz(x, 0.35, 0.),
        ));
    }
    // Submit foreground before background: visibility must come from depth, not submission order.
    for (z, scale, color) in [
        (0.2, 0.3, Color::srgb(1., 0., 0.)),
        (0., 0.6, Color::srgb(0., 0., 1.)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(scale, scale))),
            MeshMaterial3d(materials.add(dual::material(StandardMaterial {
                base_color: color,
                unlit: true,
                cull_mode: None,
                ..default()
            }))),
            Transform::from_xyz(0., -0.55, z),
        ));
    }
}

fn synthetic_groups(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
) -> Vec<OwnedScene> {
    let positions = [
        [-1., 1., 0.],
        [0., 1., 0.],
        [0., 0., 0.],
        [-1., 0., 0.],
        [1., 1., 0.],
        [1., 0., 0.],
        [0., -1., 0.],
        [1., -1., 0.],
    ];
    let triangles = [
        [0, 2, 1],
        [0, 3, 2],
        [1, 5, 4],
        [1, 2, 5],
        [2, 7, 5],
        [2, 6, 7],
    ];
    let asset = |rgba: Vec<u8>| {
        Arc::new(lbp_preview::Asset {
            instance_projection: None,
            additional_texture_slots: vec![],
            border_domain: None,
            fourth: None,
            third: None,
            dual: None,
            product: None,
            tinted_rgba: None,
            uv: vec![[0.5, 0.5]; 8],
            sampling_uv: vec![[0.5, 0.5]; 8],
            uv_metadata: lbp_mesh::UvMetadata {
                vertex_count: 8,
                channel_count: 1,
                channels: vec![],
            },
            texture: lbp_texture::Texture {
                header: lbp_texture::Header {
                    pixel_format: lbp_texture::PixelFormat::BGRA8,
                    width: 1,
                    height: 1,
                    mips: 1,
                    wrapper: "authored",
                    flags: None,
                    remap: None,
                    format_raw: None,
                    no_srgb: false,
                    dds: None,
                },
                expanded_bytes: 4,
                top_mip_block_bytes: 4,
                unparsed_after_top_mip: 0,
                rgba,
            },
            material_key: lbp_catalog::attachment::LocationKey {
                path: "authored".into(),
                offset: 0,
                size: 0,
            },
            texture_key: lbp_catalog::attachment::LocationKey {
                path: "authored".into(),
                offset: 0,
                size: 0,
            },
            wrap: lbp_preview::TextureWrapping::uniform(false),
            selection: lbp_preview::DiffuseSelection::default(),
        })
    };
    let red = asset(vec![255, 0, 0, 255]);
    let green = asset(vec![0, 255, 0, 255]);
    let buckets = adapter::buckets(
        &triangles,
        &[0..1, 1..2, 2..4, 4..6],
        &[
            Ok(red.clone()),
            Ok(red),
            Ok(green),
            Err("Unsupported: authored complex graph".into()),
        ],
    )
    .unwrap();
    buckets
        .into_iter()
        .map(|(asset, triangles)| {
            let mesh = meshes.add(
                adapter::mesh(
                    &positions,
                    &triangles,
                    asset.as_ref().map(|a| a.sampling_uv.as_slice()),
                )
                .unwrap(),
            );
            let image = asset
                .map(|a| images.add(adapter::image(1, 1, a.texture.rgba.clone(), false).unwrap()));
            let material = materials.add(dual::material(StandardMaterial {
                base_color: if image.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(0.65, 0.68, 0.72)
                },
                base_color_texture: image.clone(),
                unlit: true,
                cull_mode: None,
                ..default()
            }));
            let entity = commands
                .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())))
                .id();
            (entity, mesh, material, image)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Re: the joint hierarchy spawned for GPU skinning gives, after transform
    /// propagation, joint global × inverse bind = N × CPU skin matrix × N⁻¹.
    #[test]
    fn gpu_joint_hierarchy_matches_cpu_skin_matrices() {
        use lbp_preview::skinned::{trs, Rig, RigBone};
        let at = |y: f32| trs([0., y, 0.], [0., 0., 0., 1.], [1.; 3]);
        let rig = Rig::new(
            vec![
                RigBone {
                    parent: -1,
                    anim_hash: 10,
                    skin_pose: at(0.),
                    inv_skin_pose: at(0.),
                },
                RigBone {
                    parent: 0,
                    anim_hash: 20,
                    skin_pose: at(1.),
                    inv_skin_pose: at(-1.),
                },
            ],
            &[20, 10],
        )
        .unwrap();
        let pose = lbp_preview::skinned::Pose {
            rotations: vec![[0.6, 0., 0., 0.8], [0., 0., 0.6, 0.8]],
            positions: vec![[0., 1., 0.], [2., 0., 0.]],
            scales: vec![[1.; 3], [1.5; 3]],
            morphs: vec![],
        };
        let frame = ([1., 2., 3.], 4.);
        let mut app = App::new();
        app.add_plugins(bevy::transform::TransformPlugin);
        let world = app.world_mut();
        let mut meshes = Assets::<Mesh>::default();
        let mut bindposes = Assets::<SkinnedMeshInverseBindposes>::default();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        let gpu = gpu_rig(
            &mut commands,
            &mut meshes,
            &mut bindposes,
            &rig,
            (&[], &[]),
            frame,
            &[],
        )
        .unwrap();
        queue.apply(world);
        for (joint, m) in gpu.joints.iter().zip(rig.local_matrices(Some(&pose))) {
            *world.get_mut::<Transform>(*joint).unwrap() =
                Transform::from_matrix(Mat4::from_cols_array(&m));
        }
        app.update();
        let (_, inverse) = bindposes.iter().next().unwrap();
        let n =
            Mat4::from_scale(Vec3::splat(0.25)) * Mat4::from_translation(-Vec3::new(1., 2., 3.));
        for (i, cpu) in rig.skin_matrices(Some(&pose)).iter().enumerate() {
            let global = app.world().get::<GlobalTransform>(gpu.joints[i]).unwrap();
            let got = global.to_matrix() * inverse[i];
            let want = n * Mat4::from_cols_array(cpu) * n.inverse();
            assert!(got.abs_diff_eq(want, 1e-4), "{i}: {got} vs {want}");
        }
        // The static joint leaves display-space vertices where they are.
        let fixed = app.world().get::<GlobalTransform>(gpu.joints[2]).unwrap();
        assert!((fixed.to_matrix() * inverse[2]).abs_diff_eq(Mat4::IDENTITY, 1e-5));
    }
    #[test]
    fn hidden_window_only_becomes_visible_on_non_primary_monitor() {
        let mut app = App::new();
        app.add_systems(Update, place_on_second_monitor);
        let monitor = || bevy::window::Monitor {
            name: None,
            physical_height: 1080,
            physical_width: 1920,
            physical_position: IVec2::ZERO,
            refresh_rate_millihertz: None,
            scale_factor: 1.,
            video_modes: Vec::new(),
        };
        app.world_mut()
            .spawn((monitor(), bevy::window::PrimaryMonitor));
        let window = app
            .world_mut()
            .spawn(Window {
                visible: false,
                ..default()
            })
            .id();
        app.update();
        assert!(!app.world().get::<Window>(window).unwrap().visible);
        let second = app.world_mut().spawn(monitor()).id();
        app.update();
        let placed = app.world().get::<Window>(window).unwrap();
        assert!(placed.visible);
        assert_eq!(
            placed.position,
            bevy::window::WindowPosition::Centered(bevy::window::MonitorSelection::Entity(second))
        );
        app.world_mut().get_mut::<Window>(window).unwrap().position =
            bevy::window::WindowPosition::At(IVec2::new(20, 30));
        app.update();
        assert_eq!(
            app.world().get::<Window>(window).unwrap().position,
            bevy::window::WindowPosition::At(IVec2::new(20, 30))
        );
    }
    #[test]
    fn dropping_replaced_job_cancels_and_disconnects_stale_result() {
        let control = LoadControl::default();
        let (tx, rx) = mpsc::sync_channel(1);
        let job = Job {
            control: control.clone(),
            receiver: Mutex::new(rx),
        };
        drop(job);
        assert!(control.check().is_err());
        assert!(tx.send(Err("stale".into())).is_err());
    }
    #[test]
    fn replacing_scene_removes_entities_and_all_owned_gpu_assets() {
        let mut world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<ViewerMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mesh = meshes.add(Rectangle::new(1., 1.));
        let image = images.add(adapter::image(1, 1, vec![255; 4], false).unwrap());
        let material = materials.add(dual::material(StandardMaterial {
            base_color_texture: Some(image.clone()),
            ..default()
        }));
        let entity = world.spawn_empty().id();
        let mut v = Viewer {
            instance_input_enabled: false,
            instance_rgba: [1.; 4],
            animation_base: false,
            animation_guid: String::new(),
            animation_frame: None,
            cpu_skinning: false,
            playing: None,
            snapshot: String::new(),
            game: String::new(),
            guid: String::new(),
            status: String::new(),
            context: None,
            context_paths: default(),
            job: None,
            requested: false,
            scene: vec![(entity, mesh, material, Some(image))],
            capture: None,
            qa: false,
            ready_frames: 0,
            synthetic: false,
            browser: browser::Browser::default(),
            census: CorpusCounts::default(),
        };
        let mut entities = vec![entity];
        for _ in 0..3 {
            let mesh = meshes.add(Rectangle::new(1., 1.));
            let image = images.add(adapter::image(1, 1, vec![255; 4], false).unwrap());
            let mut surface = dual::material(StandardMaterial {
                base_color_texture: Some(image.clone()),
                ..default()
            });
            let second = images.add(adapter::image(1, 1, vec![128; 4], true).unwrap());
            dual::enable(&mut surface, second);
            let third = images.add(adapter::image(1, 1, vec![64; 4], true).unwrap());
            dual::enable_third(
                &mut surface,
                third,
                lbp_preview::RelativeSampling {
                    from_primary: false,
                    scale: [1., 1.],
                    offset: [0., 0.],
                },
            );
            let fourth = images.add(adapter::image(1, 1, vec![32; 4], true).unwrap());
            dual::enable_fourth(
                &mut surface,
                fourth,
                lbp_preview::RelativeSampling {
                    from_primary: true,
                    scale: [1., 1.],
                    offset: [0., 0.],
                },
            );
            let material = materials.add(surface);
            let entity = world.spawn_empty().id();
            entities.push(entity);
            v.scene.push((entity, mesh, material, Some(image)));
        }
        clear(
            &mut v,
            &mut Commands::new(&mut queue, &world),
            &mut meshes,
            &mut materials,
            &mut images,
        );
        queue.apply(&mut world);
        assert!(
            v.scene.is_empty() && meshes.is_empty() && materials.is_empty() && images.is_empty()
        );
        assert!(entities
            .into_iter()
            .all(|entity| world.get_entity(entity).is_err()));
    }
}
