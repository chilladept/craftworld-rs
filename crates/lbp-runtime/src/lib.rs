//! Rt: a level as the script VM's [`World`] (docs/runtime-research.md, item Rt;
//! docs/level-scripts.md). Separate from the parsing crates, which never depend on it.
//!
//! - [`LevelWorld`] builds the Thing tree from a read level: Thing handles are indices in
//!   the level's Thing list; each Thing with a PScript part gets its instance fields as
//!   script members (`GET/SET_SP_MEMBER` on that Thing), converted from the stored values.
//! - [`Runner`] runs a named function of each scripted Thing's class with the Thing as
//!   `this` in register 0 (the convention the test-script and Gooey hosts use, INFERRED for
//!   Thing scripts) and resumes yielded scripts once per tick.
//!
//! - [`LevelScripts`] is the per-tick schedule: [`CREATE`] once on every scripted Thing,
//!   then each tick resume suspended scripts and run [`UPDATE`] on the others.
//!
//! Entry names: `OnCreate__` and `Update__` are the create/update functions (PC D152:
//! across the 49 classes Rookie Test and the other levels use, `OnCreate__` is defined by
//! 24 and `Update__` by 11; the remaining names are UI, switch and enter/leave handlers).
use lbp_level::parts::{ScriptObject, ScriptPart, ScriptValue, ShapeColor, SwitchSignal};
use lbp_level::thing::ThingRecord;
use lbp_script_vm::natives::{
    Bindings, CameraFromToUp, ResourceKind, SwitchInput, World, SCORE_BOARD_SCRIPT,
};
use lbp_script_vm::{ClassId, Host, MemberKind, ObjectId, Step, Task, Value, Vm};
use std::collections::{BTreeMap, BTreeSet};

mod switches;
pub use switches::*;

/// One Thing as the runtime sees it.
#[derive(Debug, Clone, Default)]
pub struct ThingState {
    pub uid: Option<i32>,
    pub parent: Option<u32>,
    pub first_child: Option<u32>,
    pub next_sibling: Option<u32>,
    /// POS world matrix in stored order (translation at 12..14).
    pub world_matrix: Option<[f32; 16]>,
    pub parts: Vec<&'static str>,
    /// GUID of the PScript resource, when the Thing has one.
    pub script_guid: Option<u32>,
    /// Script members by field name; `None` when the Thing has no PScript layout.
    pub members: Option<BTreeMap<String, Value>>,
    /// PShape colour as a colour vector (`GetShapeEditorColour`).
    pub shape_colour: Option<[f32; 4]>,
    /// Render-mesh editor colour tint a script set (`SetRenderMeshEditorColourTint`).
    pub render_mesh_tint: Option<[f32; 4]>,
    /// Animation playback rate a script set (`Native.SetAnimSpeed`, INFERRED multiplier).
    pub anim_speed: Option<f32>,
    /// Animation position a script set (`Native.SetAnimPos`, INFERRED time or fraction).
    pub anim_pos: Option<f32>,
    /// PTrigger, when the Thing has one.
    pub trigger: Option<TriggerState>,
    /// PSwitch, when the Thing has one.
    pub switch: Option<SwitchState>,
    /// The Thing's input table by port: the switch output wired to each port, `None`
    /// for a port nothing is wired to (see [`LevelWorld::wire_inputs`]).
    pub inputs: Vec<Option<InputWire>>,
    /// The game's "deactivated" byte (Thing+0x7f): while set, the switch pass forces the
    /// Thing's outputs off ([`LevelWorld::update_switch`]). FOUND: the game derives it
    /// from the microchip or circuit board holding the Thing
    /// ([`LevelWorld::recompute_deactivated`]).
    pub deactivated: bool,
    /// PMicrochip: the chip's circuit-board Thing. The components on the board are the
    /// board's children.
    pub circuit_board: Option<u32>,
}

/// A PSwitch at run time: the stored settings and its outputs. The switch pass
/// ([`LevelWorld::update_switch`]) rewrites the outputs' signals each tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchState {
    pub switch_type: i32,
    pub inverted: bool,
    pub behavior: i32,
    /// In stored order; `None` for an output the level file leaves null.
    pub outputs: Vec<Option<SwitchOutputState>>,
    /// ManualActivation as stored. The live value is [`SwitchMemory::manual`].
    pub manual_activation: SwitchInput,
    /// ActivationHoldTime as stored: a timer's length in ticks.
    pub activation_hold_time: i32,
    /// BulletsRequired: the port count of an AND or OR gate, a counter's target.
    pub bullets_required: i32,
    /// The AND / OR output mode (stored in the RandomOffTimeMin slot).
    pub gate_mode: i32,
    pub team_filter: i32,
    pub include_touching: i32,
    /// RetardedOldJoint: the output deadband of [`compute_output`].
    pub old_joint: bool,
    /// CrappyOldLbp1Switch as stored (census only; see `force`).
    pub old_lbp1_switch: bool,
    /// Re-fire flag (PSwitch+0xe3): the next update hands `true` to the targets of every
    /// output that is on, changed or not, and clears the flag. What sets it in the game
    /// is not established; nothing sets it here.
    pub force: bool,
    /// What the switch remembers between ticks ([`compute_output`]'s state slot).
    pub memory: SwitchMemory,
}

/// One output of a PSwitch: its signal and the (Thing, port) inputs it is wired to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchOutputState {
    /// The current signal, as the switch-input natives read it.
    pub signal: SwitchInput,
    /// The signal the level file stores.
    pub stored: SwitchInput,
    pub targets: Vec<(Option<u32>, i32)>,
}

/// What is wired to one input port: output `output` of the PSwitch on Thing `switch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputWire {
    pub switch: u32,
    pub output: usize,
}

/// Input ports past this are not wired (a guard against a damaged file; the count of
/// targets it drops is [`LevelWorld::input_ports_dropped`]).
pub const MAX_INPUT_PORTS: usize = 1024;

/// A PTrigger at run time: the stored settings and the Things inside it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriggerState {
    /// 0 circle, 1 box, 4 sphere (see [`LevelWorld::update_triggers`]).
    pub trigger_type: i8,
    pub radius: f32,
    pub hysteresis: f32,
    pub all_z_layers: bool,
    pub enabled: bool,
    pub z_offset: f32,
    /// Things inside, in the order they entered (the stored list first).
    pub inside: Vec<u32>,
}

/// A player's Sackboy as a Thing of the level (see [`LevelWorld::add_player`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerState {
    pub thing: u32,
    /// Nearest and farthest z his body covers (the layers he stands in).
    pub layers: (f32, f32),
}

/// What a trigger did to a Thing this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TriggerEventKind {
    Enter,
    Inside,
    Leave,
}

impl TriggerEventKind {
    /// The script function the game calls for the event.
    pub fn function(self) -> &'static str {
        match self {
            Self::Enter => "Enter__Q5Thing",
            Self::Inside => "Inside__Q5Thing",
            Self::Leave => "Leave__Q5Thing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerEvent {
    /// The Thing with the PTrigger.
    pub trigger: u32,
    pub kind: TriggerEventKind,
    /// The Thing that entered, is inside or left.
    pub thing: u32,
}

/// `p` in the space of the stored-order matrix `m` (translation at 12..14, columns the
/// axes), or `None` when `m` cannot be inverted.
fn to_local(m: &[f32; 16], p: [f32; 3]) -> Option<[f32; 3]> {
    let (a, b, c) = (&m[0..3], &m[4..7], &m[8..11]);
    let cross = |u: &[f32], v: &[f32]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let dot = |u: &[f32], v: &[f32]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let (bc, ca, ab) = (cross(b, c), cross(c, a), cross(a, b));
    let det = dot(a, &bc);
    if det == 0. || !det.is_finite() {
        return None;
    }
    let d = [p[0] - m[12], p[1] - m[13], p[2] - m[14]];
    Some([dot(&bc, &d) / det, dot(&ca, &d) / det, dot(&ab, &d) / det])
}

/// Camera state scripts set through `Cam.*` natives, for the viewer to read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CameraState {
    /// The last `Cam.SetCameraFromToUp__rrrffff`.
    pub from_to_up: Option<CameraFromToUp>,
    pub from_to_up_calls: u64,
    /// The last `Cam.SetDOFAndFog__fff` (as passed).
    pub dof_and_fog: Option<[f32; 3]>,
    pub motion_blur_clears: u64,
}

/// An engine part a script reached through a native member (`thing.SpriteLight`), with
/// the `ref_ptr` members scripts wrote to it.
#[derive(Debug, Clone, Default)]
pub struct PartState {
    pub thing: u32,
    /// The reader's part name (`SPRITE_LIGHT`).
    pub part: &'static str,
    pub members: BTreeMap<String, Value>,
}

