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
//! the two tick in one session (part 28), and the tent of the shell's pod Thing is drawn
//! with the game's top-down dissolve while pod.bin's change-over runs (part 29), and the
//! pod computer is entered by the pod's own script: a Sackboy stands on its trigger and
//! the Puter key is the engage button on the pad the scripts read (part 30); with
//! `--play-sackboy` that Sackboy is the viewer's own, who stands and walks in the shell
//! level, and the pad zone is decided by where he is (part 32). The rest is the clock and
//! the counts.
use crate::pod_menu::PuterKey;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::level_scene;
use lbp_runtime::{
    tick_session, CameraPose, CameraSource, CameraView, DisplayCamera, LevelScripts, LevelWorld,
    LightFollower, SessionLevel, StepReport, TICK_HZ,
};
use lbp_script_vm::natives::{Bindings, ACTION_ENGAGE_POD_BUTTON, PAD_ZONE_MIN_Z};
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
    /// The camera's own lens (field of view, near and far plane, display units), kept
    /// while a script camera or a script's override camera has the lens, to put back
    /// when it is out (part 34).
    follow_lens: Option<Lens>,
    /// Frames drawn with the override camera, and the override's state at the last
    /// printed line: (in, blend at 1, target on).
    override_frames: u64,
    override_printed: (bool, bool, bool),
    /// `--level-scripts-hold-at N` (QA): no tick after tick N.
    hold_at: Option<u64>,
    /// `--level-scripts-in-use-at N` (QA, part 27): the pod computer is set in use when
    /// N ticks have run, before the next one; none once that is done.
    in_use_at: Option<u64>,
    /// `--level-scripts-press-at N` (QA, part 30): the engage button is down on tick N
    /// alone; none once that is done.
    press_at: Option<u64>,
    /// Frames drawn with a camera a script wrote itself (part 23), that camera's state
    /// at the last printed line, and its pose after the last tick it was looked at.
    script_frames: u64,
    script_state: ScriptCameraState,
    script_last: Option<(u64, CameraPose)>,
    /// Part 31: the camera pose the depths of the two Pod levels were last printed for,
    /// and whether the ticks were held then ([`layer_depths`]).
    depths_printed: Option<(CameraPose, bool)>,
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
    /// Part 29: the height of the eye of the camera a script wrote, level units, as last
    /// shown: the camera the tent's dissolve line is measured from.
    script_eye_y: Option<f32>,
    /// Part 29: the dissolve's first line was printed.
    dissolve_printed: bool,
    /// D284: the platform edge's line at value 1 was printed.
    dissolve_edge_printed: bool,
    /// D298: the depth-of-field range last printed (level units).
    dof_printed: Option<[f32; 2]>,
}

/// The script world of the Pod's shell layer (part 28): the second level of the
/// session, with a VM of its own (a Thing handle is an index into one level world).
/// It ticks before the level it is drawn over. No camera is written for it and none is
/// read from it: pod.bin's scripts ask for none. Part 31: it is drawn with the camera
/// the planets level's script writes, in its own level frame ([`shell_frame_root`]).
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
    /// Part 30: the Sackboy that stands on the pod computer's trigger, once the first
    /// tick is about to run (`Some(None)`: the level has no such trigger).
    sackboy: Option<Option<u32>>,
    /// Part 30: an engage press waits for the next tick ([`pad_word`]).
    press: bool,
    /// Part 30: the host's answers that changed "in use" in this world, as last printed.
    answers: usize,
    /// Part 32: the Sackboy is the viewer's own (`--play-sackboy`), who plays in this
    /// level; else the undrawn one on the trigger (part 30).
    own_sackboy: bool,
    /// Part 32: whether the viewer's own Sackboy was in the pad zone after the last tick,
    /// as last printed.
    zone: Option<bool>,
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

/// `--level-scripts-press-at N` (QA, part 30): whether the engage button goes down on
/// the tick about to run, the tick numbered `next` (the first is 1): once, when that is
/// tick N or a later one. `at` is cleared when it fires. The count is a tick's number,
/// as `level_scripts --press=TICK:MASK`, where `--level-scripts-in-use-at` counts the
/// ticks that have run.
fn press_now(at: &mut Option<u64>, next: u64) -> bool {
    at.take_if(|n| next >= *n).is_some()
}

/// What a Puter key of the shell layer does under the scripts (part 30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyAct {
    /// The engage button on the pad, for one tick.
    Press,
    /// A request to the host to stop using the pod computer.
    Close,
}

/// The act of a key by "pod computer in use": the opening key presses the engage button
/// while the computer is free and asks to close while it is in use; the closing key
/// asks to close while it is in use and does nothing else.
fn key_act(key: PuterKey, in_use: bool) -> Option<KeyAct> {
    match (key, in_use) {
        (_, true) => Some(KeyAct::Close),
        (PuterKey::Toggle, false) => Some(KeyAct::Press),
        (PuterKey::Back, false) => None,
    }
}

/// The button word of player 0's pad for the tick about to run: the engage bit when a
/// press waits, which is taken; else 0. So a press is down for one tick and up on the
/// next, which is what the game's "just clicked" wants: down now, up the frame before.
///
/// FOUND (part 24): action 8, the one the Pod's script asks about, is bit 0x80 of the
/// pad's button word. Which button of the pad that is, is not established: the keys
/// here are the viewer's own Puter keys (Space, gamepad Square), not a finding.
fn pad_word(press: &mut bool) -> u32 {
    if std::mem::take(press) {
        ACTION_ENGAGE_POD_BUTTON
    } else {
        0
    }
}

/// Hands the session's pad to both worlds for the tick about to run and returns its
/// button word ([`pad_word`]). Both, as `level_scripts --press`: the pad is the
/// player's, not a level's, and a script of either level may ask about it.
fn feed_pad(shell: &mut LevelWorld, level: &mut LevelWorld, press: &mut bool) -> u32 {
    let word = pad_word(press);
    shell.set_pad_buttons(0, word);
    level.set_pad_buttons(0, word);
    word
}

/// Puts the session's Sackboy on the pod computer's trigger of the shell world
/// (`LevelWorld::pod_trigger_spot`), one unit of depth either way, as
/// `level_scripts --enter-pad-zone`. None in a level without such a trigger.
///
/// PLACEHOLDER: he stands there from the first tick and is not drawn. Part 32: only
/// without `--play-sackboy`; with it the viewer's own Sackboy plays in the Pod
/// ([`stand_sackboy`]).
fn place_sackboy(shell: &mut LevelWorld) -> Option<(u32, [f32; 3])> {
    let spot = shell.pod_trigger_spot()?;
    Some((shell.add_player(spot, (spot[2] - 1., spot[2] + 1.)), spot))
}

/// The z span of the viewer's own Sackboy in the Pod for the triggers' layer rule (part
/// 33): an absolute world z range, the same in every lane. FOUND (01.33): the trigger
/// test reads two floats of the creature's current state record as absolute world z (the
/// trigger's z plus its z offset must lie between them); the creature's constructor sets
/// them to -500 and 100 and every creature update writes them again from the same two
/// constants. For a Sackboy they can widen (around the centre of a box of the level, to
/// 500 under a global flag, to 300 from another path); INFERRED that none of these
/// changes them in normal play. So pod.bin's trigger at z -193.7 holds him in all three
/// lanes, and the lane decision is the pad zone's own z rule (part 24) alone. The play
/// mode's span on other levels ([`PLAYER_HALF_DEPTH`], part 10) is left as it is.
const POD_PLAYER_Z_SPAN: (f32, f32) = (-500., 100.);

/// Part 32. Hands the place of the viewer's own Sackboy to the shell world for the tick
/// about to run: a new Sackboy Thing the first time (`LevelWorld::add_player`), then
/// `LevelWorld::move_player`. `at` is his position in the shell level's units (69.9 above
/// his feet; the lane is z). The tick's trigger pass then keeps the pod computer
/// trigger's list by where he stands, and `Native.GetIsInPadZone` answers from that list
/// and his z. Returns his Thing and whether it was made now.
fn stand_sackboy(
    shell: &mut LevelWorld,
    sackboy: &mut Option<Option<u32>>,
    at: [f32; 3],
) -> (u32, bool) {
    let layers = POD_PLAYER_Z_SPAN;
    match sackboy.flatten() {
        Some(thing) => {
            shell.move_player(thing, at, layers);
            (thing, false)
        }
        None => {
            let thing = shell.add_player(at, layers);
            *sackboy = Some(Some(thing));
            (thing, true)
        }
    }
}

/// Asks the host to stop using the pod computer, from the shell world, and makes the
/// other world agree (`lbp_runtime::tick_session` wants that of a caller that changes
/// one world's session state between two ticks). "In use" goes with the host's answer
/// at the start of the next tick, with the shut-down sound counted.
///
/// PLACEHOLDER: no script path for closing was found. The pod computer's own menus,
/// which the player leaves it through, are not modelled.
fn ask_to_close(shell: &mut LevelWorld, level: &mut LevelWorld) {
    shell.request_host_use_pod(false);
    shell.hand_session_to(level);
}

/// The progression mask the Pod's script wants before the pod computer can be engaged
/// (FOUND in the script listing, part 24).
const ENGAGE_FLAGS: u32 = 2;

