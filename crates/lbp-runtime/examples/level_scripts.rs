//! Rt PC check (docs/level-scripts.md): which scripts levels use and how they run.
//!
//! level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH NEW-JSON [--entry=NAME]... [--ticks=N]
//! level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH NEW-JSON --play=TICKS [--flags=N,..]
//!     [--puter-in-use=TICK]
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
//! Names and counts only; no bytecode. Output: a NEW JSON file under
//! ignored `local-analysis/`. Counting unit: Thing with a PScript part (instances) or
//! distinct script class (function names).
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_runtime::{
    instances, BubbleStyle, GlobalSettingsReport, LevelScripts, LevelWorld, MessageKind, Outcome,
    ReactionReport, Runner, StepReport, GLOBAL_SETTINGS, GLOBAL_SETTING_NAMES,
};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Vm;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const BUDGET_PER_CALL: u64 = 1_000_000;
const MAX_FAULTS: usize = 30;

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
    flags: BTreeSet<i32>,
    in_use_from: Option<u64>,
    strict: bool,
    visit: bool,
    pulse: bool,
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
    levels: [usize; 12],
    /// Changes of visibility, colour and world matrix marked for a display (a Thing
    /// changed on two ticks counts twice).
    marks: [u64; 3],
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
            },
        })
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

/// How far from a trigger's Thing the `--visit-triggers` player goes to leave it.
const VISIT_AWAY: f32 = 1.0e7;

/// Runs the `LevelScripts` schedule on one level whose objects are bound to `vm`.
fn play_level(vm: &mut Vm, mut world: LevelWorld, opts: &PlayOptions) -> Played {
    world.add_class_defaults(vm);
    world.progression_flags = opts.flags.clone();
    world.pod_computer_in_use = opts.in_use_from == Some(0);
    world.messages.log_limit = lbp_runtime::MAX_REQUESTS;
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
    let camera_json = |c: &lbp_runtime::CameraState| {
        serde_json::json!({
            "from_to_up": c.from_to_up.map(|c| serde_json::json!({
                "from": c.from, "to": c.to, "up": c.up, "params": c.params,
            })),
            "from_to_up_calls": c.from_to_up_calls,
        })
    };
    let mut camera_before_in_use = None;
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
        let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
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
        let player = host.world.add_player([0.; 3], (0., 0.));
        for s in spots {
            for dx in [0., 0., VISIT_AWAY] {
                host.world
                    .move_player(player, [s[0] + dx, s[1], s[2]], (s[2] - 1., s[2] + 1.));
                let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
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
    // `--flags=2,5`: LBP2 game progression flags set on the profile (default none).
    let flags: std::collections::BTreeSet<i32> = all
        .iter()
        .find_map(|a| a.strip_prefix("--flags="))
        .map(|f| f.split(',').map(str::parse).collect::<Result<_, _>>())
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
    let args: Vec<&String> = all
        .iter()
        .filter(|a| {
            !a.starts_with("--entry=")
                && !a.starts_with("--ticks=")
                && !a.starts_with("--play=")
                && !a.starts_with("--flags=")
                && !a.starts_with("--puter-in-use=")
                && *a != "--strict"
                && *a != "--visit-triggers"
                && *a != "--pulse-switches"
        })
        .collect();
    if args.len() != 5 {
        return Err(
            "usage: level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON \
                    [--entry=NAME]... [--ticks=N] [--play=TICKS] [--flags=N,..] \
                    [--puter-in-use=TICK] [--strict] [--visit-triggers] \
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
        flags: flags.clone(),
        in_use_from,
        strict: all.iter().any(|a| a == "--strict"),
        visit: all.iter().any(|a| a == "--visit-triggers"),
        pulse: all.iter().any(|a| a == "--pulse-switches"),
    });
    if play.is_some() && !entries.is_empty() {
        return Err("use --play or --entry, not both".into());
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
    if let (Some(opts), Some(world), false) = (&play_opts, last_world.take(), every) {
        runs.insert("play".into(), play_level(&mut vm, world, opts).json);
    }
    if let (false, Some(mut world)) = (entries.is_empty(), last_world) {
        world.add_class_defaults(&vm);
        world.progression_flags = flags.clone();
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
        if entry != "unbound_natives" && entry != "written" {
            let flag = if entry == "play" { "--play" } else { "--entry" };
            println!("{flag}={entry}: {r}");
        }
    }
    Ok(())
}