/// Packed PShape colour (head > 0x389) as a colour vector. INFERRED byte order
/// 0xAARRGGBB (white 0xffffffff is the common value; nothing yet tells R from B).
pub fn unpack_colour(c: u32) -> [f32; 4] {
    let ch = |shift: u32| ((c >> shift) & 0xff) as f32 / 255.;
    [ch(16), ch(8), ch(0), ch(24)]
}

/// A read level as the VM's world.
#[derive(Debug, Default)]
pub struct LevelWorld {
    pub things: Vec<ThingState>,
    pub world: Option<u32>,
    /// `WRITE` output of the scripts.
    pub written: Vec<String>,
    /// Stored non-null object references with no VM value: before [`Self::bind_objects`]
    /// every member that is not a string; after it, references (members, array elements,
    /// instance fields) whose object could not be built (resource, audio handle, instance
    /// of a class the VM has not loaded).
    pub unconverted: usize,
    /// Script object contents by the level's object number (first occurrence).
    pub stored_objects: BTreeMap<usize, ScriptObject>,
    /// Members waiting for a VM object: (Thing, field, object number).
    pub object_members: Vec<(u32, String, usize)>,
    /// Parts handed to scripts; a part handle is its index + 1 (`Raw(0)` is null).
    pub part_handles: Vec<PartState>,
    pub camera: CameraState,
    /// The PodComputer script object `GetPodComputer__` returns (see [`add_engine_objects`]).
    pub pod_computer: Option<ObjectId>,
    /// Gooey objects made ahead for `CreateGooey__i` (the host cannot allocate VM objects
    /// during a native call); each call hands out the next one.
    pub gooey_pool: Vec<ObjectId>,
    pub gooeys_created: usize,
    /// Interactive-music commands scripts sent, by name.
    pub audio_commands: BTreeMap<String, u64>,
    /// LBP2 game progression flags set on the profile (`IsLBP2GameProgressionFlagSet__i`).
    /// Empty = a fresh profile, on which PlanetsWorld starts its intro cutscene thread.
    pub progression_flags: BTreeSet<i32>,
    /// `PodComputer.GetPodComputerInUse__`: a player is at the pod computer ("PuterInUse").
    /// False by default (INFERRED: nobody at the computer when the Pod loads).
    pub pod_computer_in_use: bool,
    /// The Planets object `PodComputer.GetPlanets__` returns.
    pub planets: Option<ObjectId>,
    /// Planet objects by `GetCrafteroid__i` index - 1, with their Thing.
    pub crafteroids: Vec<(ObjectId, Option<u32>)>,
    /// `Game.SetPlayerScoreMultiplierTimer` by player number.
    pub score_multiplier_timers: BTreeMap<i32, i32>,
    /// Players' Sackboys ([`LevelWorld::add_player`]); none until a caller adds one.
    pub players: Vec<PlayerState>,
    /// Switch targets whose port already had an output wired (the first one is kept).
    pub input_collisions: usize,
    /// Switch targets with a negative port or one at or past [`MAX_INPUT_PORTS`].
    pub input_ports_dropped: usize,
    /// Script GUIDs of the loaded classes that are, or derive from, the scoreboard class
    /// (filled by [`LevelWorld::add_class_defaults`]).
    pub score_board_scripts: BTreeSet<u32>,
    /// The switch Things in pass order, once computed ([`LevelWorld::switch_order`]).
    pub switch_order: Option<Vec<u32>>,
    /// A test drive for the switch pass: while `Some(on)`, every output's computed signal
    /// is replaced by ternary 1 and activation 1.0 (`on`) or ternary 0 and activation 0.0,
    /// with the stored player. The deactivated forcing still applies. Not a game feature.
    pub switch_pulse: Option<bool>,
    /// Set once the deactivated flags have their start values
    /// ([`LevelWorld::prepare_switches`]).
    pub switches_prepared: bool,
}

/// VM value for a stored script field; `None` for a field the instance does not store.
/// Object references other than strings with their text at hand become null and are
/// counted by the caller ([`LevelWorld::bind_objects`] builds them).
pub fn member_value(v: &ScriptValue) -> Option<(Value, bool)> {
    stored_value(v, &|_| None)
}

/// [`member_value`] with `object` giving the VM value for an object number.
fn stored_value(v: &ScriptValue, object: &dyn Fn(usize) -> Option<Value>) -> Option<(Value, bool)> {
    Some(match v {
        ScriptValue::Bool(b) => (Value::Bool(*b), true),
        ScriptValue::Char(c) => (Value::Char(*c as u16), true),
        ScriptValue::Int(i) => (Value::Int(*i), true),
        ScriptValue::Float(f) => (Value::Float(*f), true),
        ScriptValue::V4(v) => (Value::Vec4(*v), true),
        ScriptValue::M44(m) => (Value::Mat44(*m), true),
        ScriptValue::Thing(t) => (Value::Thing(t.and_then(|i| u32::try_from(i).ok())), true),
        ScriptValue::Object { kind: 0, .. } => (Value::Object(None), true),
        ScriptValue::Object { id, body, .. } => match (body.as_deref(), id.and_then(object)) {
            (Some(ScriptObject::Text(t)), _) => (Value::Str(t.as_str().into()), true),
            (_, Some(v)) => (v, true),
            _ => (Value::Object(None), false),
        },
        ScriptValue::NotStored => return None,
    })
}

/// Object contents by number, from a stored value and everything nested in it.
fn collect_objects(v: &ScriptValue, out: &mut BTreeMap<usize, ScriptObject>) {
    let ScriptValue::Object {
        id: Some(id),
        body: Some(body),
        ..
    } = v
    else {
        return;
    };
    match &**body {
        ScriptObject::Objects(items) => items.iter().for_each(|i| collect_objects(i, out)),
        ScriptObject::Instance(p) => p.fields.iter().for_each(|f| collect_objects(&f.value, out)),
        _ => {}
    }
    out.entry(*id).or_insert_with(|| (**body).clone());
}

fn signal(s: &SwitchSignal) -> SwitchInput {
    SwitchInput {
        activation: s.activation,
        ternary: s.ternary,
        player: s.player,
    }
}

/// `"PSpriteLight"` -> `"SPRITE_LIGHT"`, the reader's part name.
fn part_name(native: &str) -> String {
    let base = native.strip_prefix('P').unwrap_or(native);
    let mut out = String::new();
    for (i, c) in base.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    // The one part whose reader name does not split at the capital.
    if out == "YELLOW_HEAD" {
        out = "YELLOWHEAD".into();
    }
    out
}

