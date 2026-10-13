//! The per-tick switch pass and the script reaction it raises (RScript stage 3b;
//! docs/level-scripts.md, docs/rscript-plan.md).
//!
//! FOUND in the 01.33 executable:
//! - The world update (0xeacd4) runs the switch pass (0xdd228) once per tick. The pass
//!   orders the level's PSwitches topologically by input wiring, then updates each one in
//!   that order (0x1244a4).
//! - Updating a switch computes each output's new {activation, ternary, player} (0x11bd58,
//!   by switch type: [`compute_output`]), writes it to the output, and applies it to the
//!   output's targets (0x11ddf0) when the edge rule of [`output_edge`] says so.
//! - A Thing's "deactivated" byte (+0x7f) forces its outputs to {0.0, 0, -1}. A microchip
//!   whose activation input is off sets it on its board and the components on it
//!   ([`LevelWorld::recompute_deactivated`]).
//! - Applying a signal to a target dispatches on the target Thing's object kind (byte
//!   +0x7e). A toggle, counter, timer or selector changes its state there, on the edge
//!   ([`LevelWorld::apply_signal`]). For the kinds listed at [`reaction_for_script`] the
//!   game calls the script function `SwitchPerformReaction__b` or
//!   `SwitchPerformReaction__bf` on the target's own script.
//!
//! The logic types are computed; sensors and the other types whose signal comes from the
//! world (players, tags, physics, the pads) keep the signal the level file stores.
//!
//! Not modelled:
//! - The dirty flags. The game re-runs the event-driven switch types only when something
//!   marked them dirty and every other type each tick; here every switch runs each tick,
//!   in the same order. The computed types are pure functions of their inputs and state,
//!   so the results are the same (INFERRED: not compared against the game).
//! - The skip for a Thing whose flag byte +0x78 has bit 1 set (meaning not established).
//! - Outputs without targets that a second test forces off like a deactivated Thing
//!   (pad types 0x26, 0x2c and 0x37 only).
//! - The per-type work at the end of a switch's update other than the timer's (types 0,
//!   8/0x27, 0x13, 0x1d).
//! - The random switch (type 19): two of its four pick patterns are not read yet.
//! - The player / team filter of a counter (always passes here).
//! - Deactivation inside a sequencer and of a cursor chip; the re-fire flag (PSwitch+0xe3).
//! - Switch types 0x24/0x25 standing in for their microchip's switch in the ordering.
//! - The other object kinds a signal is applied to (lights, emitters, movers and so on).
//! - The activating player the game records on the target before the dispatch.
use crate::{InputWire, LevelWorld, SwitchState, ThingState};
use lbp_script_vm::natives::SwitchInput;
use std::collections::BTreeSet;

/// The signal of an output that is forced off (FOUND, 0x1244a4: activation 0.0, ternary
/// 0, player -1).
pub const SIGNAL_OFF: SwitchInput = SwitchInput {
    activation: 0.,
    ternary: 0,
    player: -1,
};

/// What a PSwitch remembers between ticks besides its outputs. One per PSwitch, started
/// from the values the level file stores. [`compute_output`] reads it; the game changes
/// it where a signal is applied to the switch ([`LevelWorld::apply_signal`]) and in the
/// timer's step, not in the output computation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SwitchMemory {
    /// Times the switch was updated by the pass.
    pub updates: u64,
    /// ManualActivation (PSwitch+0xc): a toggle's latch, a battery's value, a timer's
    /// rate in ticks per tick; its player is the one a selector or counter hands on.
    pub manual: SwitchInput,
    /// BulletsDetected (PSwitch+0x18): a counter's count, a selector's selected output.
    pub count: i32,
    /// TimerCount (PSwitch+0x20): a timer's position in ticks.
    pub timer: f32,
}

/// Read access [`compute_output`] gets: the level, the PSwitch and its Thing.
#[derive(Clone, Copy)]
pub struct SwitchContext<'a> {
    pub world: &'a LevelWorld,
    /// The Thing that carries the PSwitch.
    pub thing: u32,
    pub switch: &'a SwitchState,
}

impl SwitchContext<'_> {
    /// The Thing's input table: per port, the output wired to it.
    pub fn inputs(&self) -> &[Option<InputWire>] {
        self.world
            .things
            .get(self.thing as usize)
            .map_or(&[], |t| &t.inputs)
    }

    /// The current signal on input port `port`, `None` when nothing is wired to it. A
    /// switch earlier in the pass order has this tick's value already; one later in the
    /// order (a cycle) still has last tick's.
    pub fn input(&self, port: usize) -> Option<SwitchInput> {
        let wire = (*self.inputs().get(port)?)?;
        let s = self
            .world
            .things
            .get(wire.switch as usize)?
            .switch
            .as_ref()?;
        Some(s.outputs.get(wire.output)?.as_ref()?.signal)
    }

    /// The signal the level file stores for output `output` of this switch.
    pub fn stored(&self, output: usize) -> Option<SwitchInput> {
        Some(self.switch.outputs.get(output)?.as_ref()?.stored)
    }

    /// Input port `port` as the game reads it: an unwired port, or one past the table,
    /// reads {0.0, 0, -1} (FOUND, 0x10a6c8).
    pub fn port(&self, port: usize) -> SwitchInput {
        self.input(port).unwrap_or(SIGNAL_OFF)
    }

    /// The signal output `output` holds now ({0.0, 0, -1} when there is no such output).
    pub fn current(&self, output: usize) -> SwitchInput {
        let o = self.switch.outputs.get(output);
        o.and_then(|o| o.as_ref()).map_or(SIGNAL_OFF, |o| o.signal)
    }
}

/// The wire on input port `port` of a Thing and the signal on it.
fn wired(world: &LevelWorld, thing: u32, port: usize) -> Option<(InputWire, SwitchInput)> {
    let wire = (*world.things.get(thing as usize)?.inputs.get(port)?)?;
    let s = world.things.get(wire.switch as usize)?.switch.as_ref()?;
    Some((wire, s.outputs.get(wire.output)?.as_ref()?.signal))
}

/// How an inverted switch turns its output round ([`compute_output`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Invert {
    /// Activation a -> 1 - a (a + 1 below zero), ternary 0 <-> 1.
    Standard,
    /// Both negated: the signed types.
    Negate,
}

fn off() -> Option<(SwitchInput, Invert)> {
    Some((SIGNAL_OFF, Invert::Standard))
}

fn clamp(x: f32, low: f32, high: f32) -> f32 {
    if x < low {
        low
    } else if x > high {
        high
    } else {
        x
    }
}

/// Whether an output carries a signed signal (FOUND, 0x900730): the three-way switch,
/// the analogue controller, the direction combiner, the stick outputs of the pads, and a
/// microchip or circuit-board output that mirrors one of those.
fn output_signed(world: &LevelWorld, wire: InputWire, depth: u32) -> bool {
    let Some(t) = world.things.get(wire.switch as usize) else {
        return false;
    };
    match t.switch.as_ref().map(|s| s.switch_type) {
        Some(2 | 10 | 20) => true,
        Some(38) => (10..=17).contains(&wire.output),
        Some(44) => (10..=24).contains(&wire.output),
        Some(55) => (7..=19).contains(&wire.output),
        Some(36 | 37) if depth < MAX_NESTING => mirrored(world, wire.switch, wire.output)
            .flatten()
            .is_some_and(|(w, _)| output_signed(world, w, depth + 1)),
        _ => false,
    }
}

/// Chips inside chips deeper than this are not followed (a guard against a parent loop).
const MAX_NESTING: u32 = 64;

/// The wire a microchip or circuit-board output mirrors: outer `None` when the port
/// offset is not known (a board on a sequencer), inner `None` when nothing is wired.
fn mirrored(
    world: &LevelWorld,
    thing: u32,
    output: usize,
) -> Option<Option<(InputWire, SwitchInput)>> {
    let t = world.things.get(thing as usize)?;
    match t.switch.as_ref()?.switch_type {
        36 => Some(t.circuit_board.and_then(|b| wired(world, b, output))),
        37 => {
            let Some(parent) = t.parent else {
                return Some(None);
            };
            let k = board_port_offset(world.things.get(parent as usize)?)?;
            Some(wired(world, parent, output + k))
        }
        _ => None,
    }
}

/// The first input port of a circuit board's owner that the board's outputs mirror
/// (FOUND, 0x3c784, by the owner's object kind): 1 for a microchip, controlinator or
/// selector (port 0 is the owner's own activation input), 2 for a data sampler, else 0.
/// `None` for a sequencer: 1 or 2 by a sequencer setting the reader does not keep.
fn board_port_offset(owner: &ThingState) -> Option<usize> {
    if owner.parts.contains(&"SEQUENCER") {
        return None;
    }
    if owner.parts.contains(&"CONTROLINATOR") {
        return Some(1);
    }
    Some(match owner.switch.as_ref().map(|s| s.switch_type) {
        Some(35 | 36) => 1,
        Some(65) => 2,
        _ => 0,
    })
}

/// The ternary of a timer's activation (FOUND, 0x11873c with a zero dead zone): 0 at or
/// below 0, 1 at or above 1, in between what output 0 holds (taken back through the
/// inversion when the switch is inverted).
fn timer_ternary(cx: &SwitchContext<'_>, activation: f32) -> i32 {
    if activation <= 0. {
        0
    } else if activation >= 1. {
        1
    } else {
        let held = cx.current(0).ternary;
        if cx.switch.inverted {
            i32::from(held == 0)
        } else {
            held
        }
    }
}

