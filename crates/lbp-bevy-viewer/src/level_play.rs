//! `--level-scripts` (docs/level-scripts.md, part 6): with a level, run the level's own
//! scripts at the game's 30 Hz tick in Bevy's `FixedUpdate`, through the same
//! `LevelScripts` schedule as the `level_scripts` example: `OnCreate__` on the first
//! tick, then each tick resume suspended scripts and run `Update__`. With `--play-sackboy`
//! the Sackboy is the player the level's triggers test, so their `Enter`, `Inside` and
//! `Leave` scripts run as he walks. A small window shows the counts. Of what the scripts
//! change, the open messages are drawn (part 17, `speech_overlay.rs`): speech bubbles at
//! their Things and the subtitle; and the level is lit again when a script changes the
//! global lighting or darkness (part 18); and the Things' own models follow what the
//! scripts hide, tint and move (part 19, `thing_view.rs`); and the camera goes to a
//! script's override camera and back (part 20), while the scripts are told which camera
//! the picture was drawn with; and the camera is put where a script writes the camera
//! itself, with the Pod's planets level hung under its script-moved world Thing (part
//! 23). The rest is the clock and the counts.
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::level_scene;
use lbp_runtime::{
    CameraPose, CameraSource, CameraView, DisplayCamera, LevelScripts, LevelWorld, LightFollower,
    StepReport, TICK_HZ,
};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Vm;
use lbp_sim::avian2d::prelude::Position;
use lbp_sim::character::SackboyBody;
use std::path::Path;

/// Instruction budget per script call (as the `level_scripts` example).
const BUDGET_PER_CALL: u64 = 1_000_000;
/// Fault lines kept for the window.
const MAX_FAULT_LINES: usize = 8;

struct Play {
    vm: Vm,
    host: Bindings<LevelWorld>,
    scripts: LevelScripts,
    created: Option<StepReport>,
    /// Sums over update ticks.
    totals: StepReport,
    ticks_with_faults: usize,
    faults: Vec<String>,
    header: String,
    hide_ui: bool,
    /// The play-mode Sackboy's Thing in the script world, once he exists.
    player: Option<u32>,
    /// Messages that entered the list and messages closed, at the last printed line.
    messages_printed: (u64, u64),
    /// The (lighting, darkness) pair the scene's light shows, once the level is drawn
    /// with a light that follows the PWorld lighting.
    light: Option<LightFollower>,
    /// The scene whose draw entities were last set from the script world
    /// (`thing_view::LevelThings::scene`), and what was done to them since the start.
    things_scene: Option<u64>,
    followed: crate::thing_view::Followed,
    /// The camera's own field of view (radians), kept while a script's override camera
    /// has the lens, to put back when the override is out.
    follow_fov: Option<f32>,
    /// Frames drawn with the override camera, and the override's state at the last
    /// printed line: (in, blend at 1, target on).
    override_frames: u64,
    override_printed: (bool, bool, bool),
    /// `--level-scripts-hold-at N` (QA): no tick after tick N.
    hold_at: Option<u64>,
    /// Frames drawn with a camera a script wrote itself (part 23), that camera's state
    /// at the last printed line, and its pose after the last tick it was looked at.
    script_frames: u64,
    script_state: ScriptCameraState,
    script_last: Option<(u64, CameraPose)>,
    /// A script wrote the camera and the viewer kept its own: said once.
    script_not_applied_printed: bool,
    /// The matrix the root of a level that hangs under its world Thing was last placed
    /// by ([`follow_world`]), and how often it was placed.
    world_root: Option<[f32; 16]>,
    world_root_placed: u64,
}

/// What the camera a script writes itself did since the tick before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptCameraState {
    Out,
    Moving,
    AtRest,
}

/// On the root of a level that is drawn under its world Thing while its scripts run
/// (the Pod's planets level; part 23). The root also carries `crate::PodPlanetsRoot`,
/// which holds the level's display matrix.
#[derive(Component)]
pub(crate) struct WorldThingRoot;

fn value(all: &[String], flag: &str) -> Option<String> {
    all.iter()
        .position(|a| a == flag)
        .and_then(|i| all.get(i + 1))
        .cloned()
}

