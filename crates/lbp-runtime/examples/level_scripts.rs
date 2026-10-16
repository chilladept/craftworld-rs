//! Rt PC check (docs/level-scripts.md): which scripts levels use and how they run.
//!
//! level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH NEW-JSON [--entry=NAME]... [--ticks=N]
//! level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH NEW-JSON --play=TICKS [--flags=MASK,..]
//!     [--puter-in-use=TICK] [--our-pod] [--enter-pad-zone=TICK] [--press=TICK:MASK,..]
//!     [--menu=TICK:KEY,..] (D325: up, down, left, right, choose, back on the drawn menu)
//!     [--player-colour=AARRGGBB] [--with-level=LEVEL-PATH]
//! level_scripts SNAPSHOT GAME-DIRS --all NEW-JSON [--play=TICKS] [--strict]
//!     [--visit-triggers] [--pulse-switches]
//!
//! Loads every magic-confirmed script with its MAP GUID (the U12 loader), reads the level(s)
//! and reports, per level: scripted Things (PScript with a resource GUID), how many resolve
//! to a loaded class, unresolved GUIDs, and whether the stored field names match the class
//! layout (class fields and superclass fields). Over all classes in use it counts function
//! names that have code, which is how the create/update entry names get settled. With
//! `--entry`, one level runs that function on every scripted Thing and resumes yielded
//! scripts for `--ticks` ticks (default 30), reporting outcomes, faults, unbound natives
//! and `WRITE` lines. `--play` runs the `LevelScripts` schedule instead: `OnCreate__`, then
//! TICKS ticks of resume + `Update__`; `--puter-in-use=TICK` makes
//! `GetPodComputerInUse__` true from that tick on (0 = from `OnCreate__`) and reports the
//! camera as it was just before the switch too. `--all --play=TICKS` plays every level on a
//! fresh VM and adds `play_all`: native calls bound and defaulted, each unbound native with
//! its calls and levels, and the remaining faults by kind. A play run goes past unbound
//! natives (they return their type's default and are counted); `--strict` makes them fault
//! instead. `--visit-triggers` then adds a player who stands on every PTrigger's Thing in
//! turn (two ticks on it, one far away), so the level's `Enter`, `Inside` and `Leave`
//! scripts run; the report counts the events raised and run. He then visits every player
//! sensor the same way (in its range, then far away), so the sensors turn on and off.
//! `--pulse-switches` then runs
//! two more ticks in which every switch output is forced on, then off, through the per-tick
//! switch pass, so the `SwitchPerformReaction` scripts run with true, then false (a test
//! drive: the pass itself computes the logic types and the tag and player sensors, and
//! keeps the stored signals of the other sensors). `sensors` in `switches` is the census of
//! the tag and player sensors and the tags; `play_all.sensors_unlike_stored` counts the
//! sensors whose output differed from the stored one on some tick. `switch_reactions` counts the
//! output targets the pass visited and the reaction calls by script GUID. `triggers` is the PTrigger
//! census over the levels read, `switches` the PSwitch census with the input tables built
//! from the outputs' targets, `pworld` the census of the PWorld values scripts read (game
//! mode, pause flag, frame, water level, global-setting arrays). The English base and patch
//! translation tables (the two GUIDs the story browser reads) are handed to every level
//! world, patch first, so `LAMS.IsInLams__i` and `Translate__i` answer from them;
//! `translation_tables` says what was loaded. `play_all.requests_kept` counts the recorded
//! calls kept with their arguments, by name. `play_all.messages` counts what the messaging
//! natives did to the world's message list: requests, messages opened, written over and
//! dropped by type, their text (a string of a loaded table, another string, empty), actor
//! names, anchors, messages closed and those open at the end. `play_all.global_settings`
//! counts the per-tick `ApplyGlobalSettings__` calls by setting type, the slots cleared,
//! and by type the levels and ticks on which the setting in force was not the one in force
//! when the level was read. `play_all.display` counts the Things whose render-mesh
//! visibility, colours or world matrix are unlike the level file's after the last tick,
//! the levels with such a Thing, and the changes marked for a display on the way.
//! `play_all.camera` counts the override camera's calls, the levels and ticks on which it
//! was in, and the camera matrix and screen position queries.
//! `play_all.pod` counts the Things with a PScriptName and their levels, the
//! `Thing.FindThing` and `Thing.GetPodThing` calls and how many found a Thing, the levels
//! with a pod Thing, and the mesh changes and fade values the Pod's script set; one
//! level's `pod` also names its named Things.
//! Part 24, the pod computer: a level is started as our Pod with `--our-pod`, and
//! without it when the level has a pod Thing (the tool's stand-in for "entered from the
//! Pod's slot": the game looks at the slot, not at the level). `--enter-pad-zone=TICK`
//! adds the player's Sackboy on the pod computer's trigger from that tick on (the first
//! tick is 1). `--press=TICK:MASK,..` gives player 0 a pad whose button word is MASK on
//! that tick and 0 on the others (0x80 is the engage action, 0x80000 Start).
//! `--player-colour` sets the players' stored colour. `--with-level=PATH` plays a second
//! level in the same session (the Pod and its planets level): both tick each round, the
//! first given first, and share the progression flags, "our Pod", "pod computer in use"
//! and the media requests; the report is `runs.session`, with the path of events tick by
//! tick. `play_all.pod` adds the pad-zone, action and player-colour calls.
//! Part 28: in a `--with-level` session `--puter-in-use=TICK` sets "in use" on both
//! levels when TICK rounds have run (the stand-in the viewer's Puter key uses).
//! Part 25: `play_all.pod` adds the planets' per-frame calls, those that answered
//! "arrived", the levels with such a call and the pod computer's per-frame video calls;
//! one level's `planets` lists each planet after ticks 1, 226, 452 and the last.
//! Names and counts only; no bytecode. Output: a NEW JSON file under
//! ignored `local-analysis/`. Counting unit: Thing with a PScript part (instances) or
//! distinct script class (function names).
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_runtime::{
    instances, tick_session, BubbleStyle, GlobalSettingsReport, LevelScripts, LevelWorld,
    MessageKind, Outcome, ReactionReport, Runner, SessionLevel, StepReport, GLOBAL_SETTINGS,
    GLOBAL_SETTING_NAMES,
};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Vm;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const BUDGET_PER_CALL: u64 = 1_000_000;
const MAX_FAULTS: usize = 30;
/// D319: drawn menu matrices kept in a session report (one a tick).
const MAX_MENU_MATRICES: usize = 2000;

fn main() {
    if let Err(e) = run() {
        eprintln!("level_scripts: {e}");
        std::process::exit(1);
    }
}

#[derive(Default)]
struct Totals {
    levels: usize,
    levels_not_read: usize,
    things: usize,
    scripted: usize,
    with_fields: usize,
    resolved: usize,
    unresolved: BTreeMap<u32, usize>,
    by_class: BTreeMap<String, usize>,
    fields_matched: usize,
    fields_missing_in_class: usize,
    unconverted: usize,
    objects_built: usize,
    classes: BTreeSet<lbp_script_vm::ClassId>,
    triggers: Triggers,
    switches: Switches,
    pworld: WorldCensus,
}

/// Census of the stored PWorld values (`LevelWorld::pworld`). Unit: level.
#[derive(Default)]
struct WorldCensus {
    without_world: usize,
    game_mode: BTreeMap<i32, usize>,
    is_paused: BTreeMap<bool, usize>,
    /// Stored `frame` by size class, and the most common exact values.
    frame_ranges: BTreeMap<&'static str, usize>,
    frame_values: BTreeMap<i32, usize>,
    frame_equals_sim_frame: usize,
    water_level_not_zero: usize,
    /// Lengths of the two global-setting arrays ("uids/priorities").
    global_setting_lengths: BTreeMap<String, usize>,
    with_a_global_setting_uid: usize,
    global_setting_priorities: BTreeMap<i32, usize>,
}

impl WorldCensus {
    fn add(&mut self, world: &LevelWorld) {
        if world.world.is_none() {
            self.without_world += 1;
            return;
        }
        let p = &world.pworld;
        *self.game_mode.entry(p.game_mode).or_default() += 1;
        *self.is_paused.entry(p.is_paused).or_default() += 1;
        let range = match p.frame {
            i32::MIN..=-1 => "negative",
            0 => "0",
            1..=30 => "1 to 30",
            31..=1800 => "31 to 1800 (a minute)",
            1801..=108_000 => "1801 to 108000 (an hour)",
            _ => "above 108000",
        };
        *self.frame_ranges.entry(range).or_default() += 1;
        *self.frame_values.entry(p.frame).or_default() += 1;
        self.frame_equals_sim_frame += usize::from(p.frame == p.sim_frame);
        self.water_level_not_zero += usize::from(p.water_level != 0.);
        let lengths = format!(
            "{}/{}",
            p.global_settings_thing_uids.len(),
            p.global_settings_thing_priority.len()
        );
        *self.global_setting_lengths.entry(lengths).or_default() += 1;
        self.with_a_global_setting_uid +=
            usize::from(p.global_settings_thing_uids.iter().any(|u| *u != 0));
        for v in &p.global_settings_thing_priority {
            *self.global_setting_priorities.entry(*v).or_default() += 1;
        }
    }

    fn json(&self) -> serde_json::Value {
        let mut common: Vec<_> = self.frame_values.iter().collect();
        common.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        common.truncate(12);
        serde_json::json!({
            "unit": "level with a PWorld; global_setting_priorities counts array entries",
            "levels_without_world": self.without_world,
            "game_mode": self.game_mode,
            "is_paused": self.is_paused.iter()
                .map(|(k, n)| (k.to_string(), *n)).collect::<BTreeMap<_, _>>(),
            "frame_ranges": self.frame_ranges,
            "frame_distinct_values": self.frame_values.len(),
            "frame_min": self.frame_values.keys().next(),
            "frame_max": self.frame_values.keys().next_back(),
            "frame_most_common": common,
            "frame_equals_sim_frame": self.frame_equals_sim_frame,
            "water_level_not_zero": self.water_level_not_zero,
            "global_setting_array_lengths": self.global_setting_lengths,
            "levels_with_a_global_setting_uid": self.with_a_global_setting_uid,
            "global_setting_priorities": self.global_setting_priorities,
        })
    }
}

/// PTrigger census. Unit: Thing with a PTrigger part.
#[derive(Default)]
struct Triggers {
    count: usize,
    by_type: BTreeMap<i8, usize>,
    not_enabled: usize,
    all_z_layers: usize,
    stored_inside: usize,
    no_position: usize,
    /// Where the script that gets Enter and Leave is: "own", "parent" or "none".
    script_at: BTreeMap<&'static str, usize>,
    /// "Class (own)" / "Class (parent)" with the trigger functions the class has.
    targets: BTreeMap<String, usize>,
    radius: Vec<f32>,
    /// Length of the trigger Thing's x axis (its scale).
    scale: Vec<f32>,
}

impl Triggers {
    fn add(&mut self, vm: &Vm, world: &LevelWorld, found: &[lbp_runtime::Instance]) {
        let class_at = |thing: u32| {
            let i = found.binary_search_by_key(&thing, |i| i.thing).ok()?;
            Some(
                found[i]
                    .class
                    .map_or("(class not loaded)", |c| vm.class_name(c)),
            )
        };
        let functions = |thing: u32| {
            let i = found.binary_search_by_key(&thing, |i| i.thing).ok()?;
            let c = found[i].class?;
            Some(
                ["Enter__Q5Thing", "Inside__Q5Thing", "Leave__Q5Thing"]
                    .iter()
                    .filter(|f| vm.has_code(c, f))
                    .map(|f| &f[..f.len() - "__Q5Thing".len()])
                    .collect::<Vec<_>>()
                    .join("+"),
            )
        };
        for (i, t) in world.things.iter().enumerate() {
            let Some(g) = &t.trigger else {
                continue;
            };
            self.count += 1;
            *self.by_type.entry(g.trigger_type).or_default() += 1;
            self.not_enabled += usize::from(!g.enabled);
            self.all_z_layers += usize::from(g.all_z_layers);
            self.stored_inside += usize::from(!g.inside.is_empty());
            self.radius.push(g.radius);
            match t.world_matrix {
                Some(m) => self
                    .scale
                    .push((m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt()),
                None => self.no_position += 1,
            }
            let own = class_at(i as u32).map(|c| (c, "own", i as u32));
            let parent = t.parent.and_then(|p| Some((class_at(p)?, "parent", p)));
            match own.or(parent) {
                Some((class, at, thing)) => {
                    *self.script_at.entry(at).or_default() += 1;
                    let f = functions(thing).unwrap_or_default();
                    *self
                        .targets
                        .entry(format!("{class} ({at}) [{f}]"))
                        .or_default() += 1;
                }
                None => *self.script_at.entry("none").or_default() += 1,
            }
        }
    }

    fn json(&self) -> serde_json::Value {
        let spread = |v: &[f32]| {
            let mut v: Vec<f32> = v.iter().copied().filter(|x| x.is_finite()).collect();
            v.sort_by(f32::total_cmp);
            match v.len() {
                0 => serde_json::Value::Null,
                n => serde_json::json!({
                    "min": v[0], "p10": v[n / 10], "median": v[n / 2], "p90": v[n * 9 / 10],
                    "max": v[n - 1],
                }),
            }
        };
        let mut targets: Vec<_> = self.targets.iter().collect();
        targets.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        serde_json::json!({
            "unit": "Thing with a PTrigger part",
            "count": self.count,
            "by_type": self.by_type,
            "not_enabled": self.not_enabled,
            "all_z_layers": self.all_z_layers,
            "stored_inside_not_empty": self.stored_inside,
            "no_position": self.no_position,
            "script_at": self.script_at,
            "targets": targets,
            "radius": spread(&self.radius),
            "thing_scale": spread(&self.scale),
        })
    }
}

/// PSwitch census and the input tables built from it (`LevelWorld::wire_inputs`).
/// Unit: Thing, switch output or output target, as each line says.
#[derive(Default)]
struct Switches {
    count: usize,
    by_type: BTreeMap<i32, usize>,
    inverted: usize,
    outputs: usize,
    outputs_null: usize,
    targets: usize,
    dangling_targets: usize,
    /// Outputs by ternary value and by "activation is zero" as stored.
    ternary: BTreeMap<i32, usize>,
    activation_not_zero: usize,
    with_player: usize,
    things_with_inputs: usize,
    wired_ports: usize,
    most_ports: usize,
    collisions: usize,
    ports_dropped: usize,
    scripted_with_input: usize,
    /// Class of the scripted Things with a wired port.
    scripted_classes: BTreeMap<String, usize>,
    /// Scripted Things `BaseWorld.IsScoreBoard__Q5Thing` holds for.
    score_boards: usize,
    /// AND (14), OR (21) and NOT (23) gates by "t<type> mode=<word> inverted=<flag>".
    gates: BTreeMap<String, usize>,
    /// Counts of the stored logic settings, by name.
    logic: BTreeMap<&'static str, usize>,
    /// Timers by behaviour.
    timer_behaviours: BTreeMap<i32, usize>,
    /// Battery (28) and toggle (17) outputs by stored output signal against the stored
    /// ManualActivation ("t<type> out=<ternary>/<player> manual=<ternary>/<player>").
    manual_outputs: BTreeMap<String, usize>,
    /// Player sensors (3), tag sensors (4) and tags (PSwitchKey) by setting.
    sensors: BTreeMap<String, usize>,
}

impl Switches {
    /// Census of the tag and player sensors and of the tags.
    fn add_sensors(&mut self, world: &LevelWorld) {
        let mut count = |name: String, holds: bool| {
            *self.sensors.entry(name).or_default() += usize::from(holds);
        };
        let placed = |i: usize| {
            let proxy = world.sensor_proxy(i as u32) as usize;
            world.things[proxy].world_matrix.is_some()
        };
        let keys: BTreeSet<_> = world.things.iter().filter_map(|t| t.switch_key).collect();
        for (i, t) in world.things.iter().enumerate() {
            if t.switch_key.is_some() {
                let wired = matches!(t.inputs.first(), Some(Some(_)));
                count("tags".into(), true);
                count("tags port 0 wired".into(), wired);
                count("tags without a position".into(), !placed(i));
            }
            let Some(s) = t.switch.as_ref() else {
                continue;
            };
            let p = match s.switch_type {
                3 => "player",
                4 => "tag",
                _ => continue,
            };
            let (cone, layer) = (s.angle_range < 180., s.include_touching != 0);
            let bullets = s.bullets_required.clamp(-1, 5);
            count(p.into(), true);
            count(format!("{p} inverted"), s.inverted);
            count(format!("{p} angle_range below 180"), cone);
            count(format!("{p} include_touching not 0"), layer);
            count(format!("{p} cone or layer test"), cone || layer);
            count(format!("{p} include_touching={}", s.include_touching), true);
            count(format!("{p} radius negative"), s.radius < 0.);
            count(format!("{p} min_radius not 0"), s.min_radius != 0.);
            count(format!("{p} bullets_required={bullets}"), true);
            count(format!("{p} detect_unspawned"), s.detect_unspawned_players);
            count(format!("{p} without a position"), !placed(i));
            if s.switch_type == 3 {
                count("player require_all".into(), s.require_all);
                count(format!("player player_mode={}", s.player_mode), true);
            } else {
                let versus = s.team_filter != 0 && matches!(world.pworld.game_mode, 1 | 5);
                count(format!("tag key_sensor_mode={}", s.key_sensor_mode), true);
                count(format!("tag team_filter={}", s.team_filter), true);
                count("tag team_filter not 0 in game mode 1 or 5".into(), versus);
                count(
                    "tag without a tag in the level".into(),
                    !keys.contains(&s.key),
                );
            }
        }
    }