impl LevelWorld {
    pub fn from_things(list: &[ThingRecord]) -> Self {
        let handle = |i: usize| u32::try_from(i).ok();
        let mut w = LevelWorld {
            things: list
                .iter()
                .map(|t| ThingState {
                    uid: t.uid,
                    parent: t.parent.and_then(handle),
                    world_matrix: t.data.pos.as_ref().map(|p| p.world),
                    parts: t.parts.clone(),
                    shape_colour: t.data.shape.as_ref().map(|s| match s.color {
                        ShapeColor::Packed(c) => unpack_colour(c),
                        ShapeColor::Vector(v) => v,
                    }),
                    trigger: t.data.trigger.as_ref().map(|g| TriggerState {
                        trigger_type: g.trigger_type,
                        radius: g.radius_multiplier,
                        hysteresis: g.hysteresis_multiplier,
                        all_z_layers: g.all_z_layers,
                        enabled: g.enabled,
                        z_offset: g.z_offset,
                        inside: g.in_things.iter().filter_map(|i| handle((*i)?)).collect(),
                    }),
                    switch: t.data.switch.as_ref().map(|s| SwitchState {
                        switch_type: s.switch_type,
                        inverted: s.inverted,
                        behavior: s.behavior,
                        outputs: s
                            .outputs
                            .iter()
                            .map(|o| {
                                o.as_ref().map(|o| SwitchOutputState {
                                    signal: signal(&o.signal),
                                    stored: signal(&o.signal),
                                    targets: o
                                        .targets
                                        .iter()
                                        .map(|&(t, port)| (t.and_then(handle), port))
                                        .collect(),
                                })
                            })
                            .collect(),
                        manual_activation: signal(&s.manual_activation),
                        activation_hold_time: s.activation_hold_time,
                        bullets_required: s.bullets_required,
                        gate_mode: s.random_off_time_min,
                        team_filter: s.team_filter,
                        include_touching: s.include_touching,
                        old_joint: s.retarded_old_joint,
                        old_lbp1_switch: s.crappy_old_lbp1_switch,
                        force: false,
                        memory: SwitchMemory {
                            updates: 0,
                            manual: signal(&s.manual_activation),
                            count: s.bullets_detected,
                            timer: s.timer_count,
                        },
                    }),
                    circuit_board: t
                        .data
                        .microchip
                        .and_then(|m| m.circuit_board)
                        .and_then(handle),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        // Children in Thing-list order.
        let mut last_child: Vec<Option<u32>> = vec![None; list.len()];
        for i in 0..list.len() {
            let (Some(me), Some(p)) = (handle(i), w.things[i].parent) else {
                continue;
            };
            let Some(pi) = usize::try_from(p).ok().filter(|&p| p < list.len()) else {
                continue;
            };
            match last_child[pi] {
                None => w.things[pi].first_child = Some(me),
                Some(prev) => w.things[prev as usize].next_sibling = Some(me),
            }
            last_child[pi] = Some(me);
        }
        w.world = list
            .iter()
            .position(|t| t.data.world.is_some())
            .and_then(handle);
        for t in list {
            if let Some(part) = &t.data.script {
                for f in &part.fields {
                    collect_objects(&f.value, &mut w.stored_objects);
                }
            }
        }
        for (i, t) in list.iter().enumerate() {
            if let Some(part) = &t.data.script {
                w.add_script(i, part);
            }
        }
        w.wire_inputs();
        w
    }

    /// Builds every Thing's input table from the PSwitch outputs' targets.
    ///
    /// FOUND in the 01.33 executable (the switch-input natives' readers, 0x7541e0 and
    /// its neighbours): a Thing holds a table with one entry per input port, each empty
    /// or the SwitchOutput wired to it. INFERRED: the table is what the outputs' stored
    /// targets say. A target (Thing, port) puts its output on that port of that Thing;
    /// the table is as long as the highest wired port + 1. When two outputs target one
    /// port the first in Thing order (then output and target order) is kept and the
    /// other is counted in `input_collisions`. Targets with a null Thing wire nothing.
    pub fn wire_inputs(&mut self) {
        let mut wires = Vec::new();
        for (i, t) in self.things.iter().enumerate() {
            let outputs = t.switch.iter().flat_map(|s| s.outputs.iter().enumerate());
            for (output, o) in outputs {
                for &(target, port) in o.iter().flat_map(|o| &o.targets) {
                    let wire = InputWire {
                        switch: i as u32,
                        output,
                    };
                    wires.push((target, port, wire));
                }
            }
        }
        for t in &mut self.things {
            t.inputs.clear();
        }
        (self.input_collisions, self.input_ports_dropped) = (0, 0);
        self.switch_order = None;
        for (target, port, wire) in wires {
            let Some(t) = target.and_then(|t| self.things.get_mut(t as usize)) else {
                continue;
            };
            let Some(port) = usize::try_from(port).ok().filter(|&p| p < MAX_INPUT_PORTS) else {
                self.input_ports_dropped += 1;
                continue;
            };
            if t.inputs.len() <= port {
                t.inputs.resize(port + 1, None);
            }
            match &mut t.inputs[port] {
                Some(_) => self.input_collisions += 1,
                empty => *empty = Some(wire),
            }
        }
    }

    /// The wire on input port `port` of a Thing, with the output it leads to.
    fn input(
        &self,
        thing: u32,
        port: i32,
    ) -> Result<Option<(InputWire, &SwitchOutputState)>, String> {
        let inputs = &self.state(thing)?.inputs;
        let wire = usize::try_from(port).ok().and_then(|p| *inputs.get(p)?);
        Ok(wire.and_then(|w| {
            let s = self.things.get(w.switch as usize)?.switch.as_ref()?;
            Some((w, s.outputs.get(w.output)?.as_ref()?))
        }))
    }

    fn add_script(&mut self, i: usize, part: &ScriptPart) {
        let stored = &self.stored_objects;
        let text = |id: usize| match stored.get(&id) {
            Some(ScriptObject::Text(t)) => Some(Value::Str(t.as_str().into())),
            _ => None,
        };
        let mut members = BTreeMap::new();
        for f in &part.fields {
            if let Some((v, converted)) = stored_value(&f.value, &text) {
                if !converted {
                    self.unconverted += 1;
                    if let ScriptValue::Object { id: Some(id), .. } = f.value {
                        self.object_members.push((i as u32, f.name.clone(), id));
                    }
                }
                members.insert(f.name.clone(), v);
            }
        }
        let state = &mut self.things[i];
        state.script_guid = part.script.and_then(|d| d.guid);
        if !part.fields.is_empty() {
            state.members = Some(members);
        }
    }

    /// Builds the level's stored script objects on the VM heap and points the members that
    /// reference them at those objects. Arrays become script arrays; an instance becomes
    /// an instance of the class with its script GUID (class defaults, then the stored
    /// fields); strings stay string values. Objects with one number are one heap object, so
    /// shared references stay shared. Resources, audio handles and instances of classes
    /// the VM has not loaded stay null and are counted in `unconverted`. Returns the
    /// number of heap objects created.
    pub fn bind_objects(&mut self, vm: &mut Vm) -> usize {
        let thing = |t: &Option<usize>| Value::Thing(t.and_then(|i| u32::try_from(i).ok()));
        let mut built = BTreeMap::<usize, Value>::new();
        let mut created = 0;
        // Allocate first so references in any order (and cycles) resolve.
        for (&id, body) in &self.stored_objects {
            let made = match body {
                ScriptObject::Text(t) => {
                    built.insert(id, Value::Str(t.as_str().into()));
                    continue;
                }
                ScriptObject::Bools(v) => vm.new_array(v.iter().map(|&b| Value::Bool(b)).collect()),
                ScriptObject::Ints(v) => vm.new_array(v.iter().map(|&i| Value::Int(i)).collect()),
                ScriptObject::Floats(v) => {
                    vm.new_array(v.iter().map(|&f| Value::Float(f)).collect())
                }
                ScriptObject::Vectors(v) => {
                    vm.new_array(v.iter().map(|&f| Value::Vec4(f)).collect())
                }
                ScriptObject::Things(v) => vm.new_array(v.iter().map(thing).collect()),
                ScriptObject::Objects(_) => vm.new_array(Vec::new()),
                ScriptObject::Instance(part) => {
                    match part
                        .script
                        .and_then(|d| d.guid)
                        .and_then(|g| vm.class_by_guid(g))
                    {
                        Some(class) => vm.instantiate(class),
                        None => continue,
                    }
                }
                ScriptObject::Resource { .. } | ScriptObject::AudioHandle => continue,
            };
            if let Ok(o) = made {
                built.insert(id, Value::Object(Some(o)));
                created += 1;
            }
        }
        let object = |i: usize| built.get(&i).cloned();
        let mut unconverted = 0;
        for (id, body) in &self.stored_objects {
            let Some(Value::Object(Some(o))) = built.get(id) else {
                continue;
            };
            match body {
                ScriptObject::Objects(items) => {
                    let values: Vec<Value> = items
                        .iter()
                        .map(|v| {
                            let (v, ok) =
                                stored_value(v, &object).unwrap_or((Value::Object(None), true));
                            unconverted += usize::from(!ok);
                            v
                        })
                        .collect();
                    if let Some(a) = vm.array_mut(*o) {
                        *a = values;
                    }
                }
                ScriptObject::Instance(part) => {
                    let mut stored = Vec::new();
                    for f in &part.fields {
                        if let Some((v, ok)) = stored_value(&f.value, &object) {
                            unconverted += usize::from(!ok);
                            stored.push((f.name.clone(), v));
                        }
                    }
                    if let Some(fields) = vm.fields_mut(*o) {
                        fields.extend(stored);
                    }
                }
                _ => {}
            }
        }
        for (t, name, id) in std::mem::take(&mut self.object_members) {
            match object(id) {
                Some(v) => {
                    if let Some(m) = self.things[t as usize].members.as_mut() {
                        m.insert(name, v);
                    }
                }
                None => unconverted += 1,
            }
        }
        self.unconverted = unconverted;
        created
    }

    /// Gives every scripted Thing whose class is loaded the fields of its class and
    /// superclasses that the instance does not store, at their type defaults. The stored
    /// layout only carries serialized fields: a superclass field such as
    /// `Tweakable.IsTweaking` (read by `Trigger_Speech_Bubble.Update`) is never stored but
    /// still exists on the instance (INFERRED, the way `instantiate` builds script objects).
    pub fn add_class_defaults(&mut self, vm: &Vm) {
        for t in &mut self.things {
            let Some((guid, class)) = t.script_guid.and_then(|g| Some((g, vm.class_by_guid(g)?)))
            else {
                continue;
            };
            let chain = vm.class_chain(class);
            if chain
                .iter()
                .any(|&c| vm.class_guid(c) == Some(SCORE_BOARD_SCRIPT))
            {
                self.score_board_scripts.insert(guid);
            }
            let Ok(defaults) = vm.field_defaults(class) else {
                continue;
            };
            let members = t.members.get_or_insert_with(BTreeMap::new);
            for (name, v) in defaults {
                members.entry(name).or_insert(v);
            }
        }
    }

    /// Adds a player's Sackboy as a new Thing (YELLOWHEAD, CREATURE, BODY, POS; no parent)
    /// at `position`, covering the z span `layers`. Triggers test players only.
    pub fn add_player(&mut self, position: [f32; 3], layers: (f32, f32)) -> u32 {
        let thing = self.things.len() as u32;
        self.things.push(ThingState {
            parts: vec!["BODY", "POS", "YELLOWHEAD", "CREATURE"],
            ..Default::default()
        });
        // `PYellowHead.PlayerNumber`, read by scripts a player walks into: the player's
        // index (INFERRED: an s32 `Game.EPlayerNumber`, as `Game.GetPlayerNumber__i`).
        let number = Value::Int(self.players.len() as i32);
        let part = self.part_handle(thing, "YELLOWHEAD") as usize - 1;
        self.part_handles[part]
            .members
            .insert("PlayerNumber".into(), number);
        self.players.push(PlayerState { thing, layers });
        self.move_player(thing, position, layers);
        thing
    }

    /// Puts a player's Sackboy at `position` (no rotation) with the z span `layers`.
    pub fn move_player(&mut self, thing: u32, position: [f32; 3], layers: (f32, f32)) {
        let mut m = [0.; 16];
        (m[0], m[5], m[10], m[15]) = (1., 1., 1., 1.);
        m[12..15].copy_from_slice(&position);
        if let Some(t) = self.things.get_mut(thing as usize) {
            t.world_matrix = Some(m);
        }
        if let Some(p) = self.players.iter_mut().find(|p| p.thing == thing) {
            p.layers = layers;
        }
    }

    /// One tick of every PTrigger against the players, in Thing order: updates the
    /// triggers' `inside` lists and returns what happened.
    ///
    /// FOUND in the 01.33 executable (trigger pass 0xe3bc0, per trigger 0x1364f0, test
    /// 0x135fe0, add 0x135a18, remove 0x1346fc):
    /// - The position is taken into the trigger Thing's own space (inverse world matrix),
    ///   so the Thing's scale sizes the trigger. The radius is `radiusMultiplier`, times
    ///   `hysteresisMultiplier` while anything is inside. Type 0: x^2 + y^2 < r^2; type 1:
    ///   |x| < r and |y| < r; type 4: x^2 + y^2 + z^2 < r^2; other types hold nothing.
    /// - A trigger that is not enabled holds nothing.
    /// - Without `allZLayers`, a Sackboy only counts when the trigger's z plus `zOffset`
    ///   lies within the z span of his body.
    /// - A Thing newly inside is appended and raises Enter; one no longer inside is
    ///   removed and raises Leave; then every Thing in the list raises Inside.
    /// - Ordinary triggers test the players' Sackboys only.
    ///
    /// Not modelled: the seven engine object kinds whose triggers test other Things too,
    /// the Sackboy states that never count (creature state 5, a YELLOWHEAD flag), the
    /// world-bounds cull, and the object kinds the engine handles without a script.
    /// INFERRED: the player's point is his Thing position; triggers run in Thing order.
    pub fn update_triggers(&mut self) -> Vec<TriggerEvent> {
        let mut events = Vec::new();
        let players: Vec<(PlayerState, [f32; 3])> = self
            .players
            .iter()
            .filter_map(|p| {
                let m = self.things.get(p.thing as usize)?.world_matrix?;
                Some((*p, [m[12], m[13], m[14]]))
            })
            .collect();
        for (i, t) in self.things.iter_mut().enumerate() {
            let (Some(g), Some(m)) = (t.trigger.as_mut(), t.world_matrix) else {
                continue;
            };
            let trigger = i as u32;
            for (p, pos) in &players {
                let radius = if g.inside.is_empty() {
                    g.radius
                } else {
                    g.radius * g.hysteresis
                };
                let z = m[14] + g.z_offset;
                let inside = g.enabled
                    && (g.all_z_layers || (p.layers.0..=p.layers.1).contains(&z))
                    && to_local(&m, *pos).is_some_and(|l| match g.trigger_type {
                        0 => l[0] * l[0] + l[1] * l[1] < radius * radius,
                        1 => l[0].abs() < radius && l[1].abs() < radius,
                        4 => l[0] * l[0] + l[1] * l[1] + l[2] * l[2] < radius * radius,
                        _ => false,
                    });
                let at = g.inside.iter().position(|x| *x == p.thing);
                let kind = match (inside, at) {
                    (true, None) => {
                        g.inside.push(p.thing);
                        TriggerEventKind::Enter
                    }
                    (false, Some(at)) => {
                        g.inside.remove(at);
                        TriggerEventKind::Leave
                    }
                    _ => continue,
                };
                events.push(TriggerEvent {
                    trigger,
                    kind,
                    thing: p.thing,
                });
            }
            events.extend(g.inside.iter().map(|&thing| TriggerEvent {
                trigger,
                kind: TriggerEventKind::Inside,
                thing,
            }));
        }
        events
    }

    /// Handle for a part a Thing carries (index + 1), created on first use.
    fn part_handle(&mut self, thing: u32, part: &'static str) -> u32 {
        if let Some(i) = self
            .part_handles
            .iter()
            .position(|p| p.thing == thing && p.part == part)
        {
            return i as u32 + 1;
        }
        self.part_handles.push(PartState {
            thing,
            part,
            members: BTreeMap::new(),
        });
        self.part_handles.len() as u32
    }

    fn part_of(&mut self, base: &Value) -> Result<&mut PartState, String> {
        match base {
            Value::Raw(0) => Err("null part".into()),
            Value::Raw(h) => self
                .part_handles
                .get_mut(*h as usize - 1)
                .ok_or_else(|| format!("no part handle {h}")),
            other => Err(format!("member base is not a part: {other:?}")),
        }
    }

    /// `thing.SpriteLight` and the like: the part as a raw pointer, null when the Thing
    /// lacks it. Only names of the reader's part list are parts.
    fn native_member(&mut self, base: &Value, class: &str, field: &str) -> Result<Value, String> {
        let want = part_name(field);
        let Some(&(part, ..)) = lbp_level::thing::PARTS.iter().find(|p| p.0 == want) else {
            return Err(format!(
                "world does not provide native member {class}.{field}"
            ));
        };
        let t = Self::thing_of(base)?;
        Ok(if self.state(t)?.parts.contains(&part) {
            Value::Raw(self.part_handle(t, part))
        } else {
            Value::Raw(0)
        })
    }

    fn state(&self, thing: u32) -> Result<&ThingState, String> {
        self.things
            .get(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))
    }

    fn thing_mut(&mut self, thing: u32) -> Result<&mut ThingState, String> {
        self.things
            .get_mut(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))
    }

    fn thing_of(base: &Value) -> Result<u32, String> {
        match base {
            Value::Thing(Some(t)) => Ok(*t),
            Value::Thing(None) => Err("null thing".into()),
            other => Err(format!("member base is not a thing: {other:?}")),
        }
    }
}

impl World for LevelWorld {
    fn thing_uid(&mut self, thing: u32) -> Result<i32, String> {
        self.state(thing)?
            .uid
            .ok_or_else(|| format!("thing {thing} has no UID"))
    }
    fn parent(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.parent)
    }
    fn first_child(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.first_child)
    }
    fn next_sibling(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.next_sibling)
    }
    fn world_thing(&mut self) -> Result<Option<u32>, String> {
        Ok(self.world)
    }
    /// Translation column of the POS world matrix (INFERRED: w as stored, normally 1).
    fn world_position(&mut self, thing: u32) -> Result<[f32; 4], String> {
        let m = self
            .state(thing)?
            .world_matrix
            .ok_or_else(|| format!("thing {thing} has no position"))?;
        Ok([m[12], m[13], m[14], m[15]])
    }
    fn has_part(&mut self, thing: u32, part: &str) -> Result<bool, String> {
        let want = part_name(part);
        Ok(self.state(thing)?.parts.iter().any(|p| *p == want))
    }
    fn trigger_count(&mut self, thing: u32) -> Result<i32, String> {
        let t = self.state(thing)?;
        Ok(t.trigger.as_ref().map_or(0, |g| g.inside.len() as i32))
    }
    fn trigger_thing(&mut self, thing: u32, index: i32) -> Result<Option<u32>, String> {
        let inside = self.state(thing)?.trigger.as_ref().map(|g| &g.inside);
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| inside?.get(i).copied()))
    }
    /// The current signal of the output wired to the port ([`LevelWorld::wire_inputs`]),
    /// as the switch pass last wrote it.
    fn switch_input(&mut self, thing: u32, port: i32) -> Result<Option<SwitchInput>, String> {
        Ok(self.input(thing, port)?.map(|(_, o)| o.signal))
    }
    fn switch_input_thing(&mut self, thing: u32, port: i32) -> Result<Option<u32>, String> {
        Ok(self.input(thing, port)?.map(|(w, _)| w.switch))
    }
    fn switch_output_target(
        &mut self,
        thing: u32,
        output: i32,
        target: i32,
    ) -> Result<Option<u32>, String> {
        let switch = self.state(thing)?.switch.as_ref();
        let (Ok(o), Ok(t)) = (usize::try_from(output), usize::try_from(target)) else {
            return Ok(None);
        };
        Ok(switch
            .and_then(|s| s.outputs.get(o)?.as_ref())
            .and_then(|o| o.targets.get(t)?.0))
    }
    /// The Thing's script is the scoreboard script, or a loaded class derived from it
    /// (known once [`LevelWorld::add_class_defaults`] ran).
    fn is_score_board(&mut self, thing: u32) -> Result<bool, String> {
        Ok(self
            .state(thing)?
            .script_guid
            .is_some_and(|g| g == SCORE_BOARD_SCRIPT || self.score_board_scripts.contains(&g)))
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        match kind {
            // Through `Thing` or a script class of a Thing (`World.World`).
            MemberKind::Native => return self.native_member(base, class, field),
            MemberKind::RefPtr => {
                let p = self.part_of(base)?;
                return p
                    .members
                    .get(field)
                    .cloned()
                    .ok_or_else(|| format!("{} member {class}.{field} not read yet", p.part));
            }
            MemberKind::Thing => {}
        }
        let t = Self::thing_of(base)?;
        self.state(t)?
            .members
            .as_ref()
            .and_then(|m| m.get(field))
            .cloned()
            .ok_or_else(|| format!("thing {t} has no script member {class}.{field}"))
    }
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        if kind == MemberKind::RefPtr {
            self.part_of(base)?.members.insert(field.to_owned(), value);
            return Ok(());
        }
        if kind != MemberKind::Thing {
            return Err(format!(
                "world does not provide {kind:?} member {class}.{field}"
            ));
        }
        let t = Self::thing_of(base)?;
        let state = self
            .things
            .get_mut(t as usize)
            .ok_or_else(|| format!("no thing {t}"))?;
        state
            .members
            .get_or_insert_with(BTreeMap::new)
            .insert(field.to_owned(), value);
        Ok(())
    }
    /// One local player (the PC run has no controllers yet).
    fn num_players(&mut self) -> Result<i32, String> {
        Ok(1)
    }
    /// Play mode (0); levels run as played, not edited.
    fn play_or_edit_mode(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    /// No dialogue list is loaded (0).
    fn dialogue_list(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    fn shape_editor_colour(&mut self, thing: u32) -> Result<[f32; 4], String> {
        self.state(thing)?
            .shape_colour
            .ok_or_else(|| format!("thing {thing} has no shape"))
    }
    fn set_shape_editor_colour(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        let t = self
            .things
            .get_mut(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))?;
        if t.shape_colour.is_none() {
            return Err(format!("thing {thing} has no shape"));
        }
        t.shape_colour = Some(colour);
        Ok(())
    }
    /// Recorded on the Thing; nothing draws it yet. Any Thing may be tinted (scripts call
    /// it on their own Thing without a `HasPRenderMesh` check).
    fn set_render_mesh_tint(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        self.thing_mut(thing)?.render_mesh_tint = Some(colour);
        Ok(())
    }
    /// Recorded on the Thing; nothing animates by it yet.
    fn set_anim_speed(&mut self, thing: u32, speed: f32) -> Result<(), String> {
        self.thing_mut(thing)?.anim_speed = Some(speed);
        Ok(())
    }
    /// Player `index` is `Game.EPlayerNumber` `index` (INFERRED: no enum values are in the
    /// script tables); outside `0..num_players()` is an error.
    fn player_number(&mut self, index: i32) -> Result<i32, String> {
        let n = self.num_players()?;
        if (0..n).contains(&index) {
            Ok(index)
        } else {
            Err(format!("player index {index} outside 0..{n}"))
        }
    }
    /// The handle is the GUID itself (INFERRED stand-in: nothing is loaded; scripts only
    /// keep the handle and pass it back to natives).
    fn load_resource(&mut self, _kind: ResourceKind, guid: u32) -> Result<u32, String> {
        Ok(guid)
    }
    fn set_world_transform(&mut self, thing: u32, m: [f32; 16]) -> Result<(), String> {
        self.thing_mut(thing)?.world_matrix = Some(m);
        Ok(())
    }
    fn set_anim_pos(&mut self, thing: u32, pos: f32) -> Result<(), String> {
        self.thing_mut(thing)?.anim_pos = Some(pos);
        Ok(())
    }
    fn pod_computer(&mut self) -> Result<Value, String> {
        self.pod_computer
            .map(|o| Value::Object(Some(o)))
            .ok_or_else(|| "no PodComputer object (class not loaded)".into())
    }
    fn planets(&mut self) -> Result<Value, String> {
        self.planets
            .map(|o| Value::Object(Some(o)))
            .ok_or_else(|| "no Planets object (class not loaded)".into())
    }
    fn crafteroid(&mut self, index: i32) -> Result<Value, String> {
        usize::try_from(index)
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.crafteroids.get(i))
            .map(|(o, _)| Value::Object(Some(*o)))
            .ok_or_else(|| format!("no crafteroid {index}"))
    }
    fn planet_thing(&mut self, planet: &Value) -> Result<Option<u32>, String> {
        match planet {
            Value::Object(Some(o)) => self
                .crafteroids
                .iter()
                .find(|(p, _)| p == o)
                .map(|(_, t)| *t)
                .ok_or_else(|| format!("object {o:?} is not a planet")),
            other => Err(format!("expected a Planet, got {other:?}")),
        }
    }
    /// INFERRED defaults for the PC run: interactive, no pending links, no avatars spawned
    /// (there is no Sackboy yet), no video, a 16:9 screen, player 0 leads, no music track
    /// (GUID 0), no user music (stopped, not paused), interactive music audible, no
    /// attraction (demo) loop.
    fn state_query(&mut self, native: &str) -> Result<Value, String> {
        Ok(match native {
            "Game.IsNonInteractive__"
            | "Video.IsVideoPlaying__"
            | "System.Is4to3__"
            | "Audio.UserMusicIsPaused__"
            | "Input.IsInAttractionLoop__i" => Value::Bool(false),
            "Audio.UserMusicIsStopped__" | "Audio.IsInteractiveMusicAudible__" => Value::Bool(true),
            "Game.GetNumPendingLinks__"
            | "Game.GetNumAvatars__"
            | "Game.GetLocalLeadersPlayerNumber__"
            | "Audio.GetCurrMusicGuid__" => Value::Int(0),
            _ => return Err(format!("world does not provide {native}")),
        })
    }
    /// Counted by name; nothing plays.
    fn audio_command(&mut self, name: &str, _args: &[Value]) -> Result<(), String> {
        *self.audio_commands.entry(name.to_owned()).or_default() += 1;
        Ok(())
    }
    /// Flags in [`LevelWorld::progression_flags`]; empty is a fresh profile (default).
    fn progression_flag(&mut self, flag: i32) -> Result<bool, String> {
        Ok(self.progression_flags.contains(&flag))
    }
    /// [`LevelWorld::pod_computer_in_use`].
    fn pod_computer_in_use(&mut self) -> Result<bool, String> {
        Ok(self.pod_computer_in_use)
    }
    /// No crash zoom in the PC run (INFERRED default).
    fn crash_zoom(&mut self) -> Result<bool, String> {
        Ok(false)
    }
    fn create_gooey(&mut self, _arg: i32) -> Result<Value, String> {
        let o = self
            .gooey_pool
            .get(self.gooeys_created)
            .copied()
            .ok_or_else(|| format!("Gooey pool of {} used up", self.gooey_pool.len()))?;
        self.gooeys_created += 1;
        Ok(Value::Object(Some(o)))
    }
    fn set_camera(&mut self, camera: CameraFromToUp) -> Result<(), String> {
        self.camera.from_to_up = Some(camera);
        self.camera.from_to_up_calls += 1;
        Ok(())
    }
    fn set_dof_and_fog(&mut self, values: [f32; 3]) -> Result<(), String> {
        self.camera.dof_and_fog = Some(values);
        Ok(())
    }
    fn clear_motion_blur(&mut self) -> Result<(), String> {
        self.camera.motion_blur_clears += 1;
        Ok(())
    }
    /// No cinema mode in the PC run (INFERRED default).
    fn cinema_mode_count(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    fn set_score_multiplier_timer(&mut self, player: i32, timer: i32) -> Result<(), String> {
        let n = self.num_players()?;
        if !(0..n).contains(&player) {
            return Err(format!("player {player} outside 0..{n}"));
        }
        self.score_multiplier_timers.insert(player, timer);
        Ok(())
    }
    fn write(&mut self, text: &str) {
        if self.written.len() < 1000 {
            self.written.push(text.to_owned());
        }
    }
}

