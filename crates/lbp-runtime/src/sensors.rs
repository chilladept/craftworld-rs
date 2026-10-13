//! The tag sensor and the player sensor of the switch pass (docs/level-scripts.md part 15).
//!
//! FOUND in the 01.33 executable (tag sensor 0x118d5c, player sensor 0x119910, their
//! ternary rule 0x11873c, the tag map 0xceac0 / 0xd5c1c, the tag's signal 0x12f6f4, the
//! team filter 0xf9a64):
//! - Both run every tick at their place in the pass order and give every output of the
//!   switch the same signal; the inversion of an inverted switch is applied afterwards
//!   ([`crate::compute_output`]), also to "off".
//! - Distances are between world positions, x and y only.
//!
//! Not modelled:
//! - The cone of a sensor whose AngleRange is below 180: the test is known (the cosine of
//!   the angle to a direction against cos(AngleRange)), the direction is not, so every
//!   direction passes (INFERRED choice).
//! - The layer test of a sensor whose IncludeTouching is not 0: it needs the z span of the
//!   sensor's and the tag's body, which the runtime does not keep, so every layer passes
//!   (INFERRED choice).
//! - A sensor or tag on a player's cursor; the body's owning player (the player a tag
//!   without one is given, and the "own" and "other" team filters of a versus level).
//! - The body centre the player sensor measures between: the Thing positions are used.
//! - A tag skipped by a Thing flag in one game state (the meaning of both not settled).
//! - Players out of play and DetectUnspawnedPlayers on a player sensor (players waiting
//!   at a checkpoint): the world has neither state.
use crate::switches::{clamp, held_ternary, wired, SwitchContext, SIGNAL_OFF};
use crate::{LevelWorld, SwitchState, TagRecord};
use lbp_script_vm::natives::SwitchInput;

/// The signal of a tag with nothing wired to it (FOUND, 0x12f6f4).
const TAG_ON: SwitchInput = SwitchInput {
    activation: 1.,
    ternary: 1,
    player: -1,
};

impl LevelWorld {
    /// The Thing's microchip Thing when it has one, else the Thing itself: the Thing a
    /// sensor or a tag is placed by (FOUND: Thing+0x5c when set; which chip, see
    /// [`LevelWorld::microchip_of`]).
    pub fn sensor_proxy(&self, thing: u32) -> u32 {
        self.microchip_of(thing).unwrap_or(thing)
    }

    /// World x and y of a Thing with a PPos.
    fn point(&self, thing: u32) -> Option<[f32; 2]> {
        let m = self.things.get(thing as usize)?.world_matrix?;
        Some([m[12], m[13]])
    }

    /// Builds the world's tag map again: per (colour, name) the tags with their
    /// positions, by ascending x.
    ///
    /// FOUND (0xd5c1c, the first call of the switch pass 0xdd228): every record's x and y
    /// are taken from the tag's proxy Thing and each key's array is sorted by x, once per
    /// tick before any switch is updated. So all sensors of a tick see the tags where
    /// they were when the pass started. A tag whose proxy has no PPos is left out here
    /// (the game drops it in the sensor, 0x118d5c).
    pub fn refresh_tags(&mut self) {
        let mut tags = std::mem::take(&mut self.tags);
        for list in tags.values_mut() {
            list.clear();
        }
        for (i, t) in self.things.iter().enumerate() {
            let (Some(key), thing) = (t.switch_key, i as u32) else {
                continue;
            };
            if let Some([x, y]) = self.point(self.sensor_proxy(thing)) {
                tags.entry(key).or_default().push(TagRecord { x, y, thing });
            }
        }
        tags.retain(|_, list| !list.is_empty());
        for list in tags.values_mut() {
            list.sort_by(|a, b| a.x.total_cmp(&b.x));
        }
        self.tags = tags;
    }

    /// The signal a tag carries.
    ///
    /// FOUND (0x12f6f4): off while the tag's Thing is deactivated; else the signal of the
    /// output wired to the Thing's input port 0, as it stands at that moment of the pass;
    /// with nothing wired {1.0, 1, -1}.
    pub fn tag_signal(&self, tag: u32) -> SwitchInput {
        match self.things.get(tag as usize) {
            Some(t) if !t.deactivated => wired(self, tag, 0).map_or(TAG_ON, |(_, s)| s),
            _ => SIGNAL_OFF,
        }
    }
}