    fn add(&mut self, vm: &Vm, world: &LevelWorld, found: &[lbp_runtime::Instance]) {
        self.collisions += world.input_collisions;
        self.ports_dropped += world.input_ports_dropped;
        self.add_sensors(world);
        for t in &world.things {
            if let Some(s) = &t.switch {
                self.count += 1;
                *self.by_type.entry(s.switch_type).or_default() += 1;
                self.inverted += usize::from(s.inverted);
                self.outputs += s.outputs.len();
                if matches!(s.switch_type, 14 | 21 | 23) {
                    let key = format!(
                        "t{} mode={} inverted={}",
                        s.switch_type, s.gate_mode, s.inverted
                    );
                    *self.gates.entry(key).or_default() += 1;
                }
                if matches!(s.switch_type, 17 | 28) {
                    let m = s.manual_activation;
                    for o in s.outputs.iter().flatten() {
                        let key = format!(
                            "t{} inverted={} out={}/{} manual={}/{}",
                            s.switch_type,
                            s.inverted,
                            o.stored.ternary,
                            o.stored.player,
                            m.ternary,
                            m.player
                        );
                        *self.manual_outputs.entry(key).or_default() += 1;
                    }
                }
                if s.switch_type == 16 {
                    *self.timer_behaviours.entry(s.behavior).or_default() += 1;
                }
                let get = |i: Option<u32>| world.things.get(i? as usize);
                let board = get(t.circuit_board);
                let is = |t: Option<&lbp_runtime::ThingState>, ty| {
                    t.and_then(|t| t.switch.as_ref())
                        .is_some_and(|s| s.switch_type == ty)
                };
                let chip = s.switch_type == 36;
                let owner = get(t.parent);
                for (name, holds) in [
                    ("old_lbp1_switch_flag", s.old_lbp1_switch),
                    ("old_joint_flag", s.old_joint),
                    (
                        "counters_with_team_filter",
                        s.switch_type == 15 && s.team_filter != 0,
                    ),
                    ("chips", chip),
                    (
                        "chips_cursor_include_touching",
                        chip && s.include_touching > 0,
                    ),
                    (
                        "chips_with_microchip_part",
                        chip && t.parts.contains(&"MICROCHIP"),
                    ),
                    ("chips_with_board", chip && board.is_some()),
                    ("chips_board_is_type_37", chip && is(board, 37)),
                    (
                        "chips_board_parent_is_chip",
                        chip && board
                            .is_some_and(|b| get(b.parent).is_some_and(|p| std::ptr::eq(p, t))),
                    ),
                    (
                        "chips_port0_wired",
                        chip && t.inputs.first().is_some_and(Option::is_some),
                    ),
                    ("boards", s.switch_type == 37),
                    ("boards_owner_chip", s.switch_type == 37 && is(owner, 36)),
                    (
                        "boards_owner_sequencer",
                        s.switch_type == 37
                            && owner.is_some_and(|o| o.parts.contains(&"SEQUENCER")),
                    ),
                    (
                        "boards_owner_controlinator",
                        s.switch_type == 37
                            && owner.is_some_and(|o| o.parts.contains(&"CONTROLINATOR")),
                    ),
                    ("on_board", is(owner, 37)),
                ] {
                    *self.logic.entry(name).or_default() += usize::from(holds);
                }
                for o in &s.outputs {
                    let Some(o) = o else {
                        self.outputs_null += 1;
                        continue;
                    };
                    *self.ternary.entry(o.signal.ternary).or_default() += 1;
                    self.activation_not_zero += usize::from(o.signal.activation != 0.);
                    self.with_player += usize::from(o.signal.player != -1);
                    self.targets += o.targets.len();
                    self.dangling_targets += o.targets.iter().filter(|t| t.0.is_none()).count();
                }
            }
            let wired = t.inputs.iter().flatten().count();
            self.things_with_inputs += usize::from(wired > 0);
            self.wired_ports += wired;
            self.most_ports = self.most_ports.max(t.inputs.len());
        }
        for inst in found {
            let class = inst.class;
            if world.things[inst.thing as usize]
                .inputs
                .iter()
                .any(Option::is_some)
            {
                self.scripted_with_input += 1;
                let name = class.map_or("(class not loaded)", |c| vm.class_name(c));
                *self.scripted_classes.entry(name.to_owned()).or_default() += 1;
            }
            let guids = class.map_or(vec![Some(inst.guid)], |c| {
                vm.class_chain(c)
                    .into_iter()
                    .map(|c| vm.class_guid(c))
                    .collect()
            });
            self.score_boards +=
                usize::from(guids.contains(&Some(lbp_script_vm::natives::SCORE_BOARD_SCRIPT)));
        }
    }

    fn json(&self) -> serde_json::Value {
        let mut classes: Vec<_> = self.scripted_classes.iter().collect();
        classes.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        serde_json::json!({
            "unit": "Thing with a PSwitch part; switch output; output target; input port",
            "count": self.count,
            "by_type": self.by_type,
            "inverted": self.inverted,
            "outputs": self.outputs,
            "outputs_null": self.outputs_null,
            "outputs_by_stored_ternary": self.ternary,
            "outputs_activation_not_zero": self.activation_not_zero,
            "outputs_with_player": self.with_player,
            "targets": self.targets,
            "dangling_targets": self.dangling_targets,
            "things_with_input_table": self.things_with_inputs,
            "wired_ports": self.wired_ports,
            "most_ports_on_a_thing": self.most_ports,
            "port_collisions": self.collisions,
            "ports_dropped": self.ports_dropped,
            "scripted_things_with_wired_port": self.scripted_with_input,
            "scripted_classes_with_wired_port": classes,
            "score_board_things": self.score_boards,
            "gates_by_type_mode_inverted": self.gates,
            "logic_settings": self.logic,
            "timers_by_behaviour": self.timer_behaviours,
            "battery_and_toggle_outputs_stored_against_manual": self.manual_outputs,
            "sensors": self.sensors,
        })
    }
}

fn add_level(vm: &Vm, world: &LevelWorld, t: &mut Totals) -> serde_json::Value {
    t.levels += 1;
    t.things += world.things.len();
    t.unconverted += world.unconverted;
    let found = instances(vm, world);
    t.triggers.add(vm, world, &found);
    t.switches.add(vm, world, &found);
    t.pworld.add(world);
    let mut level_classes = BTreeMap::<String, usize>::new();
    let mut unresolved = BTreeSet::new();
    for inst in &found {
        t.scripted += 1;
        let state = &world.things[inst.thing as usize];
        t.with_fields += usize::from(state.members.is_some());
        match inst.class {
            Some(c) => {
                t.resolved += 1;
                t.classes.insert(c);
                let name = vm.class_name(c).to_owned();
                *level_classes.entry(name.clone()).or_default() += 1;
                *t.by_class.entry(name).or_default() += 1;
                if let Some(m) = &state.members {
                    let layout: BTreeSet<String> = vm.field_names(c).into_iter().collect();
                    for f in m.keys() {
                        if layout.contains(f) {
                            t.fields_matched += 1;
                        } else {
                            t.fields_missing_in_class += 1;
                        }
                    }
                }
            }
            None => {
                *t.unresolved.entry(inst.guid).or_default() += 1;
                unresolved.insert(format!("{:x}", inst.guid));
            }
        }
    }
    serde_json::json!({
        "things": world.things.len(),
        "scripted": found.len(),
        "classes": level_classes,
        "unresolved_guids": unresolved,
        "game_mode": world.pworld.game_mode,
        "frame": world.pworld.frame,
        "is_paused": world.pworld.is_paused,
    })
}

/// What one `--play` run is given.
struct PlayOptions {
    ticks: u64,
    /// The profile's word of progression flags at the start (`--flags`).
    flags: u32,
    in_use_from: Option<u64>,
    strict: bool,
    visit: bool,
    pulse: bool,
    /// `--our-pod`: the level is started as the player's own Pod whatever is in it.
    our_pod: bool,
    /// `--enter-pad-zone`: the tick from which the Sackboy stands on the pod computer's
    /// trigger.
    pad_zone_from: Option<u64>,
    /// `--press`: (tick, button word of player 0's pad on that tick).
    presses: Vec<(u64, u32)>,
    /// D325 `--menu`: (tick, key) for the drawn Gooey menu.
    menu_keys: Vec<(u64, String)>,
    /// `--player-colour`: the players' stored colour, 0xAARRGGBB.
    colour: Option<u32>,
}

/// The start state of a level world for a play run. A level is started as our Pod when
/// the caller says so, or when it has a pod Thing: the tool's stand-in for the game's
/// rule, which is about the slot the level was entered from and not about its content.
fn start_world(world: &mut LevelWorld, opts: &PlayOptions) {
    world.progression_flags = opts.flags;
    world.pod_computer_in_use = opts.in_use_from == Some(0);
    world.our_pod = opts.our_pod || world.pod_thing(true).is_some();
    if let Some(colour) = opts.colour {
        for r in &mut world.player_records {
            r.colour = colour;
        }
    }
}

/// The input of the tick about to run (`tick` counts from 1): the Sackboy is put on the
/// pod computer's trigger on the tick `--enter-pad-zone` names, in a level that has such
/// a trigger, and player 0's pad gets the button word `--press` gives for the tick.
fn feed_input(world: &mut LevelWorld, tick: u64, opts: &PlayOptions, avatar: &mut Option<u32>) {
    if opts.pad_zone_from == Some(tick) && avatar.is_none() {
        if let Some(s) = world.pod_trigger_spot() {
            *avatar = Some(world.add_player(s, (s[2] - 1., s[2] + 1.)));
        }
    }
    if !opts.presses.is_empty() {
        let down = opts.presses.iter().filter(|(t, _)| *t == tick);
        world.set_pad_buttons(0, down.fold(0, |word, (_, mask)| word | mask));
    }
    // D325: the drawn menu's keys before the tick (up, down, left, right move the focus;
    // choose and back reach the next build).
    for (_, key) in opts.menu_keys.iter().filter(|(t, _)| *t == tick) {
        let frame = world.pworld.frame;
        let g = &mut world.gooey;
        match key.as_str() {
            "up" => drop(g.move_focus([0., -1.], None, frame)),
            "down" => drop(g.move_focus([0., 1.], None, frame)),
            "left" => drop(g.move_focus([-1., 0.], None, frame)),
            "right" => drop(g.move_focus([1., 0.], None, frame)),
            "choose" => g.input.choose = true,
            "back" => g.input.back = true,
            _ => {}
        }
    }
}

/// D325 `--menu=TICK:KEY,..`: KEY is up, down, left, right, choose or back.
fn parse_menu_keys(text: &str) -> Result<Vec<(u64, String)>, String> {
    text.split(',')
        .map(|item| {
            let (tick, key) = item
                .trim()
                .split_once(':')
                .ok_or_else(|| format!("{item:?}: expected TICK:KEY"))?;
            let tick = tick.parse::<u64>().map_err(|e| format!("{item:?}: {e}"))?;
            match key {
                "up" | "down" | "left" | "right" | "choose" | "back" => Ok((tick, key.to_owned())),
                _ => Err(format!("{item:?}: unknown key")),
            }
        })
        .collect()
}

/// `--press=TICK:MASK,..`: a mask is decimal, or hexadecimal after `0x`.
fn parse_presses(text: &str) -> Result<Vec<(u64, u32)>, String> {
    text.split(',')
        .map(|item| {
            let (tick, mask) = item
                .trim()
                .split_once(':')
                .ok_or_else(|| format!("{item:?}: expected TICK:MASK"))?;
            let tick = tick.parse::<u64>().map_err(|e| format!("{item:?}: {e}"))?;
            Ok((tick, lbp_runtime::parse_progression_flags(mask)?))
        })
        .collect()
}

/// The switch pass counts. Unit: visited output target; `g<N>` = script resource GUID.
fn reactions_json(r: &ReactionReport) -> serde_json::Value {
    let by_guid = |m: &BTreeMap<u32, usize>| {
        m.iter()
            .map(|(g, n)| (format!("g{g}"), *n))
            .collect::<BTreeMap<_, _>>()
    };
    let total = |m: &BTreeMap<u32, usize>| m.values().sum::<usize>();
    serde_json::json!({
        "targets_visited": r.targets_visited,
        "targets_visited_on": r.targets_on,
        "calls_b": total(&r.calls_b),
        "calls_bf": total(&r.calls_bf),
        "calls_b_by_guid": by_guid(&r.calls_b),
        "calls_bf_by_guid": by_guid(&r.calls_bf),
        "targets_without_script": r.without_script,
        "targets_not_whitelisted": total(&r.not_whitelisted),
        "not_whitelisted_by_guid": by_guid(&r.not_whitelisted),
        "listed_but_switch_thing": total(&r.shadowed_by_switch),
        "listed_but_switch_thing_by_guid": by_guid(&r.shadowed_by_switch),
        "listed_without_function": total(&r.no_function),
        "listed_without_function_by_guid": by_guid(&r.no_function),
        "listed_but_stopped": total(&r.to_stopped),
        "listed_but_stopped_by_guid": by_guid(&r.to_stopped),
    })
}

/// Switch outputs whose signal differs from the stored one, by switch type; with `active`
/// only those of Things that are not deactivated.
fn outputs_changed(world: &LevelWorld, active: bool) -> BTreeMap<i32, usize> {
    let mut changed = BTreeMap::<i32, usize>::new();
    for t in world.things.iter().filter(|t| !(active && t.deactivated)) {
        let Some(s) = &t.switch else {
            continue;
        };
        let n = s.outputs.iter().flatten().filter(|o| o.signal != o.stored);
        match n.count() {
            0 => {}
            n => *changed.entry(s.switch_type).or_default() += n,
        }
    }
    changed
}

/// What a play run did to the message list (`LevelWorld::messages`). Units: one call of a
/// messaging native (request); one message that entered the list, new or written over
/// another (the text, actor and anchor counts); one message (the rest).
#[derive(Default, Clone)]
struct MessageSums {
    /// By type: requests, opened as a new message, written over another, dropped as a
    /// duplicate.
    by_type: BTreeMap<&'static str, [u64; 4]>,
    /// Bubbles and subtitles that entered the list, by text: a string of a loaded
    /// translation table, another string, empty.
    text: [u64; 3],
    /// Entered with an actor name argument; with a name to show.
    actors: [u64; 2],
    /// Entered with an anchor Thing; without one.
    anchors: [u64; 2],
    pushed_out: u64,
    closed: u64,
    events_without_effect: u64,
    events_elsewhere: u64,
    open_at_end: u64,
    /// Entered messages past the log's limit (not in the text, actor and anchor counts).
    not_logged: u64,
}

fn message_type_name(kind: MessageKind, style: BubbleStyle) -> &'static str {
    match (kind, style) {
        (MessageKind::Bubble, BubbleStyle::Speech) => "bubble",
        (MessageKind::Bubble, BubbleStyle::Caption) => "caption",
        (MessageKind::Bubble, BubbleStyle::Think) => "think",
        (MessageKind::BubbleNoPrompt, BubbleStyle::Speech) => "bubble_no_prompt",
        (MessageKind::BubbleNoPrompt, BubbleStyle::Caption) => "caption_no_prompt",
        (MessageKind::BubbleNoPrompt, BubbleStyle::Think) => "think_no_prompt",
        (MessageKind::Subtitles, _) => "subtitles",
        (MessageKind::LevelLinkPrompt, _) => "level_link_prompt",
        (MessageKind::WormHolePrompt, _) => "wormhole_prompt",
    }
}

