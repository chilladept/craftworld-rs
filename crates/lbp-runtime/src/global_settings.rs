//! D267 (docs/level-scripts.md, part 18): the global settings in force, for a display to
//! read, and the counts of the per-tick `ApplyGlobalSettings__` call
//! ([`crate::LevelScripts::apply_global_settings`]).
use crate::{WorldState, GLOBAL_SETTINGS};

/// Script-side names of the eight `Game.EGlobalSettingsType` values, by slot. FOUND
/// (01.33): the order of the PWorld's blend factors (PWorld+0x5b8 + 4 * type) and of the
/// setters that write them.
pub const GLOBAL_SETTING_NAMES: [&str; GLOBAL_SETTINGS] = [
    "water_level",
    "gravity",
    "water_wave_magnitude",
    "water_tint",
    "water_murkiness",
    "lighting",
    "water_bits",
    "audio",
];

/// The value of every global setting the world keeps, as it is in force now: start +
/// (target - start) * t for the blended ones ([`crate::Blend::value`],
/// [`WorldState::lighting`]), the stored value for the water tint. The audio settings
/// (type 7) are not kept, so they are not here.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SettingsInForce {
    /// The blended level as the PWorld holds it. (`Game.GetGlobalSettingsWaterLevel__`
    /// gives 0.0 for a level not above [`crate::WATER_LEVEL_MIN`]; this does not.)
    pub water_level: f32,
    pub gravity: f32,
    pub water_wave_magnitude: f32,
    pub water_tint: f32,
    pub water_murkiness: f32,
    /// The lighting group in stored order: lighting, colour correction, fog, fog tint,
    /// darkness.
    pub lighting: [f32; 5],
    pub water_bits: f32,
}

impl SettingsInForce {
    /// The values of one type (`Game.EGlobalSettingsType`); none for audio or a number
    /// outside the eight.
    pub fn of_type(&self, kind: usize) -> &[f32] {
        match kind {
            0 => std::slice::from_ref(&self.water_level),
            1 => std::slice::from_ref(&self.gravity),
            2 => std::slice::from_ref(&self.water_wave_magnitude),
            3 => std::slice::from_ref(&self.water_tint),
            4 => std::slice::from_ref(&self.water_murkiness),
            5 => &self.lighting,
            6 => std::slice::from_ref(&self.water_bits),
            _ => &[],
        }
    }

    /// Per type, whether a value is not the same float as in `other` (compared by bits,
    /// so a NaN equals itself).
    pub fn unlike(&self, other: &Self) -> [bool; GLOBAL_SETTINGS] {
        std::array::from_fn(|kind| {
            let (a, b) = (self.of_type(kind), other.of_type(kind));
            a.iter().zip(b).any(|(a, b)| a.to_bits() != b.to_bits())
        })
    }

    /// The two values of the lighting group a display can use today: (lighting,
    /// darkness), the arguments of `LevelSettings::at_lighting` in `lbp-level` (P3m: the
    /// stored presets spread over lighting 0..1, darkness dimming exposure and fog
    /// colour). The other three (colour correction, fog, fog tint) have no settled
    /// mapping: see the part 18 "Not modelled" list.
    pub fn light(&self) -> [f32; 2] {
        [self.lighting[0], self.lighting[4]]
    }

    /// D332: (lighting, darkness, fog tint), the values the level's light is built from
    /// once the fog tint is applied (`LevelSettings::tinted_fog`).
    pub fn light_and_fog_tint(&self) -> [f32; 3] {
        [self.lighting[0], self.lighting[4], self.lighting[3]]
    }

    /// D333: (lighting, darkness, fog tint, fog), every value of the lighting group the
    /// level's light is built from but the colour correction (`LevelSettings::with_fog`).
    pub fn light_and_fog(&self) -> [f32; 4] {
        [
            self.lighting[0],
            self.lighting[4],
            self.lighting[3],
            self.lighting[2],
        ]
    }
}

impl WorldState {
    /// Every kept global setting as it is in force now.
    pub fn settings_in_force(&self) -> SettingsInForce {
        SettingsInForce {
            water_level: self.water.value(),
            gravity: self.gravity.value(),
            water_wave_magnitude: self.water_wave_magnitude.value(),
            water_tint: self.water_tint,
            water_murkiness: self.water_murkiness.value(),
            lighting: self.lighting(),
            water_bits: self.water_bits.value(),
        }
    }
}

/// What one or more [`crate::LevelScripts::apply_global_settings`] passes did. Unit: one
/// global-settings slot that held a Thing UID at its turn.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlobalSettingsReport {
    /// `ApplyGlobalSettings__` calls made, by slot (the setting type).
    pub calls: [usize; GLOBAL_SETTINGS],
    /// Slots whose Thing has a script without the function, or a script class that is
    /// not loaded. No call; the slot stays.
    pub no_function: usize,
    /// Slots whose Thing was stopped by a fault earlier. No call; the slot stays.
    pub to_stopped: usize,
    /// Slots cleared because no Thing has the UID or the Thing has no script.
    pub slots_cleared: usize,
}

impl GlobalSettingsReport {
    /// Adds `other`'s counts to this report.
    pub fn absorb(&mut self, other: GlobalSettingsReport) {
        for (mine, theirs) in self.calls.iter_mut().zip(other.calls) {
            *mine += theirs;
        }
        self.no_function += other.no_function;
        self.to_stopped += other.to_stopped;
        self.slots_cleared += other.slots_cleared;
    }

