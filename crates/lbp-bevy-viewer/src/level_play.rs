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
//! 23). With the Pod's shell layer the shell level has a script world of its own, and
//! the two tick in one session (part 28). The rest is the clock and the counts.
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::level_scene;
use lbp_runtime::{
    tick_session, CameraPose, CameraSource, CameraView, DisplayCamera, LevelScripts, LevelWorld,
    LightFollower, SessionLevel, StepReport, TICK_HZ,
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
    /// `--level-scripts-in-use-at N` (QA, part 27): the pod computer is set in use when
    /// N ticks have run, before the next one; none once that is done.
    in_use_at: Option<u64>,
    /// Frames drawn with a camera a script wrote itself (part 23), that camera's state
    /// at the last printed line, and its pose after the last tick it was looked at.
    script_frames: u64,
    script_state: ScriptCameraState,
    script_last: Option<(u64, CameraPose)>,
    /// The Pod's shell layer under the script's camera (part 26): how often the shell
    /// and tent roots were placed from it, and whether the first placing was printed.
    shell_placed: u64,
    shell_placed_printed: bool,
    /// The Puter state of the viewer's Pod rig that was last handed to the script world
    /// as "pod computer in use" (part 26).
    puter_open: Option<bool>,
    /// The matrix the root of a level that hangs under its world Thing was last placed
    /// by ([`follow_world`]), and how often it was placed.
    world_root: Option<[f32; 16]>,
    world_root_placed: u64,
    /// The Pod's shell level (pod.bin), played in one session with the level above
    /// (part 28): there under `--pod-camera N` with `--pod-shell HEX`.
    shell: Option<ShellLevel>,
}