impl MessageSums {
    fn of(world: &LevelWorld) -> Self {
        let m = &world.messages;
        let mut s = MessageSums {
            pushed_out: m.pushed_out,
            closed: m.closed,
            events_without_effect: m.events_without_effect,
            events_elsewhere: m.events_elsewhere,
            open_at_end: m.open.len() as u64,
            ..Default::default()
        };
        for ((kind, style), c) in &m.by_type {
            s.by_type.insert(
                message_type_name(*kind, *style),
                [c.requested, c.opened, c.replaced, c.duplicate],
            );
        }
        let sum = m.totals();
        s.not_logged = (sum.opened + sum.replaced).saturating_sub(m.log.len() as u64);
        if m.log.is_empty() {
            return s;
        }
        let table: std::collections::HashSet<&str> = world
            .translations
            .iter()
            .flat_map(|t| t.entries.iter().filter_map(|e| e.text.as_deref()))
            .collect();
        for e in &m.log {
            let prompt = matches!(
                e.kind,
                MessageKind::LevelLinkPrompt | MessageKind::WormHolePrompt
            );
            if !prompt {
                let i = match e.text.as_str() {
                    "" => 2,
                    t if table.contains(t) => 0,
                    _ => 1,
                };
                s.text[i] += 1;
            }
            s.actors[0] += u64::from(e.actor_tag.is_some());
            s.actors[1] += u64::from(e.actor.is_some());
            s.anchors[usize::from(e.anchor.is_none())] += 1;
        }
        s
    }

    fn entered(&self) -> u64 {
        self.by_type.values().map(|c| c[1] + c[2]).sum()
    }

    fn add(&mut self, o: &MessageSums) {
        for (name, c) in &o.by_type {
            let sum = self.by_type.entry(name).or_default();
            for i in 0..4 {
                sum[i] += c[i];
            }
        }
        for i in 0..3 {
            self.text[i] += o.text[i];
        }
        for i in 0..2 {
            self.actors[i] += o.actors[i];
            self.anchors[i] += o.anchors[i];
        }
        self.pushed_out += o.pushed_out;
        self.closed += o.closed;
        self.events_without_effect += o.events_without_effect;
        self.events_elsewhere += o.events_elsewhere;
        self.open_at_end += o.open_at_end;
        self.not_logged += o.not_logged;
    }

    fn json(&self) -> serde_json::Value {
        let by_type: BTreeMap<_, _> = self
            .by_type
            .iter()
            .map(|(name, c)| {
                let row = serde_json::json!({
                    "requested": c[0], "opened": c[1], "written_over": c[2], "duplicate": c[3],
                });
                (*name, row)
            })
            .collect();
        serde_json::json!({
            "unit": "request (a messaging native call); message entered (opened or written over another)",
            "by_type": by_type,
            "requested": self.by_type.values().map(|c| c[0]).sum::<u64>(),
            "entered": self.entered(),
            "text_in_a_loaded_table": self.text[0],
            "text_other": self.text[1],
            "text_empty": self.text[2],
            "actor_names_passed": self.actors[0],
            "actor_names_shown": self.actors[1],
            "anchored": self.anchors[0],
            "without_anchor": self.anchors[1],
            "entered_not_logged": self.not_logged,
            "pushed_out_when_full": self.pushed_out,
            "closed": self.closed,
            "events_without_effect": self.events_without_effect,
            "events_for_another_manager": self.events_elsewhere,
            "open_at_end": self.open_at_end,
        })
    }
}

/// The global settings over a play run. Units: tick; level; `ApplyGlobalSettings__` call.
/// "Unlike" compares the setting in force (`WorldState::settings_in_force`) with the one
/// in force when the level was read, by setting type. For one level the level counts are
/// 0 or 1.
#[derive(Default, Clone)]
struct SettingsSums {
    ticks: u64,
    /// Ticks after which the type was unlike; any type.
    ticks_unlike: [u64; GLOBAL_SETTINGS],
    ticks_any_unlike: u64,
    /// Levels with such a tick; any type.
    levels_unlike: [usize; GLOBAL_SETTINGS],
    levels_any_unlike: usize,
    /// Levels where the type was unlike after the last tick; any type.
    levels_unlike_at_end: [usize; GLOBAL_SETTINGS],
    levels_any_unlike_at_end: usize,
    /// Ticks after which lighting or darkness, the two values of the lighting group the
    /// viewer shows, was unlike; levels with such a tick.
    ticks_light_unlike: u64,
    levels_light_unlike: usize,
    /// The global-settings pass of every tick.
    applied: GlobalSettingsReport,
    /// Levels with a call, by type; any type.
    levels_with_call: [usize; GLOBAL_SETTINGS],
    levels_with_any_call: usize,
    /// Levels whose stored current lighting values (PWorld `lighting` to `darkness`) are
    /// not the stored from / target / blend factor taken together; of those, the levels
    /// where lighting or darkness is one of the values.
    levels_stored_lighting_unlike_blend: usize,
    levels_stored_light_unlike_blend: usize,
}

impl SettingsSums {
    /// After one tick.
    fn note(&mut self, world: &LevelWorld) {
        let unlike = world.settings_unlike_load();
        self.ticks += 1;
        for (n, u) in self.ticks_unlike.iter_mut().zip(unlike) {
            *n += u64::from(u);
        }
        self.ticks_any_unlike += u64::from(unlike.contains(&true));
        let light = |s: &lbp_runtime::SettingsInForce| s.light().map(f32::to_bits);
        let now = world.pworld.settings_in_force();
        self.ticks_light_unlike += u64::from(light(&now) != light(&world.settings_at_load));
    }

    /// After the last tick, with the summed pass report.
    fn finish(&mut self, world: &LevelWorld, applied: &GlobalSettingsReport) {
        let unlike = world.settings_unlike_load();
        self.levels_unlike = self.ticks_unlike.map(|n| usize::from(n > 0));
        self.levels_unlike_at_end = unlike.map(usize::from);
        self.levels_with_call = applied.calls.map(|n| usize::from(n > 0));
        self.levels_any_unlike = usize::from(self.ticks_any_unlike > 0);
        self.levels_light_unlike = usize::from(self.ticks_light_unlike > 0);
        self.levels_any_unlike_at_end = usize::from(unlike.contains(&true));
        self.levels_with_any_call = usize::from(applied.total_calls() > 0);
        self.applied = applied.clone();
        let (stored, start) = (
            world.pworld.lighting_stored,
            world.settings_at_load.lighting,
        );
        let differs = |i: usize| stored[i].to_bits() != start[i].to_bits();
        self.levels_stored_lighting_unlike_blend = usize::from((0..5).any(differs));
        self.levels_stored_light_unlike_blend = usize::from(differs(0) || differs(4));
    }

    fn add(&mut self, o: &SettingsSums) {
        self.ticks += o.ticks;
        self.ticks_any_unlike += o.ticks_any_unlike;
        self.levels_any_unlike += o.levels_any_unlike;
        self.levels_any_unlike_at_end += o.levels_any_unlike_at_end;
        self.levels_with_any_call += o.levels_with_any_call;
        self.ticks_light_unlike += o.ticks_light_unlike;
        self.levels_light_unlike += o.levels_light_unlike;
        for kind in 0..GLOBAL_SETTINGS {
            self.ticks_unlike[kind] += o.ticks_unlike[kind];
            self.levels_unlike[kind] += o.levels_unlike[kind];
            self.levels_unlike_at_end[kind] += o.levels_unlike_at_end[kind];
            self.levels_with_call[kind] += o.levels_with_call[kind];
        }
        self.applied.absorb(o.applied.clone());
        self.levels_stored_lighting_unlike_blend += o.levels_stored_lighting_unlike_blend;
        self.levels_stored_light_unlike_blend += o.levels_stored_light_unlike_blend;
    }

    fn json(&self) -> serde_json::Value {
        let by_type: BTreeMap<_, _> = (0..GLOBAL_SETTINGS)
            .map(|kind| {
                let row = serde_json::json!({
                    "apply_calls": self.applied.calls[kind],
                    "levels_with_an_apply_call": self.levels_with_call[kind],
                    "ticks_unlike_load": self.ticks_unlike[kind],
                    "levels_unlike_load": self.levels_unlike[kind],
                    "levels_unlike_load_at_end": self.levels_unlike_at_end[kind],
                });
                (GLOBAL_SETTING_NAMES[kind], row)
            })
            .collect();
        serde_json::json!({
            "unit": "tick; level; ApplyGlobalSettings__ call; unlike = the setting in force is not the one in force when the level was read (audio is not kept, so never unlike)",
            "ticks": self.ticks,
            "by_type": by_type,
            "apply_calls": self.applied.total_calls(),
            "levels_with_an_apply_call": self.levels_with_any_call,
            "slot_turns_without_the_function": self.applied.no_function,
            "slot_turns_of_a_stopped_thing": self.applied.to_stopped,
            "slots_cleared": self.applied.slots_cleared,
            "ticks_any_unlike_load": self.ticks_any_unlike,
            "levels_any_unlike_load": self.levels_any_unlike,
            "levels_any_unlike_load_at_end": self.levels_any_unlike_at_end,
            "ticks_lighting_or_darkness_unlike_load": self.ticks_light_unlike,
            "levels_lighting_or_darkness_unlike_load": self.levels_light_unlike,
            "levels_stored_current_lighting_group_unlike_stored_blend": self.levels_stored_lighting_unlike_blend,
            "levels_stored_current_lighting_or_darkness_unlike_stored_blend": self.levels_stored_light_unlike_blend,
        })
    }
}

/// The Things whose display values (render-mesh visibility, colours, world matrix) are
/// unlike the level file's after the last tick, and the levels that have such a Thing,
/// by the rows of `DisplaySummary`; and the changes marked on the way, by kind. For one
/// level the level counts are 0 or 1.
#[derive(Default, Clone)]
struct DisplaySums {
    things: lbp_runtime::DisplaySummary,
    levels: [usize; 13],
    /// Changes of visibility, colour, world matrix and mesh marked for a display (a
    /// Thing changed on two ticks counts twice).
    marks: [u64; 4],
    /// Things with a PRenderMesh by the visibility byte the level file stores; those the
    /// file stores as not visible in play mode, and their levels; and the Things hidden
    /// after the last tick.
    meshes_by_stored_visibility: BTreeMap<u8, usize>,
    hidden_in_file: (usize, usize),
    hidden_at_end: (usize, usize),
    /// Render-mesh tints set by a script after the last tick, by whether their red,
    /// green and blue are white and whether their alpha is full: (Things, levels).
    tints: BTreeMap<&'static str, (usize, usize)>,
}

impl DisplaySums {
    fn of(world: &LevelWorld) -> Self {
        let things = world.display_summary();
        let mut by_byte = BTreeMap::<u8, usize>::new();
        let (mut in_file, mut at_end) = (0usize, 0usize);
        let mut tints = BTreeMap::<&'static str, (usize, usize)>::new();
        for thing in 0..world.things.len() as u32 {
            let (Some(was), Some(now)) = (world.display_at_load(thing), world.thing_display(thing))
            else {
                continue;
            };
            if let Some(byte) = was.mesh_visibility {
                *by_byte.entry(byte).or_default() += 1;
            }
            in_file += usize::from(was.mesh_hidden());
            at_end += usize::from(now.mesh_hidden());
            if let Some(tint) = now.mesh_tint {
                let packed = lbp_runtime::pack_colour(tint);
                let kind = match (packed & 0xff_ffff == 0xff_ffff, packed >> 24 == 0xff) {
                    (true, true) => "white",
                    (true, false) => "white_with_an_alpha",
                    (false, true) => "coloured",
                    (false, false) => "coloured_with_an_alpha",
                };
                *tints.entry(kind).or_default() = (tints.get(kind).map_or(0, |t| t.0) + 1, 1);
            }
        }
        DisplaySums {
            things,
            levels: things.rows().map(|(_, n)| usize::from(n > 0)),
            marks: world.display.marks,
            meshes_by_stored_visibility: by_byte,
            hidden_in_file: (in_file, usize::from(in_file > 0)),
            hidden_at_end: (at_end, usize::from(at_end > 0)),
            tints,
        }
    }

    fn add(&mut self, o: &DisplaySums) {
        self.things.absorb(&o.things);
        for (sum, n) in self.levels.iter_mut().zip(o.levels) {
            *sum += n;
        }
        for (sum, n) in self.marks.iter_mut().zip(o.marks) {
            *sum += n;
        }
        for (byte, n) in &o.meshes_by_stored_visibility {
            *self.meshes_by_stored_visibility.entry(*byte).or_default() += n;
        }
        for (kind, n) in &o.tints {
            let sum = self.tints.entry(kind).or_default();
            sum.0 += n.0;
            sum.1 += n.1;
        }
        for (sum, n) in [
            (&mut self.hidden_in_file, o.hidden_in_file),
            (&mut self.hidden_at_end, o.hidden_at_end),
        ] {
            sum.0 += n.0;
            sum.1 += n.1;
        }
    }

    fn json(&self) -> serde_json::Value {
        let rows: BTreeMap<_, _> = self
            .things
            .rows()
            .iter()
            .zip(self.levels)
            .map(|((name, things), levels)| {
                (
                    *name,
                    serde_json::json!({"things": things, "levels": levels}),
                )
            })
            .collect();
        serde_json::json!({
            "unit": "Thing; level. Unlike the level file after the last tick: mesh_hidden = render mesh hidden now and not in the file (mesh_shown the reverse), mesh_visibility = visibility byte unlike, colours compared as packed 32-bit colours (a tint never set = white), moved = world matrix unlike in any bit, moved_far = an element off by more than the counting threshold (ThingDisplay::moved_far). Hidden = the play-mode bit of the visibility byte clear",
            "unlike_the_level_file": rows,
            "render_meshes_by_stored_visibility_byte": self.meshes_by_stored_visibility,
            "render_meshes_hidden_in_the_level_file": {
                "things": self.hidden_in_file.0, "levels": self.hidden_in_file.1,
            },
            "render_meshes_hidden_at_end": {
                "things": self.hidden_at_end.0, "levels": self.hidden_at_end.1,
            },
            "render_mesh_tints_set": self.tints.iter().map(|(kind, n)| {
                (*kind, serde_json::json!({"things": n.0, "levels": n.1}))
            }).collect::<BTreeMap<_, _>>(),
            "changes_marked": {
                "visibility": self.marks[0],
                "colour": self.marks[1],
                "transform": self.marks[2],
                "mesh": self.marks[3],
            },
        })
    }
}