/// Why pod.bin's script did not ask for the pod computer on a tick with the engage
/// button down: the first of the conditions FOUND for its engage test (part 24) that
/// does not hold now, as far as the host can tell.
fn press_ignored(shell: &LevelWorld, sackboy: Option<u32>) -> String {
    let flags = shell.progression_flags;
    match sackboy {
        None => "no Sackboy stands on a pod computer trigger".into(),
        Some(_) if shell.pod_computer_in_use => "the pod computer is in use".into(),
        Some(_) if flags & ENGAGE_FLAGS == 0 => format!(
            "the progression flags are {flags:#x}, without mask {ENGAGE_FLAGS:#x}, which the planets level's intro sets (MEASURED on tick 452 of a fresh profile): the script's own rule"
        ),
        Some(thing) if !shell.in_pad_zone(thing) => {
            let at = shell.things.get(thing as usize).and_then(|t| t.world_matrix);
            format!(
                "the Sackboy is not in the pad zone{}",
                at.map_or(String::new(), |m| format!(" (he stands at {:.1?})", &m[12..15]))
            )
        }
        Some(_) => {
            "another condition of the script (a video, a photo, the start menu, a button still held)"
                .into()
        }
    }
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
/// FOUND uses of the value are the pod's shape colour alpha, the pod decorations' fade,
/// the pod's mesh at 1 and the mask texture. Part 31: the shell level's place no longer
/// follows it (the shell stays in its own level frame, [`shell_frame_root`]). What still
/// reads the progress is the viewer's own: the Pod fog of the default lighting, the
/// fitted figure's turn and the overlay (PLACEHOLDER).
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
    let press_at = tick_option(&all, "--level-scripts-press-at")?;
    if press_at.is_some() && shell_guid.is_none() {
        println!(
            "--level-scripts-press-at is not used without the Pod's shell layer (--pod-camera N --pod-shell HEX): the engage press is read by the shell level's script"
        );
    }
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
            sackboy: None,
            press: false,
            answers: 0,
            own_sackboy: crate::sackboy_play::in_pod(),
            zone: None,
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
        follow_lens: None,
        override_frames: 0,
        override_printed: (false, false, false),
        hold_at,
        in_use_at,
        press_at,
        script_frames: 0,
        script_state: ScriptCameraState::Out,
        script_last: None,
        depths_printed: None,
        puter_open: None,
        world_root: None,
        world_root_placed: 0,
        shell,
        script_eye_y: None,
        dissolve_printed: false,
        dissolve_edge_printed: false,
        dof_printed: None,
    })
    .insert_resource(crate::CaptureHold(hold_at.is_some()))
    .insert_resource(Time::<Fixed>::from_hz(f64::from(TICK_HZ)))
    .init_resource::<crate::pod_menu::ScriptMenu>()
    .add_systems(
        FixedUpdate,
        (
            tick,
            relight,
            follow_fog,
            follow_things,
            follow_world,
            follow_menu,
        )
            .chain(),
    )
    // After the viewer's own camera has placed the follow camera, before a capture.
    // D319: the scripts' main menu is drawn by that camera.
    .add_systems(
        Update,
        (
            script_camera,
            follow_dof,
            follow_dissolve,
            layer_depths,
            crate::pod_menu::paint_script_menu,
        )
            .chain()
            .after(crate::camera)
            .before(crate::capture),
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
    mut pod: Option<ResMut<crate::sackboy_play::PodSackboy>>,
) {
    let p = &mut *play;
    // Part 26: the Puter key of the viewer's Pod rig is the player at the pod computer.
    // Handed over when it changes, also while the ticks are held.
    if let Some(open) = puter_change(&mut p.puter_open, puter.as_deref().map(|s| s.open)) {
        // Part 30: with the shell level in the session the keys no longer come this
        // way (`puter_key`); what is left is the state the rig starts with, `--pod-open`.
        let from = match p.shell {
            Some(_) => {
                "--pod-open (set directly, a QA option; the keys go through pod.bin's script)"
            }
            None => "the viewer's Puter state (stand-in for the pod's own script)",
        };
        let shell = p.shell.as_mut().map(|s| &mut s.host.world);
        set_in_use(&mut p.host.world, shell, open);
        println!(
            "Pod computer at tick {}: in use {open}, from {from}",
            p.scripts.ticks
        );
    }
    // Part 30: with the shell level in the session a Puter key is not "in use" itself.
    // The opening key is the engage button on the pad, for pod.bin's own script to
    // answer; a key while the computer is in use asks the host to close it.
    if let Some(key) = puter.as_deref().and_then(|s| s.key) {
        if let Some(puter) = puter.as_mut() {
            puter.key = None;
        }
        puter_key(p, key);
    }
    step(p, &mut capture, &sackboy, pod.as_deref_mut());
    // Part 28: the opening progress of the viewer's Pod rig is the change-over value of
    // pod.bin's script after the last tick (see [`opening`]). Also while
    // the ticks are held, for a rig that came up after them. Part 30: and the Puter
    // state "open" is the session's "pod computer in use", however it came about.
    if let (Some(shell), Some(puter)) = (&p.shell, puter.as_mut()) {
        if puter.t != shell.opening {
            puter.t = shell.opening;
        }
        let in_use = shell.host.world.pod_computer_in_use;
        if puter.open != in_use {
            puter.open = in_use;
        }
        p.puter_open = Some(in_use);
    }
    // Part 33: the viewer's own Sackboy at the pod computer. While it is in use (by him,
    // the only and so the primary player) his movement input is not applied, he turns to
    // face into the screen and his figure is drawn toward the engage target. D320: by the
    // pod computer layer's blend (`sackboy_play::PodUserAnim`), stepped once a script
    // tick, which also gives the layer's clip, frame and weight.
    if let (Some(shell), Some(pod)) = (&p.shell, pod.as_deref_mut()) {
        let in_use = shell.own_sackboy && shell.host.world.pod_computer_in_use;
        if pod.in_use != in_use {
            pod.in_use = in_use;
            println!(
                "Pod Sackboy at tick {}: {}",
                shell.scripts.ticks,
                if in_use {
                    "at the pod computer: his movement input is not applied, he turns to face into the screen (angle pi), the pod computer layer starts (into clip, then idle) and his figure is drawn toward the engage target by its blend; his physics place is not moved"
                } else {
                    "the pod computer is no longer in use: his input is applied again, the out clip plays and the drawn offset goes back to 0"
                }
            );
        }
        if pod.stepped != Some(shell.scripts.ticks) {
            pod.stepped = Some(shell.scripts.ticks);
            let frames = pod.frames;
            let user = pod.anim.step(in_use, frames);
            if user.map(|u| u.clip) != pod.user.map(|u| u.clip) {
                println!(
                    "Pod Sackboy at tick {}: pod computer layer {}",
                    shell.scripts.ticks,
                    user.map_or("off".into(), |u| format!(
                        "{} from frame {} (weight {:.2}, offset blend {:.2})",
                        u.clip.name(),
                        u.frame,
                        u.weight,
                        u.offset
                    ))
                );
            }
            pod.user = user;
            let blend = user.map_or(0., |u| u.offset);
            if pod.blend != blend {
                pod.blend = blend;
            }
        }
    }
}

/// A Puter key of the shell layer under the scripts (part 30; [`key_act`]). Without a
/// shell level in the session there is no such key: `pod_dolly` then sets the Puter
/// state itself.
fn puter_key(p: &mut Play, key: PuterKey) {
    let Some(shell) = p.shell.as_mut() else {
        return;
    };
    let ticks = shell.scripts.ticks;
    match key_act(key, shell.host.world.pod_computer_in_use) {
        Some(KeyAct::Press) => {
            shell.press = true;
            println!(
                "Engage key after tick {ticks}: the engage button (bit {ACTION_ENGAGE_POD_BUTTON:#x} of the pad's button word) is down on the next tick, for pod.bin's script to answer"
            );
        }
        Some(KeyAct::Close) => {
            ask_to_close(&mut shell.host.world, &mut p.host.world);
            println!(
                "Pod computer after tick {ticks}: the key asks the host to stop using it (placeholder: no script path for closing was found; the pod computer's menus are not modelled)"
            );
        }
        None => {}
    }
}