/// Loads scripts and the level and adds the 30 Hz schedule when `--level-scripts` is
/// given; otherwise does nothing.
pub fn install(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    if !all.iter().any(|a| a == "--level-scripts") {
        return Ok(());
    }
    let snapshot = value(&all, "--snapshot").ok_or("--level-scripts requires --snapshot")?;
    let game = value(&all, "--game-dir").ok_or("--level-scripts requires --game-dir")?;
    let guid = value(&all, "--level-guid")
        .map(|g| u32::from_str_radix(&g, 16).map_err(|_| "--level-guid requires hex digits"))
        .transpose()?;
    let path = value(&all, "--level-path");
    if guid.is_none() && path.is_none() {
        return Err("--level-scripts requires --level-guid or --level-path".into());
    }
    let hide_ui = all.iter().any(|a| a == "--qa-hide-ui");
    let hold_at = value(&all, "--level-scripts-hold-at")
        .map(|v| {
            v.parse::<u64>()
                .map_err(|_| "--level-scripts-hold-at takes a tick count")
        })
        .transpose()?;
    // Bit masks ORed into the profile's word of LBP2 game progression flags (as
    // `level_scripts --flags`).
    let flags = value(&all, "--level-scripts-flags")
        .map(|v| {
            lbp_runtime::parse_progression_flags(&v)
                .map_err(|e| format!("--level-scripts-flags: {e}"))
        })
        .transpose()?
        .unwrap_or_default();
    // The level reader recurses deeply: same worker stack as the level scene.
    let (mut vm, mut world, header) = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<(Vm, LevelWorld, String), String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(Path::new(&snapshot), &control)?;
            let source = AttachedSource::attach(Path::new(&game), &catalog, &control)?;
            let (scripts, not_read) = lbp_ui_script::load::load(&source, &catalog, &control)?;
            let mut vm = Vm::default();
            let loaded = scripts.len();
            for (g, s) in scripts {
                vm.load(s, g);
            }
            let (key, identity) = level_scene::resolve_level(&catalog, guid, path.as_deref())?;
            let bytes = level_scene::read_level_bytes(&source, &key, &control)?;
            let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
                .map_err(|e| e.to_string())?;
            let mut world = LevelWorld::from_things(&report.things);
            // The English tables, patch first, as the `level_scripts` example loads them
            // (INFERRED that these are the tables loaded while a level plays). A table
            // that cannot be read is left out: the scripts then give their own text for
            // a missing key and a bubble shows no close prompt.
            let (strings, _) = lbp_preview::story::load_strings(&source, &catalog, &control);
            world.translations = [strings.patch, strings.base]
                .into_iter()
                .flatten()
                .map(std::sync::Arc::new)
                .collect();
            let header = format!(
                "Level scripts: {identity}; scripts loaded {loaded} (not read {not_read}); translation tables {}",
                world.translations.len()
            );
            Ok((vm, world, header))
        })
        .map_err(|e| format!("level scripts thread: {e}"))?
        .join()
        .map_err(|_| "level scripts thread panicked".to_string())??;
    let built = world.bind_objects(&mut vm);
    world.add_class_defaults(&vm);
    world.progression_flags = flags;
    // Unbound natives return their type's default and are counted (rscript-plan stage 1).
    let mut host = Bindings::defaulting(world);
    let mut faults = lbp_runtime::add_engine_objects(&mut vm, &mut host, BUDGET_PER_CALL);
    faults.truncate(MAX_FAULT_LINES);
    let scripts = LevelScripts::new(&vm, &host.world);
    let header = format!(
        "{header}; scripted Things {}; objects built {built}, unconverted {}",
        scripts.instances.len(),
        host.world.unconverted
    );
    println!("{header}");
    app.insert_non_send(Play {
        vm,
        host,
        scripts,
        created: None,
        totals: StepReport::default(),
        ticks_with_faults: 0,
        faults,
        header,
        hide_ui,
        player: None,
        messages_printed: (0, 0),
        light: None,
        things_scene: None,
        followed: Default::default(),
        follow_fov: None,
        override_frames: 0,
        override_printed: (false, false, false),
        hold_at,
        script_frames: 0,
        script_state: ScriptCameraState::Out,
        script_last: None,
        script_not_applied_printed: false,
        world_root: None,
        world_root_placed: 0,
    })
    .insert_resource(crate::CaptureHold(hold_at.is_some()))
    .insert_resource(Time::<Fixed>::from_hz(f64::from(TICK_HZ)))
    .add_systems(
        FixedUpdate,
        (tick, relight, follow_things, follow_world).chain(),
    )
    // After the viewer's own camera has placed the follow camera, before a capture.
    .add_systems(
        Update,
        script_camera.after(crate::camera).before(crate::capture),
    )
    .add_systems(EguiPrimaryContextPass, (speech, window));
    Ok(())
}

fn note(play: &mut Play, r: &StepReport) {
    for (thing, f) in &r.faults {
        if play.faults.len() < MAX_FAULT_LINES {
            play.faults.push(format!("thing {thing}: {f}"));
        }
    }
}

/// Half the z span of the play-mode Sackboy for the triggers' layer rule. PLACEHOLDER:
/// the game takes the span from his body; it has not been measured.
const PLAYER_HALF_DEPTH: f32 = 100.;