    /// All calls made.
    pub fn total_calls(&self) -> usize {
        self.calls.iter().sum()
    }
}

/// A change of lighting or darkness below this is not worth a new light: a display that
/// relights every material can skip it. A 2 second blend over the whole range is about
/// 60 steps this way, one per tick. PLACEHOLDER for the viewer, not a game value.
pub const RELIGHT_STEP: f32 = 0.002;

/// Follows the (lighting, darkness) pair in force (or, D332, any `N` values such as
/// [`SettingsInForce::light_and_fog_tint`]) for a display that rebuilds its light only
/// when they have moved enough ([`RELIGHT_STEP`]), and once more when they have come to
/// rest, so the last shown values are the ones in force.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightFollower<const N: usize = 2> {
    /// The pair the display shows.
    pub shown: [f32; N],
    /// The pair at the last [`Self::update`].
    last: [f32; N],
    /// Times [`Self::update`] asked for a new light.
    pub relights: u64,
}

impl<const N: usize> LightFollower<N> {
    /// A display that shows `shown` now (the pair its light was built from at load).
    pub fn new(shown: [f32; N]) -> Self {
        LightFollower {
            shown,
            last: shown,
            relights: 0,
        }
    }

    /// Call once per tick with the pair in force ([`SettingsInForce::light`]). Gives the
    /// pair to build a new light from, or nothing when the shown one will do. A pair with
    /// a value that is not finite is never shown.
    pub fn update(&mut self, now: [f32; N]) -> Option<[f32; N]> {
        if now.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let far = |a: [f32; N], b: [f32; N]| (0..N).any(|i| (a[i] - b[i]).abs() > RELIGHT_STEP);
        let at_rest = now == self.last && now != self.shown;
        self.last = now;
        if !(far(now, self.shown) || at_rest) {
            return None;
        }
        self.shown = now;
        self.relights += 1;
        Some(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Blend;

    #[test]
    fn settings_in_force_blend_each_type() {
        let mut p = WorldState {
            water: Blend::at(2.),
            gravity: Blend {
                from: 1.,
                to: 3.,
                t: 0.25,
            },
            water_tint: 4.,
            lighting_from: [0., 1., 2., 3., 4.],
            lighting_to: [1., 1., 4., 3., 0.],
            lighting_blend: 0.5,
            ..Default::default()
        };
        let before = p.settings_in_force();
        assert_eq!(before.water_level, 2.);
        assert_eq!(before.gravity, 1.5);
        assert_eq!(before.water_tint, 4.);
        assert_eq!(before.lighting, [0.5, 1., 3., 3., 2.]);
        assert_eq!(before.light(), [0.5, 2.]);
        assert_eq!(before.unlike(&before), [false; GLOBAL_SETTINGS]);
        // One value of the lighting group and the gravity move; audio is never unlike.
        p.lighting_blend = 1.;
        p.gravity.t = 1.;
        p.audio_blend = 0.3;
        let after = p.settings_in_force();
        let unlike = after.unlike(&before);
        assert_eq!(
            unlike,
            [false, true, false, false, false, true, false, false]
        );
        assert_eq!(after.of_type(5), &[1., 1., 4., 3., 0.]);
        assert!(after.of_type(7).is_empty() && after.of_type(8).is_empty());
        assert_eq!(GLOBAL_SETTING_NAMES[5], "lighting");
    }

    #[test]
    fn light_follower_steps_and_comes_to_rest() {
        let mut f = LightFollower::new([0.4, 0.]);
        // Nothing moved.
        assert_eq!(f.update([0.4, 0.]), None);
        // Below the step: not shown yet.
        assert_eq!(f.update([0.401, 0.]), None);
        // The same small change on the next tick: at rest, so it is shown.
        assert_eq!(f.update([0.401, 0.]), Some([0.401, 0.]));
        assert_eq!(f.update([0.401, 0.]), None);
        // A blend: each step past the threshold is shown, also when only darkness moves.
        assert_eq!(f.update([0.5, 0.]), Some([0.5, 0.]));
        assert_eq!(f.update([0.5, 0.1]), Some([0.5, 0.1]));
        // The blend ends a hair further on and stays there.
        assert_eq!(f.update([0.5, 0.1005]), None);
        assert_eq!(f.update([0.5, 0.1005]), Some([0.5, 0.1005]));
        assert_eq!(f.shown, [0.5, 0.1005]);
        assert_eq!(f.relights, 4);
        // A value that is not finite is never shown.
        assert_eq!(f.update([f32::NAN, 0.]), None);
        assert_eq!(f.update([f32::NAN, 0.]), None);
        assert_eq!(f.shown, [0.5, 0.1005]);
    }

    #[test]
    fn reports_add_up() {
        let mut a = GlobalSettingsReport::default();
        a.calls[5] = 2;
        a.slots_cleared = 1;
        let mut b = GlobalSettingsReport::default();
        b.calls[5] = 1;
        b.calls[1] = 4;
        b.no_function = 3;
        b.to_stopped = 2;
        a.absorb(b);
        assert_eq!(a.calls, [0, 4, 0, 0, 0, 3, 0, 0]);
        assert_eq!((a.no_function, a.to_stopped, a.slots_cleared), (3, 2, 1));
        assert_eq!(a.total_calls(), 7);
    }
}