/// The new signal of output `output` of a PSwitch, before the deactivated forcing.
///
/// FOUND: the game computes it in 0x11bd58 through a jump table on the switch type
/// (0x11be00, 0x42 entries), then, when the switch is inverted (PSwitch+0xc5), turns it
/// round through a second table (0x11bfd4), then applies the old-joint deadband
/// (PSwitch+0xc8). The handlers do not change the switch's state.
///
/// Types whose signal comes from the world keep the stored signal as it is (it was
/// stored after the inversion).
pub fn compute_output(
    cx: &SwitchContext<'_>,
    output: usize,
    memory: &mut SwitchMemory,
) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let stored = cx.stored(output).unwrap_or(SIGNAL_OFF);
    let standard = |signal| Some((signal, Invert::Standard));
    let mirror = || match mirrored(w, cx.thing, output) {
        None => None,
        Some(None) => off(),
        Some(Some((wire, signal))) if output_signed(w, wire, 0) => Some((signal, Invert::Negate)),
        Some(Some((_, signal))) => standard(signal),
    };
    let computed = match s.switch_type {
        // MICROCHIP, 0x11c918, FOUND: the signal wired to port `output` of its board.
        // CIRCUIT_BOARD, 0x11c9c8, FOUND: the signal wired to its owner's port.
        36 | 37 => mirror(),
        // NOT, NOP, CIRCUIT_NODE, 0x11ca88, FOUND: port 0 passed through.
        23 | 24 | 40 => standard(cx.port(0)),
        // AND, 0x11cb74, FOUND (the names of the two modes INFERRED).
        14 => standard(and_gate(cx)),
        // OR, 0x11c3ec, FOUND (the names of the two modes INFERRED).
        21 => standard(or_gate(cx)),
        // XOR, 0x11c574, FOUND.
        22 => standard(xor_gate(cx)),
        // ALWAYS_ON (battery), TOGGLE, 0x11c820, FOUND: ManualActivation.
        17 | 28 => standard(memory.manual),
        // DIRECTION (combiner), 0x11c328, FOUND: port 1 positive, port 0 negative.
        20 => {
            let (a, b) = (cx.port(0), cx.port(1));
            let signal = SwitchInput {
                activation: b.activation.abs() - a.activation.abs(),
                ternary: (b.ternary.abs() - a.ternary.abs()).clamp(-1, 1),
                player: if a.activation.abs() <= b.activation.abs() && b.player != -1 {
                    b.player
                } else {
                    a.player
                },
            };
            Some((signal, Invert::Negate))
        }
        // SIGN_SPLIT (splitter), 0x11c7cc, FOUND: output 0 negative part, others positive.
        27 => {
            let a = cx.port(0);
            let sign = if output == 0 { -1 } else { 1 };
            standard(SwitchInput {
                activation: (sign as f32 * a.activation).max(0.),
                ternary: (sign * a.ternary).max(0),
                player: a.player,
            })
        }
        // SELECTOR, 0x11c8f8, FOUND: on at the selected output.
        35 if usize::try_from(memory.count) == Ok(output) => standard(SwitchInput {
            activation: 1.,
            ternary: 1,
            player: memory.manual.player,
        }),
        35 => off(),
        // COUNTDOWN (counter), 0x11ca04, FOUND: count / target, on when full.
        15 => {
            let activation = clamp(
                memory.count as f32 / s.bullets_required.max(1) as f32,
                0.,
                1.,
            );
            standard(SwitchInput {
                activation,
                ternary: i32::from(activation >= 1.),
                player: memory.manual.player,
            })
        }
        // TIMER, 0x11ccfc, FOUND: position / length (ticks; 30 per second INFERRED).
        16 => {
            let activation = clamp(memory.timer / s.activation_hold_time.max(1) as f32, 0., 1.);
            standard(SwitchInput {
                activation,
                ternary: timer_ternary(cx, activation),
                player: memory.manual.player,
            })
        }
        // PRESSURE, SACKBOT_*, INACTIVE, DEATH, CUTSCENE_CAM_ACTIVE and the later types
        // whose signal is not made in the pass, 0x11bf08, FOUND: {0.0, 0, -1}.
        7 | 11..=13 | 26 | 31 | 33 | 46..=53 | 56 | 58..=64 => off(),
        // Past the table, 0x11bd58, FOUND: {0.0, 0, -1} (INFERRED: not inverted).
        t if t >= SWITCH_KIND_TYPES => return SIGNAL_OFF,
        // RANDOM (19, 0x11c2e0) and the sensors: the stored signal.
        _ => None,
    };
    let Some((mut new, invert)) = computed else {
        return stored;
    };
    if s.inverted {
        // Table 0x11bfd4, FOUND. The player is kept.
        match invert {
            Invert::Negate => {
                new.activation = -new.activation;
                new.ternary = -new.ternary;
            }
            Invert::Standard => {
                new.activation = if new.activation < 0. {
                    new.activation + 1.
                } else {
                    1. - new.activation
                };
                new.ternary = i32::from(new.ternary == 0);
            }
        }
    }
    if s.old_joint {
        // The old-joint deadband, FOUND: a small move of the activation with the same
        // player keeps the signal the output holds.
        let old = cx.current(output);
        let a = new.activation;
        if (a % 1. != 0. || old.activation == a)
            && (a - old.activation).abs() <= 0.025
            && old.player == new.player
        {
            return old;
        }
    }
    new
}

/// AND: ports 0 and 1, then up to the gate's port count (BulletsRequired).
fn and_gate(cx: &SwitchContext<'_>) -> SwitchInput {
    let one = |t: i32| if t == 0 { 1. } else { t as f32 };
    let ports = usize::try_from(cx.switch.bullets_required).unwrap_or(0);
    let (a, b) = (cx.port(0), cx.port(1));
    if cx.switch.gate_mode == 0 {
        let mut r = SwitchInput {
            activation: a.activation.abs().min(b.activation.abs())
                * (one(a.ternary) * one(b.ternary)),
            ternary: a.ternary * b.ternary,
            player: if b.ternary.abs() < a.ternary.abs() || a.player == -1 {
                b.player
            } else {
                a.player
            },
        };
        for k in 2..ports {
            let p = cx.port(k);
            r.activation =
                r.activation.abs().min(p.activation.abs()) * (one(r.ternary) * one(p.ternary));
            if p.ternary.abs() < r.ternary.abs() || r.player == -1 {
                r.player = p.player;
            }
            r.ternary *= p.ternary;
        }
        r
    } else {
        let mut r = SwitchInput {
            activation: a.activation * b.activation,
            ternary: a.ternary * b.ternary,
            player: if a.activation.abs() < b.activation.abs() || a.player == -1 {
                b.player
            } else {
                a.player
            },
        };
        for k in 2..ports {
            let p = cx.port(k);
            if r.activation.abs() < p.activation.abs() || r.player == -1 {
                r.player = p.player;
            }
            r.activation *= p.activation;
            r.ternary *= p.ternary;
        }
        r
    }
}

/// OR: ports 0 and 1, then the rest of the input table (mode 0) or up to the gate's
/// port count (the adding mode).
fn or_gate(cx: &SwitchContext<'_>) -> SwitchInput {
    let (a, b) = (cx.port(0), cx.port(1));
    if cx.switch.gate_mode == 0 {
        let mut r = SwitchInput {
            activation: if a.activation.abs() < b.activation.abs() {
                b.activation
            } else {
                a.activation
            },
            ternary: if a.ternary.abs() < b.ternary.abs() {
                b.ternary
            } else {
                a.ternary
            },
            player: if a.ternary.abs() < b.ternary.abs() || a.player == -1 {
                b.player
            } else {
                a.player
            },
        };
        for k in 2..cx.inputs().len() {
            let p = cx.port(k);
            if r.activation.abs() < p.activation.abs() {
                r.activation = p.activation;
            }
            if r.ternary.abs() < p.ternary.abs() || r.player == -1 {
                r.player = p.player;
            }
            if r.ternary.abs() < p.ternary.abs() {
                r.ternary = p.ternary;
            }
        }
        r
    } else {
        let mut r = SwitchInput {
            activation: a.activation + b.activation,
            ternary: (a.ternary + b.ternary).clamp(-1, 1),
            player: if a.activation.abs() < b.activation.abs() || a.player == -1 {
                b.player
            } else {
                a.player
            },
        };
        for k in 2..usize::try_from(cx.switch.bullets_required).unwrap_or(0) {
            let p = cx.port(k);
            if r.activation.abs() < p.activation.abs() || r.player == -1 {
                r.player = p.player;
            }
            r.activation += p.activation;
            r.ternary = (r.ternary + p.ternary).clamp(-1, 1);
        }
        r
    }
}

/// XOR over the whole input table: on when exactly one input is on.
fn xor_gate(cx: &SwitchContext<'_>) -> SwitchInput {
    let mut r = SIGNAL_OFF;
    let mut active = 0;
    for k in 0..cx.inputs().len() {
        let p = cx.port(k);
        if r.activation.abs() < p.activation.abs() {
            r.activation = p.activation;
        }
        if p.ternary != 0 {
            active += 1;
            if active == 1 {
                (r.ternary, r.player) = (p.ternary, p.player);
            }
        }
    }
    if active > 1 {
        SIGNAL_OFF
    } else {
        r
    }
}

/// What an output's change of signal means for its targets ([`output_edge`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputEdge {
    /// The value handed to the targets (`SwitchPerformReaction`'s bool).
    pub on: bool,
    /// Whether the targets are visited at all this tick.
    pub visit: bool,
}

/// The edge rule of the per-tick switch update.
///
/// FOUND (0x1245fc-0x1246f4), per output, old against new signal:
/// `new_on` = new ternary != 0, `old_on` = old ternary != 0, `tern_changed` = the
/// ternaries differ, `player_changed` = the players differ, `force` = PSwitch+0xe3.
/// `on = new_on && (tern_changed || force)`; the targets are visited only if
/// `old_on != new_on || player_changed || on`.
///
/// So: a rising edge gives true, a falling edge false, a sign flip while on true again.
/// A player change with the same non-zero ternary visits with false (as coded in the
/// game). A change of the activation alone visits nothing.
pub fn output_edge(old: SwitchInput, new: SwitchInput, force: bool) -> OutputEdge {
    let (new_on, old_on) = (new.ternary != 0, old.ternary != 0);
    let tern_changed = old.ternary != new.ternary;
    let player_changed = old.player != new.player;
    let on = new_on && (tern_changed || force);
    OutputEdge {
        on,
        visit: old_on != new_on || player_changed || on,
    }
}