/// Whether a distance (squared) lies in the sensor's range.
///
/// FOUND (0x1192f4, 0x119f50): MinRadius^2 <= d^2 <= Radius^2, compared as squares; no
/// test at all with a negative Radius.
fn in_range(s: &SwitchState, d2: f32) -> bool {
    s.radius < 0. || (s.min_radius * s.min_radius <= d2 && d2 <= s.radius * s.radius)
}

/// How close a candidate at distance `d` is, 1 at the sensor; not clamped.
///
/// FOUND (0x1192f4, 0x119f50; the constant 0.75 read from the executable): 1.0 with a
/// negative Radius; (1 - d / Radius) / 0.75 with MinRadius 0; else
/// 1 - (d - MinRadius) / (Radius - MinRadius).
fn closeness(s: &SwitchState, d: f32) -> f32 {
    if s.radius < 0. {
        1.
    } else if s.min_radius == 0. {
        (1. - d / s.radius) / 0.75
    } else {
        1. - (d - s.min_radius) / (s.radius - s.min_radius)
    }
}

/// The ternary of a sensor's activation.
///
/// FOUND (0x11873c; 3.75, 0.25 and 0.75 read from the executable): with s = activation *
/// 3.75, off at s <= 0.25, on at s >= 0.75, in between what output 0 holds. With
/// MinRadius 0 that is on within 0.85 of the Radius and off beyond 0.95 of it.
fn sensor_ternary(cx: &SwitchContext<'_>, activation: f32) -> i32 {
    let s = activation * 3.75;
    if s <= 0.25 {
        0
    } else if s >= 0.75 {
        1
    } else {
        held_ternary(cx)
    }
}

/// The team filter of a tag sensor: whether the tag signal's player passes.
///
/// FOUND (0xf9a64), by the PWorld's game mode. `owner` is the player who owns the
/// sensor's body.
/// - Mode 1 (versus): filters 0, 3 and 4 pass; 1 passes the owner, 2 any other player;
///   no player fails both.
/// - Mode 5 (teams): 0 passes; 1 the owner's team, 2 another team, 3 team 0, 4 team 1;
///   no player fails these four.
/// - Any other mode: 0, 1, 3 and 4 pass, 2 fails.
///
/// INFERRED: a filter value past 4 fails in modes 1 and 5 as well.
fn team_filter(world: &LevelWorld, player: i32, filter: i32, owner: i32) -> bool {
    let records = &world.player_records;
    let team = |p: i32| {
        records
            .iter()
            .find(|r| r.number == p)
            .map_or(-1, |r| r.team)
    };
    match (world.pworld.game_mode, filter) {
        (_, 0) | (1, 3 | 4) => true,
        (1, 1) => player != -1 && player == owner,
        (1, 2) => player != -1 && player != owner,
        (5, 1) => player != -1 && team(player) == team(owner),
        (5, 2) => player != -1 && team(player) != team(owner),
        (5, 3) => player != -1 && team(player) == 0,
        (5, 4) => player != -1 && team(player) == 1,
        (1 | 5, _) => false,
        (_, f) => matches!(f, 1 | 3 | 4),
    }
}

/// The body's owning player, which the runtime does not keep: nobody.
const NO_OWNER: i32 = -1;