/// Gooey objects made ahead for `Gooey.CreateGooey__i`.
pub const GOOEY_POOL: usize = 16;

/// Runs `function` (`.init__`, `.ctor__`) on a heap object with it as `this` in register 0;
/// a fault comes back as text with its site.
fn run_on_object(
    vm: &mut Vm,
    host: &mut dyn Host,
    class: ClassId,
    object: ObjectId,
    function: &str,
    budget: u64,
) -> Result<(), String> {
    if !vm.has_code(class, function) {
        return Ok(());
    }
    let mut b = budget;
    let regs = [(0, Value::Object(Some(object)))];
    match vm.start_registers(host, class, function, &regs, &mut b) {
        Ok(Step::Done(_)) => Ok(()),
        Ok(Step::Yielded(_)) => Err(format!("{function} yielded")),
        Err(f) => {
            let site = vm.last_site().map_or(String::new(), |s| s.to_string());
            Err(format!("{function}: {f:?} at {site}"))
        }
    }
}

/// Makes the engine-owned script objects natives hand out: [`GOOEY_POOL`] Gooey objects
/// (`.init__`) and the PodComputer (`.init__`, then `.ctor__`). INFERRED: the engine holds
/// one PodComputer and creates Gooeys on demand. The PodComputer is registered before its
/// `.init__` runs, because that code already asks for it (`GetPodComputer__`). Returns the
/// faults, if any; objects whose `.init__`/`.ctor__` faulted are still kept.
pub fn add_engine_objects(
    vm: &mut Vm,
    host: &mut Bindings<LevelWorld>,
    budget: u64,
) -> Vec<String> {
    let mut faults = Vec::new();
    let make = |vm: &mut Vm, host: &mut Bindings<LevelWorld>, class, pod: bool| {
        let o = vm.instantiate(class).map_err(|f| format!("{f:?}"))?;
        if pod {
            host.world.pod_computer = Some(o);
        } else {
            host.world.gooey_pool.push(o);
        }
        run_on_object(vm, host, class, o, ".init__", budget)?;
        if pod {
            run_on_object(vm, host, class, o, ".ctor__", budget)?;
        }
        Ok::<(), String>(())
    };
    if let Some(class) = vm.class_by_name("Gooey") {
        for _ in 0..GOOEY_POOL {
            if let Err(e) = make(vm, host, class, false) {
                faults.push(format!("Gooey {e}"));
            }
        }
    }
    // Planets and its crafteroids: INFERRED index 1 = the level's Earth (a PlanetThing),
    // 2 = its OrbitingMoon, from PlanetsView, which reads crafteroids 1 and 2 and returns
    // early when either Thing is null.
    if let Some(class) = vm.class_by_name("Planets") {
        match vm.instantiate(class) {
            Ok(o) => {
                host.world.planets = Some(o);
                if let Err(e) = run_on_object(vm, host, class, o, ".init__", budget) {
                    faults.push(format!("Planets {e}"));
                }
            }
            Err(f) => faults.push(format!("Planets {f:?}")),
        }
    }
    if let Some(class) = vm.class_by_name("Planet") {
        for name in ["Earth", "OrbitingMoon"] {
            let thing = host.world.things.iter().position(|t| {
                t.script_guid
                    .and_then(|g| vm.class_by_guid(g))
                    .is_some_and(|c| vm.class_name(c) == name)
            });
            match vm.instantiate(class) {
                Ok(o) => host
                    .world
                    .crafteroids
                    .push((o, thing.and_then(|t| u32::try_from(t).ok()))),
                Err(f) => faults.push(format!("Planet {f:?}")),
            }
        }
    }
    if let Some(class) = vm.class_by_name("PodComputer") {
        if let Err(e) = make(vm, host, class, true) {
            faults.push(format!("PodComputer {e}"));
        }
    }
    faults.dedup();
    faults
}