/// One target of an output that the pass visits this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetVisit {
    /// The Thing with the PSwitch, and which of its outputs.
    pub switch: u32,
    pub output: usize,
    /// The target Thing and the input port the output is wired to.
    pub target: u32,
    pub port: i32,
    /// [`OutputEdge::on`].
    pub on: bool,
    /// The output's new signal.
    pub signal: SwitchInput,
}

/// Which script function a switch reaction calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    /// `SwitchPerformReaction__b(on)`.
    Bool,
    /// `SwitchPerformReaction__bf(on, ternary != 0 ? 1.0 : 0.0)`.
    BoolFloat,
}

/// `SwitchPerformReaction__b` (FOUND: function record 0xe45ef8, called at 0x11e5d8).
pub const REACTION_B: &str = "SwitchPerformReaction__b";
/// `SwitchPerformReaction__bf` (FOUND: function record 0xe46960, called at 0x11e928).
pub const REACTION_BF: &str = "SwitchPerformReaction__bf";

/// The switch types the game's per-type tables cover (0x11be00, 0x3eed4) end below this.
pub const SWITCH_KIND_TYPES: i32 = 0x42;

/// The reaction a script gets, by the GUID of its script resource; `None` = no call.
///
/// FOUND: the game picks the action from the target Thing's object kind (byte +0x7e, set
/// by 0x3ed2c). For scripted Things the kind follows the script resource GUID, and only
/// these kinds reach a script call (0x11ddf0, cases 0x11e500 and 0x11e7cc):
///
/// | kind | script GUID | call |
/// |---|---|---|
/// | 0x48 | 18256 | `__b` |
/// | 0x4f | 25580 | `__b` |
/// | 0x51, 0x52 | 65238 (a script query picks which; both get the call) | `__b` |
/// | 0x55 | 79156 | `__b` |
/// | 0x56 | 87281 | `__b` |
/// | 0x64 | 125169, 125367, 125391, 125392, 158628 | `__b` |
/// | 0x6c | 18420 | `__bf` |
/// | 0x73 | 98497 | `__b` |
/// | 0x90 | 27452 | `__b` |
/// | 0x99 | 188288 | `__b` |
/// | 0x9d | 242022 | `__b` |
///
/// A script with any other GUID gets no reaction call, whatever functions it defines.
///
/// Left out: kinds 0x49, 0x53 and 0x54 also get `__b`, but what gives a Thing those
/// kinds is not resolved. Kind 0x4f has a second source (a part, not a script) that is
/// not modelled. The `__bf` case has three more gates on game state whose meaning is not
/// established; here the call is always made.
pub fn reaction_for_script(guid: u32) -> Option<Reaction> {
    match guid {
        18256 | 25580 | 27452 | 65238 | 79156 | 87281 | 98497 | 125169 | 125367 | 125391
        | 125392 | 158628 | 188288 | 242022 => Some(Reaction::Bool),
        18420 => Some(Reaction::BoolFloat),
        _ => None,
    }
}

/// Whether the object-kind test gives a PSwitch of this type a kind of its own.
///
/// FOUND (0x3ed2c, jump table 0x3eed4 on the switch type, 0x42 entries): these types
/// return a kind (0x27..0x43, 0x93..0x97, 0x9b, 0xa0, or 3 / 6 / 0x96 for a microchip),
/// none of them a script-reaction kind. Every other type below 0x42 jumps on to the
/// tests that follow, as does a type at or past 0x42. Type 3 (the player sensor) returns
/// a kind only on a Thing without a PShape (INFERRED: which register holds the part
/// mask in that one test).
pub fn switch_type_has_kind(switch_type: i32, has_shape: bool) -> bool {
    match switch_type {
        0..=2 | 4..=6 | 8 | 14..=23 | 25 | 27..=30 | 35..=44 | 54 | 55 | 65 => true,
        3 => !has_shape,
        _ => false,
    }
}

/// Parts the object-kind test looks at before the PSwitch, in its order (FOUND, 0x3ed2c).
/// Each gives the Thing a kind that is not a script-reaction kind.
const KIND_PARTS_BEFORE_SWITCH: [&str; 11] = [
    "NPC",
    "YELLOWHEAD",
    "SEQUENCER",
    "SWITCH_KEY",
    "INSTRUMENT",
    "EMITTER",
    "AUDIO_WORLD",
    "CREATURE",
    "MATERIAL_TWEAK",
    "WORLD",
    "CONTROLINATOR",
];

/// Parts tested after the PSwitch and before the script (FOUND, 0x3ed2c). A
/// PSwitchInput always ends the test (kind 0x44..0x46 or none). INFERRED for the joint
/// and the physics tweak: their type is in the range the test covers (below 10 and
/// below 15), which the reader does not keep.
const KIND_PARTS_AFTER_SWITCH: [&str; 4] = ["JOINT", "SWITCH_INPUT", "PHYSICS_TWEAK", "WORMHOLE"];

/// The part that gives a Thing its object kind before its script is looked at, if any;
/// such a Thing gets no script reaction whatever its script.
///
/// FOUND (0x3ed2c): the kind tests run in a fixed order and the first that matches
/// returns: the parts of [`KIND_PARTS_BEFORE_SWITCH`], a PSwitch whose type has a kind
/// ([`switch_type_has_kind`]), the parts of [`KIND_PARTS_AFTER_SWITCH`], then tests on
/// the mesh, the trigger's parent and the gameplay data (not modelled), then the script
/// GUID ([`reaction_for_script`]). So a magic mouth (switch type 34) or a wormhole
/// (type 57) keeps the kind of its script, while a scripted Thing that is also a
/// button, a gate or a sensor does not.
pub fn kind_before_script(thing: &ThingState) -> Option<&'static str> {
    let has = |part: &&str| thing.parts.contains(part);
    if let Some(part) = KIND_PARTS_BEFORE_SWITCH.iter().find(|p| has(p)) {
        return Some(part);
    }
    if let Some(s) = &thing.switch {
        if switch_type_has_kind(s.switch_type, has(&"SHAPE")) {
            return Some("SWITCH");
        }
    }
    KIND_PARTS_AFTER_SWITCH.iter().find(|p| has(p)).copied()
}

/// Scripts that make a microchip a controlinator for the object-kind test (FOUND,
/// 0x3f4c0).
const CONTROLINATOR_SCRIPTS: [u32; 2] = [68742, 88656];

/// Whether a Thing has the microchip object kind (3): a PSwitch of type 36 that is not
/// a cursor chip (IncludeTouching above 0) or a controlinator, and no part that is
/// tested before the PSwitch (FOUND, 0x3ed2c and its case 0x3f45c).
fn is_chip(t: &ThingState) -> bool {
    t.switch
        .as_ref()
        .is_some_and(|s| s.switch_type == 36 && s.include_touching <= 0)
        && !t
            .script_guid
            .is_some_and(|g| CONTROLINATOR_SCRIPTS.contains(&g))
        && !KIND_PARTS_BEFORE_SWITCH.iter().any(|p| t.parts.contains(p))
}

/// Whether a Thing has the circuit-board object kind (0x40).
fn is_board(t: &ThingState) -> bool {
    t.switch.as_ref().is_some_and(|s| s.switch_type == 37)
        && !KIND_PARTS_BEFORE_SWITCH.iter().any(|p| t.parts.contains(p))
}

impl LevelWorld {
    /// The Things with a PSwitch in the order the pass updates them: a switch whose output
    /// is wired to another switch's Thing comes before that switch. Computed on first use
    /// and kept until [`LevelWorld::wire_inputs`] runs again.
    ///
    /// FOUND (0xdd228): a topological sort over the input wiring (each Thing's input
    /// table gives the outputs it depends on, each output its owning switch); cycles are
    /// broken. INFERRED: the tie-breaks. Here, among the switches that are ready the
    /// lowest Thing index goes first, and when none is ready (a cycle) the lowest Thing
    /// index not yet placed goes next. A switch wired to itself does not wait for itself.
    pub fn switch_order(&mut self) -> &[u32] {
        if self.switch_order.is_none() {
            self.switch_order = Some(self.sort_switches());
        }
        self.switch_order.as_deref().unwrap_or(&[])
    }

    fn sort_switches(&self) -> Vec<u32> {
        let n = self.things.len();
        // Per switch Thing: the switches it waits for, and those waiting for it.
        let mut waits = vec![0usize; n];
        let mut after: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut left = BTreeSet::new();
        for (i, t) in self.things.iter().enumerate() {
            if t.switch.is_none() {
                continue;
            }
            left.insert(i as u32);
            let before: BTreeSet<u32> = t
                .inputs
                .iter()
                .flatten()
                .map(|w| w.switch)
                .filter(|&s| s as usize != i && (s as usize) < n)
                .filter(|&s| self.things[s as usize].switch.is_some())
                .collect();
            waits[i] = before.len();
            for s in before {
                after[s as usize].push(i as u32);
            }
        }
        let mut ready: BTreeSet<u32> = left
            .iter()
            .copied()
            .filter(|&s| waits[s as usize] == 0)
            .collect();
        let mut order = Vec::with_capacity(left.len());
        // Ready first; in a cycle the lowest index left.
        while let Some(next) = ready.first().or(left.first()).copied() {
            ready.remove(&next);
            left.remove(&next);
            order.push(next);
            for &s in &after[next as usize] {
                let w = &mut waits[s as usize];
                *w = w.saturating_sub(1);
                if *w == 0 && left.contains(&s) {
                    ready.insert(s);
                }
            }
        }
        order
    }