fn tick(
    mut play: NonSendMut<Play>,
    mut capture: ResMut<crate::CaptureHold>,
    sackboy: Query<(&Position, &lbp_sim::ZSlab), With<SackboyBody>>,
) {
    let p = &mut *play;
    // `--level-scripts-hold-at N`: a capture waits for tick N, and no tick follows it.
    let held = p.hold_at.is_some_and(|n| p.scripts.ticks >= n);
    if capture.0 != (p.hold_at.is_some() && !held) {
        capture.0 = p.hold_at.is_some() && !held;
    }
    if p.created.is_none() {
        let r = p.scripts.create(&mut p.vm, &mut p.host, BUDGET_PER_CALL);
        note(p, &r);
        p.created = Some(r);
        return;
    }
    if held {
        return;
    }
    // The play-mode Sackboy (`--play-sackboy`), when there is one, is the player the
    // level's triggers test.
    if let Ok((pos, slab)) = sackboy.single() {
        let [x, y] = lbp_sim::to_level(pos.0);
        let at = [x, y, slab.z];
        let layers = (slab.z - PLAYER_HALF_DEPTH, slab.z + PLAYER_HALF_DEPTH);
        match p.player {
            Some(thing) => p.host.world.move_player(thing, at, layers),
            None => p.player = Some(p.host.world.add_player(at, layers)),
        }
    }
    let r = p
        .scripts
        .tick_level(&mut p.vm, &mut p.host, BUDGET_PER_CALL);
    p.ticks_with_faults += usize::from(!r.faults.is_empty());
    note(p, &r);
    p.totals.absorb(r);
    print_messages(p);
}

/// Frames a loaded scene is drawn before its light may be built again. MEASURED on the
/// PC (one level, release build): a new light in the first frame after the scene was
/// spawned lost the GPU device, twice out of two runs; after this wait 89 new lights on
/// 89 ticks in a row did not. The cause is not established. PLACEHOLDER guard.
const RELIGHT_AFTER_FRAMES: u32 = 60;

/// Lights the scene again when the lighting or the darkness in force has moved away from
/// the pair the scene's light was built from (part 18): the level's stored presets
/// blended at the new lighting and dimmed by the new darkness, the rule the viewer
/// already uses for the level file's own values (P3m, `LevelSettings::at_lighting`).
/// Nothing while the level is not drawn yet ([`RELIGHT_AFTER_FRAMES`]), or is lit another
/// way (the Pod, a chosen preset, the main block). Colour correction, fog and fog tint
/// are not applied: their mapping is not established. One line per new light: the check
/// that needs no screenshot.
fn relight(mut play: NonSendMut<Play>, mut v: ResMut<crate::Viewer>) {
    let p = &mut *play;
    if v.ready_frames < RELIGHT_AFTER_FRAMES {
        // A scene loaded again is lit from the level file's values again.
        p.light = None;
        return;
    }
    let Some(level) = v.lighting.level.as_ref() else {
        return;
    };
    let follower = p
        .light
        .get_or_insert_with(|| LightFollower::new(level.stored));
    let now = p.host.world.pworld.settings_in_force();
    let Some([lighting, darkness]) = follower.update(now.light()) else {
        return;
    };
    let light = level.light(lighting, darkness);
    println!(
        "Lighting at tick {}: lighting {lighting:.3}, darkness {darkness:.3} (new light {}); not applied: colour correction {:.3}, fog {:.3}, fog tint {:.3}",
        p.scripts.ticks, follower.relights, now.lighting[1], now.lighting[2], now.lighting[3]
    );
    v.lighting.light = light;
    v.lighting.dirty = true;
}

/// Sets the level's draw entities from the script world (part 19): the Things on the
/// world's dirty list each tick, and for a scene that was just spawned every mapped
/// Thing that is hidden or unlike the level file (scripts tick while the scene loads).
/// Like [`relight`], nothing until the scene has been drawn for
/// [`RELIGHT_AFTER_FRAMES`]: the same guard, since a first tint makes a new material.
/// One line per tick on which the scene changed: the check that needs no screenshot.
fn follow_things(
    mut play: NonSendMut<Play>,
    mut v: ResMut<crate::Viewer>,
    things: Option<Res<crate::thing_view::LevelThings>>,
    mut entities: Query<crate::thing_view::Parts>,
    mut materials: ResMut<Assets<crate::ViewerMaterial>>,
) {
    let p = &mut *play;
    let Some(things) = things else {
        return;
    };
    if v.ready_frames < RELIGHT_AFTER_FRAMES {
        p.things_scene = None;
        return;
    }
    let world = &mut p.host.world;
    let first = p.things_scene != Some(things.scene);
    let list = if first {
        p.things_scene = Some(things.scene);
        world.take_display_dirty();
        crate::thing_view::first_list(world, &things)
    } else {
        world.take_display_dirty()
    };
    if list.is_empty() {
        return;
    }
    let done = crate::thing_view::apply(
        world,
        &list,
        &things,
        &mut entities,
        &mut materials,
        &mut v.scene,
    );
    p.followed.absorb(&done);
    if first || done.any() {
        // The first few Things by what the list says of them, to find them in the scene.
        let some = |keep: &dyn Fn(u32, &lbp_runtime::ThingDisplay) -> bool| -> Vec<u32> {
            let kept = list.iter().filter(|(thing, _)| {
                things.entities.contains_key(thing)
                    && world
                        .thing_display(*thing)
                        .is_some_and(|d| keep(*thing, &d))
            });
            kept.map(|(thing, _)| *thing)
                .take(MAX_THINGS_NAMED)
                .collect()
        };
        let hidden = some(&|_, d| d.mesh_hidden());
        let by_script = some(&|t, d| {
            d.mesh_hidden()
                && !world
                    .display_at_load(t)
                    .is_some_and(|was| was.mesh_hidden())
        });
        // A tint with its colour (red, green, blue, alpha).
        let tinted: Vec<(u32, [f32; 4])> = some(&|_, d| d.mesh_tint.is_some())
            .into_iter()
            .filter_map(|t| Some((t, world.thing_display(t)?.mesh_tint?)))
            .collect();
        println!(
            "Things at tick {}{}: {}; first Things hidden {hidden:?} (of them by a script {by_script:?}), tinted {tinted:?}",
            p.scripts.ticks,
            if first { " (new scene)" } else { "" },
            done.line()
        );
    }
}