/// A scripted Thing and its class, when the class is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instance {
    pub thing: u32,
    pub guid: u32,
    pub class: Option<ClassId>,
}

/// Every Thing with a PScript resource GUID, in Thing order.
pub fn instances(vm: &Vm, world: &LevelWorld) -> Vec<Instance> {
    world
        .things
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            let guid = t.script_guid?;
            Some(Instance {
                thing: u32::try_from(i).ok()?,
                guid,
                class: vm.class_by_guid(guid),
            })
        })
        .collect()
}

/// Result of starting or resuming one script.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The class (or its superclasses) has no code for the function.
    NoFunction,
    /// The script's class is not loaded.
    NoClass,
    Done,
    Yielded,
    Fault(String),
}

/// Runs entry functions on scripted Things and keeps yielded ones for later ticks.
#[derive(Default)]
pub struct Runner {
    tasks: Vec<(u32, Task)>,
}

impl Runner {
    /// Scripts suspended by `System.Yield`, waiting for the next tick.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }

    /// Things with a suspended script.
    pub fn pending_things(&self) -> BTreeSet<u32> {
        self.tasks.iter().map(|(t, _)| *t).collect()
    }

    /// Runs `function` on one instance with the Thing as `this` in register 0.
    pub fn start(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        inst: Instance,
        function: &str,
        budget: &mut u64,
    ) -> Outcome {
        self.start_with(vm, host, inst, function, &[], budget)
    }