    /// Sets a Thing's "deactivated" flag ([`crate::ThingState::deactivated`]) directly.
    /// The next pass forces the Thing's outputs off and raises the falling edge.
    pub fn set_deactivated(&mut self, thing: u32, deactivated: bool) {
        if let Some(t) = self.things.get_mut(thing as usize) {
            t.deactivated = deactivated;
        }
    }

    /// Whether the Thing sits on a circuit board that belongs to a sequencer (FOUND,
    /// 0xf3d94).
    fn on_sequencer(&self, thing: u32) -> bool {
        let get = |i: u32| self.things.get(i as usize);
        let board = get(thing).and_then(|t| t.parent).and_then(get);
        board.is_some_and(|b| {
            is_board(b)
                && b.parent
                    .and_then(get)
                    .is_some_and(|o| o.parts.contains(&"SEQUENCER"))
        })
    }

    /// Works out a Thing's "deactivated" flag again and hands it down.
    ///
    /// FOUND (0x409dc): a microchip is deactivated when its input port 0 is wired and
    /// that signal's ternary is 0 (the activation and the player are not looked at); a
    /// circuit board takes the flag of the chip it belongs to; any other Thing starts
    /// from "not deactivated". A Thing whose parent is a circuit board is also
    /// deactivated when that board is. The chip's flag is then written to its board and
    /// every child of the board is worked out the same way, so chips inside chips
    /// follow. The state a switch remembers is not reset.
    ///
    /// Not modelled: a board that belongs to a sequencer (its contents switch by the
    /// playhead) and a cursor chip; their flags are left as they are.
    pub fn recompute_deactivated(&mut self, thing: u32) {
        self.recompute_deactivated_at(thing, 0);
    }

    fn recompute_deactivated_at(&mut self, thing: u32, depth: u32) {
        let get = |i: u32| self.things.get(i as usize);
        let Some(t) = get(thing) else {
            return;
        };
        let cursor = t
            .switch
            .as_ref()
            .is_some_and(|s| s.switch_type == 36 && s.include_touching > 0);
        if depth > MAX_NESTING || self.on_sequencer(thing) || cursor {
            return;
        }
        let parent = t.parent.and_then(get);
        let mut d = if is_chip(t) {
            wired(self, thing, 0).is_some_and(|(_, s)| s.ternary == 0)
        } else if is_board(t) {
            parent.is_some_and(|p| {
                p.deactivated
                    && (p.parts.contains(&"CONTROLINATOR")
                        || p.switch.as_ref().is_some_and(|s| s.switch_type == 36))
            })
        } else {
            false
        };
        d |= parent.is_some_and(|p| is_board(p) && p.deactivated);
        let board = t.circuit_board.filter(|_| !t.parts.contains(&"SEQUENCER"));
        self.things[thing as usize].deactivated = d;
        let Some(board) = board.filter(|&b| b != thing && (b as usize) < self.things.len()) else {
            return;
        };
        self.things[board as usize].deactivated = d;
        let mut child = self.things[board as usize].first_child;
        let mut left = self.things.len();
        while let Some(c) = child.filter(|_| left > 0) {
            left -= 1;
            self.recompute_deactivated_at(c, depth + 1);
            child = self.things.get(c as usize).and_then(|t| t.next_sibling);
        }
    }

    /// Gives the deactivated flags their start values, once, before the first pass:
    /// every microchip is worked out from the stored signal on its port 0
    /// ([`Self::recompute_deactivated`]).
    ///
    /// INFERRED: how the game sets the flags when a level loads is not read yet (it
    /// restores them from a saved bit); the stored signals are taken to agree with them.
    pub fn prepare_switches(&mut self) {
        if std::mem::replace(&mut self.switches_prepared, true) {
            return;
        }
        for i in 0..self.things.len() {
            if is_chip(&self.things[i]) {
                self.recompute_deactivated(i as u32);
            }
        }
    }

    /// What a signal does to the target it is applied to, besides the script reaction:
    /// the state changes of the logic types, which the game makes here and not in the
    /// output computation. `on` is [`OutputEdge::on`] (a rising edge, or a sign flip
    /// while on); `signal` is the output's new signal.
    ///
    /// FOUND (0x11ddf0, by the target's object kind):
    /// - microchip, port 0, any visit: its deactivated flag is worked out again (case
    ///   0x11e304).
    /// - toggle, any port, `on`: the latch flips and takes the signal's player (case
    ///   0x11f0b4).
    /// - counter, `on`: port 0 adds the signal's ternary (so -1 counts down) while the
    ///   result stays within 0..=target and the counter's output is settled; port 1
    ///   sets the count to 0 (case 0x11eeec). The player / team filter of port 0 is not
    ///   modelled.
    /// - selector, port 0, `on`: steps the selected output by the signal's ternary,
    ///   wrapping (case 0x11ed80).
    /// - timer, `on`: port 1 resets it; port 0 starts it counting up (behaviour 2) or
    ///   down (behaviour 3, from full when at 0) (case 0x11ee20).
    pub fn apply_signal(&mut self, target: u32, port: i32, on: bool, signal: SwitchInput) {
        let Some(t) = self.things.get(target as usize) else {
            return;
        };
        if is_chip(t) {
            if port == 0 {
                self.recompute_deactivated(target);
            }
            return;
        }
        let Some(s) = t.switch.as_ref().filter(|_| on) else {
            return;
        };
        let mut m = s.memory;
        match (s.switch_type, port) {
            (17, _) => {
                m.manual = SwitchInput {
                    activation: if m.manual.activation < 0. {
                        m.manual.activation + 1.
                    } else {
                        1. - m.manual.activation
                    },
                    ternary: i32::from(m.manual.ternary == 0),
                    player: signal.player,
                };
            }
            (15, 0) => {
                let n = m.count.saturating_add(signal.ternary);
                let cx = SwitchContext {
                    world: self,
                    thing: target,
                    switch: s,
                };
                let settled = compute_output(&cx, 0, &mut m).ternary == cx.current(0).ternary;
                if (0..=s.bullets_required).contains(&n) && settled {
                    if signal.player != -1 {
                        m.manual.player = signal.player;
                    }
                    m.count = n;
                }
            }
            (15, 1) => {
                m.count = 0;
                m.manual.player = signal.player;
            }
            (35, 0) => {
                if let Ok(count @ 1..) = u32::try_from(s.outputs.len()) {
                    // Unsigned in the game: a negative index wraps the same way.
                    let step = (m.count as u32)
                        .wrapping_add(signal.ternary as u32)
                        .wrapping_add(count);
                    m.count = (step % count) as i32;
                    m.manual.player = signal.player;
                }
            }
            (16, 1) => {
                m.manual = SwitchInput {
                    activation: 0.,
                    ternary: 0,
                    player: signal.player,
                };
                m.timer = 0.;
            }
            (16, 0) if s.behavior == 2 => {
                m.manual = SwitchInput {
                    activation: 1.,
                    ternary: 1,
                    player: signal.player,
                };
            }
            (16, 0) if s.behavior == 3 => {
                if m.timer == 0. {
                    m.timer = s.activation_hold_time as f32;
                }
                m.manual = SwitchInput {
                    activation: -1.,
                    ternary: -1,
                    player: signal.player,
                };
            }
            _ => return,
        }
        if let Some(s) = self.things[target as usize].switch.as_mut() {
            s.memory = m;
        }
    }

    /// The timer's step at the end of its update: sets its rate from input port 0, then
    /// moves its position by the rate.
    ///
    /// FOUND (0x11f69c). Port 0 unwired: the timer runs forwards at one tick per tick
    /// (on a sequencer by behaviour and the deactivated flag). Port 0 wired, by
    /// behaviour: 0 runs while the input is on; 1 runs forwards while it is on and
    /// backwards while it is off (a signed input that is off stops it); 4 takes the
    /// input as the rate; 2 and 3 are started by the edge ([`Self::apply_signal`]).
    /// The position then moves by the rate, kept within 0..=length, unless the output's
    /// ternary is about to change. INFERRED: the names of the behaviours.
    fn timer_step(&mut self, thing: u32) {
        let Some(s) = self
            .things
            .get(thing as usize)
            .and_then(|t| t.switch.as_ref())
        else {
            return;
        };
        let rate = |activation: f32, ternary, player| SwitchInput {
            activation,
            ternary,
            player,
        };
        let mut m = s.memory;
        match wired(self, thing, 0) {
            None if !self.on_sequencer(thing) => m.manual = rate(1., 1, -1),
            None => {
                let deactivated = self.things[thing as usize].deactivated;
                match (s.behavior, deactivated) {
                    (0 | 1 | 4, false) => m.manual = rate(1., 1, -1),
                    (1, true) => m.manual = rate(-1., -1, -1),
                    (0 | 4, true) => m.manual = rate(0., 0, -1),
                    _ => {}
                }
            }
            Some((wire, i)) => match s.behavior {
                0 => {
                    let on = i.ternary != 0;
                    m.manual = rate(f32::from(u8::from(on)), i32::from(on), i.player);
                }
                1 if i.ternary == 0 && !output_signed(self, wire, 0) => {
                    m.manual = rate(-1., -1, i.player);
                }
                1 => m.manual = rate(i.ternary as f32, i.ternary, i.player),
                4 => m.manual = i,
                _ => {}
            },
        }
        let cx = SwitchContext {
            world: self,
            thing,
            switch: s,
        };
        if compute_output(&cx, 0, &mut m).ternary == cx.current(0).ternary {
            let length = s.activation_hold_time as f32;
            m.timer = clamp(m.timer + m.manual.activation, 0., length);
        }
        if let Some(s) = self.things[thing as usize].switch.as_mut() {
            s.memory = m;
        }
    }

