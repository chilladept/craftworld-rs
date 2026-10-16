mod adapter;
mod colour_grade;
mod dof_post;
mod dual;
mod fog_tint;
mod game_font;
mod glow_post;
mod gooey_ui;
mod menu_layer;
mod mlaa_post;
mod pod_menu;
mod popit_browser;
mod popit_play;
mod puter;
mod slot_browser;
mod text_browser;
mod ui_script;
use dual::ViewerMaterial;
mod browser;
mod level_play;
mod level_selector;
mod lighting;
mod pod_dissolve;
mod sackboy_play;
mod speech_overlay;
mod thing_view;
mod water;
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
use lbp_preview::level_scene;
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
    level: Option<Result<LevelLoaded, String>>,
    clip: Option<ClipResult>,
    /// P2d `--pod-hold-animation`: the figure's clip while the Puter is open.
    hold_clip: Option<ClipResult>,
    /// `--play-sackboy` clip bank: the clips after `clip` (stand), bound to its rig, in
    /// `lbp_sim::sackboy_anim::ClipId::ALL` order (None with `--play-animation`).
    play_bank: Option<Vec<Result<lbp_preview::skinned::BoundAnimation, String>>>,
}
type ClipResult = Result<lbp_preview::skinned::SkinnedClip, String>;
/// A shell CHECKPOINT Thing and its world matrix (`LevelScene::checkpoints`).
type Checkpoint = (usize, [f32; 16]);
/// Rd: CPU-skinned playback of one animation on the displayed mesh.
struct Playing {
    clip: lbp_preview::skinned::SkinnedClip,
    frame: adapter::DisplayFrame,
    started: f32,
    shown: Option<i64>,
    gpu: Option<GpuRig>,
    /// The meshes the clip poses (CPU positions and morphs).
    meshes: Vec<Handle<Mesh>>,
    /// P2d: the other Pod figure clip (standing / holding), swapped in by `pod_figure`.
    alt: Option<lbp_preview::skinned::SkinnedClip>,
    /// D232: a pose blended from several clips (\--play-sackboy\ layers), applied once by
    /// \nimate\ and then cleared.
    blended: Option<lbp_preview::skinned::BlendPose>,
    /// D232: posed only from \lended\ (the frame clock is ignored).
    layered: bool,
}
/// Re: joint entities under a root carrying the display normalisation; the standard
/// vertex shader skins. Mesh positions stay in display space (see `gpu_rig`).
struct GpuRig {
    root: Entity,
    joints: Vec<Entity>,
    /// One scale node under each joint when the rig's scale rule is not plain
    /// inheritance (`Rig::joint_inherits`); these are then the skinning joints. Empty
    /// otherwise.
    scales: Vec<Entity>,
    /// Morph weights are applied on the CPU into positions; skip when there are none.
    morphs: bool,
}
type LevelScene = level_scene::LevelScene<level_scene::ScenePayload>;
struct LevelLoaded {
    scene: LevelScene,
    identity: String,
    /// P2b `--pod-shell HEX`: the Pod shell level drawn over this one.
    shell: Option<LevelScene>,
    /// The shell's own spawn for the figure, from its data (feet in shell level units).
    shell_feet: Option<Result<([f32; 3], String), String>>,
    /// D278: the transition mask of the tent's dissolve, decoded when the shell layer
    /// has a script world (`LevelMode::shell_scripted`); an error when it is not there.
    dissolve_mask: Option<Result<lbp_texture::Texture, String>>,
    /// D284: pod_mask, the card materials' second mask, decoded with it.
    pod_mask: Option<Result<lbp_texture::Texture, String>>,
    /// D332: the fog tint's palettes (`fog_tint`), and whether the level is the Pod's.
    fog_palettes: Result<fog_tint::FogPalettes, String>,
    pod_level: bool,
    /// D334: the water colours (`water::palette`).
    water_palette: Result<Vec<[f32; 3]>, String>,
}
/// D278: the mask of the Pod tent's dissolve (`pod_dissolve::TRANSITION_MASK`) as an
/// image, with a word on how the load went; there while the shell layer has a script
/// world. Without the image the band under the dissolve's line is not tinted.
#[derive(Resource)]
pub(crate) struct PodDissolveMask {
    pub(crate) image: Option<Handle<Image>>,
    pub(crate) note: String,
    /// D284: pod_mask (pod_dissolve::POD_MASK); without it no part of the tent stands
    /// higher and the platform's edge is cut whole at value 1.
    pub(crate) island: Option<Handle<Image>>,
    pub(crate) island_note: String,
}
/// D278: reads and decodes one texture by GUID (the recipe of the level scene's
/// runtime textures).
fn decode_texture(
    catalog: &Catalog,
    source: &AttachedSource,
    guid: u32,
    control: &LoadControl,
) -> Result<lbp_texture::Texture, String> {
    control.check()?;
    let key = lbp_preview::resolve(catalog, Some(guid), None)?;
    lbp_texture::decode_reader(
        key.size,
        |offset, count| {
            source
                .read_hex_checked(&key, offset, count, control)
                .map(|p| p.bytes)
                .map_err(lbp_texture::Error::Input)
        },
        || control.check().is_err(),
    )
    .map_err(|e| e.to_string())
}
/// D113 level mode (docs/d113-level-scene.md). Empty target = single-mesh mode.
#[derive(Default)]
struct LevelMode {
    guid: Option<u32>,
    path: Option<String>,
    /// D268 `--level-scripts`: the level's scripts run, so its scene gets a Thing map.
    scripts: bool,
    /// D120 POS placement (`--level-placement`; default POS x inverse bind of bone 0).
    placement: level_scene::PosPlacement,
    report: bool,
    /// D122 `--shape-bevels`: bevelled shape extrusion (inferred, unverified). Default off.
    shape_bevels: bool,
    /// P2m `--per-bone-placement`: models with boneThings placed per bone (default off).
    per_bone: bool,
    /// P2o `--no-per-bone-placement`: keep the bone-0 rule under --pod-game-camera.
    no_per_bone: bool,
    /// D147 `--level-focus-thing N`: frame the view on that Thing's item (QA captures).
    focus: Option<usize>,
    /// P2 `--pod-camera N`: home view placing Thing N (the Pod globe) where the RPCS3
    /// reference shows it (pod_menu::pod_camera).
    pod: Option<usize>,
    /// P2b `--pod-shell HEX`: the Pod shell level (000027b9) as a layer over the view.
    shell: Option<u32>,
    /// P2d `--pod-open` (1) / P2h `--pod-t T`: Puter opening progress to start at, for
    /// QA captures of the open and mid-dolly views.
    t0: Option<f32>,
    /// P2l `--lighting-preset N`: light from stored preset N instead of the main settings.
    lighting_preset: Option<usize>,
    /// P2m `--lighting-preset main`: the main block even with --lbp-lighting.
    lighting_main: bool,
    /// P2e `--runtime-texture MESH:SLOT:TEXTURE` (repeatable; docs/p2e-runtime-texture.md).
    runtime_textures: Vec<lbp_preview::runtime_texture::RuntimeTexture>,
    /// P2f `--pod-shell-camera FX,FY,FZ,TX,TY,TZ`: the closed shell eye from the game's
    /// camera preset (shell level units, up +Y) instead of the DualShock fit.
    shell_camera: Option<[f32; 6]>,
    /// P2k `--pod-game-camera`: the planets view through the game's own camera, World
    /// matrix and menu placement (docs/p2k-pod-game-camera.md) instead of the globe fit.
    /// `--pod-figure-data`: the Pod figure at the shell's data spawn, not the fit.
    figure_data: bool,
    game_camera: bool,
    lines: Vec<String>,
    home: Option<Orbit>,
}
impl LevelMode {
    /// P2o: per-bone placement is on with `--per-bone-placement`, and by default under
    /// `--pod-game-camera`, where D185/D203 confirmed it for the Pod Earth
    /// (`--no-per-bone-placement` turns it off there). D317: by default where the
    /// planets' own scripts place them (`pod_scripted`) too. The per-bone mesh is baked
    /// from the stored places of the bone Things in the frame of the item's own Thing,
    /// and the Thing map moves it with that Thing (`thing_item`). MEASURED (probe
    /// `pod_bone_things`): the Earth's bone Things 2 and 3 are children of Thing 1 and
    /// the scripts move all three by one translation, so the stored places relative to
    /// Thing 1 hold at every tick (D275 had per-bone off here because the per-bone
    /// model was left out of the Thing map and stayed at its stored place).
    fn per_bone_on(&self) -> bool {
        self.per_bone || (!self.no_per_bone && (self.game_camera || self.pod_scripted()))
    }
    /// D275: the Pod's planets level (`--pod-camera N`) under `--level-scripts`: its
    /// world script writes the camera and moves the world Thing, and the planets' own
    /// scripts place them. `--pod-game-camera`'s constants are not used then.
    fn pod_scripted(&self) -> bool {
        self.scripts && self.pod.is_some()
    }
    /// D277: the shell layer (`--pod-shell HEX`) has a script world of its own, ticked
    /// in one session with the planets level's (`level_play`): its scene loads the
    /// meshes the scripts swap and its Things' models are mapped.
    fn shell_scripted(&self) -> bool {
        self.pod_scripted() && self.shell.is_some()
    }
}
impl LevelMode {
    fn active(&self) -> bool {
        self.guid.is_some() || self.path.is_some()
    }
    /// Level selector: point at `entry` and drop Pod/QA framing from the previous target.
    fn retarget(&mut self, entry: &lbp_preview::level_list::LevelEntry) {
        self.guid = entry.guid;
        self.path = Some(entry.path.clone());
        self.focus = None;
        self.pod = None;
        self.shell = None;
        self.t0 = None;
        self.shell_camera = None;
        self.game_camera = false;
        self.lines.clear();
        self.home = None;
    }
    fn label(&self) -> String {
        match (&self.path, self.guid) {
            (Some(p), _) => p.clone(),
            (None, Some(g)) => format!("{g:08x}"),
            (None, None) => String::new(),
        }
    }
}
struct Job {
    started: std::time::Instant,
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
    /// P2q `--bump-gloss` / `--no-bump-gloss` (docs/p2q-bump-gloss.md).
    bump_gloss: bool,
    no_bump_gloss: bool,
    /// P3g `--no-lbp-shadows`: LBP lighting without the sun-shadow map.
    no_shadows: bool,
    instance_input_enabled: bool,
    instance_rgba: [f32; 4],
    animation_base: bool,
    /// D188 standalone defaults (single-mesh preview only; levels keep explicit
    /// inputs): white instance colour and animation base values unless opted out.
    instance_default: bool,
    animation_base_default: bool,
    animation_guid: String,
    /// P2d `--pod-hold-animation HEX`.
    hold_animation_guid: String,
    animation_frame: Option<i64>,
    cpu_skinning: bool,
    /// D148: single-mesh model space is Z-up (default) or Y-up (`--model-up`).
    model_z_up: bool,
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
    /// `--qa-hide-ui`: draw no egui panels (clean level/asset captures).
    hide_ui: bool,
    /// `--window-16-9`: a 1280x720 window (the game's frame aspect) for RPCS3 comparisons.
    window_16_9: bool,
    browser: browser::Browser,
    selector: level_selector::LevelSelector,
    census: CorpusCounts,
    level: LevelMode,
    /// D121 `--preview-lighting` (docs/d121-preview-lighting.md); default off.
    lighting: lighting::LightingState,
}
/// P3h `--no-glow`: draw material glow (port 4) nowhere, for A/B captures.
static NO_GLOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// P3k `--no-baked-ao`: backdrop lightmaps stay unbound.
static NO_BAKED_AO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
impl Viewer {
    fn preview_options(&self) -> lbp_preview::PreviewOptions {
        lbp_preview::PreviewOptions {
            animation_base: self.animation_base,
            instance_color: self
                .instance_input_enabled
                .then_some(lbp_preview::DiffuseTint {
                    bits: self.instance_rgba.map(f32::to_bits),
                }),
            surface: self.bump_gloss_on(),
        }
    }
    /// D188: single-mesh options with the standalone defaults filled in.
    fn standalone_options(&self) -> (lbp_preview::PreviewOptions, lbp_preview::AppliedDefaults) {
        lbp_preview::StandaloneDefaults {
            instance_white: self.instance_default,
            animation_base: self.animation_base_default,
        }
        .apply(self.preview_options())
    }
    /// D191: the same defaults in level scenes (per-Thing editorColor instead of white);
    /// `--no-instance-default` / `--no-animation-base` turn them off there too.
    fn level_defaults(&self) -> lbp_preview::StandaloneDefaults {
        lbp_preview::StandaloneDefaults {
            instance_white: self.instance_default,
            animation_base: self.animation_base_default,
        }
    }
    /// P2q: `--bump-gloss`, or on with `--lbp-lighting` unless `--no-bump-gloss`.
    fn bump_gloss_on(&self) -> bool {
        !self.no_bump_gloss && (self.bump_gloss || self.lighting.lbp)
    }
    /// Level selector click: load that level in place, dropping any Pod/QA framing that
    /// belonged to the previous target.
    fn select_level(&mut self, entry: &lbp_preview::level_list::LevelEntry) {
        self.level.retarget(entry);
        self.requested = true;
    }
    fn start(&mut self) {
        self.browser.displayed_guid = None;
        self.job = None;
        self.status = "Loading snapshot / validating read-only source / preparing preview…".into();
        self.ready_frames = 0;
        let paths = (self.snapshot.clone(), self.game.clone());
        if paths != self.context_paths {
            self.context = None;
            self.browser.clear_source();
            self.selector.clear();
        }
        let cached = (paths == self.context_paths)
            .then(|| self.context.clone())
            .flatten();
        // Level mode ignores the single-mesh GUID, except as the Pod figure (P2c) or the
        // play figure (--play-sackboy).
        let guid = if self.level.active() && self.level.shell.is_none() && !sackboy_play::enabled()
        {
            String::new()
        } else {
            self.guid.clone()
        };
        let level_target = self
            .level
            .active()
            .then(|| (self.level.guid, self.level.path.clone()));
        let shell_guid = self.level.shell;
        let scene_options = level_scene::LevelSceneOptions {
            shape_bevels: self.level.shape_bevels,
            placement: self.level.placement,
            runtime_textures: self.level.runtime_textures.clone(),
            per_bone: self.level.per_bone_on(),
            defaults: self.level_defaults(),
            // D276: the level a script world plays also loads the meshes its scripts
            // swap in. Without `--level-scripts` there are none.
            mesh_alternates: swapped_meshes(self.level.scripts),
        };
        // D277: the shell layer loads the swapped meshes when it has a script world
        // (`--level-scripts --pod-camera N`); else it is loaded as without scripts.
        let shell_options = level_scene::LevelSceneOptions {
            mesh_alternates: swapped_meshes(self.level.shell_scripted()),
            ..scene_options.clone()
        };
        let dissolve_mask_wanted = self.level.shell_scripted();
        let options = self.preview_options();
        let (mesh_options, _) = self.standalone_options();
        let animation_guid = self.animation_guid.trim().to_ascii_lowercase();
        let hold_guid = self.hold_animation_guid.trim().to_ascii_lowercase();
        self.playing = None;
        let control = LoadControl::default();
        let worker = control.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        // Large reserved stack: the level reader recurses natively (D107).
        let spawned = std::thread::Builder::new()
            .stack_size(level_scene::LEVEL_STACK_BYTES)
            .spawn(move || {
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
                            &source,
                            &catalog,
                            &key,
                            &worker,
                            mesh_options,
                        )
                        .map(Some)
                    })();
                    let level = level_target.map(|(level_guid, level_path)| {
                        let (key, identity) = level_scene::resolve_level(
                            &catalog,
                            level_guid,
                            level_path.as_deref(),
                        )?;
                        let scene = level_scene::load_level_scene_with_options(
                            &source,
                            &catalog,
                            &key,
                            &worker,
                            options,
                            scene_options.clone(),
                        )?;
                        let shell = shell_guid
                            .map(|g| {
                                let (key, _) = level_scene::resolve_level(&catalog, Some(g), None)?;
                                let shell = level_scene::load_level_scene_with_options(
                                    &source,
                                    &catalog,
                                    &key,
                                    &worker,
                                    options,
                                    shell_options,
                                )?;
                                let feet = sackboy_play::level_spawn_feet(
                                    &source, &catalog, &key, &worker,
                                );
                                Ok::<_, String>((shell, feet))
                            })
                            .transpose()?;
                        let (shell, shell_feet) = shell.unzip();
                        // D278: a mask that is not there leaves the dissolve untinted.
                        let dissolve_mask = dissolve_mask_wanted.then(|| {
                            let guid = pod_dissolve::TRANSITION_MASK;
                            decode_texture(&catalog, &source, guid, &worker)
                        });
                        let pod_mask = dissolve_mask_wanted.then(|| {
                            let guid = pod_dissolve::POD_MASK;
                            decode_texture(&catalog, &source, guid, &worker)
                        });
                        let fog_palettes = fog_tint::load_palettes(&source, &catalog, &worker);
                        let water_palette =
                            decode_texture(&catalog, &source, water::WATER_COLOURS, &worker)
                                .map(|t| water::palette(&t));
                        Ok(LevelLoaded {
                            scene,
                            identity,
                            shell,
                            shell_feet,
                            dissolve_mask,
                            pod_mask,
                            fog_palettes,
                            pod_level: level_guid == Some(fog_tint::POD_LEVEL_GUID),
                            water_palette,
                        })
                    });
                    let load_clip = |guid: &str| {
                        (preview.as_ref().is_ok_and(Option::is_some) && !guid.is_empty()).then(
                            || {
                                let number = u32::from_str_radix(guid, 16)
                                    .ok()
                                    .filter(|_| guid.len() == 8)
                                    .ok_or("Animation GUID must be eight hexadecimal digits")?;
                                let key =
                                    lbp_preview::skinned::resolve_animation(&catalog, number)?;
                                let mesh_key = mesh_key.as_ref().ok_or("No mesh key")?;
                                lbp_preview::skinned::load_clip(&source, mesh_key, &key, &worker)
                            },
                        )
                    };
                    // --play-sackboy without --play-animation: Sackboy's own clip bank
                    // (one rig, every clip the picker uses).
                    let bank_mesh = mesh_key
                        .as_ref()
                        .filter(|_| sackboy_play::enabled() && animation_guid.is_empty());
                    let (clip, play_bank) = match bank_mesh {
                        Some(mesh_key) if preview.as_ref().is_ok_and(Option::is_some) => {
                            match sackboy_play::load_bank(&source, &catalog, mesh_key, &worker) {
                                Ok((clip, bank)) => (Some(Ok(clip)), Some(bank)),
                                Err(e) => (Some(Err(e)), None),
                            }
                        }
                        _ => (load_clip(&animation_guid), None),
                    };
                    let hold_clip = load_clip(&hold_guid);
                    worker.check()?;
                    Ok(Completed {
                        context: (catalog, source),
                        preview,
                        identity,
                        guid,
                        level,
                        clip,
                        hold_clip,
                        play_bank,
                    })
                };
                let _ = tx.send(run());
            });
        if let Err(e) = spawned {
            self.status = format!("Load failed: worker thread: {e}");
            return;
        }
        self.context_paths = (self.snapshot.clone(), self.game.clone());
        self.job = Some(Job {
            started: std::time::Instant::now(),
            control,
            receiver: Mutex::new(rx),
        });
    }
}
#[derive(Resource, Clone, Copy)]
struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
}
/// D148 single-mesh home view: from the front (+Z), camera height 2 x sin(0.15) = 0.3
/// display units above the centre (about eye height of a unit-tall upright figure), so
/// it looks slightly down at the subject instead of from steeply above or below.
impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.,
            pitch: 0.15,
            distance: 2.,
            target: Vec3::ZERO,
        }
    }
}
/// D148: game meshes are authored Z-up with the front toward -Y (sack_boy.mol: head bone
/// z 108, toes z 0, eye bones y -34). Rotating -90 degrees about X maps (x, y, z) to
/// (x, z, -y): Bevy Y-up, front toward the +Z camera. Proper rotation, winding unchanged.
fn model_up(z_up: bool) -> Quat {
    if z_up {
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else {
        Quat::IDENTITY
    }
}
fn options() -> Result<Viewer, String> {
    options_from(std::env::args().skip(1))
}
/// The command line without the program name (D286: separate so tests can read options).
fn options_from(mut args: impl Iterator<Item = String>) -> Result<Viewer, String> {
    let mut v = Viewer {
        bump_gloss: false,
        no_bump_gloss: false,
        no_shadows: false,
        instance_input_enabled: false,
        instance_rgba: [1.; 4],
        animation_base: false,
        instance_default: true,
        animation_base_default: true,
        animation_guid: String::new(),
        hold_animation_guid: String::new(),
        animation_frame: None,
        cpu_skinning: false,
        model_z_up: true,
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
        hide_ui: false,
        window_16_9: false,
        browser: browser::Browser::default(),
        selector: level_selector::LevelSelector::default(),
        census: CorpusCounts::default(),
        level: LevelMode::default(),
        lighting: lighting::LightingState::default(),
    };
    let mut print_census = false;
    let mut play_sackboy = false;
    let mut no_lbp_lighting = false;
    // D300: `Some(true)` under `--final-pass`, `Some(false)` under `--no-final-pass`.
    let mut final_pass: Option<bool> = None;
    // D304: likewise `--depth-of-field` / `--no-depth-of-field` and `--mlaa` / `--no-mlaa`
    // (both on by default in the Pod picture).
    let mut depth_of_field: Option<bool> = None;
    let mut mlaa: Option<bool> = None;
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
        if flag == "--no-animation-base" {
            v.animation_base_default = false;
            continue;
        }
        if flag == "--no-instance-default" {
            v.instance_default = false;
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
        if flag == "--preview-lighting" {
            v.lighting.enabled = true;
            continue;
        }
        if flag == "--lbp-lighting" {
            v.lighting.enabled = true;
            v.lighting.lbp = true;
            continue;
        }
        if flag == "--no-lbp-lighting" {
            no_lbp_lighting = true;
            continue;
        }
        if flag == "--no-glow" {
            NO_GLOW.store(true, std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        if flag == "--no-edge-glow" {
            glow_post::disable();
            continue;
        }
        if flag == "--final-pass" {
            final_pass = Some(true);
            continue;
        }
        if flag == "--no-final-pass" {
            final_pass = Some(false);
            continue;
        }
        if flag == "--depth-of-field" {
            depth_of_field = Some(true);
            continue;
        }
        if flag == "--no-depth-of-field" {
            depth_of_field = Some(false);
            continue;
        }
        if flag == "--mlaa" {
            mlaa = Some(true);
            continue;
        }
        if flag == "--no-mlaa" {
            mlaa = Some(false);
            continue;
        }
        if flag == "--no-baked-ao" {
            NO_BAKED_AO.store(true, std::sync::atomic::Ordering::Relaxed);
            continue;
        }
        if flag == "--no-lbp-shadows" {
            v.no_shadows = true;
            continue;
        }
        if flag == "--bump-gloss" {
            v.bump_gloss = true;
            continue;
        }
        if flag == "--no-bump-gloss" {
            v.no_bump_gloss = true;
            continue;
        }
        // D286: the side panels are glass and take no texture by default any more; the
        // flag is still accepted so older command lines run unchanged.
        if flag == "--no-pod-wall" {
            continue;
        }
        if flag == "--pod-game-camera" {
            v.level.game_camera = true;
            continue;
        }
        if flag == "--pod-figure-data" {
            v.level.figure_data = true;
            continue;
        }
        if flag == "--pod-open" {
            v.level.t0 = Some(1.);
            continue;
        }
        if flag == "--window-16-9" {
            v.window_16_9 = true;
            continue;
        }
        if flag == "--qa-hide-ui" {
            v.hide_ui = true;
            continue;
        }
        if flag == "--no-activate" {
            // Read by activate_on_show().
            continue;
        }
        if flag == "--level-report" {
            v.level.report = true;
            continue;
        }
        if flag == "--popit-open" || flag == "--popit-cursor" {
            // Read by popit_play.rs.
            continue;
        }
        if flag == "--level-scripts" {
            // Read by level_play.rs; here for the Thing map of the level scene (D268).
            v.level.scripts = true;
            continue;
        }
        if flag == "--play-sackboy" {
            // Read by sackboy_play.rs; here only for the default figure mesh.
            play_sackboy = true;
            continue;
        }
        if flag == "--no-pod-sackboy" {
            // D320: read by sackboy_play.rs (`pod_shell`).
            continue;
        }
        if flag == "--shape-bevels" {
            v.level.shape_bevels = true;
            continue;
        }
        if flag == "--per-bone-placement" {
            v.level.per_bone = true;
            continue;
        }
        if flag == "--no-per-bone-placement" {
            v.level.no_per_bone = true;
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
            println!("lbp-bevy-viewer --snapshot FILE --game-dir DIRECTORY [--select-guid HEX]\n--play-animation HEX: play an animation on the selected mesh\n--animation-frame N: hold that running frame instead of playing (QA captures)\n--skinning gpu|cpu: Bevy joint skinning (default) or the CPU reference path\n--model-up z|y: single-mesh model up axis (default z: Z-up, front -Y, shown upright facing the camera; y: raw axes)\n--browse-query TEXT; --show-unavailable: browser filters\n--animation-base: local animation base-row snapshot (not game time; also levels)\n--no-animation-base, --no-instance-default: turn off the D188 single-mesh defaults (animation base values; white instance color 1,1,1,1 when no Thing supplies one) and the D191 level defaults (animation base values; each Thing's editorColor as its instance color, inferred)\n--instance-rgba R,G,B,A: supplied linear instance color (overrides the white default)\n--synthetic: authored depth/UV/repeat/clamp scene\n--synthetic-groups: authored mixed material groups\n--synthetic-affine: authored UV scale/offset/reflection\n--qa-capture NEW_PNG_UNDER_LOCAL_ANALYSIS: capture then exit (window shown without taking focus)
--no-activate: show the window without taking focus or the foreground (interactive runs)
--qa-hide-ui: draw no panels (clean captures)\n--window-16-9: 1280x720 window (the game's 16:9 frame; RPCS3 comparisons)\n--preview-lighting: key + ambient preview lighting (not game lighting; levels use their stored settings, approximate)\n--lighting-preset N|main: with a level, light from its stored preset N instead of the main settings (P2l; --lbp-lighting defaults to preset 0 when one is stored, main forces the main block)\n--lbp-lighting: preview lighting shaped like the community brdf.cg reconstruction (hemisphere ambient, sun, two rims, squared combine; unfitted stored values, uniform mapping inferred, P2l); on by default in level mode (P2w), --no-lbp-lighting draws levels unlit, --preview-lighting keeps the older Lambert preview\n--no-glow: skip material glow (output port 4; P3h A/B)\n--no-edge-glow: skip the glow post pass on the Pod dissolve's line (D292 A/B)
--final-pass / --no-final-pass: with a level on the level lighting, the game's final pass over the frame, x1.2998 and its vignette mixed back by the scene's alpha: 0 in the Pod picture (--pod-camera), the scene's own elsewhere (D294, D300; on by default)\n--depth-of-field / --no-depth-of-field: with a level on the level lighting, the game's depth of field over the scripts' Cam.SetDOFAndFog range (D298; on by default in the Pod picture, --pod-camera, off by default in other levels, D304)\n--mlaa / --no-mlaa: in the Pod picture (--pod-camera) on the level lighting, the game's anti-aliasing job (EDGE MLAA: edge thresholds 10 and 0.348 of the brighter pixel, Reshetov blend) after the depth of field, without MSAA (D302; on by default there, D304)\n--no-baked-ao: skip the backdrop lightmap baked AO under --lbp-lighting (P3k A/B)\n--bump-gloss / --no-bump-gloss: material bump maps (output port 3) and static gloss colours (port 2) under --lbp-lighting, on with it by default (P2q; offsets and frame inferred)\n{CENSUS_FLAGS_HELP}\n--print-census: print the corpus-count panel text and exit (no window)\n--ui-demo: authored UI layer demo (no game data; other options ignored)\n--slot-browser: story slot browser over --snapshot/--game-dir (--slot-browser-report prints counts and exits; --slot-browser-slots adds one line per slot)\n--text-browser: .trans tables (search by text, tag or key) and font glyph sheets (--text-browser-report prints counts and exits)\n--ui-script CLASS: run one of your UI scripts headlessly (stub natives) and draw the frame tree it builds; --ui-script-function NAME (default UpdateUI__Q5Gooey), --ui-script-report prints counts and the outline (--qa-hide-ui draws the tree without the count lines)\n--pod-menu CLASS: the Pod main menu as pills on a globe and moon (placeholder spheres; pill positions from the RPCS3 reference), Up/Down/Enter; with a level, the pills over the level view\n--pod-camera N: with a level, home view that puts Thing N (the Pod globe) where the RPCS3 frame has it\n--pod-shell HEX: with --pod-camera, the Pod shell level (000027b9) as a layer over the view; Space or gamepad Square opens/closes the Puter (camera dolly), Esc or Circle closes; --pod-game-camera: the planets view through the game's own camera (presets, vfov 57.3 deg), World transform and main-menu Earth/Moon placement instead of the globe fit (P2k);\n--pod-open: start open; the side panels are clear glass (D286; --runtime-texture 00002f6c:0:HEX paints a texture on them, --no-pod-wall is accepted and does nothing); --pod-t T: start at opening progress T (0-1, mid-dolly captures); --pod-shell-camera FX,FY,FZ,TX,TY,TZ: closed shell eye from the game camera preset (shell level units); --select-guid HEX (Sackboy 0000043f) stands in front of the DualShock, --play-animation HEX plays on him, --pod-hold-animation HEX while the Puter is open; --pod-figure-data: closed spot from the shell's data (checkpoint + spawn offset, dropped to the floor) instead of the RPCS3 fit (both are reported)\n--puter-screen CLASS: in the 3D view, that UI script's frame tree on a screen quad (arrows/Enter focus); --puter-function NAME, --puter-at X,Y,Z[,WIDTH] (display units, facing +Z; default 1.1,0,0,1.2)\n--popit: inventory browser over the disc plans (--popit-report prints counts; add --popit-decode-icons to measure icon decoding)\n--level-guid HEX | --level-path MAP_PATH: level scene mode (RENDER_MESH Things at POS)\n--level-placement inverse-bind|pos-only: POS Things at POS x inverse bind of bone 0 (default, D120) or POS only (D113)\n--shape-bevels: with a level, bevelled shapes (geometry inferred, unverified; default straight)\n--per-bone-placement: with a level, models with boneThings place each vertex by its bone's Thing (P2m; inferred rigid skinning; default bone 0 rule, but on under --pod-game-camera and in the scripted Pod, --level-scripts with --pod-camera N, unless --no-per-bone-placement)\n--runtime-texture MESH:SLOT:TEXTURE (hex GUIDs, slot 0-7, repeatable): with a level (and --pod-shell), draw Solid groups of that mesh whose material lists SLOT as unrendered with TEXTURE (UV0, Repeat, untinted; inferred, game binding unconfirmed)\n--level-report: with a level, print scene counts and exit (no window)\n--level-focus-thing N: with a level, frame the view on Thing N (reader order)\n--level-scripts: with a level, run its scripts at 30 Hz (OnCreate__, then Update__ each tick) and show the counts, with the open speech bubbles and the subtitle drawn as plain boxes the level lit again when a script changes the global lighting or darkness, and each Thing's own model hidden, tinted and moved as the scripts set it (a mesh that is not visible in play mode is hidden), and the camera moved to a script's override camera while one is on, or put at the camera a script writes itself (the Pod's planets level: with --pod-camera N and no --pod-shell the level also hangs under its script-moved world Thing); --level-scripts-flags N,..: progression flags set on the profile; --level-scripts-hold-at N (QA): the scripts stop after tick N, so a capture shows that tick; --level-scripts-in-use-at N (QA): the pod computer is in use once N ticks have run (a stand-in for the player engaging it), so the Pod level's change-over runs and its pod mesh is swapped; with --pod-camera N and --pod-shell HEX the shell level is played in the same session with a Sackboy (not drawn, a placeholder) on the pod computer's trigger, and Space or gamepad Square is the engage button on the pad the scripts read for one tick (the pod's own script then asks for the pod computer, once the intro has set its progression flags), or while the computer is in use a request to the host to close it, as Esc or Circle (placeholder: no script path for closing was found); --level-scripts-press-at N (QA): that engage press on tick N without a key; the scripted Pod has the viewer's own Sackboy (the play mode in pod.bin) with the pod computer's clips, --no-pod-sackboy leaves him out\n--play-sackboy: with a level, its shapes as a physics world and Sackboy at the first checkpoint, drawn as his mesh (0000043f unless --select-guid) with stand/walk/lane/jump/land clips picked from his state (rules inferred; --play-animation HEX plays one clip instead) (arrows walk and change lanes, Space jumps, I/K/L/J moods happy/sad/angry/scared; camera follows); --play-sackboy-at X,Y,Z spawn; --play-sackboy-input r:40,rj:12,-:30 scripted keys (l r j b f, mood presses U D R L) with the position printed each tick; --qa-record DIR,START,END[,SPEED] (QA only) saves one PNG per scripted tick START..END into a new local-analysis DIR, game time at SPEED (default 0.25), then exits\nLeft drag orbit; right/middle drag pan; wheel zoom; R reset; F frame.");
            std::process::exit(0);
        }
        let value = args.next().ok_or("Missing option value")?;
        match flag.as_str() {
            "--snapshot" => v.snapshot = value,
            "--game-dir" => v.game = value,
            "--select-guid" => v.guid = value,
            "--play-animation" => v.animation_guid = value,
            "--pod-hold-animation" => v.hold_animation_guid = value,
            "--animation-frame" => {
                v.animation_frame = Some(value.parse().map_err(|_| "invalid animation frame")?)
            }
            "--skinning" => {
                v.cpu_skinning = match value.as_str() {
                    "cpu" => true,
                    "gpu" => false,
                    _ => return Err("--skinning takes cpu or gpu".into()),
                }
            }
            "--model-up" => {
                v.model_z_up = match value.as_str() {
                    "z" => true,
                    "y" => false,
                    _ => return Err("--model-up takes z or y".into()),
                }
            }
            "--browse-query" => v.browser.query = value,
            "--level-guid" => {
                if value.len() != 8 {
                    return Err("--level-guid requires eight hexadecimal digits".into());
                }
                v.level.guid = Some(
                    u32::from_str_radix(&value, 16)
                        .map_err(|_| "--level-guid requires eight hexadecimal digits")?,
                );
            }
            "--level-path" => v.level.path = Some(value),
            "--level-placement" => v.level.placement = level_scene::PosPlacement::parse(&value)?,
            "--runtime-texture" => push_runtime_texture(&mut v.level.runtime_textures, &value)?,
            "--lighting-preset" => {
                if value == "main" {
                    v.level.lighting_main = true;
                } else {
                    let n = value
                        .parse::<usize>()
                        .ok()
                        .filter(|&n| n < lbp_level::parts::MAX_KEPT_PRESETS)
                        .ok_or("--lighting-preset takes a preset index (0 = first) or main")?;
                    v.level.lighting_preset = Some(n);
                }
            }
            "--pod-t" => {
                let t = value
                    .parse::<f32>()
                    .ok()
                    .filter(|t| (0. ..=1.).contains(t))
                    .ok_or("--pod-t takes a number from 0 (closed) to 1 (open)")?;
                v.level.t0 = Some(t);
            }
            "--pod-shell-camera" => {
                let n: Vec<f32> = value
                    .split(',')
                    .map(|x| x.trim().parse::<f32>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| "--pod-shell-camera takes six numbers FX,FY,FZ,TX,TY,TZ")?;
                let n: [f32; 6] = n
                    .try_into()
                    .ok()
                    .filter(|n: &[f32; 6]| n.iter().all(|x| x.is_finite()))
                    .ok_or("--pod-shell-camera takes six finite numbers FX,FY,FZ,TX,TY,TZ")?;
                if n[..3] == n[3..] {
                    return Err("--pod-shell-camera: from and to are the same point".into());
                }
                v.level.shell_camera = Some(n);
            }
            "--pod-shell" => {
                v.level.shell = Some(
                    u32::from_str_radix(&value, 16)
                        .ok()
                        .filter(|_| value.len() == 8)
                        .ok_or("--pod-shell requires eight hexadecimal digits")?,
                )
            }
            "--pod-camera" => {
                v.level.pod = Some(value.parse().map_err(|_| "invalid --pod-camera")?)
            }
            "--level-focus-thing" => {
                v.level.focus = Some(value.parse().map_err(|_| "invalid --level-focus-thing")?)
            }
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
            // Read by puter.rs, pod_menu.rs, level_play.rs and sackboy_play.rs.
            "--puter-screen"
            | "--puter-function"
            | "--puter-at"
            | "--pod-menu"
            | "--ui-script-function"
            | "--level-scripts-flags"
            | "--level-scripts-hold-at"
            | "--level-scripts-in-use-at"
            | "--level-scripts-press-at"
            | "--level-scripts-choose-at"
            | "--pod-card-pins"
            | "--play-sackboy-at"
            | "--popit-mode"
            | "--popit-page"
            | "--play-sackboy-input"
            | "--qa-record" => {}
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
    if v.level.active() && v.synthetic {
        return Err("Level mode cannot be combined with --synthetic".into());
    }
    // P2w: level scenes are lit like the game by default (RSX-fitted model, docs/p2w-level-lighting-default.md);
    // --preview-lighting keeps the older Lambert preview and --no-lbp-lighting draws unlit.
    if v.level.active() && !v.lighting.enabled && !no_lbp_lighting {
        v.lighting.enabled = true;
        v.lighting.lbp = true;
    }
    // D294: the game's final pass (x1.2998 and the vignette) on the level-lighting route
    // only: that route is fitted to the game's scene target, before its final pass.
    // D300: on by default there, mixing back by the scene's alpha as the game does: alpha 0
    // in the Pod picture (the anti-aliasing job's copy back), the scene's own elsewhere (1 on
    // opaque surfaces: no change); `--no-final-pass` turns it off.
    let level_route = v.level.active() && v.lighting.lbp;
    let (on, cleared) = glow_post::final_pass_route(level_route, final_pass, v.level.pod.is_some());
    if on {
        glow_post::enable_final_pass();
    }
    if cleared {
        glow_post::clear_scene_alpha();
    }
    if !level_route && final_pass == Some(true) {
        eprintln!("--final-pass: ignored (only on the level-lighting route, a level without --preview-lighting or --no-lbp-lighting)");
    }
    // D298: the game's depth of field, on the same route (it runs before the final pass).
    // D304: on by default in the Pod picture, off elsewhere unless `--depth-of-field`;
    // `--no-depth-of-field` turns it off.
    let pod_picture = v.level.pod.is_some();
    if dof_post::dof_route(level_route, pod_picture, depth_of_field) {
        dof_post::enable();
    } else if depth_of_field == Some(true) {
        eprintln!("--depth-of-field: ignored (only on the level-lighting route, a level without --preview-lighting or --no-lbp-lighting)");
    }
    // D302: the game's anti-aliasing job, where the game runs it: the Pod picture.
    // D304: on by default there; `--no-mlaa` turns it off.
    if mlaa_post::mlaa_route(level_route, pod_picture, mlaa) {
        mlaa_post::enable();
    } else if mlaa == Some(true) {
        eprintln!(
            "--mlaa: ignored (only in the Pod picture, --pod-camera, on the level-lighting route)"
        );
    }
    if v.level.guid.is_some() && v.level.path.is_some() {
        return Err("Give --level-guid or --level-path, not both".into());
    }
    // --play-sackboy draws Sackboy's own body mesh unless --select-guid names another
    // (0000043f sack_boy.mol; the default naked costume adds no geometry, INFERRED).
    // D320: so does the scripted Pod's Sackboy (`sackboy_play::in_pod`).
    if (play_sackboy || sackboy_play::in_pod()) && v.level.active() && v.guid.trim().is_empty() {
        v.guid = sackboy_play::SACKBOY_MESH.into();
    }
    if v.level.report && !v.level.active() {
        return Err("--level-report requires --level-guid or --level-path".into());
    }
    if v.level.shape_bevels && !v.level.active() {
        return Err("--shape-bevels requires --level-guid or --level-path".into());
    }
    if !v.level.runtime_textures.is_empty() && !v.level.active() {
        return Err("--runtime-texture requires --level-guid or --level-path".into());
    }
    if !v.synthetic && (v.snapshot.is_empty() || v.game.is_empty()) {
        return Err(
            "Provide --snapshot, --game-dir or --synthetic (--select-guid is optional)".into(),
        );
    }
    Ok(v)
}
fn main() {
    if std::env::args().skip(1).any(|a| a == "--ui-demo") {
        gooey_ui::run_ui_demo();
        return;
    }
    if std::env::args().skip(1).any(|a| a == "--popit") {
        if let Err(e) = popit_browser::run_popit() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().skip(1).any(|a| a == "--text-browser") {
        if let Err(e) = text_browser::run_text_browser() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    let level_given = std::env::args()
        .skip(1)
        .any(|a| a == "--level-guid" || a == "--level-path");
    if !level_given && std::env::args().skip(1).any(|a| a == "--pod-menu") {
        if let Err(e) = pod_menu::run_pod_menu() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().skip(1).any(|a| a == "--ui-script") {
        if let Err(e) = ui_script::run_ui_script() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().skip(1).any(|a| a == "--slot-browser") {
        if let Err(e) = slot_browser::run_slot_browser() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    let viewer = options().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    if viewer.level.report {
        std::process::exit(match level_report(&viewer) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
    let resolution = if viewer.window_16_9 {
        (1280, 720)
    } else {
        (1280, 800)
    };
    let mut app = App::new();
    app.insert_resource(viewer)
        .insert_resource(bevy::light::DirectionalLightShadowMap {
            size: SHADOW_MAP_SIZE,
        })
        .init_resource::<Orbit>()
        .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.045)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "craftworld-rs".into(),
                resolution: resolution.into(),
                visible: false,
                focused: activate_on_show(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(dual::DualPlugin)
        .add_plugins((glow_post::GlowPostPlugin, colour_grade::ColourGradePlugin))
        .add_plugins(dof_post::DofPostPlugin)
        .add_plugins(mlaa_post::MlaaPostPlugin)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                place_on_second_monitor,
                clear_pod_shell,
                load,
                animate,
                apply_lighting,
                pod_dolly,
                pod_figure,
                camera,
                capture,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, panel);
    if let Err(e) = puter::install(&mut app)
        .and_then(|()| pod_menu::install_overlay(&mut app))
        .and_then(|()| level_play::install(&mut app))
        .and_then(|()| sackboy_play::install(&mut app))
    {
        eprintln!("{e}");
        std::process::exit(1);
    }
    app.run();
}
/// `Window::focused` for a launch window: false under `--qa-capture` or `--no-activate`.
/// Bevy passes it to winit's `with_active`, and on Windows winit then shows the hidden
/// window with `SW_SHOWNOACTIVATE`, so it never takes focus or the foreground.
pub(crate) fn activate_on_show() -> bool {
    activate_on_show_for(std::env::args().skip(1))
}
fn activate_on_show_for(mut args: impl Iterator<Item = String>) -> bool {
    !args.any(|a| a == "--qa-capture" || a == "--no-activate" || a == "--qa-record")
}
// Keep test/launcher windows hidden until positioned on a non-primary display.
pub(crate) fn place_on_second_monitor(
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
    // D298: the depth of field reads the depth buffer, so it is made readable under the flag.
    // D312: and copied between the opaque and the transparent pass (`dof_post.rs`).
    let mut camera = Camera3d::default();
    if dof_post::enabled() {
        camera.depth_texture_usages =
            (bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT
                | bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
                | bevy::render::render_resource::TextureUsages::COPY_SRC)
                .into();
    }
    let mut main = commands.spawn((
        camera,
        Projection::Perspective(PerspectiveProjection {
            near: 0.01,
            far: 100.,
            ..default()
        }),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::default(),
        // D292: the edge glow's chain runs on this camera while a dissolve is on (and, D294,
        // its final pass on every frame on the level-lighting route, D300).
        glow_post::EdgeGlow::default(),
        // D298: the scripts' depth-of-field range (`level_play::follow_dof`).
        dof_post::DepthOfField::default(),
        // D302: the anti-aliasing job (extracted only while it is on: the Pod picture, D304).
        mlaa_post::Mlaa,
    ));
    // D302: the game's scene target has one sample (MEASURED, frame 5247): while the job is on
    // it takes the place of the viewer's MSAA.
    if mlaa_post::enabled() {
        main.insert(bevy::render::view::Msaa::Off);
    }
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
        v.lighting.light = lighting::PreviewLight::generic();
        v.lighting.shell = None;
        v.lighting.level = None;
        v.lighting.dirty = true;
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
        .filter(|done| done.level.is_none())
        .map(|done| done.guid.trim().to_ascii_lowercase());
    let mut evidence = browser::PreviewEvidence::default();
    v.lighting.dirty = true;
    let rendered = (|| -> Result<(), String> {
        let done = result?;
        v.context = Some(done.context);
        if let Some(level) = done.level {
            let mut level = level?;
            // D275: under `--level-scripts` the planets level is placed by its scripts,
            // with or without the shell layer; the menu constants are left out.
            let scripted = v.level.pod_scripted();
            let game = v.level.game_camera && level.shell.is_some() && !scripted;
            let menu_line = match (game, v.level.pod) {
                (true, Some(globe)) => Some(place_menu_planets(&mut level.scene, globe)?),
                _ => None,
            };
            let mut followed = None;
            let pod_level = level.pod_level;
            // P2m: --lbp-lighting uses preset 0 unless told otherwise (D201: the main
            // block holds defaults in pod_planets; preset 0 matches RPCS3 far better).
            let preset = match (v.level.lighting_main, v.level.lighting_preset) {
                (true, _) => None,
                (false, Some(n)) => Some(n),
                // P3m: other levels follow their PWorld lighting (below).
                (false, None) => (v.level.pod.is_some() && v.lighting.lbp).then_some(0),
            };
            // P3b: the Pod's live light is the backdrop plan's preset (also for the plain
            // `--preview-lighting`, whose main block would otherwise be the wrong holder).
            let from_plan = v
                .level
                .pod
                .and(preset.or((!v.level.lighting_main).then_some(0)))
                .and_then(|n| level.scene.backdrop_presets.get(n).map(|p| (n, *p)));
            if let Some((n, p)) = from_plan {
                level.scene.settings = Some(p);
                if let Some(shell) = level.shell.as_mut() {
                    shell.settings = Some(p);
                }
                println!(
                    "Lighting preset {n} of {} from the backdrop plan (Pod live source)",
                    level.scene.backdrop_presets.len()
                );
            } else if let Some(n) = preset {
                println!("{}", use_lighting_preset(&mut level.scene, n));
                if let Some(shell) = level.shell.as_mut() {
                    println!("Shell: {}", use_lighting_preset(shell, n));
                }
            } else if v.level.pod.is_none()
                && !v.level.lighting_main
                && v.level.lighting_preset.is_none()
            {
                // P3m: the game lights a level with its stored presets blended by PWorld
                // `lighting`, dimmed by `darkness`; the main block is not what it uses.
                // D267: the blocks are kept, so `--level-scripts` can light the level
                // again when a script changes those two values.
                // D332: and the fog colour tinted by `fogTint` over the constants' palette.
                // D333: and the fog distances by `fog`.
                let [lighting, _, fog, fog_tint, darkness] =
                    level.scene.world_lighting.unwrap_or_default();
                let palette = match &level.fog_palettes {
                    Ok(p) => p.for_level(level.pod_level).to_vec(),
                    Err(e) => {
                        println!(
                            "Fog tint: palettes not read ({e}); the fog colour stays untinted"
                        );
                        Vec::new()
                    }
                };
                followed = level.scene.settings.map(|main| {
                    let presets = level.scene.settings_presets.clone();
                    (
                        main,
                        presets,
                        [lighting, darkness, fog_tint, fog],
                        palette.clone(),
                    )
                });
                println!(
                    "{}",
                    use_world_lighting(&mut level.scene, &palette, pod_level)
                );
            }
            let plan = plan_level(&level.scene, v.level.focus, v.level.pod)?;
            let light_frame = plan.light_frame;
            commands.insert_resource(sackboy_play::LevelDisplay(plan.normalize));
            let planets_normalize = plan.normalize;
            let rig = match (&level.shell, v.level.pod) {
                (Some(shell), Some(globe)) => Some(pod_shell(
                    &level.scene,
                    &plan,
                    globe,
                    shell,
                    v.level.shell_camera,
                    (game, scripted),
                )?),
                _ => None,
            };
            let planets_first = v.scene.len();
            // D272: the Pod's planets level under `--level-scripts` hangs under a root
            // that follows its world Thing (D275: with the shell layer too).
            let under_world_thing = scripted;
            // D268: the Pod places its planets by its own rules, so it is not followed.
            // D274: but under that root the planets' own scripts place them, and the
            // Thing map moves each one's model inside the root.
            let follow = v.level.scripts && (v.level.pod.is_none() || under_world_thing);
            let things = spawn_level(
                &mut v,
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                plan,
                follow,
            );
            if let Some(things) = things {
                commands.insert_resource(things);
            }
            if v.level.pod.is_none() {
                println!(
                    "{}",
                    spawn_water(
                        &mut v,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &level,
                        planets_normalize,
                    )
                );
            }
            v.lighting.level = followed.zip(light_frame).map(
                |((main, presets, stored, fog_palette), (centre, scale))| lighting::LevelRelight {
                    main,
                    presets,
                    centre,
                    scale,
                    stored,
                    fog_palette,
                    pod: pod_level,
                },
            );
            if game {
                // P2k: the planets level hangs under the World transform.
                let t0 = v.level.t0.unwrap_or(0.);
                let root = commands
                    .spawn((
                        planets_root(planets_normalize, t0),
                        Visibility::default(),
                        PodPlanetsRoot(planets_normalize),
                    ))
                    .id();
                for (entity, ..) in &v.scene[planets_first..] {
                    commands.entity(root).add_child(*entity);
                }
            } else if under_world_thing {
                // D272: the Pod's planets level under `--level-scripts`: the same root,
                // placed each tick from the world Thing the level's script moves
                // (`level_play::follow_world`).
                let root = commands
                    .spawn((
                        Transform::IDENTITY,
                        Visibility::default(),
                        PodPlanetsRoot(planets_normalize),
                        level_play::WorldThingRoot,
                    ))
                    .id();
                for (entity, ..) in &v.scene[planets_first..] {
                    commands.entity(root).add_child(*entity);
                }
            }
            *orbit = v.level.home.unwrap_or_default();
            if let Some((rig, shell_plan, rig_line)) = rig {
                let checkpoint = level
                    .shell
                    .as_ref()
                    .and_then(|s| s.checkpoints.first().copied());
                let normalize = shell_plan.normalize;
                // D277: under `--level-scripts` the opening follows pod.bin's script
                // (`level_play::tick`), so the layer starts closed; `--pod-open` is
                // the Puter state "open" and a `--pod-t` between has no meaning there.
                let asked = v.level.t0.unwrap_or(0.);
                let t0 = if scripted { 0. } else { asked };
                // D280: under `--level-scripts` the shell level is drawn in its own level
                // frame, by the camera the planets level's script writes: both roots are
                // that frame and stay there. The rig's fitted shell eyes place the roots
                // only without the scripts.
                let frame = scripted
                    .then(|| level_play::shell_frame_root(planets_normalize, normalize))
                    .flatten();
                let (shell_at, tent_at) = match frame {
                    Some(frame) => (frame, frame),
                    None => (rig.shell_root(t0), rig.tent_root(t0)),
                };
                let root = commands
                    .spawn((shell_at, Visibility::default(), PodShellRoot))
                    .id();
                // P2g: pod.mol (the tent) on its own root; the DualShock, cords and the
                // figure stay on the shell root.
                let tent = commands
                    .spawn((tent_at, Visibility::default(), PodShellRoot, PodTentRoot))
                    .id();
                let tent_flags: Vec<bool> = level.shell.as_ref().map_or(Vec::new(), |shell| {
                    shell_plan
                        .placed
                        .iter()
                        .flat_map(|(mesh, _)| {
                            let n = shell_plan.prepared[*mesh].as_ref().map_or(0, Vec::len);
                            let is_tent =
                                shell.meshes[*mesh].source.base_guid() == Some(POD_TENT_MESH);
                            std::iter::repeat_n(is_tent, n)
                        })
                        .collect()
                });
                let first = v.scene.len();
                let planets_light = v.lighting.light.clone();
                // D277: with a script world of its own the shell's Things are mapped,
                // in a map of their own beside the planets level's.
                let shell_things = spawn_level(
                    &mut v,
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut images,
                    shell_plan,
                    scripted,
                );
                if let Some(mut things) = shell_things {
                    // D278: the tent of the shell's pod Thing is drawn with the dissolve;
                    // D284: and the platform's card edge at value 1.
                    things.dissolve_mesh = Some(POD_TENT_MESH);
                    things.dissolve_edge =
                        Some((pod_dissolve::PLATFORM_MESH, pod_dissolve::CARD_EDGE));
                    commands.insert_resource(thing_view::ShellThings(things));
                }
                if let Some(mask) = level.dissolve_mask.take() {
                    let island = level.pod_mask.take();
                    let mask = dissolve_mask_image(mask, island, &mut images);
                    v.level.lines.push(format!(
                        "Pod tent dissolve mask {:08x}: {}; pod_mask {:08x}: {}",
                        pod_dissolve::TRANSITION_MASK,
                        mask.note,
                        pod_dissolve::POD_MASK,
                        mask.island_note
                    ));
                    commands.insert_resource(mask);
                }
                if scripted && asked > 0. && asked < 1. {
                    v.level.lines.push(format!(
                        "--pod-t {asked} is not used under --level-scripts: the opening follows pod.bin's script (mid stills: --level-scripts-in-use-at N --level-scripts-hold-at N+k, value 0.05 k)"
                    ));
                }
                // P2l: the shell keeps its own level's light; the planets keep theirs.
                let mut shell_light =
                    std::mem::replace(&mut v.lighting.light, planets_light.clone());
                if v.lighting.lbp {
                    // P2r: the RSX capture shows every lit Pod draw, shell included, with
                    // the planets' preset 0 constants; the shell keeps its own direction.
                    shell_light = lighting::PreviewLight {
                        toward_light: shell_light.toward_light,
                        display_scale: shell_light.display_scale,
                        ..planets_light
                    };
                }
                if frame.is_some() {
                    // D280: in the shared frame a display unit of the shell is the
                    // planets level's (the shell's own scale times the root's).
                    shell_light.display_scale = planets_light.display_scale;
                }
                v.lighting.shell = Some((first, shell_light));
                for (i, (entity, ..)) in v.scene[first..].iter().enumerate() {
                    let parent = if tent_flags.get(i).copied().unwrap_or(false) {
                        tent
                    } else {
                        root
                    };
                    commands.entity(parent).add_child(*entity);
                }
                // The shell plan's lines replace the level's; keep P2e rule lines for both.
                v.level.lines.extend(
                    lbp_preview::runtime_texture::lines(&level.scene.counts.runtime_textures)
                        .into_iter()
                        .map(|l| format!("Main level: {l}")),
                );
                // Home (R) stays the closed view.
                v.level.home = Some(pod_orbit(&rig, 0.));
                *orbit = pod_orbit(&rig, t0);
                commands.insert_resource(PodRigRes(rig, scripted));
                commands.insert_resource(pod_menu::PuterState {
                    t: t0,
                    open: asked >= 1.,
                    // Mid-dolly starts stay put (QA); Space then opens or closes.
                    hold: t0 > 0. && t0 < 1.,
                    key: None,
                });
                v.level.lines.push(rig_line);
                if let Some(frame) = frame {
                    v.level.lines.push(format!(
                        "Pod shell level: drawn in its own level frame under the planets level's script camera (root scale {:.4}, the planets level's display scale over the shell's; no turn; level origin at display {:.4?}); the rig's fitted shell eyes are not used",
                        frame.scale.x,
                        planets_normalize.transform_point3(Vec3::ZERO).to_array()
                    ));
                }
                v.level.lines.extend(menu_line.clone());
                // D281: with `--play-sackboy` under the scripts the figure is the viewer's
                // own Sackboy, who plays in the shell level: the play figure, hung under
                // the shell's frame root and placed by the shell's display matrix, so he
                // stands in the shell level's own frame like everything else of it. The
                // fitted Pod figure is not spawned then.
                let own_sackboy = frame.is_some() && sackboy_play::in_pod();
                let line = match done.preview {
                    Ok(Some(preview)) if own_sackboy => sackboy_play::spawn_figure(
                        &mut v,
                        (&mut commands, &mut meshes, &mut materials, &mut images),
                        &mut bindposes,
                        normalize.x_axis.truncate().length(),
                        &preview,
                        (done.clip, done.play_bank),
                        Some((root, normalize)),
                    )
                    .map(|line| {
                        format!("{line}; in the Pod: under the shell level's frame root")
                    })?,
                    Ok(Some(preview)) => spawn_figure(
                        &mut v,
                        (&mut commands, &mut meshes, &mut materials, &mut images),
                        &mut bindposes,
                        (root, &rig, normalize, checkpoint),
                        &preview,
                        (done.clip, done.hold_clip),
                        level.shell_feet,
                    )?,
                    Err(e) => format!("Figure not loaded: {e}"),
                    Ok(None) => String::new(),
                };
                if !line.is_empty() {
                    v.level.lines.push(line);
                }
            } else if sackboy_play::enabled() {
                // Part 4: --select-guid names the play figure (default the Sackboy mesh).
                let line = match done.preview {
                    Ok(Some(preview)) => sackboy_play::spawn_figure(
                        &mut v,
                        (&mut commands, &mut meshes, &mut materials, &mut images),
                        &mut bindposes,
                        planets_normalize.x_axis.truncate().length(),
                        &preview,
                        (done.clip, done.play_bank),
                        None,
                    )?,
                    Err(e) => format!("Play figure not loaded: {e}"),
                    Ok(None) => String::new(),
                };
                if !line.is_empty() {
                    v.level.lines.push(line);
                }
            }
            v.status = format!(
                "Loaded level {}\n{}",
                level.identity,
                v.level.lines.join("\n")
            );
            println!("{}", v.status);
            return Ok(());
        }
        let Some(preview) = done.preview? else {
            v.status = "Source attached. Choose an asset in the browser.".into();
            return Ok(());
        };
        let g = &preview.geometry;
        let mut details = Vec::new();
        let prepared = prepare(&preview, true, &mut evidence, &mut details)?;
        let (_, applied) = v.standalone_options();
        evidence.instance_default = applied.instance_white && evidence.instance_input;
        evidence.animation_default = applied.animation_base && evidence.animation_snapshot;
        if evidence.instance_default {
            details.push("D188 default: instance color white (1,1,1,1), no Thing supplied one (placement default; THING_COLOR link inferred)".into());
        }
        if evidence.animation_default {
            details.push(
                "D188 default: material animation base values (t = base, no game time)".into(),
            );
        }
        let up = model_up(v.model_z_up);
        for p in prepared {
            let (mesh, material, image) = upload(p, &mut meshes, &mut materials, &mut images);
            let entity = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_rotation(up),
                ))
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
                        (frame, up),
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
                    meshes: v.scene.iter().map(|s| s.1.clone()).collect(),
                    alt: None,
                    blended: None,
                    layered: false,
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
/// P3h: glow colour (multiplies the map) and the optional map with its UV mode.
type GlowDraw = ([f32; 3], Option<(Image, u32)>);
/// P3i: reflection textures, encoded steps and ReflectionBlur.
/// P3i/P3l: 2D maps by graph slot (None = the cube slot), the cube map, steps, blur.
type ReflectionDraw = (
    Vec<Option<Image>>,
    Option<Image>,
    Vec<lbp_preview::FragmentStep>,
    f32,
);
/// CPU-side display preparation shared by the single-mesh and level paths: one entry per
/// supported material bucket (plus one neutral bucket). `normalize` centres and scales
/// the mesh to unit size (single-mesh view); level scenes keep model coordinates.
struct Prepared {
    mesh: Mesh,
    image: Option<Image>,
    second_image: Option<Image>,
    third_image: Option<(Image, lbp_preview::RelativeSampling)>,
    fourth_image: Option<(Image, lbp_preview::RelativeSampling)>,
    color: Color,
    alpha_mix: bool,
    /// D160: sticker decal overlay (alpha blended, drawn in front of its surface).
    decal: bool,
    /// D121: generated normals usable for preview lighting.
    normals_trusted: bool,
    /// P2q: bump map (linear) and BumpLevel, sampled on UV1 (docs/p2q-bump-gloss.md).
    bump: Option<(Image, f32)>,
    /// P2q: static gloss colour and CosinePower.
    gloss: Option<([f32; 3], f32)>,
    /// P3d: gloss colour map (linear) and its UV mode (1 = primary UV, 2 = UV1).
    spec_map: Option<(Image, u32)>,
    /// P3a: AlphaTestLevel cut-out (0 = none), LBP-lighting materials only.
    alpha_test: f32,
    /// P3h: unlit glow colour and its optional map (sRGB image, UV mode 2 = primary, 3 = UV1).
    glow: Option<GlowDraw>,
    /// D309: the glow alpha's form in the material's own program (fog mix or not).
    glow_alpha: lbp_preview::GlowAlphaForm,
    reflection: Option<ReflectionDraw>,
    /// D189: graph textures 1.. (texture 0 is `image`) and the encoded steps.
    graph: Option<(Vec<Image>, Vec<lbp_preview::FragmentStep>)>,
    /// P3k: backdrop baked-AO lightmap (R8, mipped) sampled on UV1.
    lightmap: Option<Image>,
    /// D284: the bucket's index in the list prepare_buckets was given.
    bucket: usize,
    /// D284: GUID of the bucket's material (model meshes only; none for a neutral bucket).
    material: Option<u32>,
    /// D284: the input that holds the mesh's raw UV0 (`pod_dissolve::uv0_source`).
    raw_uv0: u8,
    /// D286: clear glass (`window_glassy`): its output alpha, drawn premultiplied.
    glass: Option<f32>,
}
/// D286: draws a bucket as the game's glass: opacity [`GLASS_OPACITY`], and its raw UV0
/// as `in.uv` (MEASURED: the captured draw's TC0 is the mesh's UV0), so the dissolve's
/// mask lies on the mesh's own UV and not on the position tile. `raw_uv0` is given only
/// for a bucket without diffuse coordinates (a solid bucket's `in.uv` is all zero).
fn mark_glass(p: &mut Prepared, raw_uv0: Option<Vec<[f32; 2]>>) {
    p.glass = Some(GLASS_OPACITY);
    if let Some(uv) = raw_uv0.filter(|uv| uv.len() == p.mesh.count_vertices()) {
        p.mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        p.raw_uv0 = 1;
    }
}
/// D284: which input of a bucket holds raw UV0: the primary coordinates when they are
/// channel 0 unchanged, else the second sample's (drawn as UV_1) when they are.
fn raw_uv0_source(asset: Option<&lbp_preview::MaterialAsset>) -> u8 {
    let Some(a) = asset.and_then(|a| a.texture()) else {
        return 0;
    };
    let unchanged = |t: lbp_preview::SamplingTransform| match t {
        lbp_preview::SamplingTransform::ScaleOffset(t) => t.is_identity(),
        lbp_preview::SamplingTransform::Matrix(m) => m.values() == [1., 0., 0., 0., 1., 0.],
    };
    let primary = (a.selection.uv_channel, a.sampling_uv == a.uv);
    let second = a
        .dual
        .as_ref()
        .and(a.selection.second_sample)
        .filter(|_| a.instance_projection.is_none())
        .map(|s| (s.uv_channel, unchanged(s.uv_transform)));
    pod_dissolve::uv0_source(Some(primary), second)
}
/// D284: the material GUID of each bucket `adapter::buckets` makes, in its order: the
/// first primitive's of the bucket (every primitive joins one, as there), none for the
/// neutral bucket.
fn bucket_materials(preview: &lbp_preview::GroupedMesh) -> Vec<Option<u32>> {
    type Seen<'a> = Option<&'a std::sync::Arc<lbp_preview::MaterialAsset>>;
    let mut seen: Vec<(Seen, Option<u32>)> = Vec::new();
    let primitives = &preview.geometry.metadata.primitives;
    for (primitive, binding) in primitives.iter().zip(&preview.bindings) {
        let asset = binding.as_ref().ok();
        let known = seen.iter().any(|(a, _)| match (a, asset) {
            (None, None) => true,
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            _ => false,
        });
        if !known {
            seen.push((asset, asset.and(primitive.material_guid)));
        }
    }
    seen.into_iter().map(|(_, guid)| guid).collect()
}
fn prepare(
    preview: &lbp_preview::GroupedMesh,
    normalize: bool,
    evidence: &mut browser::PreviewEvidence,
    details: &mut Vec<String>,
) -> Result<Vec<Prepared>, String> {
    let g = &preview.geometry;
    let (triangles, ranges) = drop_materialless(preview)?;
    let buckets = adapter::buckets(&triangles, &ranges, &preview.bindings)?;
    let materials = bucket_materials(preview);
    for (index, (range, binding)) in g
        .primitive_triangles
        .iter()
        .zip(&preview.bindings)
        .enumerate()
    {
        if !range.is_empty() && binding.is_err() && preview.materialless(index) {
            // D188/D192: hidden (the game skips it), outside the diffuse-group denominator.
            evidence.materialless_groups += 1;
        } else if !range.is_empty() {
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
    for (index, (range, binding)) in g
        .primitive_triangles
        .iter()
        .zip(&preview.bindings)
        .enumerate()
    {
        let label = match binding {
            Ok(a) => a.label(),
            Err(e) if preview.materialless(index) => format!("Hidden: {e}"),
            Err(e) => format!("Neutral: {e}"),
        };
        details.push(format!(
            "Primitive {index}: {} triangles • {label}",
            range.len()
        ));
    }
    // D286: the glass's raw UV0, recovered from its bump map's coordinates (a solid
    // bucket has no diffuse coordinates of its own).
    let mut glass_uv: Vec<Option<Vec<[f32; 2]>>> = buckets
        .iter()
        .zip(&materials)
        .map(|((asset, _), material)| {
            let bump = asset
                .as_ref()
                .filter(|a| *material == Some(WINDOW_GLASSY) && a.sampling_uv().is_none())
                .and_then(|a| a.surface())
                .and_then(|s| s.bump.as_deref())?;
            raw_uv0_from_bump(bump.uv_channel, bump.uv_transform, &bump.sampling_uv)
        })
        .collect();
    let mut prepared = prepare_buckets(
        &g.positions,
        preview.normals.as_deref(),
        buckets,
        normalize,
        None,
        evidence,
        details,
    )?;
    for p in &mut prepared {
        p.material = materials.get(p.bucket).copied().flatten();
        if p.material == Some(WINDOW_GLASSY) {
            mark_glass(p, glass_uv.get_mut(p.bucket).and_then(Option::take));
        }
    }
    Ok(prepared)
}
/// D192: triangles and primitive ranges without the material-less groups. The game's
/// per-primitive draw loop skips a primitive whose material is null (no bind, no draw;
/// Ghidra, CONFIRMED), so those groups are not drawn. A whole-mesh depth/shadow pass
/// may still use them (INFERRED, not drawn here). Ranges stay one per primitive.
type TrianglePartition = (Vec<[u32; 3]>, Vec<std::ops::Range<usize>>);
fn drop_materialless(preview: &lbp_preview::GroupedMesh) -> Result<TrianglePartition, String> {
    let g = &preview.geometry;
    let mut triangles = Vec::with_capacity(g.triangles.len());
    let mut ranges = Vec::with_capacity(g.primitive_triangles.len());
    let mut end = 0;
    for (index, range) in g.primitive_triangles.iter().enumerate() {
        let source = g
            .triangles
            .get(range.clone())
            .filter(|_| range.start == end)
            .ok_or("Invalid: primitive triangle partition")?;
        end = range.end;
        let start = triangles.len();
        if !preview.materialless(index) {
            triangles.extend_from_slice(source);
        }
        ranges.push(start..triangles.len());
    }
    if end != g.triangles.len() {
        return Err("Invalid: unowned triangles".into());
    }
    Ok((triangles, ranges))
}
/// Material buckets over one position array (model mesh or merged shape batch).
fn prepare_buckets(
    positions: &[[f32; 3]],
    game_normals: Option<&[[f32; 3]]>,
    buckets: Vec<adapter::Bucket<lbp_preview::MaterialAsset>>,
    normalize: bool,
    lightmap_in: Option<(&level_scene::Lightmap, &[[f32; 2]])>,
    evidence: &mut browser::PreviewEvidence,
    details: &mut Vec<String>,
) -> Result<Vec<Prepared>, String> {
    // Prepare all adapters before allocating GPU assets. Shared full-mesh bounds
    // keep different groups aligned; supported groups share one draw per binding.
    let mut prepared = Vec::new();
    for (bucket, (asset, triangles)) in buckets.into_iter().enumerate() {
        if triangles.is_empty() {
            continue;
        }
        let build = if normalize {
            adapter::mesh
        } else {
            adapter::mesh_local
        };
        let mut mesh = match build(
            positions,
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
        let mut normals_trusted = adapter::mesh_normals_trusted(&mesh);
        // P2v: the game's decoded normals replace ours; the draw is trusted when every
        // referenced vertex took one.
        if let Some(game) = game_normals {
            let (used, total, agree) = adapter::apply_game_normals(&mut mesh, game);
            if total > 0 {
                details.push(format!(
                    "Normals: game (decoded) on {used}/{total} referenced vertices, mean |dot| with generated {agree:.3}"
                ));
                normals_trusted = used == total;
            }
        }
        let surface = asset.as_ref().and_then(|a| a.surface());
        let gloss = surface.and_then(|s| s.specular.map(|c| (c, s.cosine_power)));
        let alpha_test = surface.map_or(0., |s| s.alpha_test);
        let alpha_mix = asset
            .as_ref()
            .and_then(|a| a.texture())
            .is_some_and(|a| a.dual.is_some() && a.selection.texture_alpha_mix.is_some());
        // D189: per-fragment graph on raw UV0 (mesh UV) and raw UV1 (docs/d189-mixed-uv-graphs.md).
        let graph = match asset
            .as_ref()
            .and_then(|a| a.texture())
            .and_then(|a| a.fragment.as_ref())
        {
            Some(f) => {
                adapter::secondary_uv(&mut mesh, &f.uv1)?;
                let images = f
                    .textures
                    .iter()
                    .map(|t| {
                        adapter::image_axes(
                            t.texture.header.width,
                            t.texture.header.height,
                            t.texture.rgba.clone(),
                            t.wrap,
                        )
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Some((images, f.steps.clone()))
            }
            None => None,
        };
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
                    adapter::set_nearest(&mut image);
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
        // P2q: the bump map takes UV1 when the diffuse route leaves it free.
        let bump = match surface.and_then(|s| s.bump.as_deref().map(|b| (b, s.bump_level))) {
            Some((b, level)) if second_image.is_none() && graph.is_none() => {
                adapter::secondary_uv(&mut mesh, &b.sampling_uv)?;
                Some((
                    adapter::linear_image(
                        b.texture.header.width,
                        b.texture.header.height,
                        b.texture.rgba.clone(),
                        b.wrap,
                    )?,
                    level,
                ))
            }
            Some(_) => {
                details
                    .push("P2q: bump not drawn (UV1 is used by a second diffuse texture)".into());
                None
            }
            None => None,
        };
        // P3d: the gloss map shares the diffuse UVs, or takes UV1 when that is free or holds
        // the same coordinates (graph slot 5 carries the image, so no graph may be bound).
        let mut gloss = gloss;
        let spec_map = match surface.and_then(|s| s.spec_map.as_deref().map(|m| (m, s))) {
            Some((m, s)) if graph.is_none() => {
                let same_diffuse = asset
                    .as_ref()
                    .and_then(|a| a.sampling_uv())
                    .is_some_and(|d| d == m.sampling_uv.as_slice());
                let same_bump = surface
                    .and_then(|s| s.bump.as_deref())
                    .is_some_and(|b| bump.is_some() && b.sampling_uv == m.sampling_uv);
                let mode = if same_diffuse {
                    Some(1)
                } else if same_bump {
                    Some(2)
                } else if second_image.is_none() && bump.is_none() {
                    adapter::secondary_uv(&mut mesh, &m.sampling_uv)?;
                    Some(2)
                } else {
                    None
                };
                match mode {
                    Some(mode) => {
                        gloss.get_or_insert(([1.; 3], s.cosine_power));
                        Some((
                            adapter::linear_image(
                                m.texture.header.width,
                                m.texture.header.height,
                                m.texture.rgba.clone(),
                                m.wrap,
                            )?,
                            mode,
                        ))
                    }
                    None => {
                        details.push("P3d: gloss map not drawn (UV1 is taken)".into());
                        None
                    }
                }
            }
            Some(_) => {
                details
                    .push("P3d: gloss map not drawn (a per-fragment graph uses its slot)".into());
                None
            }
            None => None,
        };
        // P3h: glow (output port 4) is a solid colour, or one baked map on the diffuse UVs or
        // on UV1 when that is free or holds the same coordinates (graph slot 4, so no graph).
        let glow_alpha = surface.map_or(Default::default(), |s| s.glow_alpha);
        let glow = match surface.and_then(|s| s.glow.as_deref()) {
            Some(lbp_preview::MaterialAsset::Solid(g)) => {
                let [r, gr, b, _] = g.selection.color.values();
                ([r, gr, b] != [0.; 3]).then_some(([r, gr, b], None))
            }
            Some(lbp_preview::MaterialAsset::Texture(g))
                if graph.is_none()
                    && g.dual.is_none()
                    && g.third.is_none()
                    && g.fourth.is_none()
                    && g.product.is_none()
                    && g.fragment.is_none() =>
            {
                let same_diffuse = asset
                    .as_ref()
                    .and_then(|a| a.sampling_uv())
                    .is_some_and(|d| d == g.sampling_uv.as_slice());
                let owner: Option<&[[f32; 2]]> = asset
                    .as_ref()
                    .and_then(|a| a.texture())
                    .and_then(|a| a.dual.as_ref())
                    .map(|d| d.sampling_uv.as_slice())
                    .or_else(|| {
                        surface
                            .and_then(|s| s.bump.as_deref())
                            .filter(|_| bump.is_some())
                            .map(|b| b.sampling_uv.as_slice())
                    })
                    .or_else(|| {
                        surface
                            .and_then(|s| s.spec_map.as_deref())
                            .filter(|_| spec_map.as_ref().is_some_and(|(_, m)| *m == 2))
                            .map(|m| m.sampling_uv.as_slice())
                    });
                let mode = if same_diffuse {
                    Some(2)
                } else if owner == Some(g.sampling_uv.as_slice()) {
                    Some(3)
                } else if owner.is_none() {
                    adapter::secondary_uv(&mut mesh, &g.sampling_uv)?;
                    Some(3)
                } else {
                    None
                };
                match mode {
                    Some(mode) => {
                        let [width, height] = g.display_extent();
                        let mut image = adapter::image_axes(
                            width,
                            height,
                            g.display_pixels().to_vec(),
                            g.wrap,
                        )?;
                        if g.nearest_sampling() {
                            adapter::set_nearest(&mut image);
                        }
                        Some(([1.; 3], Some((image, mode))))
                    }
                    None => {
                        details.push("P3h: glow map not drawn (UV1 is taken)".into());
                        None
                    }
                }
            }
            Some(lbp_preview::MaterialAsset::Texture(_)) => {
                details
                    .push("P3h: glow map not drawn (it needs several textures or a graph)".into());
                None
            }
            None => None,
        };
        // P3i: the reflection graph takes the free graph texture slots 0..=3, so it needs a
        // plain diffuse (no per-fragment graph and no second, third or fourth sample).
        let reflection = match surface.and_then(|s| s.reflection.as_deref()) {
            Some(r)
                if graph.is_none()
                    && second_image.is_none()
                    && third_image.is_none()
                    && fourth_image.is_none() =>
            {
                let mut cube = None;
                let images = r
                    .textures
                    .iter()
                    .map(|m| match m {
                        lbp_preview::surface::ReflectionMap::Flat(t, wrap) => adapter::image_axes(
                            t.header.width,
                            t.header.height,
                            t.rgba.clone(),
                            *wrap,
                        )
                        .map(Some),
                        lbp_preview::surface::ReflectionMap::Cube(c) => {
                            let faces: Vec<&[u8]> = std::iter::once(c.texture.rgba.as_slice())
                                .chain(c.extra_faces.iter().map(Vec::as_slice))
                                .collect();
                            cube = Some(adapter::cube_image(c.texture.header.width, &faces)?);
                            Ok(None)
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Some((images, cube, r.steps.clone(), r.blur))
            }
            Some(_) => {
                details.push(
                    "P3i: reflection not drawn (the diffuse uses the graph texture slots)".into(),
                );
                None
            }
            None => None,
        };
        // P3k: the backdrop lightmap needs UV1 (the mesh's own unwrap); any feature above that
        // already took UV1 for a material coordinate set wins and the AO is not drawn.
        let lightmap = match lightmap_in {
            Some((map, uv1)) if mesh.attribute(Mesh::ATTRIBUTE_UV_1).is_none() => {
                adapter::secondary_uv(&mut mesh, uv1)?;
                Some(adapter::lightmap_image(map.width, map.height, &map.r8)?)
            }
            Some(_) => {
                details.push("P3k: baked AO not drawn (UV1 is taken)".into());
                None
            }
            None => None,
        };
        // D284: decided after every coordinate set above has been put on the mesh.
        let raw_uv0 = raw_uv0_source(asset.as_deref());
        prepared.push(Prepared {
            mesh,
            image,
            second_image,
            third_image,
            fourth_image,
            color,
            alpha_mix,
            decal: false,
            normals_trusted,
            bump,
            gloss,
            spec_map,
            alpha_test,
            glow,
            glow_alpha,
            reflection,
            graph,
            lightmap,
            bucket,
            material: None,
            raw_uv0,
            glass: None,
        });
    }
    if prepared.is_empty() {
        return Err("Unavailable: no nondegenerate faces to display".into());
    }
    Ok(prepared)
}
fn upload(
    p: Prepared,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
) -> (Handle<Mesh>, Handle<ViewerMaterial>, Option<Handle<Image>>) {
    let blend = alpha_mode(&p);
    let bias = depth_bias(&p);
    let image = p.image.map(|i| images.add(i));
    let mut surface = dual::material(StandardMaterial {
        base_color: p.color,
        base_color_texture: image.clone(),
        unlit: true,
        cull_mode: None,
        alpha_mode: blend,
        depth_bias: bias,
        ..default()
    });
    if let Some(second) = p.second_image {
        dual::enable(&mut surface, images.add(second));
        if p.alpha_mix {
            dual::enable_alpha_mix(&mut surface);
        }
    }
    if let Some((third, relative)) = p.third_image {
        dual::enable_third(&mut surface, images.add(third), relative);
    }
    if let Some((fourth, relative)) = p.fourth_image {
        dual::enable_fourth(&mut surface, images.add(fourth), relative);
    }
    if let Some((bump, level)) = p.bump {
        dual::enable_bump(&mut surface, images.add(bump), level);
    }
    if let (Some((extra, steps)), Some(primary)) = (p.graph, image.clone()) {
        let mut handles = vec![primary];
        handles.extend(extra.into_iter().map(|i| images.add(i)));
        if let Err(e) = dual::enable_graph(&mut surface, handles, &steps) {
            // Budgets are checked on the CPU side; a refusal here keeps the base texture.
            eprintln!("D189: {e}");
        }
    }
    if let Some((textures, cube, steps, blur)) = p.reflection {
        let handles = textures
            .into_iter()
            .map(|i| i.map(|i| images.add(i)))
            .collect();
        let cube = cube.map(|i| images.add(i));
        if let Err(e) = dual::enable_reflection(&mut surface, handles, cube, &steps, blur) {
            eprintln!("P3i: {e}");
        }
    }
    if let Some((color, map)) = p
        .glow
        .filter(|_| !NO_GLOW.load(std::sync::atomic::Ordering::Relaxed))
    {
        dual::enable_glow(
            &mut surface,
            color,
            map.map(|(image, mode)| (images.add(image), mode)),
            dual::glow_alpha_fogged(p.glow_alpha),
        );
    }
    if let Some(map) = p
        .lightmap
        .filter(|_| !NO_BAKED_AO.load(std::sync::atomic::Ordering::Relaxed))
    {
        dual::enable_lightmap(&mut surface, images.add(map));
    }
    if p.alpha_test > 0. {
        dual::enable_alpha_test(&mut surface, p.alpha_test);
    }
    if let Some((color, power)) = p.gloss {
        dual::enable_gloss(&mut surface, color, power);
    }
    if let Some((map, mode)) = p.spec_map {
        dual::enable_spec_map(&mut surface, images.add(map), mode);
    }
    if let Some(opacity) = p.glass {
        dual::enable_glass(
            &mut surface,
            opacity,
            GLASS_DISSOLVE_GREEN,
            GLASS_SPECULAR_SCALE,
        );
    }
    surface.extension.normals_trusted = p.normals_trusted;
    surface.extension.raw_uv0 = p.raw_uv0;
    (meshes.add(p.mesh), materials.add(surface), image)
}
/// How a bucket blends. D286: the glass is `Premultiplied`, Bevy's One /
/// OneMinusSrcAlpha with the shader's colour taken as already multiplied by its alpha:
/// the captured draw's `One, InvSrcAlpha` (MEASURED). Its depth is not written (Bevy's
/// transparent pass; the game writes it), so nothing behind it is lost.
fn alpha_mode(p: &Prepared) -> AlphaMode {
    if p.glass.is_some() {
        AlphaMode::Premultiplied
    } else if p.decal {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    }
}
/// D160: a sticker decal is drawn this much in front of its surface.
const DECAL_DEPTH_BIAS: f32 = 8.0;
/// D288 (docs/level-scripts.md, part 39): the constant depth bias of a bucket, so that a
/// mesh's later material bucket wins where two of its buckets lie on the same triangles.
/// The planets room's hanging cloths are such a pair: a plain cloth (gfx material
/// 0001f3ad, the mesh's earlier bucket) under the painted cloths (0001f3b3, 0001f3b4,
/// later buckets) at the same depth. The game shows the painted ones (MEASURED, the d156
/// frame); Bevy's opaque pass draws buckets in an order of its own, so which one won the
/// equal depth changed with the pipelines (the preview route, the level alone, any change
/// to the draw list). Each bucket is one bias step nearer than the one before it
/// (INFERRED: the game draws a mesh's primitives in order and keeps the later fragment at
/// equal depth). Bucket 0, so every one-material mesh, keeps no bias; decals and the
/// glass keep theirs.
fn depth_bias(p: &Prepared) -> f32 {
    if p.decal {
        DECAL_DEPTH_BIAS
    } else if p.glass.is_some() {
        0.0
    } else {
        p.bucket.min(adapter::MAX_DRAWS) as f32
    }
}
/// D113: display plan for a level scene, built without GPU or window. Per scene mesh the
/// prepared buckets (or the display error); per displayed item its mesh and Transform
/// (world matrix after the scene normalization below).
struct LevelPlan {
    prepared: Vec<Result<Vec<Prepared>, String>>,
    placed: Vec<(usize, Transform)>,
    /// D268: per entry of `placed`, the Thing whose own model the item draws (its render
    /// mesh or a decoration); none for a merged shape batch, the backdrop, decals and
    /// per-item skinned copies.
    things: Vec<Option<thing_view::Item>>,
    /// D276: per entry of `placed`, whether the item is spawned hidden: the other mesh
    /// of a pair the level's scripts swap (`SceneItem::alternate`).
    hidden: Vec<bool>,
    lines: Vec<String>,
    home: Orbit,
    /// Level units to display units (centre and unit-size scale).
    normalize: Mat4,
    /// D121: level-settings preview light (generic when absent or unusable).
    light: lighting::PreviewLight,
    /// The frame the light was built in (`light_frame`), when the scene has bounds.
    light_frame: Option<(Vec3, f32)>,
}
fn plan_level(
    scene: &LevelScene,
    focus: Option<usize>,
    pod: Option<usize>,
) -> Result<LevelPlan, String> {
    let prepared: Vec<_> = scene
        .meshes
        .iter()
        .map(|m| {
            let mut evidence = browser::PreviewEvidence::default();
            let mut details = Vec::new();
            match &m.payload {
                level_scene::ScenePayload::Model(g) => {
                    prepare(g, false, &mut evidence, &mut details)
                }
                // World-space batch; a material that did not select draws neutral grey.
                level_scene::ScenePayload::Shapes { batch, material } => prepare_buckets(
                    &batch.positions,
                    None,
                    vec![(material.as_ref().ok().cloned(), batch.triangles.clone())],
                    false,
                    None,
                    &mut evidence,
                    &mut details,
                ),
                // D119 backdrop batch (mesh space; the item matrix places it).
                level_scene::ScenePayload::Backdrop {
                    batch,
                    material,
                    lightmap,
                } => prepare_buckets(
                    &batch.positions,
                    None,
                    vec![(material.as_ref().ok().cloned(), batch.triangles.clone())],
                    false,
                    lightmap.as_deref().map(|l| (l, batch.uv1.as_slice())),
                    &mut evidence,
                    &mut details,
                ),
                // D160 sticker decal overlays (world space, alpha blended).
                level_scene::ScenePayload::Decals { batch, material } => prepare_buckets(
                    &batch.positions,
                    None,
                    vec![(Some(material.clone()), batch.triangles.clone())],
                    false,
                    None,
                    &mut evidence,
                    &mut details,
                )
                .map(|mut v| {
                    for p in &mut v {
                        p.decal = true;
                    }
                    v
                }),
            }
        })
        .collect();
    let buckets: Vec<Result<usize, String>> = prepared
        .iter()
        .map(|p| p.as_ref().map(Vec::len).map_err(Clone::clone))
        .collect();
    let (placed, home, display, normalize) = place_items(scene, &buckets, focus, pod)?;
    // The items `place_items` placed, in its order: those whose mesh has display buckets.
    let (things, hidden): (Vec<_>, Vec<_>) = scene
        .items
        .iter()
        .filter(|i| buckets.get(i.mesh).is_some_and(Result::is_ok))
        .map(|i| (thing_item(scene, i), i.alternate))
        .unzip();
    let mut lines = scene.counts.lines();
    lines.extend(display);
    let light = level_light(scene, &buckets);
    let light_frame = light_frame(scene, &buckets);
    Ok(LevelPlan {
        prepared,
        placed,
        things,
        hidden,
        lines,
        home,
        normalize,
        light,
        light_frame,
    })
}
/// D268: the Thing a scene item draws, when the item is the Thing's own model: a mesh
/// named by a RENDER_MESH or a decoration, placed by its own matrix. D317: or that mesh
/// skinned per bone (P2m), which is baked in the frame of the item's matrix and so
/// moves with the item's Thing like the file's mesh. INFERRED for bone Things that keep
/// their places relative to the item's Thing under the scripts (MEASURED for the Pod
/// Earth: its bone Things are children of the Earth Thing and move with it).
fn thing_item<T>(
    scene: &level_scene::LevelScene<T>,
    item: &level_scene::SceneItem,
) -> Option<thing_view::Item> {
    let source = &scene.meshes.get(item.mesh)?.source;
    let model = matches!(
        source,
        level_scene::MeshSource::Guid(_)
            | level_scene::MeshSource::Sha1(_)
            | level_scene::MeshSource::BoneSkin { .. }
    );
    let backdrop = Some(item.thing) == scene.counts.backdrop.thing;
    (model && !backdrop && item.shapes == 0).then(|| thing_view::Item {
        thing: item.thing as u32,
        decoration: item.decoration,
        world: Mat4::from_cols_array(&item.world),
        mesh: source.base_guid(),
        alternate: item.alternate,
    })
}
/// D276: the mesh pairs a level scene loads both meshes of: those the scripts swap,
/// when the level is played by a script world (`--level-scripts`), else none.
fn swapped_meshes(scripts: bool) -> Vec<[u32; 2]> {
    if scripts {
        thing_view::SWAPPED_MESHES.to_vec()
    } else {
        Vec::new()
    }
}
/// D121: preview light from the level's stored settings, in the frame `level_home` uses
/// (sun direction from the displayed bounds centre; fog distances scaled to display).
/// P2l: lights the level from its stored preset `n` (which block the game applies is
/// unknown; docs/p2l-level-presets.md). A missing preset keeps the main settings.
fn use_lighting_preset(scene: &mut LevelScene, n: usize) -> String {
    match scene.settings_presets.get(n) {
        Some(preset) => {
            scene.settings = Some(*preset);
            format!(
                "Lighting preset {n} of {} (instead of the main settings)",
                scene.settings_presets.len()
            )
        }
        None => format!(
            "Lighting preset {n}: not stored ({} kept); main settings",
            scene.settings_presets.len()
        ),
    }
}
/// P3m: lights the level from its stored presets blended by PWorld `lighting` and dimmed
/// by `darkness` (`LevelSettings::at_lighting`; live reads of tutorial_23 and Bravery
/// Test). Without presets the main block stays (darkness applies).
fn use_world_lighting(scene: &mut LevelScene, fog_palette: &[[f32; 4]], pod: bool) -> String {
    let Some(main) = scene.settings else {
        return "Lighting: no level settings read".into();
    };
    let [lighting, _, fog, fog_tint, darkness] = scene.world_lighting.unwrap_or_default();
    let (settings, at) =
        level_scene::LevelSettings::at_lighting(&main, &scene.settings_presets, lighting, darkness);
    let settings = settings
        .with_fog_tint(fog_tint, fog_palette, pod)
        .with_fog(fog);
    if fog != 0. {
        println!(
            "Fog {fog:.3}: fog near {:.1}, far {:.1} level units (D333)",
            settings.fog_near, settings.fog_far
        );
    }
    scene.settings = Some(settings);
    if fog_tint > 0. {
        let c = settings.fog_color;
        println!(
            "Fog tint {fog_tint:.3}: fog colour ({:.3}, {:.3}, {:.3}, {:.3}) over {} palette colours (D332)",
            c[0], c[1], c[2], c[3], fog_palette.len()
        );
    }
    match at {
        Some(at) => format!(
            "Lighting: {} stored presets blended at position {at:.2} (PWorld lighting {lighting:.2}, darkness {darkness:.2})",
            scene.settings_presets.len()
        ),
        None => format!(
            "Lighting: no presets stored; main block (PWorld darkness {darkness:.2}; the live game's choice here is INFERRED)"
        ),
    }
}
/// D334: the level's water body (`water::WaterBody`), from the level bottom up to the
/// surface the PWorld's water level gives, over the level's span (backdrop excluded),
/// coloured by its water tint. Hidden while there is no water; play mode
/// (`level_play::follow_water`) moves it with the values in force. A line on what it did.
fn spawn_water(
    v: &mut Viewer,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    level: &LevelLoaded,
    normalize: Mat4,
) -> String {
    let scene = &level.scene;
    let Some([water_level, _, tint, _]) = scene.world_water else {
        return "Water: no PWorld".into();
    };
    let backdrop = |i: &level_scene::SceneItem| Some(i.thing) == scene.counts.backdrop.thing;
    let Some([lo, hi]) = scene
        .bounds_where(|i| !backdrop(i))
        .or_else(|| scene.bounds_where(|_| true))
    else {
        return "Water: no level bounds".into();
    };
    let (palette, note) = match &level.water_palette {
        Ok(p) => (p.clone(), format!("{} colours", p.len())),
        Err(e) => (Vec::new(), format!("colours not read ({e}), grey")),
    };
    let body = water::WaterBody {
        floor: lo[1],
        centre: Vec2::new(lo[0] + hi[0], lo[2] + hi[2]) * 0.5,
        size: Vec2::new(hi[0] - lo[0], hi[2] - lo[2]),
        normalize,
        palette,
        shown: None,
    };
    let surface = lbp_runtime::water_surface_y(water_level);
    let transform = body.transform(surface.unwrap_or(body.floor));
    let material = materials.add(water::material(body.colour(tint)));
    let mesh = meshes.add(Cuboid::new(1., 1., 1.));
    let visibility = if surface.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let shown = surface.map(|y| (y, tint));
    let entity = commands
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            transform,
            visibility,
            water::WaterBody { shown, ..body },
        ))
        .id();
    v.scene.push((entity, mesh, material, None));
    match surface {
        Some(y) => format!(
            "Water (D334): level {water_level:.3} -> surface Y {y:.1} level units, tint {tint:.3}; {note}"
        ),
        None => format!("Water (D334): none (level {water_level:.3}); {note}"),
    }
}
/// The frame of a level's light: the displayed bounds centre in level units and the
/// display units per level unit.
fn light_frame(scene: &LevelScene, buckets: &[Result<usize, String>]) -> Option<(Vec3, f32)> {
    let shown = |i: &level_scene::SceneItem| buckets.get(i.mesh).is_some_and(Result::is_ok);
    // Same bounds as `place_items` (D119: backdrop excluded) so the fog scale and sun
    // centre match the display normalization.
    let backdrop = |i: &level_scene::SceneItem| Some(i.thing) == scene.counts.backdrop.thing;
    let [lo, hi] = scene
        .bounds_where(|i| shown(i) && !backdrop(i))
        .or_else(|| scene.bounds_where(shown))?;
    let (lo, hi) = (Vec3::from(lo), Vec3::from(hi));
    let span = (hi - lo).max_element();
    Some(((lo + hi) * 0.5, 1. / span))
}
fn level_light(scene: &LevelScene, buckets: &[Result<usize, String>]) -> lighting::PreviewLight {
    let (Some(settings), Some((centre, scale))) = (&scene.settings, light_frame(scene, buckets))
    else {
        let mut light = lighting::PreviewLight::generic();
        light
            .notes
            .push("no level settings read; generic light".into());
        return light;
    };
    lighting::PreviewLight::from_settings(settings, centre, scale)
}
/// Scene normalization and framing. The whole level is centred and scaled to unit size
/// (largest axis = 1) so the shared orbit camera, clip planes and pan limits of the
/// single-mesh view apply unchanged; the home view looks along -Z (LBP side-on) and
/// fits the X/Y extent in a 1280x800 window with the default 45 degree vertical FOV.
fn level_home(lo: Vec3, hi: Vec3) -> Result<(Mat4, Orbit), String> {
    let size = hi - lo;
    let span = size.max_element();
    if !span.is_finite() || span <= 0. {
        return Err("Unavailable: zero-size or nonfinite level bounds".into());
    }
    let normalize =
        Mat4::from_scale(Vec3::splat(1. / span)) * Mat4::from_translation(-(lo + hi) * 0.5);
    let unit = size / span;
    let tan = (PerspectiveProjection::default().fov * 0.5).tan();
    let aspect = 1280. / 800.;
    let distance = ((unit.x * 0.5 / (tan * aspect)).max(unit.y * 0.5 / tan) * 1.1 + unit.z * 0.5)
        .clamp(0.15, 30.);
    Ok((
        normalize,
        Orbit {
            yaw: 0.,
            pitch: 0.,
            distance,
            target: Vec3::ZERO,
        },
    ))
}
/// Items whose mesh has display buckets get a Transform; others are counted by reason.
type Placement = (Vec<(usize, Transform)>, Orbit, Vec<String>, Mat4);
fn place_items<T>(
    scene: &level_scene::LevelScene<T>,
    buckets: &[Result<usize, String>],
    focus: Option<usize>,
    pod: Option<usize>,
) -> Result<Placement, String> {
    let shown = |i: &level_scene::SceneItem| buckets.get(i.mesh).is_some_and(Result::is_ok);
    // D119: frame the level itself; the backdrop (items carrying the backdrop Thing) is
    // drawn but only frames the view when nothing else is shown.
    let backdrop = |i: &level_scene::SceneItem| Some(i.thing) == scene.counts.backdrop.thing;
    let [lo, hi] = scene
        .bounds_where(|i| shown(i) && !backdrop(i))
        .or_else(|| scene.bounds_where(shown))
        .ok_or("Unavailable: no displayable level items")?;
    let (lo, hi) = match focus {
        None => (Vec3::from(lo), Vec3::from(hi)),
        Some(n) => {
            // D147: the focused Thing's bounds, widened threefold around their centre.
            let [a, b] = scene
                .bounds_where(|i| shown(i) && i.thing == n)
                .ok_or("--level-focus-thing: Thing not drawn")?;
            let (a, b) = (Vec3::from(a), Vec3::from(b));
            let half = Vec3::splat((b - a).max_element().max(1.) * 1.5);
            ((a + b) / 2. - half, (a + b) / 2. + half)
        }
    };
    let (normalize, mut home) = level_home(lo, hi)?;
    if let Some(n) = pod {
        let [a, b] = scene
            .bounds_where(|i| shown(i) && i.thing == n)
            .ok_or("--pod-camera: Thing not drawn")?;
        let (a, b) = (
            normalize.transform_point3(Vec3::from(a)),
            normalize.transform_point3(Vec3::from(b)),
        );
        let centre = (a + b) / 2.;
        let eye = pod_menu::pod_camera(
            centre,
            (b - a).max_element() / 2.,
            PerspectiveProjection::default().fov,
        );
        home = Orbit {
            yaw: 0.,
            pitch: 0.,
            distance: eye.z - centre.z,
            target: Vec3::new(eye.x, eye.y, centre.z),
        };
    }
    let mut placed = Vec::new();
    let mut entities = 0usize;
    let mut sheared = 0usize;
    let (mut model_things, mut shape_things, mut backdrop_items) = (0usize, 0usize, 0usize);
    let mut skipped = std::collections::BTreeMap::<String, usize>::new();
    for item in &scene.items {
        // D122: a bevelled shape split over several material batches is counted in its
        // first batch only, so later batches can carry 0 Things.
        let is_shape = item.shapes > 0
            || scene
                .meshes
                .get(item.mesh)
                .is_some_and(|m| matches!(m.source, level_scene::MeshSource::Shapes { .. }));
        let things = if is_shape { item.shapes } else { 1 };
        let count = match buckets.get(item.mesh) {
            Some(Ok(n)) => *n,
            Some(Err(e)) => {
                *skipped.entry(e.clone()).or_default() += things;
                continue;
            }
            None => {
                *skipped.entry("Invalid: mesh index".into()).or_default() += things;
                continue;
            }
        };
        if backdrop(item) {
            backdrop_items += 1;
        } else if is_shape {
            shape_things += item.shapes;
        } else {
            model_things += 1;
        }
        let matrix = normalize * Mat4::from_cols_array(&item.world);
        let transform = Transform::from_matrix(matrix);
        if !transform.to_matrix().abs_diff_eq(matrix, 1e-4) {
            sheared += 1;
        }
        entities += count;
        placed.push((item.mesh, transform));
    }
    let mut lines = vec![
        format!("Drawn in viewer: {model_things} RENDER_MESH Things, {shape_things} shape Things, {backdrop_items} backdrop items ({entities} draw entities)"),
    ];
    for (reason, n) in &skipped {
        lines.push(format!("  display-skipped {n} Things: {reason}"));
    }
    lines.push(format!(
        "Shear dropped by TRS transform: {sheared} Things (placement approximate)"
    ));
    lines.push(format!(
        "Bounds (level units): ({:.1}, {:.1}, {:.1}) .. ({:.1}, {:.1}, {:.1})",
        lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]
    ));
    if let Some(n) = focus {
        lines.push(format!("View framed on Thing {n} (--level-focus-thing)"));
    }
    if let Some(n) = pod {
        lines.push(format!(
            "Pod camera on Thing {n} (--pod-camera; matched to the RPCS3 frame, not the game's camera)"
        ));
    }
    if backdrop_items > 0 {
        if let Some([a, b]) = scene.bounds_where(shown) {
            lines.push(format!(
                "Bounds with backdrop (level units): ({:.1}, {:.1}, {:.1}) .. ({:.1}, {:.1}, {:.1})",
                a[0], a[1], a[2], b[0], b[1], b[2]
            ));
        }
    }
    Ok((placed, home, lines, normalize))
}
/// P2b: the shell level planned on its own, and the rig that places it over the view
/// (pod_menu::PodRig). The DualShock (mesh 000042af, PC d155) anchors the shell camera.
fn pod_shell(
    scene: &LevelScene,
    plan: &LevelPlan,
    globe: usize,
    shell: &LevelScene,
    camera: Option<[f32; 6]>,
    (game, scripted): (bool, bool),
) -> Result<(pod_menu::PodRig, LevelPlan, String), String> {
    const DUALSHOCK: u32 = 0x0000_42af;
    let display = |m: &Mat4, [a, b]: [[f32; 3]; 2]| {
        let (a, b) = (
            m.transform_point3(Vec3::from(a)),
            m.transform_point3(Vec3::from(b)),
        );
        ((a + b) / 2., b - a)
    };
    let g = scene
        .bounds_where(|i| i.thing == globe)
        .ok_or("--pod-camera: Thing not drawn")?;
    let (g_centre, g_size) = display(&plan.normalize, g);
    let shell_plan = plan_level(shell, None, None)?;
    let pad = shell
        .bounds_where(|i| shell.meshes[i.mesh].source.base_guid() == Some(DUALSHOCK))
        .ok_or("--pod-shell: no DualShock (mesh 000042af) in the shell level")?;
    let (p_centre, p_size) = display(&shell_plan.normalize, pad);
    let [lo, hi] = shell.bounds().ok_or("--pod-shell: empty shell level")?;
    let n = shell_plan.normalize;
    let near = n.transform_point3(Vec3::from(lo)).z;
    let mut rig = pod_menu::PodRig::new(
        (g_centre, g_size.max_element() / 2.),
        (p_centre, p_size.x),
        near,
        // D275: the game's field of view also whenever the level's scripts run: the
        // script's camera has it (1.0 rad), and the shell eyes are fitted for the rig's.
        if game || scripted {
            pod_menu::GAME_FOV
        } else {
            PerspectiveProjection::default().fov
        },
    );
    let mut game_line = String::new();
    if game {
        // P2k: planets eyes from the game's presets, in planets display units; the globe
        // as the closed World transform places it.
        let m = plan.normalize;
        let presets = pod_menu::CAMERA_PRESETS.map(|pair| pair.map(|p| m.transform_point3(p)));
        let globe_closed = planets_root(m, 0.).transform_point(g_centre);
        rig = rig.with_planets_camera(presets, globe_closed);
        // Where the globe should show (16:9 fractions, radius in heights), for comparison
        // with the capture: a mismatch points at rendering, not the model.
        let r = g_size.max_element() / 2.;
        let shown: Vec<String> = [0., 0.5, 1.]
            .iter()
            .map(|&t| {
                let root = planets_root(m, t);
                let local = rig.planet_turn(t).inverse()
                    * (root.transform_point(g_centre) - rig.planet_eye(t));
                let c = Mat4::perspective_rh(rig.fov, pod_menu::ASPECT_16_9, 0.001, 1000.)
                    .project_point3(local);
                let radius = r * root.scale.x / -local.z / (rig.fov / 2.).tan() / 2.;
                format!(
                    "t {t}: {:.3}/{:.3} r {radius:.3} h",
                    (c.x + 1.) / 2.,
                    (1. - c.y) / 2.
                )
            })
            .collect();
        game_line = format!(
            "; globe expected (16:9, vfov {:.1} deg) {}",
            rig.fov.to_degrees(),
            shown.join(", ")
        );
    }
    if scripted {
        // D275: the level's script writes the planets camera; the rig's own planets eyes
        // (the same presets) only give the first frames, Home and the size of the
        // shell: its far side at half the resting camera's distance to the Earth, at the
        // place the Earth's script gives it under the closed World transform (INFERRED,
        // as P2k's rule for the constant placement).
        let m = plan.normalize;
        let presets = pod_menu::CAMERA_PRESETS.map(|pair| pair.map(|p| m.transform_point3(p)));
        let earth = planets_root(m, 0.).transform_point(m.transform_point3(pod_menu::EARTH_MENU.0));
        rig = rig.with_planets_camera(presets, earth);
        game_line = format!(
            "; under --level-scripts the planets camera is the script's (vfov {:.1} deg) and draws the shell level in its own frame: these eyes are not used",
            rig.fov.to_degrees()
        );
    }
    if let Some(c) = camera {
        let [from, to] = [
            n.transform_point3(Vec3::new(c[0], c[1], c[2])),
            n.transform_point3(Vec3::new(c[3], c[4], c[5])),
        ];
        let bounds = [
            n.transform_point3(Vec3::from(lo)),
            n.transform_point3(Vec3::from(hi)),
        ];
        rig = rig.with_game_camera(from, to, bounds);
    }
    let level = |p: Vec3| n.inverse().transform_point3(p);
    let [closed, open] = rig.shell.map(level);
    let line = format!(
        "Shell eyes (shell level units): closed ({:.0}, {:.0}, {:.0}) {}, open ({:.0}, {:.0}, {:.0}) fitted to the DualShock",
        closed.x,
        closed.y,
        closed.z,
        if camera.is_some() {
            "from --pod-shell-camera"
        } else {
            "fitted to the DualShock, looking -Z"
        },
        open.x,
        open.y,
        open.z
    ) + &game_line;
    Ok((rig, shell_plan, line))
}
/// P2c: the `--select-guid` mesh (Sackboy) standing in the Pod shell at model scale
/// (level units = model units), upright (D148 rotation), feet centred on the solved spot.
/// The spot is fitted so the closed view matches RPCS3 (`pod_menu::SACKBOY_CLOSED`):
/// the shell's checkpoint POS put him above the frame (PC run of #52). D-next: with
/// `--pod-figure-data` the closed spot comes from the shell's data instead (checkpoint x +
/// the Moon spawn x, lane 0, dropped to the Pod floor; `sackboy_play::level_spawn_feet`);
/// both spots are reported either way. `--play-animation` plays on him;
/// `--pod-hold-animation` while the Puter is open (P2d).
fn spawn_figure(
    v: &mut Viewer,
    (commands, meshes, materials, images): (
        &mut Commands,
        &mut Assets<Mesh>,
        &mut Assets<ViewerMaterial>,
        &mut Assets<Image>,
    ),
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    (root, rig, normalize, checkpoint): (Entity, &pod_menu::PodRig, Mat4, Option<Checkpoint>),
    preview: &lbp_preview::GroupedMesh,
    (clip, hold): (Option<ClipResult>, Option<ClipResult>),
    data_feet: Option<Result<([f32; 3], String), String>>,
) -> Result<String, String> {
    let bounds @ [lo, hi] =
        adapter_bounds(&preview.geometry.positions).ok_or("Unsupported: empty figure")?;
    let k = normalize.x_axis.truncate().length();
    let feet = rig.fit_figure(k * (hi[2] - lo[2]));
    // Raw Z-up model: feet are the lowest z, centred in x and y (depth).
    let origin = Mat4::from_translation(Vec3::new(
        -(lo[0] + hi[0]) / 2.,
        -(lo[1] + hi[1]) / 2.,
        -lo[2],
    ));
    let upright = Mat4::from_scale(Vec3::splat(k)) * Mat4::from_quat(model_up(true)) * origin;
    let fitted = Mat4::from_translation(feet) * upright;
    let data = data_feet.map(|r| {
        r.map(|(f, line)| {
            let at = normalize.transform_point3(Vec3::from(f));
            (Vec3::from(f), Mat4::from_translation(at) * upright, line)
        })
    });
    let place = match &data {
        Some(Ok((_, m, _))) if v.level.figure_data => *m,
        _ => fitted,
    };
    // P2d: holding the controller he is seen from behind: half a turn about the level's
    // up axis (Y), P2f: at his own spot, fitted so his centre shows where RPCS3 has it.
    let open_feet = rig.fit_figure_open(k * (hi[2] - lo[2]));
    let back =
        Mat4::from_translation(open_feet) * Mat4::from_rotation_y(std::f32::consts::PI) * upright;
    let figure = commands
        .spawn((
            Transform::from_matrix(place),
            Visibility::default(),
            ChildOf(root),
            PodFigure {
                front: Transform::from_matrix(place),
                back: Transform::from_matrix(back),
                holding: false,
            },
        ))
        .id();
    let mut evidence = browser::PreviewEvidence::default();
    let mut details = Vec::new();
    let first = v.scene.len();
    for p in prepare(preview, false, &mut evidence, &mut details)? {
        let (mesh, material, image) = upload(p, meshes, materials, images);
        let entity = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                ChildOf(figure),
            ))
            .id();
        v.scene.push((entity, mesh, material, image));
    }
    let level = normalize.inverse().transform_point3(feet);
    let ([x0, x1], top, low) = figure_frac(rig, 0., fitted, bounds);
    let ([o0, o1], o_top, o_low) = figure_frac(rig, 1., back, bounds);
    let mut line = format!(
        "Figure fitted to the RPCS3 closed view at model scale: feet at level ({:.0}, {:.0}, {:.0}); closed x {x0:.2}..{x1:.2} w, {top:.2}..{low:.2} h; open (from behind) x {o0:.2}..{o1:.2} w, centre {:.2} h (RPCS3 {:.2} h)",
        level.x,
        level.y,
        level.z,
        (o_top + o_low) / 2.,
        pod_menu::SACKBOY_OPEN[1]
    );
    line += &match checkpoint {
        Some((thing, w)) => format!(
            "; shell checkpoint Thing {thing} at ({:.0}, {:.0}, {:.0})",
            w[12], w[13], w[14]
        ),
        None => "; no shell checkpoint".into(),
    };
    line += &match &data {
        None => String::new(),
        Some(Err(e)) => format!("; data spawn not found: {e}"),
        Some(Ok((f, m, l))) => {
            let ([d0, d1], d_top, d_low) = figure_frac(rig, 0., *m, bounds);
            format!(
                "; {l}: feet at level ({:.0}, {:.0}, {:.0}), closed x {d0:.2}..{d1:.2} w, {d_top:.2}..{d_low:.2} h{}",
                f.x,
                f.y,
                f.z,
                if v.level.figure_data { " (used)" } else { " (not used; --pod-figure-data)" }
            )
        }
    };
    let hold = match hold {
        None => None,
        Some(Err(e)) => {
            line += &format!("; hold animation not played: {e}");
            None
        }
        Some(Ok(clip)) => Some(clip),
    };
    let scene: Vec<_> = v.scene[first..]
        .iter()
        .map(|s| (s.0, s.1.clone()))
        .collect();
    match clip {
        None => {}
        Some(Err(e)) => line += &format!("; animation not played: {e}"),
        Some(Ok(clip)) => {
            // Raw model units: the identity display frame.
            let frame = ([0.; 3], 1.);
            let gpu = if v.cpu_skinning {
                None
            } else {
                let rig = gpu_rig(
                    commands,
                    meshes,
                    bindposes,
                    &clip.rig,
                    (&clip.skin.joints, &clip.skin.weights),
                    (frame, Quat::IDENTITY),
                    &scene,
                )?;
                commands.entity(rig.root).insert(ChildOf(figure));
                Some(GpuRig {
                    morphs: !clip.skin.morphs.is_empty() && clip.report.animation_morphs > 0,
                    ..rig
                })
            };
            line += &format!("; animation {} frames", clip.report.frames);
            if let Some(h) = &hold {
                line += &format!("; hold animation {} frames", h.report.frames);
            }
            v.playing = Some(Playing {
                clip,
                frame,
                started: -1.,
                shown: None,
                gpu,
                meshes: scene.into_iter().map(|s| s.1).collect(),
                alt: hold,
                blended: None,
                layered: false,
            });
        }
    }
    Ok(line)
}
/// Where a figure with raw `bounds`, placed by `place` in shell display space, appears at
/// Puter progress `t`: x range, head top and feet as 16:9 frame fractions.
fn figure_frac(
    rig: &pod_menu::PodRig,
    t: f32,
    place: Mat4,
    [lo, hi]: [[f32; 3]; 2],
) -> ([f32; 2], f32, f32) {
    let (mut x, mut top, mut feet) = ([f32::MAX, f32::MIN], f32::MAX, f32::MIN);
    for c in 0..8 {
        let p = Vec3::new(
            if c & 1 == 0 { lo[0] } else { hi[0] },
            if c & 2 == 0 { lo[1] } else { hi[1] },
            if c & 4 == 0 { lo[2] } else { hi[2] },
        );
        let [fx, fy] = rig.shell_frac(t, place.transform_point3(p));
        x = [x[0].min(fx), x[1].max(fx)];
        top = top.min(fy);
        feet = feet.max(fy);
    }
    (x, top, feet)
}
/// P2d: the Pod figure's two placements; `pod_figure` switches at half-open.
#[derive(Component)]
struct PodFigure {
    front: Transform,
    back: Transform,
    holding: bool,
}
/// P2d: past half-open the figure turns its back (holding the controller) and the hold
/// clip, when given, replaces the standing one; closing reverses both. The game's switch
/// point during the dolly is UNKNOWN (RPCS3 shows only the end states).
fn pod_figure(
    state: Option<Res<pod_menu::PuterState>>,
    mut v: ResMut<Viewer>,
    mut figures: Query<(&mut PodFigure, &mut Transform)>,
) {
    let Some(state) = state else { return };
    let holding = state.t > 0.5;
    for (mut figure, mut transform) in &mut figures {
        if figure.holding == holding {
            continue;
        }
        figure.holding = holding;
        *transform = if holding { figure.back } else { figure.front };
        if let Some(playing) = v.playing.as_mut() {
            if let Some(mut other) = playing.alt.take() {
                std::mem::swap(&mut playing.clip, &mut other);
                playing.alt = Some(other);
                playing.started = -1.;
                playing.shown = None;
            }
        }
    }
}
/// Raw axis-aligned bounds; None when empty or nonfinite.
fn adapter_bounds(positions: &[[f32; 3]]) -> Option<[[f32; 3]; 2]> {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in positions {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    lo.iter()
        .chain(&hi)
        .all(|v| v.is_finite())
        .then_some([lo, hi])
}
#[derive(Component)]
struct PodShellRoot;
/// P2g: the root carrying the tent mesh (pod.mol), placed by `PodRig::tent_root`.
#[derive(Component)]
struct PodTentRoot;
/// pod.mol, the Pod tent (shell level 000027b9 Thing 2).
const POD_TENT_MESH: u32 = 0x0000_2f6c;
/// Runs before `load`: a new load request drops the previous Pod shell (and its figure).
fn clear_pod_shell(mut commands: Commands, v: Res<Viewer>, roots: Query<Entity, PodRoots>) {
    if v.requested {
        for root in &roots {
            commands.entity(root).despawn();
        }
        // D277: the shell's Thing map goes with its entities.
        commands.remove_resource::<thing_view::ShellThings>();
        // D278: and the dissolve's mask.
        commands.remove_resource::<PodDissolveMask>();
    }
}
/// D278: the decoded transition mask as a data texture (linear, repeating, with mips).
/// D284: and `pod_mask` (linear, clamped, with mips: it is sampled at plain UV0 inside
/// the unit square).
fn dissolve_mask_image(
    mask: Result<lbp_texture::Texture, String>,
    island: Option<Result<lbp_texture::Texture, String>>,
    images: &mut Assets<Image>,
) -> PodDissolveMask {
    let mut load = |texture: Result<lbp_texture::Texture, String>, repeat: bool, without: &str| {
        let image = texture.and_then(|t| {
            let size = (t.header.width, t.header.height);
            let wrap = lbp_preview::TextureWrapping::uniform(repeat);
            adapter::linear_image(size.0, size.1, t.rgba, wrap).map(|image| (image, size))
        });
        match image {
            Ok((image, (width, height))) => (
                Some(images.add(image)),
                format!("loaded, {width} x {height}"),
            ),
            Err(e) => (None, format!("not loaded ({e}); {without}")),
        }
    };
    let (image, note) = load(mask, true, "the band under the line is not tinted");
    let island = island.unwrap_or_else(|| Err("not asked for".into()));
    let (island, island_note) = load(island, false, "no part of the tent stands higher");
    PodDissolveMask {
        image,
        note,
        island,
        island_note,
    }
}
/// The Pod rig, and (D275) whether the planets level's scripts run: the camera is then
/// placed by `level_play::script_camera`, and (D280) the shell and tent roots stand in
/// the shell level's own frame (`level_play::shell_frame_root`).
#[derive(Resource)]
struct PodRigRes(pod_menu::PodRig, bool);
/// Shell, tent and (P2k) planets roots.
type PodRoots = Or<(With<PodShellRoot>, With<PodPlanetsRoot>)>;
/// P2b/P2k: the shell and tent roots, the planets root and the camera projection, for
/// `pod_dolly`.
type PodLayers<'w, 's> = (
    Query<'w, 's, (&'static mut Transform, Has<PodTentRoot>), With<PodShellRoot>>,
    Query<'w, 's, (&'static mut Transform, &'static PodPlanetsRoot), Without<PodShellRoot>>,
    Query<'w, 's, &'static mut Projection, With<Camera3d>>,
);
/// The planets view at `t` as an orbit: from the planets eye along its turn, target at
/// the rig's focus distance (P2k: a turned camera gives yaw and pitch).
fn pod_orbit(rig: &pod_menu::PodRig, t: f32) -> Orbit {
    let eye = rig.planet_eye(t);
    let back = rig.planet_turn(t) * Vec3::Z;
    Orbit {
        yaw: back.x.atan2(back.z),
        pitch: back.y.clamp(-1., 1.).asin(),
        distance: rig.focus,
        target: eye - back * rig.focus,
    }
}
/// P2k: root of the planets level under `--pod-game-camera`; holds the level's display
/// normalization (level units to display units). D272: also the root that follows the
/// world Thing under `--level-scripts` (with `level_play::WorldThingRoot`).
#[derive(Component)]
pub(crate) struct PodPlanetsRoot(pub(crate) Mat4);
/// P2k: the World transform at `t` in display units: `N W(t) N^-1`.
fn planets_root(normalize: Mat4, t: f32) -> Transform {
    Transform::from_matrix(normalize * pod_menu::planets_world_at(t) * normalize.inverse())
}
/// P2k: moves the Earth (Thing `globe`) and the Moon to their main-menu placement in World
/// space: each Thing's items are scaled about the Thing's first model item origin to the menu
/// scale and moved to the menu position (rotation kept; the game also spins the Earth).
fn place_menu_planets(scene: &mut LevelScene, globe: usize) -> Result<String, String> {
    let mut parts = Vec::new();
    for (thing, (to, scale)) in [
        (globe, pod_menu::EARTH_MENU),
        (pod_menu::MOON_THING, pod_menu::MOON_MENU),
    ] {
        // The pivot is the Thing's first model item (decal and shape batches carry
        // identity matrices).
        let model = |i: &&level_scene::SceneItem| {
            i.thing == thing
                && matches!(
                    scene.meshes.get(i.mesh).map(|m| &m.payload),
                    Some(level_scene::ScenePayload::Model(_))
                )
        };
        let Some(first) = scene.items.iter().find(model) else {
            parts.push(format!("Thing {thing} not drawn"));
            continue;
        };
        let world = Mat4::from_cols_array(&first.world);
        let from = world.w_axis.truncate();
        let stored = world.x_axis.truncate().length();
        if !(stored.is_finite() && stored > 1e-6 && from.is_finite()) {
            return Err(format!(
                "--pod-game-camera: Thing {thing} has no usable matrix"
            ));
        }
        let moved = Mat4::from_translation(to)
            * Mat4::from_scale(Vec3::splat(scale / stored))
            * Mat4::from_translation(-from);
        let mut n = 0;
        for item in scene.items.iter_mut().filter(|i| i.thing == thing) {
            item.world = (moved * Mat4::from_cols_array(&item.world)).to_cols_array();
            n += 1;
        }
        parts.push(format!(
            "Thing {thing}: {n} items from ({:.0}, {:.0}, {:.0}) item scale {stored:.3} to ({:.0}, {:.0}, {:.0}) scale {scale}",
            from.x, from.y, from.z, to.x, to.y, to.z
        ));
    }
    Ok(format!(
        "Game camera (World, presets, vfov 57.3 deg; menu placement): {}",
        parts.join("; ")
    ))
}
/// P2b: Space opens or closes the Puter: the view dollies through the Pod window in
/// `DOLLY_SECONDS` while the shell layer follows its own camera.
#[allow(clippy::too_many_arguments)]
fn pod_dolly(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    rig: Option<Res<PodRigRes>>,
    state: Option<ResMut<pod_menu::PuterState>>,
    mut orbit: ResMut<Orbit>,
    mut layers: PodLayers,
    own_sackboy: Option<Res<sackboy_play::PodSackboy>>,
) {
    let (Some(rig), Some(mut state)) = (rig, state) else {
        return;
    };
    // D275: under `--level-scripts` the script's camera gives the field of view and the
    // view (`level_play::script_camera`); D280: the roots stay in the shell level's own
    // frame. D277: `t` is not
    // moved here either: it is written from pod.bin's change-over value after each tick
    // (`level_play::tick`). D279: the keys do not set `open` there either: the key that
    // went down waits in `key` for the scripts' next step, which presses the engage
    // button on the pad the scripts read (or asks the host to close), and `open` follows
    // the script world's "pod computer in use".
    let scripted = rig.1;
    // P2k: the camera's field of view follows the rig (the game's under --pod-game-camera).
    for mut projection in &mut layers.2 {
        if let Projection::Perspective(p) = &mut *projection {
            if p.fov != rig.0.fov && !scripted {
                p.fov = rig.0.fov;
                println!("Pod camera: vertical FOV {:.1} deg", p.fov.to_degrees());
            }
        }
    }
    let pad = |b| pads.iter().any(|g| g.just_pressed(b));
    // D281: with the viewer's own Sackboy in the Pod Space is his jump; the engage button
    // is the play mode's Square key, Z.
    let engage_key = match own_sackboy {
        Some(_) => KeyCode::KeyZ,
        None => KeyCode::Space,
    };
    let toggle = keys.just_pressed(engage_key) || pad(GamepadButton::West);
    let back =
        keys.any_just_pressed([KeyCode::Escape, KeyCode::Backspace]) || pad(GamepadButton::East);
    if scripted {
        if let Some(key) = pod_menu::puter_key(toggle, back) {
            state.key = Some(key);
        }
        return;
    }
    if toggle {
        state.open = !state.open;
        state.hold = false;
    }
    if back {
        state.open = false;
        state.hold = false;
    }
    if state.hold {
        return;
    }
    let target = if state.open { 1. } else { 0. };
    if state.t == target {
        return;
    }
    let step = time.delta_secs() / pod_menu::DOLLY_SECONDS;
    state.t = if state.open {
        (state.t + step).min(1.)
    } else {
        (state.t - step).max(0.)
    };
    *orbit = pod_orbit(&rig.0, state.t);
    for (mut root, PodPlanetsRoot(normalize)) in &mut layers.1 {
        *root = planets_root(*normalize, state.t);
    }
    for (mut root, tent) in &mut layers.0 {
        *root = if tent {
            rig.0.tent_root(state.t)
        } else {
            rig.0.shell_root(state.t)
        };
    }
}
/// `follow` (D268): the scene is the level a script world plays; its Things' own models
/// are tagged and listed in a `thing_view::LevelThings` map, which is returned (D277:
/// the caller keeps it as the played level's map or as the shell layer's).
fn spawn_level(
    v: &mut Viewer,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ViewerMaterial>,
    images: &mut Assets<Image>,
    mut plan: LevelPlan,
    follow: bool,
) -> Option<thing_view::LevelThings> {
    // One set of GPU assets per scene mesh, shared by every instance.
    let uploaded: Vec<Vec<_>> = plan
        .prepared
        .into_iter()
        .map(|p| {
            p.map(|list| {
                list.into_iter()
                    .map(|p| {
                        let guid = p.material;
                        let (mesh, material, image) = upload(p, meshes, materials, images);
                        (mesh, material, image, guid)
                    })
                    .collect()
            })
            .unwrap_or_default()
        })
        .collect();
    let mut things = follow.then(|| thing_view::LevelThings::new(plan.normalize));
    for (n, (mesh_index, transform)) in plan.placed.into_iter().enumerate() {
        let item = plan.things.get(n).copied().flatten();
        let hidden = plan.hidden.get(n).copied().unwrap_or(false);
        for (mesh, material, image, guid) in &uploaded[mesh_index] {
            let entity = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    transform,
                ))
                .id();
            if hidden {
                // D276: the other mesh of a swapped pair waits for its script.
                commands.entity(entity).insert(Visibility::Hidden);
            }
            if let (Some(things), Some(item)) = (things.as_mut(), item) {
                commands.entity(entity).insert(thing_view::LevelThing {
                    item,
                    scene_row: v.scene.len(),
                    own_material: false,
                    dissolve: None,
                    material: *guid,
                });
                things.add(item.thing, entity);
            }
            v.scene
                .push((entity, mesh.clone(), material.clone(), image.clone()));
        }
    }
    if let Some(things) = &things {
        let (mapped, entities) = things.counts();
        plan.lines.push(format!(
            "Thing map (level scripts): {mapped} Things with a model of their own, {entities} draw entities"
        ));
    }
    v.level.lines = plan.lines;
    v.level.home = Some(plan.home);
    v.lighting.light = plan.light;
    things
}
/// D286 (docs/level-scripts.md, part 37): `window_glassy`, the gfx material of the Pod's
/// side panels (pod.mol primitive 2). P2j's default wall art on it (0001f3b9, D184) is
/// gone: the game draws it as clear glass over the room. The rule is this GUID only
/// (the material's alpha mode 4 was measured on this one draw, so it is not used).
const WINDOW_GLASSY: u32 = 0x0001_d14c;
/// D286: the glass's output alpha (MEASURED, fc32 of the captured draw): the colour is
/// premultiplied by it and the room behind is kept by one minus it.
const GLASS_OPACITY: f32 = 0.2;
/// D286: the constant the glass's shader takes in place of the surface green in the
/// dissolve's edge (FOUND, 30 / 255).
const GLASS_DISSOLVE_GREEN: f32 = 0.117_65;
/// D287 (part 38): the scale of the glass's specular, added to its output after the
/// alpha (MEASURED, fc45 of the captured draw; the gloss colour 0.6 is fc30).
const GLASS_SPECULAR_SCALE: f32 = 6.0;
/// D286: raw UV0 from a bump map's coordinates when they are UV0 through a scale and
/// offset with no zero scale (the glass's tissue paper is UV0 x 2, FOUND); none
/// otherwise. The inverse of the forward map, so exact for a power-of-two scale.
fn raw_uv0_from_bump(
    uv_channel: usize,
    transform: lbp_preview::SamplingTransform,
    sampling_uv: &[[f32; 2]],
) -> Option<Vec<[f32; 2]>> {
    let lbp_preview::SamplingTransform::ScaleOffset(t) = transform else {
        return None;
    };
    let [sx, sy, ox, oy] = t.values();
    (uv_channel == 0 && sx != 0. && sy != 0.).then(|| {
        sampling_uv
            .iter()
            .map(|&[u, v]| [(u - ox) / sx, (v - oy) / sy])
            .collect()
    })
}
/// P2e: one `--runtime-texture` value; repeatable, conflicts and the rule cap refused.
fn push_runtime_texture(
    rules: &mut Vec<lbp_preview::runtime_texture::RuntimeTexture>,
    value: &str,
) -> Result<(), String> {
    let rule = lbp_preview::runtime_texture::RuntimeTexture::parse(value)?;
    if !rules.contains(&rule) {
        rules.push(rule);
    }
    lbp_preview::runtime_texture::validate(rules).inspect_err(|_| {
        rules.pop();
    })
}
/// `--level-report`: builds the same scene and display plan without a window.
fn level_report(v: &Viewer) -> Result<(), String> {
    let (snapshot, game) = (PathBuf::from(&v.snapshot), PathBuf::from(&v.game));
    let (level_guid, level_path) = (v.level.guid, v.level.path.clone());
    let placement = v.level.placement;
    let focus = v.level.focus;
    let pod = v.level.pod;
    let options = v.preview_options();
    let lighting_on = v.lighting.enabled;
    let surface_on = v.bump_gloss_on();
    let lbp_on = v.lighting.lbp;
    let scene_options = level_scene::LevelSceneOptions {
        shape_bevels: v.level.shape_bevels,
        placement,
        runtime_textures: v.level.runtime_textures.clone(),
        per_bone: v.level.per_bone_on(),
        defaults: v.level_defaults(),
        mesh_alternates: swapped_meshes(v.level.scripts),
    };
    let lines = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<Vec<String>, String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(&snapshot, &control)?;
            let source = AttachedSource::attach(&game, &catalog, &control)?;
            let (key, identity) =
                level_scene::resolve_level(&catalog, level_guid, level_path.as_deref())?;
            let scene = level_scene::load_level_scene_with_options(
                &source,
                &catalog,
                &key,
                &control,
                options,
                scene_options,
            )?;
            let plan = plan_level(&scene, focus, pod)?;
            let mut lines = vec![format!("Level: {identity}")];
            if lighting_on {
                // Same per-entity rule as `apply_lighting`, without a GPU.
                let (mut unlit, mut total) = (0, 0);
                for (mesh, _) in &plan.placed {
                    for p in plan.prepared[*mesh].iter().flatten() {
                        total += 1;
                        unlit += usize::from(!p.normals_trusted);
                    }
                }
                let light = lighting::PreviewLight {
                    lbp: lbp_on,
                    ..plan.light.clone()
                };
                lines.extend(lighting::summary(&light, unlit, total));
            }
            if surface_on {
                // P2q: displayed draws (entity × bucket) carrying each surface extra.
                let (mut bump, mut gloss, mut alpha, mut glow, mut total) = (0, 0, 0, 0, 0);
                for (mesh, _) in &plan.placed {
                    for p in plan.prepared[*mesh].iter().flatten() {
                        total += 1;
                        bump += usize::from(p.bump.is_some());
                        gloss += usize::from(p.gloss.is_some());
                        alpha += usize::from(p.alpha_test > 0.);
                        glow += usize::from(p.glow.is_some());
                    }
                }
                lines.push(format!(
                    "P2q surface: {bump} of {total} displayed draws bump-mapped, {gloss} glossy, {alpha} alpha-tested, {glow} glowing (P3a/P3h; drawn only under --lbp-lighting)"
                ));
            }
            lines.extend(plan.lines);
            for m in &scene.meshes {
                let users = scene
                    .items
                    .iter()
                    .filter(|i| std::ptr::eq(&scene.meshes[i.mesh], m))
                    .count();
                lines.push(format!(
                    "  mesh {}: {users} instances{}",
                    m.source.label(),
                    if m.neutral_groups {
                        ", neutral groups"
                    } else {
                        ""
                    }
                ));
                for (reason, groups) in level_scene::neutral_reasons(&m.payload) {
                    lines.push(format!("    neutral ({groups} groups): {reason}"));
                }
                // Which shapes a batch merges (reader order), to tie it to a visible object.
                if let level_scene::ScenePayload::Shapes { batch, .. } = &m.payload {
                    const SHOWN: usize = 16;
                    let shown: Vec<String> = batch
                        .things
                        .iter()
                        .take(SHOWN)
                        .map(ToString::to_string)
                        .collect();
                    let more = batch.things.len().saturating_sub(SHOWN);
                    lines.push(format!(
                        "    shape Things ({}): {}{}",
                        batch.things.len(),
                        shown.join(", "),
                        if more > 0 {
                            format!(" (+{more} more)")
                        } else {
                            String::new()
                        }
                    ));
                }
            }
            Ok(lines)
        })
        .map_err(|e| format!("level worker thread: {e}"))?
        .join()
        .map_err(|_| "level worker panicked".to_string())??;
    for line in lines {
        println!("{line}");
    }
    Ok(())
}
/// Spawns the joint hierarchy and turns every scene mesh into a skinned mesh.
fn gpu_rig(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    rig: &lbp_preview::skinned::Rig,
    (skin_joints, skin_weights): (&[[u8; 4]], &[[f32; 4]]),
    ((centre, span), up): (adapter::DisplayFrame, Quat),
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
    // Skinned vertices ignore the mesh entity transform, so the D148 model-up rotation
    // goes on the root: skin = U x N x world x inverse bind x N^-1 on display vertices.
    let root = commands
        .spawn((
            Transform::from_matrix(Mat4::from_quat(up) * n),
            Visibility::default(),
        ))
        .id();
    let inherits = rig.joint_inherits();
    let split = inherits.iter().any(|i| !i);
    let local: Vec<_> = if split {
        rig.joint_locals(&rig.pose_locals(None))
    } else {
        let bind = rig.local_matrices(None);
        bind.into_iter().map(|m| (m, [1.; 3])).collect()
    };
    let mut joints: Vec<Entity> = local
        .iter()
        .map(|(m, _)| {
            commands
                .spawn(Transform::from_matrix(Mat4::from_cols_array(m)))
                .id()
        })
        .collect();
    // Scale nodes only for a rig whose bones do not all inherit scale (D246).
    let mut scales: Vec<Entity> = if split {
        local
            .iter()
            .zip(&joints)
            .map(|((_, k), joint)| {
                commands
                    .spawn((Transform::from_scale(Vec3::from_array(*k)), ChildOf(*joint)))
                    .id()
            })
            .collect()
    } else {
        Vec::new()
    };
    for (i, parent) in rig.parents().into_iter().enumerate() {
        let parent = usize::try_from(parent).map_or(root, |p| {
            if split && inherits[i] {
                scales[p]
            } else {
                joints[p]
            }
        });
        commands.entity(joints[i]).insert(ChildOf(parent));
    }
    let fixed = commands.spawn((Transform::IDENTITY, ChildOf(root))).id();
    joints.push(fixed);
    let skin_joints_entities = if split {
        let mut skin = scales.clone();
        skin.push(fixed);
        skin
    } else {
        joints.clone()
    };
    scales.shrink_to_fit();
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
                joints: skin_joints_entities.clone(),
            },
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    Ok(GpuRig {
        root,
        joints,
        scales,
        morphs: false,
    })
}
/// Writes one pose (a local transform per bone) into the joint and scale nodes of a rig
/// built with scale nodes (`GpuRig::scales`, D246).
fn pose_joints(
    gpu: &GpuRig,
    rig: &lbp_preview::skinned::Rig,
    local: &[lbp_preview::skinned::Trs],
    transforms: &mut Query<&mut Transform>,
) {
    for (i, (m, k)) in rig.joint_locals(local).into_iter().enumerate() {
        if let Some(mut t) = gpu.joints.get(i).and_then(|j| transforms.get_mut(*j).ok()) {
            *t = Transform::from_matrix(Mat4::from_cols_array(&m));
        }
        if let Some(mut t) = gpu.scales.get(i).and_then(|j| transforms.get_mut(*j).ok()) {
            *t = Transform::from_scale(Vec3::from_array(k));
        }
    }
}
fn animate(
    time: Res<Time>,
    mut v: ResMut<Viewer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut transforms: Query<&mut Transform>,
) {
    let held = v.animation_frame;
    let Some(playing) = v.playing.as_mut() else {
        return;
    };
    if playing.layered {
        // D232: a blended pose from the play figure's layers, once per new pose.
        let Some(pose) = playing.blended.take() else {
            return;
        };
        let posed = if let Some(gpu) = &playing.gpu {
            if gpu.scales.is_empty() {
                for (joint, m) in gpu.joints.iter().zip(pose.local_matrices()) {
                    if let Ok(mut t) = transforms.get_mut(*joint) {
                        *t = Transform::from_matrix(Mat4::from_cols_array(&m));
                    }
                }
            } else {
                pose_joints(gpu, &playing.clip.rig, &pose.bones, &mut transforms);
            }
            if !gpu.morphs {
                return;
            }
            playing.clip.morphed_bind_weights(&pose.morphs)
        } else {
            playing.clip.blended_positions(&pose)
        };
        let display = adapter::to_display(&posed, playing.frame);
        for handle in &playing.meshes {
            if let Some(mut mesh) = meshes.get_mut(handle) {
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, display.clone());
            }
        }
        return;
    }
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
        let rig = &playing.clip.rig;
        if gpu.scales.is_empty() {
            for (joint, m) in gpu.joints.iter().zip(rig.local_matrices(Some(&pose))) {
                if let Ok(mut t) = transforms.get_mut(*joint) {
                    *t = Transform::from_matrix(Mat4::from_cols_array(&m));
                }
            }
        } else {
            pose_joints(gpu, rig, &rig.pose_locals(Some(&pose)), &mut transforms);
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
    for handle in &playing.meshes {
        if let Some(mut mesh) = meshes.get_mut(handle) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, display.clone());
        }
    }
}
/// D121: pushes the preview light (or its removal) into every scene material and the
/// camera fog once per toggle or scene change. Off and never applied: touches nothing,
/// so default captures are unchanged.
/// P2l: the planets root and the shell root (not the tent), for turning each layer's light.
/// D314: the planets root says whether it hangs under the world Thing (`planets_light_turn`).
type LightRoots<'w, 's> = (
    Query<'w, 's, (&'static Transform, Has<level_play::WorldThingRoot>), With<PodPlanetsRoot>>,
    Query<'w, 's, &'static Transform, (With<PodShellRoot>, Without<PodTentRoot>)>,
);
/// P3g: the sun's shadow-map edge. The game renders three 768x768 sun-shadow maps (Moon
/// capture); Bevy wants a power of two for stable cascades, so 1024 stands in (INFERRED).
const SHADOW_MAP_SIZE: usize = 1024;
/// P3g: the one directional light that exists only to feed the cascaded shadow map (the
/// viewer shader does its own lighting). Holds the display scale its cascades were cut for.
#[derive(Component)]
struct SunShadow(f32);
/// The sun's `&mut Transform` must not overlap the `LightRoots` queries of `apply_lighting`
/// (Bevy B0001 at start-up otherwise).
type SunFilter = (Without<PodPlanetsRoot>, Without<PodShellRoot>);
/// P3g: far bounds of the three cascades, in level units along the view axis. The first
/// covers the gameplay view (about 1,200 units of camera distance), INFERRED from the
/// game's three maps; the cut is a preview choice.
const SHADOW_CASCADE_UNITS: [f32; 2] = [3500., 40000.];
fn sync_sun_shadow(
    commands: &mut Commands,
    suns: &mut Query<(Entity, &SunShadow, &mut Transform), SunFilter>,
    light: Option<&lighting::PreviewLight>,
) {
    let want = light.filter(|l| l.shadows && l.toward_light.is_finite());
    let mut kept = false;
    for (entity, cut, mut transform) in suns.iter_mut() {
        match want {
            Some(l) if !kept && (cut.0 - l.display_scale).abs() <= f32::EPSILON => {
                *transform = sun_transform(l.toward_light);
                kept = true;
            }
            _ => commands.entity(entity).despawn(),
        }
    }
    let Some(l) = want.filter(|_| !kept) else {
        return;
    };
    let s = l.display_scale;
    commands.spawn((
        SunShadow(s),
        DirectionalLight {
            // Near black for any StandardMaterial in the scene; the viewer shader ignores it.
            illuminance: 1.,
            shadow_maps_enabled: true,
            ..default()
        },
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 0.01,
            maximum_distance: SHADOW_CASCADE_UNITS[1] * s,
            first_cascade_far_bound: SHADOW_CASCADE_UNITS[0] * s,
            overlap_proportion: 0.2,
        }
        .build(),
        sun_transform(l.toward_light),
    ));
}
/// D314: the turn the planets level's light takes. A root that hangs under the world Thing
/// (`--level-scripts`, the script moving the world Thing each tick) leaves the light where
/// it is: the game turns the level's geometry with the world Thing but keeps the sun
/// (FragmentConstants sun position = the level's sun x 300000, unturned) in world space
/// (FOUND, RenderDoc capture 8980, the open room's draws 2400 and 2416: the turned normals
/// against the unturned sun give the game's N.L, the turned sun gave the viewer x1.07-1.09
/// light). Other roots (the dolly) keep turning the light with them (P2l).
fn planets_light_turn(root: Option<(&Transform, bool)>) -> Quat {
    match root {
        Some((t, false)) => t.rotation,
        _ => Quat::IDENTITY,
    }
}
/// D314: the planets level's sun as the game has it under the world Thing: a point at the
/// level's sun position x `sun_position_scale` (300,000 in pod_planets.bin's settings),
/// unturned, in display units (FOUND, captures 8980 and 5247: the programs take
/// normalize(sun - position) with no falloff). Elsewhere the direction stays.
fn planets_sun_point(light: &lighting::PreviewLight, world_thing: bool) -> Option<Vec3> {
    if world_thing && light.lbp {
        light.sun_display
    } else {
        None
    }
}
fn sun_transform(toward_light: Vec3) -> Transform {
    let up = if toward_light.y.abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    Transform::default().looking_to(-toward_light, up)
}
fn apply_lighting(
    mut commands: Commands,
    mut suns: Query<(Entity, &SunShadow, &mut Transform), SunFilter>,
    mut v: ResMut<Viewer>,
    mut materials: ResMut<Assets<ViewerMaterial>>,
    cameras: Query<Entity, With<Camera3d>>,
    roots: LightRoots,
    puter: Option<Res<pod_menu::PuterState>>,
) {
    // P2l: the LBP light lives in each level's frame, so it turns with the planets and
    // shell roots (re-applied when the dolly turns them).
    let first_turn = |t: Option<&Transform>| t.map_or(Quat::IDENTITY, |t| t.rotation);
    let (turn, shell_turn) = (
        planets_light_turn(roots.0.iter().next()),
        first_turn(roots.1.iter().next()),
    );
    let pod_t = puter.map_or(v.lighting.pod_t, |p| p.t);
    let world_thing = roots.0.iter().next().is_some_and(|(_, w)| w);
    let turned = !v.lighting.dirty
        && v.lighting.lbp
        && v.lighting.applied
        && (turn.angle_between(v.lighting.turn) > 1e-3
            || shell_turn.angle_between(v.lighting.shell_turn) > 1e-3
            || (pod_t - v.lighting.pod_t).abs() > 1e-3);
    if !(v.lighting.dirty || turned) || v.job.is_some() {
        return;
    }
    v.lighting.turn = turn;
    v.lighting.shell_turn = shell_turn;
    v.lighting.pod_t = pod_t;
    v.lighting.dirty = false;
    let on = v.lighting.enabled;
    if !on && !v.lighting.applied {
        return;
    }
    // The Pod (shell present) draws everything with fragment variant 8: no shadow, planets
    // included (CONFIRMED, 161 of 162 main-pass draws); only plain levels get the sun map.
    let no_shadows = v.no_shadows || v.lighting.shell.is_some();
    // P2x: Pod scenes (shell layer, game camera, globe view) keep the live-game fog constants.
    let pod_fog = v.lighting.shell.is_some() || v.level.game_camera || v.level.pod.is_some();
    let light = on.then(|| {
        if v.lighting.lbp {
            let mut l = v.lighting.light.lbp_shaped(turn);
            l.pod_t = pod_t;
            l.sun_point = planets_sun_point(&l, world_thing);
            l.shadows &= !no_shadows;
            l.pod_fog = pod_fog;
            l
        } else {
            v.lighting.light.clone()
        }
    });
    let shell = v.lighting.shell.as_ref().map(|(from, l)| {
        let l = if v.lighting.lbp {
            let mut l = l.lbp_shaped(shell_turn);
            l.pod_t = pod_t;
            // The Pod shell draws with fragment variant 8: no shadow (CONFIRMED).
            l.shadows = false;
            l.pod_fog = pod_fog;
            l
        } else {
            l.clone()
        };
        (*from, on.then_some(l))
    });
    sync_sun_shadow(&mut commands, &mut suns, light.as_ref());
    let mut unlit = 0usize;
    for (i, (_, _, handle, _)) in v.scene.iter().enumerate() {
        let layer = match &shell {
            Some((from, l)) if i >= *from => l.as_ref(),
            _ => light.as_ref(),
        };
        // Counts draws without trusted normals: unlit, or flat-lit in LBP mode (P2s).
        let smooth = materials
            .get_mut(handle.id())
            .is_some_and(|mut m| dual::set_lighting(&mut m, layer) && m.extension.normals_trusted);
        unlit += usize::from(!smooth);
    }
    for camera in &cameras {
        // P2t: LBP mode fogs in the shader (game output space), not with Bevy's fog.
        // D274: a played level's scripts move the preview fog out by their fog bias.
        let bias = v.lighting.fog_bias;
        let preview = light.as_ref().filter(|l| !l.lbp);
        match preview.and_then(|l| Some(l.fog?.biased(bias, l.display_scale))) {
            Some(fog) => {
                commands.entity(camera).insert(bevy::pbr::DistanceFog {
                    color: Color::linear_rgb(fog.color.x, fog.color.y, fog.color.z),
                    falloff: bevy::pbr::FogFalloff::Linear {
                        start: fog.start,
                        end: fog.end,
                    },
                    ..default()
                });
            }
            None => {
                commands.entity(camera).remove::<bevy::pbr::DistanceFog>();
            }
        }
    }
    v.lighting.applied = on;
    v.lighting.lines = match &light {
        Some(l) => lighting::summary(l, unlit, v.scene.len()),
        None => vec!["Lighting: off (unlit)".into()],
    };
    let bias = v.lighting.fog_bias;
    let biased = light.as_ref().filter(|l| !l.lbp && bias != 0.);
    if let Some((l, fog)) = biased.and_then(|l| Some((l, l.fog?))) {
        let scale = l.display_scale;
        let fog = fog.biased(bias, scale);
        v.lighting.lines.push(format!(
            "  fog moved out by the level scripts' fog bias {bias:.1}: linear {:.1}..{:.1} level units",
            fog.start / scale,
            fog.end / scale
        ));
    }
    if let Some((from, Some(l))) = &shell {
        v.lighting
            .lines
            .push(format!("  Pod shell (entities {from}..): own level light"));
        v.lighting
            .lines
            .extend(l.notes.iter().map(|n| format!("    {n}")));
    }
    if !turned {
        for line in &v.lighting.lines {
            println!("{line}");
        }
    }
}
fn panel(mut contexts: EguiContexts, mut v: ResMut<Viewer>, mut orbit: ResMut<Orbit>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if v.hide_ui {
        return Ok(());
    }
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
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut v.selector.levels_tab, false, "Assets");
                    ui.selectable_value(&mut v.selector.levels_tab, true, "Levels");
                });
                ui.separator();
                let context = v.context.clone();
                if v.selector.levels_tab {
                    ui.heading("Browse levels");
                    let current = v.level.path.clone();
                    if let Some(entry) = v.selector.panel(ui, context.as_ref(), current.as_deref())
                    {
                        v.select_level(&entry);
                    }
                } else if let Some((catalog, _)) = context {
                    ui.heading("Browse assets");
                    let selected = v.guid.clone();
                    if let Some(guid) = v.browser.panel(ui, &catalog, &selected) {
                        v.guid = guid;
                        v.level.guid = None;
                        v.level.path = None;
                        v.level.lines.clear();
                        v.level.home = None;
                        v.requested = true;
                    }
                } else {
                    ui.label("Attach a snapshot and game directory to browse.");
                }
            });
    }
    if let Some(job) = &v.job {
        let target = if v.level.active() {
            let label = v.level.label();
            let name = v.selector.name_of(&label).unwrap_or(&label).to_string();
            format!("Loading level {name}")
        } else {
            "Loading".to_string()
        };
        let detail = if v.context.is_none() {
            "reading snapshot and attaching game data"
        } else if v.level.active() {
            "reading level and building scene"
        } else {
            "reading and preparing preview"
        };
        level_selector::loading_overlay(ctx, &target, detail, job.started.elapsed());
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
            if ui.button("Reset").clicked() { *orbit = v.level.home.unwrap_or_default(); }
            if ui.button("Frame selected").clicked() {
                let home = v.level.home.unwrap_or_default();
                orbit.target = home.target;
                orbit.distance = home.distance;
            }
        });
        ui.separator();
        if !v.synthetic {
            if ui.checkbox(&mut v.animation_base,"Preview stored material animation base values").changed() {
                v.browser.clear_source();v.requested=true;
            }
            if v.animation_base {ui.small("Stored base rows matched to global declarations; no game time or motion inferred. Reload clears verification.");}
            if ui.checkbox(&mut v.animation_base_default,"Single mesh: animation base values by default (D188)").changed() {
                v.browser.clear_source();v.requested=true;
            }
            if ui.checkbox(&mut v.instance_default,"Single mesh: white instance color when none supplied (D188)").changed() {
                v.browser.clear_source();v.requested=true;
            }
            if v.instance_default && !v.instance_input_enabled {ui.small("Default white (1,1,1,1): placement default (local census + Toolkit); its link to THING_COLOR is inferred.");}
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
            if ui.checkbox(&mut v.lighting.enabled, "Preview lighting (not game lighting)").changed() {
                v.lighting.dirty = true;
            }
            if v.lighting.enabled {
                for line in &v.lighting.lines { ui.small(line); }
            }
            v.browser.review_panel(ui);
        }
        egui::ScrollArea::vertical().max_height(260.).show(ui, |ui| { ui.label(&v.status); });
        ui.separator();
        ui.label("Left drag: orbit\nRight / middle drag: pan\nWheel: zoom • R: reset • F: frame");
        if v.lighting.enabled {
            ui.small(format!("{} • opaque • sRGB • double-sided\nGenerated diagnostic normals. Animation: positions only, integer frames.\nTop mip only. Original game lighting, alpha and axis conventions unverified.", v.lighting.light.label()));
        } else {
            ui.small("Unlit • opaque • sRGB • double-sided\nGenerated diagnostic normals. Animation: positions only, integer frames.\nTop mip only. Original game lighting, alpha and axis conventions unverified.");
        }
    });
    if !v.synthetic || !v.census.is_empty() {
        census_window(ctx, &v.census);
    }
    if v.level.active() {
        egui::Window::new("Level scene counts")
            .default_pos([420., 460.])
            .default_width(420.)
            .show(ctx, |ui| {
                ui.label(format!("Level: {}", v.level.label()));
                for line in &v.level.lines {
                    ui.label(line);
                }
                if v.level.lines.is_empty() {
                    ui.small("Loading or failed; see status.");
                }
                ui.small("Models: static bind pose at each Thing's POS matrix; bones and animation not applied; decorations at POS x offset (inferred), stickers not drawn.");
                ui.small(format!("Shapes: {}.", level_scene::SHAPE_PREVIEW_LABEL));
                if v.level.shape_bevels {
                    ui.small(format!("Shape bevels on: {}.", level_scene::SHAPE_BEVEL_LABEL));
                }
            });
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
            *orbit = v.level.home.unwrap_or_default();
        }
        if keys.just_pressed(KeyCode::KeyF) {
            let home = v.level.home.unwrap_or_default();
            orbit.target = home.target;
            orbit.distance = home.distance;
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
        orbit.distance = (orbit.distance * (-scroll.delta.y * 0.12).exp()).clamp(0.01, 30.);
    }
    let rotation = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(-orbit.pitch);
    for mut transform in &mut cameras {
        *transform =
            Transform::from_translation(orbit.target + rotation * Vec3::Z * orbit.distance)
                .looking_at(orbit.target, Vec3::Y);
    }
}
/// QA: while true, `--qa-capture` waits (`--level-scripts-hold-at`: until the level's
/// scripts have reached the tick to capture).
#[derive(Resource)]
pub(crate) struct CaptureHold(pub(crate) bool);
fn capture(
    mut commands: Commands,
    mut v: ResMut<Viewer>,
    time: Res<Time>,
    hold: Option<Res<CaptureHold>>,
) {
    if v.job.is_none() && !v.requested {
        v.ready_frames += 1;
    }
    let held = hold.is_some_and(|h| h.0);
    if v.ready_frames > 120 && time.elapsed_secs() > 10. && !held {
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
        adapter::set_nearest(&mut image);
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
            surface: None,
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
            runtime_texture: None,
            fragment: None,
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
    #[test]
    fn runtime_texture_flag_is_repeatable_and_refuses_conflicts() {
        let mut rules = Vec::new();
        super::push_runtime_texture(&mut rules, "00002f6c:0:000027fb").unwrap();
        super::push_runtime_texture(&mut rules, "00002f6c:0:000027fb").unwrap();
        super::push_runtime_texture(&mut rules, "00002f6c:1:00004277").unwrap();
        assert_eq!(rules.len(), 2);
        assert!(super::push_runtime_texture(&mut rules, "00002f6c:0:00004277").is_err());
        assert!(super::push_runtime_texture(&mut rules, "00002f6c:0").is_err());
        assert_eq!(rules.len(), 2);
    }
    use super::*;

    /// D314: the planets root under the world Thing does not turn the light; the dolly's
    /// root (no world Thing) still does, and no root leaves it unturned.
    #[test]
    fn d314_world_thing_root_keeps_the_sun_in_world_space() {
        let turned =
            Transform::from_rotation(Quat::from_xyzw(0.1638, -0.0345, -0.0057, 0.9859).normalize());
        assert_eq!(planets_light_turn(Some((&turned, true))), Quat::IDENTITY);
        assert_eq!(planets_light_turn(Some((&turned, false))), turned.rotation);
        assert_eq!(planets_light_turn(None), Quat::IDENTITY);
    }

    /// D314 (RenderDoc capture 8980, the open room's mural, draw 2416): the room's world
    /// normal (median of the box) against the game's sun gives N.L 0.79; the viewer's normal
    /// there matches the game's, so the unturned sun lands within 0.03 of it, and the sun
    /// turned with the root at tick 616 (19 deg) gave 0.94, the x1.07-1.09 light measured.
    #[test]
    fn d314_open_room_sun_term_matches_the_capture() {
        let normal = Vec3::new(0.529, -0.224, 0.82).normalize();
        let toward = Vec3::new(0.5505, 0.3909, 0.7377).normalize();
        let turn = Quat::from_xyzw(0.1638, -0.0345, -0.0057, 0.9859).normalize();
        let world = Transform::from_rotation(turn);
        let kept = planets_light_turn(Some((&world, true))) * toward;
        assert!(
            (normal.dot(kept) - 0.79).abs() < 0.03,
            "{}",
            normal.dot(kept)
        );
        let old = planets_light_turn(Some((&world, false))) * toward;
        assert!(normal.dot(old) > 0.9, "{}", normal.dot(old));
    }

    /// D314: the sun as a point only on the LBP route under the world Thing; elsewhere the
    /// direction stays (no point).
    #[test]
    fn d314_point_sun_only_under_the_world_thing() {
        let mut l = lighting::PreviewLight::generic();
        l.sun_display = Some(Vec3::new(2.6, 1.9, 3.5));
        assert_eq!(planets_sun_point(&l, true), None, "preview route");
        l.lbp = true;
        assert_eq!(planets_sun_point(&l, true), l.sun_display);
        assert_eq!(planets_sun_point(&l, false), None);
        l.sun_display = None;
        assert_eq!(planets_sun_point(&l, true), None);
    }

    /// D314 (8980, the mural): the game's N.L comes from normalize(sun - position) with no
    /// falloff; with the box's median normal and position (camera-relative, as the
    /// capture's TC4 and fc0) the point gives 0.83 where the level-wide direction gives 0.81.
    #[test]
    fn d314_point_sun_direction_from_the_surface() {
        let normal = Vec3::new(0.52, -0.223, 0.825).normalize();
        let sun = Vec3::new(222_886.6, 160_217.0, 298_809.0);
        let at = Vec3::new(-22_112., 3_261., -27_701.);
        let point = normal.dot((sun - at).normalize());
        assert!((point - 0.827).abs() < 0.005, "{point}");
        let direction = normal.dot(Vec3::new(0.5505, 0.3909, 0.7377).normalize());
        assert!((direction - 0.808).abs() < 0.005, "{direction}");
    }

    /// CI never launches the viewer, so a Bevy B0001 query conflict in `apply_lighting`
    /// (P3g: the sun query overlapped the light-root queries) panicked every start-up.
    /// Initialising the system checks its parameter access without a window or a GPU.
    #[test]
    fn apply_lighting_parameters_do_not_conflict() {
        use bevy::ecs::system::{IntoSystem, System};
        let mut world = World::new();
        let mut system = IntoSystem::into_system(apply_lighting);
        system.initialize(&mut world);
    }
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
        // D246: the same with both bones compensating scale (no shear from the root's
        // non-uniform scale reaches the child), through the frame + scale node pairs.
        let mut biped = rig.clone();
        biped.set_biped_bones(&[true, true]);
        biped.scale_rule = lbp_preview::skinned::ScaleRule::Biped;
        let squash = lbp_preview::skinned::Pose {
            scales: vec![[0.8, 1., 1.2], [1.5, 0.5, 2.]],
            ..pose.clone()
        };
        let frame = ([1., 2., 3.], 4.);
        // Raw axes and the D148 Z-up rotation on the root.
        for (up, rig, pose) in [
            (model_up(false), &rig, &pose),
            (model_up(true), &rig, &pose),
            (model_up(true), &biped, &squash),
        ] {
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
                rig,
                (&[], &[]),
                (frame, up),
                &[],
            )
            .unwrap();
            queue.apply(world);
            assert_eq!(gpu.scales.len(), if rig == &biped { 2 } else { 0 });
            let local: Vec<_> = if gpu.scales.is_empty() {
                let plain = rig.local_matrices(Some(pose));
                plain.into_iter().map(|m| (m, [1.; 3])).collect()
            } else {
                rig.joint_locals(&rig.pose_locals(Some(pose)))
            };
            for (i, (m, k)) in local.into_iter().enumerate() {
                *world.get_mut::<Transform>(gpu.joints[i]).unwrap() =
                    Transform::from_matrix(Mat4::from_cols_array(&m));
                if let Some(scale) = gpu.scales.get(i) {
                    *world.get_mut::<Transform>(*scale).unwrap() =
                        Transform::from_scale(Vec3::from_array(k));
                }
            }
            app.update();
            let (_, inverse) = bindposes.iter().next().unwrap();
            let n = Mat4::from_scale(Vec3::splat(0.25))
                * Mat4::from_translation(-Vec3::new(1., 2., 3.));
            for (i, cpu) in rig.skin_matrices(Some(pose)).iter().enumerate() {
                let joint = gpu.scales.get(i).unwrap_or(&gpu.joints[i]);
                let global = app.world().get::<GlobalTransform>(*joint).unwrap();
                let got = global.to_matrix() * inverse[i];
                let want = Mat4::from_quat(up) * n * Mat4::from_cols_array(cpu) * n.inverse();
                assert!(got.abs_diff_eq(want, 1e-4), "{i}: {got} vs {want}");
            }
            // The static joint only applies the model-up rotation to display vertices.
            let fixed = app.world().get::<GlobalTransform>(gpu.joints[2]).unwrap();
            let u = Mat4::from_quat(up);
            assert!((fixed.to_matrix() * inverse[2]).abs_diff_eq(u, 1e-5));
        }
    }
    #[test]
    fn z_up_models_stand_upright_facing_the_default_camera() {
        // sack_boy.mol bind bones (model units): head top above toes, eyes toward -Y.
        let up = model_up(true);
        let (toe, head, eye) = (
            Vec3::new(29.9, -13., 0.3),
            Vec3::new(-0.1, 0.3, 108.1),
            Vec3::new(24., -34.2, 166.4),
        );
        assert!((up * head).y > (up * toe).y);
        assert!((up * eye).z > 0. && (up * eye).x > 0.);
        assert_eq!(model_up(false), Quat::IDENTITY);
        // Default camera: in front (+Z), a little above the centre, never below it.
        let o = Orbit::default();
        let eye_pos = Quat::from_rotation_y(o.yaw) * Quat::from_rotation_x(-o.pitch) * Vec3::Z;
        assert!(eye_pos.z > 0.9 && eye_pos.y > 0. && eye_pos.y * o.distance < 0.5);
    }
    #[test]
    fn qa_capture_and_no_activate_show_without_focus() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(activate_on_show_for(
            args(&["--select-guid", "43f"]).into_iter()
        ));
        assert!(!activate_on_show_for(
            args(&["--qa-capture", "local-analysis/x.png"]).into_iter()
        ));
        assert!(!activate_on_show_for(args(&["--no-activate"]).into_iter()));
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
    /// D277. The shell layer's scene loads the swapped meshes only when the shell has a
    /// script world: `--level-scripts` with `--pod-camera N` and `--pod-shell HEX`. Of
    /// the pair, the tent goes under the tent root and the platform under the shell root.
    #[test]
    fn d277_the_shell_layer_loads_the_swapped_meshes_only_with_a_script_world() {
        let mode = |scripts, pod, shell| LevelMode {
            scripts,
            pod,
            shell,
            ..Default::default()
        };
        let on = mode(true, Some(1), Some(0x27b9));
        assert!(on.pod_scripted() && on.shell_scripted());
        assert_eq!(
            swapped_meshes(on.shell_scripted()),
            thing_view::SWAPPED_MESHES
        );
        for off in [
            mode(false, Some(1), Some(0x27b9)),
            mode(true, None, Some(0x27b9)),
            mode(true, Some(1), None),
        ] {
            assert!(!off.shell_scripted());
            assert!(swapped_meshes(off.shell_scripted()).is_empty());
        }
        let [[tent, platform]] = thing_view::SWAPPED_MESHES;
        assert_eq!(tent, POD_TENT_MESH);
        assert_ne!(platform, POD_TENT_MESH);
    }
    /// P2j, D286: the shell adds no texture to the glass slot any more (P2j's default wall
    /// art is gone, with or without `--no-pod-wall`); an explicit rule for that slot is
    /// still taken as given.
    #[test]
    fn p2j_d286_the_shell_adds_no_wall_texture_but_keeps_an_explicit_rule() {
        use lbp_preview::runtime_texture::RuntimeTexture;
        let run = |extra: &[&str]| {
            let args = [
                "--snapshot",
                "s",
                "--game-dir",
                "g",
                "--level-guid",
                "00000001",
                "--pod-shell",
                "000027b9",
            ];
            options_from(args.iter().chain(extra).map(|s| s.to_string()))
                .unwrap()
                .level
                .runtime_textures
        };
        assert!(run(&[]).is_empty());
        assert!(run(&["--no-pod-wall"]).is_empty());
        assert_eq!(
            run(&["--runtime-texture", "00002f6c:0:0001f3b9"]),
            [RuntimeTexture {
                mesh: POD_TENT_MESH,
                slot: 0,
                texture: 0x0001_f3b9,
            }]
        );
    }
    /// D286: a bucket with no diffuse coordinates and its raw UV0 (as the glass is built).
    fn d286_bucket(raw_uv0: u8) -> Prepared {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        Prepared {
            mesh: adapter::mesh_local(&positions, &[[0, 1, 2]], None).unwrap(),
            image: None,
            second_image: None,
            third_image: None,
            fourth_image: None,
            color: Color::linear_rgba(0.118, 0.118, 0.118, 1.),
            alpha_mix: false,
            decal: false,
            normals_trusted: true,
            bump: None,
            gloss: None,
            spec_map: None,
            alpha_test: 0.,
            glow: None,
            glow_alpha: Default::default(),
            reflection: None,
            graph: None,
            lightmap: None,
            bucket: 0,
            material: Some(WINDOW_GLASSY),
            raw_uv0,
            glass: None,
        }
    }
    /// D286: the glass bucket draws premultiplied at opacity 0.2 with the dissolve's
    /// constant green; any other bucket stays opaque (a decal blended) and not glass.
    #[test]
    fn d286_the_glass_bucket_blends_premultiplied_at_0_2() {
        let mut plain = d286_bucket(0);
        assert_eq!(alpha_mode(&plain), AlphaMode::Opaque);
        plain.decal = true;
        assert_eq!(alpha_mode(&plain), AlphaMode::Blend);
        let mut glass = d286_bucket(0);
        mark_glass(&mut glass, None);
        assert_eq!(glass.glass, Some(0.2));
        assert_eq!(alpha_mode(&glass), AlphaMode::Premultiplied);
        let (mut meshes, mut materials, mut images) = (
            Assets::<Mesh>::default(),
            Assets::<ViewerMaterial>::default(),
            Assets::<Image>::default(),
        );
        let (_, material, _) = upload(glass, &mut meshes, &mut materials, &mut images);
        let material = materials.get(&material).unwrap();
        assert_eq!(material.base.alpha_mode, AlphaMode::Premultiplied);
        assert_eq!(dual::glass(material), Some(GLASS_OPACITY));
        let (_, material, _) = upload(d286_bucket(0), &mut meshes, &mut materials, &mut images);
        let material = materials.get(&material).unwrap();
        assert_eq!(material.base.alpha_mode, AlphaMode::Opaque);
        assert_eq!(dual::glass(material), None);
        assert!((GLASS_DISSOLVE_GREEN - 30. / 255.).abs() < 1e-5);
    }
    /// D288: two material buckets on the same triangles (the planets room's plain cloth,
    /// then its painted cloth): the later bucket is uploaded nearer by its index, so it
    /// wins the equal depth whatever order Bevy draws them in. Bucket 0 has no bias; a
    /// decal keeps its own and the glass none. Nothing here depends on the lighting route.
    #[test]
    fn d288_a_later_bucket_on_the_same_triangles_is_drawn_nearer() {
        let tri = [[0u32, 1, 2]];
        let plain = std::sync::Arc::new(1u8);
        let painted = std::sync::Arc::new(2u8);
        let bindings = [Ok(plain.clone()), Ok(painted.clone()), Ok(plain)];
        let triangles = [tri[0], tri[0], tri[0]];
        let buckets = adapter::buckets(&triangles, &[0..1, 1..2, 2..3], &bindings).unwrap();
        assert_eq!(
            buckets.len(),
            2,
            "first appearance order, one bucket per material"
        );
        assert!(std::sync::Arc::ptr_eq(
            buckets[1].0.as_ref().unwrap(),
            &painted
        ));
        let (mut meshes, mut materials, mut images) = (
            Assets::<Mesh>::default(),
            Assets::<ViewerMaterial>::default(),
            Assets::<Image>::default(),
        );
        let mut bias = |p: Prepared| {
            let (_, m, _) = upload(p, &mut meshes, &mut materials, &mut images);
            materials.get(&m).unwrap().base.depth_bias
        };
        let mut later = d286_bucket(0);
        later.bucket = 1;
        let (first, second) = (bias(d286_bucket(0)), bias(later));
        assert_eq!(first, 0.);
        assert!(second > first, "{second} > {first}");
        let mut last = d286_bucket(0);
        last.bucket = adapter::MAX_DRAWS + 10;
        assert_eq!(bias(last), adapter::MAX_DRAWS as f32);
        let mut decal = d286_bucket(0);
        decal.decal = true;
        decal.bucket = 3;
        assert_eq!(bias(decal), DECAL_DEPTH_BIAS);
        let mut glass = d286_bucket(0);
        glass.bucket = 2;
        mark_glass(&mut glass, None);
        assert_eq!(bias(glass), 0.);
    }
    /// D287: the uploaded glass carries the specular scale added after its alpha (6); a
    /// plain bucket has none.
    #[test]
    fn d287_the_glass_carries_its_specular_scale() {
        let mut glass = d286_bucket(0);
        mark_glass(&mut glass, None);
        let (mut meshes, mut materials, mut images) = (
            Assets::<Mesh>::default(),
            Assets::<ViewerMaterial>::default(),
            Assets::<Image>::default(),
        );
        let (_, material, _) = upload(glass, &mut meshes, &mut materials, &mut images);
        let material = materials.get(&material).unwrap();
        assert_eq!(dual::glass_specular_scale(material), GLASS_SPECULAR_SCALE);
        assert_eq!(GLASS_SPECULAR_SCALE, 6.);
        let (_, material, _) = upload(d286_bucket(0), &mut meshes, &mut materials, &mut images);
        let material = materials.get(&material).unwrap();
        assert_eq!(dual::glass_specular_scale(material), 0.);
    }
    /// D286: the glass gets the mesh's raw UV0 as `in.uv`, recovered from its bump map's
    /// UV0 x 2; a bump on another channel, a matrix transform or a zero scale gives none.
    #[test]
    fn d286_the_glass_takes_raw_uv0() {
        let x2 = lbp_preview::SamplingTransform::ScaleOffset(lbp_preview::UvTransform {
            bits: [2f32.to_bits(), 2f32.to_bits(), 0, 0],
        });
        let raw = [[0.83, 0.007], [0.95, 0.9], [0.9, 0.5]];
        let bumped: Vec<[f32; 2]> = raw.iter().map(|&[u, v]| [u * 2., v * 2.]).collect();
        let back = raw_uv0_from_bump(0, x2, &bumped).unwrap();
        assert_eq!(back, raw);
        assert_eq!(raw_uv0_from_bump(1, x2, &bumped), None);
        let zero = lbp_preview::SamplingTransform::ScaleOffset(lbp_preview::UvTransform {
            bits: [0, 2f32.to_bits(), 0, 0],
        });
        assert_eq!(raw_uv0_from_bump(0, zero, &bumped), None);
        let matrix = lbp_preview::SamplingTransform::Matrix(lbp_preview::UvMatrix {
            bits: [1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0],
        });
        assert_eq!(raw_uv0_from_bump(0, matrix, &bumped), None);
        let mut glass = d286_bucket(0);
        mark_glass(&mut glass, Some(back.clone()));
        assert_eq!(glass.raw_uv0, 1);
        match glass.mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) => assert_eq!(*uv, back),
            other => panic!("UV0 {other:?}"),
        }
        // A wrong length, or none given: still glass, the mask stays on the position.
        for raw in [Some(back[..2].to_vec()), None] {
            let mut other = d286_bucket(0);
            mark_glass(&mut other, raw);
            assert_eq!((other.raw_uv0, other.glass), (0, Some(0.2)));
            match other.mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) => {
                    assert_eq!(*uv, [[0f32; 2]; 3])
                }
                other => panic!("UV0 {other:?}"),
            }
        }
    }
    #[test]
    fn d317_per_bone_is_on_in_the_scripted_pod_unless_turned_off() {
        let mode = |scripts, pod: bool, game_camera, per_bone, no_per_bone| LevelMode {
            scripts,
            pod: pod.then_some(1),
            game_camera,
            per_bone,
            no_per_bone,
            ..Default::default()
        };
        // Off by default; on with the flag.
        assert!(!mode(false, false, false, false, false).per_bone_on());
        assert!(mode(false, false, false, true, false).per_bone_on());
        // Under --pod-game-camera, and in the scripted Pod with or without it.
        assert!(mode(false, true, true, false, false).per_bone_on());
        assert!(mode(true, true, false, false, false).per_bone_on());
        assert!(mode(true, true, true, false, false).per_bone_on());
        // Scripts without the Pod, or the Pod without scripts or its camera: off.
        assert!(!mode(true, false, false, false, false).per_bone_on());
        assert!(!mode(false, true, false, false, false).per_bone_on());
        // --no-per-bone-placement keeps the bone-0 rule in both.
        assert!(!mode(false, true, true, false, true).per_bone_on());
        assert!(!mode(true, true, false, false, true).per_bone_on());
    }
    #[test]
    fn d317_a_model_skinned_per_bone_is_on_the_thing_map() {
        let mut scene = level_scene::LevelScene::<u8>::default();
        let base = || Box::new(level_scene::MeshSource::Guid(0x27bd));
        for source in [
            level_scene::MeshSource::Guid(0x27bd),
            level_scene::MeshSource::BoneSkin {
                mesh: base(),
                thing: 1,
            },
            level_scene::MeshSource::JointSkin {
                mesh: base(),
                thing: 1,
            },
        ] {
            scene.meshes.push(level_scene::SceneMesh {
                source,
                payload: 0,
                bounds: None,
                neutral_groups: false,
            });
        }
        let item = |mesh| level_scene::SceneItem {
            mesh,
            world: Mat4::from_translation(Vec3::X).to_cols_array(),
            thing: 1,
            uid: None,
            shapes: 0,
            decoration: false,
            alternate: false,
        };
        let file = thing_item(&scene, &item(0)).expect("the file's mesh");
        let skinned = thing_item(&scene, &item(1)).expect("the per-bone mesh");
        // Same Thing, matrix and base mesh: the scripts move it as the file's mesh.
        assert_eq!(
            (skinned.thing, skinned.world, skinned.mesh),
            (file.thing, file.world, Some(0x27bd))
        );
        // A joint span is placed at its own origin, not by the Thing: not mapped.
        assert!(thing_item(&scene, &item(2)).is_none());
    }
    #[test]
    fn level_placement_normalizes_frames_side_on_and_counts_display_skips() {
        let mut scene = level_scene::LevelScene::<u8>::default();
        for (source, bounds) in [
            (
                level_scene::MeshSource::Guid(1),
                Some([[-1., -1., -1.], [1., 1., 1.]]),
            ),
            (level_scene::MeshSource::Guid(2), Some([[0.; 3], [1.; 3]])),
        ] {
            scene.meshes.push(level_scene::SceneMesh {
                source,
                payload: 0,
                bounds,
                neutral_groups: false,
            });
        }
        let at = |x: f32| Mat4::from_translation(Vec3::new(x, 0., 0.)).to_cols_array();
        for (mesh, x, shapes) in [(0, 0., 0), (0, 98., 0), (1, 500., 3)] {
            scene
                .push_item(level_scene::SceneItem {
                    mesh,
                    world: at(x),
                    thing: 0,
                    uid: None,
                    shapes,
                    decoration: false,
                    alternate: false,
                })
                .unwrap();
        }
        let (placed, home, lines, _) = place_items(
            &scene,
            &[Ok(2), Err("Unavailable: test".into())],
            None,
            None,
        )
        .unwrap();
        assert_eq!(placed.len(), 2);
        // Displayed bounds x -1..99 -> unit span centred at the origin.
        let a = placed[0].1.translation;
        let b = placed[1].1.translation;
        assert!((a.x + 0.49).abs() < 1e-5 && (b.x - 0.49).abs() < 1e-5);
        assert!((placed[0].1.scale - Vec3::splat(0.01)).length() < 1e-6);
        assert_eq!((home.yaw, home.pitch, home.target), (0., 0., Vec3::ZERO));
        assert!(home.distance > 0.15 && home.distance < 2.);
        assert!(lines[0].starts_with(
            "Drawn in viewer: 2 RENDER_MESH Things, 0 shape Things, 0 backdrop items (4 draw entities)"
        ));
        assert!(lines
            .iter()
            .any(|l| l == "  display-skipped 3 Things: Unavailable: test"));
        assert!(level_home(Vec3::ZERO, Vec3::ZERO).is_err());
    }
    #[test]
    fn selecting_a_level_retargets_and_clears_previous_framing() {
        let mut mode = LevelMode {
            pod: Some(3),
            shell: Some(0x27b9),
            focus: Some(1),
            game_camera: true,
            lines: vec!["old".into()],
            ..LevelMode::default()
        };
        mode.retarget(&lbp_preview::level_list::LevelEntry {
            path: "a/b/c.bin".into(),
            guid: Some(7),
            group: "a/b".into(),
            name: "c".into(),
            named: false,
            supported: true,
        });
        assert_eq!(
            (mode.guid, mode.path.as_deref()),
            (Some(7), Some("a/b/c.bin"))
        );
        assert!(mode.pod.is_none() && mode.shell.is_none() && mode.focus.is_none());
        assert!(!mode.game_camera && mode.lines.is_empty() && mode.home.is_none());
        assert!(mode.active());
    }
    #[test]
    fn dropping_replaced_job_cancels_and_disconnects_stale_result() {
        let control = LoadControl::default();
        let (tx, rx) = mpsc::sync_channel(1);
        let job = Job {
            started: std::time::Instant::now(),
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
            bump_gloss: false,
            no_bump_gloss: false,
            no_shadows: false,
            instance_input_enabled: false,
            instance_rgba: [1.; 4],
            animation_base: false,
            instance_default: true,
            animation_base_default: true,
            animation_guid: String::new(),
            hold_animation_guid: String::new(),
            animation_frame: None,
            cpu_skinning: false,
            model_z_up: true,
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
            hide_ui: false,
            window_16_9: false,
            selector: level_selector::LevelSelector::default(),
            browser: browser::Browser::default(),
            census: CorpusCounts::default(),
            level: LevelMode::default(),
            lighting: lighting::LightingState::default(),
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