/// Places the root of a level that hangs under its world Thing (part 23): the Pod's
/// planets level, whose world script moves the world Thing each tick and with it the
/// view of the planets. The root's matrix is `N M N^-1`, `N` the level's display matrix
/// and `M` the matrix that takes the world Thing from its stored place to its place now
/// (`LevelWorld::world_thing_moved`). Like [`relight`], nothing until the scene has been
/// drawn for [`RELIGHT_AFTER_FRAMES`]: the scene's light turns with the root.
fn follow_world(
    mut play: NonSendMut<Play>,
    v: Res<crate::Viewer>,
    mut roots: Query<(&mut Transform, &crate::PodPlanetsRoot), With<WorldThingRoot>>,
) {
    let p = &mut *play;
    if roots.is_empty() || v.ready_frames < RELIGHT_AFTER_FRAMES {
        p.world_root = None;
        return;
    }
    let Some(moved) = p.host.world.world_thing_moved() else {
        return;
    };
    if p.world_root == Some(moved) {
        return;
    }
    let first = p.world_root.is_none();
    p.world_root = Some(moved);
    p.world_root_placed += 1;
    for (mut transform, crate::PodPlanetsRoot(normalize)) in &mut roots {
        *transform = world_root(*normalize, Mat4::from_cols_array(&moved));
    }
    if first {
        println!(
            "World Thing at tick {}: the level hangs under it; moved from its stored place by a matrix with translation {:.1?} and x axis of length {:.3}",
            p.scripts.ticks,
            &moved[12..15],
            Vec3::new(moved[0], moved[1], moved[2]).length()
        );
    }
}

/// The root of a level drawn under its world Thing: `moved` (level units) in display
/// units, when `normalize` takes level units to display units.
fn world_root(normalize: Mat4, moved: Mat4) -> Transform {
    Transform::from_matrix(normalize * moved * normalize.inverse())
}

/// Where the viewer's camera goes for a camera the runtime hands out (level units), when
/// `display` takes level units to display units: the eye through `display`, and the
/// turn of the runtime's own camera matrix, so the up vector is the runtime's rule. The
/// display matrix is a uniform scale and a translation: directions are kept.
fn place_camera(display: Mat4, camera: &DisplayCamera) -> Transform {
    let axes = Mat3::from_mat4(Mat4::from_cols_array(&camera.matrix()));
    Transform {
        translation: display.transform_point3(Vec3::from(camera.pose.eye)),
        rotation: Quat::from_mat3(&axes),
        scale: Vec3::ONE,
    }
}

/// The camera a display hands to the scripts for a `Camera3d` at `global` (display
/// units) with the lens `clip_from_view`, when `display` takes level units to display
/// units: the camera matrix in level units with axes of length 1, and the matrix from a
/// level point to clip space. `None` when a matrix has no inverse.
fn camera_view(display: Mat4, global: Mat4, clip_from_view: Mat4) -> Option<CameraView> {
    let (to_level, view) = (display.inverse(), global.inverse());
    if !to_level.is_finite() || !view.is_finite() {
        return None;
    }
    let mut camera = to_level * global;
    for axis in [&mut camera.x_axis, &mut camera.y_axis, &mut camera.z_axis] {
        *axis = axis.truncate().normalize_or_zero().extend(0.);
    }
    Some(CameraView {
        camera: camera.to_cols_array(),
        view_proj: (clip_from_view * view * display).to_cols_array(),
    })
}