/// The script world of the Pod's shell layer (part 28): the second level of the
/// session, with a VM of its own (a Thing handle is an index into one level world).
/// It ticks before the level it is drawn over. No camera is written for it and none is
/// read from it: pod.bin's scripts ask for none, and which camera the game draws the
/// shell with is not established.
struct ShellLevel {
    vm: Vm,
    host: Bindings<LevelWorld>,
    scripts: LevelScripts,
    created: Option<StepReport>,
    /// Sums over update ticks.
    totals: StepReport,
    ticks_with_faults: usize,
    /// The shell scene whose draw entities were last set from this world
    /// (`thing_view::ShellThings`), and what was done to them since the start.
    things_scene: Option<u64>,
    followed: crate::thing_view::Followed,
    /// The opening progress after the last tick ([`opening`]).
    opening: f32,
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

/// The tick count after `flag`: none without the flag, an error for another value.
fn tick_option(all: &[String], flag: &str) -> Result<Option<u64>, String> {
    value(all, flag)
        .map(|v| {
            v.parse::<u64>()
                .map_err(|_| format!("{flag} takes a tick count"))
        })
        .transpose()
}

/// `--level-scripts-in-use-at N` (QA, part 27): whether to set "pod computer in use"
/// now: once, when `ticks` ticks have run and that is the option's count or more.
/// `at` is cleared when it fires.
fn in_use_now(at: &mut Option<u64>, ticks: u64) -> bool {
    at.take_if(|n| ticks >= *n).is_some()
}

/// Sets "pod computer in use" for the session: on the played level's world and, with
/// the shell layer, on the shell's. Both, because the first level of a session tick
/// hands its own value to the others afterwards (`lbp_runtime::tick_session`): a value
/// set on the second world only is written over before that world ticks.
fn set_in_use(level: &mut LevelWorld, shell: Option<&mut LevelWorld>, in_use: bool) {
    level.pod_computer_in_use = in_use;
    if let Some(shell) = shell {
        shell.pod_computer_in_use = in_use;
    }
}

/// The opening progress of the viewer's Pod rig for the change-over value of pod.bin's
/// script (`LevelWorld::pod_transition`): the value kept inside 0..1; 0 before the
/// script's first update and for a value that is not a number.
///
/// PLACEHOLDER: FOUND uses of the value are the pod's shape colour alpha, the pod
/// decorations' fade, the pod's mesh at 1 and the mask texture. Nothing read ties the
/// shell's opening (the viewer's fitted shell eyes) or the tent's slide out of the frame
/// to it; the camera the game draws the shell level with is not established.
fn opening(transition: Option<f32>) -> f32 {
    match transition {
        Some(t) if t.is_finite() => t.clamp(0., 1.),
        _ => 0.,
    }
}

/// What to say of the opening progress on a tick: something when it leaves one of its
/// two ends, and when it arrives at one.
fn opening_event(was: f32, now: f32) -> Option<&'static str> {
    let at_end = |t: f32| t == 0. || t == 1.;
    match (was == now, at_end(was), at_end(now)) {
        (true, ..) => None,
        (_, _, true) if now == 1. => Some("open"),
        (_, _, true) => Some("closed"),
        (_, true, false) if now > was => Some("opening"),
        (_, true, false) => Some("closing"),
        _ => None,
    }
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
    // Part 28: with the Pod's shell layer (`--pod-camera N --pod-shell HEX`, the
    // condition under which the viewer builds its Pod rig) the shell level is played too.
    let shell_guid = value(&all, "--pod-shell")
        .filter(|_| value(&all, "--pod-camera").is_some())
        .map(|g| u32::from_str_radix(&g, 16).map_err(|_| "--pod-shell requires hex digits"))
        .transpose()?;
    let hold_at = tick_option(&all, "--level-scripts-hold-at")?;
    let in_use_at = tick_option(&all, "--level-scripts-in-use-at")?;
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
    type Loaded = (Vm, LevelWorld, String, Option<(Vm, LevelWorld, String)>);
    let (mut vm, mut world, header, shell) = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<Loaded, String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(Path::new(&snapshot), &control)?;
            let source = AttachedSource::attach(Path::new(&game), &catalog, &control)?;
            let (scripts, not_read) = lbp_ui_script::load::load(&source, &catalog, &control)?;
            let mut vm = Vm::default();
            // The shell level's scripts run on a VM of their own, loaded in the same
            // order (as the `level_scripts` example's second level).
            let mut shell_vm = shell_guid.map(|_| Vm::default());
            let loaded = scripts.len();
            for (g, s) in scripts {
                if let Some(shell_vm) = shell_vm.as_mut() {
                    shell_vm.load(s.clone(), g);
                }
                vm.load(s, g);
            }
            let read_world = |guid: Option<u32>, path: Option<&str>| {
                let (key, identity) = level_scene::resolve_level(&catalog, guid, path)?;
                let bytes = level_scene::read_level_bytes(&source, &key, &control)?;
                let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>((LevelWorld::from_things(&report.things), identity))
            };
            let (mut world, identity) = read_world(guid, path.as_deref())?;
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
            let shell = shell_vm
                .zip(shell_guid)
                .map(|(shell_vm, g)| {
                    let (mut shell, identity) = read_world(Some(g), None)?;
                    shell.translations.clone_from(&world.translations);
                    Ok::<_, String>((shell_vm, shell, identity))
                })
                .transpose()?;
            Ok((vm, world, header, shell))
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
    // Part 28: the shell level's world, set up the same way, and the session's start.
    let shell = shell.map(|(mut shell_vm, mut shell_world, identity)| {
        let built = shell_world.bind_objects(&mut shell_vm);
        shell_world.add_class_defaults(&shell_vm);
        let mut shell_host = Bindings::defaulting(shell_world);
        let engine = lbp_runtime::add_engine_objects(&mut shell_vm, &mut shell_host, BUDGET_PER_CALL);
        let room = MAX_FAULT_LINES.saturating_sub(faults.len());
        faults.extend(engine.into_iter().take(room).map(|f| format!("shell: {f}")));
        let shell_scripts = LevelScripts::new(&shell_vm, &shell_host.world);
        // The session is our Pod when the shell has a pod Thing: the stand-in of the
        // `level_scripts` example. PLACEHOLDER: the game tests the slot the level was
        // entered from. Then the two worlds agree on the state outside a level.
        host.world.our_pod = shell_host.world.pod_thing(true).is_some();
        host.world.hand_session_to(&mut shell_host.world);
        println!(
            "Level scripts, shell level: {identity}; scripted Things {}; objects built {built}, unconverted {}; one session with the level above, the shell ticks first; our Pod {}",
            shell_scripts.instances.len(),
            shell_host.world.unconverted,
            host.world.our_pod
        );
        ShellLevel {
            vm: shell_vm,
            host: shell_host,
            scripts: shell_scripts,
            created: None,
            totals: StepReport::default(),
            ticks_with_faults: 0,
            things_scene: None,
            followed: Default::default(),
            opening: 0.,
        }
    });
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
        in_use_at,
        script_frames: 0,
        script_state: ScriptCameraState::Out,
        script_last: None,
        shell_placed: 0,
        shell_placed_printed: false,
        puter_open: None,
        world_root: None,
        world_root_placed: 0,
        shell,
    })
    .insert_resource(crate::CaptureHold(hold_at.is_some()))
    .insert_resource(Time::<Fixed>::from_hz(f64::from(TICK_HZ)))
    .add_systems(
        FixedUpdate,
        (tick, relight, follow_fog, follow_things, follow_world).chain(),
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
    mut puter: Option<ResMut<crate::pod_menu::PuterState>>,
) {
    let p = &mut *play;
    // Part 26: the Puter key of the viewer's Pod rig is the player at the pod computer.
    // Handed over when it changes, also while the ticks are held.
    if let Some(open) = puter_change(&mut p.puter_open, puter.as_deref().map(|s| s.open)) {
        let shell = p.shell.as_mut().map(|s| &mut s.host.world);
        set_in_use(&mut p.host.world, shell, open);
        println!(
            "Pod computer at tick {}: in use {open}, from the viewer's Puter state (stand-in for the pod's own script)",
            p.scripts.ticks
        );
    }
    step(p, &mut capture, &sackboy);
    // Part 28: the opening progress of the viewer's Pod rig is the change-over value of
    // pod.bin's script after the last tick (PLACEHOLDER, see [`opening`]). Also while
    // the ticks are held, for a rig that came up after them.
    if let (Some(shell), Some(puter)) = (&p.shell, puter.as_mut()) {
        if puter.t != shell.opening {
            puter.t = shell.opening;
        }
    }
}

