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
//! scripts run; the report counts the events raised and run. `--pulse-switches` then runs
//! two more ticks in which every switch output is forced on, then off, through the per-tick
//! switch pass, so the `SwitchPerformReaction` scripts run with true, then false (a test
//! drive: the pass itself computes the logic types and keeps the sensors' stored signals). `switch_reactions` counts the
//! output targets the pass visited and the reaction calls by script GUID. `triggers` is the PTrigger
//! census over the levels read, `switches` the PSwitch census with the input tables built
//! from the outputs' targets. Names and counts only; no bytecode. Output: a NEW JSON file under
//! ignored `local-analysis/`. Counting unit: Thing with a PScript part (instances) or
//! distinct script class (function names).
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_runtime::{
    instances, LevelScripts, LevelWorld, Outcome, ReactionReport, Runner, StepReport,
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
}

impl Switches {
    fn add(&mut self, vm: &Vm, world: &LevelWorld, found: &[lbp_runtime::Instance]) {
        self.collisions += world.input_collisions;
        self.ports_dropped += world.input_ports_dropped;
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
    /// Switch outputs whose signal at the end differs from the stored one, by switch type.
    outputs_changed: BTreeMap<i32, usize>,
    /// Things deactivated at the end.
    deactivated: usize,
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

/// How far from a trigger's Thing the `--visit-triggers` player goes to leave it.
const VISIT_AWAY: f32 = 1.0e7;

/// Runs the `LevelScripts` schedule on one level whose objects are bound to `vm`.
fn play_level(vm: &mut Vm, mut world: LevelWorld, opts: &PlayOptions) -> Played {
    world.add_class_defaults(vm);
    world.progression_flags = opts.flags.clone();
    world.pod_computer_in_use = opts.in_use_from == Some(0);
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
    for tick in 0..opts.ticks {
        if opts.in_use_from.is_some_and(|t| t > 0 && t == tick) {
            camera_before_in_use = Some(camera_json(&host.world.camera));
            host.world.pod_computer_in_use = true;
        }
        let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
        ticks_with_faults += usize::from(!t.faults.is_empty());
        note_faults(&t);
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
    let mut visited = 0usize;
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
                ticks_with_faults += usize::from(!t.faults.is_empty());
                note_faults(&t);
                visit.absorb(t);
            }
            visited += 1;
        }
    }
    // `--pulse-switches`: one tick with every switch output forced on, one with it forced
    // off, so each wired target sees a rising and a falling edge.
    let mut pulse = StepReport::default();
    if opts.pulse {
        for on in [true, false] {
            host.world.switch_pulse = Some(on);
            let t = level.tick_level(vm, &mut host, BUDGET_PER_CALL);
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
        })
    };
    let mut unbound: Vec<_> = host.unbound.iter().collect();
    unbound.sort_by(|a, b| b.1.cmp(a.1));
    unbound.truncate(40);
    let outputs_changed = outputs_changed(&host.world, false);
    let deactivated = host.world.things.iter().filter(|t| t.deactivated).count();
    let json = serde_json::json!({
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
    sum.absorb(visit);
    sum.absorb(pulse);
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
        calls: host.calls,
        unbound: host.unbound,
        outputs_changed,
        deactivated,
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
    faults: BTreeMap<String, (u64, usize)>,
    thread_faults: BTreeMap<String, (u64, usize)>,
    outputs_changed: BTreeMap<i32, usize>,
    levels_with_outputs_changed: usize,
    deactivated: usize,
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
        serde_json::json!({
            "unit": "native call; scripted Thing; fault (a Thing stops at its first fault)",
            "ticks_per_level": opts.ticks,
            "unbound_natives_fault": opts.strict,
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
            "faults": total(&self.faults),
            "fault_kinds": self.faults.len(),
            "thread_faults": total(&self.thread_faults),
            "unbound_natives": ranked(&self.unbound, "native", usize::MAX),
            "faults_by_kind": ranked(&self.faults, "fault", 200),
            "thread_faults_by_kind": ranked(&self.thread_faults, "fault", 100),
        })
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
