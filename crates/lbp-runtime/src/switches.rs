//! The per-tick switch pass and the script reaction it raises (RScript stage 3b, part 1;
//! docs/level-scripts.md, docs/rscript-plan.md).
//!
//! FOUND in the 01.33 executable:
//! - The world update (0xeacd4) runs the switch pass (0xdd228) once per tick. The pass
//!   orders the level's PSwitches topologically by input wiring, then updates each one in
//!   that order (0x1244a4).
//! - Updating a switch computes each output's new {activation, ternary, player} (0x11bd58,
//!   by switch type), writes it to the output, and applies it to the output's targets
//!   (0x11ddf0) when the edge rule of [`output_edge`] says so.
//! - A Thing's "deactivated" byte (+0x7f) forces its outputs to {0.0, 0, -1}.
//! - Applying a signal to a target dispatches on the target Thing's object kind (byte
//!   +0x7e). For the kinds listed at [`reaction_for_script`] it calls the script function
//!   `SwitchPerformReaction__b` or `SwitchPerformReaction__bf` on the target's own script.
//!
//! This part is the scaffolding: [`compute_output`] returns the value the level file
//! stores for every switch type, so signals do not change unless a caller forces them
//! ([`LevelWorld::switch_pulse`]).
//!
//! Not modelled yet:
//! - The dirty flags. The game re-runs event-driven switch types only when something
//!   marked them dirty and every other type each tick; here every switch runs each tick.
//! - The skip for a Thing whose flag byte +0x78 has bit 1 set (meaning not established).
//! - Outputs without targets that a second test forces off like a deactivated Thing.
//! - The per-type work at the end of a switch's update (types 0, 8/0x27, 0x10, 0x13, 0x1d).
//! - Switch types 0x24/0x25 standing in for their microchip's switch in the ordering.
//! - The other object kinds a signal is applied to (lights, emitters, movers and so on);
//!   only the script reaction is raised.
//! - The activating player the game records on the target before the dispatch.
use crate::{InputWire, LevelWorld, SwitchState};
use lbp_script_vm::natives::SwitchInput;
use std::collections::BTreeSet;

/// The signal of an output that is forced off (FOUND, 0x1244a4: activation 0.0, ternary
/// 0, player -1).
pub const SIGNAL_OFF: SwitchInput = SwitchInput {
    activation: 0.,
    ternary: 0,
    player: -1,
};

/// What a PSwitch remembers between ticks besides its outputs: the slot
/// [`compute_output`] may read and write. One per PSwitch, zeroed when the level is
/// built. The per-type rules (timers, counters, toggles) add their fields here.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchMemory {
    /// Times the switch was updated by the pass.
    pub updates: u64,
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
}

/// The new signal of output `output` of a PSwitch, before the deactivated forcing.
///
/// FOUND: the game computes it in 0x11bd58 through a jump table on the switch type
/// (0x11be00, 0x42 entries; a second table, 0x11bfd4, when PSwitch+0xc5 is set, INFERRED
/// the inverted output).
///
/// This is the one place the per-type rules go. Until they are filled in every type
/// returns the signal the level file stores, so the network holds its saved state.
// One arm on purpose: the per-type arms are added here.
#[allow(clippy::match_single_binding)]
pub fn compute_output(
    cx: &SwitchContext<'_>,
    output: usize,
    _memory: &mut SwitchMemory,
) -> SwitchInput {
    match cx.switch.switch_type {
        _ => cx.stored(output).unwrap_or(SIGNAL_OFF),
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

/// Switch types below this classify their Thing by the switch, not by its script
/// ([`reaction_shadowed_by_switch`]).
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

/// Whether a target's own PSwitch takes its object kind away from its script.
///
/// FOUND (0x3ed2c): the kind tests run in a fixed order and an earlier one wins; a
/// PSwitch part with a type below [`SWITCH_KIND_TYPES`] classifies the Thing by switch
/// type before its script is looked at. INFERRED: none of those switch kinds is one of
/// the script-reaction kinds, so such a Thing gets no reaction call.
pub fn reaction_shadowed_by_switch(switch: Option<&SwitchState>) -> bool {
    switch.is_some_and(|s| s.switch_type < SWITCH_KIND_TYPES)
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

    /// Sets a Thing's "deactivated" flag ([`crate::ThingState::deactivated`]).
    ///
    /// Nothing calls this yet. FOUND (0x409dc, 0x40874): the game works the flag out
    /// from the Thing's parent chain (the microchip or circuit board that holds it and
    /// that board's input), and on a change applies an empty signal to the outputs'
    /// targets at once. Neither is modelled; the next pass forces the outputs off and
    /// raises the falling edge.
    pub fn set_deactivated(&mut self, thing: u32, deactivated: bool) {
        if let Some(t) = self.things.get_mut(thing as usize) {
            t.deactivated = deactivated;
        }
    }

    /// One tick of one PSwitch: computes and writes each output's signal and returns the
    /// targets to apply it to, in output then target order. Null outputs and targets with
    /// a null Thing are skipped.
    ///
    /// FOUND (0x1244a4): per output, the new signal is [`compute_output`], or
    /// [`SIGNAL_OFF`] when the Thing is deactivated (+0x7f); it is written to the output,
    /// which is what the switch-input natives read; the targets are visited by the rule
    /// of [`output_edge`]; the force flag (PSwitch+0xe3) is cleared at the end.
    /// The old signal on the first tick is the stored one, so a level whose signals do
    /// not change raises nothing (INFERRED: the game does not fire on load either; the
    /// connect-time application at 0x124c68 is a separate path, not modelled).
    pub fn update_switch(&mut self, thing: u32) -> Vec<TargetVisit> {
        let mut visits = Vec::new();
        let i = thing as usize;
        let Some(s) = self.things.get_mut(i).and_then(|t| t.switch.as_mut()) else {
            return visits;
        };
        let (count, force) = (s.outputs.len(), s.force);
        let mut memory = std::mem::take(&mut s.memory);
        memory.updates += 1;
        let deactivated = self.things[i].deactivated;
        for output in 0..count {
            let Some(switch) = self.things[i].switch.as_ref() else {
                break;
            };
            let Some(stored) = switch.outputs[output].as_ref().map(|o| o.stored) else {
                continue;
            };
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
            let Some(out) = self.things[i]
                .switch
                .as_mut()
                .and_then(|s| s.outputs[output].as_mut())
            else {
                continue;
            };
            let old = std::mem::replace(&mut out.signal, new);
            let edge = output_edge(old, new, force);
            if !edge.visit {
                continue;
            }
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
        }
        if let Some(s) = self.things[i].switch.as_mut() {
            s.memory = memory;
            s.force = false;
        }
        visits
    }

    /// The whole pass without the script calls: every switch in [`Self::switch_order`],
    /// returning all the target visits. [`crate::LevelScripts::switch_reactions`] is the
    /// pass with the reactions called switch by switch.
    pub fn update_switches(&mut self) -> Vec<TargetVisit> {
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
        assert!(!reaction_shadowed_by_switch(None));
        let mut s = SwitchState::default();
        assert!(reaction_shadowed_by_switch(Some(&s)));
        s.switch_type = SWITCH_KIND_TYPES;
        assert!(!reaction_shadowed_by_switch(Some(&s)));
    }
}