/// The tag sensor (switch type 4), before the inversion.
///
/// FOUND (0x118d5c):
/// - The tags are those of the world's tag map under the sensor's colour and name
///   ([`LevelWorld::refresh_tags`]). With a Radius of 0 or more only the tags whose x lies
///   within the Radius of the sensor's x are looked at, by ascending x.
/// - The sensor's point is the position of its proxy Thing, a tag's the one in the map.
/// - KeySensorMode 1 hands on the tag's own signal; any other mode measures closeness.
/// - Per tag, with its signal ([`LevelWorld::tag_signal`]): in mode 1 a tag whose
///   activation is 0 is skipped, and one whose ternary is not 0 is counted and its
///   ternary remembered, before the tests that follow; in the other modes a tag whose
///   ternary is 0 is skipped. Then the team filter on the signal's player; a tag on the
///   sensor's own proxy is skipped unless DetectUnspawnedPlayers is set; then the range.
///   In the closeness modes the signal's activation becomes the closeness.
/// - The tags that pass are kept by falling magnitude of activation (a later tag goes
///   before an equal earlier one), cut to BulletsRequired entries. Fewer than that, or
///   BulletsRequired 0: off.
/// - The result comes from the last kept entry, the BulletsRequired-th strongest, with
///   its player. Mode 1: its activation as it is and its ternary; a ternary of 0 is
///   replaced by the remembered one when at least BulletsRequired tags were counted.
///   Other modes: the closeness clamped to 0..1 with the ternary of [`sensor_ternary`].
///
/// INFERRED: a negative BulletsRequired gives off.
pub(crate) fn tag_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let me = w.sensor_proxy(cx.thing);
    let (Some(tags), Some([sx, sy])) = (w.tags.get(&s.key), w.point(me)) else {
        return SIGNAL_OFF;
    };
    let near = if s.radius >= 0. {
        let from = tags.partition_point(|r| r.x < sx - s.radius);
        let to = tags.partition_point(|r| r.x <= sx + s.radius);
        &tags[from..to.max(from)]
    } else {
        &tags[..]
    };
    let signal_mode = s.key_sensor_mode == 1;
    let needed = usize::try_from(s.bullets_required).unwrap_or(0);
    let mut best: Vec<SwitchInput> = Vec::new();
    let (mut counted, mut last_ternary) = (0usize, 0);
    for r in near {
        let mut signal = w.tag_signal(r.thing);
        if signal_mode {
            if signal.activation == 0. {
                continue;
            }
            if signal.ternary != 0 {
                counted += 1;
                last_ternary = signal.ternary;
            }
        } else if signal.ternary == 0 {
            continue;
        }
        if !team_filter(w, signal.player, s.team_filter, NO_OWNER) {
            continue;
        }
        if w.sensor_proxy(r.thing) == me && !s.detect_unspawned_players {
            continue;
        }
        let (dx, dy) = (r.x - sx, r.y - sy);
        let d2 = dx * dx + dy * dy;
        if !in_range(s, d2) {
            continue;
        }
        if !signal_mode {
            signal.activation = closeness(s, d2.sqrt());
        }
        let strength = signal.activation.abs();
        let at = best.iter().position(|b| b.activation.abs() <= strength);
        best.insert(at.unwrap_or(best.len()), signal);
        best.truncate(needed);
    }
    let Some(e) = best.get(needed.wrapping_sub(1)).copied() else {
        return SIGNAL_OFF;
    };
    if signal_mode {
        let held = e.ternary == 0 && counted >= needed;
        return SwitchInput {
            ternary: if held { last_ternary } else { e.ternary },
            ..e
        };
    }
    let activation = clamp(e.activation, 0., 1.);
    SwitchInput {
        activation,
        ternary: sensor_ternary(cx, activation),
        player: e.player,
    }
}