    /// One tick of one PSwitch: computes and writes each output's signal, applies it to
    /// the output's targets ([`Self::apply_signal`]) and returns the targets visited, in
    /// output then target order. Null outputs and targets with a null Thing are skipped.
    ///
    /// FOUND (0x1244a4): a deactivated Thing (+0x7f) gets [`SIGNAL_OFF`] on every
    /// output, except that a counter, timer, toggle or selector (and types 32 and 34) on
    /// a sequencer's board keeps running. A selector that is not deactivated first takes
    /// its direct inputs: the highest port i + 1 that is on selects output i. Then, per
    /// output, the new signal is [`compute_output`]; it is written to the output, which
    /// is what the switch-input natives read; the targets are visited by the rule of
    /// [`output_edge`]; the force flag (PSwitch+0xe3) is cleared at the end; a timer
    /// that is not deactivated then takes its step ([`Self::timer_step`]).
    /// The old signal on the first tick is the stored one. The level files store most
    /// outputs off, a battery's too, so the first pass raises the edges of the level
    /// powering up (INFERRED: the game compares against the stored signals in its first
    /// pass as well; the connect-time application at 0x124c68 is a separate path, not
    /// modelled).
    pub fn update_switch(&mut self, thing: u32) -> Vec<TargetVisit> {
        let mut visits = Vec::new();
        let i = thing as usize;
        let Some(s) = self.things.get_mut(i).and_then(|t| t.switch.as_mut()) else {
            return visits;
        };
        let (count, force, switch_type) = (s.outputs.len(), s.force, s.switch_type);
        s.memory.updates += 1;
        let mut deactivated = self.things[i].deactivated;
        if deactivated && matches!(switch_type, 15..=17 | 32 | 34 | 35) {
            deactivated = !self.on_sequencer(thing);
        }
        if !deactivated && switch_type == 35 {
            let direct = (0..count)
                .rev()
                .find_map(|o| Some((o, wired(self, thing, o + 1)?.1)).filter(|p| p.1.ternary != 0));
            if let (Some((o, signal)), Some(s)) = (direct, self.things[i].switch.as_mut()) {
                s.memory.count = i32::try_from(o).unwrap_or(i32::MAX);
                s.memory.manual.player = signal.player;
            }
        }
        for output in 0..count {
            let Some(switch) = self.things[i].switch.as_ref() else {
                break;
            };
            let Some(stored) = switch.outputs[output].as_ref().map(|o| o.stored) else {
                continue;
            };
            let mut memory = switch.memory;
            let new = if deactivated {
                SIGNAL_OFF
            } else if let Some(on) = self.switch_pulse {
                SwitchInput {
                    activation: f32::from(u8::from(on)),
                    ternary: i32::from(on),
                    player: stored.player,
                }
            } else {
                let cx = SwitchContext {
                    world: self,
                    thing,
                    switch,
                };
                compute_output(&cx, output, &mut memory)
            };
            let Some(s) = self.things[i].switch.as_mut() else {
                break;
            };
            s.memory = memory;
            let Some(out) = s.outputs[output].as_mut() else {
                continue;
            };
            let old = std::mem::replace(&mut out.signal, new);
            let edge = output_edge(old, new, force);
            if !edge.visit {
                continue;
            }
            let first = visits.len();
            visits.extend(out.targets.iter().filter_map(|&(target, port)| {
                Some(TargetVisit {
                    switch: thing,
                    output,
                    target: target?,
                    port,
                    on: edge.on,
                    signal: new,
                })
            }));
            for v in &visits[first..] {
                self.apply_signal(v.target, v.port, v.on, v.signal);
            }
        }
        if let Some(s) = self.things[i].switch.as_mut() {
            s.force = false;
        }
        if !deactivated && switch_type == 16 {
            self.timer_step(thing);
        }
        visits
    }

    /// The whole pass without the script calls: every switch in [`Self::switch_order`],
    /// returning all the target visits. [`crate::LevelScripts::switch_reactions`] is the
    /// pass with the reactions called switch by switch.
    pub fn update_switches(&mut self) -> Vec<TargetVisit> {
        self.prepare_switches();
        let order = self.switch_order().to_vec();
        order
            .into_iter()
            .flat_map(|s| self.update_switch(s))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SwitchOutputState, ThingState};