    /// [`Runner::start`] with argument registers (byte offsets) staged after `this`.
    pub fn start_with(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        inst: Instance,
        function: &str,
        args: &[(u16, Value)],
        budget: &mut u64,
    ) -> Outcome {
        let Some(class) = inst.class else {
            return Outcome::NoClass;
        };
        if !vm.has_code(class, function) {
            return Outcome::NoFunction;
        }
        let mut regs = vec![(0, Value::Thing(Some(inst.thing)))];
        regs.extend_from_slice(args);
        let step = vm.start_registers(host, class, function, &regs, budget);
        self.settle(vm, inst.thing, step)
    }

    /// Resumes every yielded script once, in the order they yielded.
    pub fn tick(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        budget: &mut u64,
    ) -> Vec<(u32, Outcome)> {
        let tasks = std::mem::take(&mut self.tasks);
        let mut out = Vec::with_capacity(tasks.len());
        for (thing, task) in tasks {
            let step = vm.resume(host, task, budget);
            out.push((thing, self.settle(vm, thing, step)));
        }
        out
    }

    fn settle(&mut self, vm: &Vm, thing: u32, step: Result<Step, lbp_script_vm::Fault>) -> Outcome {
        match step {
            Ok(Step::Done(_)) => Outcome::Done,
            Ok(Step::Yielded(task)) => {
                self.tasks.push((thing, task));
                Outcome::Yielded
            }
            Err(f) => {
                let site = vm.last_site().map_or(String::new(), |s| s.to_string());
                Outcome::Fault(format!("{f:?} at {site}"))
            }
        }
    }
}

/// Function the game runs once when a scripted Thing is created.
pub const CREATE: &str = "OnCreate__";
/// Function the game runs on every scripted Thing each tick.
pub const UPDATE: &str = "Update__";
/// What a started script thread runs (`Thread.ThreadMain__`: OnStart, Run, OnFinish).
pub const THREAD_MAIN: &str = "ThreadMain__";
/// Thread starts handled in one step (threads can start threads).
pub const MAX_THREAD_STARTS: usize = 256;
/// Game logic rate (X1, CONFIRMED_FROM_LOCAL_DATA in RPCS3: 30 ticks per second).
pub const TICK_HZ: u32 = 30;

/// What a [`FrameEntry`] builds its arguments from.
pub struct FrameContext<'a> {
    pub vm: &'a Vm,
    pub instances: &'a [Instance],
    /// Ticks run before this one.
    pub tick: u64,
}