/// The override camera and the screen natives over a run (part 20). For one level the
/// level counts are 0 or 1.
#[derive(Default, Clone)]
struct CameraSums {
    /// `Cam.SetOverrideCamera__rr` and `SetOverrideCameraActivation__bf` calls, and the
    /// activation calls that turned the override on.
    set_calls: u64,
    activation_calls: u64,
    activations_on: u64,
    levels_activated: usize,
    /// Ticks after which the override's blend was above the threshold, the levels with
    /// such a tick, and the levels where it was still above it after the last tick.
    ticks_in: u64,
    levels_in: usize,
    levels_in_at_end: usize,
    /// Levels whose blend reached 1.0.
    levels_full: usize,
    matrix_queries: u64,
    levels_matrix: usize,
    /// `HUD.GetScreenSpacePosition__r` calls: all, answered without a view, answered
    /// with a point more than the script's margin outside the screen.
    screen_queries: u64,
    levels_screen: usize,
    without_view: u64,
    off_screen: u64,
    /// `Cam.SetCameraFromToUp__rrrffff` (part 23): calls, the levels with one, the ticks
    /// on which a script wrote the camera, and those of them with the override in too.
    from_to_up_calls: u64,
    levels_from_to_up: usize,
    script_ticks: u64,
    script_ticks_with_override: u64,
    levels_script_and_override: usize,
}

impl CameraSums {
    fn of(world: &LevelWorld) -> Self {
        let (c, o) = (&world.camera, &world.camera.override_camera);
        CameraSums {
            set_calls: o.set_calls,
            activation_calls: o.activation_calls,
            activations_on: o.activations_on,
            levels_activated: usize::from(o.activations_on > 0),
            ticks_in: o.ticks_in,
            levels_in: usize::from(o.ticks_in > 0),
            levels_in_at_end: usize::from(o.is_in()),
            levels_full: usize::from(o.blend_max >= 1.),
            matrix_queries: c.matrix_queries,
            levels_matrix: usize::from(c.matrix_queries > 0),
            screen_queries: c.screen_queries,
            levels_screen: usize::from(c.screen_queries > 0),
            without_view: c.screen_queries_without_view,
            off_screen: c.screen_queries_off_screen,
            from_to_up_calls: c.from_to_up_calls,
            levels_from_to_up: usize::from(c.from_to_up_calls > 0),
            script_ticks: c.script_camera_ticks,
            script_ticks_with_override: c.script_camera_ticks_with_override,
            levels_script_and_override: usize::from(c.script_camera_ticks_with_override > 0),
        }
    }

    fn add(&mut self, o: &CameraSums) {
        self.set_calls += o.set_calls;
        self.activation_calls += o.activation_calls;
        self.activations_on += o.activations_on;
        self.levels_activated += o.levels_activated;
        self.ticks_in += o.ticks_in;
        self.levels_in += o.levels_in;
        self.levels_in_at_end += o.levels_in_at_end;
        self.levels_full += o.levels_full;
        self.matrix_queries += o.matrix_queries;
        self.levels_matrix += o.levels_matrix;
        self.screen_queries += o.screen_queries;
        self.levels_screen += o.levels_screen;
        self.without_view += o.without_view;
        self.off_screen += o.off_screen;
        self.from_to_up_calls += o.from_to_up_calls;
        self.levels_from_to_up += o.levels_from_to_up;
        self.script_ticks += o.script_ticks;
        self.script_ticks_with_override += o.script_ticks_with_override;
        self.levels_script_and_override += o.levels_script_and_override;
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "unit": "native call; tick (one tick_level); level. The override is in while its blend is above 0.0001. No view is written in this run, so a screen query is answered with the middle of the screen",
            "override_set_calls": self.set_calls,
            "override_activation_calls": self.activation_calls,
            "override_activations_on": self.activations_on,
            "levels_with_an_override_activation": self.levels_activated,
            "ticks_with_the_override_in": self.ticks_in,
            "levels_with_the_override_in": self.levels_in,
            "levels_with_the_override_in_at_end": self.levels_in_at_end,
            "levels_with_the_override_fully_in": self.levels_full,
            "camera_matrix_queries": self.matrix_queries,
            "levels_with_a_camera_matrix_query": self.levels_matrix,
            "screen_position_queries": self.screen_queries,
            "levels_with_a_screen_position_query": self.levels_screen,
            "screen_position_queries_without_a_view": self.without_view,
            "screen_position_queries_off_screen": self.off_screen,
            "from_to_up_calls": self.from_to_up_calls,
            "levels_with_a_from_to_up_call": self.levels_from_to_up,
            "ticks_with_a_script_camera": self.script_ticks,
            "ticks_with_a_script_camera_and_the_override_in": self.script_ticks_with_override,
            "levels_with_a_script_camera_and_the_override_in": self.levels_script_and_override,
        })
    }
}

/// Script names and the Pod natives over a run (part 21). For one level the level counts
/// are 0 or 1.
#[derive(Default, Clone)]
struct PodSums {
    /// Things with a PScriptName, those with a name that is not empty, and the levels
    /// with such a Thing.
    named: usize,
    named_not_empty: usize,
    levels_named: usize,
    /// Levels where two Things have the same name that is not empty.
    levels_with_a_name_twice: usize,
    find_calls: u64,
    find_hits: u64,
    levels_find: usize,
    pod_thing_calls: u64,
    pod_thing_hits: u64,
    /// Levels with a pod Thing after the last tick.
    levels_with_a_pod_thing: usize,
    mesh_sets: u64,
    mesh_changes: u64,
    fade_calls: u64,
    host_use_pod_requests: usize,
    /// Part 24: levels started as our Pod, the calls of the pod computer's natives.
    levels_our_pod: usize,
    host_use_pod_refused: u64,
    in_use_changes: usize,
    pad_zone_calls: u64,
    pad_zone_hits: u64,
    action_click_calls: u64,
    action_clicks: u64,
    actions_not_read: u64,
    action_prompts: u64,
    player_colour_calls: u64,
    transition_texture_calls: u64,
    /// Part 25: planet entries that ran, those that answered "arrived", levels with a
    /// planet entry, and the pod computer's per-frame video calls. Set by the caller
    /// from the run's step reports (0 for a level of a session).
    planet_updates: usize,
    planets_arrived: usize,
    levels_with_a_planet: usize,
    video_updates: usize,
}

impl PodSums {
    fn of(world: &LevelWorld) -> Self {
        let (named, named_not_empty) = world.script_names();
        let mut seen = BTreeSet::new();
        let twice = world
            .things
            .iter()
            .filter_map(|t| t.script_name.as_ref().filter(|n| !n.is_empty()))
            .any(|n| !seen.insert(n));
        let p = &world.pod;
        PodSums {
            named,
            named_not_empty,
            levels_named: usize::from(named > 0),
            levels_with_a_name_twice: usize::from(twice),
            find_calls: p.find_thing_calls,
            find_hits: p.find_thing_hits,
            levels_find: usize::from(p.find_thing_calls > 0),
            pod_thing_calls: p.pod_thing_calls,
            pod_thing_hits: p.pod_thing_hits,
            levels_with_a_pod_thing: usize::from(world.pod_thing(true).is_some()),
            mesh_sets: p.mesh_sets,
            mesh_changes: p.mesh_changes,
            fade_calls: p.fade_calls,
            host_use_pod_requests: p.host_use_pod_requests.len(),
            levels_our_pod: usize::from(world.our_pod),
            host_use_pod_refused: p.host_use_pod_refused,
            in_use_changes: p.in_use_changes.len(),
            pad_zone_calls: p.pad_zone_calls,
            pad_zone_hits: p.pad_zone_hits,
            action_click_calls: p.action_click_calls,
            action_clicks: p.action_clicks,
            actions_not_read: p.actions_not_read.values().sum(),
            action_prompts: p.action_prompts.values().sum(),
            player_colour_calls: p.player_colour_calls,
            transition_texture_calls: p.transition_texture_calls,
            ..Default::default()
        }
    }

    fn add(&mut self, o: &PodSums) {
        self.named += o.named;
        self.named_not_empty += o.named_not_empty;
        self.levels_named += o.levels_named;
        self.levels_with_a_name_twice += o.levels_with_a_name_twice;
        self.find_calls += o.find_calls;
        self.find_hits += o.find_hits;
        self.levels_find += o.levels_find;
        self.pod_thing_calls += o.pod_thing_calls;
        self.pod_thing_hits += o.pod_thing_hits;
        self.levels_with_a_pod_thing += o.levels_with_a_pod_thing;
        self.mesh_sets += o.mesh_sets;
        self.mesh_changes += o.mesh_changes;
        self.fade_calls += o.fade_calls;
        self.host_use_pod_requests += o.host_use_pod_requests;
        self.levels_our_pod += o.levels_our_pod;
        self.host_use_pod_refused += o.host_use_pod_refused;
        self.in_use_changes += o.in_use_changes;
        self.pad_zone_calls += o.pad_zone_calls;
        self.pad_zone_hits += o.pad_zone_hits;
        self.action_click_calls += o.action_click_calls;
        self.action_clicks += o.action_clicks;
        self.actions_not_read += o.actions_not_read;
        self.action_prompts += o.action_prompts;
        self.player_colour_calls += o.player_colour_calls;
        self.transition_texture_calls += o.transition_texture_calls;
        self.planet_updates += o.planet_updates;
        self.planets_arrived += o.planets_arrived;
        self.levels_with_a_planet += o.levels_with_a_planet;
        self.video_updates += o.video_updates;
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "unit": "Thing with a PScriptName part; native call; level. A hit is a call that returned a Thing",
            "things_with_a_script_name": self.named,
            "things_with_a_script_name_not_empty": self.named_not_empty,
            "levels_with_a_script_name": self.levels_named,
            "levels_with_a_name_on_two_things": self.levels_with_a_name_twice,
            "find_thing_calls": self.find_calls,
            "find_thing_hits": self.find_hits,
            "levels_with_a_find_thing_call": self.levels_find,
            "get_pod_thing_calls": self.pod_thing_calls,
            "get_pod_thing_hits": self.pod_thing_hits,
            "levels_with_a_pod_thing": self.levels_with_a_pod_thing,
            "set_render_mesh_mesh_calls_on_a_render_mesh": self.mesh_sets,
            "render_mesh_mesh_changes": self.mesh_changes,
            "set_fade_pod_decorations_calls": self.fade_calls,
            "host_use_pod_requests": self.host_use_pod_requests,
            "levels_started_as_our_pod": self.levels_our_pod,
            "host_use_pod_requests_refused": self.host_use_pod_refused,
            "pod_computer_in_use_changes": self.in_use_changes,
            "get_is_in_pad_zone_calls": self.pad_zone_calls,
            "get_is_in_pad_zone_true": self.pad_zone_hits,
            "is_just_clicked_by_id_calls": self.action_click_calls,
            "is_just_clicked_by_id_true": self.action_clicks,
            "is_just_clicked_by_id_actions_not_read": self.actions_not_read,
            "is_just_clicked_by_id_prompts": self.action_prompts,
            "get_player_colour_calls_on_a_yellowhead": self.player_colour_calls,
            "set_pod_transition_texture_calls": self.transition_texture_calls,
            "planet_update_calls": self.planet_updates,
            "planet_update_calls_answering_arrived": self.planets_arrived,
            "levels_with_a_planet_update": self.levels_with_a_planet,
            "pod_computer_update_video_calls": self.video_updates,
        })
    }
}

/// Video and voice-over requests, the other audio calls of part 22 and the progression
/// flags over a run. For one level the level counts are 0 or 1.
#[derive(Default, Clone)]
struct MediaSums {
    video_requests: u64,
    levels_video: usize,
    video_stops: u64,
    voice_over_requests: u64,
    levels_voice_over: usize,
    voice_over_stops: u64,
    mix_overlays_set: u64,
    mix_overlays_removed: u64,
    levels_mix_overlay: usize,
    /// Levels whose scripts set a progression flag that was clear.
    levels_flags_changed: usize,
    /// Script threads started and run to their end.
    threads_started: usize,
    threads_done: usize,
    levels_thread: usize,
}

impl MediaSums {
    fn of(world: &LevelWorld, log: &TickLog) -> Self {
        let m = &world.media;
        let (set, removed) = (
            m.mix_overlays_set.values().sum::<u64>(),
            m.mix_overlays_removed.values().sum::<u64>(),
        );
        MediaSums {
            video_requests: m.video_requests(),
            levels_video: usize::from(m.video_requests() > 0),
            video_stops: m.video_stops,
            voice_over_requests: m.voice_over_requests(),
            levels_voice_over: usize::from(m.voice_over_requests() > 0),
            voice_over_stops: m.voice_over_stops,
            mix_overlays_set: set,
            mix_overlays_removed: removed,
            levels_mix_overlay: usize::from(set + removed > 0),
            levels_flags_changed: usize::from(!log.flag_changes.is_empty()),
            threads_started: log.threads_started,
            threads_done: log.threads_done,
            levels_thread: usize::from(log.threads_started > 0),
        }
    }

    fn add(&mut self, o: &MediaSums) {
        self.video_requests += o.video_requests;
        self.levels_video += o.levels_video;
        self.video_stops += o.video_stops;
        self.voice_over_requests += o.voice_over_requests;
        self.levels_voice_over += o.levels_voice_over;
        self.voice_over_stops += o.voice_over_stops;
        self.mix_overlays_set += o.mix_overlays_set;
        self.mix_overlays_removed += o.mix_overlays_removed;
        self.levels_mix_overlay += o.levels_mix_overlay;
        self.levels_flags_changed += o.levels_flags_changed;
        self.threads_started += o.threads_started;
        self.threads_done += o.threads_done;
        self.levels_thread += o.levels_thread;
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "unit": "native call; level; script thread. Nothing plays: a request is kept and the script is told that nothing started",
            "video_requests": self.video_requests,
            "levels_with_a_video_request": self.levels_video,
            "video_stops": self.video_stops,
            "voice_over_requests": self.voice_over_requests,
            "levels_with_a_voice_over_request": self.levels_voice_over,
            "voice_over_stops": self.voice_over_stops,
            "mix_overlays_set": self.mix_overlays_set,
            "mix_overlays_removed": self.mix_overlays_removed,
            "levels_with_a_mix_overlay_call": self.levels_mix_overlay,
            "levels_where_a_script_set_a_progression_flag": self.levels_flags_changed,
            "threads_started": self.threads_started,
            "threads_done": self.threads_done,
            "levels_with_a_thread": self.levels_thread,
        })
    }
}

/// Most ticks [`TickLog`] lists of a kind.
const MAX_LOGGED_TICKS: usize = 64;

/// Which ticks of a run started or ended a script thread, and which changed the
/// profile's progression flags. A tick's number is the count of ticks run when it ended
/// (the first tick is 1); the create step is tick 0.
#[derive(Default)]
struct TickLog {
    flags: u32,
    threads_started: usize,
    threads_done: usize,
    started: Vec<u64>,
    done: Vec<u64>,
    /// (tick, the word after it).
    flag_changes: Vec<(u64, u32)>,
}

impl TickLog {
    fn note(&mut self, tick: u64, r: &StepReport, world: &LevelWorld) {
        self.threads_started += r.threads_started;
        self.threads_done += r.threads_done;
        if r.threads_started > 0 && self.started.len() < MAX_LOGGED_TICKS {
            self.started.push(tick);
        }
        if r.threads_done > 0 && self.done.len() < MAX_LOGGED_TICKS {
            self.done.push(tick);
        }
        if world.progression_flags != self.flags {
            self.flags = world.progression_flags;
            if self.flag_changes.len() < MAX_LOGGED_TICKS {
                self.flag_changes.push((tick, self.flags));
            }
        }
    }
}