/// The player sensor (switch type 3), before the inversion.
///
/// FOUND (0x119910):
/// - The candidates are the players' Sackboys in the PWorld's list, in its order. With
///   PlayerMode 0 only the leader's counts. Each one that counts is "eligible", in range
///   or not.
/// - The squared distances of those in range are kept ascending, cut to 4 entries with
///   RequireAll and to BulletsRequired without. The output's player is the nearest one.
/// - Needed in range: every eligible player with RequireAll, else BulletsRequired.
///   Fewer, or none needed: off. The activation is the closeness of the last one needed,
///   clamped to 0..1 (1.0 with a negative Radius), with the ternary of
///   [`sensor_ternary`].
///
/// INFERRED: the leader is the first player record's number (as
/// `Game.GetPartyLeadersPlayerNumber__`); the points are the positions of the sensor's
/// proxy Thing and of the Sackboy's Thing (the game takes the centre of each one's body);
/// a sensor without a position gives off.
pub(crate) fn player_sensor(cx: &SwitchContext<'_>) -> SwitchInput {
    let (s, w) = (cx.switch, cx.world);
    let Some([sx, sy]) = w.point(w.sensor_proxy(cx.thing)) else {
        return SIGNAL_OFF;
    };
    let leader = w.player_records.first().map(|r| r.number);
    let bullets = usize::try_from(s.bullets_required).unwrap_or(0);
    let cap = if s.require_all { 4 } else { bullets };
    let mut nearest: Vec<f32> = Vec::new();
    let (mut player, mut eligible) = (-1, 0usize);
    for p in &w.players {
        if s.player_mode == 0 && Some(p.number) != leader {
            continue;
        }
        eligible += 1;
        let Some([px, py]) = w.point(p.thing) else {
            continue;
        };
        let (dx, dy) = (px - sx, py - sy);
        let d2 = dx * dx + dy * dy;
        if !in_range(s, d2) {
            continue;
        }
        if nearest.first().is_none_or(|&n| d2 < n) {
            player = p.number;
        }
        let at = nearest.iter().position(|&n| n >= d2);
        nearest.insert(at.unwrap_or(nearest.len()), d2);
        nearest.truncate(cap);
    }
    let needed = if s.require_all { eligible } else { bullets };
    let Some(d2) = nearest.get(needed.wrapping_sub(1)) else {
        return SIGNAL_OFF;
    };
    let activation = clamp(closeness(s, d2.sqrt()), 0., 1.);
    SwitchInput {
        activation,
        ternary: sensor_ternary(cx, activation),
        player,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SwitchOutputState, TagKey, ThingState};

    fn sig(activation: f32, ternary: i32, player: i32) -> SwitchInput {
        SwitchInput {
            activation,
            ternary,
            player,
        }
    }

    fn at(x: f32, y: f32) -> Option<[f32; 16]> {
        let mut m = [0.; 16];
        (m[0], m[5], m[10], m[15]) = (1., 1., 1., 1.);
        (m[12], m[13]) = (x, y);
        Some(m)
    }

    /// A switch of type `ty` at (x, y) with one output that stores "off" and feeds the
    /// given (Thing, port) inputs.
    fn switch(ty: i32, x: f32, y: f32, targets: &[(u32, i32)]) -> ThingState {
        ThingState {
            world_matrix: at(x, y),
            switch: Some(SwitchState {
                switch_type: ty,
                outputs: vec![Some(SwitchOutputState {
                    signal: SIGNAL_OFF,
                    stored: SIGNAL_OFF,
                    targets: targets.iter().map(|&(t, p)| (Some(t), p)).collect(),
                })],
                radius: 100.,
                angle_range: 180.,
                bullets_required: 1,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    const KEY: TagKey = TagKey { colour: 2, name: 0 };

    /// A tag sensor at the origin (radius 100, one tag needed) looking for [`KEY`].
    fn tag_sensor_thing() -> ThingState {
        let mut t = switch(4, 0., 0., &[]);
        t.switch.as_mut().unwrap().key = KEY;
        t
    }

    fn tag(x: f32, y: f32) -> ThingState {
        ThingState {
            world_matrix: at(x, y),
            switch_key: Some(KEY),
            ..Default::default()
        }
    }

    /// A battery holding `value`, wired to the given inputs.
    fn battery(value: SwitchInput, targets: &[(u32, i32)]) -> ThingState {
        let mut t = switch(28, 0., 0., targets);
        t.switch.as_mut().unwrap().memory.manual = value;
        t
    }

    fn world(things: Vec<ThingState>) -> LevelWorld {
        let mut w = LevelWorld {
            things,
            ..Default::default()
        };
        w.wire_inputs();
        w
    }

    fn state(w: &mut LevelWorld, thing: usize) -> &mut SwitchState {
        w.things[thing].switch.as_mut().unwrap()
    }

    /// One pass, then output 0 of Thing `thing`.
    fn pass(w: &mut LevelWorld, thing: usize) -> SwitchInput {
        w.update_switches();
        w.things[thing].switch.as_ref().unwrap().outputs[0]
            .as_ref()
            .unwrap()
            .signal
    }

    fn move_to(w: &mut LevelWorld, thing: usize, x: f32, y: f32) {
        w.things[thing].world_matrix = at(x, y);
    }

    #[test]
    fn tag_sensor_range_edges_and_the_hold_ring() {
        let mut w = world(vec![tag_sensor_thing(), tag(0., 0.)]);
        // The sensor's own Thing is its proxy here; the tag is another Thing.
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        // On up to 0.85 of the radius, y counts as x does.
        move_to(&mut w, 1, 0., 85.);
        let s = pass(&mut w, 0);
        assert!((s.activation - 0.2).abs() < 1e-5 && s.ternary == 1);
        // Between 0.85 and 0.95 the ternary holds: on while it was on.
        move_to(&mut w, 1, 90., 0.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // Off from 0.95, then the ring holds "off" on the way back in.
        move_to(&mut w, 1, 96., 0.);
        let s = pass(&mut w, 0);
        assert!(s.activation > 0. && s.ternary == 0);
        move_to(&mut w, 1, 90., 0.);
        assert_eq!(pass(&mut w, 0).ternary, 0);
        move_to(&mut w, 1, 84., 0.);
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // The edge itself is in range (activation 0); past it there is no candidate.
        move_to(&mut w, 1, 100., 0.);
        assert_eq!(pass(&mut w, 0), sig(0., 0, -1));
        move_to(&mut w, 1, 100.5, 0.);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // A negative radius has no range: full strength anywhere.
        state(&mut w, 0).radius = -1.;
        move_to(&mut w, 1, 5000., -900.);
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        // Another colour or name is another tag.
        w.things[1].switch_key = Some(TagKey { colour: 3, name: 0 });
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.things[1].switch_key = Some(TagKey { colour: 2, name: 1 });
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
    }

    #[test]
    fn tag_sensor_min_radius() {
        let mut w = world(vec![tag_sensor_thing(), tag(10., 0.)]);
        state(&mut w, 0).min_radius = 20.;
        // Inside the inner edge: not a candidate.
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // On the inner edge: full strength; it falls to 0 at the outer edge.
        move_to(&mut w, 1, 20., 0.);
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        move_to(&mut w, 1, 0., -60.);
        assert_eq!(pass(&mut w, 0), sig(0.5, 1, -1));
        move_to(&mut w, 1, 100., 0.);
        assert_eq!(pass(&mut w, 0), sig(0., 0, -1));
    }

    #[test]
    fn tag_sensor_takes_the_nth_strongest() {
        let mut w = world(vec![
            tag_sensor_thing(),
            tag(-25., 0.),
            tag(40., 0.),
            tag(70., 0.),
        ]);
        let strength = |d: f32| (1. - d / 100.) / 0.75;
        assert_eq!(pass(&mut w, 0).activation, 1.);
        state(&mut w, 0).bullets_required = 2;
        assert!((pass(&mut w, 0).activation - strength(40.)).abs() < 1e-5);
        state(&mut w, 0).bullets_required = 3;
        assert!((pass(&mut w, 0).activation - strength(70.)).abs() < 1e-5);
        // More needed than there are, none needed, or a negative count: off.
        for n in [4, 0, -1] {
            state(&mut w, 0).bullets_required = n;
            assert_eq!(pass(&mut w, 0), SIGNAL_OFF, "{n}");
        }
    }

    #[test]
    fn tag_signal_unwired_wired_and_deactivated() {
        // Thing 0 the sensor, 1 a tag, 2 a battery on the tag's port 0.
        let mut w = world(vec![
            tag_sensor_thing(),
            tag(50., 0.),
            battery(sig(0.5, 1, 3), &[(1, 0)]),
        ]);
        w.things[2].world_matrix = None;
        assert_eq!(w.tag_signal(1), sig(0., 0, -1));
        // The battery powers up in the pass; the sensor (Thing 0) ran before it.
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        assert_eq!(w.tag_signal(1), sig(0.5, 1, 3));
        // The tag's player is handed on, the activation is the closeness.
        let s = pass(&mut w, 0);
        assert!((s.activation - 0.5 / 0.75).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (1, 3));
        // A tag that is off is not seen.
        state(&mut w, 2).memory.manual = sig(0.5, 0, 3);
        w.update_switches();
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // Nothing wired: on, no player.
        w.things[2].switch = None;
        w.wire_inputs();
        assert_eq!(w.tag_signal(1), sig(1., 1, -1));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        // A deactivated tag is off, wired or not.
        w.set_deactivated(1, true);
        assert_eq!(w.tag_signal(1), SIGNAL_OFF);
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
    }

    #[test]
    fn tag_sensor_signal_mode_hands_on_the_tags_signal() {
        // Tags 1 and 2 fed by batteries 3 and 4. The sensor is first in the pass order,
        // so it sees a battery's value one pass later.
        let mut w = world(vec![
            tag_sensor_thing(),
            tag(10., 0.),
            tag(95., 0.),
            battery(sig(0.25, 1, 5), &[(1, 0)]),
            battery(sig(-0.75, -1, 6), &[(2, 0)]),
        ]);
        (w.things[3].world_matrix, w.things[4].world_matrix) = (None, None);
        state(&mut w, 0).key_sensor_mode = 1;
        w.update_switches();
        // The strongest signal by magnitude, as it is: no closeness, no clamp.
        assert_eq!(pass(&mut w, 0), sig(-0.75, -1, 6));
        state(&mut w, 0).bullets_required = 2;
        assert_eq!(pass(&mut w, 0), sig(0.25, 1, 5));
        // A tag with activation but ternary 0 is kept and not counted: one tag counted
        // is too few for two needed, so its 0 stands.
        state(&mut w, 3).memory.manual = sig(0.25, 0, 5);
        state(&mut w, 0).bullets_required = 1;
        w.update_switches();
        assert_eq!(pass(&mut w, 0), sig(-0.75, -1, 6));
        state(&mut w, 0).bullets_required = 2;
        assert_eq!(pass(&mut w, 0), sig(0.25, 0, 5));
        // A tag outside the circle but inside the x window is still counted, and with
        // enough counted the last counted ternary stands in for the 0.
        move_to(&mut w, 2, 95., 80.);
        state(&mut w, 0).bullets_required = 1;
        assert_eq!(pass(&mut w, 0), sig(0.25, -1, 5));
        // Activation 0 is skipped.
        state(&mut w, 3).memory.manual = sig(0., 1, 5);
        w.update_switches();
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
    }

    #[test]
    fn tag_sensor_inversion_own_chip_and_team_filter() {
        // Thing 0 a chip whose board is Thing 1; the sensor (2) and a tag (3) sit on the
        // board, so both are placed by the chip.
        let chip = ThingState {
            world_matrix: at(0., 0.),
            circuit_board: Some(1),
            ..Default::default()
        };
        let child = |mut t: ThingState| {
            (t.parent, t.world_matrix) = (Some(1), None);
            t
        };
        let mut w = world(vec![
            chip,
            ThingState::default(),
            child(tag_sensor_thing()),
            child(tag(0., 0.)),
        ]);
        w.board_owners.insert(1, 0);
        assert_eq!(w.sensor_proxy(2), 0);
        // The tag on the sensor's own chip is not seen, unless the flag asks for it.
        assert_eq!(pass(&mut w, 2), SIGNAL_OFF);
        state(&mut w, 2).detect_unspawned_players = true;
        assert_eq!(pass(&mut w, 2), sig(1., 1, -1));
        // Inversion comes after the sensor: on becomes off, and "nothing found" on.
        state(&mut w, 2).inverted = true;
        assert_eq!(pass(&mut w, 2), sig(0., 0, -1));
        state(&mut w, 2).detect_unspawned_players = false;
        assert_eq!(pass(&mut w, 2), sig(1., 1, -1));
        // The team filter outside versus and team levels: only 2 rejects.
        let s = state(&mut w, 2);
        (s.inverted, s.detect_unspawned_players) = (false, true);
        for (filter, on) in [(0, 1), (1, 1), (2, 0), (3, 1), (4, 1), (5, 0)] {
            state(&mut w, 2).team_filter = filter;
            assert_eq!(pass(&mut w, 2).ternary, on, "{filter}");
        }
        // Versus: a tag without a player fails "own" and "others".
        w.pworld.game_mode = 1;
        for (filter, on) in [(0, 1), (1, 0), (2, 0), (3, 1), (4, 1)] {
            state(&mut w, 2).team_filter = filter;
            assert_eq!(pass(&mut w, 2).ternary, on, "{filter}");
        }
    }

    #[test]
    fn tags_are_placed_when_the_pass_starts() {
        let mut w = world(vec![tag_sensor_thing(), tag(50., 0.), tag(-30., 0.)]);
        w.update_switches();
        let xs: Vec<_> = w.tags[&KEY].iter().map(|r| (r.x, r.thing)).collect();
        assert_eq!(xs, [(-30., 2), (50., 1)]);
        // A tag without a position leaves the map; an empty key goes with it.
        (w.things[1].world_matrix, w.things[2].world_matrix) = (None, None);
        w.refresh_tags();
        assert!(w.tags.is_empty());
    }

    #[test]
    fn player_sensor_range_hold_and_inversion() {
        let mut w = world(vec![switch(3, 0., 0., &[])]);
        // No player in the level: off, and an inverted sensor is on.
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        state(&mut w, 0).inverted = true;
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        state(&mut w, 0).inverted = false;
        let p = w.add_player([30., 40., 0.], (0., 0.));
        let s = pass(&mut w, 0);
        assert!((s.activation - 0.5 / 0.75).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (1, 0));
        // The hold ring keeps on, then off.
        w.move_player(p, [90., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        w.move_player(p, [97., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 0);
        w.move_player(p, [90., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 0);
        // Inverted: the activation is turned round, and the ring holds the output, read
        // back through the inversion.
        state(&mut w, 0).inverted = true;
        let s = pass(&mut w, 0);
        assert!((s.activation - (1. - 0.1 / 0.75)).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (0, 0));
        w.move_player(p, [97., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        w.move_player(p, [90., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 1);
        w.move_player(p, [10., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0).ternary, 0);
        // Out of range: off before the inversion, so on after it, with no player.
        w.move_player(p, [500., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), sig(1., 1, -1));
        // MinRadius: a player inside the inner edge is not in range.
        let s = state(&mut w, 0);
        (s.inverted, s.min_radius) = (false, 50.);
        w.move_player(p, [10., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.move_player(p, [75., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), sig(0.5, 1, 0));
    }

    #[test]
    fn player_sensor_counts_players() {
        let mut w = world(vec![switch(3, 0., 0., &[])]);
        state(&mut w, 0).player_mode = 1;
        let a = w.add_player([60., 0., 0.], (0., 0.));
        let b = w.add_player([0., 30., 0.], (0., 0.));
        let strength = |d: f32| (1. - d / 100.) / 0.75;
        // One needed: the nearest gives the strength and the player.
        let s = pass(&mut w, 0);
        assert!((s.activation - strength(30.)).abs() < 1e-5);
        assert_eq!(s.player, 1);
        // Two needed: the strength of the second nearest, the player still the nearest.
        state(&mut w, 0).bullets_required = 2;
        let s = pass(&mut w, 0);
        assert!((s.activation - strength(60.)).abs() < 1e-5);
        assert_eq!((s.ternary, s.player), (1, 1));
        state(&mut w, 0).bullets_required = 3;
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        // RequireAll: every eligible player in range, whatever the count says.
        let s = state(&mut w, 0);
        (s.require_all, s.bullets_required) = (true, 1);
        assert!((pass(&mut w, 0).activation - strength(60.)).abs() < 1e-5);
        w.move_player(a, [300., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        state(&mut w, 0).require_all = false;
        assert_eq!(pass(&mut w, 0).player, 1);
        // PlayerMode 0: the leader only (player 0, who is out of range).
        state(&mut w, 0).player_mode = 0;
        assert_eq!(pass(&mut w, 0), SIGNAL_OFF);
        w.move_player(a, [10., 0., 0.], (0., 0.));
        w.move_player(b, [1., 0., 0.], (0., 0.));
        let s = pass(&mut w, 0);
        assert!((s.activation - 1.).abs() < 1e-5 && s.player == 0);
        // A negative radius: any eligible player, full strength.
        state(&mut w, 0).radius = -1.;
        w.move_player(a, [9000., 0., 0.], (0., 0.));
        assert_eq!(pass(&mut w, 0), sig(1., 1, 0));
    }
}