/// One fixed step of the scripts: the create step first, then a tick each, unless held.
fn step(
    p: &mut Play,
    capture: &mut ResMut<crate::CaptureHold>,
    sackboy: &Query<(&Position, &lbp_sim::ZSlab), With<SackboyBody>>,
    mut pod: Option<&mut crate::sackboy_play::PodSackboy>,
) {
    // `--level-scripts-hold-at N`: a capture waits for tick N, and no tick follows it.
    let held = p.hold_at.is_some_and(|n| p.scripts.ticks >= n);
    // Part 32: the viewer's own Sackboy in the Pod holds his input and pose with them.
    if let Some(pod) = pod.as_deref_mut() {
        if pod.held != held {
            pod.held = held;
        }
    }
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
    // tick after tick N on (as `level_scripts --puter-in-use=N`). The direct way, kept
    // for QA: the path of part 24 (pad zone, press, the host's answer) is the Puter
    // key's and `--level-scripts-press-at`'s (part 30).
    if in_use_now(&mut p.in_use_at, p.scripts.ticks) {
        let shell = p.shell.as_mut().map(|s| &mut s.host.world);
        set_in_use(&mut p.host.world, shell, true);
        println!(
            "Pod computer at tick {}: in use true, from --level-scripts-in-use-at (stand-in for the player engaging it)",
            p.scripts.ticks
        );
    }
    // Part 30: the session's Sackboy and pad. He is put on the pod computer's trigger
    // of the shell level before its first tick; the pad's button word is written for
    // every tick, with the engage bit on the tick of a press.
    let mut pressed = false;
    if let Some(shell) = p.shell.as_mut() {
        let next = shell.scripts.ticks + 1;
        if let Some(pod) = pod.as_deref_mut() {
            // Part 32: the viewer's own Sackboy plays in the shell level. His place after
            // the physics tick before goes to the shell world, so this tick's trigger
            // pass and the pad zone test are decided by where he stands. Until he is
            // spawned there is no Sackboy in the session.
            if let Some((at, grounded)) = pod.at {
                let (thing, new) = stand_sackboy(&mut shell.host.world, &mut shell.sackboy, at);
                if new {
                    println!(
                        "Pod Sackboy before tick {next}: Thing {thing} of the shell level, the viewer's own Sackboy at {at:.1?} ({}); his place is handed to the shell level before every tick, z span {:.0} to {:.0} (absolute world z, FOUND constants of the 01.33 creature update) for the triggers' layer rule; the pad zone wants him in the pod computer trigger's list and z at least {PAD_ZONE_MIN_Z}",
                        if grounded { "on the floor" } else { "in the air" },
                        POD_PLAYER_Z_SPAN.0,
                        POD_PLAYER_Z_SPAN.1
                    );
                }
            }
            if std::mem::take(&mut pod.engage) {
                shell.press = true;
                println!("Engage press on tick {next}, from --play-sackboy-input (e)");
            }
        } else if shell.sackboy.is_none() {
            let placed = place_sackboy(&mut shell.host.world);
            shell.sackboy = Some(placed.map(|(thing, _)| thing));
            match placed {
                Some((thing, spot)) => println!(
                    "Pod Sackboy before tick {next}: Thing {thing} of the shell level, on the pod computer's trigger at {spot:.1?} (z at least {PAD_ZONE_MIN_Z} for the pad zone: {}); not drawn, a placeholder for the player walking there",
                    spot[2] >= PAD_ZONE_MIN_Z
                ),
                None => println!(
                    "Pod Sackboy: the shell level has no pod computer trigger; nobody can engage the pod computer"
                ),
            }
        }
        if press_now(&mut p.press_at, next) {
            shell.press = true;
            println!("Engage press on tick {next}, from --level-scripts-press-at");
        }
        let word = feed_pad(&mut shell.host.world, &mut p.host.world, &mut shell.press);
        pressed = word != 0;
    }
    // The play-mode Sackboy (`--play-sackboy`), when there is one, is the player the
    // level's triggers test. Part 32: not in the Pod, where he plays in the shell level.
    if let (None, Ok((pos, slab))) = (&pod, sackboy.single()) {
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
            // Part 30: what the script made of an engage press, and the host's answer.
            let world = &shell.host.world;
            let ticks = shell.scripts.ticks;
            if pressed {
                match world.pod.host_use_pod_pending {
                    Some(true) => println!(
                        "Engage press on tick {ticks}: pod.bin's script took the click and asked the host for the pod computer (RequestHostUsePod(true)); the answer comes with the next tick"
                    ),
                    _ => println!(
                        "Engage press on tick {ticks}: ignored, pod.bin's script made no request: {}",
                        press_ignored(world, shell.sackboy.flatten())
                    ),
                }
            }
            if world.pod.in_use_changes.len() != shell.answers {
                shell.answers = world.pod.in_use_changes.len();
                let count = |sound: &str| world.sounds_played.get(sound).copied().unwrap_or(0);
                println!(
                    "Pod computer at tick {ticks}: in use {}, by the host's answer to a request; sounds counted in the shell level: start-up {}, shut-down {} (none is played)",
                    world.pod_computer_in_use,
                    count(lbp_runtime::POD_COMPUTER_STARTUP_SOUND),
                    count(lbp_runtime::POD_COMPUTER_SHUTDOWN_SOUND)
                );
            }
            // Part 32: the pad zone by where the viewer's own Sackboy stands, when it
            // changes.
            if let (true, Some(thing)) = (shell.own_sackboy, shell.sackboy.flatten()) {
                let now = world.in_pad_zone(thing);
                if shell.zone.replace(now) != Some(now) {
                    let at = world
                        .things
                        .get(thing as usize)
                        .and_then(|t| t.world_matrix);
                    println!(
                        "Pod Sackboy at tick {ticks}: {} the pad zone, standing at {:.1?} (Native.GetIsInPadZone is {now} for him)",
                        if now { "in" } else { "out of" },
                        at.map_or([f32::NAN; 3], |m| [m[12], m[13], m[14]])
                    );
                }
            }
            let was = std::mem::replace(
                &mut shell.opening,
                opening(shell.host.world.pod_transition()),
            );
            if let Some(what) = opening_event(was, shell.opening) {
                println!(
                    "Pod change-over at tick {}: {what}, value {:.2} from pod.bin's script; the tent's dissolve and the pod's mesh follow it, the shell level stays in its own frame",
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
        "Fog bias at tick {}: {bias:.1} level units from the scripts (Cam.SetDOFAndFog), added to the start and the end of the preview fog; the first two values (depth of field) apply under --depth-of-field",
        play.scripts.ticks
    );
}

/// D298 (part 49): the depth-of-field range a script handed the camera: the first two
/// values of the last `Cam.SetDOFAndFog` call (level units along the view, FOUND: the
/// game projects both distances along the camera's axis), `None` before any call and
/// unless 0 < start < end, both numbers.
fn dof_range(world: &LevelWorld) -> Option<[f32; 2]> {
    let [start, end, _] = world.camera.dof_and_fog?;
    (start.is_finite() && end.is_finite() && start > 0. && end > start).then_some([start, end])
}

/// D298: puts the scripts' depth-of-field range on the main camera in display units every
/// frame (the passes run only under `--depth-of-field`, `dof_post.rs`). One line when the
/// range changes, in level units.
fn follow_dof(
    mut play: NonSendMut<Play>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    mut cameras: Query<&mut crate::dof_post::DepthOfField>,
) {
    let range = dof_range(&play.host.world);
    let scale = display.map_or(1., |d| d.0.x_axis.x);
    let shown = range
        .map(|[a, b]| [a * scale, b * scale])
        .filter(|[a, b]| a.is_finite() && b.is_finite() && *a > 0.);
    for mut camera in &mut cameras {
        if camera.range != shown {
            camera.range = shown;
        }
    }
    if crate::dof_post::enabled() && play.dof_printed != range {
        play.dof_printed = range;
        match range {
            Some([a, b]) => println!(
                "Depth of field at tick {}: blur from {a:.1} to {b:.1} level units along the view (Cam.SetDOFAndFog), whole beyond {b:.1}",
                play.scripts.ticks
            ),
            None => println!(
                "Depth of field at tick {}: no range from the scripts, no blur",
                play.scripts.ticks
            ),
        }
    }
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

/// D319 (part 70): the main menu the pod computer's scripts built and drew, handed to the
/// viewer after each tick (`pod_menu::paint_script_menu` draws it); none before.
fn follow_menu(play: NonSend<Play>, mut menu: ResMut<crate::pod_menu::ScriptMenu>) {
    let now = crate::pod_menu::ScriptMenuFrame::of(&play.host.world);
    if menu.0 != now {
        if menu.0.is_none() && now.is_some() {
            println!(
                "Main menu at tick {}: built by the pod computer's scripts and drawn by their matrix ({} items)",
                play.scripts.ticks,
                now.as_ref().map_or(0, |m| m.placed.len())
            );
        }
        menu.0 = now;
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

/// The part of a perspective lens a script camera sets (part 34): the vertical field of
/// view in radians, the near and far plane in display units. The aspect is not in it:
/// it stays the viewer's (the window's, 16:9 under `--window-16-9`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Lens {
    fov: f32,
    near: f32,
    far: f32,
}

impl Lens {
    fn of(lens: &PerspectiveProjection) -> Self {
        Self {
            fov: lens.fov,
            near: lens.near,
            far: lens.far,
        }
    }

    /// Writes the three into `lens`; its aspect is left as it is.
    fn apply(self, lens: &mut PerspectiveProjection) {
        lens.fov = self.fov;
        lens.near = self.near;
        lens.far = self.far;
    }
}

/// The lens the viewer draws a script's camera with (part 34), `scale` being the
/// level's display scale (display units per level unit) and `own` the viewer's lens.
///
/// FOUND (01.33): the render camera holds an ortho factor, a vertical field of view in
/// radians, an aspect, a near and a far plane, and is a perspective camera when the ortho
/// factor is 0. `Cam.SetCameraFromToUp` writes near, far, ortho factor and field of view
/// from its four floats on every call (the Pod passes 10, 100,000, 0 and 1.0) and does
/// not write the aspect. The override camera has no lens of its own: it is the game
/// camera's (near 10, far 40,000, 60 degrees; the runtime's `DisplayCamera` carries
/// them). INFERRED: the game's world units are level units, so the planes are scaled
/// by `scale` like every other level length.
///
/// The field of view is the camera's when it has a usable one, else `own`'s. The planes
/// are the camera's when both are above 0 (FOUND: the game rebuilds the projection only
/// then) and far is beyond near, else `own`'s. With an ortho factor other than 0 the
/// viewer keeps `own` whole (PLACEHOLDER: the game's orthographic branch is not drawn;
/// no script seen passes one). In Bevy the far plane only culls whole draw entities (its
/// projection has no far plane), so a mesh that crosses it is still drawn whole.
fn script_lens(camera: &DisplayCamera, ortho: f32, scale: f32, own: Lens) -> Lens {
    if ortho != 0. {
        return own;
    }
    let fov = camera.fov_y_deg.map_or(own.fov, f32::to_radians);
    let (near, far) = (camera.near * scale, camera.far * scale);
    let planes = near > 0. && far > near && far.is_finite();
    Lens {
        fov,
        near: if planes { near } else { own.near },
        far: if planes { far } else { own.far },
    }
}

/// The lens to draw a frame with: under a script's camera (`shown`, with its ortho
/// factor) [`script_lens`], the viewer's lens of before it being kept in `follow`; with
/// no script camera the kept lens once (to put back), then `None`: the viewer's lens
/// stands untouched. `own` is the lens the projection has now.
fn wanted_lens(
    shown: Option<(&DisplayCamera, f32)>,
    follow: &mut Option<Lens>,
    own: Option<Lens>,
    scale: f32,
) -> Option<Lens> {
    match shown {
        Some((camera, ortho)) => {
            if follow.is_none() {
                *follow = own;
            }
            follow.map(|kept| script_lens(camera, ortho, scale, kept))
        }
        None => follow.take(),
    }
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
/// With the Pod's shell layer (`--pod-shell` with `--pod-camera`) nothing is placed
/// here (part 31): the shell and tent roots stand in the shell level's own frame from
/// the start ([`shell_frame_root`]), and this camera, the one the planets level's
/// script wrote, draws both levels. MEASURED at the two end states: the game hands the
/// pod's draws that camera's eye. The shell's own script world gets no camera view
/// here.
///
/// INFERRED: world +Y is up under the override. The lens (part 34, [`script_lens`]): the
/// camera a script wrote draws with the field of view, near and far plane the script
/// passed, the override with the game camera's (60 degrees, 10, 40,000), the planes
/// taken from level units to display units; the viewer's own lens comes back with its
/// own camera. One line is printed when a camera comes in, is fully in or comes to
/// rest, or goes out: the check without a screenshot.
fn script_camera(
    mut play: NonSendMut<Play>,
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
    let shown = world.display_camera(follow_eye.to_array(), follow_dir.to_array());
    // The third float of the call that wrote the camera (FOUND: the ortho factor).
    let ortho = world.script_camera().map_or(0., |c| c.lens_value);
    let scale = display.0.x_axis.x;
    let own = match projection.as_ref() {
        Projection::Perspective(lens) => Some(Lens::of(lens)),
        _ => None,
    };
    let wanted = match &shown {
        Some(camera) => {
            *transform = place_camera(display.0, camera);
            if camera.source == CameraSource::Script {
                p.script_eye_y = Some(camera.pose.eye[1]);
            }
            match camera.source {
                CameraSource::Script => p.script_frames += 1,
                CameraSource::Override => p.override_frames += 1,
            }
            let ortho = if camera.source == CameraSource::Script {
                ortho
            } else {
                0.
            };
            wanted_lens(Some((camera, ortho)), &mut p.follow_lens, own, scale)
        }
        None => wanted_lens(None, &mut p.follow_lens, own, scale),
    };
    if let (Some(lens), Some(own)) = (wanted, own) {
        if lens != own {
            if let Projection::Perspective(perspective) = projection.as_mut() {
                lens.apply(perspective);
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
        let drawn = match projection.as_ref() {
            Projection::Perspective(lens) => format!(
                "drawn with vertical field of view {:.2} deg, near {:.5} and far {:.3} display units ({:.1} and {:.0} level units, scale {scale:.4e}), aspect {:.4}",
                lens.fov.to_degrees(),
                lens.near,
                lens.far,
                lens.near / scale,
                lens.far / scale,
                lens.aspect_ratio
            ),
            _ => "not a perspective lens".into(),
        };
        match script {
            Some(c) => println!(
                "Script camera at tick {ticks}: {what}; eye {:.1?}, direction {:.3?}, up {:.2?}, field of view {} (level units); passed near {:.0}, far {:.0}, ortho factor {ortho}; {drawn}; frames drawn with it {}",
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

/// Part 31. The root of the Pod's shell level (pod.bin) under `--level-scripts`: the
/// shell is drawn in its own level frame, under the camera the planets level's script
/// writes. `planets` and `shell` take each level's units to its display units (a uniform
/// scale and a translation each, `level_home`); the camera stands at `planets * eye`, so
/// a shell entity at `shell * X` under the root `planets * shell^-1` stands at
/// `planets * X`: level point `X` of the shell and level point `X` of the camera's frame
/// are one point. No turn, and nothing follows the opening progress.
///
/// MEASURED (two captured frames, part 29): the camera position the game hands the pod's
/// draws is the planets script camera's eye, (0, -250, 1130) closed and (0, -299.5,
/// 902.4) with the pod computer open, and the tent's heights over it are its level
/// heights plus 250. INFERRED: between the two states too. `None` when the shell's
/// matrix has no inverse.
pub(crate) fn shell_frame_root(planets: Mat4, shell: Mat4) -> Option<Transform> {
    let frame = planets * shell.inverse();
    frame.is_finite().then(|| Transform {
        translation: frame.w_axis.truncate(),
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(frame.x_axis.x),
    })
}

/// The draw entities of the shell layer, for [`follow_dissolve`]: the tag, the material
/// and the root the entity hangs under.
type Dissolved<'w, 's> = Query<
    'w,
    's,
    (
        &'static crate::thing_view::LevelThing,
        &'static MeshMaterial3d<crate::ViewerMaterial>,
        Option<&'static ChildOf>,
    ),
>;

/// Writes the line of the tent's dissolve into the materials of the draw entities that
/// carry a dissolve value (part 29; `thing_view::apply` keeps the value on the tag),
/// each frame after [`script_camera`] has placed the camera.
///
/// The line is `camera y + 1150 - 1550 v` in the shell level's own y
/// (`pod_dissolve::line_y`; FOUND), the camera being the one the planets level's script
/// wrote (MEASURED at the two end states; INFERRED that the two levels share one frame
/// between them). It goes to the shader as a plane in Bevy world space: the
/// level-to-world matrix is the `Transform` of the root the entity hangs under times
/// the shell's display matrix. Part 31: that root is the shell's level frame
/// ([`shell_frame_root`]), so the product is the planets level's display matrix and the
/// plane is plainly "level y minus the line", the line being the camera's y plus a
/// constant: one frame for the tent, the camera and the line. A material is written
/// only when its values change, so a held tick costs nothing. Nothing while no script camera has been shown: the tent is
/// then drawn whole. One line is printed the first time, with the mask's state.
fn follow_dissolve(
    mut play: NonSendMut<Play>,
    things: Option<Res<crate::thing_view::ShellThings>>,
    mask: Option<Res<crate::PodDissolveMask>>,
    tagged: Dissolved,
    roots: Query<&Transform>,
    mut materials: ResMut<Assets<crate::ViewerMaterial>>,
) {
    let (Some(things), Some(camera_y)) = (things, play.script_eye_y) else {
        return;
    };
    let image = mask.as_ref().and_then(|m| m.image.clone());
    let island = mask.as_ref().and_then(|m| m.island.clone());
    let (mut written, mut value) = (0, None);
    // D284: entities written per input that holds raw UV0 (none, `in.uv`, `in.uv_b`), and
    // those given `pod_mask`.
    let (mut sources, mut islands) = ([0; 3], 0);
    for (tag, material, parent) in &tagged {
        let Some(dissolve) = tag.dissolve else {
            continue;
        };
        let root = parent
            .and_then(|p| roots.get(p.parent()).ok())
            .map_or(Mat4::IDENTITY, Transform::to_matrix);
        // D284: `pod_mask` on the two card materials only (the glass has no slot 6).
        let card = tag.material.is_some_and(|g| {
            [crate::pod_dissolve::CARD, crate::pod_dissolve::CARD_EDGE].contains(&g)
        });
        let Some(wanted) = crate::pod_dissolve::uniforms(
            dissolve.value,
            dissolve.colour,
            camera_y,
            root * things.0.normalize,
            image.clone(),
            island.clone().filter(|_| card),
        ) else {
            continue;
        };
        let id = material.0.id();
        let stale = |m: &crate::ViewerMaterial| crate::dual::dissolve(m).as_ref() != Some(&wanted);
        if !materials.get(id).is_some_and(stale) {
            continue;
        }
        if let Some(mut m) = materials.get_mut(id) {
            crate::dual::set_dissolve(&mut m, Some(&wanted));
            written += 1;
            sources[usize::from(crate::dual::raw_uv0(&m).min(2))] += 1;
            islands += usize::from(wanted.island.is_some());
            value = Some((dissolve.value, dissolve.colour));
        }
    }
    let Some((v, colour)) = value else {
        return;
    };
    let uv = format!(
        "masks at raw UV0 from in.uv {}, in.uv_b {}, none (position) {}; pod_mask on {islands} ({})",
        sources[1],
        sources[2],
        sources[0],
        mask.as_ref().map_or("not asked for", |m| m.island_note.as_str()),
    );
    if v >= 1. {
        // D284: the platform's card edge at value 1, once.
        if !play.dissolve_edge_printed {
            play.dissolve_edge_printed = true;
            println!(
                "Pod platform edge at tick {}: value 1, {written} draw entities; cut above level y {:.1} where pod_mask is black, {} bands higher where it is white (camera y {camera_y:.1}); {uv}",
                play.scripts.ticks,
                crate::pod_dissolve::line_y(camera_y, v) - crate::pod_dissolve::BAND,
                2. * crate::pod_dissolve::ISLAND_RISE,
            );
        }
        return;
    }
    if play.dissolve_printed {
        return;
    }
    play.dissolve_printed = true;
    let preloaded = crate::pod_dissolve::TRANSITION_MASK;
    let named = play
        .shell
        .as_ref()
        .and_then(|s| s.host.world.pod.transition_texture);
    let same = match named {
        Some(g) if g == preloaded => "the same".to_string(),
        Some(g) => format!("another, {g} (the six other backdrop masks are not loaded)"),
        None => "none yet".to_string(),
    };
    println!(
        "Pod tent dissolve at tick {}: value {v:.2}, {written} draw entities; cut above level y {:.1} (camera y {camera_y:.1} + {} - {} x value), flat colour {colour:.3?} (shown as display bytes {:?}); mask {preloaded} {}; the script's mask GUID: {same}; {uv}",
        play.scripts.ticks,
        crate::pod_dissolve::line_y(camera_y, v),
        crate::pod_dissolve::LINE_AT_ZERO,
        crate::pod_dissolve::LINE_FALL,
        crate::pod_dissolve::shown_bytes(colour),
        mask.as_ref().map_or("not asked for", |m| m.note.as_str()),
    );
}

/// The roots of the two Pod levels, for [`layer_depths`]: the root's place and whether it
/// is a root of the shell level.
type LayerRoots<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, Has<crate::PodShellRoot>),
    Or<(With<crate::PodShellRoot>, With<crate::PodPlanetsRoot>)>,
>;

/// The draw entities under those roots: place under the root, mesh, bounds, whether
/// shown.
type LayerParts<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static Mesh3d,
        &'static bevy::camera::primitives::Aabb,
        &'static InheritedVisibility,
        &'static ChildOf,
    ),
>;

/// A box by its two corners, and how far a point is outside it (0 inside).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Box3 {
    lo: Vec3,
    hi: Vec3,
}

impl Box3 {
    const NONE: Self = Self {
        lo: Vec3::INFINITY,
        hi: Vec3::NEG_INFINITY,
    };

    fn add(&mut self, p: Vec3) {
        self.lo = self.lo.min(p);
        self.hi = self.hi.max(p);
    }

    fn distance(&self, p: Vec3) -> f32 {
        (self.lo - p).max(p - self.hi).max(Vec3::ZERO).length()
    }
}

/// The eight corners of the box `centre` +- `half` under `matrix`.
fn corners(matrix: Mat4, centre: Vec3, half: Vec3) -> [Vec3; 8] {
    std::array::from_fn(|corner| {
        let sign = Vec3::new(
            if corner & 1 == 0 { -1. } else { 1. },
            if corner & 2 == 0 { -1. } else { 1. },
            if corner & 4 == 0 { -1. } else { 1. },
        );
        matrix.transform_point3(centre + sign * half)
    })
}

/// The depth range of one level in front of a camera: the smallest and the largest
/// distance along the view of the corners of its entities' bounds (a box's nearest and
/// farthest point along a direction is a corner), and the entities counted.
#[derive(Clone, Copy, Debug, PartialEq)]
struct DepthRange {
    near: f32,
    far: f32,
    entities: usize,
}

impl DepthRange {
    const NONE: Self = Self {
        near: f32::INFINITY,
        far: f32::NEG_INFINITY,
        entities: 0,
    };

    /// Takes in a box `centre` +- `half` under `to_view` (the box's frame to the
    /// camera's, which looks along -Z).
    fn add(&mut self, to_view: Mat4, centre: Vec3, half: Vec3) {
        for corner in corners(to_view, centre, half) {
            self.near = self.near.min(-corner.z);
            self.far = self.far.max(-corner.z);
        }
        self.entities += 1;
    }
}

/// Points per triangle edge in [`nearest_to_box`]: each triangle is looked at on a grid
/// of this many steps a side.
const TRIANGLE_STEPS: u16 = 8;

/// The smallest distance from `target` to the surface of a triangle mesh under
/// `to_world`, looked at on a grid of points over each triangle (its corners, edges and
/// inside; [`TRIANGLE_STEPS`]), and the triangles looked at. 0 when a point is inside
/// the box. A sampled value: a triangle much larger than the box can pass it between
/// two points. `None` for a mesh without a whole triangle.
fn nearest_to_box(
    target: &Box3,
    to_world: Mat4,
    positions: &[[f32; 3]],
    mut indices: impl Iterator<Item = usize>,
) -> Option<(f32, usize)> {
    let at = |i: usize| Some(to_world.transform_point3(Vec3::from(*positions.get(i)?)));
    let (mut best, mut triangles) = (f32::INFINITY, 0);
    while let (Some(a), Some(b), Some(c)) = (indices.next(), indices.next(), indices.next()) {
        let (Some(a), Some(b), Some(c)) = (at(a), at(b), at(c)) else {
            continue;
        };
        triangles += 1;
        let n = TRIANGLE_STEPS;
        for i in 0..=n {
            for j in 0..=(n - i) {
                let (u, v) = (f32::from(i) / f32::from(n), f32::from(j) / f32::from(n));
                best = best.min(target.distance(a + (b - a) * u + (c - a) * v));
            }
        }
    }
    (triangles > 0).then_some((best, triangles))
}

/// Part 31, the check without a screenshot of how the two Pod levels lie under the one
/// camera: one line when the script camera has come to rest, or the ticks are held for
/// a capture. It gives the depth range of the shown draw entities of the shell level and
/// of the planets level (level units along the view, from the corners of their bounds),
/// the camera's near and far planes, and how near the planets level's surfaces come to
/// the box around the shell ([`nearest_to_box`]): the bounds alone do not say, since
/// the planets level's room is a box around everything. The shell stands in its own
/// frame around the camera; the planets level hangs under its moved world Thing.
/// Entities are placed by their own `Transform` under their root's (the roots have no
/// parent), so a root moved on this tick counts at once.
fn layer_depths(
    mut play: NonSendMut<Play>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    cameras: Query<(&Transform, &Projection), With<Camera3d>>,
    roots: LayerRoots,
    parts: LayerParts,
    meshes: Res<Assets<Mesh>>,
) {
    let p = &mut *play;
    let (Some(_), Some((ticks, pose))) = (&p.shell, p.script_last) else {
        return;
    };
    let held = p.hold_at.is_some_and(|n| ticks >= n);
    if !(held || p.script_state == ScriptCameraState::AtRest)
        || p.depths_printed == Some((pose, held))
    {
        return;
    }
    let (Some(display), Ok((camera, projection))) = (display, cameras.single()) else {
        return;
    };
    let scale = display.0.x_axis.x;
    let to_view = camera.to_matrix().inverse();
    if !(to_view.is_finite() && scale.is_finite() && scale > 0.) {
        return;
    }
    let placed = parts
        .iter()
        .filter_map(|(local, mesh, bounds, shown, parent)| {
            let (root, of_shell) = roots.get(parent.parent()).ok()?;
            let to_world = root.to_matrix() * local.to_matrix();
            shown.get().then_some((to_world, mesh, bounds, of_shell))
        });
    let (mut shell, mut planets) = (DepthRange::NONE, DepthRange::NONE);
    let mut shell_box = Box3::NONE;
    for (to_world, _, bounds, of_shell) in placed.clone() {
        let (centre, half) = (bounds.center.into(), bounds.half_extents.into());
        let range = if of_shell { &mut shell } else { &mut planets };
        range.add(to_view * to_world, centre, half);
        if of_shell {
            for corner in corners(to_world, centre, half) {
                shell_box.add(corner);
            }
        }
    }
    if shell.entities == 0 || planets.entities == 0 {
        return;
    }
    // How near the planets level's surfaces come to the box around the shell.
    let (mut nearest, mut triangles, mut unread) = (f32::INFINITY, 0, 0);
    for (to_world, mesh, ..) in placed.filter(|part| !part.3) {
        let mesh = meshes.get(&mesh.0);
        let positions = mesh
            .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
            .and_then(|a| a.as_float3());
        let found = match (positions, mesh.and_then(Mesh::indices)) {
            (Some(positions), Some(indices)) => {
                nearest_to_box(&shell_box, to_world, positions, indices.iter())
            }
            (Some(positions), None) => {
                nearest_to_box(&shell_box, to_world, positions, 0..positions.len())
            }
            _ => None,
        };
        match found {
            Some((distance, n)) => {
                nearest = nearest.min(distance);
                triangles += n;
            }
            None => unread += 1,
        }
    }
    p.depths_printed = Some((pose, held));
    let (near, far) = match projection {
        Projection::Perspective(lens) => (lens.near / scale, lens.far / scale),
        _ => (f32::NAN, f32::NAN),
    };
    let level = |b: Vec3| (display.0.inverse().transform_point3(b)).to_array();
    println!(
        "Pod levels under one camera at tick {ticks} ({}; level units): along the view, by the bounds of the shown draw entities, the shell level reaches from {:.0} to {:.0} ({} entities) and the planets level from {:.0} to {:.0} ({} entities; its room is around both); the box around the shell is {:.0?} to {:.0?}, and the nearest point of the planets level's surfaces to it is {:.0} away ({triangles} triangles looked at on a grid, {unread} entities without readable triangles); camera near plane {near:.0}, far plane {far:.0}",
        if held { "ticks held" } else { "camera at rest" },
        shell.near / scale,
        shell.far / scale,
        shell.entities,
        planets.near / scale,
        planets.far / scale,
        planets.entities,
        level(shell_box.lo),
        level(shell_box.hi),
        nearest / scale,
    );
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
                let pod = &shell.host.world.pod;
                let sackboy = shell.sackboy.flatten();
                ui.small(format!(
                    "  Sackboy, {}: {}; in the pad zone {}; engage prompts asked for {}, clicks {}, requests {}",
                    if shell.own_sackboy {
                        "the viewer's own, where he stands"
                    } else {
                        "on the pod computer's trigger (not drawn)"
                    },
                    sackboy.map_or("none".into(), |t| format!("Thing {t}")),
                    sackboy.is_some_and(|t| shell.host.world.in_pad_zone(t)),
                    pod.action_prompts.values().sum::<u64>(),
                    pod.action_clicks,
                    pod.host_use_pod_requests.len()
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

    /// Two levels with display matrices of their own (`level_home`: a scale after a
    /// shift), the planets level's and the shell's.
    fn two_displays() -> (Mat4, Mat4) {
        (
            Mat4::from_scale(Vec3::splat(1. / 12_000.))
                * Mat4::from_translation(Vec3::new(300., -900., 2500.)),
            Mat4::from_scale(Vec3::splat(1. / 4_959.))
                * Mat4::from_translation(Vec3::new(28., -958., -169.)),
        )
    }

    /// D280. The shell level's root under the scripts is its own level frame: a shell
    /// level point `X` stands where the planets level's display matrix puts level point
    /// `X`, so the script camera's eye, given in level units, is a point of the shell's
    /// frame too. No turn, one scale; a matrix without an inverse gives no root.
    #[test]
    fn d280_the_shell_root_is_the_shells_own_level_frame() {
        let (planets, shell) = two_displays();
        let root = shell_frame_root(planets, shell).expect("both matrices have an inverse");
        assert_eq!(root.rotation, Quat::IDENTITY);
        assert!((root.scale.x - 4_959. / 12_000.).abs() < 1e-6);
        assert_eq!(root.scale, Vec3::splat(root.scale.x));
        let near = |a: Vec3, b: Vec3| (a - b).length() < 1e-5;
        // The game's two camera eyes, the tent's top, the pod computer's trigger.
        for x in [
            Vec3::new(0., -250., 1130.),
            Vec3::new(0., -300., 900.),
            Vec3::new(0., 1278., 0.),
            Vec3::new(15.8, -532.2, -193.7),
            Vec3::ZERO,
        ] {
            let drawn = root.transform_point(shell.transform_point3(x));
            assert!(near(drawn, planets.transform_point3(x)), "{x:?} {drawn:?}");
        }
        // From a camera placed at the closed eye the tent's top is 1528 up and 1130 in
        // front, in level units: its level offset, as the game's draws have it.
        let eye = planets.transform_point3(Vec3::new(0., -250., 1130.));
        let top = root.transform_point(shell.transform_point3(Vec3::new(0., 1278., 0.)));
        let offset = (top - eye) / planets.x_axis.x;
        assert!(
            (offset - Vec3::new(0., 1528., -1130.)).length() < 0.05,
            "{offset:?}"
        );
        // The same display matrix for both levels: the root is the identity.
        let same = shell_frame_root(shell, shell).unwrap();
        assert!(same.translation.length() < 1e-6 && (same.scale.x - 1.).abs() < 1e-6);
        assert_eq!(shell_frame_root(planets, Mat4::ZERO), None);
    }

    /// D280. With the tent under that root the dissolve's plane is "level y minus the
    /// line": the level-to-world matrix `follow_dissolve` hands on (the root times the
    /// shell's display matrix) is the planets level's display matrix, and the height a
    /// tent point has over the line is its level y minus camera y minus the constant,
    /// whatever the camera's y and the change-over value.
    #[test]
    fn d280_the_dissolve_line_is_level_y_minus_camera_y_in_the_shared_frame() {
        let (planets, shell) = two_displays();
        let root = shell_frame_root(planets, shell).unwrap();
        let to_world = root.to_matrix() * shell;
        assert!(to_world.abs_diff_eq(planets, 1e-6));
        for (camera_y, v) in [(-250., 0.05), (-275., 0.5), (-299.5, 0.95)] {
            let line = crate::pod_dissolve::line_y(camera_y, v);
            let rule =
                camera_y + crate::pod_dissolve::LINE_AT_ZERO - crate::pod_dissolve::LINE_FALL * v;
            assert!((line - rule).abs() < 1e-3);
            let plane = crate::pod_dissolve::world_plane(Vec4::new(0., 1., 0., -line), to_world)
                .expect("the matrix has an inverse");
            for x in [
                Vec3::new(0., 1278., 0.),
                Vec3::new(-640., -560., 310.),
                Vec3::new(15.8, -1516., -193.7),
            ] {
                // Where the entity is drawn is where the camera's frame has the point.
                let drawn = root.transform_point(shell.transform_point3(x));
                let over = plane.truncate().dot(drawn) + plane.w;
                assert!((over - (x.y - line)).abs() < 0.05, "{x:?} {over} {line}");
            }
        }
    }

    /// D280. The depth range of a level in front of the camera is taken from the corners
    /// of its entities' bounds: a box 100 to 300 in front of a camera that looks down
    /// -Z, a second box that hangs turned under a root far behind, and a box behind the
    /// camera.
    #[test]
    fn d280_the_depth_range_is_from_the_corners_of_the_bounds() {
        let camera = Transform::from_xyz(0., -250., 1130.);
        let to_view = camera.to_matrix().inverse();
        let mut range = DepthRange::NONE;
        range.add(to_view, Vec3::new(0., 0., 930.), Vec3::new(50., 20., 100.));
        assert_eq!((range.near, range.far, range.entities), (100., 300., 1));
        // Half sizes (10, 10, 40), turned a quarter about Y under a root 31,000 behind
        // the level's origin: 10 either way along the view.
        let root = Transform::from_xyz(0., 0., -31_000.)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        range.add(
            to_view * root.to_matrix(),
            Vec3::ZERO,
            Vec3::new(10., 10., 40.),
        );
        assert_eq!((range.near, range.entities), (100., 2));
        assert!((range.far - 32_140.).abs() < 0.5, "{}", range.far);
        let mut behind = DepthRange::NONE;
        behind.add(to_view, Vec3::new(0., 0., 1184.), Vec3::splat(4.));
        assert!(behind.far < 0.);
    }

    /// D280. How near a mesh comes to a box: a wall 500 behind the box is 500 away
    /// although its corners are far out to the sides; a triangle through the box is at
    /// 0; a mesh under a matrix is measured where the matrix puts it; no whole triangle,
    /// or an index out of range, gives nothing.
    #[test]
    fn d280_the_nearest_surface_to_the_shell_box_is_measured_on_the_triangles() {
        let mut target = Box3::NONE;
        for p in [Vec3::new(-100., -100., -100.), Vec3::new(100., 100., 100.)] {
            target.add(p);
        }
        assert_eq!(target.distance(Vec3::new(50., -20., 99.)), 0.);
        assert_eq!(target.distance(Vec3::new(100., 400., 500.)), 500.);
        // A wall across z = -600, its corners 4000 out: the corners are far, the wall
        // is not.
        let wall = [
            [-4000., -4000., -600.],
            [4000., -4000., -600.],
            [0., 4000., -600.],
        ];
        assert!(target.distance(Vec3::from(wall[0])) > 5000.);
        let (d, n) = nearest_to_box(&target, Mat4::IDENTITY, &wall, 0..3).unwrap();
        assert_eq!((d, n), (500., 1));
        // The same wall twice the size and 31,000 further.
        let moved =
            Mat4::from_translation(Vec3::new(0., 0., -31_000.)) * Mat4::from_scale(Vec3::splat(2.));
        let (d, _) = nearest_to_box(&target, moved, &wall, 0..3).unwrap();
        assert_eq!(d, 32_100.);
        // Through the box, twice.
        let through = [[-500., 0., 0.], [500., 0., 10.], [0., 0., 900.]];
        let twice = [0, 1, 2, 0, 1, 2].into_iter();
        let (d, n) = nearest_to_box(&target, Mat4::IDENTITY, &through, twice).unwrap();
        assert_eq!((d, n), (0., 2));
        assert_eq!(nearest_to_box(&target, Mat4::IDENTITY, &wall, 0..2), None);
        assert_eq!(
            nearest_to_box(&target, Mat4::IDENTITY, &wall, [0, 1, 7].into_iter()),
            None
        );
    }

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

    /// One level of a session without scripts: a VM, a world and its schedule.
    struct Side(Vm, Bindings<LevelWorld>, LevelScripts);

    impl Side {
        fn of(world: LevelWorld) -> Self {
            let (vm, host) = (Vm::default(), Bindings::defaulting(world));
            let scripts = LevelScripts::new(&vm, &host.world);
            Side(vm, host, scripts)
        }
    }

    /// A session of a shell with a pod computer trigger and a level without Things.
    fn session() -> (Side, Side) {
        (
            Side::of(pod_world()),
            Side::of(LevelWorld::from_things(&[])),
        )
    }

    /// One session tick in the viewer's order: the shell, then the level.
    fn round(shell: &mut Side, level: &mut Side) {
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
    }

    /// A shell world for the tests: no Thing of a level file, one pod computer trigger
    /// at a made-up place (a circle of radius 50), started as our Pod.
    fn pod_world() -> LevelWorld {
        let mut world = LevelWorld::from_things(&[]);
        let mut at = Mat4::IDENTITY.to_cols_array();
        at[12..15].copy_from_slice(&[40., -470., -20.]);
        world.things.push(lbp_runtime::ThingState {
            trigger: Some(lbp_runtime::TriggerState {
                radius: 50.,
                hysteresis: 1.5,
                enabled: true,
                z_offset: 5.,
                ..Default::default()
            }),
            world_matrix: Some(at),
            script_classes: vec![lbp_runtime::POD_TRIGGER_CLASS.into()],
            ..Default::default()
        });
        world.our_pod = true;
        world
    }

    /// The game's "just clicked" for the engage action of player 0, as a script asks.
    fn clicked(world: &mut LevelWorld) -> bool {
        use lbp_script_vm::natives::{dispatch, ACTION_ENGAGE_POD};
        use lbp_script_vm::Value;
        let args = [
            Value::Int(0),
            Value::Int(ACTION_ENGAGE_POD),
            Value::Str("MADE_UP_KEY".into()),
        ];
        let answer = dispatch(world, "Input", "IsJustClickedByID__iiQ6String", &args);
        matches!(answer, Some(Ok(Value::Bool(true))))
    }

    fn in_use(shell: &Side, level: &Side) -> (bool, bool) {
        (
            shell.1.world.pod_computer_in_use,
            level.1.world.pod_computer_in_use,
        )
    }

    fn pending(shell: &Side, level: &Side) -> (Option<bool>, Option<bool>) {
        (
            shell.1.world.pod.host_use_pod_pending,
            level.1.world.pod.host_use_pod_pending,
        )
    }

    /// A shell world with the pod computer trigger as pod.bin stores it (MEASURED from
    /// the level file: a box of half sizes 167.3 by 264.7 around (15.8, -532.2), z
    /// -193.7, no z offset, one layer).
    fn pod_bin_world() -> LevelWorld {
        let mut world = LevelWorld::from_things(&[]);
        let mut at = Mat4::from_scale(Vec3::new(167.3, 264.7, 1.)).to_cols_array();
        at[12..15].copy_from_slice(&[15.8, -532.2, -193.7]);
        world.things.push(lbp_runtime::ThingState {
            trigger: Some(lbp_runtime::TriggerState {
                trigger_type: 1,
                radius: 1.,
                hysteresis: 1.,
                enabled: true,
                ..Default::default()
            }),
            world_matrix: Some(at),
            script_classes: vec![lbp_runtime::POD_TRIGGER_CLASS.into()],
            ..Default::default()
        });
        world.our_pod = true;
        world
    }

    /// D281. The pad zone is decided by where the viewer's own Sackboy stands: his place
    /// goes to the shell world before each tick, the tick's trigger pass keeps the pod
    /// computer trigger's list, and the zone is that list and his z. Where the game has
    /// him on arrival (MEASURED) he is in it; walked off to the side he is not; in the
    /// lane behind he is; in the back lane the trigger holds him and the z rule says no.
    /// Part 33: the span is the fixed absolute range, so the back lane is out by the z
    /// rule only.
    #[test]
    fn d281_the_pad_zone_is_decided_by_where_the_sackboy_stands() {
        let mut shell = Side::of(pod_bin_world());
        let mut level = Side::of(LevelWorld::from_things(&[]));
        shell.1.world.hand_session_to(&mut level.1.world);
        let mut sackboy = None;
        let arrival = [-5.37, -477.97, 0.];
        let (thing, new) = stand_sackboy(&mut shell.1.world, &mut sackboy, arrival);
        assert!(new && sackboy == Some(Some(thing)));
        // Not before a tick has run the trigger pass.
        assert!(!shell.1.world.in_pad_zone(thing));
        round(&mut shell, &mut level);
        assert!(shell.1.world.in_pad_zone(thing));
        let mut at = |place: [f32; 3]| {
            assert_eq!(
                stand_sackboy(&mut shell.1.world, &mut sackboy, place),
                (thing, false)
            );
            round(&mut shell, &mut level);
            let listed = shell.1.world.things[0]
                .trigger
                .as_ref()
                .is_some_and(|g| g.inside.contains(&thing));
            (listed, shell.1.world.in_pad_zone(thing))
        };
        // The measured spot after a walk to the right: outside the box.
        assert_eq!(at([338.1, -475.53, 0.]), (false, false));
        // Back, at the measured spot beside the controller.
        assert_eq!(at([-16.23, -477.81, 0.]), (true, true));
        // The box's sides in x: 15.8 +- 167.3.
        assert_eq!(at([182., -477., 0.]), (true, true));
        assert_eq!(at([184., -477., 0.]), (false, false));
        assert_eq!(at([-152., -477., 0.]), (false, false));
        // A jump does not leave it (the box reaches up to y -267.5).
        assert_eq!(at([0., -300., 0.]), (true, true));
        assert_eq!(at([0., -260., 0.]), (false, false));
        // The lane behind: in. The back lane (part 33, the fixed span): in the trigger's
        // list, out of the zone by the pad zone's own z rule, as under -300.
        assert_eq!(at([0., -477., -200.]), (true, true));
        assert_eq!(at([0., -477., -400.]), (true, false));
        assert_eq!(at([0., -477., -310.]), (true, false));
        assert_eq!(at([0., -477., -300.]), (true, true));
        // One Sackboy all along, player 0, with the pad.
        assert_eq!(shell.1.world.players.len(), 1);
        // With the play mode's half depth the front lane does not reach the trigger's z:
        // the reason for the Pod's own span.
        let mut narrow = Side::of(pod_bin_world());
        let layers = (-PLAYER_HALF_DEPTH, PLAYER_HALF_DEPTH);
        let other = narrow.1.world.add_player(arrival, layers);
        round(&mut narrow, &mut level);
        assert!(!narrow.1.world.in_pad_zone(other));
    }

    /// D282. His z span for the layer rule is the absolute range the 01.33 creature update
    /// writes, -500 to 100, whatever lane he is in; pod.bin's trigger at z -193.7 holds
    /// him in all three, and the pad zone is decided by its own z rule: in at lanes 0 and
    /// -200, out at -400.
    #[test]
    fn d282_the_span_is_the_fixed_range_in_every_lane() {
        assert_eq!(POD_PLAYER_Z_SPAN, (-500., 100.));
        let mut shell = Side::of(pod_bin_world());
        let mut level = Side::of(LevelWorld::from_things(&[]));
        shell.1.world.hand_session_to(&mut level.1.world);
        let mut sackboy = None;
        for (lane, zone) in [(0., true), (-200., true), (-400., false), (0., true)] {
            let (thing, _) = stand_sackboy(&mut shell.1.world, &mut sackboy, [-5.37, -477., lane]);
            let player = shell.1.world.players.iter().find(|p| p.thing == thing);
            assert_eq!(
                player.map(|p| p.layers),
                Some(POD_PLAYER_Z_SPAN),
                "lane {lane}"
            );
            round(&mut shell, &mut level);
            let listed = shell.1.world.things[0]
                .trigger
                .as_ref()
                .is_some_and(|g| g.inside.contains(&thing));
            assert!(listed, "lane {lane}");
            assert_eq!(shell.1.world.in_pad_zone(thing), zone, "lane {lane}");
        }
        assert_eq!(shell.1.world.players.len(), 1);
    }

    /// D281. With the viewer's own Sackboy the press is taken where he stands in the
    /// zone and ignored where he does not, and the reason says where he is.
    #[test]
    fn d281_a_press_outside_the_pad_zone_is_ignored_with_his_place() {
        let mut shell = Side::of(pod_bin_world());
        let mut level = Side::of(LevelWorld::from_things(&[]));
        shell.1.world.progression_flags = 3;
        shell.1.world.hand_session_to(&mut level.1.world);
        let mut sackboy = None;
        let (thing, _) = stand_sackboy(&mut shell.1.world, &mut sackboy, [338.1, -475.53, 0.]);
        round(&mut shell, &mut level);
        let mut press = true;
        assert_eq!(
            feed_pad(&mut shell.1.world, &mut level.1.world, &mut press),
            0x80
        );
        assert!(clicked(&mut shell.1.world) && !shell.1.world.in_pad_zone(thing));
        let why = press_ignored(&shell.1.world, sackboy.flatten());
        assert!(
            why.contains("not in the pad zone") && why.contains("338.1"),
            "{why}"
        );
        // Walked back, the same press finds him in the zone.
        stand_sackboy(&mut shell.1.world, &mut sackboy, [-5.37, -477.97, 0.]);
        round(&mut shell, &mut level);
        assert!(shell.1.world.in_pad_zone(thing));
        let why = press_ignored(&shell.1.world, sackboy.flatten());
        assert!(why.contains("another condition"), "{why}");
    }

    /// D279. A press is the engage bit on one tick and nothing on the next, on both
    /// worlds' pads: the game's "just clicked" is true on that tick alone.
    #[test]
    fn d279_a_press_is_one_tick_long() {
        let mut press = true;
        assert_eq!(pad_word(&mut press), 0x80);
        assert!(!press);
        assert_eq!(pad_word(&mut press), 0);
        assert_eq!(ACTION_ENGAGE_POD_BUTTON, 0x80);

        let (mut shell, mut level) = session();
        let words = |shell: &Side, level: &Side| {
            let word = |side: &Side| side.1.world.player_records[0].pad.map(|p| p.now);
            (word(shell), word(level))
        };
        // Before the first tick nobody has a pad: no click, whatever is asked.
        assert_eq!(words(&shell, &level), (None, None));
        assert!(!clicked(&mut shell.1.world));
        let mut press = false;
        // Tick 1: no press. The pads are there, with nothing down.
        let word = feed_pad(&mut shell.1.world, &mut level.1.world, &mut press);
        assert_eq!(word, 0);
        assert_eq!(words(&shell, &level), (Some(0), Some(0)));
        assert!(!clicked(&mut shell.1.world));
        round(&mut shell, &mut level);
        // Tick 2: the press. Down on both pads, a click in both worlds.
        press = true;
        let word = feed_pad(&mut shell.1.world, &mut level.1.world, &mut press);
        assert_eq!(word, 0x80);
        assert!(!press);
        assert_eq!(words(&shell, &level), (Some(0x80), Some(0x80)));
        assert!(clicked(&mut shell.1.world) && clicked(&mut level.1.world));
        round(&mut shell, &mut level);
        // Tick 3: up again, and no click. Nor on tick 4.
        for _ in 0..2 {
            let word = feed_pad(&mut shell.1.world, &mut level.1.world, &mut press);
            assert_eq!(word, 0);
            assert_eq!(words(&shell, &level), (Some(0), Some(0)));
            assert!(!clicked(&mut shell.1.world) && !clicked(&mut level.1.world));
            round(&mut shell, &mut level);
        }
        // A second press is a second click.
        press = true;
        feed_pad(&mut shell.1.world, &mut level.1.world, &mut press);
        assert!(clicked(&mut shell.1.world));
        // The prompt of the action was asked for on every asking with a pad.
        let prompts = &shell.1.world.pod.action_prompts;
        assert_eq!(prompts.get("MADE_UP_KEY"), Some(&5));
    }

    /// D279. The keys under the scripts: the opening key presses while the computer is
    /// free and asks to close while it is in use; the closing key only asks to close.
    /// Asking to close goes through the host: "in use" ends on both worlds with the next
    /// tick, and the shut-down sound is counted where the Sackboy is.
    #[test]
    fn d279_the_key_while_in_use_asks_the_host_to_close() {
        use crate::pod_menu::puter_key as key_of;
        use lbp_runtime::{POD_COMPUTER_SHUTDOWN_SOUND, POD_COMPUTER_STARTUP_SOUND};
        assert_eq!(key_act(PuterKey::Toggle, false), Some(KeyAct::Press));
        assert_eq!(key_act(PuterKey::Toggle, true), Some(KeyAct::Close));
        assert_eq!(key_act(PuterKey::Back, true), Some(KeyAct::Close));
        assert_eq!(key_act(PuterKey::Back, false), None);
        // The key of a frame: closing wins over opening.
        assert_eq!(key_of(false, false), None);
        assert_eq!(key_of(true, false), Some(PuterKey::Toggle));
        assert_eq!(key_of(false, true), Some(PuterKey::Back));
        assert_eq!(key_of(true, true), Some(PuterKey::Back));

        let (mut shell, mut level) = session();
        let (sackboy, spot) = place_sackboy(&mut shell.1.world).unwrap();
        assert_eq!(spot, [40., -470., -15.]);
        shell.1.world.hand_session_to(&mut level.1.world);
        set_in_use(&mut level.1.world, Some(&mut shell.1.world), true);
        round(&mut shell, &mut level);
        assert_eq!(in_use(&shell, &level), (true, true));
        assert!(shell.1.world.in_pad_zone(sackboy));
        // The request waits in both worlds and changes nothing yet.
        ask_to_close(&mut shell.1.world, &mut level.1.world);
        assert_eq!(pending(&shell, &level), (Some(false), Some(false)));
        assert_eq!(in_use(&shell, &level), (true, true));
        // The host's answer comes with the next tick, in both worlds.
        round(&mut shell, &mut level);
        assert_eq!(in_use(&shell, &level), (false, false));
        assert_eq!(pending(&shell, &level), (None, None));
        let sounds = &shell.1.world.sounds_played;
        assert_eq!(sounds.get(POD_COMPUTER_SHUTDOWN_SOUND), Some(&1));
        assert_eq!(sounds.get(POD_COMPUTER_STARTUP_SOUND), None);
        assert!(level.1.world.sounds_played.is_empty());
        assert_eq!(shell.1.world.pod.in_use_changes.len(), 1);
        // Outside our Pod the request is dropped, as the game drops it.
        shell.1.world.our_pod = false;
        ask_to_close(&mut shell.1.world, &mut level.1.world);
        assert_eq!(pending(&shell, &level).0, None);
        assert_eq!(shell.1.world.pod.host_use_pod_refused, 1);
        // A level without a pod computer trigger gets no Sackboy.
        assert_eq!(place_sackboy(&mut LevelWorld::from_things(&[])), None);
    }

    /// D279. `--level-scripts-press-at N` is the number of a tick and fires once, on
    /// the first tick about to run that is tick N or a later one.
    #[test]
    fn d279_the_press_option_is_a_tick_number_and_fires_once() {
        let args = |list: &[&str]| -> Vec<String> { list.iter().map(|a| a.to_string()).collect() };
        let flag = "--level-scripts-press-at";
        assert_eq!(tick_option(&args(&["--level-scripts"]), flag), Ok(None));
        assert_eq!(tick_option(&args(&[flag, "600"]), flag), Ok(Some(600)));
        // The other tick options are not this one.
        let other = args(&["--level-scripts-in-use-at", "600"]);
        assert_eq!(tick_option(&other, flag), Ok(None));
        for bad in ["-1", "soon", "0x80"] {
            let e = tick_option(&args(&[flag, bad]), flag).unwrap_err();
            assert_eq!(e, "--level-scripts-press-at takes a tick count");
        }
        // 599 ticks have run: tick 600 is about to. Not before, and not again.
        let mut at = Some(600);
        assert!(!press_now(&mut at, 1) && !press_now(&mut at, 599));
        assert_eq!(at, Some(600));
        assert!(press_now(&mut at, 600));
        assert_eq!(at, None);
        assert!(!press_now(&mut at, 601));
        // A tick already passed, and the first tick for 0 and 1.
        assert!(press_now(&mut Some(3), 7));
        assert!(press_now(&mut Some(0), 1) && press_now(&mut Some(1), 1));
        assert!(!press_now(&mut None, 7));
        // With the pad: the option's press is on that tick's word alone.
        let mut at = Some(3);
        let words: Vec<u32> = (1..=5)
            .map(|next| {
                let mut press = press_now(&mut at, next);
                pad_word(&mut press)
            })
            .collect();
        assert_eq!(words, [0, 0, 0x80, 0, 0]);
    }

    /// D279. The path from the press to "in use" over a session of two worlds, with the
    /// one step of pod.bin's script stood in for (there is no script in a test): when
    /// the Sackboy is in the pad zone, the flags are there and the engage action was
    /// just clicked, the script asks the host for the pod computer during its tick.
    /// Press on tick N: the request on N, "in use" on both worlds on N + 1, with the
    /// start-up sound counted where the Sackboy is. Without the flags the press is
    /// ignored and the reason names them.
    #[test]
    fn d279_a_press_leads_to_in_use_on_both_worlds_a_tick_later() {
        let (mut shell, mut level) = session();
        shell.1.world.hand_session_to(&mut level.1.world);
        assert!(level.1.world.our_pod);
        let sackboy = place_sackboy(&mut shell.1.world).map(|(thing, _)| thing);
        assert!(sackboy.is_some());
        // One tick of the session: the pad, the script's stand-in, the round. Gives
        // (the pad's word, a request was made, in use in the shell and in the level).
        let tick = |shell: &mut Side, level: &mut Side, press: &mut bool| {
            let word = feed_pad(&mut shell.1.world, &mut level.1.world, press);
            let w = &mut shell.1.world;
            let wanted = clicked(w)
                && sackboy.is_some_and(|thing| w.in_pad_zone(thing))
                && w.progression_flags & ENGAGE_FLAGS != 0
                && !w.pod_computer_in_use;
            round(shell, level);
            if wanted {
                // During the shell's tick, so after the host's answer of this round was
                // taken; the session hands the request on after the shell's tick.
                shell.1.world.request_host_use_pod(true);
                shell.1.world.hand_session_to(&mut level.1.world);
            }
            (word, wanted, in_use(shell, level))
        };
        let (free, used) = ((false, false), (true, true));
        let mut press = false;
        // Tick 1: he is put in the trigger's list.
        assert_eq!(tick(&mut shell, &mut level, &mut press), (0, false, free));
        assert!(shell.1.world.in_pad_zone(sackboy.unwrap()));
        // Tick 2: a press before the flags. Ignored, and the reason is the flags.
        press = true;
        assert_eq!(
            tick(&mut shell, &mut level, &mut press),
            (0x80, false, free)
        );
        let why = press_ignored(&shell.1.world, sackboy);
        assert!(why.contains("progression flags are 0x0"), "{why}");
        assert_eq!(tick(&mut shell, &mut level, &mut press), (0, false, free));
        // The flags arrive from the other level (the intro), and reach the shell.
        level.1.world.progression_flags = 3;
        level.1.world.hand_session_to(&mut shell.1.world);
        assert_eq!(tick(&mut shell, &mut level, &mut press), (0, false, free));
        // Tick 5: the press. The request on this tick, "in use" not yet.
        press = true;
        assert_eq!(tick(&mut shell, &mut level, &mut press), (0x80, true, free));
        assert_eq!(pending(&shell, &level), (Some(true), Some(true)));
        // Tick 6: the host's answer, on both worlds; the pad is up and nothing is asked.
        assert_eq!(tick(&mut shell, &mut level, &mut press), (0, false, used));
        assert_eq!((shell.2.ticks, level.2.ticks), (6, 6));
        let sounds = &shell.1.world.sounds_played;
        assert_eq!(
            sounds.get(lbp_runtime::POD_COMPUTER_STARTUP_SOUND),
            Some(&1)
        );
        assert!(level.1.world.sounds_played.is_empty());
        assert_eq!(shell.1.world.pod.host_use_pod_requests.len(), 1);
        // It stays in use; a press now is no request, and the reason says so.
        press = true;
        assert_eq!(
            tick(&mut shell, &mut level, &mut press),
            (0x80, false, used)
        );
        assert_eq!(
            press_ignored(&shell.1.world, sackboy),
            "the pod computer is in use"
        );
        // The other reasons: nobody on a trigger; out of the pad zone.
        assert!(press_ignored(&shell.1.world, None).contains("no Sackboy"));
        let w = &mut shell.1.world;
        w.pod_computer_in_use = false;
        w.move_player(sackboy.unwrap(), [40., -470., -400.], (-401., -399.));
        assert!(press_ignored(w, sackboy).contains("not in the pad zone"));
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

    /// The viewer's lens before any script camera (`setup` in main.rs), and a display
    /// scale like the planets level's (its 0.01 and 100 display units are 64 and
    /// 636,705 level units).
    const VIEWER_LENS: Lens = Lens {
        fov: std::f32::consts::FRAC_PI_4,
        near: 0.01,
        far: 100.,
    };
    const PLANETS_SCALE: f32 = 100. / 636_705.;

    /// A camera written by `Cam.SetCameraFromToUp` with `params` (near, far, ortho
    /// factor, field of view), as the runtime shows it.
    fn written(params: [f32; 4]) -> (DisplayCamera, f32) {
        use lbp_runtime::ScriptCamera;
        use lbp_script_vm::natives::CameraFromToUp;
        let call = CameraFromToUp {
            from: [0., -250., 1130., 1.],
            to: [0., 0., -750., 1.],
            up: [0., 1., 0., 0.],
            params,
        };
        let c = ScriptCamera::of(&call, 0).unwrap();
        let shown = DisplayCamera {
            source: CameraSource::Script,
            pose: c.pose,
            up: c.up,
            fov_y_deg: c.fov_y_deg(),
            near: c.near,
            far: c.far,
        };
        (shown, c.lens_value)
    }

    /// D283. The Pod's call (10, 100,000, 0, 1.0): a perspective lens of 1 radian
    /// (57.3 degrees) vertical field of view, near and far from level units to display
    /// units by the level's display scale; the aspect is the viewer's, 16:9.
    #[test]
    fn d283_the_lens_is_the_natives_arguments_in_display_units() {
        let (camera, ortho) = written([10., 100_000., 0., 1.]);
        assert_eq!(ortho, 0.);
        let lens = script_lens(&camera, ortho, PLANETS_SCALE, VIEWER_LENS);
        assert!((lens.fov - 1.).abs() < 1e-6);
        assert!((lens.fov.to_degrees() - 57.296).abs() < 1e-3);
        assert!((lens.near - 10. * PLANETS_SCALE).abs() < 1e-9);
        assert!((lens.far - 100_000. * PLANETS_SCALE).abs() < 1e-4);
        // 0.00157 and 15.71 display units; the viewer's own were 64 and 636,705 level
        // units, the game's are 10 and 100,000.
        assert!((lens.near - 0.001_570_6).abs() < 1e-7, "{}", lens.near);
        assert!((lens.far - 15.706).abs() < 1e-3, "{}", lens.far);
        let mut perspective = PerspectiveProjection {
            aspect_ratio: 16. / 9.,
            ..Default::default()
        };
        lens.apply(&mut perspective);
        assert_eq!(Lens::of(&perspective), lens);
        assert_eq!(perspective.aspect_ratio, 16. / 9.);
        // A level point on the picture's top edge at that field of view: the projection
        // is a perspective one, x and y over the depth.
        let clip = perspective.get_clip_from_view();
        for depth in [100., 1_000.] {
            let edge = clip.project_point3(Vec3::new(0., depth * 0.5f32.tan(), -depth));
            assert!((edge.y - 1.).abs() < 1e-4, "{edge:?}");
            let side = clip.project_point3(Vec3::new(depth * 0.5f32.tan() * 16. / 9., 0., -depth));
            assert!((side.x - 1.).abs() < 1e-4, "{side:?}");
        }
        // An ortho factor other than 0 is not drawn: the viewer's lens stays whole.
        let (camera, ortho) = written([10., 100_000., 0.5, 1.]);
        assert_eq!(
            script_lens(&camera, ortho, PLANETS_SCALE, VIEWER_LENS),
            VIEWER_LENS
        );
        // Planes the game would not rebuild with keep the viewer's; the field of view
        // still is the script's.
        let (camera, ortho) = written([0., 100_000., 0., 1.]);
        let lens = script_lens(&camera, ortho, PLANETS_SCALE, VIEWER_LENS);
        assert_eq!((lens.near, lens.far), (VIEWER_LENS.near, VIEWER_LENS.far));
        assert!((lens.fov - 1.).abs() < 1e-6);
        // A field of view no lens can have keeps the viewer's.
        let (camera, ortho) = written([10., 100_000., 0., 4.]);
        let lens = script_lens(&camera, ortho, PLANETS_SCALE, VIEWER_LENS);
        assert_eq!(lens.fov, VIEWER_LENS.fov);
    }

    /// D283. The override camera draws with the game camera's lens: 60 degrees, near 10
    /// and far 40,000 level units.
    #[test]
    fn d283_the_override_lens_is_the_game_cameras() {
        let (written, _) = written([10., 100_000., 0., 1.]);
        let over = DisplayCamera {
            source: CameraSource::Override,
            fov_y_deg: Some(OVERRIDE_FOV_DEG),
            near: lbp_runtime::OVERRIDE_NEAR,
            far: lbp_runtime::OVERRIDE_FAR,
            ..written
        };
        let lens = script_lens(&over, 0., PLANETS_SCALE, VIEWER_LENS);
        assert!((lens.fov.to_degrees() - 60.).abs() < 1e-4);
        assert!((lens.near / PLANETS_SCALE - 10.).abs() < 1e-3);
        assert!((lens.far / PLANETS_SCALE - 40_000.).abs() < 0.1);
    }

    /// D283. Without a script camera the viewer's lens is not touched; under one it is
    /// the script's, and when the camera goes out the lens of before comes back once.
    #[test]
    fn d283_a_level_without_a_script_camera_keeps_the_old_lens() {
        let mut follow = None;
        for _ in 0..3 {
            assert_eq!(
                wanted_lens(None, &mut follow, Some(VIEWER_LENS), PLANETS_SCALE),
                None
            );
            assert_eq!(follow, None);
        }
        let (camera, ortho) = written([10., 100_000., 0., 1.]);
        let pod = script_lens(&camera, ortho, PLANETS_SCALE, VIEWER_LENS);
        let first = wanted_lens(
            Some((&camera, ortho)),
            &mut follow,
            Some(VIEWER_LENS),
            PLANETS_SCALE,
        );
        assert_eq!(first, Some(pod));
        assert_eq!(follow, Some(VIEWER_LENS));
        // The next frame the projection has the script's lens; the kept one stays.
        let next = wanted_lens(
            Some((&camera, ortho)),
            &mut follow,
            Some(pod),
            PLANETS_SCALE,
        );
        assert_eq!((next, follow), (Some(pod), Some(VIEWER_LENS)));
        // Out: the viewer's own lens back, then nothing more is written.
        assert_eq!(
            wanted_lens(None, &mut follow, Some(pod), PLANETS_SCALE),
            Some(VIEWER_LENS)
        );
        assert_eq!(
            wanted_lens(None, &mut follow, Some(VIEWER_LENS), PLANETS_SCALE),
            None
        );
    }
}