/// One level played: the report and what `--all` adds up.
struct Played {
    json: serde_json::Value,
    instances: usize,
    stopped: usize,
    visited: usize,
    events: [usize; 3],
    events_run: [usize; 3],
    events_without_script: usize,
    events_to_stopped: usize,
    reactions: ReactionReport,
    fault_kinds: BTreeMap<String, usize>,
    thread_faults: Vec<String>,
    engine_faults: usize,
    calls: BTreeMap<String, u64>,
    unbound: BTreeMap<String, u64>,
    /// `Game.GotoScoreboard__b` requests recorded.
    scoreboard_requests: usize,
    /// Recorded calls kept with their arguments (`LevelWorld::requests`), by name.
    requests: BTreeMap<&'static str, u64>,
    messages: MessageSums,
    settings: SettingsSums,
    display: DisplaySums,
    camera: CameraSums,
    pod: PodSums,
    media: MediaSums,
    /// Global-settings slots that hold a Thing UID at the end.
    global_settings_held: usize,
    /// Switch outputs whose signal at the end differs from the stored one, by switch type.
    outputs_changed: BTreeMap<i32, usize>,
    /// Things deactivated at the end.
    deactivated: usize,
    /// Player sensors the `--visit-triggers` player stood at.
    sensors_visited: usize,
    /// Tag and player sensors by "t<type>", and those whose output 0, while their Thing
    /// was not deactivated, differed from the stored signal on some tick (the pulse
    /// ticks aside): any field, and the ternary.
    sensors: BTreeMap<String, usize>,
    sensors_unlike: BTreeMap<String, usize>,
    sensors_unlike_ternary: BTreeMap<String, usize>,
    /// After the first tick: the switch pass counts, the outputs changed (all, and of
    /// Things that are not deactivated) and the Things deactivated.
    first_tick: Option<FirstTick>,
}

struct FirstTick {
    reactions: ReactionReport,
    changed: BTreeMap<i32, usize>,
    changed_active: BTreeMap<i32, usize>,
    deactivated: usize,
}

/// Things whose world matrix is not the one the level stores: the count, and for the
/// first `MAX_MOVED` the world and local position with the length of the x axis (the
/// scale).
const MAX_MOVED: usize = 24;

fn moved_things(world: &LevelWorld, loaded: &[Option<[f32; 16]>]) -> serde_json::Value {
    let place = |m: &[f32; 16]| {
        let scale = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
        serde_json::json!({"position": m[12..15], "x_axis_length": scale})
    };
    let moved: Vec<_> = world
        .things
        .iter()
        .zip(loaded)
        .enumerate()
        .filter(|(_, (t, m))| t.world_matrix != **m)
        .collect();
    serde_json::json!({
        "count": moved.len(),
        "first": moved.iter().take(MAX_MOVED).map(|(i, (t, _))| serde_json::json!({
            "thing": i,
            "world": t.world_matrix.as_ref().map(place),
            "local": t.local_matrix.as_ref().map(place),
        })).collect::<Vec<_>>(),
    })
}

/// The ticks after which a level played alone reports its script camera (part 23): the
/// first, and the middle and the tick after the end of a zoom of 451 ticks.
const CAMERA_SAMPLE_TICKS: [u64; 3] = [1, 226, 452];

fn script_camera_json(tick: u64, world: &LevelWorld) -> serde_json::Value {
    let camera = world.script_camera().map(|c| {
        serde_json::json!({
            "eye": c.pose.eye,
            "direction": c.pose.direction,
            "up": c.up,
            "fov_y_deg": c.fov_y_deg(),
            "near": c.near,
            "far": c.far,
            "third_float": c.lens_value,
        })
    });
    serde_json::json!({"tick": tick, "camera": camera})
}

/// Part 25: each planet after a tick. The place and the axis lengths are the Thing's
/// world matrix (level units); Spin and Pitch are the members its script keeps (radians).
fn planets_json(tick: u64, level: &LevelScripts, world: &LevelWorld) -> serde_json::Value {
    let planets: Vec<_> = level
        .planets
        .iter()
        .map(|(&thing, frame)| {
            let t = world.things.get(thing as usize);
            let m = t.and_then(|t| t.world_matrix);
            let axis = |m: &[f32; 16], c: usize| {
                (m[c * 4].powi(2) + m[c * 4 + 1].powi(2) + m[c * 4 + 2].powi(2)).sqrt()
            };
            let member = |name: &str| {
                let members = t.and_then(|t| t.members.as_ref());
                match members.and_then(|m| m.get(name)) {
                    Some(lbp_script_vm::Value::Float(f)) => Some(*f),
                    _ => None,
                }
            };
            serde_json::json!({
                "thing": thing,
                "class": t.and_then(|t| t.script_classes.first()),
                "point": frame.point,
                "arrived": frame.arrived,
                "calls": frame.calls,
                "place": m.map(|m| [m[12], m[13], m[14]]),
                "axis_lengths": m.map(|m| [axis(&m, 0), axis(&m, 1), axis(&m, 2)]),
                "spin": member("Spin"),
                "pitch": member("Pitch"),
                "spin_offset": member("SpinOffset"),
            })
        })
        .collect();
    serde_json::json!({"tick": tick, "planets": planets})
}

/// How far from a trigger's Thing the `--visit-triggers` player goes to leave it.
const VISIT_AWAY: f32 = 1.0e7;

/// Runs the `LevelScripts` schedule on one level whose objects are bound to `vm`.
fn play_level(vm: &mut Vm, mut world: LevelWorld, opts: &PlayOptions) -> Played {
    world.add_class_defaults(vm);
    start_world(&mut world, opts);
    let mut avatar = None;
    world.messages.log_limit = lbp_runtime::MAX_REQUESTS;
    let mut log = TickLog {
        flags: opts.flags,
        ..Default::default()
    };
    let loaded: Vec<_> = world.things.iter().map(|t| t.world_matrix).collect();
    let mut host = Bindings::new(world);
    host.default_unbound = !opts.strict;
    let engine_faults = lbp_runtime::add_engine_objects(vm, &mut host, BUDGET_PER_CALL);
    let mut level = LevelScripts::new(vm, &host.world);
    let created = level.create(vm, &mut host, BUDGET_PER_CALL);
    let mut sum = StepReport::default();
    let mut fault_kinds = BTreeMap::<String, usize>::new();
    let mut first_faults = Vec::new();
    let mut ticks_with_faults = 0usize;
    let mut note_faults = |r: &StepReport| {
        for (thing, f) in &r.faults {
            let kind = f.replace(&format!("thing {thing} "), "thing # ");
            *fault_kinds.entry(kind).or_default() += 1;
            if first_faults.len() < MAX_FAULTS {
                first_faults.push(format!("thing {thing}: {f}"));
            }
        }
    };
    note_faults(&created);
    log.note(0, &created, &host.world);
    let camera_json = |c: &lbp_runtime::CameraState| {
        serde_json::json!({
            "from_to_up": c.from_to_up.map(|c| serde_json::json!({
                "from": c.from, "to": c.to, "up": c.up, "params": c.params,
            })),
            "from_to_up_calls": c.from_to_up_calls,
        })
    };
    let mut camera_before_in_use = None;
    let mut script_cameras = Vec::new();
    let mut planets_after = Vec::new();
    let mut first_tick = None;
    let sensors: Vec<usize> = (0..host.world.things.len())
        .filter(|&i| {
            let s = host.world.things[i].switch.as_ref();
            s.is_some_and(|s| matches!(s.switch_type, 3 | 4))
        })
        .collect();
    let mut settings = SettingsSums::default();
    let (mut unlike, mut unlike_ternary) = (BTreeSet::new(), BTreeSet::new());
    let mut note_sensors = |world: &LevelWorld| {
        for &i in &sensors {
            let t = &world.things[i];
            let o = t.switch.as_ref().and_then(|s| s.outputs.first()?.as_ref());
            let Some(o) = o.filter(|_| !t.deactivated) else {
                continue;
            };
            if o.signal != o.stored {
                unlike.insert(i);
            }
            if o.signal.ternary != o.stored.ternary {
                unlike_ternary.insert(i);
            }
        }
    };
    for tick in 0..opts.ticks {
        if opts.in_use_from.is_some_and(|t| t > 0 && t == tick) {
            camera_before_in_use = Some(camera_json(&host.world.camera));
            host.world.pod_computer_in_use = true;
        }
        feed_input(&mut host.world, tick + 1, opts, &mut avatar);
        let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
        if CAMERA_SAMPLE_TICKS.contains(&level.ticks) || tick + 1 == opts.ticks {
            script_cameras.push(script_camera_json(level.ticks, &host.world));
            planets_after.push(planets_json(level.ticks, &level, &host.world));
        }
        log.note(level.ticks, &t, &host.world);
        settings.note(&host.world);
        ticks_with_faults += usize::from(!t.faults.is_empty());
        note_faults(&t);
        note_sensors(&host.world);
        if tick == 0 {
            first_tick = Some(FirstTick {
                reactions: t.reactions.clone(),
                changed: outputs_changed(&host.world, false),
                changed_active: outputs_changed(&host.world, true),
                deactivated: host.world.things.iter().filter(|t| t.deactivated).count(),
            });
        }
        sum.absorb(t);
    }
    // `--visit-triggers`: a player stands on each trigger's Thing for two ticks (Enter,
    // then Inside) and far away for one (Leave), in Thing order.
    let mut visit = StepReport::default();
    let (mut visited, mut sensors_visited) = (0usize, 0usize);
    if opts.visit {
        let spots: Vec<[f32; 3]> = host
            .world
            .things
            .iter()
            .filter_map(|t| {
                let (g, m) = (t.trigger.as_ref()?, t.world_matrix?);
                Some([m[12], m[13], m[14] + g.z_offset])
            })
            .collect();
        let player = match avatar {
            Some(p) => p,
            None => host.world.add_player([0.; 3], (0., 0.)),
        };
        for s in spots {
            for dx in [0., 0., VISIT_AWAY] {
                host.world
                    .move_player(player, [s[0] + dx, s[1], s[2]], (s[2] - 1., s[2] + 1.));
                let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
                log.note(level.ticks, &t, &host.world);
                settings.note(&host.world);
                ticks_with_faults += usize::from(!t.faults.is_empty());
                note_faults(&t);
                note_sensors(&host.world);
                visit.absorb(t);
            }
            visited += 1;
        }
        // Then the player sensors: two ticks in the sensor's range (at its point, or
        // midway between the two edges when it has an inner one), one far away.
        let spots: Vec<[f32; 3]> = host
            .world
            .things
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let s = t.switch.as_ref().filter(|s| s.switch_type == 3)?;
                let proxy = host.world.sensor_proxy(i as u32) as usize;
                let m = host.world.things[proxy].world_matrix?;
                let dx = if s.min_radius > 0. && s.radius > s.min_radius {
                    (s.min_radius + s.radius) / 2.
                } else {
                    0.
                };
                Some([m[12] + dx, m[13], m[14]])
            })
            .collect();
        for s in spots {
            for dx in [0., 0., VISIT_AWAY] {
                host.world
                    .move_player(player, [s[0] + dx, s[1], s[2]], (s[2] - 1., s[2] + 1.));
                let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
                log.note(level.ticks, &t, &host.world);
                settings.note(&host.world);
                ticks_with_faults += usize::from(!t.faults.is_empty());
                note_faults(&t);
                note_sensors(&host.world);
                visit.absorb(t);
            }
            sensors_visited += 1;
        }
    }
    // `--pulse-switches`: one tick with every switch output forced on, one with it forced
    // off, so each wired target sees a rising and a falling edge.
    let mut pulse = StepReport::default();
    if opts.pulse {
        for on in [true, false] {
            host.world.switch_pulse = Some(on);
            let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
            log.note(level.ticks, &t, &host.world);
            settings.note(&host.world);
            ticks_with_faults += usize::from(!t.faults.is_empty());
            note_faults(&t);
            pulse.absorb(t);
        }
        host.world.switch_pulse = None;
    }
    let report = |r: &StepReport| {
        serde_json::json!({
            "done": r.done, "yielded": r.yielded, "resumed": r.resumed,
            "no_function": r.no_function, "no_class": r.no_class, "faults": r.faults.len(),
            "threads_started": r.threads_started, "threads_done": r.threads_done,
            "thread_faults": r.thread_faults.iter().take(MAX_FAULTS).collect::<Vec<_>>(),
            "trigger_events_enter_inside_leave": r.trigger_events,
            "trigger_events_run": r.events_run,
            "trigger_events_without_script": r.events_without_script,
            "trigger_events_to_stopped": r.events_to_stopped,
            "switch_reactions": reactions_json(&r.reactions),
            "global_settings_apply_calls": r.global_settings.total_calls(),
        })
    };
    let mut unbound: Vec<_> = host.unbound.iter().collect();
    unbound.sort_by(|a, b| b.1.cmp(a.1));
    unbound.truncate(40);
    let outputs_changed = outputs_changed(&host.world, false);
    let deactivated = host.world.things.iter().filter(|t| t.deactivated).count();
    let by_type = |things: &mut dyn Iterator<Item = usize>| {
        let mut m = BTreeMap::<String, usize>::new();
        for i in things {
            let ty = host.world.things[i].switch.as_ref().map(|s| s.switch_type);
            *m.entry(format!("t{}", ty.unwrap_or(-1))).or_default() += 1;
        }
        m
    };
    let sensor_count = by_type(&mut sensors.iter().copied());
    let sensors_unlike = by_type(&mut unlike.iter().copied());
    let sensors_unlike_ternary = by_type(&mut unlike_ternary.iter().copied());
    let mut requests = BTreeMap::<&'static str, u64>::new();
    for r in &host.world.requests {
        *requests.entry(r.name).or_default() += 1;
    }
    let messages = MessageSums::of(&host.world);
    let mut json = serde_json::json!({
        "instances": level.instances.len(),
        "ticks": level.ticks,
        "tick_hz": lbp_runtime::TICK_HZ,
        "create": report(&created),
        "ticks_total": report(&sum),
        "triggers_visited": visited,
        "visit_total": report(&visit),
        "switches_pulsed": opts.pulse,
        "pulse_total": report(&pulse),
        "ticks_with_faults": ticks_with_faults,
        "switch_outputs_changed_by_type": outputs_changed,
        "things_deactivated_at_end": deactivated,
        "things_moved": moved_things(&host.world, &loaded),
        "frame_at_end": host.world.pworld.frame,
        "players": host.world.player_records.iter().map(|r| serde_json::json!({
            "number": r.number, "team": r.team, "score": r.score,
            "multiplier_counter": r.multiplier_counter,
            "multiplier_timer": r.multiplier_timer,
            "multiplier_source": r.multiplier_source,
            "multiplier_visible": r.multiplier_visible,
        })).collect::<Vec<_>>(),
        "global_settings_thing_uids": host.world.pworld.global_settings_thing_uids,
        "global_settings_thing_priority": host.world.pworld.global_settings_thing_priority,
        "engine_calls": host.world.engine_calls,
        "scoreboard_requests": host.world.scoreboard_requests,
        "requests_kept": requests,
        "requests_dropped": host.world.requests_dropped,
        "lighting_in_force": host.world.pworld.lighting(),
        "stopped_by_fault": level.stopped.len(),
        "threads_running_at_end": level.threads(),
        "pending_at_end": level.pending(),
        "faults_by_kind": fault_kinds,
        "first_faults": first_faults,
        "unbound_natives": unbound
            .iter()
            .map(|(k, n)| serde_json::json!({"native": k, "calls": n}))
            .collect::<Vec<_>>(),
        "written": host.world.written.iter().take(40).collect::<Vec<_>>(),
        "engine_object_faults": engine_faults,
        "gooeys_created": host.world.gooeys_created,
        "puter_in_use_from": opts.in_use_from,
        "camera_before_in_use": camera_before_in_use,
        "camera": {
            "from_to_up": host.world.camera.from_to_up.map(|c| serde_json::json!({
                "from": c.from, "to": c.to, "up": c.up, "params": c.params,
            })),
            "from_to_up_calls": host.world.camera.from_to_up_calls,
            "script_camera": {
                "unit": "the camera a display draws with after that tick, when a script wrote it with SetCameraFromToUp on the tick (null: none); eye, near and far in level units, direction of length 1, vertical field of view in degrees. Ticks 1, 226, 452 and the last",
                "after_tick": script_cameras,
            },
            "dof_and_fog": host.world.camera.dof_and_fog,
            "motion_blur_clears": host.world.camera.motion_blur_clears,
        },
        "frame_entries": level.instances.iter().filter_map(|i| {
            let e = lbp_runtime::frame_entry(vm, i.class?)?;
            Some(format!("thing {}: {}.{}", i.thing, e.class, e.function))
        }).collect::<Vec<_>>(),
    });
    json["player_sensors_visited"] = serde_json::json!(sensors_visited);
    json["sensors"] = serde_json::json!(sensor_count);
    json["sensors_unlike_stored"] = serde_json::json!(sensors_unlike);
    json["sensors_unlike_stored_ternary"] = serde_json::json!(sensors_unlike_ternary);
    json["messages"] = messages.json();
    sum.absorb(visit);
    sum.absorb(pulse);
    settings.finish(&host.world, &sum.global_settings);
    json["global_settings"] = settings.json();
    let display = DisplaySums::of(&host.world);
    json["display"] = display.json();
    let camera = CameraSums::of(&host.world);
    json["camera"]["override"] = camera.json();
    let o = &host.world.camera.override_camera;
    json["camera"]["override"]["at_end"] = serde_json::json!({
        "position": o.position, "direction": o.direction,
        "target": o.target, "rate": o.rate, "blend": o.blend,
    });
    let mut pod = PodSums::of(&host.world);
    pod.planet_updates = sum.planet_updates;
    pod.planets_arrived = sum.planets_arrived;
    pod.levels_with_a_planet = usize::from(!level.planets.is_empty());
    pod.video_updates = sum.video_updates;
    json["pod"] = pod.json();
    json["pod"]["pod_computer_per_frame_calls"] = serde_json::json!(sum.pod_computer_frames);
    json["planets"] = serde_json::json!({
        "unit": "a planet Thing (class Earth or OrbitingMoon) after that tick: the point it is told to turn toward the camera (a placeholder while the host sets none), arrived, calls of its per-frame function, place and axis lengths of its world matrix in level units, the script's Spin, Pitch and SpinOffset in radians. Ticks 1, 226, 452 and the last",
        "after_tick": planets_after,
    });
    let pod_thing = host.world.pod_thing(true);
    json["pod"]["at_end"] = serde_json::json!({
        "pod_thing": pod_thing,
        "pod_thing_mesh": pod_thing.and_then(|t| host.world.things[t as usize].mesh_guid),
        "pod_thing_shape_colour": pod_thing
            .and_then(|t| host.world.things[t as usize].shape_colour),
        "fade": host.world.pod.fade,
        "planets_background": host.world.planets_background(),
        "host_use_pod_requests": host.world.pod.host_use_pod_requests.iter().map(|r| {
            serde_json::json!({"in_use": r.in_use, "frame": r.frame})
        }).collect::<Vec<_>>(),
        "our_pod": host.world.our_pod,
        "search": search_json(&host.world.pod.search),
        "pod_computer_in_use": host.world.pod_computer_in_use,
        "in_use_changes": host.world.pod.in_use_changes.iter().map(|c| {
            serde_json::json!({"in_use": c.in_use, "frame": c.frame})
        }).collect::<Vec<_>>(),
        "transition_texture": host.world.pod.transition_texture,
        "actions_not_read": host.world.pod.actions_not_read,
        "action_prompts": host.world.pod.action_prompts,
        "sounds_played": host.world.sounds_played,
        "avatar": avatar,
        "named_things": host.world.things.iter().enumerate().filter_map(|(i, t)| {
            let name = String::from_utf8_lossy(t.script_name.as_ref()?).into_owned();
            // The members scripts read or wrote on the Thing's parts.
            let members: BTreeMap<_, _> = host
                .world
                .part_handles
                .iter()
                .filter(|p| p.thing as usize == i)
                .map(|p| (p.part, format!("{:?}", p.members)))
                .collect();
            Some(serde_json::json!({
                "thing": i, "name": name, "parts": t.parts, "part_members": members,
            }))
        }).collect::<Vec<_>>(),
    });
    let media = MediaSums::of(&host.world, &log);
    json["media"] = media.json();
    let m = &host.world.media;
    json["media"]["videos"] = serde_json::json!(m
        .videos
        .iter()
        .map(|v| serde_json::json!({
            "thing": v.thing, "guid": v.guid, "flag": v.flag, "frame": v.frame,
        }))
        .collect::<Vec<_>>());
    json["media"]["voice_overs"] = serde_json::json!(m
        .voice_overs
        .iter()
        .map(|v| { serde_json::json!({"name": v.name, "flags": v.flags, "frame": v.frame}) })
        .collect::<Vec<_>>());
    json["media"]["mix_overlays_set_by_name"] = serde_json::json!(m.mix_overlays_set);
    json["media"]["mix_overlays_removed_by_name"] = serde_json::json!(m.mix_overlays_removed);
    json["progression_flags"] = serde_json::json!({
        "unit": "bit mask of the profile's word; a tick's number is the count of ticks run when it ended, 0 is the create step",
        "at_start": opts.flags,
        "at_end": host.world.progression_flags,
        "changes": log.flag_changes.iter().map(|(tick, word)| {
            serde_json::json!({"tick": tick, "flags": word})
        }).collect::<Vec<_>>(),
    });
    json["thread_ticks"] = serde_json::json!({
        "unit": "tick on which a script thread started or ran to its end (first 64 of each)",
        "started": log.started,
        "done": log.done,
    });
    json["lighting_in_force_at_load"] = serde_json::json!(host.world.settings_at_load.lighting);
    json["lighting_stored_current"] = serde_json::json!(host.world.pworld.lighting_stored);
    Played {
        json,
        instances: level.instances.len(),
        stopped: level.stopped.len(),
        visited,
        events: sum.trigger_events,
        events_run: sum.events_run,
        events_without_script: sum.events_without_script,
        events_to_stopped: sum.events_to_stopped,
        reactions: sum.reactions,
        fault_kinds,
        thread_faults: sum
            .thread_faults
            .iter()
            .chain(&created.thread_faults)
            .cloned()
            .collect(),
        engine_faults: engine_faults.len(),
        scoreboard_requests: host.world.scoreboard_requests.len(),
        requests,
        messages,
        settings,
        display,
        camera,
        pod,
        media,
        global_settings_held: host
            .world
            .pworld
            .global_settings_thing_uids
            .iter()
            .filter(|&&uid| uid != 0)
            .count(),
        calls: host.calls,
        unbound: host.unbound,
        outputs_changed,
        deactivated,
        sensors_visited,
        sensors: sensor_count,
        sensors_unlike,
        sensors_unlike_ternary,
        first_tick,
    }
}