/// The script camera (parts 20 and 23), each frame after the viewer's own camera system
/// has placed the camera: that placement is the follow camera. The runtime says which
/// camera the frame is drawn with (`LevelWorld::display_camera`):
///
/// - A camera a script wrote itself on the last tick (`Cam.SetCameraFromToUp`, the
///   Pod's planets level): the camera is put there outright, with the script's up
///   vector and field of view. It goes before an override.
/// - Else, while a script's override is in: the pose the runtime mixes from the follow
///   camera and the override, with the override's field of view.
/// - Else the viewer's own camera stands, and its own field of view comes back.
///
/// The viewer's `Orbit` is not touched, so orbit, pan and zoom go on underneath and the
/// view returns to them. Then the camera the picture is drawn with is written for the
/// scripts (`CameraState::view`): `Cam.GetCameraMatrix__` and
/// `HUD.GetScreenSpacePosition__r` answer from it on the next ticks.
///
/// With the Pod's shell layer (`--pod-shell` with `--pod-camera`) a camera a script
/// wrote itself is not applied: that layer is placed for the viewer's own Pod camera
/// (`pod_menu::PodRig`), and the two would not fit. Said once.
///
/// INFERRED: world +Y is up under the override. Not applied: the near and far planes of
/// either script camera (the viewer keeps its own, in display units). One line is
/// printed when a camera comes in, is fully in or comes to rest, or goes out: the check
/// without a screenshot.
fn script_camera(
    mut play: NonSendMut<Play>,
    v: Res<crate::Viewer>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
) {
    let (Some(display), Ok((mut transform, mut projection))) = (display, cameras.single_mut())
    else {
        return;
    };
    let p = &mut *play;
    let to_level = display.0.inverse();
    if !to_level.is_finite() {
        return;
    }
    // The display matrix is a uniform scale and a translation: directions are kept.
    let follow_eye = to_level.transform_point3(transform.translation);
    let follow_dir = transform.forward().as_vec3();
    let world = &mut p.host.world;
    let shell_layer = v.level.shell.is_some() && v.level.pod.is_some();
    let shown = world
        .display_camera(follow_eye.to_array(), follow_dir.to_array())
        .filter(|c| !(shell_layer && c.source == CameraSource::Script));
    if shell_layer && !p.script_not_applied_printed && world.script_camera().is_some() {
        p.script_not_applied_printed = true;
        println!(
            "Script camera at tick {}: a script writes the camera; not applied, the Pod shell layer is placed for the viewer's own Pod camera",
            p.scripts.ticks
        );
    }
    let own_fov = match projection.as_ref() {
        Projection::Perspective(lens) => Some(lens.fov),
        _ => None,
    };
    let wanted_fov = match &shown {
        Some(camera) => {
            *transform = place_camera(display.0, camera);
            match camera.source {
                CameraSource::Script => p.script_frames += 1,
                CameraSource::Override => p.override_frames += 1,
            }
            if p.follow_fov.is_none() {
                p.follow_fov = own_fov;
            }
            // A script camera without a usable field of view keeps the viewer's own.
            camera.fov_y_deg.map(f32::to_radians).or(p.follow_fov)
        }
        None => p.follow_fov.take(),
    };
    if let (Some(fov), Some(own)) = (wanted_fov, own_fov) {
        if fov != own {
            if let Projection::Perspective(lens) = projection.as_mut() {
                lens.fov = fov;
            }
        }
    }
    world.camera.view = camera_view(
        display.0,
        transform.to_matrix(),
        projection.get_clip_from_view(),
    );
    let o = &world.camera.override_camera;
    let now = (o.is_in(), o.blend >= 1., o.target > 0.);
    if now != p.override_printed {
        p.override_printed = now;
        let at = transform.translation;
        println!(
            "Camera at tick {}: override {} (blend {:.2}, weight {:.2}, target {}); set calls {}, activations {}; frames drawn with it {}; camera at display {:.3?}, level {:.1?}",
            p.scripts.ticks,
            match now {
                (false, ..) => "out",
                (true, true, _) => "fully in",
                (true, false, true) => "coming in",
                (true, false, false) => "going out",
            },
            o.blend,
            o.weight(),
            if now.2 { "on" } else { "off" },
            o.set_calls,
            o.activation_calls,
            p.override_frames,
            at.to_array(),
            to_level.transform_point3(at).to_array(),
        );
    }
    // The camera a script wrote itself: one line when it comes in, comes to rest, moves
    // again or goes out, looked at once per tick.
    let script = shown.filter(|c| c.source == CameraSource::Script);
    let ticks = p.scripts.ticks;
    let state = match (script, p.script_last) {
        (None, _) => ScriptCameraState::Out,
        (Some(_), Some((at, _))) if at == ticks => p.script_state,
        (Some(c), Some((_, last))) if c.pose == last => ScriptCameraState::AtRest,
        (Some(_), _) => ScriptCameraState::Moving,
    };
    p.script_last = script.map(|c| (ticks, c.pose));
    if state != p.script_state {
        let was = std::mem::replace(&mut p.script_state, state);
        let what = match (was, state) {
            (_, ScriptCameraState::Out) => "out",
            (ScriptCameraState::Out, _) => "in",
            (_, ScriptCameraState::AtRest) => "at rest",
            (_, ScriptCameraState::Moving) => "moving again",
        };
        match script {
            Some(c) => println!(
                "Script camera at tick {ticks}: {what}; eye {:.1?}, direction {:.3?}, up {:.2?}, field of view {} (level units); near {:.0} and far {:.0} not applied; frames drawn with it {}",
                c.pose.eye,
                c.pose.direction,
                c.up,
                c.fov_y_deg
                    .map_or("not usable, the viewer's own".into(), |f| format!("{f:.1} deg")),
                c.near,
                c.far,
                p.script_frames
            ),
            None => println!(
                "Script camera at tick {ticks}: {what}; frames drawn with it {}",
                p.script_frames
            ),
        }
    }
}