/// A per-tick function the engine calls with arguments on instances of one class, in
/// place of [`UPDATE`] (an `export` entry no script calls).
pub struct FrameEntry {
    /// Exact script class name of the instance.
    pub class: &'static str,
    pub function: &'static str,
    /// Argument registers (byte offsets) and values; `this` is register 0.
    pub args: fn(&FrameContext<'_>) -> Vec<(u16, Value)>,
}

/// PlanetsWorld (the Pod's planet view): no script calls the export
/// `UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb`, and it is the only caller of
/// `UpdateState__` (private), `UpdatePodCamera__Q11PlanetThingbb` and
/// `UpdateInteractiveMusic__`, so the engine calls it each frame (INFERRED from the
/// listings; `OnEnterPod__`/`OnExitPod__` are called by PodComputer on pod entry/exit).
/// Arguments (INFERRED from how the function uses them): +4 bool, passed on to
/// `UpdatePodCamera` (false); +8 the Earth (a PlanetThing; it gets `EarthSpinOffset`); +12 the
/// moon (`MoonSpinOffset`); +16 bool "snap the view" (true on the first tick only); +17
/// bool "take the zoom from the pod computer's state" (true).
fn planets_view_args(cx: &FrameContext<'_>) -> Vec<(u16, Value)> {
    let find = |name: &str| {
        cx.instances
            .iter()
            .find(|i| i.class.is_some_and(|c| cx.vm.class_name(c) == name))
            .map(|i| i.thing)
    };
    vec![
        (4, Value::Bool(false)),
        (8, Value::Thing(find("Earth"))),
        (12, Value::Thing(find("OrbitingMoon"))),
        (16, Value::Bool(cx.tick == 0)),
        (17, Value::Bool(true)),
    ]
}

/// Per-tick entries other than [`UPDATE`], by class (docs/level-scripts.md "Part 6").
pub const FRAME_ENTRIES: &[FrameEntry] = &[FrameEntry {
    class: "PlanetsWorld",
    function: "UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb",
    args: planets_view_args,
}];

/// The per-tick entry for a class, if it has one other than [`UPDATE`].
pub fn frame_entry(vm: &Vm, class: ClassId) -> Option<&'static FrameEntry> {
    let name = vm.class_name(class);
    FRAME_ENTRIES.iter().find(|e| e.class == name)
}

/// Outcomes of one schedule step, by kind, with the faults.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepReport {
    pub done: usize,
    pub yielded: usize,
    pub resumed: usize,
    pub no_function: usize,
    pub no_class: usize,
    pub faults: Vec<(u32, String)>,
    /// Script threads started, finished (`ThreadMain__` returned) and faulted this step.
    pub threads_started: usize,
    pub threads_done: usize,
    pub thread_faults: Vec<String>,
    /// Trigger events raised, by [`TriggerEventKind`] (Enter, Inside, Leave).
    pub trigger_events: [usize; 3],
    /// Trigger events whose script function exists and was run, by kind.
    pub events_run: [usize; 3],
    /// Trigger events with no script to call (none on the trigger's Thing or its parent).
    pub events_without_script: usize,
    /// Trigger events for a Thing already stopped by a fault.
    pub events_to_stopped: usize,
    /// What the switch pass did ([`LevelScripts::switch_reactions`]).
    pub reactions: ReactionReport,
}

/// Counts of one or more switch passes. Unit: a visited output target (one Thing on one
/// port, once per edge); the maps are by script resource GUID of the target.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReactionReport {
    /// Output targets the pass visited ([`output_edge`]).
    pub targets_visited: usize,
    /// Visits with `on` true.
    pub targets_on: usize,
    /// `SwitchPerformReaction__b` calls made.
    pub calls_b: BTreeMap<u32, usize>,
    /// `SwitchPerformReaction__bf` calls made.
    pub calls_bf: BTreeMap<u32, usize>,
    /// Visited targets with no script.
    pub without_script: usize,
    /// Visited targets whose script is not on the list ([`reaction_for_script`]).
    pub not_whitelisted: BTreeMap<u32, usize>,
    /// Listed scripts whose Thing gets its object kind from a part tested before the
    /// script ([`kind_before_script`]); no call.
    pub shadowed_by_switch: BTreeMap<u32, usize>,
    /// Listed scripts whose class is not loaded or has no code for the function.
    pub no_function: BTreeMap<u32, usize>,
    /// Listed scripts already stopped by a fault.
    pub to_stopped: BTreeMap<u32, usize>,
}

impl ReactionReport {
    /// Adds `other`'s counts to this report.
    pub fn absorb(&mut self, other: ReactionReport) {
        self.targets_visited += other.targets_visited;
        self.targets_on += other.targets_on;
        self.without_script += other.without_script;
        for (mine, theirs) in [
            (&mut self.calls_b, other.calls_b),
            (&mut self.calls_bf, other.calls_bf),
            (&mut self.not_whitelisted, other.not_whitelisted),
            (&mut self.shadowed_by_switch, other.shadowed_by_switch),
            (&mut self.no_function, other.no_function),
            (&mut self.to_stopped, other.to_stopped),
        ] {
            for (guid, n) in theirs {
                *mine.entry(guid).or_default() += n;
            }
        }
    }
}

impl StepReport {
    /// Adds `other`'s counts and faults to this report.
    pub fn absorb(&mut self, other: StepReport) {
        self.done += other.done;
        self.yielded += other.yielded;
        self.resumed += other.resumed;
        self.no_function += other.no_function;
        self.no_class += other.no_class;
        self.faults.extend(other.faults);
        self.threads_started += other.threads_started;
        self.threads_done += other.threads_done;
        self.thread_faults.extend(other.thread_faults);
        for (a, b) in self.trigger_events.iter_mut().zip(other.trigger_events) {
            *a += b;
        }
        for (a, b) in self.events_run.iter_mut().zip(other.events_run) {
            *a += b;
        }
        self.events_without_script += other.events_without_script;
        self.events_to_stopped += other.events_to_stopped;
        self.reactions.absorb(other.reactions);
    }

    fn add(&mut self, thing: u32, o: Outcome) {
        match o {
            Outcome::Done => self.done += 1,
            Outcome::Yielded => self.yielded += 1,
            Outcome::NoFunction => self.no_function += 1,
            Outcome::NoClass => self.no_class += 1,
            Outcome::Fault(f) => self.faults.push((thing, f)),
        }
    }
}

/// Per-tick script schedule for one level.
///
/// INFERRED (docs/level-scripts.md): `create` runs [`CREATE`] on every instance in Thing
/// order; each `tick` first resumes scripts suspended by `System.Yield` (in the order
/// they yielded), then runs [`UPDATE`] on every instance that was not suspended at the
/// start of the tick, in Thing order. A Thing whose script faults is not run again.
/// A class listed in [`FRAME_ENTRIES`] runs that entry with its arguments instead of
/// [`UPDATE`].
///
/// Script threads (`Thread.Start__` -> `Thread.StartImpl__(this)`, docs/script-vm.md):
/// a thread started during a step runs `ThreadMain__` on the thread object at the end of
/// that step, until its first `System.Yield`; each later tick resumes it once, after the
/// Things' suspended scripts and before the updates (order INFERRED). A faulted thread
/// is dropped.
pub struct LevelScripts {
    pub instances: Vec<Instance>,
    runner: Runner,
    /// Suspended script threads, in start order.
    threads: Vec<(ObjectId, Task)>,
    /// Things stopped by a fault.
    pub stopped: BTreeSet<u32>,
    /// Ticks run so far.
    pub ticks: u64,
}

impl LevelScripts {
    pub fn new(vm: &Vm, world: &LevelWorld) -> Self {
        Self {
            instances: instances(vm, world),
            runner: Runner::default(),
            threads: Vec::new(),
            stopped: BTreeSet::new(),
            ticks: 0,
        }
    }

    /// Scripts suspended by `System.Yield`.
    pub fn pending(&self) -> usize {
        self.runner.pending()
    }

    /// Script threads waiting for the next tick.
    pub fn threads(&self) -> usize {
        self.threads.len()
    }

    /// Runs [`CREATE`] on every instance; `budget` is per call.
    pub fn create(&mut self, vm: &mut Vm, host: &mut dyn Host, budget: u64) -> StepReport {
        let mut report = self.run_all(vm, host, Some(CREATE), budget, &BTreeSet::new());
        self.start_threads(vm, host, budget, &mut report);
        report
    }

    fn settle_thread(
        &mut self,
        vm: &Vm,
        id: ObjectId,
        step: Result<Step, lbp_script_vm::Fault>,
        report: &mut StepReport,
    ) {
        match step {
            Ok(Step::Done(_)) => report.threads_done += 1,
            Ok(Step::Yielded(task)) => self.threads.push((id, task)),
            Err(f) => {
                let class = vm.class_of(id).map_or("?", |c| vm.class_name(c));
                let site = vm.last_site().map_or(String::new(), |s| s.to_string());
                report
                    .thread_faults
                    .push(format!("thread {class}: {f:?} at {site}"));
            }
        }
    }