/// One level of a `--with-level` session while it is played.
struct SessionSide<'a> {
    label: &'a str,
    vm: &'a mut Vm,
    host: Bindings<LevelWorld>,
    scripts: LevelScripts,
    avatar: Option<u32>,
    engine_faults: usize,
    faults: usize,
    first_faults: Vec<String>,
    /// What the path was last told: requests made, the change-over value, the pod
    /// Thing's mesh, the sounds asked for, the camera's eye and whether it moved.
    requests: usize,
    fade: Option<f32>,
    mesh: Option<u32>,
    sounds: BTreeMap<String, u64>,
    eye: Option<[f32; 3]>,
    camera_moving: bool,
    in_zone: bool,
    /// Part 46: the step reports' planet entries, those that answered "arrived", the pod
    /// computer's video calls and its per-frame steps, summed over the session; the first
    /// thread faults (the pod computer's calls report theirs there).
    planet_updates: usize,
    planets_arrived: usize,
    video_updates: usize,
    pod_computer_frames: usize,
    thread_faults: Vec<String>,
    /// The planets world's view type and blend the path was last told.
    view: Option<serde_json::Value>,
    /// D319: the pod computer's draw calls; the menu frames built when the path was last
    /// told; the drawn menu's matrix by tick.
    pod_computer_draws: usize,
    menu_builds: u64,
    menu_matrices: Vec<serde_json::Value>,
}

impl<'a> SessionSide<'a> {
    fn new(label: &'a str, vm: &'a mut Vm, mut world: LevelWorld, opts: &PlayOptions) -> Self {
        world.add_class_defaults(vm);
        start_world(&mut world, opts);
        let mut host = Bindings::new(world);
        host.default_unbound = !opts.strict;
        let engine_faults = lbp_runtime::add_engine_objects(vm, &mut host, BUDGET_PER_CALL).len();
        let scripts = LevelScripts::new(vm, &host.world);
        let mesh = host.world.pod_thing(true);
        let mesh = mesh.and_then(|t| host.world.things[t as usize].mesh_guid);
        SessionSide {
            label,
            vm,
            host,
            scripts,
            avatar: None,
            engine_faults,
            faults: 0,
            first_faults: Vec::new(),
            requests: 0,
            fade: None,
            mesh,
            sounds: BTreeMap::new(),
            eye: None,
            camera_moving: false,
            in_zone: false,
            planet_updates: 0,
            planets_arrived: 0,
            video_updates: 0,
            pod_computer_frames: 0,
            pod_computer_draws: 0,
            thread_faults: Vec::new(),
            view: None,
            menu_builds: 0,
            menu_matrices: Vec::new(),
        }
    }

    fn note_faults(&mut self, r: &StepReport) {
        self.faults += r.faults.len();
        for (thing, f) in &r.faults {
            if self.first_faults.len() < MAX_FAULTS {
                self.first_faults.push(format!("thing {thing}: {f}"));
            }
        }
        self.planet_updates += r.planet_updates;
        self.planets_arrived += r.planets_arrived;
        self.video_updates += r.video_updates;
        self.pod_computer_frames += r.pod_computer_frames;
        self.pod_computer_draws += r.pod_computer_draws;
        for f in &r.thread_faults {
            if self.thread_faults.len() < MAX_FAULTS {
                self.thread_faults.push(f.clone());
            }
        }
    }

    /// Part 46: the planets world's `ViewType` and `ViewTransition` (members of the
    /// level's world script), or `None` in a level without them.
    fn planets_view(&self) -> Option<serde_json::Value> {
        let w = &self.host.world;
        let world = w.things.get(w.world? as usize)?;
        let members = world.members.as_ref()?;
        let value = |name: &str| match members.get(name)? {
            lbp_script_vm::Value::Int(i) => Some(serde_json::json!(i)),
            lbp_script_vm::Value::Float(f) => Some(serde_json::json!(f)),
            lbp_script_vm::Value::Bool(b) => Some(serde_json::json!(b)),
            other => Some(serde_json::json!(format!("{other:?}"))),
        };
        let (view_type, transition) = (value("ViewType")?, value("ViewTransition")?);
        Some(serde_json::json!({"view_type": view_type, "transition": transition}))
    }

    /// What changed in this level on the tick that just ended, for the path.
    fn events(&mut self, tick: u64, path: &mut Vec<serde_json::Value>) {
        let level = self.label;
        let view = self.planets_view();
        let view_changed = view != self.view;
        if view_changed {
            self.view.clone_from(&view);
        }
        let mut say = |event: &str, value: serde_json::Value| {
            path.push(serde_json::json!({
                "tick": tick, "level": level, "event": event, "value": value,
            }));
        };
        if view_changed {
            say("planets_view", serde_json::json!(view));
        }
        let w = &self.host.world;
        let in_zone = self.avatar.is_some_and(|a| w.in_pad_zone(a));
        if in_zone != self.in_zone {
            self.in_zone = in_zone;
            say("player_in_pad_zone", serde_json::json!(in_zone));
        }
        for r in w.pod.host_use_pod_requests.iter().skip(self.requests) {
            say("request_host_use_pod", serde_json::json!(r.in_use));
        }
        self.requests = w.pod.host_use_pod_requests.len();
        if w.pod.fade != self.fade {
            self.fade = w.pod.fade;
            // The change-over value the Pod's script hands to the fade each tick; its
            // rest value on the first tick is not an event.
            if self.fade != Some(0.) || tick > 1 {
                say("pod_transition", serde_json::json!(self.fade));
            }
        }
        let mesh = w.pod_thing(true);
        let mesh = mesh.and_then(|t| w.things[t as usize].mesh_guid);
        if mesh != self.mesh {
            self.mesh = mesh;
            say("pod_mesh_guid", serde_json::json!(mesh));
        }
        for (name, n) in &w.sounds_played {
            if self.sounds.get(name) != Some(n) {
                say("sound_requested", serde_json::json!(name));
            }
        }
        self.sounds.clone_from(&w.sounds_played);
        let eye = w.script_camera().map(|c| c.pose.eye);
        let moving = eye != self.eye;
        if moving != self.camera_moving && tick > 1 {
            let event = if moving {
                "camera_moves_from"
            } else {
                "camera_rests_at"
            };
            say(event, serde_json::json!(self.eye));
        }
        (self.eye, self.camera_moving) = (eye, moving);
        // D319: the pod computer's menu frames built, and the drawn menu's matrix on
        // every tick it is set.
        if w.gooey.builds != self.menu_builds {
            if self.menu_builds == 0 {
                say("menu_built", serde_json::json!(w.gooey.frames.len()));
            }
            self.menu_builds = w.gooey.builds;
        }
        if let Some(menu) = w.gooey.drawn_menu() {
            if self.menu_matrices.len() < MAX_MENU_MATRICES {
                self.menu_matrices.push(serde_json::json!({
                    "tick": tick,
                    "frame": menu.matrix_frame,
                    "first_built": menu.first_built,
                    "matrix": menu.matrix,
                }));
            }
        }
    }