/// One fixed step of the scripts: the create step first, then a tick each, unless held.
fn step(
    p: &mut Play,
    capture: &mut ResMut<crate::CaptureHold>,
    sackboy: &Query<(&Position, &lbp_sim::ZSlab), With<SackboyBody>>,
) {
    // `--level-scripts-hold-at N`: a capture waits for tick N, and no tick follows it.
    let held = p.hold_at.is_some_and(|n| p.scripts.ticks >= n);
    if capture.0 != (p.hold_at.is_some() && !held) {
        capture.0 = p.hold_at.is_some() && !held;
    }
    if p.created.is_none() {
        // Part 28: the shell is created first, and each hands its session state on (the
        // order of the `level_scripts` example).
        if let Some(shell) = p.shell.as_mut() {
            let r = shell
                .scripts
                .create(&mut shell.vm, &mut shell.host, BUDGET_PER_CALL);
            shell.host.world.hand_session_to(&mut p.host.world);
            note_shell(&mut p.faults, &r);
            shell.created = Some(r);
        }
        let r = p.scripts.create(&mut p.vm, &mut p.host, BUDGET_PER_CALL);
        if let Some(shell) = p.shell.as_mut() {
            p.host.world.hand_session_to(&mut shell.host.world);
        }
        note(p, &r);
        p.created = Some(r);
        return;
    }
    if held {
        return;
    }
    // Part 27, `--level-scripts-in-use-at N`: a player is at the pod computer from the
    // tick after tick N on (as `level_scripts --puter-in-use=N`). A stand-in for the
    // path of part 24 (pad zone, press, the host's answer), which needs a Sackboy and a
    // pad the viewer does not feed in yet.
    if in_use_now(&mut p.in_use_at, p.scripts.ticks) {
        let shell = p.shell.as_mut().map(|s| &mut s.host.world);
        set_in_use(&mut p.host.world, shell, true);
        println!(
            "Pod computer at tick {}: in use true, from --level-scripts-in-use-at (stand-in for the player engaging it)",
            p.scripts.ticks
        );
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
    let r = match p.shell.as_mut() {
        // Part 28: both Pod levels in one session, the shell first. INFERRED: the
        // order of the levels in the game's frame was not read; MEASURED that the other
        // order gives the same events on the same ticks (part 24).
        Some(shell) => {
            let mut levels = [
                SessionLevel {
                    scripts: &mut shell.scripts,
                    vm: &mut shell.vm,
                    host: &mut shell.host,
                },
                SessionLevel {
                    scripts: &mut p.scripts,
                    vm: &mut p.vm,
                    host: &mut p.host,
                },
            ];
            let mut reports = tick_session(&mut levels, BUDGET_PER_CALL).into_iter();
            let (of_shell, of_level) = (reports.next(), reports.next());
            let of_shell = of_shell.unwrap_or_default();
            shell.ticks_with_faults += usize::from(!of_shell.faults.is_empty());
            note_shell(&mut p.faults, &of_shell);
            shell.totals.absorb(of_shell);
            let was = std::mem::replace(
                &mut shell.opening,
                opening(shell.host.world.pod_transition()),
            );
            if let Some(what) = opening_event(was, shell.opening) {
                println!(
                    "Pod change-over at tick {}: {what}, value {:.2} from pod.bin's script; the shell's opening progress follows it (placeholder: the game's shell camera is not established)",
                    shell.scripts.ticks, shell.opening
                );
            }
            of_level.unwrap_or_default()
        }
        None => p
            .scripts
            .tick_level(&mut p.vm, &mut p.host, BUDGET_PER_CALL),
    };
    p.ticks_with_faults += usize::from(!r.faults.is_empty());
    note(p, &r);
    p.totals.absorb(r);
    print_messages(p);
}

/// The shell level's faults, in the same list as the level's.
fn note_shell(faults: &mut Vec<String>, r: &StepReport) {
    for (thing, f) in &r.faults {
        if faults.len() < MAX_FAULT_LINES {
            faults.push(format!("shell thing {thing}: {f}"));
        }
    }
}

/// The Puter state to hand to the script world as "pod computer in use": `now` when it
/// is not the value handed last (`last`, which is updated). A closed Puter that was
/// never open hands nothing: the world starts not in use. Nothing without a Pod rig.
fn puter_change(last: &mut Option<bool>, now: Option<bool>) -> Option<bool> {
    let now = now?;
    let was = last.replace(now);
    (was != Some(now) && (now || was.is_some())).then_some(now)
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

/// The fog bias a script handed the camera (part 25): the third value of the last
/// `Cam.SetDOFAndFog` call, level units; 0 before any call and for a value that is not
/// a number.
fn fog_bias(world: &LevelWorld) -> f32 {
    let bias = world.camera.dof_and_fog.map_or(0., |v| v[2]);
    if bias.is_finite() {
        bias
    } else {
        0.
    }
}

/// Hands the scripts' fog bias to the scene's light when it changes (part 25). The
/// preview fog of `--preview-lighting` then starts and ends that much further out
/// (`lighting::Fog::biased`); the default lighting fogs in its shader and is not touched.
/// Like [`relight`], nothing until the scene has been drawn for
/// [`RELIGHT_AFTER_FRAMES`]. One line per change.
fn follow_fog(play: NonSend<Play>, mut v: ResMut<crate::Viewer>) {
    if v.ready_frames < RELIGHT_AFTER_FRAMES {
        return;
    }
    let bias = fog_bias(&play.host.world);
    if bias == v.lighting.fog_bias {
        return;
    }
    v.lighting.fog_bias = bias;
    v.lighting.dirty = true;
    println!(
        "Fog bias at tick {}: {bias:.1} level units from the scripts (Cam.SetDOFAndFog), added to the start and the end of the preview fog; the first two values (depth of field) are not applied",
        play.scripts.ticks
    );
}

/// Sets the level's draw entities from the script world (part 19): the Things on the
/// world's dirty list each tick, and for a scene that was just spawned every mapped
/// Thing that is hidden or unlike the level file (scripts tick while the scene loads).
/// Like [`relight`], nothing until the scene has been drawn for
/// [`RELIGHT_AFTER_FRAMES`]: the same guard, since a first tint makes a new material.
/// One line per tick on which the scene changed: the check that needs no screenshot.
///
/// Part 28: with the Pod's shell layer the same is done for the shell level, from its
/// own script world through its own map (`thing_view::ShellThings`); its line starts
/// with `Shell Things`.
fn follow_things(
    mut play: NonSendMut<Play>,
    mut v: ResMut<crate::Viewer>,
    things: Option<Res<crate::thing_view::LevelThings>>,
    shell_things: Option<Res<crate::thing_view::ShellThings>>,
    mut entities: Query<crate::thing_view::Parts>,
    mut materials: ResMut<Assets<crate::ViewerMaterial>>,
) {
    let p = &mut *play;
    let ready = v.ready_frames >= RELIGHT_AFTER_FRAMES;
    if let Some(things) = &things {
        let state = (&mut p.things_scene, &mut p.followed);
        let level = (&mut p.host.world, p.scripts.ticks, "Things");
        follow_level(
            level,
            things,
            state,
            ready,
            &mut v,
            &mut entities,
            &mut materials,
        );
    }
    if let (Some(shell), Some(things)) = (p.shell.as_mut(), &shell_things) {
        let state = (&mut shell.things_scene, &mut shell.followed);
        let level = (&mut shell.host.world, shell.scripts.ticks, "Shell Things");
        follow_level(
            level,
            &things.0,
            state,
            ready,
            &mut v,
            &mut entities,
            &mut materials,
        );
    }
}

/// [`follow_things`] for one level: its script world, tick count and the name of its
/// printed line; its map; the scene last set from the world and the sums.
fn follow_level(
    (world, ticks, label): (&mut LevelWorld, u64, &str),
    things: &crate::thing_view::LevelThings,
    (things_scene, followed): (&mut Option<u64>, &mut crate::thing_view::Followed),
    ready: bool,
    v: &mut ResMut<crate::Viewer>,
    entities: &mut Query<crate::thing_view::Parts>,
    materials: &mut Assets<crate::ViewerMaterial>,
) {
    if !ready {
        *things_scene = None;
        return;
    }
    let first = *things_scene != Some(things.scene);
    let list = if first {
        *things_scene = Some(things.scene);
        world.take_display_dirty();
        crate::thing_view::first_list(world, things)
    } else {
        world.take_display_dirty()
    };
    if list.is_empty() {
        return;
    }
    let done = crate::thing_view::apply(world, &list, things, entities, materials, &mut v.scene);
    followed.absorb(&done);
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
        // Part 27: a mesh other than the level file's, as (Thing, mesh GUID).
        let other_mesh: Vec<(u32, String)> = some(&|t, d| {
            world
                .display_at_load(t)
                .is_some_and(|was| was.mesh != d.mesh)
        })
        .into_iter()
        .filter_map(|t| Some((t, format!("{:08x}", world.thing_display(t)?.mesh?))))
        .collect();
        println!(
            "{label} at tick {ticks}{}: {}; first Things hidden {hidden:?} (of them by a script {by_script:?}), tinted {tinted:?}, with another mesh than the file's {other_mesh:?}",
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
/// With the Pod's shell layer (`--pod-shell` with `--pod-camera`; part 26) the shell and
/// tent roots are then placed from the camera a script wrote: its eye and turn, and the
/// opening progress `t` of the viewer's Puter state (`PodRig::shell_root_at`), which
/// [`tick`] writes from the change-over value of pod.bin's script (part 28;
/// PLACEHOLDER, see [`opening`]). A shell point keeps its place in the frame; the
/// planets behind it are the script's view. While no script camera is shown the roots
/// stay where they were. The shell's own script world gets no camera view here.
///
/// INFERRED: world +Y is up under the override. Not applied: the near and far planes of
/// either script camera (the viewer keeps its own, in display units). One line is
/// printed when a camera comes in, is fully in or comes to rest, or goes out: the check
/// without a screenshot.
fn script_camera(
    mut play: NonSendMut<Play>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
    (rig, puter, mut shell_roots): ShellLayer,
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
    let shown = world.display_camera(follow_eye.to_array(), follow_dir.to_array());
    let own_fov = match projection.as_ref() {
        Projection::Perspective(lens) => Some(lens.fov),
        _ => None,
    };
    let wanted_fov = match &shown {
        Some(camera) => {
            *transform = place_camera(display.0, camera);
            // Part 26: the Pod's shell layer follows the camera a script wrote.
            if let (CameraSource::Script, Some(rig), Some(puter)) = (camera.source, &rig, &puter) {
                let (eye, turn, t) = (transform.translation, transform.rotation, puter.t);
                for (mut root, tent) in &mut shell_roots {
                    let placed = if tent {
                        rig.0.tent_root_at(eye, turn, t)
                    } else {
                        rig.0.shell_root_at(eye, turn, t)
                    };
                    if *root != placed {
                        *root = placed;
                        p.shell_placed += 1;
                    }
                }
                if !p.shell_placed_printed && p.shell_placed > 0 {
                    p.shell_placed_printed = true;
                    println!(
                        "Pod shell layer at tick {}: placed from the script's camera (display eye {:.3?}, opening {t:.2}, shell scale {:.4})",
                        p.scripts.ticks,
                        eye.to_array(),
                        rig.0.scale
                    );
                }
            }
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

/// The Pod rig, its Puter state and the shell and tent roots, for [`script_camera`]
/// (part 26). The rig is there only with the shell layer.
type ShellLayer<'w, 's> = (
    Option<Res<'w, crate::PodRigRes>>,
    Option<Res<'w, crate::pod_menu::PuterState>>,
    Query<
        'w,
        's,
        (&'static mut Transform, Has<crate::PodTentRoot>),
        (With<crate::PodShellRoot>, Without<Camera3d>),
    >,
);

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
            if let Some(shell) = &play.shell {
                ui.label(format!(
                    "Shell level: tick {}; suspended {}; stopped by a fault {}; ticks with faults {}; in use {}; change-over {:.2}",
                    shell.scripts.ticks,
                    shell.scripts.pending(),
                    shell.scripts.stopped.len(),
                    shell.ticks_with_faults,
                    shell.host.world.pod_computer_in_use,
                    shell.opening
                ));
                if let Some(c) = &shell.created {
                    ui.small(format!("  create: {}", line(c)));
                }
                ui.small(format!("  ticks: {}", line(&shell.totals)));
                ui.small(match shell.things_scene {
                    Some(_) => format!("  Things: {}", shell.followed.line()),
                    None => "  Things: the scene does not follow them (yet)".into(),
                });
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

    /// Part 25. A planet under the root of its level: the Thing map gives its model the
    /// matrix "display x moved x item", the root "display x world Thing moved x
    /// display^-1", so a point of the model lands where the world Thing's matrix takes
    /// the place the planet's own script gave it. And the fog bias is the third value
    /// of the scripts' last `SetDOFAndFog`.
    #[test]
    fn a_planet_its_script_placed_is_drawn_under_the_world_thing_and_the_fog_bias() {
        use lbp_script_vm::natives::dispatch;
        let display = Mat4::from_scale(Vec3::splat(0.001))
            * Mat4::from_translation(Vec3::new(-500., -200., 0.));
        // Made-up numbers. The level file has the planet turned and off to one side;
        // its mesh's bind pose is 1.5 times its size.
        let stored =
            Mat4::from_translation(Vec3::new(-2000., 1000., 40.)) * Mat4::from_rotation_x(1.2);
        let bind = Mat4::from_scale(Vec3::splat(1.5));
        let item = stored * bind;
        // The script puts it at a place of its own, unturned, at 0.9 times the size.
        let now =
            Mat4::from_translation(Vec3::new(-1250., 0., 0.)) * Mat4::from_scale(Vec3::splat(0.9));
        let was = lbp_runtime::ThingDisplay {
            world: Some(stored.to_cols_array()),
            ..Default::default()
        };
        let is = lbp_runtime::ThingDisplay {
            world: Some(now.to_cols_array()),
            ..Default::default()
        };
        let by = Mat4::from_cols_array(&is.moved_from(&was).unwrap());
        // What `thing_view::apply` gives the entity, and what `follow_world` the root.
        let child = Transform::from_matrix(display * by * item);
        let world_moved = Mat4::from_translation(Vec3::new(0., 4700., -30000.))
            * Mat4::from_scale(Vec3::splat(2.));
        let root = world_root(display, world_moved);
        let point = Vec3::new(30., -40., 50.);
        let got = root.transform_point(child.transform_point(point));
        let want = display
            .transform_point3(world_moved.transform_point3((now * bind).transform_point3(point)));
        assert!((got - want).length() < 1e-4, "{got:?} {want:?}");
        // Its size on the screen is the script's size times the bind pose's, times the
        // world Thing's: 0.9 x 1.5 x 2 level units a unit, in display units.
        let size = (root.to_matrix() * child.to_matrix())
            .transform_vector3(Vec3::X)
            .length();
        assert!((size - 0.9 * 1.5 * 2. * 0.001).abs() < 1e-7, "{size}");

        let mut world = LevelWorld::default();
        assert_eq!(fog_bias(&world), 0.);
        let f = lbp_script_vm::Value::Float;
        let set = |world: &mut LevelWorld, bias: f32| {
            let args = [f(80000.), f(100000.), f(bias)];
            dispatch(world, "Cam", "SetDOFAndFog__fff", &args)
                .unwrap()
                .unwrap();
        };
        set(&mut world, 20000.);
        assert_eq!(fog_bias(&world), 20000.);
        set(&mut world, 0.);
        assert_eq!(fog_bias(&world), 0.);
        set(&mut world, f32::NAN);
        assert_eq!(fog_bias(&world), 0.);
    }

    /// Part 27. `--level-scripts-in-use-at N` is a tick count, and fires once: on the
    /// first look at which N ticks or more have run.
    #[test]
    fn the_in_use_option_is_a_tick_count_and_fires_once() {
        let args = |list: &[&str]| -> Vec<String> { list.iter().map(|a| a.to_string()).collect() };
        let flag = "--level-scripts-in-use-at";
        assert_eq!(tick_option(&args(&["--level-scripts"]), flag), Ok(None));
        assert_eq!(tick_option(&args(&[flag, "60"]), flag), Ok(Some(60)));
        assert_eq!(tick_option(&args(&[flag, "0"]), flag), Ok(Some(0)));
        for bad in ["-1", "soon", "1.5"] {
            let e = tick_option(&args(&[flag, bad]), flag).unwrap_err();
            assert_eq!(e, "--level-scripts-in-use-at takes a tick count");
        }
        let mut at = Some(60);
        assert!(!in_use_now(&mut at, 0) && !in_use_now(&mut at, 59));
        assert_eq!(at, Some(60));
        assert!(in_use_now(&mut at, 60));
        assert_eq!(at, None);
        assert!(!in_use_now(&mut at, 61));
        // A count already passed (the option's 0, or ticks run before the look).
        assert!(in_use_now(&mut Some(0), 0) && in_use_now(&mut Some(3), 7));
        assert!(!in_use_now(&mut None, 7));
    }

    /// Part 28. The opening progress is the change-over value inside 0..1, and 0 while
    /// there is none; a line is due when it leaves an end and when it reaches one.
    #[test]
    fn d277_the_opening_is_the_change_over_value_clamped() {
        assert_eq!(opening(None), 0.);
        assert_eq!(opening(Some(0.35)), 0.35);
        assert_eq!(opening(Some(0.)), 0.);
        assert_eq!(opening(Some(1.)), 1.);
        assert_eq!(opening(Some(-0.2)), 0.);
        assert_eq!(opening(Some(1.05)), 1.);
        for odd in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(opening(Some(odd)), 0.);
        }
        assert_eq!(opening_event(0., 0.), None);
        assert_eq!(opening_event(0., 0.05), Some("opening"));
        assert_eq!(opening_event(0.05, 0.1), None);
        assert_eq!(opening_event(0.95, 1.), Some("open"));
        assert_eq!(opening_event(1., 1.), None);
        assert_eq!(opening_event(1., 0.95), Some("closing"));
        assert_eq!(opening_event(0.5, 0.45), None);
        assert_eq!(opening_event(0.05, 0.), Some("closed"));
        // A snap from end to end is an arrival.
        assert_eq!(opening_event(0., 1.), Some("open"));
        assert_eq!(opening_event(1., 0.), Some("closed"));
    }

    /// Part 28. "In use" is set on both worlds of the session. The first level of a
    /// session tick hands its own value on, so a value set on the second world alone
    /// is lost before that world ticks: the trap `set_in_use` is there for.
    #[test]
    fn d277_in_use_is_set_on_both_worlds_of_the_session() {
        struct Side(Vm, Bindings<LevelWorld>, LevelScripts);
        let side = || {
            let (vm, host) = (Vm::default(), Bindings::defaulting(LevelWorld::default()));
            let scripts = LevelScripts::new(&vm, &host.world);
            Side(vm, host, scripts)
        };
        // The viewer's order: the shell, then the level.
        let round = |shell: &mut Side, level: &mut Side| {
            let mut levels = [
                SessionLevel {
                    scripts: &mut shell.2,
                    vm: &mut shell.0,
                    host: &mut shell.1,
                },
                SessionLevel {
                    scripts: &mut level.2,
                    vm: &mut level.0,
                    host: &mut level.1,
                },
            ];
            assert_eq!(tick_session(&mut levels, BUDGET_PER_CALL).len(), 2);
        };
        let in_use = |shell: &Side, level: &Side| {
            (
                shell.1.world.pod_computer_in_use,
                level.1.world.pod_computer_in_use,
            )
        };
        let (mut shell, mut level) = (side(), side());
        set_in_use(&mut level.1.world, Some(&mut shell.1.world), true);
        assert_eq!(in_use(&shell, &level), (true, true));
        round(&mut shell, &mut level);
        assert_eq!(in_use(&shell, &level), (true, true));
        assert_eq!((shell.2.ticks, level.2.ticks), (1, 1));
        set_in_use(&mut level.1.world, Some(&mut shell.1.world), false);
        round(&mut shell, &mut level);
        assert_eq!(in_use(&shell, &level), (false, false));
        // The trap: set on the level's world only, the shell's hand-over takes it back.
        level.1.world.pod_computer_in_use = true;
        round(&mut shell, &mut level);
        assert_eq!(in_use(&shell, &level), (false, false));
        // Without a shell the one world is set, as before.
        let mut alone = LevelWorld::default();
        set_in_use(&mut alone, None, true);
        assert!(alone.pod_computer_in_use);
    }

    /// Part 26. The Puter state of the Pod rig goes to the script world when it changes;
    /// a Puter that starts closed hands nothing, one that starts open hands "in use".
    #[test]
    fn the_puter_state_is_handed_to_the_scripts_when_it_changes() {
        let mut last = None;
        assert_eq!(puter_change(&mut last, None), None);
        assert_eq!(last, None);
        assert_eq!(puter_change(&mut last, Some(false)), None);
        assert_eq!(puter_change(&mut last, Some(false)), None);
        assert_eq!(puter_change(&mut last, Some(true)), Some(true));
        assert_eq!(puter_change(&mut last, Some(true)), None);
        assert_eq!(puter_change(&mut last, Some(false)), Some(false));
        assert_eq!(puter_change(&mut last, Some(false)), None);
        // No rig for a while (another level): the state handed last is kept.
        assert_eq!(puter_change(&mut last, None), None);
        assert_eq!(puter_change(&mut last, Some(false)), None);
        let mut open_at_start = None;
        assert_eq!(puter_change(&mut open_at_start, Some(true)), Some(true));
    }
}