    fn sig(activation: f32, ternary: i32, player: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player,
        }
    }

    /// A switch Thing with one output per entry: (stored signal, targets).
    fn switch(outputs: &[(SwitchInput, &[(u32, i32)])]) -> ThingState {
        ThingState {
            switch: Some(SwitchState {
                outputs: outputs
                    .iter()
                    .map(|(s, targets)| {
                        Some(SwitchOutputState {
                            signal: *s,
                            stored: *s,
                            targets: targets.iter().map(|&(t, p)| (Some(t), p)).collect(),
                        })
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn world(things: Vec<ThingState>) -> LevelWorld {
        let mut w = LevelWorld {
            things,
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    #[test]
    fn edge_rule() {
        let e = |old: (i32, i32), new: (i32, i32), force| {
            let o = output_edge(sig(0., old.0, old.1), sig(0., new.0, new.1), force);
            (o.on, o.visit)
        };
        // Rising, falling, sign flip while on.
        assert_eq!(e((0, -1), (1, -1), false), (true, true));
        assert_eq!(e((1, -1), (0, -1), false), (false, true));
        assert_eq!(e((1, -1), (-1, -1), false), (true, true));
        // Nothing changed: no visit, on or off.
        assert_eq!(e((1, -1), (1, -1), false), (false, false));
        assert_eq!(e((0, -1), (0, -1), false), (false, false));
        // A player change alone visits with false, also while on.
        assert_eq!(e((1, 0), (1, 1), false), (false, true));
        assert_eq!(e((0, 0), (0, 1), false), (false, true));
        // Force re-fires an unchanged on output; it does nothing to one that is off.
        assert_eq!(e((1, -1), (1, -1), true), (true, true));
        assert_eq!(e((0, -1), (0, -1), true), (false, false));
        // The activation alone changes nothing.
        let o = output_edge(sig(0.2, 1, 0), sig(0.9, 1, 0), false);
        assert_eq!((o.on, o.visit), (false, false));
    }

    #[test]
    fn order_is_topological_and_breaks_cycles_by_index() {
        let off = sig(0., 0, -1);
        // 4 -> 1 -> 0, and 2 on its own: owners first, then by index.
        let mut w = world(vec![
            switch(&[(off, &[])]),
            switch(&[(off, &[(0, 0)])]),
            switch(&[(off, &[])]),
            ThingState::default(),
            switch(&[(off, &[(1, 0)])]),
        ]);
        assert_eq!(w.switch_order(), [2, 4, 1, 0]);
        // 0 -> 1 -> 2 -> 1 (a cycle of 1 and 2), 2 -> 3, 3 -> 3 (itself).
        let mut w = world(vec![
            switch(&[(off, &[(1, 0)])]),
            switch(&[(off, &[(2, 0)])]),
            switch(&[(off, &[(1, 1), (3, 0)])]),
            switch(&[(off, &[(3, 1)])]),
        ]);
        assert_eq!(w.switch_order(), [0, 1, 2, 3]);
        // A pure cycle 0 -> 2 -> 1 -> 0 starts at the lowest index and then follows it.
        let mut w = world(vec![
            switch(&[(off, &[(2, 0)])]),
            switch(&[(off, &[(0, 0)])]),
            switch(&[(off, &[(1, 0)])]),
        ]);
        assert_eq!(w.switch_order(), [0, 2, 1]);
        // Kept until the inputs are wired again.
        w.things[0].switch = None;
        assert_eq!(w.switch_order(), [0, 2, 1]);
        w.wire_inputs();
        assert_eq!(w.switch_order(), [2, 1]);
    }

    #[test]
    fn stored_signals_raise_nothing_and_a_pulse_goes_through_the_pass() {
        // Output 0 off, to Things 1 and 2; output 1 on (player 3), to Thing 1.
        let mut w = world(vec![
            switch(&[
                (sig(0., 0, -1), &[(1, 0), (2, 0)]),
                (sig(1., -1, 3), &[(1, 1)]),
            ]),
            ThingState::default(),
            ThingState::default(),
        ]);
        w.things[0].switch.as_mut().unwrap().outputs.push(None);
        assert!(w.update_switches().is_empty());
        assert!(w.update_switches().is_empty());
        assert_eq!(w.things[0].switch.as_ref().unwrap().memory.updates, 2);

        w.switch_pulse = Some(true);
        let v = w.update_switches();
        let seen: Vec<_> = v
            .iter()
            .map(|v| (v.output, v.target, v.port, v.on))
            .collect();
        // Output 0 rises; output 1 flips sign while on.
        assert_eq!(seen, [(0, 1, 0, true), (0, 2, 0, true), (1, 1, 1, true)]);
        assert_eq!(v[2].signal, sig(1., 1, 3));
        // Written where the input natives read it.
        let cx = SwitchContext {
            world: &w,
            thing: 1,
            switch: w.things[0].switch.as_ref().unwrap(),
        };
        assert_eq!(cx.input(0), Some(sig(1., 1, -1)));
        assert_eq!((cx.input(1), cx.input(2)), (Some(sig(1., 1, 3)), None));
        assert!(w.update_switches().is_empty());

        w.switch_pulse = Some(false);
        let v = w.update_switches();
        assert!(v.len() == 3 && v.iter().all(|v| !v.on));
        // Back to the stored signals: output 1 rises again, output 0 stays off.
        w.switch_pulse = None;
        let v = w.update_switches();
        assert_eq!((v.len(), v[0].output, v[0].on), (1, 1, true));
        assert_eq!(v[0].signal, sig(1., -1, 3));
    }

    #[test]
    fn force_refires_once() {
        let mut w = world(vec![
            switch(&[(sig(1., 1, -1), &[(1, 0)])]),
            ThingState::default(),
        ]);
        w.things[0].switch.as_mut().unwrap().force = true;
        let v = w.update_switches();
        assert_eq!((v.len(), v[0].on), (1, true));
        assert!(w.update_switches().is_empty());
    }

    #[test]
    fn a_deactivated_thing_forces_its_outputs_off() {
        let mut w = world(vec![
            switch(&[(sig(0.5, 1, 2), &[(1, 0)]), (sig(0.5, 0, -1), &[(1, 1)])]),
            ThingState::default(),
        ]);
        w.set_deactivated(0, true);
        // Also over a pulse.
        w.switch_pulse = Some(true);
        let v = w.update_switches();
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].output, v[0].on, v[0].signal), (0, false, SIGNAL_OFF));
        let outputs = &w.things[0].switch.as_ref().unwrap().outputs;
        assert!(outputs.iter().flatten().all(|o| o.signal == SIGNAL_OFF));
        assert!(w.update_switches().is_empty());
        // Active again: back to the stored value, a rising edge.
        w.switch_pulse = None;
        w.set_deactivated(0, false);
        let v = w.update_switches();
        assert_eq!((v.len(), v[0].on, v[0].signal), (1, true, sig(0.5, 1, 2)));
    }

    #[test]
    fn whitelist() {
        for g in [
            18256, 25580, 27452, 65238, 79156, 87281, 98497, 125169, 125367, 125391, 125392,
            158628, 188288, 242022,
        ] {
            assert_eq!(reaction_for_script(g), Some(Reaction::Bool), "{g}");
        }
        assert_eq!(reaction_for_script(18420), Some(Reaction::BoolFloat));
        for g in [0, 11599, 18257, 61034] {
            assert_eq!(reaction_for_script(g), None, "{g}");
        }
        // The kind tests that come before the script.
        let thing = |ty: Option<i32>, parts: &[&'static str]| ThingState {
            switch: ty.map(|switch_type| SwitchState {
                switch_type,
                ..Default::default()
            }),
            parts: parts.to_vec(),
            ..Default::default()
        };
        assert_eq!(kind_before_script(&thing(None, &["SCRIPT", "POS"])), None);
        for ty in [0, 4, 14, 17, 23, 36, 44, 65] {
            assert_eq!(kind_before_script(&thing(Some(ty), &[])), Some("SWITCH"));
        }
        // A magic mouth, a wormhole switch and the types past the table go on to the script.
        for ty in [-1, 7, 24, 34, 45, 57, 64, SWITCH_KIND_TYPES, 70] {
            assert_eq!(
                kind_before_script(&thing(Some(ty), &["SCRIPT"])),
                None,
                "{ty}"
            );
        }
        assert_eq!(kind_before_script(&thing(Some(3), &[])), Some("SWITCH"));
        assert_eq!(kind_before_script(&thing(Some(3), &["SHAPE"])), None);
        assert_eq!(
            kind_before_script(&thing(Some(34), &["SWITCH_INPUT"])),
            Some("SWITCH_INPUT")
        );
        assert_eq!(
            kind_before_script(&thing(Some(4), &["EMITTER", "JOINT"])),
            Some("EMITTER")
        );
    }
    /// A battery (type 28) holding `value`, wired to the given (Thing, port) inputs.
    fn battery(value: SwitchInput, targets: &[(u32, i32)]) -> ThingState {
        let mut t = switch(&[(value, targets)]);
        let s = t.switch.as_mut().unwrap();
        s.switch_type = 28;
        s.memory.manual = value;
        t
    }

    /// A switch of type `ty` with `outputs` unwired outputs that store "off".
    fn logic(ty: i32, outputs: usize) -> ThingState {
        let mut t = switch(&vec![(SIGNAL_OFF, &[][..]); outputs]);
        t.switch.as_mut().unwrap().switch_type = ty;
        t
    }

    fn state(w: &mut LevelWorld, thing: usize) -> &mut SwitchState {
        w.things[thing].switch.as_mut().unwrap()
    }

    fn out(w: &LevelWorld, thing: usize, output: usize) -> SwitchInput {
        w.things[thing].switch.as_ref().unwrap().outputs[output]
            .as_ref()
            .unwrap()
            .signal
    }

    fn set_battery(w: &mut LevelWorld, thing: usize, value: SwitchInput) {
        state(w, thing).memory.manual = value;
    }

    /// One gate of type `ty` fed by a battery per `Some` input (port = position), after
    /// one pass: the gate's outputs. `tweak` adjusts the gate first.
    fn gate(
        ty: i32,
        outputs: usize,
        inputs: &[Option<SwitchInput>],
        tweak: impl Fn(&mut SwitchState),
    ) -> Vec<SwitchInput> {
        let g = inputs.len() as u32;
        let mut things: Vec<ThingState> = inputs
            .iter()
            .enumerate()
            .map(|(port, v)| match v {
                Some(v) => battery(*v, &[(g, port as i32)]),
                None => ThingState::default(),
            })
            .collect();
        things.push(logic(ty, outputs));
        tweak(things[g as usize].switch.as_mut().unwrap());
        let mut w = world(things);
        w.update_switches();
        (0..outputs).map(|o| out(&w, g as usize, o)).collect()
    }

    const ON: SwitchInput = SwitchInput {
        activation: 1.,
        ternary: 1,
        player: 0,
    };

    #[test]
    fn pass_through_types_and_the_inverted_rule() {
        let half = sig(0.25, 1, 2);
        // NOT, NOP and the circuit node pass port 0 through; unwired reads off.
        for ty in [23, 24, 40] {
            assert_eq!(gate(ty, 1, &[Some(half)], |_| {}), [half], "{ty}");
            assert_eq!(gate(ty, 1, &[None], |_| {}), [SIGNAL_OFF], "{ty}");
        }
        // A NOT gate negates only when its Inverted flag is set; the player is kept.
        let inverted = |s: &mut SwitchState| s.inverted = true;
        assert_eq!(gate(23, 1, &[Some(half)], inverted), [sig(0.75, 0, 2)]);
        assert_eq!(gate(23, 1, &[None], inverted), [sig(1., 1, -1)]);
        assert_eq!(
            gate(23, 1, &[Some(sig(-0.25, -1, 2))], inverted),
            [sig(0.75, 0, 2)]
        );
        // The battery gives its stored value, inverted or not.
        let mut w = world(vec![battery(half, &[])]);
        w.update_switches();
        assert_eq!(out(&w, 0, 0), half);
        state(&mut w, 0).inverted = true;
        w.update_switches();
        assert_eq!(out(&w, 0, 0), sig(0.75, 0, 2));
    }

    #[test]
    fn and_gate_modes() {
        let ports = |n| move |s: &mut SwitchState| s.bullets_required = n;
        let (a, b, c) = (sig(0.5, 1, 3), sig(-0.75, -1, 4), sig(0.25, 1, 5));
        // Smallest magnitude, the sign of the product.
        assert_eq!(
            gate(14, 1, &[Some(a), Some(b)], ports(2)),
            [sig(-0.5, -1, 3)]
        );
        assert_eq!(
            gate(14, 1, &[Some(a), Some(b), Some(c)], ports(3)),
            [sig(-0.25, -1, 3)]
        );
        // A port past the gate's count is not read; an unwired one switches it off.
        assert_eq!(
            gate(14, 1, &[Some(a), Some(b), Some(c)], ports(2)),
            [sig(-0.5, -1, 3)]
        );
        assert_eq!(gate(14, 1, &[Some(a), None], ports(2)), [sig(0., 0, -1)]);
        assert_eq!(
            gate(14, 1, &[Some(a), Some(b), None], ports(3)),
            [sig(0., 0, -1)]
        );
        // The multiplying mode.
        let multiply = |s: &mut SwitchState| (s.bullets_required, s.gate_mode) = (3, 1);
        assert_eq!(
            gate(14, 1, &[Some(a), Some(b), Some(c)], multiply),
            [sig(-0.09375, -1, 4)]
        );
        // Inverted: a NAND.
        let nand = |s: &mut SwitchState| (s.bullets_required, s.inverted) = (2, true);
        assert_eq!(gate(14, 1, &[Some(ON), Some(ON)], nand), [sig(0., 0, 0)]);
        assert_eq!(gate(14, 1, &[Some(ON), None], nand), [sig(1., 1, -1)]);
    }

    #[test]
    fn or_and_xor_gates() {
        let (a, b, c) = (sig(0.5, 1, 3), sig(-0.75, -1, 4), sig(0.25, 0, 5));
        // Largest magnitude; the first input that is on gives the ternary and player.
        assert_eq!(gate(21, 1, &[Some(a), Some(b)], |_| {}), [sig(-0.75, 1, 3)]);
        assert_eq!(gate(21, 1, &[None, Some(b)], |_| {}), [b]);
        assert_eq!(
            gate(21, 1, &[None, None, Some(a)], |_| {}),
            [sig(0.5, 1, 3)]
        );
        assert_eq!(gate(21, 1, &[None, None], |_| {}), [SIGNAL_OFF]);
        // The adding mode, up to the gate's port count.
        let add = |s: &mut SwitchState| (s.bullets_required, s.gate_mode) = (3, 1);
        assert_eq!(
            gate(21, 1, &[Some(a), Some(a), Some(c)], add),
            [sig(1.25, 1, 3)]
        );
        assert_eq!(
            gate(21, 1, &[Some(a), Some(b), None], add),
            [sig(-0.25, 0, 4)]
        );
        // Inverted: a NOR.
        let nor = |s: &mut SwitchState| s.inverted = true;
        assert_eq!(gate(21, 1, &[None, None], nor), [sig(1., 1, -1)]);
        assert_eq!(gate(21, 1, &[Some(ON), None], nor), [sig(0., 0, 0)]);

        // XOR: exactly one input on.
        assert_eq!(gate(22, 1, &[Some(c), Some(b)], |_| {}), [b]);
        assert_eq!(gate(22, 1, &[Some(a), Some(b)], |_| {}), [SIGNAL_OFF]);
        assert_eq!(gate(22, 1, &[Some(c), None], |_| {}), [sig(0.25, 0, -1)]);
        assert_eq!(gate(22, 1, &[], |_| {}), [SIGNAL_OFF]);
        assert_eq!(gate(22, 1, &[Some(a), Some(b)], nor), [sig(1., 1, -1)]);
    }

    #[test]
    fn direction_combiner_and_splitter() {
        let (neg, pos) = (sig(0.25, 1, 3), sig(0.75, 1, 4));
        // Port 1 is the positive input, port 0 the negative one.
        assert_eq!(
            gate(20, 1, &[Some(neg), Some(pos)], |_| {}),
            [sig(0.5, 0, 4)]
        );
        assert_eq!(gate(20, 1, &[Some(neg), None], |_| {}), [sig(-0.25, -1, 3)]);
        assert_eq!(gate(20, 1, &[None, Some(pos)], |_| {}), [sig(0.75, 1, 4)]);
        // Inverted, the combiner negates.
        let inverted = |s: &mut SwitchState| s.inverted = true;
        assert_eq!(
            gate(20, 1, &[None, Some(pos)], inverted),
            [sig(-0.75, -1, 4)]
        );
        // The splitter: output 0 the negative part, output 1 the positive part.
        assert_eq!(
            gate(27, 2, &[Some(sig(-0.5, -1, 2))], |_| {}),
            [sig(0.5, 1, 2), sig(0., 0, 2)]
        );
        assert_eq!(
            gate(27, 2, &[Some(sig(0.5, 1, 2))], |_| {}),
            [sig(0., 0, 2), sig(0.5, 1, 2)]
        );
        assert_eq!(
            gate(27, 2, &[Some(sig(0.5, 1, 2))], inverted),
            [sig(1., 1, 2), sig(0.5, 0, 2)]
        );
    }

    #[test]
    fn constant_and_stored_types() {
        // Types whose signal is not made in the pass read off, whatever the file stores.
        for ty in [7, 11, 26, 31, 33, 46, 56, 64, SWITCH_KIND_TYPES, 80] {
            let mut t = switch(&[(sig(0.5, 1, 2), &[])]);
            t.switch.as_mut().unwrap().switch_type = ty;
            let mut w = world(vec![t]);
            w.update_switches();
            assert_eq!(out(&w, 0, 0), SIGNAL_OFF, "{ty}");
        }
        // Sensors and the random switch keep the stored signal, also when inverted.
        for ty in [0, 3, 4, 6, 18, 19, 30, 38, 41, 54, 65] {
            let mut t = switch(&[(sig(0.5, 1, 2), &[])]);
            let s = t.switch.as_mut().unwrap();
            (s.switch_type, s.inverted) = (ty, true);
            let mut w = world(vec![t]);
            assert!(w.update_switches().is_empty());
            assert_eq!(out(&w, 0, 0), sig(0.5, 1, 2), "{ty}");
        }
    }

    #[test]
    fn old_joint_deadband_holds_small_moves() {
        let mut w = world(vec![battery(sig(0.5, 1, 0), &[(1, 0)]), logic(24, 1)]);
        state(&mut w, 1).old_joint = true;
        w.update_switches();
        assert_eq!(out(&w, 1, 0), sig(0.5, 1, 0));
        // Within 0.025: held. Past it: followed. Another player: followed.
        set_battery(&mut w, 0, sig(0.52, 1, 0));
        w.update_switches();
        assert_eq!(out(&w, 1, 0), sig(0.5, 1, 0));
        set_battery(&mut w, 0, sig(0.53, 1, 0));
        w.update_switches();
        assert_eq!(out(&w, 1, 0), sig(0.53, 1, 0));
        set_battery(&mut w, 0, sig(0.54, 1, 1));
        w.update_switches();
        assert_eq!(out(&w, 1, 0), sig(0.54, 1, 1));
        // A whole value is taken at once.
        set_battery(&mut w, 0, sig(0.99, 1, 1));
        w.update_switches();
        set_battery(&mut w, 0, sig(1., 1, 1));
        w.update_switches();
        assert_eq!(out(&w, 1, 0), sig(1., 1, 1));
    }

    #[test]
    fn toggle_flips_on_the_rising_edge() {
        let mut w = world(vec![battery(SIGNAL_OFF, &[(1, 0)]), logic(17, 1)]);
        state(&mut w, 1).memory.manual = sig(0., 0, -1);
        let mut seen = Vec::new();
        for value in [SIGNAL_OFF, ON, ON, SIGNAL_OFF, sig(1., 1, 2), SIGNAL_OFF] {
            set_battery(&mut w, 0, value);
            w.update_switches();
            seen.push(out(&w, 1, 0));
        }
        // Flips when the input rises, holds otherwise; takes the input's player.
        let (on, off) = (sig(1., 1, 0), sig(0., 0, 2));
        assert_eq!(seen, [SIGNAL_OFF, on, on, on, off, off]);
        // Inverted, the output is the opposite of the latch.
        state(&mut w, 1).inverted = true;
        w.update_switches();
        assert_eq!(out(&w, 1, 0), sig(1., 1, 2));
        assert_eq!(state(&mut w, 1).memory.manual, off);
    }

    #[test]
    fn counter_counts_edges_and_resets() {
        // Battery 0 -> port 0 (count), battery 1 -> port 1 (reset).
        let mut w = world(vec![
            battery(SIGNAL_OFF, &[(2, 0)]),
            battery(SIGNAL_OFF, &[(2, 1)]),
            logic(15, 1),
        ]);
        state(&mut w, 2).bullets_required = 3;
        let pulse = |w: &mut LevelWorld, thing: usize, value: SwitchInput| {
            set_battery(w, thing, value);
            w.update_switches();
            set_battery(w, thing, SIGNAL_OFF);
            w.update_switches();
            (state(w, 2).memory.count, out(w, 2, 0))
        };
        assert_eq!(pulse(&mut w, 0, ON), (1, sig(1. / 3., 0, 0)));
        assert_eq!(pulse(&mut w, 0, ON), (2, sig(2. / 3., 0, 0)));
        // A negative signal counts down.
        assert_eq!(pulse(&mut w, 0, sig(-1., -1, 1)), (1, sig(1. / 3., 0, 1)));
        assert_eq!(pulse(&mut w, 0, ON).0, 2);
        assert_eq!(pulse(&mut w, 0, ON), (3, sig(1., 1, 0)));
        // Full: further edges change nothing.
        assert_eq!(pulse(&mut w, 0, ON), (3, sig(1., 1, 0)));
        // Reset, with the player of the reset signal.
        assert_eq!(pulse(&mut w, 1, sig(1., 1, 2)), (0, sig(0., 0, 2)));
        // Not below zero.
        assert_eq!(pulse(&mut w, 0, sig(-1., -1, 1)).0, 0);
        // Holding the input on counts once.
        set_battery(&mut w, 0, ON);
        for _ in 0..3 {
            w.update_switches();
        }
        assert_eq!(state(&mut w, 2).memory.count, 1);
    }

    #[test]
    fn selector_steps_and_selects() {
        // Battery 0 -> port 0 (cycle), battery 1 -> port 3 (select output 2).
        let mut w = world(vec![
            battery(SIGNAL_OFF, &[(2, 0)]),
            battery(SIGNAL_OFF, &[(2, 3)]),
            logic(35, 3),
        ]);
        let outputs =
            |w: &LevelWorld| [out(w, 2, 0), out(w, 2, 1), out(w, 2, 2)].map(|s| s.ternary);
        let step = |w: &mut LevelWorld, value: SwitchInput| {
            set_battery(w, 0, value);
            w.update_switches();
            set_battery(w, 0, SIGNAL_OFF);
            w.update_switches();
            outputs(w)
        };
        w.update_switches();
        assert_eq!(outputs(&w), [1, 0, 0]);
        assert_eq!(step(&mut w, ON), [0, 1, 0]);
        assert_eq!(out(&w, 2, 1), sig(1., 1, 0));
        assert_eq!(step(&mut w, ON), [0, 0, 1]);
        assert_eq!(step(&mut w, ON), [1, 0, 0]);
        // A negative signal steps back, wrapping.
        assert_eq!(step(&mut w, sig(-1., -1, 0)), [0, 0, 1]);
        assert_eq!(step(&mut w, sig(-1., -1, 0)), [0, 1, 0]);
        // A direct input selects its output while it is on, and it stays selected.
        set_battery(&mut w, 1, sig(1., 1, 4));
        w.update_switches();
        assert_eq!(outputs(&w), [0, 0, 1]);
        assert_eq!(out(&w, 2, 2), sig(1., 1, 4));
        set_battery(&mut w, 1, SIGNAL_OFF);
        w.update_switches();
        assert_eq!(outputs(&w), [0, 0, 1]);
    }

    fn timer(length: i32, behavior: i32) -> ThingState {
        let mut t = logic(16, 1);
        let s = t.switch.as_mut().unwrap();
        (s.activation_hold_time, s.behavior) = (length, behavior);
        t
    }

    /// Runs `ticks` passes and returns the timer's (position, output ternary) after each.
    fn run(w: &mut LevelWorld, thing: usize, ticks: usize) -> Vec<(f32, i32)> {
        (0..ticks)
            .map(|_| {
                w.update_switches();
                (state(w, thing).memory.timer, out(w, thing, 0).ternary)
            })
            .collect()
    }

    #[test]
    fn timer_runs_free_and_by_its_input() {
        // Unwired: one tick per tick up to its length; the output follows a tick later.
        let mut w = world(vec![timer(3, 0)]);
        assert_eq!(
            run(&mut w, 0, 5),
            [(1., 0), (2., 0), (3., 0), (3., 1), (3., 1)]
        );
        assert_eq!(out(&w, 0, 0), sig(1., 1, -1));

        // Behaviour 0: runs while the input is on, holds while it is off.
        let mut w = world(vec![battery(SIGNAL_OFF, &[(1, 0)]), timer(4, 0)]);
        assert_eq!(run(&mut w, 1, 2), [(0., 0), (0., 0)]);
        set_battery(&mut w, 0, ON);
        assert_eq!(run(&mut w, 1, 2), [(1., 0), (2., 0)]);
        assert_eq!(out(&w, 1, 0), sig(0.25, 0, 0));
        set_battery(&mut w, 0, SIGNAL_OFF);
        assert_eq!(run(&mut w, 1, 2), [(2., 0), (2., 0)]);

        // Behaviour 1: forwards while on, backwards while off.
        let mut w = world(vec![battery(ON, &[(1, 0)]), timer(2, 1)]);
        assert_eq!(run(&mut w, 1, 3), [(1., 0), (2., 0), (2., 1)]);
        set_battery(&mut w, 0, SIGNAL_OFF);
        // The output's ternary holds between the ends.
        assert_eq!(run(&mut w, 1, 4), [(1., 1), (0., 1), (0., 0), (0., 0)]);

        // Behaviour 4: the input's activation is the rate.
        let mut w = world(vec![battery(sig(0.5, 1, 0), &[(1, 0)]), timer(2, 4)]);
        assert_eq!(run(&mut w, 1, 3), [(0.5, 0), (1., 0), (1.5, 0)]);

        // Inverted: on until it is full.
        let mut w = world(vec![timer(2, 0)]);
        state(&mut w, 0).inverted = true;
        assert_eq!(run(&mut w, 0, 4), [(1., 1), (2., 1), (2., 0), (2., 0)]);
    }

    #[test]
    fn timer_edge_behaviours_and_reset() {
        // Behaviour 2: an edge on port 0 starts it; it keeps running after the input
        // goes off. An edge on port 1 resets and stops it.
        let mut w = world(vec![
            battery(SIGNAL_OFF, &[(2, 0)]),
            battery(SIGNAL_OFF, &[(2, 1)]),
            timer(3, 2),
        ]);
        assert_eq!(run(&mut w, 2, 2), [(0., 0), (0., 0)]);
        set_battery(&mut w, 0, sig(1., 1, 5));
        assert_eq!(run(&mut w, 2, 1), [(1., 0)]);
        set_battery(&mut w, 0, SIGNAL_OFF);
        assert_eq!(run(&mut w, 2, 3), [(2., 0), (3., 0), (3., 1)]);
        assert_eq!(out(&w, 2, 0), sig(1., 1, 5));
        set_battery(&mut w, 1, ON);
        // The reset comes before the timer in the pass, so the output falls at once.
        assert_eq!(run(&mut w, 2, 2), [(0., 0), (0., 0)]);
        assert_eq!(state(&mut w, 2).memory.manual, sig(0., 0, 0));

        // Behaviour 3: an edge fills an empty timer and counts it down.
        let mut w = world(vec![battery(SIGNAL_OFF, &[(1, 0)]), timer(2, 3)]);
        w.update_switches();
        set_battery(&mut w, 0, ON);
        assert_eq!(run(&mut w, 1, 4), [(1., 1), (0., 1), (0., 0), (0., 0)]);
    }

    /// Things: 0 the chip's activation battery -> chip port 0; 1 the chip (type 36,
    /// board 2); 2 its board (type 37, child of the chip); 3 a battery on the board
    /// -> toggle port 0; 4 a toggle on the board -> Thing 5 port 0; 5 a plain Thing.
    fn chip_world(activation: SwitchInput) -> LevelWorld {
        let mut chip = logic(36, 1);
        chip.circuit_board = Some(2);
        let mut board = logic(37, 1);
        board.parent = Some(1);
        let mut inner = battery(SIGNAL_OFF, &[(4, 0)]);
        inner.parent = Some(2);
        let mut toggle = switch(&[(SIGNAL_OFF, &[(5, 0)])]);
        toggle.parent = Some(2);
        let s = toggle.switch.as_mut().unwrap();
        (s.switch_type, s.memory.manual) = (17, sig(0., 0, -1));
        let mut things = vec![
            battery(activation, &[(1, 0)]),
            chip,
            board,
            inner,
            toggle,
            ThingState::default(),
        ];
        things[2].first_child = Some(3);
        things[3].next_sibling = Some(4);
        world(things)
    }

    #[test]
    fn microchip_off_forces_its_components_off_and_keeps_their_state() {
        let mut w = chip_world(ON);
        let flags = |w: &LevelWorld| w.things.iter().map(|t| t.deactivated).collect::<Vec<_>>();
        // On: the toggle latches on from the battery inside.
        w.update_switches();
        assert_eq!(flags(&w), [false; 6]);
        set_battery(&mut w, 3, ON);
        let v = w.update_switches();
        assert_eq!(out(&w, 4, 0), sig(1., 1, 0));
        assert!(v.iter().any(|v| v.target == 5 && v.on));
        // The chip's activation goes off: chip, board and both components are
        // deactivated in the same pass and their outputs read off; the falling edge
        // reaches the toggle's target.
        set_battery(&mut w, 0, SIGNAL_OFF);
        let v = w.update_switches();
        assert_eq!(flags(&w), [false, true, true, true, true, false]);
        assert_eq!((out(&w, 3, 0), out(&w, 4, 0)), (SIGNAL_OFF, SIGNAL_OFF));
        assert!(v.iter().any(|v| v.target == 5 && !v.on));
        // The latch is kept, and comes back with the chip.
        assert_eq!(state(&mut w, 4).memory.manual, sig(1., 1, 0));
        set_battery(&mut w, 3, SIGNAL_OFF);
        set_battery(&mut w, 0, ON);
        let v = w.update_switches();
        assert_eq!(flags(&w), [false; 6]);
        assert_eq!(out(&w, 4, 0), sig(1., 1, 0));
        assert!(v.iter().any(|v| v.target == 5 && v.on));
        // A component that is on when its chip comes back raises an edge like any other:
        // the battery inside, on again, flips the toggle once the chip is switched off
        // and on.
        set_battery(&mut w, 3, ON);
        w.update_switches();
        assert_eq!(state(&mut w, 4).memory.manual, sig(0., 0, 0));
        for on in [SIGNAL_OFF, ON] {
            set_battery(&mut w, 0, on);
            w.update_switches();
        }
        assert_eq!(state(&mut w, 4).memory.manual, sig(1., 1, 0));
    }

    #[test]
    fn microchip_start_state_and_port_mirrors() {
        // A chip whose stored activation input is off starts deactivated (INFERRED).
        let mut w = chip_world(SIGNAL_OFF);
        w.prepare_switches();
        let flags: Vec<_> = w.things.iter().map(|t| t.deactivated).collect();
        assert_eq!(flags, [false, true, true, true, true, false]);
        // A chip with nothing on port 0 is always on.
        let mut w = chip_world(SIGNAL_OFF);
        state(&mut w, 0).outputs[0]
            .as_mut()
            .unwrap()
            .targets
            .clear();
        w.wire_inputs();
        w.update_switches();
        assert!(w.things.iter().all(|t| !t.deactivated));

        // Mirrors: the board's output i is the chip's input port i + 1; the chip's
        // output i is the board's input port i.
        let mut chip = logic(36, 2);
        chip.circuit_board = Some(3);
        let mut board = logic(37, 2);
        board.parent = Some(2);
        let outer = sig(0.5, 1, 1);
        let inner = sig(0.25, 1, 2);
        let mut w = world(vec![
            battery(outer, &[(2, 2)]),
            battery(inner, &[(3, 0)]),
            chip,
            board,
        ]);
        w.update_switches();
        assert_eq!([out(&w, 3, 0), out(&w, 3, 1)], [SIGNAL_OFF, outer]);
        assert_eq!([out(&w, 2, 0), out(&w, 2, 1)], [inner, SIGNAL_OFF]);
        // An inverted chip output: standard for a plain source, negated for a signed one.
        state(&mut w, 2).inverted = true;
        w.update_switches();
        assert_eq!(out(&w, 2, 0), sig(0.75, 0, 2));
        state(&mut w, 1).switch_type = 10;
        w.update_switches();
        assert_eq!(out(&w, 2, 0), sig(-0.25, -1, 2));
        // A board owned by a sequencer is not computed: it has the signal its file stores.
        w.things[2].parts.push("SEQUENCER");
        w.update_switches();
        assert_eq!(out(&w, 3, 1), SIGNAL_OFF);
    }

    #[test]
    fn a_wire_against_the_order_reads_last_ticks_value() {
        // 0 -> 1 -> 0: a NOT gate (inverted) feeding a node that feeds it back. Thing 0
        // goes first and reads the node's value of the tick before, so the pair
        // oscillates with a period of two ticks.
        let mut not = switch(&[(SIGNAL_OFF, &[(1, 0)])]);
        let s = not.switch.as_mut().unwrap();
        (s.switch_type, s.inverted) = (23, true);
        let mut node = switch(&[(SIGNAL_OFF, &[(0, 0)])]);
        node.switch.as_mut().unwrap().switch_type = 24;
        let mut w = world(vec![not, node]);
        assert_eq!(w.switch_order(), [0, 1]);
        let seen: Vec<_> = (0..4)
            .map(|_| {
                w.update_switches();
                (out(&w, 0, 0).ternary, out(&w, 1, 0).ternary)
            })
            .collect();
        assert_eq!(seen, [(1, 1), (0, 0), (1, 1), (0, 0)]);
    }
}