    /// D319: the menus the pod computer's states built, the drawn one laid out.
    fn gooey_json(&self) -> serde_json::Value {
        let g = &self.host.world.gooey;
        let drawn = g.drawn_menu().map(|m| {
            let placed: Vec<_> = lbp_runtime::layout(m.frame, m.focus)
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "kind": format!("{:?}", p.kind),
                        "centre": p.centre,
                        "size": p.size,
                        "angle": p.angle,
                    })
                })
                .collect();
            serde_json::json!({
                "root": format!("{:?}", m.frame),
                "focus": m.focus,
                "first_built": m.first_built,
                "matrix_frame": m.matrix_frame,
                "matrix": m.matrix,
                "placed": placed,
            })
        });
        serde_json::json!({
            "frames_built": g.builds,
            "build_frames": g.build_frames,
            "top_level_frames": g.frames.keys().map(|k| format!("{k:?}")).collect::<Vec<_>>(),
            "renders": g.renders,
            "colours_player": g.colours_player,
            "dropped": g.dropped,
            "calls": g.calls,
            "pod_computer_draws": self.pod_computer_draws,
            "drawn_menu": drawn,
            "menu_matrices": self.menu_matrices,
        })
    }

    fn json(&self) -> serde_json::Value {
        let w = &self.host.world;
        let mut unbound: Vec<_> = self.host.unbound.iter().collect();
        unbound.sort_by(|a, b| b.1.cmp(a.1));
        let pod_thing = w.pod_thing(true);
        let mut sums = PodSums::of(w);
        sums.planet_updates = self.planet_updates;
        sums.planets_arrived = self.planets_arrived;
        sums.levels_with_a_planet = usize::from(!self.scripts.planets.is_empty());
        sums.video_updates = self.video_updates;
        let mut pod = sums.json();
        pod["pod_computer_per_frame_calls"] = serde_json::json!(self.pod_computer_frames);
        pod["at_end"] = serde_json::json!({
            "our_pod": w.our_pod,
            "pod_computer_in_use": w.pod_computer_in_use,
            "pod_thing": pod_thing,
            "pod_thing_mesh": pod_thing.and_then(|t| w.things[t as usize].mesh_guid),
            "pod_thing_shape_colour": pod_thing.and_then(|t| w.things[t as usize].shape_colour),
            "fade": w.pod.fade,
            "transition_texture": w.pod.transition_texture,
            "action_prompts": w.pod.action_prompts,
            "actions_not_read": w.pod.actions_not_read,
            "search": search_json(&w.pod.search),
            "avatar": self.avatar,
        });
        serde_json::json!({
            "level": self.label,
            "instances": self.scripts.instances.len(),
            "ticks": self.scripts.ticks,
            "stopped_by_fault": self.scripts.stopped.len(),
            "faults": self.faults,
            "first_faults": self.first_faults,
            "first_thread_faults": self.thread_faults,
            "engine_object_faults": self.engine_faults,
            "threads_running_at_end": self.scripts.threads(),
            "unbound_natives": unbound
                .iter()
                .map(|(k, n)| serde_json::json!({"native": k, "calls": n}))
                .collect::<Vec<_>>(),
            "unbound_native_calls": self.host.unbound.values().sum::<u64>(),
            "pod": pod,
            "sounds_requested": w.sounds_played,
            "progression_flags_at_end": w.progression_flags,
            "video_requests": w.media.video_requests(),
            "voice_over_requests": w.media.voice_over_requests(),
            "script_camera_at_end": script_camera_json(self.scripts.ticks, w),
            "gooey": self.gooey_json(),
            "things_inside_triggers_at_end": w.things.iter()
                .filter_map(|t| t.trigger.as_ref())
                .map(|g| g.inside.len())
                .sum::<usize>(),
        })
    }
}

/// `--with-level`: two levels in one session (part 24). Both are created, then each
/// round feeds the input and ticks the first level, then the second
/// (`lbp_runtime::tick_session`). The report is the path: every tick on which the
/// progression flags, the pad zone, a request to use the pod computer, "in use", the
/// Pod's change-over value, the pod Thing's mesh, a requested sound or the rest of a
/// script camera changed.
fn play_session(
    first: (&str, &mut Vm, LevelWorld),
    second: (&str, &mut Vm, LevelWorld),
    opts: &PlayOptions,
) -> serde_json::Value {
    let mut a = SessionSide::new(first.0, first.1, first.2, opts);
    let mut b = SessionSide::new(second.0, second.1, second.2, opts);
    // One game object: when either level is started as our Pod, the session is.
    let our_pod = a.host.world.our_pod || b.host.world.our_pod;
    a.host.world.our_pod = our_pod;
    a.host.world.hand_session_to(&mut b.host.world);
    let mut path = Vec::new();
    let (mut flags, mut in_use) = (opts.flags, a.host.world.pod_computer_in_use);
    let mut session_events = |tick: u64, world: &LevelWorld, path: &mut Vec<serde_json::Value>| {
        if world.progression_flags != flags {
            flags = world.progression_flags;
            path.push(serde_json::json!({
                "tick": tick, "event": "progression_flags", "value": flags,
            }));
        }
        if world.pod_computer_in_use != in_use {
            in_use = world.pod_computer_in_use;
            path.push(serde_json::json!({
                "tick": tick, "event": "pod_computer_in_use", "value": in_use,
            }));
        }
    };
    let created = a.scripts.create(a.vm, &mut a.host, BUDGET_PER_CALL);
    a.note_faults(&created);
    a.host.world.hand_session_to(&mut b.host.world);
    let created = b.scripts.create(b.vm, &mut b.host, BUDGET_PER_CALL);
    b.note_faults(&created);
    b.host.world.hand_session_to(&mut a.host.world);
    session_events(0, &a.host.world, &mut path);
    for tick in 1..=opts.ticks {
        for side in [&mut a, &mut b] {
            // Part 28, `--puter-in-use=N`: in use when N rounds have run, set on both
            // worlds (a value set on one is written over by the first level's
            // hand-over). 0 is `start_world`'s.
            if opts.in_use_from.is_some_and(|t| t > 0 && t + 1 == tick) {
                side.host.world.pod_computer_in_use = true;
            }
            feed_input(&mut side.host.world, tick, opts, &mut side.avatar);
        }
        let reports = {
            let mut levels = [
                SessionLevel {
                    scripts: &mut a.scripts,
                    vm: &mut *a.vm,
                    host: &mut a.host,
                },
                SessionLevel {
                    scripts: &mut b.scripts,
                    vm: &mut *b.vm,
                    host: &mut b.host,
                },
            ];
            tick_session(&mut levels, BUDGET_PER_CALL)
        };
        a.note_faults(&reports[0]);
        b.note_faults(&reports[1]);
        session_events(tick, &a.host.world, &mut path);
        a.events(tick, &mut path);
        b.events(tick, &mut path);
    }
    serde_json::json!({
        "unit": "tick: the count of rounds run when the event was seen (the first is 1, 0 is the create step); one round ticks the first level, then the second; an event is a value that differs from the round before",
        "ticks": opts.ticks,
        "tick_hz": lbp_runtime::TICK_HZ,
        "flags_at_start": opts.flags,
        "puter_in_use_from": opts.in_use_from,
        "enter_pad_zone_tick": opts.pad_zone_from,
        "presses": opts.presses.iter().map(|(tick, mask)| {
            serde_json::json!({"tick": tick, "buttons": mask})
        }).collect::<Vec<_>>(),
        "player_colour": a.host.world.player_records.first().map(|r| r.colour),
        "path": path,
        "levels": [a.json(), b.json()],
    })
}

/// `--all --play` sums. Pairs are (count, levels).
#[derive(Default)]
struct PlayTotals {
    levels: usize,
    instances: usize,
    stopped: usize,
    levels_with_faults: usize,
    engine_faults: usize,
    visited: usize,
    events: [usize; 3],
    events_run: [usize; 3],
    events_without_script: usize,
    events_to_stopped: usize,
    reactions: ReactionReport,
    calls: BTreeMap<String, (u64, usize)>,
    unbound: BTreeMap<String, (u64, usize)>,
    scoreboard_requests: usize,
    levels_with_scoreboard_requests: usize,
    requests: BTreeMap<&'static str, u64>,
    messages: MessageSums,
    levels_with_message_requests: usize,
    levels_with_messages: usize,
    settings: SettingsSums,
    display: DisplaySums,
    camera: CameraSums,
    pod: PodSums,
    media: MediaSums,
    global_settings_held: usize,
    faults: BTreeMap<String, (u64, usize)>,
    thread_faults: BTreeMap<String, (u64, usize)>,
    outputs_changed: BTreeMap<i32, usize>,
    levels_with_outputs_changed: usize,
    deactivated: usize,
    sensors_visited: usize,
    sensors: BTreeMap<String, usize>,
    sensors_unlike: BTreeMap<String, usize>,
    sensors_unlike_ternary: BTreeMap<String, usize>,
    first_reactions: ReactionReport,
    first_changed: BTreeMap<i32, usize>,
    first_changed_active: BTreeMap<i32, usize>,
    first_deactivated: usize,
}

impl PlayTotals {
    fn add(&mut self, p: &Played) {
        self.levels += 1;
        self.instances += p.instances;
        self.stopped += p.stopped;
        self.engine_faults += p.engine_faults;
        self.visited += p.visited;
        for i in 0..3 {
            self.events[i] += p.events[i];
            self.events_run[i] += p.events_run[i];
        }
        self.events_without_script += p.events_without_script;
        self.events_to_stopped += p.events_to_stopped;
        self.reactions.absorb(p.reactions.clone());
        for (ty, n) in &p.outputs_changed {
            *self.outputs_changed.entry(*ty).or_default() += n;
        }
        self.levels_with_outputs_changed += usize::from(!p.outputs_changed.is_empty());
        self.deactivated += p.deactivated;
        self.sensors_visited += p.sensors_visited;
        for (sum, level) in [
            (&mut self.sensors, &p.sensors),
            (&mut self.sensors_unlike, &p.sensors_unlike),
            (&mut self.sensors_unlike_ternary, &p.sensors_unlike_ternary),
        ] {
            for (ty, n) in level {
                *sum.entry(ty.clone()).or_default() += n;
            }
        }
        self.scoreboard_requests += p.scoreboard_requests;
        self.levels_with_scoreboard_requests += usize::from(p.scoreboard_requests > 0);
        for (name, n) in &p.requests {
            *self.requests.entry(name).or_default() += n;
        }
        self.messages.add(&p.messages);
        self.levels_with_message_requests += usize::from(!p.messages.by_type.is_empty());
        self.levels_with_messages += usize::from(p.messages.entered() > 0);
        self.settings.add(&p.settings);
        self.display.add(&p.display);
        self.camera.add(&p.camera);
        self.pod.add(&p.pod);
        self.media.add(&p.media);
        self.global_settings_held += p.global_settings_held;
        if let Some(f) = &p.first_tick {
            self.first_reactions.absorb(f.reactions.clone());
            for (ty, n) in &f.changed {
                *self.first_changed.entry(*ty).or_default() += n;
            }
            for (ty, n) in &f.changed_active {
                *self.first_changed_active.entry(*ty).or_default() += n;
            }
            self.first_deactivated += f.deactivated;
        }
        self.levels_with_faults +=
            usize::from(!p.fault_kinds.is_empty() || !p.thread_faults.is_empty());
        let bump = |m: &mut BTreeMap<String, (u64, usize)>, k: &str, n: u64| {
            let e = m.entry(k.to_owned()).or_default();
            e.0 += n;
            e.1 += 1;
        };
        for (k, n) in &p.calls {
            bump(&mut self.calls, k, *n);
        }
        for (k, n) in &p.unbound {
            bump(&mut self.unbound, k, *n);
        }
        for (k, n) in &p.fault_kinds {
            bump(&mut self.faults, k, *n as u64);
        }
        let mut threads = BTreeMap::<&str, u64>::new();
        for f in &p.thread_faults {
            *threads.entry(f).or_default() += 1;
        }
        for (k, n) in threads {
            bump(&mut self.thread_faults, k, n);
        }
    }

    fn json(&self, opts: &PlayOptions) -> serde_json::Value {
        let ranked = |m: &BTreeMap<String, (u64, usize)>, key: &str, max: usize| {
            let mut v: Vec<_> = m.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            v.iter()
                .take(max)
                .map(|(k, (n, levels))| serde_json::json!({key: k, "count": n, "levels": levels}))
                .collect::<Vec<_>>()
        };
        let total = |m: &BTreeMap<String, (u64, usize)>| m.values().map(|v| v.0).sum::<u64>();
        let (calls, unbound) = (total(&self.calls), total(&self.unbound));
        let recorded: Vec<(String, (u64, usize))> = lbp_script_vm::natives::RECORDED_ONLY
            .iter()
            .filter_map(|(class, name)| {
                let key = format!("{class}.{name}");
                let n = *self.calls.get(&key)?;
                Some((key, n))
            })
            .collect();
        // The natives that ask for a video or a sound and get the game's answer for
        // "nothing started".
        let requests: Vec<_> = lbp_script_vm::natives::RECORDED_REQUESTS
            .iter()
            .filter_map(|(class, name, answer)| {
                let key = format!("{class}.{name}");
                let n = *self.calls.get(&key)?;
                Some(serde_json::json!({
                    "native": key, "count": n.0, "levels": n.1, "answer": answer,
                }))
            })
            .collect();
        let mut doc = serde_json::json!({
            "unit": "native call; scripted Thing; fault (a Thing stops at its first fault)",
            "ticks_per_level": opts.ticks,
            "unbound_natives_fault": opts.strict,
            "scoreboard_requests": self.scoreboard_requests,
            "levels_with_scoreboard_requests": self.levels_with_scoreboard_requests,
            "requests_kept": self.requests,
            "global_setting_slots_held_at_end": self.global_settings_held,
            "levels_played": self.levels,
            "levels_with_faults": self.levels_with_faults,
            "instances": self.instances,
            "stopped_by_fault": self.stopped,
            "engine_object_faults": self.engine_faults,
            "triggers_visited": self.visited,
            "trigger_events_enter_inside_leave": self.events,
            "trigger_events_run": self.events_run,
            "trigger_events_without_script": self.events_without_script,
            "trigger_events_to_stopped": self.events_to_stopped,
            "switches_pulsed": opts.pulse,
            "switch_reactions": reactions_json(&self.reactions),
            "switch_outputs_changed": self.outputs_changed.values().sum::<usize>(),
            "switch_outputs_changed_by_type": self.outputs_changed,
            "levels_with_switch_outputs_changed": self.levels_with_outputs_changed,
            "things_deactivated_at_end": self.deactivated,
            "first_tick": {
                "switch_reactions": reactions_json(&self.first_reactions),
                "switch_outputs_changed_by_type": self.first_changed,
                "switch_outputs_changed_not_deactivated_by_type": self.first_changed_active,
                "things_deactivated": self.first_deactivated,
            },
            "native_calls": calls,
            "native_calls_bound": calls - unbound,
            "native_calls_unbound": unbound,
            "natives_called": self.calls.len(),
            "natives_called_unbound": self.unbound.len(),
            // Bound natives whose whole effect is a record in the world (a subset of
            // `native_calls_bound`).
            "native_calls_recorded_only": recorded.iter().map(|(_, n)| n.0).sum::<u64>(),
            "recorded_only_natives": recorded.iter().map(|(k, n)| serde_json::json!({
                "native": k, "count": n.0, "levels": n.1,
            })).collect::<Vec<_>>(),
            "natives_in_table": lbp_script_vm::natives::bound().count(),
            "faults": total(&self.faults),
            "fault_kinds": self.faults.len(),
            "thread_faults": total(&self.thread_faults),
            "unbound_natives": ranked(&self.unbound, "native", usize::MAX),
            "faults_by_kind": ranked(&self.faults, "fault", 200),
            "thread_faults_by_kind": ranked(&self.thread_faults, "fault", 100),
        });
        doc["player_sensors_visited"] = serde_json::json!(self.sensors_visited);
        doc["sensors"] = serde_json::json!(self.sensors);
        doc["sensors_unlike_stored"] = serde_json::json!(self.sensors_unlike);
        doc["sensors_unlike_stored_ternary"] = serde_json::json!(self.sensors_unlike_ternary);
        doc["messages"] = self.messages.json();
        doc["messages"]["levels_with_a_request"] =
            serde_json::json!(self.levels_with_message_requests);
        doc["messages"]["levels_with_a_message"] = serde_json::json!(self.levels_with_messages);
        doc["global_settings"] = self.settings.json();
        doc["display"] = self.display.json();
        doc["camera"] = self.camera.json();
        doc["pod"] = self.pod.json();
        doc["media"] = self.media.json();
        doc["recorded_request_natives"] = serde_json::json!(requests);
        doc
    }
}