    /// Runs threads started since the last step until their first yield; threads they
    /// start run too, up to [`MAX_THREAD_STARTS`] per step.
    fn start_threads(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        budget: u64,
        report: &mut StepReport,
    ) {
        let mut started = 0;
        loop {
            let new = vm.take_started_threads();
            if new.is_empty() {
                break;
            }
            for id in new {
                started += 1;
                if started > MAX_THREAD_STARTS {
                    report
                        .thread_faults
                        .push("thread start limit reached".into());
                    return;
                }
                report.threads_started += 1;
                let Some(class) = vm.class_of(id) else {
                    report
                        .thread_faults
                        .push(format!("thread object {} has no class", id.0));
                    continue;
                };
                let mut b = budget;
                let step = vm.start_registers(
                    host,
                    class,
                    THREAD_MAIN,
                    &[(0, Value::Object(Some(id)))],
                    &mut b,
                );
                self.settle_thread(vm, id, step, report);
            }
        }
    }

    /// Runs the level's triggers against the players ([`LevelWorld::update_triggers`]) and
    /// calls the script function of each event with the Thing as its argument.
    ///
    /// FOUND in the 01.33 executable (0x135a18, 0x1346fc, 0x1364f0): Enter and Leave go
    /// to the script of the trigger's Thing, or to its parent's when that Thing has no
    /// script; Inside goes to the parent's script. (There the engine calls Inside for four
    /// object kinds only; here every parent script that has the function gets it.)
    /// INFERRED: events are delivered in trigger order.
    pub fn trigger_events(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = StepReport::default();
        for e in host.world.update_triggers() {
            report.trigger_events[e.kind as usize] += 1;
            let instance = |thing: u32| {
                let i = self.instances.binary_search_by_key(&thing, |i| i.thing);
                i.ok().map(|i| self.instances[i])
            };
            let parent = host
                .world
                .things
                .get(e.trigger as usize)
                .and_then(|t| t.parent)
                .and_then(instance);
            let target = match e.kind {
                TriggerEventKind::Inside => parent,
                _ => instance(e.trigger).or(parent),
            };
            let Some(inst) = target else {
                report.events_without_script += 1;
                continue;
            };
            if self.stopped.contains(&inst.thing) {
                report.events_to_stopped += 1;
                continue;
            }
            let mut b = budget;
            let args = [(4, Value::Thing(Some(e.thing)))];
            let o = self
                .runner
                .start_with(vm, host, inst, e.kind.function(), &args, &mut b);
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(inst.thing);
            }
            if !matches!(o, Outcome::NoFunction | Outcome::NoClass) {
                report.events_run[e.kind as usize] += 1;
            }
            report.add(inst.thing, o);
        }
        self.start_threads(vm, host, budget, &mut report);
        report
    }

    /// The per-tick switch pass ([`LevelWorld::switch_order`], [`LevelWorld::update_switch`])
    /// with the script reaction of each visited target.
    ///
    /// FOUND in the 01.33 executable (0x1244a4 -> 0x11ddf0): a switch's targets get the
    /// signal while that switch is updated, before the next switch in the order, so a
    /// reaction script sees later switches' outputs as they were last tick. The call goes
    /// to the script of the target Thing itself (no parent walk; no script, no call), and
    /// only for the scripts [`reaction_for_script`] lists:
    /// `SwitchPerformReaction__b(on)` with `on` from [`output_edge`], or for one script
    /// `SwitchPerformReaction__bf(on, level)`, `level` 1.0 when the output's new ternary
    /// is not 0, else 0.0 (it is not the activation).
    /// INFERRED: the argument registers (+4 bool, +8 float), as for the other entries.
    pub fn switch_reactions(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = StepReport::default();
        host.world.prepare_switches();
        for switch in host.world.switch_order().to_vec() {
            for v in host.world.update_switch(switch) {
                let r = &mut report.reactions;
                r.targets_visited += 1;
                r.targets_on += usize::from(v.on);
                let target = host.world.things.get(v.target as usize);
                let Some(guid) = target.and_then(|t| t.script_guid) else {
                    r.without_script += 1;
                    continue;
                };
                let Some(kind) = reaction_for_script(guid) else {
                    *r.not_whitelisted.entry(guid).or_default() += 1;
                    continue;
                };
                if target.is_some_and(|t| kind_before_script(t).is_some()) {
                    *r.shadowed_by_switch.entry(guid).or_default() += 1;
                    continue;
                }
                let found = self.instances.binary_search_by_key(&v.target, |i| i.thing);
                let Ok(inst) = found.map(|i| self.instances[i]) else {
                    *r.no_function.entry(guid).or_default() += 1;
                    continue;
                };
                if self.stopped.contains(&v.target) {
                    *r.to_stopped.entry(guid).or_default() += 1;
                    continue;
                }
                let level = f32::from(u8::from(v.signal.ternary != 0));
                let (function, args) = match kind {
                    Reaction::Bool => (REACTION_B, vec![(4, Value::Bool(v.on))]),
                    Reaction::BoolFloat => (
                        REACTION_BF,
                        vec![(4, Value::Bool(v.on)), (8, Value::Float(level))],
                    ),
                };
                let mut b = budget;
                let o = self
                    .runner
                    .start_with(vm, host, inst, function, &args, &mut b);
                let r = &mut report.reactions;
                let count = match (&o, kind) {
                    (Outcome::NoFunction | Outcome::NoClass, _) => &mut r.no_function,
                    (_, Reaction::Bool) => &mut r.calls_b,
                    (_, Reaction::BoolFloat) => &mut r.calls_bf,
                };
                *count.entry(guid).or_default() += 1;
                if matches!(o, Outcome::Fault(_)) {
                    self.stopped.insert(inst.thing);
                }
                report.add(inst.thing, o);
            }
        }
        self.start_threads(vm, host, budget, &mut report);
        report
    }

    /// One game tick of a level: [`Self::tick`], then [`Self::trigger_events`], so a
    /// script an event suspends is resumed on the next tick (order INFERRED), then
    /// [`Self::switch_reactions`].
    ///
    /// FOUND in the 01.33 executable: the world update (0xeacd4) calls the switch pass
    /// (0xdd228, at 0xeb270) after 0xe0f0c, 0xcfc40 and 0xcfbb4 and before 0xcfed4, and
    /// not while the game-wide pause test (0x18cf18) holds. Which of those neighbours are
    /// the trigger pass and the script update is not established, so the place of the
    /// switch pass after both here is INFERRED.
    pub fn tick_level(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = self.tick(vm, host, budget);
        report.absorb(self.trigger_events(vm, host, budget));
        report.absorb(self.switch_reactions(vm, host, budget));
        report
    }

    /// One game tick; `budget` is per call or resume.
    pub fn tick(&mut self, vm: &mut Vm, host: &mut dyn Host, budget: u64) -> StepReport {
        // A Thing suspended at the start of the tick is resumed, not updated again.
        let busy = self.runner.pending_things();
        let mut resume_budget = budget.saturating_mul(self.runner.pending() as u64);
        let mut report = StepReport::default();
        for (thing, o) in self.runner.tick(vm, host, &mut resume_budget) {
            if matches!(o, Outcome::Done | Outcome::Yielded) {
                report.resumed += 1;
            }
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(thing);
                report.add(thing, o);
            }
        }
        for (id, task) in std::mem::take(&mut self.threads) {
            let mut b = budget;
            let step = vm.resume(host, task, &mut b);
            self.settle_thread(vm, id, step, &mut report);
        }
        let update = self.run_all(vm, host, None, budget, &busy);
        report.done += update.done;
        report.yielded += update.yielded;
        report.no_function += update.no_function;
        report.no_class += update.no_class;
        report.faults.extend(update.faults);
        self.start_threads(vm, host, budget, &mut report);
        self.ticks += 1;
        report
    }

    fn run_all(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        function: Option<&str>,
        budget: u64,
        skip: &BTreeSet<u32>,
    ) -> StepReport {
        let mut report = StepReport::default();
        for inst in self.instances.clone() {
            if skip.contains(&inst.thing) || self.stopped.contains(&inst.thing) {
                continue;
            }
            let mut b = budget;
            // `None`: the tick's entry, a frame entry or `Update__`.
            let entry = match function {
                Some(_) => None,
                None => inst.class.and_then(|c| frame_entry(vm, c)),
            };
            let o = match entry {
                Some(e) => {
                    let args = (e.args)(&FrameContext {
                        vm,
                        instances: &self.instances,
                        tick: self.ticks,
                    });
                    self.runner
                        .start_with(vm, host, inst, e.function, &args, &mut b)
                }
                None => {
                    let f = function.unwrap_or(UPDATE);
                    self.runner.start(vm, host, inst, f, &mut b)
                }
            };
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(inst.thing);
            }
            report.add(inst.thing, o);
        }
        report
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn part_names() {
        assert_eq!(super::part_name("PSpriteLight"), "SPRITE_LIGHT");
        assert_eq!(super::part_name("PJoint"), "JOINT");
        assert_eq!(super::part_name("PSwitch"), "SWITCH");
        assert_eq!(super::part_name("PYellowHead"), "YELLOWHEAD");
        assert_eq!(super::part_name("YellowHead"), "YELLOWHEAD");
    }
}