/// Thing numbers named per kind in the line [`follow_things`] prints.
const MAX_THINGS_NAMED: usize = 8;

/// One line per tick on which a message entered the list or was closed: the check of the
/// message display that needs no screenshot.
fn print_messages(p: &mut Play) {
    let m = &p.host.world.messages;
    let sum = m.totals();
    let now = (sum.opened + sum.replaced, m.closed);
    if now == p.messages_printed {
        return;
    }
    p.messages_printed = now;
    let o = p.host.world.message_overlay();
    let anchors: Vec<(u32, [f32; 3])> = o.bubbles.iter().map(|b| (b.anchor, b.position)).collect();
    println!(
        "Messages at tick {}: open {}, opened {}, written over {}, closed {}; drawn: bubbles {} at (Thing, place) {anchors:?}, subtitle {}; not drawn: prompts {}, empty {}, unplaced {}",
        p.scripts.ticks,
        m.open.len(),
        sum.opened,
        sum.replaced,
        m.closed,
        o.bubbles.len(),
        u8::from(o.subtitle.is_some()),
        o.prompts_not_drawn,
        o.empty_not_drawn,
        o.unplaced_not_drawn
    );
}

/// Draws the open messages over the scene (also under `--qa-hide-ui`: they are part of
/// the level, not a panel). The anchor goes from level units to display units by the
/// level's display matrix and to the screen through the camera.
fn speech(
    mut contexts: EguiContexts,
    play: NonSend<Play>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Result {
    let (Some(display), Ok((camera, camera_at))) = (display, cameras.single()) else {
        return Ok(());
    };
    let overlay = play.host.world.message_overlay();
    crate::speech_overlay::draw(contexts.ctx_mut()?, &overlay, |at| {
        let at = display.0.transform_point3(Vec3::from(at));
        camera.world_to_viewport(camera_at, at).ok().map(Into::into)
    });
    Ok(())
}

fn line(r: &StepReport) -> String {
    format!(
        "done {}, yielded {}, resumed {}, no function {}, no class {}, faults {}",
        r.done,
        r.yielded,
        r.resumed,
        r.no_function,
        r.no_class,
        r.faults.len()
    )
}

fn window(mut contexts: EguiContexts, play: NonSend<Play>) -> Result {
    if play.hide_ui {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    egui::Window::new("Level scripts")
        .default_pos([16., 480.])
        .show(ctx, |ui| {
            ui.small(&play.header);
            ui.label(format!(
                "Tick {} at {TICK_HZ} Hz; suspended {}; stopped by a fault {}",
                play.scripts.ticks,
                play.scripts.pending(),
                play.scripts.stopped.len()
            ));
            if let Some(c) = &play.created {
                ui.label(format!("Create: {}", line(c)));
            }
            ui.label(format!("Ticks: {}", line(&play.totals)));
            ui.label(format!("Ticks with faults: {}", play.ticks_with_faults));
            let [enter, inside, leave] = play.totals.trigger_events;
            let [e, i, l] = play.totals.events_run;
            ui.label(format!(
                "Triggers ({}): enter {enter} ({e} run), inside {inside} ({i} run), leave {leave} ({l} run)",
                if play.player.is_some() { "player in play" } else { "no player" }
            ));
            let messages = &play.host.world.messages;
            let sum = messages.totals();
            let shown = play.host.world.message_overlay();
            ui.label(format!(
                "Messages: open {} (drawn: bubbles {}, subtitle {}); opened {}, written over {}, closed {}",
                messages.open.len(),
                shown.bubbles.len(),
                u8::from(shown.subtitle.is_some()),
                sum.opened,
                sum.replaced,
                messages.closed
            ));
            let now = play.host.world.pworld.settings_in_force();
            let [lighting, correction, fog, fog_tint, darkness] = now.lighting;
            ui.label(format!(
                "Global settings: {} apply calls; lighting {lighting:.3}, darkness {darkness:.3} ({})",
                play.totals.global_settings.total_calls(),
                match &play.light {
                    Some(f) => format!("scene lit {} times by them", f.relights),
                    None => "scene light does not follow them".into(),
                }
            ));
            ui.small(format!(
                "  not applied: colour correction {correction:.3}, fog {fog:.3}, fog tint {fog_tint:.3}, gravity {:.3}, water level {:.3}",
                now.gravity, now.water_level
            ));
            ui.label(match play.things_scene {
                Some(_) => format!("Things: {}", play.followed.line()),
                None => "Things: the scene does not follow them (yet)".into(),
            });
            let mut unbound: Vec<_> = play.host.unbound.iter().collect();
            unbound.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            if !unbound.is_empty() {
                let calls: u64 = play.host.calls.values().sum();
                let defaulted: u64 = unbound.iter().map(|u| *u.1).sum();
                ui.small(format!(
                    "Native calls: bound {}, defaulted {defaulted} ({} unbound natives):",
                    calls - defaulted,
                    unbound.len()
                ));
                for (name, n) in unbound.iter().take(6) {
                    ui.small(format!("  {name}: {n}"));
                }
            }
            let cam = &play.host.world.camera;
            let o = &cam.override_camera;
            ui.label(format!(
                "Camera: override blend {:.2} (target {}), set {} times, {} frames drawn with it; screen queries {} ({} without a view, {} off screen)",
                o.blend,
                if o.target > 0. { "on" } else { "off" },
                o.set_calls,
                play.override_frames,
                cam.screen_queries,
                cam.screen_queries_without_view,
                cam.screen_queries_off_screen
            ));
            if let Some(c) = &cam.from_to_up {
                ui.small(format!(
                    "Script camera ({} calls, {}; {} frames drawn with it): from {:?} to {:?} up {:?}, lens {:?}",
                    cam.from_to_up_calls,
                    if play.host.world.script_camera().is_some() {
                        "standing"
                    } else {
                        "not standing"
                    },
                    play.script_frames,
                    c.from,
                    c.to,
                    c.up,
                    c.params
                ));
            }
            if play.world_root_placed > 0 {
                ui.small(format!(
                    "World Thing: the level hangs under it, placed {} times",
                    play.world_root_placed
                ));
            }
            for f in &play.faults {
                ui.small(f);
            }
            for w in play.host.world.written.iter().rev().take(4) {
                ui.small(format!("WRITE {w}"));
            }
        });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::CameraProjection;
    use lbp_runtime::OVERRIDE_FOV_DEG;

    /// The view handed to the scripts: a level point in front of the camera lands in the
    /// middle of the runtime's screen, the camera matrix is in level units with axes of
    /// length 1, and a display matrix without an inverse gives no view.
    #[test]
    fn the_view_for_the_scripts_is_in_level_units() {
        // Level units to display units: a scale of 1/1000 after a shift.
        let display = Mat4::from_scale(Vec3::splat(0.001))
            * Mat4::from_translation(Vec3::new(-500., -200., 0.));
        // A camera in display units, 2 in front of the level point (1500, 700, 0).
        let target = display.transform_point3(Vec3::new(1500., 700., 0.));
        let global = Transform::from_translation(target + Vec3::Z * 2.)
            .looking_at(target, Vec3::Y)
            .to_matrix();
        let lens = PerspectiveProjection {
            fov: OVERRIDE_FOV_DEG.to_radians(),
            aspect_ratio: 16. / 9.,
            ..Default::default()
        };
        let view = camera_view(display, global, lens.get_clip_from_view()).unwrap();
        // Column W: the eye in level units, 2000 in front of the point. Column Z points
        // backward (+z here), at length 1.
        let c = view.camera;
        assert!((c[12] - 1500.).abs() < 0.01 && (c[13] - 700.).abs() < 0.01);
        assert!((c[14] - 2000.).abs() < 0.01);
        assert!((c[10] - 1.).abs() < 1e-5 && (c[0] - 1.).abs() < 1e-5);
        let at = |x: f32, y: f32, z: f32| {
            lbp_runtime::screen_position(&view.view_proj, [x, y, z, 1.], lbp_runtime::SCREEN)
        };
        let mid = at(1500., 700., 0.);
        assert!(
            (mid[0] - 640.).abs() < 0.01 && (mid[1] - 360.).abs() < 0.01,
            "{mid:?}"
        );
        // Level +y is up on the screen (the smaller y), level +x to the right; the edge
        // of the picture is 2000 tan 30 above the middle.
        let h = 2000. * 30f32.to_radians().tan();
        let top = at(1500., 700. + h, 0.);
        assert!(
            top[1].abs() < 0.05 && (top[0] - 640.).abs() < 0.01,
            "{top:?}"
        );
        let right = at(1500. + h * 16. / 9., 700., 0.);
        assert!((right[0] - 1280.).abs() < 0.05, "{right:?}");
        assert_eq!(
            camera_view(Mat4::ZERO, global, lens.get_clip_from_view()),
            None
        );
    }

    /// A camera a script wrote, in level units, becomes the viewer's camera in display
    /// units: the eye through the display matrix, the view direction and the up vector
    /// as the runtime's camera matrix has them, the field of view the script's. Drawn
    /// with it, the point the script looks at is in the middle of the picture.
    #[test]
    fn a_script_camera_in_level_units_places_the_viewer_camera() {
        use lbp_runtime::{CameraSource, ScriptCamera};
        use lbp_script_vm::natives::CameraFromToUp;
        let display = Mat4::from_scale(Vec3::splat(0.001))
            * Mat4::from_translation(Vec3::new(-500., -200., 0.));
        // Made-up numbers: an eye, a point 2500 away, one radian of field of view.
        let (from, to) = (Vec3::new(1500., 700., 2000.), Vec3::new(1500., 2200., 0.));
        let call = |up: Vec3| CameraFromToUp {
            from: from.extend(1.).to_array(),
            to: to.extend(1.).to_array(),
            up: up.extend(0.).to_array(),
            params: [10., 100_000., 0., 1.],
        };
        let shown = |up: Vec3| {
            let c = ScriptCamera::of(&call(up), 0).unwrap();
            DisplayCamera {
                source: CameraSource::Script,
                pose: c.pose,
                up: c.up,
                fov_y_deg: c.fov_y_deg(),
                near: c.near,
                far: c.far,
            }
        };
        let camera = shown(Vec3::Y);
        let placed = place_camera(display, &camera);
        let near = |a: Vec3, b: Vec3| (a - b).length() < 1e-5;
        assert!(near(placed.translation, display.transform_point3(from)));
        assert!(near(placed.forward().as_vec3(), Vec3::new(0., 0.6, -0.8)));
        assert!(near(placed.right().as_vec3(), Vec3::X));
        assert!(near(placed.up().as_vec3(), Vec3::new(0., 0.8, 0.6)));
        assert_eq!(placed.scale, Vec3::ONE);
        // The picture: the point looked at in the middle, and a point at the tangent of
        // half the field of view above it (along the camera's up) on the top edge.
        let fov = camera.fov_y_deg.unwrap().to_radians();
        assert!((fov - 1.).abs() < 1e-6);
        let lens = PerspectiveProjection {
            fov,
            aspect_ratio: 16. / 9.,
            ..Default::default()
        };
        let view = camera_view(display, placed.to_matrix(), lens.get_clip_from_view()).unwrap();
        let at = |p: Vec3| {
            lbp_runtime::screen_position(
                &view.view_proj,
                p.extend(1.).to_array(),
                lbp_runtime::SCREEN,
            )
        };
        let mid = at(to);
        assert!(
            (mid[0] - 640.).abs() < 0.01 && (mid[1] - 360.).abs() < 0.01,
            "{mid:?}"
        );
        let top = at(to + Vec3::new(0., 0.8, 0.6) * 2500. * 0.5f32.tan());
        assert!(
            top[1].abs() < 0.05 && (top[0] - 640.).abs() < 0.01,
            "{top:?}"
        );
        // The matrix handed to the scripts is the one the script wrote.
        let written = ScriptCamera::of(&call(Vec3::Y), 0).unwrap().matrix();
        for (a, b) in view.camera.iter().zip(&written) {
            assert!((a - b).abs() < 0.01, "{:?} {written:?}", view.camera);
        }
        // The script's up vector rolls the camera: with +X as up, the picture's up is
        // +X and its right is what was down.
        let rolled = place_camera(display, &shown(Vec3::X));
        assert!(near(rolled.up().as_vec3(), Vec3::X));
        assert!(near(rolled.forward().as_vec3(), Vec3::new(0., 0.6, -0.8)));
        // A level drawn under its world Thing: the root takes a display point where
        // the moved world Thing takes its level point.
        let moved = Mat4::from_translation(Vec3::new(10., 20., -3000.))
            * Mat4::from_scale(Vec3::splat(2.))
            * Mat4::from_rotation_x(0.15);
        let root = world_root(display, moved);
        let point = Vec3::new(-2296., 1132., 40.);
        let want = display.transform_point3(moved.transform_point3(point));
        let got = root.transform_point(display.transform_point3(point));
        assert!((want - got).length() < 1e-4, "{want:?} {got:?}");
        // A world Thing where the level file has it leaves the level where it is.
        let still = world_root(display, Mat4::IDENTITY).to_matrix();
        assert!(still.abs_diff_eq(Mat4::IDENTITY, 1e-5), "{still:?}");
    }
}