fn run() -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let entries: Vec<&str> = all
        .iter()
        .filter_map(|a| a.strip_prefix("--entry="))
        .collect();
    let ticks: usize = all
        .iter()
        .find_map(|a| a.strip_prefix("--ticks="))
        .map_or(Ok(30), str::parse)
        .map_err(|e| format!("--ticks: {e}"))?;
    let play: Option<u64> = all
        .iter()
        .find_map(|a| a.strip_prefix("--play="))
        .map(str::parse)
        .transpose()
        .map_err(|e| format!("--play: {e}"))?;
    // `--flags=2,0x10`: bit masks ORed into the profile's word of LBP2 game progression
    // flags (default 0, a fresh profile).
    let flags: u32 = all
        .iter()
        .find_map(|a| a.strip_prefix("--flags="))
        .map(lbp_runtime::parse_progression_flags)
        .transpose()
        .map_err(|e| format!("--flags: {e}"))?
        .unwrap_or_default();
    // `--puter-in-use=TICK`: a player is at the pod computer from that tick on.
    let in_use_from: Option<u64> = all
        .iter()
        .find_map(|a| a.strip_prefix("--puter-in-use="))
        .map(str::parse)
        .transpose()
        .map_err(|e| format!("--puter-in-use: {e}"))?;
    // Part 24: how the level is started and what the player does.
    let option = |name: &str| all.iter().find_map(|a| a.strip_prefix(name));
    let pad_zone_from: Option<u64> = option("--enter-pad-zone=")
        .map(str::parse)
        .transpose()
        .map_err(|e| format!("--enter-pad-zone: {e}"))?;
    let presses = option("--press=")
        .map(parse_presses)
        .transpose()
        .map_err(|e| format!("--press: {e}"))?
        .unwrap_or_default();
    let menu_keys = option("--menu=")
        .map(parse_menu_keys)
        .transpose()
        .map_err(|e| format!("--menu: {e}"))?
        .unwrap_or_default();
    let colour = option("--player-colour=")
        .map(|c| u32::from_str_radix(c.trim_start_matches("0x"), 16))
        .transpose()
        .map_err(|e| format!("--player-colour: {e}"))?;
    let with_level = option("--with-level=");
    let args: Vec<&String> = all
        .iter()
        .filter(|a| {
            !a.starts_with("--entry=")
                && !a.starts_with("--ticks=")
                && !a.starts_with("--play=")
                && !a.starts_with("--flags=")
                && !a.starts_with("--puter-in-use=")
                && !a.starts_with("--enter-pad-zone=")
                && !a.starts_with("--press=")
                && !a.starts_with("--menu=")
                && !a.starts_with("--player-colour=")
                && !a.starts_with("--with-level=")
                && *a != "--our-pod"
                && *a != "--strict"
                && *a != "--visit-triggers"
                && *a != "--pulse-switches"
        })
        .collect();
    if args.len() != 5 {
        return Err(
            "usage: level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON \
                    [--entry=NAME]... [--ticks=N] [--play=TICKS] [--flags=MASK,..] \
                    [--puter-in-use=TICK] [--our-pod] [--enter-pad-zone=TICK] \
                    [--press=TICK:MASK,..] [--menu=TICK:KEY,..] [--player-colour=AARRGGBB] \
                    [--with-level=LEVEL-PATH] [--strict] [--visit-triggers] \
                    [--pulse-switches]"
                .into(),
        );
    }
    let output = Path::new(args[4]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let every = args[3] == "--all";
    if every && !entries.is_empty() {
        return Err("--entry runs one level, not --all".into());
    }
    let play_opts = play.map(|ticks| PlayOptions {
        ticks,
        flags,
        in_use_from,
        strict: all.iter().any(|a| a == "--strict"),
        visit: all.iter().any(|a| a == "--visit-triggers"),
        pulse: all.iter().any(|a| a == "--pulse-switches"),
        our_pod: all.iter().any(|a| a == "--our-pod"),
        pad_zone_from,
        presses,
        menu_keys,
        colour,
    });
    if play.is_some() && !entries.is_empty() {
        return Err("use --play or --entry, not both".into());
    }
    if with_level.is_some() && (every || play.is_none()) {
        return Err("--with-level goes with --play on one level".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(args[2]), &catalog, &control)?;
    let (scripts, scripts_not_read) = lbp_ui_script::load::load(&source, &catalog, &control)?;
    // The English tables, patch first (as the story browser layers them). INFERRED that
    // these are loaded while a level plays; a table that cannot be read is left out.
    let (strings, table_status) = lbp_preview::story::load_strings(&source, &catalog, &control);
    let translations: Vec<std::sync::Arc<_>> = [strings.patch, strings.base]
        .into_iter()
        .flatten()
        .map(std::sync::Arc::new)
        .collect();
    let mut vm = Vm::default();
    let scripts_loaded = scripts.len();
    let with_guid = scripts.iter().filter(|(g, _)| g.is_some()).count();
    for (guid, s) in &scripts {
        vm.load(s.clone(), *guid);
    }

    let mut keys = Vec::new();
    if every {
        let mut hashes = BTreeMap::new();
        for r in &catalog.snapshot.resources {
            if r.display_type() == "level" {
                hashes
                    .entry(r.map_row.sha1.to_ascii_lowercase())
                    .or_insert_with(|| r.map_row.path.clone());
            }
        }
        for (h, path) in hashes {
            if let Ok(key) = lbp_preview::resolve(&catalog, None, Some(&h)) {
                keys.push((key, path));
            }
        }
    } else {
        let (key, label) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(args[3]))?;
        keys.push((key, label));
    }

    let mut totals = Totals::default();
    let mut levels = Vec::new();
    let mut last_world = None;
    let mut play_totals = PlayTotals::default();
    for (key, label) in &keys {
        let read =
            lbp_preview::level_scene::read_level_bytes(&source, key, &control).and_then(|b| {
                lbp_level::read(&b, lbp_level::Limits::default(), &|| false)
                    .map_err(|e| e.to_string())
            });
        let report = match read {
            Ok(r) => r,
            Err(_) => {
                totals.levels_not_read += 1;
                continue;
            }
        };
        let mut world = LevelWorld::from_things(&report.things);
        world.translations = translations.clone();
        // `--all --play`: each level on its own VM (same load order, so the same class
        // ids), so no heap object or static outlives its level.
        let mut fresh = match (&play_opts, every) {
            (Some(_), true) => {
                let mut v = Vm::default();
                for (guid, s) in &scripts {
                    v.load(s.clone(), *guid);
                }
                Some(v)
            }
            _ => None,
        };
        let level_vm = fresh.as_mut().unwrap_or(&mut vm);
        totals.objects_built += world.bind_objects(level_vm);
        let mut row = add_level(level_vm, &world, &mut totals);
        row["level"] = serde_json::json!(label);
        row["stopped"] = serde_json::json!(report.stop.is_some());
        if let (Some(opts), true) = (&play_opts, every) {
            let p = play_level(level_vm, world, opts);
            play_totals.add(&p);
            row["play"] = serde_json::json!({
                "instances": p.instances,
                "stopped_by_fault": p.stopped,
                "unbound_native_calls": p.unbound.values().sum::<u64>(),
                // Display values unlike the level file, the kinds with a Thing only.
                "display_unlike_the_level_file": p.display.things.rows().iter()
                    .filter(|(_, n)| *n > 0)
                    .copied()
                    .collect::<BTreeMap<_, _>>(),
                "render_mesh_tints_set": p.display.tints.iter()
                    .map(|(kind, n)| (*kind, n.0))
                    .collect::<BTreeMap<_, _>>(),
            });
            levels.push(row);
            continue;
        }
        levels.push(row);
        last_world = Some(world);
    }

    // Function names with code over the classes in use.
    let mut names = BTreeMap::<String, usize>::new();
    for &c in &totals.classes {
        for n in vm.function_names(c) {
            *names.entry(n.to_owned()).or_default() += 1;
        }
    }
    let mut ranked: Vec<_> = names.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(60);

    // Optional run on the single level.
    let mut runs = serde_json::Map::new();
    let second_level = with_level.and_then(|path| Some((path, last_world.take()?)));
    if let (Some(opts), Some((path, world))) = (&play_opts, second_level) {
        // The second level of the session, on a VM of its own (same load order).
        let (key, label) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(path))?;
        let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
        let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
            .map_err(|e| e.to_string())?;
        let mut second = LevelWorld::from_things(&report.things);
        second.translations = translations.clone();
        second.story_slots = story_slots(&catalog, &source, &control)?;
        let mut second_vm = Vm::default();
        for (guid, s) in &scripts {
            second_vm.load(s.clone(), *guid);
        }
        second.bind_objects(&mut second_vm);
        let first = (keys[0].1.as_str(), &mut vm, world);
        let session = play_session(first, (label.as_str(), &mut second_vm, second), opts);
        runs.insert("session".into(), session);
    }
    if let (Some(opts), Some(world), false) = (&play_opts, last_world.take(), every) {
        runs.insert("play".into(), play_level(&mut vm, world, opts).json);
    }
    if let (false, Some(mut world)) = (entries.is_empty(), last_world) {
        world.add_class_defaults(&vm);
        world.progression_flags = flags;
        let mut host = Bindings::new(world);
        let found = instances(&vm, &host.world);
        let mut runner = Runner::default();
        for entry in &entries {
            let mut outcomes = BTreeMap::<String, usize>::new();
            let mut faults = Vec::new();
            // Faults by site and message, Thing numbers removed.
            let mut fault_kinds = BTreeMap::<String, usize>::new();
            let mut note = |o: &Outcome, thing: u32, outcomes: &mut BTreeMap<String, usize>| {
                let key = match o {
                    Outcome::Fault(_) => "fault",
                    Outcome::Done => "done",
                    Outcome::Yielded => "yielded",
                    Outcome::NoClass => "no_class",
                    Outcome::NoFunction => "no_function",
                };
                *outcomes.entry(key.into()).or_default() += 1;
                if let Outcome::Fault(f) = o {
                    let kind = f.replace(&format!("thing {thing} "), "thing # ");
                    *fault_kinds.entry(kind).or_default() += 1;
                    if faults.len() < MAX_FAULTS {
                        faults.push(format!("thing {thing}: {f}"));
                    }
                }
            };
            for inst in &found {
                let mut budget = BUDGET_PER_CALL;
                let o = runner.start(&mut vm, &mut host, *inst, entry, &mut budget);
                note(&o, inst.thing, &mut outcomes);
            }
            let mut resumed = BTreeMap::<String, usize>::new();
            for _ in 0..ticks {
                if runner.pending() == 0 {
                    break;
                }
                let mut budget = BUDGET_PER_CALL.saturating_mul(runner.pending() as u64);
                for (thing, o) in runner.tick(&mut vm, &mut host, &mut budget) {
                    note(&o, thing, &mut resumed);
                }
            }
            runs.insert(
                (*entry).to_owned(),
                serde_json::json!({
                    "started": outcomes,
                    "resumed": resumed,
                    "still_pending": runner.pending(),
                    "first_faults": faults,
                    "faults_by_kind": fault_kinds,
                }),
            );
        }
        let mut unbound: Vec<_> = host.unbound.iter().collect();
        unbound.sort_by(|a, b| b.1.cmp(a.1));
        unbound.truncate(40);
        runs.insert(
            "unbound_natives".into(),
            serde_json::json!(unbound
                .iter()
                .map(|(k, n)| serde_json::json!({"native": k, "calls": n}))
                .collect::<Vec<_>>()),
        );
        runs.insert(
            "written".into(),
            serde_json::json!(host.world.written.iter().take(40).collect::<Vec<_>>()),
        );
    }

    let mut by_class: Vec<_> = totals.by_class.iter().collect();
    by_class.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let unresolved: BTreeMap<String, usize> = totals
        .unresolved
        .iter()
        .map(|(g, n)| (format!("{g:x}"), *n))
        .collect();
    let doc = serde_json::json!({
        "schema_version": 1,
        "unit": "Thing with a PScript resource GUID; function names per distinct class",
        "scripts_loaded": scripts_loaded,
        "scripts_with_guid": with_guid,
        "scripts_not_read": scripts_not_read,
        "translation_tables": {
            "unit": "table read; distinct LAMS key, summed over the tables",
            "read": translations.len(),
            "keys": translations.iter().map(|t| t.distinct_keys()).sum::<usize>(),
            "status": table_status.iter()
                .map(|t| format!("g{}: {:?} {}", t.guid, t.state, t.detail))
                .collect::<Vec<_>>(),
        },
        "levels": totals.levels,
        "levels_not_read": totals.levels_not_read,
        "things": totals.things,
        "scripted_things": totals.scripted,
        "with_fields": totals.with_fields,
        "resolved": totals.resolved,
        "unresolved_by_guid": unresolved,
        "classes_in_use": totals.classes.len(),
        "instances_by_class": by_class,
        "fields_matched": totals.fields_matched,
        "fields_missing_in_class": totals.fields_missing_in_class,
        "unconverted_values": totals.unconverted,
        "objects_built": totals.objects_built,
        "function_names": ranked,
        "triggers": totals.triggers.json(),
        "switches": totals.switches.json(),
        "pworld": totals.pworld.json(),
        "per_level": levels,
        "runs": runs,
        "play_all": play_opts.as_ref().filter(|_| every).map(|o| play_totals.json(o)),
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "scripts {scripts_loaded} (with GUID {with_guid}, not read {scripts_not_read}); \
         levels {} (not read {}); scripted things {} resolved {} (classes {}), unresolved \
         GUIDs {}; fields matched {} missing in class {}; unconverted values {}; objects built {}",
        totals.levels,
        totals.levels_not_read,
        totals.scripted,
        totals.resolved,
        totals.classes.len(),
        totals.unresolved.len(),
        totals.fields_matched,
        totals.fields_missing_in_class,
        totals.unconverted,
        totals.objects_built
    );
    if let (Some(o), true) = (&play_opts, every) {
        let mut j = play_totals.json(o);
        for k in ["unbound_natives", "faults_by_kind", "thread_faults_by_kind"] {
            j.as_object_mut().map(|m| m.remove(k));
        }
        println!("--all --play={}: {j}", o.ticks);
    }
    for (entry, r) in &runs {
        if entry == "session" {
            println!("--with-level session path: {}", r["path"]);
        } else if entry != "unbound_natives" && entry != "written" {
            let flag = if entry == "play" { "--play" } else { "--entry" };
            println!("{flag}={entry}: {r}");
        }
    }
    Ok(())
}

/// D328: the slots of `gamedata/data/lbp2story_slots.slt` (GUID 000162ec), the list the
/// Story search's results are made from.
fn story_slots(
    catalog: &lbp_catalog::Catalog,
    source: &AttachedSource,
    control: &LoadControl,
) -> Result<Vec<lbp_runtime::SearchSlot>, String> {
    let key = lbp_preview::resolve(catalog, Some(0x162ec), None)?;
    let bytes = lbp_preview::level_scene::read_resource(source, &key, control, 16 << 20)?;
    let report = lbp_level::slots::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    match report.result {
        Ok(lbp_level::slots::UiData::SlotList(list)) => {
            Ok(lbp_runtime::SearchSlot::from_list(&list))
        }
        Ok(_) => Err("lbp2story_slots.slt is not a slot list".into()),
        Err(e) => Err(format!("lbp2story_slots.slt: {e:?}")),
    }
}

/// D327: what the pod's search state told the planets and the pod computer.
fn search_json(s: &lbp_runtime::PlanetsSearch) -> serde_json::Value {
    serde_json::json!({
        "selected_planet": s.selected_planet,
        "cursor_mode": s.cursor_mode,
        "browse_mode": s.browse_mode,
        "allowed_browse_modes": s.allowed_browse_modes,
        "loader": s.loader,
        "earth_has_control": s.earth_has_control,
        "want_search_details_calls": s.want_search_details_calls,
        "publish_cursor_badge_calls": s.publish_cursor_badge_calls,
        "error_message_calls": s.error_message_calls,
        "default_result": s.default_result,
        "cur_result": s.cur_result,
        "set_search_results_calls": s.set_search_results_calls,
    })
}
