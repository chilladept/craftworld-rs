//! Sackboy's animation layers, frames and facing for the controller state of one 30 Hz tick
//! (D232, docs/d231-sackboy-animations.md). Bevy-free.
//!
//! Evidence: the clip GUIDs, frame counts and the 30 fps rate are FOUND (MAP paths under
//! `gamedata/mesh_library/player_character/sack_boy_pinched_anims/` and the d116 census).
//! The selection rules are FOUND in the 01.33 EBOOT (Ghidra pass,
//! `local-analysis/results/2026-10-11-ghidra-sackboy-anim-rules.md`, function addresses on
//! each constant): continuous walk weights over stand with one shared 13-tick phase, facing
//! eased 0.4 per tick, the jump frame driven by vy, the land as the jump clip's tail, 5-tick
//! stop/turn ramps, 3-tick lane ramps, the idle LCG chains and the blink overlay. A second
//! pass (`local-analysis/results/2026-10-11-ghidra-sackboy-anim-open-items.md`) settled the
//! idle roll range, the layer order, "idle allowed", the seed, the first blink, the landing
//! exponent and the walk_z sign. What neither pass resolved is marked INFERRED or OPEN where
//! it is used.
//!
//! The output is a stack of [`Layer`]s applied in order, as the game does onto its final
//! layer: the first sets the pose, each [`Blend::Lerp`] layer then lerps the pose toward its
//! clip by its weight, and each [`Blend::Additive`] layer adds its frame minus a reference
//! frame. One clip frame per tick (30 fps clips, 30 Hz ticks).
//!
//! D233 (docs/d233-sackboy-moods-headlook.md, Ghidra pass 3
//! `local-analysis/results/2026-10-11-ghidra-sackboy-moods-headlook.md`) adds the four moods
//! (d-pad press edges, levels 1-3, anticipation -> emote -> to_neutral, mood stands) and the
//! three additive head-look layers.
use crate::sackboy::{Sackboy, WALK_SPEED};
use std::f32::consts::FRAC_PI_2;

pub use crate::sackboy::Mood;

/// The clips the picker uses (EBOOT anim-object fields in the doc of each variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipId {
    /// 0x280 (and its second copy 0x284, "one more stand cycle").
    Stand,
    /// 0x2f4 / 0x2f8 / 0x2fc.
    WalkSlow,
    WalkMedium,
    WalkFast,
    /// 0x300 / 0x304.
    WalkFastStop,
    WalkFastTurn,
    /// 0x308: lane change with dz > 0 (FOUND, 0x21685c..0x216b74; here dz = lane - z, a
    /// step toward the camera in our z. That the game's +z also points at the camera is
    /// INFERRED from its camera eye at +1218 z).
    WalkZForwards,
    /// 0x30c: dz < 0.
    WalkZBackwards,
    /// 0x310 / 0x314: the held-jump clips.
    JumpStand,
    JumpWalk,
    /// 0x318 / 0x31c: the tap-jump clips, mixed with the held ones by [`JUMP_HOLD_FULL`]
    /// (FOUND, 0x209fc8; D245).
    JumpSmallStand,
    JumpSmallWalk,
    /// 0x36c: an additive spring pose in the game, driven on the ground path by shape
    /// overlap depth `max over contacts of min(depth / 350, 1)` (FOUND, 0x6c56c). Not
    /// driven here: we model no body overlap.
    StretchCrouch,
    /// 0x408: additive eyelid overlay.
    Blink,
    /// 0x2a4 / 0x2a8 / 0x2ac.
    IdleYawn,
    IdleShiftFeet,
    IdleFidget,
    /// 0x2dc / 0x2e0 / 0x2e4.
    StandIIdleBored,
    IdleBored,
    IdleBoredIStand,
    /// 0x2e8 / 0x2ec / 0x2f0.
    StandIIdleSit,
    IdleSit,
    IdleSitIStand,
    /// 0x418: played before every emote (D233).
    EmoteAnticipation,
    /// Emote clips 0x8c4[i] + 2 * level - 2 and their to_neutral clips
    /// 0x8d4[i] - 2 + 2 * level (FOUND, 0x203cdc / 0x1f7844).
    EmoteHappy,
    EmoteHappy2,
    EmoteHappy3,
    EmoteHappyToNeutral,
    EmoteHappy2ToNeutral,
    EmoteHappy3ToNeutral,
    /// Sad levels 2 and 3 loop: `_2_loop` repeats, `_3_into` leads into `_3_loop`.
    EmoteSad,
    EmoteSad2Loop,
    EmoteSad3Into,
    EmoteSad3Loop,
    EmoteSadToNeutral,
    EmoteSad2ToNeutral,
    EmoteSad3ToNeutral,
    EmoteAngry,
    EmoteAngry2,
    EmoteAngry3,
    EmoteAngryToNeutral,
    EmoteAngry2ToNeutral,
    EmoteAngry3ToNeutral,
    EmoteScared,
    EmoteScared2,
    EmoteScared3,
    EmoteScaredToNeutral,
    EmoteScared2ToNeutral,
    EmoteScared3ToNeutral,
    /// 0x294 + i: the mood stands.
    StandHappy,
    StandSad,
    StandAngry,
    StandScared,
    /// 0x340 / 0x33c / 0x344: additive head-look axes.
    LookUpdown,
    LookLeftright,
    LookRoll,
}

impl ClipId {
    /// Stand first: the viewer's clip bank binds the rest to stand's rig.
    pub const ALL: [ClipId; 56] = [
        ClipId::Stand,
        ClipId::WalkSlow,
        ClipId::WalkMedium,
        ClipId::WalkFast,
        ClipId::WalkFastStop,
        ClipId::WalkFastTurn,
        ClipId::WalkZForwards,
        ClipId::WalkZBackwards,
        ClipId::JumpStand,
        ClipId::JumpWalk,
        ClipId::JumpSmallStand,
        ClipId::JumpSmallWalk,
        ClipId::StretchCrouch,
        ClipId::Blink,
        ClipId::IdleYawn,
        ClipId::IdleShiftFeet,
        ClipId::IdleFidget,
        ClipId::StandIIdleBored,
        ClipId::IdleBored,
        ClipId::IdleBoredIStand,
        ClipId::StandIIdleSit,
        ClipId::IdleSit,
        ClipId::IdleSitIStand,
        ClipId::EmoteAnticipation,
        ClipId::EmoteHappy,
        ClipId::EmoteHappy2,
        ClipId::EmoteHappy3,
        ClipId::EmoteHappyToNeutral,
        ClipId::EmoteHappy2ToNeutral,
        ClipId::EmoteHappy3ToNeutral,
        ClipId::EmoteSad,
        ClipId::EmoteSad2Loop,
        ClipId::EmoteSad3Into,
        ClipId::EmoteSad3Loop,
        ClipId::EmoteSadToNeutral,
        ClipId::EmoteSad2ToNeutral,
        ClipId::EmoteSad3ToNeutral,
        ClipId::EmoteAngry,
        ClipId::EmoteAngry2,
        ClipId::EmoteAngry3,
        ClipId::EmoteAngryToNeutral,
        ClipId::EmoteAngry2ToNeutral,
        ClipId::EmoteAngry3ToNeutral,
        ClipId::EmoteScared,
        ClipId::EmoteScared2,
        ClipId::EmoteScared3,
        ClipId::EmoteScaredToNeutral,
        ClipId::EmoteScared2ToNeutral,
        ClipId::EmoteScared3ToNeutral,
        ClipId::StandHappy,
        ClipId::StandSad,
        ClipId::StandAngry,
        ClipId::StandScared,
        ClipId::LookUpdown,
        ClipId::LookLeftright,
        ClipId::LookRoll,
    ];

    /// Index into [`ClipId::ALL`] (for clip banks).
    pub fn index(self) -> usize {
        self as usize
    }

    /// The clip's animation GUID (FOUND, MAP + census).
    pub fn guid(self) -> u32 {
        match self {
            ClipId::Stand => 0x1241,
            ClipId::WalkSlow => 0x1242,
            ClipId::WalkMedium => 0x1243,
            ClipId::WalkFast => 0x1244,
            ClipId::WalkFastStop => 0x1247,
            ClipId::WalkFastTurn => 0x1248,
            ClipId::WalkZForwards => 0x2fa7,
            ClipId::WalkZBackwards => 0x2fa5,
            ClipId::JumpStand => 0x1249,
            ClipId::JumpWalk => 0x124a,
            ClipId::JumpSmallStand => 0x2fa2,
            ClipId::JumpSmallWalk => 0x2fa3,
            ClipId::StretchCrouch => 0x125a,
            ClipId::Blink => 0x22ba,
            ClipId::IdleYawn => 0x3110,
            ClipId::IdleShiftFeet => 0x316b,
            ClipId::IdleFidget => 0x317f,
            ClipId::StandIIdleBored => 0x310f,
            ClipId::IdleBored => 0x310e,
            ClipId::IdleBoredIStand => 0x3111,
            ClipId::StandIIdleSit => 0x2ff5,
            ClipId::IdleSit => 0x301d,
            ClipId::IdleSitIStand => 0x301c,
            ClipId::EmoteAnticipation => 0x25bf,
            ClipId::EmoteHappy => 0x25c0,
            ClipId::EmoteHappy2 => 0x43a5,
            ClipId::EmoteHappy3 => 0x43a6,
            ClipId::EmoteHappyToNeutral => 0x25c1,
            ClipId::EmoteHappy2ToNeutral => 0x42c7,
            ClipId::EmoteHappy3ToNeutral => 0x42c8,
            ClipId::EmoteSad => 0x25c2,
            ClipId::EmoteSad2Loop => 0x43a7,
            ClipId::EmoteSad3Into => 0x43a8,
            ClipId::EmoteSad3Loop => 0x43a9,
            ClipId::EmoteSadToNeutral => 0x25c3,
            ClipId::EmoteSad2ToNeutral => 0x42c9,
            ClipId::EmoteSad3ToNeutral => 0x42cb,
            ClipId::EmoteAngry => 0x25bd,
            ClipId::EmoteAngry2 => 0x42c2,
            ClipId::EmoteAngry3 => 0x42c3,
            ClipId::EmoteAngryToNeutral => 0x25be,
            ClipId::EmoteAngry2ToNeutral => 0x42c6,
            ClipId::EmoteAngry3ToNeutral => 0x42c5,
            ClipId::EmoteScared => 0x25c4,
            ClipId::EmoteScared2 => 0x43aa,
            ClipId::EmoteScared3 => 0x43ab,
            ClipId::EmoteScaredToNeutral => 0x25c5,
            ClipId::EmoteScared2ToNeutral => 0x42cd,
            ClipId::EmoteScared3ToNeutral => 0x42ce,
            ClipId::StandHappy => 0x128a,
            ClipId::StandSad => 0x128b,
            ClipId::StandAngry => 0x1289,
            ClipId::StandScared => 0x128c,
            ClipId::LookUpdown => 0x124c,
            ClipId::LookLeftright => 0x124b,
            ClipId::LookRoll => 0x124d,
        }
    }

    /// Frame count from the d116 census (FOUND).
    pub fn frames(self) -> u16 {
        match self {
            ClipId::Stand => 19,
            ClipId::WalkFastStop => 21,
            ClipId::WalkFastTurn => 19,
            ClipId::JumpStand
            | ClipId::JumpWalk
            | ClipId::JumpSmallStand
            | ClipId::JumpSmallWalk => 31,
            ClipId::StretchCrouch => 9,
            ClipId::IdleYawn => 76,
            ClipId::IdleShiftFeet => 26,
            ClipId::IdleFidget => 81,
            ClipId::StandIIdleBored => 33,
            ClipId::IdleBored => 25,
            ClipId::IdleBoredIStand => 28,
            ClipId::StandIIdleSit => 22,
            ClipId::IdleSit => 17,
            ClipId::IdleSitIStand => 21,
            ClipId::EmoteAnticipation => 6,
            ClipId::EmoteSad | ClipId::EmoteSad3Into => 14,
            ClipId::EmoteHappy
            | ClipId::EmoteAngry2
            | ClipId::EmoteHappyToNeutral
            | ClipId::EmoteHappy2ToNeutral
            | ClipId::EmoteHappy3ToNeutral
            | ClipId::EmoteSadToNeutral
            | ClipId::EmoteSad2ToNeutral
            | ClipId::EmoteSad3ToNeutral
            | ClipId::EmoteAngryToNeutral
            | ClipId::EmoteAngry2ToNeutral
            | ClipId::EmoteAngry3ToNeutral
            | ClipId::EmoteScaredToNeutral
            | ClipId::EmoteScared2ToNeutral
            | ClipId::EmoteScared3ToNeutral => 16,
            ClipId::StandHappy
            | ClipId::StandSad
            | ClipId::StandAngry
            | ClipId::StandScared
            | ClipId::LookRoll => 17,
            ClipId::EmoteHappy2 | ClipId::EmoteScared | ClipId::EmoteScared2 => 18,
            ClipId::EmoteSad2Loop | ClipId::EmoteAngry => 19,
            ClipId::EmoteAngry3 => 23,
            ClipId::EmoteScared3 => 26,
            ClipId::EmoteHappy3 => 31,
            ClipId::EmoteSad3Loop => 51,
            ClipId::LookUpdown | ClipId::LookLeftright => 9,
            _ => 13,
        }
    }

    /// The MAP name, for logs.
    pub fn name(self) -> &'static str {
        match self {
            ClipId::Stand => "stand",
            ClipId::WalkSlow => "walk_slow",
            ClipId::WalkMedium => "walk_medium",
            ClipId::WalkFast => "walk_fast",
            ClipId::WalkFastStop => "walk_fast_stop",
            ClipId::WalkFastTurn => "walk_fast_turn",
            ClipId::WalkZForwards => "walk_z_forwards",
            ClipId::WalkZBackwards => "walk_z_backwards",
            ClipId::JumpStand => "jump_stand",
            ClipId::JumpWalk => "jump_walk",
            ClipId::JumpSmallStand => "jump_small_stand",
            ClipId::JumpSmallWalk => "jump_small_walk",
            ClipId::StretchCrouch => "stretch_crouch",
            ClipId::Blink => "blink",
            ClipId::IdleYawn => "idle_yawn",
            ClipId::IdleShiftFeet => "idle_shift_feet",
            ClipId::IdleFidget => "idle_fidget",
            ClipId::StandIIdleBored => "stand_i_idle_bored",
            ClipId::IdleBored => "idle_bored",
            ClipId::IdleBoredIStand => "idle_bored_i_stand",
            ClipId::StandIIdleSit => "stand_i_idle_sit",
            ClipId::IdleSit => "idle_sit",
            ClipId::IdleSitIStand => "idle_sit_i_stand",
            ClipId::EmoteAnticipation => "emote_anticipation",
            ClipId::EmoteHappy => "emote_happy",
            ClipId::EmoteHappy2 => "emote_happy_2",
            ClipId::EmoteHappy3 => "emote_happy_3",
            ClipId::EmoteHappyToNeutral => "emote_happy_to_neutral",
            ClipId::EmoteHappy2ToNeutral => "emote_happy_2_to_neutral",
            ClipId::EmoteHappy3ToNeutral => "emote_happy_3_to_neutral",
            ClipId::EmoteSad => "emote_sad",
            ClipId::EmoteSad2Loop => "emote_sad_2_loop",
            ClipId::EmoteSad3Into => "emote_sad_3_into",
            ClipId::EmoteSad3Loop => "emote_sad_3_loop",
            ClipId::EmoteSadToNeutral => "emote_sad_to_neutral",
            ClipId::EmoteSad2ToNeutral => "emote_sad_2_to_neutral",
            ClipId::EmoteSad3ToNeutral => "emote_sad_3_to_neutral",
            ClipId::EmoteAngry => "emote_angry",
            ClipId::EmoteAngry2 => "emote_angry_2",
            ClipId::EmoteAngry3 => "emote_angry_3",
            ClipId::EmoteAngryToNeutral => "emote_angry_to_neutral",
            ClipId::EmoteAngry2ToNeutral => "emote_angry_2_to_neutral",
            ClipId::EmoteAngry3ToNeutral => "emote_angry_3_to_neutral",
            ClipId::EmoteScared => "emote_scared",
            ClipId::EmoteScared2 => "emote_scared_2",
            ClipId::EmoteScared3 => "emote_scared_3",
            ClipId::EmoteScaredToNeutral => "emote_scared_to_neutral",
            ClipId::EmoteScared2ToNeutral => "emote_scared_2_to_neutral",
            ClipId::EmoteScared3ToNeutral => "emote_scared_3_to_neutral",
            ClipId::StandHappy => "stand_happy",
            ClipId::StandSad => "stand_sad",
            ClipId::StandAngry => "stand_angry",
            ClipId::StandScared => "stand_scared",
            ClipId::LookUpdown => "look_updown",
            ClipId::LookLeftright => "look_leftright",
            ClipId::LookRoll => "look_roll",
        }
    }
}

/// The emote clip of a mood level 1..=3 (FOUND layout `0x8c4[i] + 2 * level - 2`; the
/// names per slot from the MAP).
pub fn emote_clip(mood: Mood, level: u8) -> ClipId {
    match (mood, level) {
        (Mood::Happy, 1) => ClipId::EmoteHappy,
        (Mood::Happy, 2) => ClipId::EmoteHappy2,
        (Mood::Happy, _) => ClipId::EmoteHappy3,
        (Mood::Sad, 1) => ClipId::EmoteSad,
        (Mood::Sad, 2) => ClipId::EmoteSad2Loop,
        (Mood::Sad, _) => ClipId::EmoteSad3Into,
        (Mood::Angry, 1) => ClipId::EmoteAngry,
        (Mood::Angry, 2) => ClipId::EmoteAngry2,
        (Mood::Angry, _) => ClipId::EmoteAngry3,
        (Mood::Scared, 1) => ClipId::EmoteScared,
        (Mood::Scared, 2) => ClipId::EmoteScared2,
        (Mood::Scared, _) => ClipId::EmoteScared3,
    }
}

/// The to_neutral clip for the level a mood leaves (`0x8d4[i] - 2 + 2 * level`).
pub fn to_neutral_clip(mood: Mood, level: u8) -> ClipId {
    match (mood, level) {
        (Mood::Happy, 1) => ClipId::EmoteHappyToNeutral,
        (Mood::Happy, 2) => ClipId::EmoteHappy2ToNeutral,
        (Mood::Happy, _) => ClipId::EmoteHappy3ToNeutral,
        (Mood::Sad, 1) => ClipId::EmoteSadToNeutral,
        (Mood::Sad, 2) => ClipId::EmoteSad2ToNeutral,
        (Mood::Sad, _) => ClipId::EmoteSad3ToNeutral,
        (Mood::Angry, 1) => ClipId::EmoteAngryToNeutral,
        (Mood::Angry, 2) => ClipId::EmoteAngry2ToNeutral,
        (Mood::Angry, _) => ClipId::EmoteAngry3ToNeutral,
        (Mood::Scared, 1) => ClipId::EmoteScaredToNeutral,
        (Mood::Scared, 2) => ClipId::EmoteScared2ToNeutral,
        (Mood::Scared, _) => ClipId::EmoteScared3ToNeutral,
    }
}

/// `stand_<mood>` by slot (clip field 0x294 + i).
pub const MOOD_STANDS: [ClipId; 4] = [
    ClipId::StandHappy,
    ClipId::StandSad,
    ClipId::StandAngry,
    ClipId::StandScared,
];
/// The head-look clips in argument order of 0x2007e4: up/down, left/right, roll.
pub const LOOK_CLIPS: [ClipId; 3] = [ClipId::LookUpdown, ClipId::LookLeftright, ClipId::LookRoll];

/// The loop a sad emote chains into after its first clip (FOUND: sad level >= 2 chains into
/// the 0x47c loop; which clip that is per level is INFERRED from the names: level 2 repeats
/// `_2_loop`, level 3 goes `_3_into` -> `_3_loop`).
fn sad_loop(mood: Mood, level: u8) -> Option<ClipId> {
    match (mood, level) {
        (Mood::Sad, 2) => Some(ClipId::EmoteSad2Loop),
        (Mood::Sad, 3) => Some(ClipId::EmoteSad3Loop),
        _ => None,
    }
}

/// Where the emote sequence is (the game's 0x890: 0, 1, 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmoteState {
    Anticipation,
    Emote,
    ToNeutral,
}

impl EmoteState {
    pub fn name(self) -> &'static str {
        match self {
            EmoteState::Anticipation => "anticipation",
            EmoteState::Emote => "emote",
            EmoteState::ToNeutral => "to_neutral",
        }
    }
}

/// The running emote sequence: `level` picks the clips (for to_neutral, the level left).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Emote {
    mood: Mood,
    level: u8,
    state: EmoteState,
    frame: u32,
    /// A sad emote that has chained into its loop clip.
    looping: bool,
}

impl Emote {
    fn clip(&self) -> ClipId {
        match self.state {
            EmoteState::Anticipation => ClipId::EmoteAnticipation,
            EmoteState::Emote => match sad_loop(self.mood, self.level) {
                Some(clip) if self.looping => clip,
                _ => emote_clip(self.mood, self.level),
            },
            EmoteState::ToNeutral => to_neutral_clip(self.mood, self.level),
        }
    }

    /// The emote clip has reached its last frame and holds it (the game's
    /// `0x764 >= len - 1`); the sad loops never do.
    fn finished(&self) -> bool {
        self.state == EmoteState::Emote
            && sad_loop(self.mood, self.level).is_none()
            && self.frame + 1 >= u32::from(self.clip().frames())
    }
}

/// The three head-look values (up/down, left/right, roll), each clamped to [-1, 1] (FOUND,
/// 0x200aa0 -> 0x2007e4). `look` = the blended look inputs (p, y, r); `tilt` = the relative
/// angle term f25; `w` the attention weight; `wf` the walk factor
/// 0.2 w_slow + 0.8 w_fast; `sgn` the sign of the anim speed; `yaw` the eased facing.
pub fn look_axes(look: [f32; 3], tilt: f32, w: f32, wf: f32, sgn: f32, yaw: f32) -> [f32; 3] {
    let [p, mut y, r] = look;
    // Yaw compensation: looking the way he already faces is reduced by yaw / (pi/2).
    if y > 0. && yaw > 0. {
        y = (y - yaw / FRAC_PI_2).clamp(0., 1.);
    } else if y < 0. && yaw < 0. {
        y = (y - yaw / FRAC_PI_2).clamp(-1., 0.);
    }
    let wf = wf.clamp(0., 1.);
    [
        0.5 * sgn * wf.sqrt() * tilt * w + p,
        y,
        // The 2 * [0x808] * cfg[0xa8] * ([0x810] - 0.5) term is OPEN and left out.
        tilt * w * (1. - wf).sqrt() + r,
    ]
    .map(|v| v.clamp(-1., 1.))
}

/// The frame of a look clip for value `v` and its reference (neutral) frame: additive of
/// frame `(0.5 v + 0.5) n` minus frame `0.5 n` (FOUND, 0x2007e4). n is taken as the last
/// frame index so that v = 1 is the last frame and the neutral the middle key (INFERRED).
pub fn look_frames(clip: ClipId, v: f32) -> (f32, f32) {
    let n = f32::from(clip.frames() - 1);
    ((0.5 * v.clamp(-1., 1.) + 0.5) * n, 0.5 * n)
}

/// How a layer combines with the pose built so far.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Blend {
    /// pose = lerp(pose, clip at `frame`, weight) (`0x1eae54`, FOUND).
    Lerp,
    /// pose += weight * (clip at `frame` - clip at `reference`) (`0x1eafc0`, FOUND; how the
    /// rotation difference is composed is INFERRED in the viewer).
    Additive { reference: f32 },
    /// pose = lerp(pose, clip at `frame`, weight) on the upper body only: `Bip01 Spine1` and
    /// every bone below it except `Bip01 Head` and the bones below the head (`0x1eaccc`, mask
    /// at skeleton+0x3a0 built in `0x1e79d8`, FOUND).
    UpperBody,
}

/// Weight of the stand clip over the bind pose: full (D246). D242-D244 lowered it to 0.25
/// because the idle stretched far too much, but the stretch was the viewer passing the
/// spine bones' breathing scale on to every bone below them. With that scale kept to the
/// scaled bone (`ScaleRule::Biped` in lbp-preview), the full-weight stand clip follows the
/// game's standing mesh height tick for tick (201.3..206.4 against our 200.8..206.1, RMS 0.4;
/// MEASURED, docs/d242-idle-stretch.md).
pub const STAND_BASE_WEIGHT: f32 = 1.;

/// One clip sampled at one frame with a weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    pub clip: ClipId,
    /// Clip frame; fractional for the vy-driven jump frame (sample between frames).
    pub frame: f32,
    pub weight: f32,
    /// The game's reverse/mirror flag (`0x1ebb48` mode 5): walk layers walking backwards,
    /// jump_walk and stop/turn when moving left. The viewer mirrors such a layer left/right
    /// through the skeleton's mirror table.
    pub mirror: bool,
    pub blend: Blend,
}

/// The pose recipe for one tick and the facing angle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimFrame {
    /// Applied in order; the first is the base (weight 1).
    pub layers: Vec<Layer>,
    /// Turn about the vertical axis, radians: 0 faces the camera, +pi/2 is profile walking
    /// right (+x), -pi/2 walking left. Already eased (FOUND 0.4 per tick).
    pub yaw: f32,
}

impl AnimFrame {
    /// Each lerp layer's share of the final pose (its weight times what later lerp layers
    /// leave); additive layers get 0. Sums to 1 over the lerp layers.
    pub fn effective_weights(&self) -> Vec<f32> {
        let mut out = vec![0.; self.layers.len()];
        let mut keep = 1.;
        for (i, l) in self.layers.iter().enumerate().rev() {
            if l.blend != Blend::Lerp {
                continue;
            }
            let w = if i == 0 { 1. } else { l.weight };
            out[i] = w * keep;
            keep *= 1. - w;
        }
        out
    }

    /// The lerp layer with the largest share (for logs and tests).
    pub fn dominant(&self) -> Option<Layer> {
        let w = self.effective_weights();
        (0..self.layers.len())
            .max_by(|&a, &b| w[a].total_cmp(&w[b]))
            .map(|i| self.layers[i])
    }
}

/// `WalkSpeedMaxDesired` = 30 x speedMod (FOUND at 0x201be0; speedMod 1 INFERRED).
pub const WALK_MAX_DESIRED: f32 = WALK_SPEED;
/// Walk weights from r = |vx| / [`WALK_MAX_DESIRED`] (FOUND, 0x201680): slow `r/0.1`,
/// medium `(r-0.1)/0.3`, fast `(r-0.4)/0.6`, each clamped to [0, 1], lerped in that order
/// over stand.
pub const WALK_RAMPS: [(f32, f32); 3] = [(0.0, 0.1), (0.1, 0.3), (0.4, 0.6)];
/// The shared walk phase: all three 13-frame walk clips sampled at `tick mod 13`, not
/// speed-scaled (FOUND, 0x20178c).
pub const WALK_PHASE_FRAMES: u32 = 13;
/// Ticks of opposite sign before the walk layers' backwards flag flips (FOUND 5-tick
/// hysteresis at 0x968/0x96c; the exact counting is INFERRED).
pub const BACKWARDS_HYSTERESIS: u32 = 5;
/// Facing eases this fraction of the way to its target each tick (FOUND, 0x201cc0).
pub const FACING_EASE: f32 = 0.4;
/// Facing during a lane change, radians (72 degrees, FOUND 1.2566 at 0x2002ac; the 0x1128
/// scale is taken as 1 and the sign as the side he already faces: INFERRED).
pub const LANE_YAW: f32 = 1.2566;
/// Stick |x| below which walk_fast_stop starts, above which walk_fast_turn may (FOUND
/// 0xc4ab20 / 0xc4ab00), and the w_fast both need (FOUND).
pub const STOP_STICK: f32 = 0.1;
pub const TURN_STICK: f32 = 0.9;
pub const SHOT_MIN_FAST: f32 = 0.5;
/// Stop/turn weight ramps in and out over this many frames (FOUND).
pub const SHOT_RAMP: f32 = 5.;
/// |lane - z| that starts a lane clip, the frames it skips and its ramp (FOUND, 0x216b0c
/// and 0x201418).
pub const LANE_TRIGGER: f32 = 100.;
pub const LANE_SKIP: u32 = 3;
pub const LANE_RAMP: f32 = 3.;
/// Jump layer (FOUND, 0x20a5f4): weight = clamp(peak/10), the peak |vy| follower eases at
/// 0.125 per tick. A jump from the ground is at full weight (FOUND: the take-off sets the
/// peak to [`JUMP_PEAK_FULL`]; D245). A ledge fall eases the follower up from 0 (D257,
/// MEASURED on three RPCS3 falls: full weight about 10 ticks after the edge; this replaced
/// the D234 ramp of 0.7 rising to 0.9).
pub const JUMP_PEAK_FULL: f32 = 10.;
pub const JUMP_PEAK_EASE: f32 = 0.125;
/// Stand-vs-walk jump mix per |vx| in the air (FOUND 0.045, full at 22.2).
pub const JUMP_MIX_PER_VX: f32 = 0.045;
/// Tap-vs-held jump mix (FOUND, 0x20a5f4 / 0x209fc8): a counter is 1 on the take-off tick
/// and goes up one per tick while the jump button stays held; the pose is
/// `lerp(jump_small_*, jump_*, clamp(count / 10))`. Releasing the button freezes it for the
/// rest of the flight and the landing tail.
pub const JUMP_HOLD_FULL: f32 = 10.;
/// Jump frame (FOUND): take-off ticks 1..5 show frame = tick; rising
/// `8 * (1 - clamp(vy / 73))`; falling `8 + 6 * clamp(-vy / 30.26)`.
pub const JUMP_TAKEOFF_TICKS: u32 = 5;
pub const JUMP_RISE_VY: f32 = 73.;
pub const JUMP_FALL_VY: f32 = 30.26;
pub const JUMP_APEX_FRAME: f32 = 8.;
pub const JUMP_FALL_FRAMES: f32 = 6.;
/// Landing (FOUND): the jump clip plays on from `int((airFrame + 15) * 0.5)` at one frame
/// per tick, weight times `clamp(1 - (f - 15) / (n - 15))^k` with k = 1 (FOUND: the fade is
/// Perlin's bias(t, 0.5) = t, 0x20a7fc..0x20a888).
pub const LAND_FADE_FROM: f32 = 15.;
pub const LAND_FADE_POW: f32 = 1.;
/// Per-tick ease of the in-air state s toward 1 while the jump layer runs (in the air and
/// through the landing tail) and toward 0 otherwise (FOUND, dispatcher 0x20e28c, 0xd99394 =
/// 0.9 kept). The arm pass (0x2097e0 -> 0x208dec) then pulls the upper body back toward
/// frame 0 of the stand clip by s (1 - s), s being the value before the tick's ease: nothing
/// on the ground, a quarter at most. D250: this is the "air lean": without it our Spine1
/// pitched up to 4.4 degrees further than the game's through a jump.
pub const AIR_EASE: f32 = 0.1;
/// Ticks between blinks: `30 + rand(70)` (FOUND, 0x2040a0).
/// Crouch spring (FOUND, 0x2011d4 and its input 0x6c56c): contact depth that gives input 1
/// (0xd937f8), the exponent of bias(x, 0.2) = x^(ln 0.2 / ln 0.5), the cut below which the
/// kick and the pose are dropped, and the two carry factors (0xd99764, 0xd99760).
pub const CROUCH_DEPTH_FULL: f32 = 350.;
pub const CROUCH_BIAS_POW: f32 = 2.321_928;
pub const CROUCH_MIN: f32 = 0.01;
pub const CROUCH_KEEP: f32 = 0.75;
pub const CROUCH_BACK: f32 = -0.25;

pub const BLINK_GAP: u32 = 30;
pub const BLINK_GAP_RAND: u32 = 70;
/// The LCG state the anim object's constructor stores (FOUND 0x2a at 0x1ff64c; shared by
/// the idle roll and the blink gap).
pub const LCG_SEED: u32 = 42;
/// The idle roll's range: `mulli 0xfa` gives 0..249 (FOUND, 0x20e0ac..0x20e288).
pub const IDLE_ROLL: u32 = 250;
/// The sit loop's stand-up roll: `mulli 0x3e8`, 0..999, stands up when > 950 (FOUND,
/// 0x20e044).
pub const SIT_ROLL: u32 = 1000;
pub const SIT_STAND_UP_ABOVE: u32 = 950;
/// "Idle allowed" (0x1fb4a8) needs the no-input counter 0x75c above this many ticks
/// (FOUND, 0x1fb51c..0x1fb59c): about 10 s.
pub const IDLE_AFTER_TICKS: u32 = 300;
/// Mood levels of plain Sackboy: the same button cycles 1, 2, 3, off (FOUND, 0x1f7844:
/// `(level + 1) mod 4`; the 0x1124 model variants have one level and are not modelled).
pub const MOOD_LEVELS: u8 = 3;
/// Mood layer weights 0x8a4[i] ease this much per tick toward 1 (level > 0) or 0 (FOUND
/// 0.1, 0x203cdc; taken as a fraction of the remaining distance like the facing: INFERRED).
pub const MOOD_EASE: f32 = 0.1;
/// A mood layer below this weight is not applied (FOUND 0.01, 0x2022b8).
pub const MOOD_MIN_WEIGHT: f32 = 0.01;
/// The head-look attention weight 0x884 eases this fraction per tick toward 1 - S (FOUND,
/// 0x200f28).
pub const LOOK_EASE: f32 = 0.1;

/// The game's idle LCG step (FOUND, 0x20dee0): `seed * 0x2e90edd + 0xe908fc1`, 30 bits.
pub fn lcg_next(seed: u32) -> u32 {
    seed.wrapping_mul(0x2e90edd).wrapping_add(0xe908fc1) & 0x3fff_ffff
}

/// `rand(n)` from a seed: `((seed >> 16) * n) >> 14`, 0..n-1 (FOUND).
pub fn lcg_rand(seed: u32, n: u32) -> u32 {
    ((seed >> 16) * n) >> 14
}

/// The idle the stand-cycle roll (0..249) picks (FOUND table, 0x20dee0): 0..2 sit, 3..4
/// yawn, 5..6 bored, 7..9 fidget, 10 shift_feet, otherwise one more stand cycle. Odds per
/// roll: 3, 2, 2, 3 and 1 in 250.
pub fn idle_for_roll(r: u32) -> ClipId {
    match r {
        0..=2 => ClipId::StandIIdleSit,
        3..=4 => ClipId::IdleYawn,
        5..=6 => ClipId::StandIIdleBored,
        7..=9 => ClipId::IdleFidget,
        10 => ClipId::IdleShiftFeet,
        _ => ClipId::Stand,
    }
}

/// Walk weights (slow, medium, fast) for |vx| (FOUND, [`WALK_RAMPS`]).
pub fn walk_weights(vx: f32) -> [f32; 3] {
    let r = (vx.abs() / WALK_MAX_DESIRED).clamp(0., 1.);
    WALK_RAMPS.map(|(from, span)| ((r - from) / span).clamp(0., 1.))
}

/// Facing target for the anim speed: `clamp(vx / WalkMax) * pi/2` (FOUND, 0x2002ac).
pub fn facing_target(vx: f32) -> f32 {
    (vx / WALK_MAX_DESIRED).clamp(-1., 1.) * FRAC_PI_2
}

/// The vy-driven jump frame after the take-off ticks (FOUND).
pub fn jump_frame(vy: f32) -> f32 {
    if vy > 0. {
        JUMP_APEX_FRAME * (1. - (vy / JUMP_RISE_VY).clamp(0., 1.))
    } else {
        JUMP_APEX_FRAME + JUMP_FALL_FRAMES * (-vy / JUMP_FALL_VY).clamp(0., 1.)
    }
}

/// Weight of frame `i` of an `n`-frame one-shot that ramps in and out over `k` frames.
fn ramp(i: u32, n: u16, k: f32) -> f32 {
    let i = i as f32;
    (i / k).min((f32::from(n) - i) / k).clamp(0., 1.)
}

/// A one-shot clip playing one frame per tick.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shot {
    clip: ClipId,
    i: u32,
    mirror: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Air {
    /// Airborne ticks so far (1 on the first).
    since: u32,
    takeoff: bool,
    blend_in: f32,
    frame: f32,
}

/// The `stretch_crouch` spring (FOUND, 0x2011d4): three values between ticks (creature
/// +0x850, +0x854, +0x858), each clamped to [-1, 1].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CrouchSpring {
    s850: f32,
    s854: f32,
    s858: f32,
}

impl CrouchSpring {
    /// One tick with the contact input `e` in [0, 1] (the game's 0x6c56c value: the deepest
    /// contact's depth over [`CROUCH_DEPTH_FULL`]). Returns the pose value v in [-1, 1]:
    /// positive squashes, negative stretches; exactly 0 while at rest with no input.
    ///
    /// FOUND: x = clamp(sqrt(e), 0.01, 0.99), h = bias(x, 0.2) = x^2.3219, dropped below
    /// 0.01; a = s858 + h, b = s854 + a, v = s850 + b (each clamped), s854 = b; after the
    /// pose s850 = 0.75 v and s858 = -0.25 v. A one-tick kick h gives v / h = 1, 1.5, 1.5,
    /// 1.125, 0.5625, 0, then a rebound to -0.63: a damped swing of period 12 ticks.
    pub fn step(&mut self, e: f32) -> f32 {
        let x = e.max(0.).sqrt().clamp(CROUCH_MIN, 0.99);
        let mut h = x.powf(CROUCH_BIAS_POW);
        if h < CROUCH_MIN {
            h = 0.;
        }
        let a = (self.s858 + h).clamp(-1., 1.);
        let b = (self.s854 + a).clamp(-1., 1.);
        let v = (self.s850 + b).clamp(-1., 1.);
        self.s854 = b;
        self.s850 = CROUCH_KEEP * v;
        self.s858 = CROUCH_BACK * v;
        v
    }
}

/// Per-Sackboy animation state between ticks.
#[derive(Debug, Clone, PartialEq)]
pub struct SackboyAnim {
    now: u64,
    walk_tick: u32,
    backwards: bool,
    flip_count: u32,
    yaw: f32,
    peak: f32,
    jump_mix: f32,
    jump_mirror: bool,
    /// The game's hold counter 0x790 and its "still held" flag 0x78a.
    jump_hold: u32,
    jump_holding: bool,
    air: Option<Air>,
    land: Option<f32>,
    /// The eased in-air state 0x6ec ([`AIR_EASE`]).
    air_ease: f32,
    crouch: CrouchSpring,
    /// The deepest contact's depth for the next tick's crouch spring (the caller's to set;
    /// 0 unless set, and cleared by each tick).
    contact_depth: f32,
    lane: Option<Shot>,
    lane_armed: bool,
    shot: Option<Shot>,
    idle: ClipId,
    idle_frame: u32,
    idle_loops: u32,
    /// Ticks with no input on the ground (the game's 0x75c).
    quiet_ticks: u32,
    seed: u32,
    blink: Option<u32>,
    next_blink: u64,
    /// Mood levels 0x894[i] and layer weights 0x8a4[i] (slot order of [`Mood::ALL`]).
    levels: [u8; 4],
    weights: [f32; 4],
    emote: Option<Emote>,
    /// The mood whose stand is lerped over the base (set when its emote has finished).
    mood_stand: Option<Mood>,
    /// The d-pad direction held last tick, for the press edge.
    mood_held: Option<Mood>,
    /// Head-look attention weight 0x884.
    attention: f32,
}

impl Default for SackboyAnim {
    fn default() -> Self {
        Self::new()
    }
}

impl SackboyAnim {
    /// Standing, facing the camera, seed 42, first blink on the first tick (FOUND: the ctor
    /// zeroes the blink start 0x750 and the tick counter 0x754, 0x1f9984/0x1f9988).
    pub fn new() -> Self {
        Self {
            now: 0,
            walk_tick: 0,
            backwards: false,
            flip_count: 0,
            yaw: 0.,
            peak: 0.,
            jump_mix: 0.,
            jump_mirror: false,
            jump_hold: 0,
            jump_holding: false,
            air: None,
            land: None,
            air_ease: 0.,
            crouch: CrouchSpring::default(),
            contact_depth: 0.,
            lane: None,
            lane_armed: true,
            shot: None,
            idle: ClipId::Stand,
            idle_frame: 0,
            idle_loops: 0,
            quiet_ticks: 0,
            seed: LCG_SEED,
            blink: None,
            next_blink: 0,
            levels: [0; 4],
            weights: [0.; 4],
            emote: None,
            mood_stand: None,
            mood_held: None,
            attention: 0.,
        }
    }

    /// Sets the deepest contact's depth (level units) that the next tick's crouch spring
    /// reads (FOUND input, 0x6c56c). Nothing in the sim reports a depth yet.
    pub fn set_contact_depth(&mut self, depth: f32) {
        self.contact_depth = depth.max(0.);
    }

    /// The eased facing angle (radians, see [`AnimFrame::yaw`]).
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// The active mood and its level 1..=3 (moods are exclusive).
    pub fn mood(&self) -> Option<(Mood, u8)> {
        Mood::ALL
            .into_iter()
            .zip(self.levels)
            .find(|&(_, level)| level > 0)
    }

    /// The mood layer weights, in [`Mood::ALL`] order.
    pub fn mood_weights(&self) -> [f32; 4] {
        self.weights
    }

    /// The emote sequence's mood, state and frame while one runs (a held last frame
    /// included).
    pub fn emote(&self) -> Option<(Mood, EmoteState, u32)> {
        self.emote.map(|e| (e.mood, e.state, e.frame))
    }

    /// True once the emote clip holds its last frame (the mood stand is then lerped on).
    pub fn emote_held(&self) -> bool {
        self.emote.is_some_and(|e| e.finished())
    }

    /// The head-look attention weight (1 = calm and standing, 0 = moving).
    pub fn attention(&self) -> f32 {
        self.attention
    }

    /// One press of a mood's d-pad direction (FOUND, 0x1ffe58..0x1ffe74 and 0x1f7844): the
    /// opposite mood, if on, is ended and nothing starts; otherwise the level steps
    /// 1, 2, 3, off, a level above 0 switching every other mood off. [`Self::tick`] calls
    /// this on the press edge of [`crate::sackboy::Input::mood`].
    pub fn press_mood(&mut self, mood: Mood) {
        let (i, o) = (mood.index(), mood.opposite().index());
        if self.levels[o] > 0 {
            self.end_emote(mood.opposite());
            return;
        }
        let level = (self.levels[i] + 1) % (MOOD_LEVELS + 1);
        if level == 0 {
            self.end_emote(mood);
            return;
        }
        self.levels = [0; 4];
        self.levels[i] = level;
        // 0x1f7844 stores state 1 and frame 0 here while the playback's state 0 is the
        // anticipation; pass 3 reads this as "the anticipation restarts", which is what
        // plays here on every step to a level above 0 (INFERRED numbering).
        self.emote = Some(Emote {
            mood,
            level,
            state: EmoteState::Anticipation,
            frame: 0,
            looping: false,
        });
        self.mood_stand = None;
    }

    /// 0x1f65b0: level off, the to_neutral clip of the level left from frame 0.
    fn end_emote(&mut self, mood: Mood) {
        let level = std::mem::take(&mut self.levels[mood.index()]);
        self.emote = Some(Emote {
            mood,
            level,
            state: EmoteState::ToNeutral,
            frame: 0,
            looping: false,
        });
    }

    /// The mood layers for this tick, over the base (FOUND playback 0x203cdc; the emote is
    /// the base the walk and jump blends go over). While a sequence runs its clip covers
    /// the base; `stand_<mood>` is lerped on by the mood's weight (0x201fb8) once the emote
    /// holds its last frame (0x2022b8's `0x764 >= len - 1` test, FOUND there under the
    /// 0x1124 flag; applying it to plain Sackboy, and keeping the stand while its weight
    /// fades under to_neutral, is INFERRED).
    fn mood_layers(&mut self, layers: &mut Vec<Layer>) {
        for (w, level) in self.weights.iter_mut().zip(self.levels) {
            let target = if level > 0 { 1. } else { 0. };
            *w += MOOD_EASE * (target - *w);
        }
        if let Some(mut e) = self.emote.take() {
            let clip = e.clip();
            let n = u32::from(clip.frames());
            layers.push(Layer {
                clip,
                frame: e.frame.min(n - 1) as f32,
                weight: 1.,
                mirror: false,
                blend: Blend::Lerp,
            });
            if e.finished() {
                self.mood_stand = Some(e.mood);
            }
            e.frame += 1;
            let mut done = false;
            if e.frame >= n {
                match e.state {
                    EmoteState::Anticipation => {
                        e.state = EmoteState::Emote;
                        e.frame = 0;
                    }
                    // No end handling: the last frame holds (FOUND); sad 2 and 3 loop.
                    EmoteState::Emote if sad_loop(e.mood, e.level).is_some() => {
                        e.looping = true;
                        e.frame = 0;
                    }
                    EmoteState::Emote => e.frame = n - 1,
                    // Then 0x890, 0x888 and 0x764 are reset (FOUND).
                    EmoteState::ToNeutral => done = true,
                }
            }
            self.emote = (!done).then_some(e);
        }
        if let Some(m) = self.mood_stand {
            let weight = self.weights[m.index()];
            if weight > MOOD_MIN_WEIGHT {
                let clip = MOOD_STANDS[m.index()];
                layers.push(Layer {
                    clip,
                    // Its own loop, phase from the tick counter (INFERRED).
                    frame: (self.now % u64::from(clip.frames())) as f32,
                    weight,
                    mirror: false,
                    blend: Blend::Lerp,
                });
            } else {
                self.mood_stand = None;
            }
        }
    }

    fn rand(&mut self, n: u32) -> u32 {
        self.seed = lcg_next(self.seed);
        lcg_rand(self.seed, n)
    }

    /// The layers and facing for this tick. Call once per tick after [`Sackboy::step`],
    /// with `grounded` the floor contact after that tick's move: the tick a fall is stopped
    /// by the floor is already grounded, so the landing tail starts on it (D247, MEASURED:
    /// the game's mesh is on tail frame 15 one tick after its feet reach the floor).
    pub fn tick(&mut self, s: &Sackboy, grounded: bool) -> AnimFrame {
        self.now += 1;
        let airborne = !grounded || s.jump_tick().is_some();
        let vx = s.vx;
        let input = s.prev_input();
        let stick = input.x;
        let [w_slow, w_med, w_fast] = walk_weights(vx);
        let mut layers = Vec::with_capacity(8);

        // Base: stand or the idle chain (hard switches at clip ends, FOUND).
        // Idle allowed (0x1fb4a8, FOUND): creature state in {0, 1, 7, 10} (taken as
        // grounded: INFERRED), no mood level above 0 and the no-input counter 0x75c > 300.
        // The counter (0x20d3a0) counts ticks with no pad input in those states and resets
        // otherwise; "no input" for the 0x7feff mask is taken as stick 0 and no jump/lane
        // button (INFERRED; the mood d-pad bits are outside the mask, FOUND).
        // Mood press edge (FOUND 0x388358: down now, not down last tick), before the
        // playback as in the creature tick. The gates on the loop (0x965, creature states,
        // sad skipped while the Popit global is set) are not modelled.
        if let Some(mood) = input.mood.filter(|&m| self.mood_held != Some(m)) {
            self.press_mood(mood);
        }
        self.mood_held = input.mood;
        let quiet = !airborne && stick == 0. && !input.jump && !input.back && !input.front;
        self.quiet_ticks = if quiet { self.quiet_ticks + 1 } else { 0 };
        let idle_ok = self.quiet_ticks > IDLE_AFTER_TICKS && self.mood().is_none();
        layers.push(Layer {
            clip: self.idle,
            frame: self.idle_frame as f32,
            weight: if self.idle == ClipId::Stand {
                STAND_BASE_WEIGHT
            } else {
                1.
            },
            mirror: false,
            blend: Blend::Lerp,
        });
        self.advance_idle(idle_ok);
        self.mood_layers(&mut layers);

        // Walk: continuous weights over the base, shared 13-tick phase (FOUND).
        if (vx < 0.) != self.backwards {
            self.flip_count += 1;
            if self.flip_count >= BACKWARDS_HYSTERESIS {
                self.backwards = !self.backwards;
                self.flip_count = 0;
            }
        } else {
            self.flip_count = 0;
        }
        let phase = (self.walk_tick % WALK_PHASE_FRAMES) as f32;
        self.walk_tick = self.walk_tick.wrapping_add(1);
        for (clip, weight) in [
            (ClipId::WalkSlow, w_slow),
            (ClipId::WalkMedium, w_med),
            (ClipId::WalkFast, w_fast),
        ] {
            if weight > 0. {
                layers.push(Layer {
                    clip,
                    frame: phase,
                    weight,
                    mirror: self.backwards,
                    blend: Blend::Lerp,
                });
            }
        }

        // Layer order after the walk (FOUND, dispatcher 0x20e28c): stop -> turn -> jump and
        // landing tail -> lane.

        // walk_fast_stop / walk_fast_turn (FOUND triggers; INFERRED: cut short in the air).
        if airborne {
            self.shot = None;
        } else if self.shot.is_none() && w_fast > SHOT_MIN_FAST {
            let clip = if stick.abs() < STOP_STICK {
                Some(ClipId::WalkFastStop)
            } else if stick.abs() > TURN_STICK && stick.signum() != vx.signum() {
                Some(ClipId::WalkFastTurn)
            } else {
                None
            };
            self.shot = clip.map(|clip| Shot {
                clip,
                i: 0,
                mirror: vx < 0.,
            });
        }
        if let Some(mut sh) = self.shot.take() {
            let n = sh.clip.frames();
            layers.push(Layer {
                clip: sh.clip,
                frame: sh.i as f32,
                weight: ramp(sh.i, n, SHOT_RAMP),
                mirror: sh.mirror,
                blend: Blend::Lerp,
            });
            sh.i += 1;
            if sh.i < u32::from(n) {
                self.shot = Some(sh);
            } else if sh.clip == ClipId::WalkFastTurn {
                // The turn's end resets the walk phase and the idle frame (FOUND). The tick
                // after the turn's last frame the game's mesh is on walk frame 1, not 0
                // (MEASURED, D251, three captures), so the phase is already one on.
                self.walk_tick = 1;
                self.idle_frame = 0;
            }
        }

        // Jump layer: vy-driven frame in the air, the clip's tail after landing (FOUND).
        let vy = s.vy.abs();
        // D234 put the layer at 0.7..0.9 in the air from RPCS3 vertex fits of a standing
        // jump. D245: those fits compared against jump_stand alone; the game mixes
        // jump_small_stand under it by the hold count (below), and with that mix the D243
        // capture fits a full-weight layer, which is what 0x20a5f4 does after a take-off.
        // D257: a ledge fall runs the 0x20a5f4 peak follower from 0 instead.
        // The follower holds while a landing tail plays and is back at 0 once it is over
        // (INFERRED: a second fall a few ticks after a landing starts near full weight).
        let tail = self.land.is_some();
        if !airborne && self.air.is_none() && !tail {
            self.peak = 0.;
        }
        // Hold counter (FOUND): stopped by the first tick without the button; cleared on
        // the ground once no landing tail is playing (the game's test is its tail index
        // being unset, which in the air it is not).
        if !input.jump {
            self.jump_holding = false;
            if !airborne && self.air.is_none() && self.land.is_none() {
                self.jump_hold = 0;
            }
        } else if s.jump_tick() == Some(1) {
            self.jump_hold = 1;
            self.jump_holding = true;
        } else if self.jump_holding {
            self.jump_hold += 1;
        }
        let mut jump = None;
        if airborne {
            let air = self.air.get_or_insert(Air {
                since: 0,
                takeoff: s.jump_tick() == Some(1),
                // MEASURED (D257): an off-ledge fall uses the same blend-in. INFERRED: none
                // when he leaves the ground again with the landing tail still playing.
                blend_in: if tail { 0. } else { 1. + 4. * vx.abs() / 20. },
                frame: 0.,
            });
            air.since += 1;
            air.frame = if air.takeoff && air.since <= JUMP_TAKEOFF_TICKS {
                air.since as f32
            } else if air.frame > JUMP_APEX_FRAME {
                // Past the apex the frame only moves on (INFERRED). Before D247 the
                // touchdown tick came here with vy 0 and `grounded` still false, which
                // mapped back to the apex frame 8 and started the landing tail at 11
                // instead of 14; that tick is grounded now, so this is only a guard for a
                // caller that reports contact late.
                jump_frame(s.vy).max(air.frame)
            } else {
                jump_frame(s.vy)
            };
            self.land = None;
            self.jump_mix = (vx * JUMP_MIX_PER_VX).abs().min(1.);
            self.jump_mirror = vx < 0.;
            if air.takeoff {
                self.peak = JUMP_PEAK_FULL;
            } else {
                self.peak += JUMP_PEAK_EASE * (vy - self.peak);
            }
            let level = (self.peak / JUMP_PEAK_FULL).clamp(0., 1.);
            let blend = if air.blend_in > 0. {
                (air.since as f32 / air.blend_in).clamp(0., 1.)
            } else {
                1.
            };
            let w = level * blend;
            jump = Some((air.frame, w));
        } else if let Some(air) = self.air.take() {
            self.land = Some(((air.frame + LAND_FADE_FROM) * 0.5).floor());
        }
        if let (false, Some(f)) = (airborne, self.land) {
            let n = f32::from(ClipId::JumpStand.frames());
            let fade = (1. - (f - LAND_FADE_FROM) / (n - LAND_FADE_FROM))
                .clamp(0., 1.)
                .powf(LAND_FADE_POW);
            jump = Some((f, (self.peak / JUMP_PEAK_FULL).clamp(0., 1.) * fade));
            self.land = (f + 1. < n).then_some(f + 1.);
        }
        if let Some((frame, w)) = jump.filter(|&(_, w)| w > 0.01) {
            // lerp(pose, lerp(stand pair, walk pair, m), w), each pair being
            // lerp(jump_small_*, jump_*, h), as four sequential lerps: a clip whose share of
            // the result is c gets the weight c / (1 - the shares applied after it).
            let m = self.jump_mix;
            let h = (self.jump_hold as f32 / JUMP_HOLD_FULL).clamp(0., 1.);
            let mut after = w;
            for (clip, share, mirror) in [
                (ClipId::JumpSmallStand, w * (1. - m) * (1. - h), false),
                (ClipId::JumpStand, w * (1. - m) * h, false),
                (ClipId::JumpSmallWalk, w * m * (1. - h), self.jump_mirror),
                (ClipId::JumpWalk, w * m * h, self.jump_mirror),
            ] {
                after -= share;
                let rest = 1. - after;
                let weight = if rest > 1e-6 { share / rest } else { 0. };
                if weight > 0. {
                    layers.push(Layer {
                        clip,
                        frame,
                        weight,
                        mirror,
                        blend: Blend::Lerp,
                    });
                }
            }
        }

        // Lane change, after the jump (FOUND order): one clip per slide (re-armed once the
        // slide is within the trigger; INFERRED), frame i + 3, 3-frame ramps (FOUND).
        let dz = s.lane - s.z;
        if dz.abs() < LANE_TRIGGER {
            self.lane_armed = true;
        } else if self.lane_armed && self.lane.is_none() && !airborne {
            self.lane_armed = false;
            self.lane = Some(Shot {
                clip: if dz > 0. {
                    ClipId::WalkZForwards
                } else {
                    ClipId::WalkZBackwards
                },
                i: 0,
                mirror: false,
            });
        }
        let lane_active = self.lane.is_some();
        if let Some(mut l) = self.lane.take() {
            let n = l.clip.frames();
            let end = u32::from(n) - LANE_SKIP;
            let weight = ((l.i + 1) as f32 / LANE_RAMP).clamp(0., 1.)
                * ((end - l.i) as f32 / LANE_RAMP).clamp(0., 1.);
            layers.push(Layer {
                clip: l.clip,
                frame: (l.i + LANE_SKIP) as f32,
                weight,
                mirror: false,
                blend: Blend::Lerp,
            });
            l.i += 1;
            if l.i < end {
                self.lane = Some(l);
            }
        }

        // Upper body back toward stand (FOUND, 0x208dec, after the jump and lane layers):
        // weight max(u, arm terms) (1 - u) with u = 1 - s; the arm terms (0xae0, 0xae4) are
        // not modelled (0), so s (1 - s). The target is frame 0 of the stand clip, or of the
        // mood stand while one is held (0x2022b8; that it is exactly our `mood_stand` is
        // INFERRED).
        let s0 = self.air_ease;
        self.air_ease += AIR_EASE * (if jump.is_some() { 1. } else { 0. } - s0);
        let upper = s0 * (1. - s0);
        if upper > 0.001 {
            layers.push(Layer {
                clip: self
                    .mood_stand
                    .map_or(ClipId::Stand, |m| MOOD_STANDS[m.index()]),
                frame: 0.,
                weight: upper,
                mirror: false,
                blend: Blend::UpperBody,
            });
        }

        // stretch_crouch: an additive spring (FOUND, 0x2011d4, run by the dispatcher after
        // jump, lane and grab) driven by the deepest contact's depth over 350 (0x6c56c; 0 in
        // the wall-cling state, which is not modelled). Applied as frame (0.5 v + 0.5) n
        // minus frame 0.5 n at weight 1 while |v| > 0.01 (FOUND); n as for the look clips
        // (INFERRED). Our contacts have no depth, so nothing feeds it and it stays at rest.
        // It is NOT the leg squash after a landing (D248, MEASURED on d234 `jump.bin`): fed
        // a landing depth on any of the first four ticks, the best fit still misses the
        // game's height and pelvis height by up to 3.9, shortens the spine by 3 to 4 where
        // the game's does not change, and predicts a rebound the capture does not show.
        let v = self
            .crouch
            .step((self.contact_depth / CROUCH_DEPTH_FULL).min(1.));
        self.contact_depth = 0.;
        if v.abs() > CROUCH_MIN {
            let (frame, reference) = look_frames(ClipId::StretchCrouch, v);
            layers.push(Layer {
                clip: ClipId::StretchCrouch,
                frame,
                weight: 1.,
                mirror: false,
                blend: Blend::Additive { reference },
            });
        }

        // Blink: additive frame i minus frame 0, weight 1, first on tick 0, then every
        // 30 + rand(70) ticks (FOUND). Its slot in the stack is INFERRED (last): 0x2040a0
        // is reached through a pointer, not from the dispatcher.
        if self.blink.is_none() && self.now >= self.next_blink {
            self.blink = Some(0);
        }
        if let Some(i) = self.blink {
            layers.push(Layer {
                clip: ClipId::Blink,
                frame: i as f32,
                weight: 1.,
                mirror: false,
                blend: Blend::Additive { reference: 0. },
            });
            if i + 1 < u32::from(ClipId::Blink.frames()) {
                self.blink = Some(i + 1);
            } else {
                self.blink = None;
                let gap = BLINK_GAP + self.rand(BLINK_GAP_RAND);
                self.next_blink = self.now + u64::from(gap);
            }
        }

        // 0x2002ac returns +-72 degrees by phys+0x490 during a lane change and also while a
        // stop/turn clip plays (0x77c >= 0) (FOUND). The sign is the side he faces: the
        // stick's side, else his travel, else the current yaw (INFERRED; 0x490 is the same
        // +-1 the grab branch turns into +-90 degrees). Holding the side matters for the
        // stop: its body turns to the camera within the clip (-90 relative to its root by
        // frame 20), so easing to 0 underneath it overshot to a three-quarter left view.
        // D251, MEASURED on three captured reversals: through walk_fast_turn the game's
        // facing eases 0.4 toward the plain speed angle (within 0.4 degrees for the first 16
        // ticks), with no hold; holding 72 degrees on the stick's side had him up to 111
        // degrees off. So for our purposes 0x77c >= 0 means the stop clip only (INFERRED).
        let stopping = self
            .shot
            .as_ref()
            .is_some_and(|sh| sh.clip == ClipId::WalkFastStop);
        let target = if lane_active || stopping {
            let side = if stick != 0. {
                stick
            } else if vx != 0. {
                vx
            } else if self.yaw != 0. {
                self.yaw
            } else {
                1.
            };
            LANE_YAW.copysign(side)
        } else {
            facing_target(vx)
        };
        self.yaw += FACING_EASE * (target - self.yaw);

        // Head look (FOUND, 0x200aa0 -> 0x2007e4), last in the stack: the dispatcher runs
        // it after the arms. Attention w eases 0.1 toward 1 - S and is then scaled in place
        // by clamp(1 - |0x6d4| / scale). S = P + |v - cfg[0xc]| / scale (0x1fb6dc) is taken
        // as |vx| / WalkMax: v / scale is the anim-speed pair (FOUND), cfg[0xc] = 0, the
        // strain term P = 0 and 0x6d4 = vx are INFERRED. Not modelled, so 0 here: the look
        // inputs (right stick A, target B with the creature+0xc20 object) and the relative
        // tilt term f25; with those at 0 every axis sits on its neutral frame. The skip
        // states (c14 0xc, 2 with +0xf20 = 2) and the L3 free-look toggle are not modelled.
        let r = vx.abs() / WALK_MAX_DESIRED;
        self.attention += LOOK_EASE * ((1. - r) - self.attention);
        self.attention *= (1. - r).clamp(0., 1.);
        let sgn = if vx < 0. { -1. } else { 1. };
        let look = look_axes(
            [0.; 3],
            0.,
            self.attention,
            0.2 * w_slow + 0.8 * w_fast,
            sgn,
            self.yaw,
        );
        for (clip, v) in LOOK_CLIPS.into_iter().zip(look) {
            let (frame, reference) = look_frames(clip, v);
            layers.push(Layer {
                clip,
                frame,
                weight: 1.,
                mirror: false,
                blend: Blend::Additive { reference },
            });
        }
        AnimFrame {
            layers,
            yaw: self.yaw,
        }
    }

    /// Moves the base clip on one frame; at a clip end, the chain's next clip (FOUND).
    fn advance_idle(&mut self, idle_ok: bool) {
        self.idle_frame += 1;
        if self.idle_frame < u32::from(self.idle.frames()) {
            return;
        }
        self.idle_frame = 0;
        let current = self.idle;
        self.idle = match current {
            ClipId::Stand if idle_ok => idle_for_roll(self.rand(IDLE_ROLL)),
            ClipId::StandIIdleSit => ClipId::IdleSit,
            // Stands up 5 % of the loops, or when idle is no longer allowed.
            ClipId::IdleSit => {
                if !idle_ok || self.rand(SIT_ROLL) > SIT_STAND_UP_ABOVE {
                    // Standing up resets the no-input counter (FOUND, 0x20e05c).
                    self.quiet_ticks = 0;
                    ClipId::IdleSitIStand
                } else {
                    ClipId::IdleSit
                }
            }
            ClipId::StandIIdleBored => {
                self.idle_loops = 0;
                ClipId::IdleBored
            }
            ClipId::IdleBored => {
                self.idle_loops += 1;
                if self.idle_loops > 10 || !idle_ok {
                    ClipId::IdleBoredIStand
                } else {
                    ClipId::IdleBored
                }
            }
            _ => ClipId::Stand,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sackboy::{Contact, Input};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn moving(vx: f32) -> Sackboy {
        let mut s = Sackboy::at_lane(0.);
        s.vx = vx;
        s
    }

    /// Moving at `vx` with the stick held toward `x` (one controller step for the input,
    /// then the speed put back).
    fn holding(vx: f32, x: f32) -> Sackboy {
        let lanes = |_: f32| true;
        let mut s = Sackboy::at_lane(0.);
        s.step(
            Input {
                x,
                ..Default::default()
            },
            &Contact {
                grounded: true,
                lane_has_floor: &lanes,
            },
        );
        s.vx = vx;
        s
    }

    fn weight_of(f: &AnimFrame, clip: ClipId) -> f32 {
        f.layers
            .iter()
            .filter(|l| l.clip == clip)
            .map(|l| l.weight)
            .sum()
    }

    fn layer(f: &AnimFrame, clip: ClipId) -> Option<Layer> {
        f.layers.iter().copied().find(|l| l.clip == clip)
    }

    const JUMP_CLIPS: [ClipId; 4] = [
        ClipId::JumpSmallStand,
        ClipId::JumpStand,
        ClipId::JumpSmallWalk,
        ClipId::JumpWalk,
    ];

    /// The jump layer's total weight over the pose under it (its clips are sequential
    /// lerps) and each clip's part of the result, in [`JUMP_CLIPS`] order.
    fn jump_parts(f: &AnimFrame) -> (f32, [f32; 4]) {
        let mut under = 1.;
        let mut parts = [0.; 4];
        for l in f.layers.iter().filter(|l| JUMP_CLIPS.contains(&l.clip)) {
            parts.iter_mut().for_each(|p| *p *= 1. - l.weight);
            under *= 1. - l.weight;
            parts[JUMP_CLIPS.iter().position(|&c| c == l.clip).unwrap()] += l.weight;
        }
        (1. - under, parts)
    }

    #[test]
    fn clip_table() {
        let guids: Vec<u32> = ClipId::ALL.iter().map(|c| c.guid()).collect();
        assert_eq!(guids.len(), 56);
        let mut unique = guids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 56);
        assert_eq!(ClipId::EmoteAnticipation.guid(), 0x25bf);
        assert_eq!(ClipId::EmoteSad3Loop.frames(), 51);
        assert_eq!(
            MOOD_STANDS.map(ClipId::guid),
            [0x128a, 0x128b, 0x1289, 0x128c]
        );
        assert_eq!(guids[..4], [0x1241, 0x1242, 0x1243, 0x1244]);
        assert_eq!(ClipId::ALL[0], ClipId::Stand);
        for (i, c) in ClipId::ALL.iter().enumerate() {
            assert_eq!(c.index(), i);
        }
    }

    #[test]
    fn walk_weights_follow_the_found_ramps() {
        // Full slow at 3 u/tick, full medium at 12, full fast at 30 (rules note).
        assert_eq!(walk_weights(0.), [0., 0., 0.]);
        assert!(close(walk_weights(1.5)[0], 0.5));
        assert_eq!(walk_weights(3.), [1., 0., 0.]);
        assert!(close(walk_weights(-7.5)[1], 0.5));
        assert_eq!(walk_weights(12.), [1., 1., 0.]);
        // r 0.7 (21 u/tick): w_fast 0.5, the stop/turn gate.
        assert!(close(walk_weights(21.)[2], 0.5));
        assert_eq!(walk_weights(30.), [1., 1., 1.]);
        assert_eq!(walk_weights(-45.), [1., 1., 1.]);
    }

    #[test]
    fn walk_layers_share_one_13_tick_phase_at_any_speed() {
        for vx in [3., 15., 30.] {
            let mut a = SackboyAnim::new();
            let s = moving(vx);
            let frames: Vec<f32> = (0..14)
                .map(|_| layer(&a.tick(&s, true), ClipId::WalkSlow).unwrap().frame)
                .collect();
            let want: Vec<f32> = (0..13).chain(0..1).map(|f| f as f32).collect();
            assert_eq!(frames, want, "vx {vx}");
        }
        // At 15 u/tick: stand, slow 1, medium 1, fast 0.1667; all at the same frame (the
        // tick-0 blink, additive, dropped).
        let mut f = SackboyAnim::new().tick(&moving(15.), true);
        f.layers.retain(|l| l.blend == Blend::Lerp);
        let clips: Vec<ClipId> = f.layers.iter().map(|l| l.clip).collect();
        assert_eq!(
            clips,
            [
                ClipId::Stand,
                ClipId::WalkSlow,
                ClipId::WalkMedium,
                ClipId::WalkFast
            ]
        );
        assert!(close(f.layers[3].weight, 1. / 6.));
        let eff = f.effective_weights();
        assert!(close(eff.iter().sum::<f32>(), 1.));
        assert!(close(eff[0], 0.) && close(eff[1], 0.) && close(eff[3], 1. / 6.));
    }

    #[test]
    fn walking_left_flags_the_walk_layers_after_five_ticks() {
        let mut a = SackboyAnim::new();
        let s = moving(-20.);
        let flags: Vec<bool> = (0..6)
            .map(|_| layer(&a.tick(&s, true), ClipId::WalkFast).unwrap().mirror)
            .collect();
        assert_eq!(flags, [false, false, false, false, true, true]);
    }

    #[test]
    fn facing_eases_toward_the_speed_angle() {
        let mut a = SackboyAnim::new();
        assert_eq!(a.tick(&moving(0.), true).yaw, 0.);
        // Full speed right, stick held: 0.4 of the way per tick toward +90 degrees.
        let mut a = SackboyAnim::new();
        let run = holding(30., 1.);
        let y1 = a.tick(&run, true).yaw;
        assert!(close(y1, 0.4 * FRAC_PI_2));
        for _ in 0..4 {
            a.tick(&run, true);
        }
        // About 90 % of the way in 4.5 ticks (1 - 0.6^5 = 0.922 after five).
        assert!(close(a.yaw(), (1. - 0.6f32.powi(5)) * FRAC_PI_2));
        // Half speed is three-quarter (45 degrees); left is negative.
        let mut a = SackboyAnim::new();
        for _ in 0..60 {
            a.tick(&moving(-15.), true);
        }
        assert!(close(a.yaw(), -FRAC_PI_2 / 2.));
    }

    #[test]
    fn stop_holds_72_degrees_and_turn_follows_the_speed() {
        // Stop moving left: the target stays -72 while walk_fast_stop plays, then 0.
        let mut a = SackboyAnim::new();
        for _ in 0..30 {
            a.tick(&holding(-30., -1.), true);
        }
        let mut s = moving(-30.);
        let mut yaws = Vec::new();
        for i in 0..40 {
            s.vx = (-30. + 3. * i as f32).min(0.);
            let f = a.tick(&s, true);
            yaws.push((layer(&f, ClipId::WalkFastStop).is_some(), f.yaw));
        }
        let during: Vec<f32> = yaws.iter().filter(|y| y.0).map(|y| y.1).collect();
        let after: Vec<f32> = yaws.iter().filter(|y| !y.0).map(|y| y.1).collect();
        assert_eq!(during.len(), 21);
        // Held through frame 19; the last frame (20) already eases back toward 0.
        assert!(close(during[19], -LANE_YAW), "{}", during[19]);
        assert!(close(during[20], -LANE_YAW * 0.6), "{}", during[20]);
        assert!(after.last().unwrap().abs() < 0.01);
        // Turn: stick pushed right while running left. The facing keeps easing toward the
        // speed angle (D251): 3 per tick through zero, as the controller decelerates.
        let mut a = SackboyAnim::new();
        for _ in 0..30 {
            a.tick(&holding(-30., -1.), true);
        }
        let mut want = a.yaw();
        for i in 0..18 {
            let vx = -27. + 3. * i as f32;
            let f = a.tick(&holding(vx, 1.), true);
            assert!(layer(&f, ClipId::WalkFastTurn).is_some());
            want += FACING_EASE * (facing_target(vx) - want);
            assert!(close(f.yaw, want), "{i} {} {want}", f.yaw);
        }
        // Moving right at 24 by now: well past the camera, not held at 72 degrees.
        assert!(want > 0.6 && (want - LANE_YAW).abs() > 0.05, "{want}");
    }

    #[test]
    fn jump_frame_numbers() {
        assert!(close(jump_frame(0.), 8.)); // apex
        assert!(close(jump_frame(-30.26), 14.)); // full fall
        assert!(close(jump_frame(-200.), 14.));
        assert!(close(jump_frame(-15.13), 11.));
        assert!(close(jump_frame(36.5), 4.));
        assert!(close(jump_frame(73.), 0.));
    }

    #[test]
    fn stop_and_turn_ramp_over_five_frames() {
        assert_eq!(
            (0..21).map(|i| ramp(i, 21, 5.)).collect::<Vec<_>>()[..7],
            [0., 0.2, 0.4, 0.6, 0.8, 1., 1.]
        );
        assert!(close(ramp(20, 21, 5.), 0.2));
        // Stick released at full speed: walk_fast_stop plays 21 frames, mirrored moving left.
        let mut a = SackboyAnim::new();
        let mut s = moving(-30.);
        let f = a.tick(&s, true);
        let stop = layer(&f, ClipId::WalkFastStop).unwrap();
        assert_eq!((stop.frame, stop.weight, stop.mirror), (0., 0., true));
        s.vx = 0.;
        let count = 1
            + (0..30)
                .filter(|_| layer(&a.tick(&s, true), ClipId::WalkFastStop).is_some())
                .count();
        assert_eq!(count, 21);
        // Below w_fast 0.5 (r 0.7) nothing starts.
        let f = SackboyAnim::new().tick(&moving(20.), true);
        assert!(layer(&f, ClipId::WalkFastStop).is_none());
    }

    #[test]
    fn turn_plays_19_frames_then_resets_the_walk_phase() {
        let mut a = SackboyAnim::new();
        let mut s = moving(30.);
        // Push the stick hard the other way.
        let push = |a: &mut SackboyAnim, s: &mut Sackboy, x: f32| {
            let lanes = |_: f32| true;
            let mut t = *s;
            t.step(
                Input {
                    x,
                    ..Default::default()
                },
                &Contact {
                    grounded: true,
                    lane_has_floor: &lanes,
                },
            );
            // Keep the speed fixed so only the stick changes.
            t.vx = s.vx;
            *s = t;
            a.tick(s, true)
        };
        for _ in 0..3 {
            push(&mut a, &mut s, 1.);
        }
        let mut turn = 0;
        let mut last = AnimFrame::default();
        for _ in 0..19 {
            last = push(&mut a, &mut s, -1.);
            turn += usize::from(layer(&last, ClipId::WalkFastTurn).is_some());
        }
        assert_eq!(turn, 19);
        assert_eq!(layer(&last, ClipId::WalkFastTurn).unwrap().frame, 18.);
        // The tick after the turn ends shows walk frame 1 (and a new turn starts).
        let f = push(&mut a, &mut s, -1.);
        assert_eq!(layer(&f, ClipId::WalkFast).unwrap().frame, 1.);
    }

    #[test]
    fn lane_clip_skips_three_frames_and_ramps_over_three() {
        let mut a = SackboyAnim::new();
        let mut s = Sackboy::at_lane(0.);
        s.lane = -200.;
        let mut got = Vec::new();
        let mut yaw = Vec::new();
        for _ in 0..12 {
            let f = a.tick(&s, true);
            if let Some(l) = layer(&f, ClipId::WalkZBackwards) {
                got.push((l.frame, l.weight));
                yaw.push(f.yaw);
            }
        }
        let frames: Vec<f32> = got.iter().map(|g| g.0).collect();
        assert_eq!(frames, (3..13).map(|f| f as f32).collect::<Vec<_>>());
        let w: Vec<f32> = got.iter().map(|g| g.1).collect();
        for (i, want) in [1. / 3., 2. / 3., 1., 1., 1., 1., 1., 1., 2. / 3., 1. / 3.]
            .iter()
            .enumerate()
        {
            assert!(close(w[i], *want), "{i}: {}", w[i]);
        }
        // Facing turns toward 72 degrees while it plays.
        assert!(close(yaw[9], LANE_YAW * (1. - 0.6f32.powi(10))));
    }

    #[test]
    fn idle_roll_table_and_lcg() {
        assert_eq!(idle_for_roll(0), ClipId::StandIIdleSit);
        assert_eq!(idle_for_roll(2), ClipId::StandIIdleSit);
        assert_eq!(idle_for_roll(4), ClipId::IdleYawn);
        assert_eq!(idle_for_roll(6), ClipId::StandIIdleBored);
        assert_eq!(idle_for_roll(9), ClipId::IdleFidget);
        assert_eq!(idle_for_roll(10), ClipId::IdleShiftFeet);
        assert_eq!(idle_for_roll(11), ClipId::Stand);
        assert_eq!(lcg_next(0), 0xe908fc1);
        assert!(lcg_next(u32::MAX) < 1 << 30);
        assert_eq!(lcg_rand(0x3fff_ffff, 1000), 999);
        assert_eq!(lcg_rand(0, 1000), 0);
        // The idle roll is 0..249 (mulli 0xfa).
        assert_eq!(lcg_rand(0x3fff_ffff, IDLE_ROLL), 249);
        // Over the LCG from seed 42, the five idles take about 11 rolls in 250.
        let mut seed = LCG_SEED;
        let n = 100_000;
        let idles = (0..n)
            .filter(|_| {
                seed = lcg_next(seed);
                idle_for_roll(lcg_rand(seed, IDLE_ROLL)) != ClipId::Stand
            })
            .count();
        let rate = idles as f32 / n as f32;
        assert!((rate - 11. / 250.).abs() < 0.005, "{rate}");
    }

    #[test]
    fn idle_waits_for_300_quiet_ticks() {
        // Standing with no input: stand only until the counter passes 300, then the chain.
        let mut a = SackboyAnim::new();
        let still = moving(0.);
        let first_idle = (0..5_000u32)
            .find(|_| a.tick(&still, true).layers[0].clip != ClipId::Stand)
            .unwrap();
        assert!(first_idle > IDLE_AFTER_TICKS, "{first_idle}");
        // Any input (here the stick, at zero speed) resets the counter: no idle ever starts.
        let mut a = SackboyAnim::new();
        let pushing = holding(0., 1.);
        for t in 0..5_000u32 {
            let s = if t % 250 == 0 { &pushing } else { &still };
            assert_eq!(a.tick(s, true).layers[0].clip, ClipId::Stand, "tick {t}");
        }
    }

    #[test]
    fn layers_run_walk_stop_jump_then_lane() {
        // Start a lane change toward +z (dz > 0: walk_z_forwards), then fall while it plays.
        let mut a = SackboyAnim::new();
        let mut s = moving(0.);
        s.lane = 200.;
        let f = a.tick(&s, true);
        assert!(layer(&f, ClipId::WalkZForwards).is_some());
        s.vy = -25.;
        s.vx = 10.;
        let f = a.tick(&s, false);
        let at = |c: ClipId| f.layers.iter().position(|l| l.clip == c).unwrap();
        assert!(at(ClipId::Stand) < at(ClipId::WalkSlow));
        assert!(at(ClipId::WalkSlow) < at(ClipId::JumpSmallStand));
        assert!(at(ClipId::JumpSmallStand) < at(ClipId::WalkZForwards));
        assert!(at(ClipId::WalkZForwards) < at(ClipId::Blink));
    }

    #[test]
    fn idle_chains_and_blinks_while_standing() {
        let mut a = SackboyAnim::new();
        let s = moving(0.);
        let mut bases = Vec::new();
        let mut blinks = Vec::new();
        for t in 0..20_000u32 {
            let f = a.tick(&s, true);
            let expect = if f.layers[0].clip == ClipId::Stand {
                STAND_BASE_WEIGHT
            } else {
                1.
            };
            assert_eq!(f.layers[0].weight, expect);
            if bases.last() != Some(&f.layers[0].clip) {
                bases.push(f.layers[0].clip);
            }
            if let Some(b) = layer(&f, ClipId::Blink) {
                assert_eq!(b.blend, Blend::Additive { reference: 0. });
                if b.frame == 0. {
                    blinks.push(t);
                }
            }
        }
        // Intros lead into their loops and loops back out through their outros.
        for w in bases.windows(2) {
            let ok = match w[0] {
                ClipId::Stand => true,
                ClipId::StandIIdleSit => w[1] == ClipId::IdleSit,
                ClipId::IdleSit => w[1] == ClipId::IdleSitIStand,
                ClipId::StandIIdleBored => w[1] == ClipId::IdleBored,
                ClipId::IdleBored => w[1] == ClipId::IdleBoredIStand,
                _ => w[1] == ClipId::Stand,
            };
            assert!(ok, "{:?} -> {:?}", w[0], w[1]);
        }
        assert!(bases.len() > 3, "{bases:?}");
        // First blink on the first tick; the next starts 30 + rand(70) ticks after the last
        // blink frame (12 ticks after its start).
        assert_eq!(blinks[0], 0);
        for w in blinks.windows(2) {
            let gap = w[1] - w[0];
            assert!((42..=111).contains(&gap), "{gap}");
        }
        // Walking with the stick held never starts the chain.
        let mut a = SackboyAnim::new();
        let run = holding(30., 1.);
        for _ in 0..1_000 {
            let f = a.tick(&run, true);
            assert_eq!(f.layers[0].clip, ClipId::Stand);
        }
    }

    /// Standing with the mood d-pad direction held (or released).
    fn dpad(mood: Option<Mood>) -> Sackboy {
        let lanes = |_: f32| true;
        let mut s = Sackboy::at_lane(0.);
        s.step(
            Input {
                mood,
                ..Default::default()
            },
            &Contact {
                grounded: true,
                lane_has_floor: &lanes,
            },
        );
        s
    }

    /// One press: the direction down for a tick, then released for `after` ticks.
    fn press(a: &mut SackboyAnim, mood: Mood, after: usize) -> Vec<AnimFrame> {
        let mut out = vec![a.tick(&dpad(Some(mood)), true)];
        out.extend((0..after).map(|_| a.tick(&dpad(None), true)));
        out
    }

    #[test]
    fn same_mood_cycles_levels_one_two_three_off() {
        let mut a = SackboyAnim::new();
        assert_eq!(a.mood(), None);
        let mut seen = Vec::new();
        for _ in 0..5 {
            press(&mut a, Mood::Angry, 2);
            seen.push(a.mood());
        }
        let on = |l| Some((Mood::Angry, l));
        assert_eq!(seen, [on(1), on(2), on(3), None, on(1)]);
        // Holding the direction is one press (edge), and there is no timeout.
        let mut a = SackboyAnim::new();
        for _ in 0..2_000 {
            a.tick(&dpad(Some(Mood::Scared)), true);
        }
        assert_eq!(a.mood(), Some((Mood::Scared, 1)));
        // Movement and a jump do not cancel it.
        a.tick(&holding(30., 1.), true);
        a.tick(&holding(30., 1.), false);
        assert_eq!(a.mood(), Some((Mood::Scared, 1)));
        // Stepping off from level 3 plays that level's to_neutral clip.
        let mut a = SackboyAnim::new();
        for _ in 0..3 {
            press(&mut a, Mood::Happy, 1);
        }
        let f = press(&mut a, Mood::Happy, 0);
        assert!(layer(&f[0], ClipId::EmoteHappy3ToNeutral).is_some());
    }

    #[test]
    fn opposite_mood_cancels_and_another_replaces() {
        assert_eq!(Mood::Happy.opposite(), Mood::Sad);
        assert_eq!(Mood::Scared.opposite(), Mood::Angry);
        let mut a = SackboyAnim::new();
        press(&mut a, Mood::Happy, 1);
        press(&mut a, Mood::Happy, 1);
        // Sad ends happy through happy's level-2 to_neutral and does not start itself.
        let f = press(&mut a, Mood::Sad, 1);
        assert_eq!(a.mood(), None);
        assert_eq!(
            a.emote().map(|e| (e.0, e.1)),
            Some((Mood::Happy, EmoteState::ToNeutral))
        );
        let l = layer(&f[0], ClipId::EmoteHappy2ToNeutral).unwrap();
        assert_eq!(l.frame, 0.);
        // The next sad press starts sad.
        press(&mut a, Mood::Sad, 1);
        assert_eq!(a.mood(), Some((Mood::Sad, 1)));
        // A mood that is not the opposite replaces it at level 1, from the anticipation.
        press(&mut a, Mood::Sad, 1);
        let f = press(&mut a, Mood::Scared, 0);
        assert_eq!(a.mood(), Some((Mood::Scared, 1)));
        let l = layer(&f[0], ClipId::EmoteAnticipation).unwrap();
        assert_eq!(l.frame, 0.);
    }

    #[test]
    fn emote_plays_anticipation_then_the_clip_then_holds_under_the_mood_stand() {
        let mut a = SackboyAnim::new();
        let frames = press(&mut a, Mood::Happy, 39);
        let seq: Vec<(ClipId, f32)> = frames
            .iter()
            .map(|f| (f.layers[1].clip, f.layers[1].frame))
            .collect();
        // 6 anticipation frames, the 16 emote frames, then the last frame held.
        let want: Vec<(ClipId, f32)> = (0..6)
            .map(|i| (ClipId::EmoteAnticipation, i as f32))
            .chain((0..16).map(|i| (ClipId::EmoteHappy, i as f32)))
            .chain((0..18).map(|_| (ClipId::EmoteHappy, 15.)))
            .collect();
        assert_eq!(seq, want);
        assert!(frames.iter().all(|f| f.layers[0].clip == ClipId::Stand));
        assert!(frames.iter().all(|f| f.layers[1].weight == 1.));
        // stand_happy comes on by the mood weight once the emote is on its last frame.
        for (t, f) in frames.iter().enumerate() {
            let stand = layer(f, ClipId::StandHappy);
            assert_eq!(stand.is_some(), t >= 21, "tick {t}");
            if let Some(s) = stand {
                assert!(close(s.weight, 1. - 0.9f32.powi(t as i32 + 1)));
                assert_eq!(f.layers[2].clip, ClipId::StandHappy);
            }
        }
        assert_eq!(a.emote(), Some((Mood::Happy, EmoteState::Emote, 15)));
        assert!(a.emote_held());
        // Level 2 restarts from the anticipation into emote_happy_2 (18 frames).
        let frames = press(&mut a, Mood::Happy, 30);
        assert_eq!(frames[0].layers[1].clip, ClipId::EmoteAnticipation);
        assert!(layer(&frames[0], ClipId::StandHappy).is_none());
        assert_eq!(frames[6].layers[1].clip, ClipId::EmoteHappy2);
        assert_eq!(frames[30].layers[1].frame, 17.);
    }

    #[test]
    fn to_neutral_plays_through_then_the_mood_resets() {
        let mut a = SackboyAnim::new();
        press(&mut a, Mood::Angry, 40);
        let frames = press(&mut a, Mood::Scared, 80);
        let tail: Vec<f32> = frames
            .iter()
            .filter_map(|f| layer(f, ClipId::EmoteAngryToNeutral).map(|l| l.frame))
            .collect();
        assert_eq!(tail, (0..16).map(|i| i as f32).collect::<Vec<_>>());
        // stand_angry fades with the weight (x 0.9 per tick), then leaves the stack.
        let w0 = layer(&frames[0], ClipId::StandAngry).unwrap().weight;
        let w5 = layer(&frames[5], ClipId::StandAngry).unwrap().weight;
        assert!(close(w5, w0 * 0.9f32.powi(5)));
        assert_eq!(a.emote(), None);
        assert_eq!(a.mood(), None);
        let last = frames.last().unwrap();
        let lerps: Vec<ClipId> = last
            .layers
            .iter()
            .filter(|l| l.blend == Blend::Lerp)
            .map(|l| l.clip)
            .collect();
        assert_eq!(lerps, [ClipId::Stand]);
    }

    #[test]
    fn mood_weights_ease_a_tenth_per_tick() {
        let mut a = SackboyAnim::new();
        press(&mut a, Mood::Sad, 0);
        assert!(close(a.mood_weights()[1], 0.1));
        a.tick(&dpad(None), true);
        assert!(close(a.mood_weights()[1], 0.19));
        for _ in 0..8 {
            a.tick(&dpad(None), true);
        }
        assert!(close(a.mood_weights()[1], 1. - 0.9f32.powi(10)));
        assert_eq!(a.mood_weights()[0], 0.);
        // Replaced by angry: sad falls by a tenth of itself, angry rises.
        let w = a.mood_weights()[1];
        press(&mut a, Mood::Angry, 0);
        assert!(close(a.mood_weights()[1], 0.9 * w));
        assert!(close(a.mood_weights()[2], 0.1));
    }

    #[test]
    fn sad_levels_two_and_three_loop() {
        let clips = |frames: &[AnimFrame]| -> Vec<(ClipId, f32)> {
            frames
                .iter()
                .map(|f| (f.layers[1].clip, f.layers[1].frame))
                .collect()
        };
        let mut a = SackboyAnim::new();
        press(&mut a, Mood::Sad, 1);
        let two = clips(&press(&mut a, Mood::Sad, 6 + 19 + 2));
        assert_eq!(two[6], (ClipId::EmoteSad2Loop, 0.));
        assert_eq!(two[6 + 18], (ClipId::EmoteSad2Loop, 18.));
        assert_eq!(two[6 + 19], (ClipId::EmoteSad2Loop, 0.));
        let three = clips(&press(&mut a, Mood::Sad, 6 + 14 + 51 + 2));
        assert_eq!(three[6], (ClipId::EmoteSad3Into, 0.));
        assert_eq!(three[6 + 13], (ClipId::EmoteSad3Into, 13.));
        assert_eq!(three[6 + 14], (ClipId::EmoteSad3Loop, 0.));
        assert_eq!(three[6 + 14 + 50], (ClipId::EmoteSad3Loop, 50.));
        assert_eq!(three[6 + 14 + 51], (ClipId::EmoteSad3Loop, 0.));
    }

    #[test]
    fn a_mood_blocks_the_idle_chain() {
        let mut a = SackboyAnim::new();
        press(&mut a, Mood::Happy, 0);
        let still = moving(0.);
        for _ in 0..5_000 {
            assert_eq!(a.tick(&still, true).layers[0].clip, ClipId::Stand);
        }
        assert_eq!(a.mood(), Some((Mood::Happy, 1)));
        // Mood off again: the chain starts as before.
        press(&mut a, Mood::Sad, 0);
        let mut idle = false;
        for _ in 0..5_000 {
            idle |= a.tick(&still, true).layers[0].clip != ClipId::Stand;
        }
        assert!(idle);
    }

    #[test]
    fn look_layers_close_the_stack_on_their_reference_frames() {
        let mut a = SackboyAnim::new();
        for s in [moving(0.), moving(30.), moving(-12.)] {
            let f = a.tick(&s, true);
            let look = &f.layers[f.layers.len() - 3..];
            let clips: Vec<ClipId> = look.iter().map(|l| l.clip).collect();
            assert_eq!(clips, LOOK_CLIPS);
            // No look input is modelled: each axis sits on its neutral frame 0.5 n.
            for (l, want) in look.iter().zip([4., 4., 8.]) {
                assert_eq!(l.blend, Blend::Additive { reference: want });
                assert_eq!((l.frame, l.weight, l.mirror), (want, 1., false));
            }
        }
        assert_eq!(look_frames(ClipId::LookUpdown, 1.), (8., 4.));
        assert_eq!(look_frames(ClipId::LookLeftright, -1.), (0., 4.));
        assert_eq!(look_frames(ClipId::LookRoll, 0.5), (12., 8.));
        assert_eq!(look_frames(ClipId::LookRoll, 7.), (16., 8.));
    }

    #[test]
    fn look_axes_follow_the_found_formula() {
        // Standing (wf 0): the tilt term goes to the roll, the inputs pass through.
        let v = look_axes([0.2, 0.5, 0.1], 0.5, 1., 0., 1., 0.);
        assert!(close(v[0], 0.2) && close(v[1], 0.5) && close(v[2], 0.6));
        // Full walk left (wf 1, sgn -1): half the tilt on up/down, none on the roll.
        let v = look_axes([0.2, 0.5, 0.1], 0.5, 1., 1., -1., 0.);
        assert!(close(v[0], -0.05) && close(v[2], 0.1));
        // Attention scales the tilt term; outputs clamp to [-1, 1].
        assert!(close(look_axes([0.; 3], 0.5, 0.4, 0., 1., 0.)[2], 0.2));
        assert_eq!(look_axes([3., -3., 0.], 9., 1., 0., 1., 0.), [1., -1., 1.]);
        // Looking the way he faces is reduced by yaw / 90 degrees; the other way is not.
        let side = |y: f32, yaw: f32| look_axes([0., y, 0.], 0., 1., 0., 1., yaw)[1];
        assert!(close(side(0.5, FRAC_PI_2 / 2.), 0.));
        assert!(close(side(-0.5, -FRAC_PI_2 / 4.), -0.25));
        assert!(close(side(-0.5, FRAC_PI_2), -0.5));
    }

    #[test]
    fn attention_rises_standing_and_drops_with_speed() {
        let mut a = SackboyAnim::new();
        a.tick(&moving(0.), true);
        assert!(close(a.attention(), 0.1));
        for _ in 0..200 {
            a.tick(&moving(0.), true);
        }
        assert!(a.attention() > 0.999);
        // Half speed: 0.1 of the way toward 0.5, then scaled by 0.5.
        a.tick(&moving(15.), true);
        assert!(close(a.attention(), 0.95 * 0.5), "{}", a.attention());
        a.tick(&moving(30.), true);
        assert_eq!(a.attention(), 0.);
    }

    /// Drives the real controller over a flat floor; returns the per-tick frames.
    fn run(inputs: impl Fn(usize) -> Input, ticks: usize) -> Vec<AnimFrame> {
        let lanes = |_: f32| true;
        let mut s = Sackboy::at_lane(0.);
        let mut a = SackboyAnim::new();
        let mut y = 0f32;
        (0..ticks)
            .map(|t| {
                let grounded = y <= 0. && s.vy <= 0.;
                let d = s.step(
                    inputs(t),
                    &Contact {
                        grounded,
                        lane_has_floor: &lanes,
                    },
                );
                // As `character::drive_sackboy`: a fall stopped by the floor zeroes vy and
                // counts as contact on that same tick.
                let touched_down = d[1] < 0. && y + d[1] <= 0.;
                if touched_down {
                    s.vy = 0.;
                }
                y = (y + d[1]).max(0.);
                a.tick(&s, grounded || touched_down)
            })
            .collect()
    }

    #[test]
    fn standing_jump_takes_off_peaks_falls_and_lands_on_the_tail() {
        let frames = run(
            |t| Input {
                jump: (5..=14).contains(&t),
                ..Default::default()
            },
            70,
        );
        let jump: Vec<(usize, f32, f32)> = frames
            .iter()
            .enumerate()
            .filter_map(|(t, f)| layer(f, ClipId::JumpStand).map(|l| (t, l.frame, l.weight)))
            .collect();
        // Take-off on tick 5: frames 1..5 on the first five airborne ticks.
        let first: Vec<f32> = jump[..5].iter().map(|j| j.1).collect();
        assert_eq!(first, [1., 2., 3., 4., 5.]);
        assert_eq!(jump[0].0, 5);
        // D245: the layer is at full weight from the take-off, and jump_stand's part of it
        // is the hold count over 10 (button held 10 ticks here), jump_small_stand the rest.
        for (k, j) in jump[..10].iter().enumerate() {
            assert!(close(j.2, (k + 1) as f32 / 10.), "{k} {j:?}");
            assert!(close(jump_parts(&frames[j.0]).0, 1.));
            let small = layer(&frames[j.0], ClipId::JumpSmallStand);
            assert_eq!(small.is_some(), k < 9, "{k}");
            assert!(small.is_none_or(|l| l.frame == j.1));
        }
        // The frame reaches the apex (8) and the full-fall range, never past 14 in the air.
        let air_max = jump
            .iter()
            .map(|j| j.1)
            .take_while(|&f| f <= 14.)
            .fold(0f32, f32::max);
        assert!(air_max > 8. && air_max <= 14.);
        // Landing: the tail plays one frame per tick from int((airFrame + 15) / 2) and fades.
        let tail: Vec<(f32, f32)> = jump
            .iter()
            .filter(|j| j.1 > 14.)
            .map(|j| (j.1, j.2))
            .collect();
        assert!(tail.len() > 5, "{jump:?}");
        for w in tail.windows(2) {
            assert_eq!(w[1].0, w[0].0 + 1.);
            assert!(w[1].1 <= w[0].1);
        }
        assert!(tail.last().unwrap().0 <= 30.);
        // Standing still: no jump_walk.
        assert!(frames.iter().all(|f| layer(f, ClipId::JumpWalk).is_none()));
    }

    #[test]
    fn upper_body_eases_back_toward_stand_in_the_air() {
        let frames = run(
            |t| Input {
                jump: (5..11).contains(&t),
                ..Default::default()
            },
            150,
        );
        let upper = |f: &AnimFrame| {
            let l: Vec<_> = f
                .layers
                .iter()
                .filter(|l| l.blend == Blend::UpperBody)
                .collect();
            assert!(l.len() <= 1);
            l.first().map_or(0., |l| {
                assert_eq!((l.clip, l.frame, l.mirror), (ClipId::Stand, 0., false));
                l.weight
            })
        };
        let air = |f: &AnimFrame| JUMP_CLIPS.iter().any(|&c| weight_of(f, c) > 0.);
        let first = frames.iter().position(air).expect("jumps");
        // Standing: no layer. The first jump tick still reads s = 0, then s = 0.1, 0.19, ...
        assert!(frames[..=first].iter().all(|f| upper(f) == 0.));
        let mut s = 0f32;
        for f in &frames[first..first + 12] {
            assert!(close(upper(f), s * (1. - s)), "{} {s}", upper(f));
            s += AIR_EASE * (1. - s);
        }
        // Never more than a quarter, and gone again some ticks after the landing.
        assert!(frames.iter().all(|f| upper(f) <= 0.25));
        assert_eq!(upper(frames.last().unwrap()), 0.);
        // It is not a lerp layer for the shares.
        let f = &frames[first + 8];
        assert!(close(f.effective_weights().iter().sum::<f32>(), 1.));
    }

    #[test]
    fn crouch_spring_follows_the_found_recurrence() {
        // A one-tick kick h (0x2011d4): v / h = 1, 1.5, 1.5, 1.125, 0.5625, 0, then the
        // rebound, a damped swing of period 12 ticks.
        let e = 0.25f32;
        let h = e.sqrt().powf(CROUCH_BIAS_POW);
        assert!(close(h, 0.2));
        let mut s = CrouchSpring::default();
        let v: Vec<f32> = (0..13)
            .map(|t| s.step(if t == 0 { e } else { 0. }) / h)
            .collect();
        let want = [
            1.,
            1.5,
            1.5,
            1.125,
            0.5625,
            0.,
            -0.421_875,
            -0.632_812_5,
            -0.632_812_5,
            -0.474_609_4,
            -0.237_304_7,
            0.,
            0.177_978_5,
        ];
        for (t, (v, w)) in v.iter().zip(want).enumerate() {
            assert!((v - w).abs() < 1e-4, "{t}: {v} {w}");
        }
        // A held input settles at v = 4h; every value stays inside [-1, 1].
        let mut s = CrouchSpring::default();
        let held = (0..200).map(|_| s.step(e)).last().unwrap();
        assert!(close(held, 0.8), "{held}");
        let mut s = CrouchSpring::default();
        assert!((0..50).all(|_| s.step(1.).abs() <= 1.));
    }

    #[test]
    fn crouch_spring_gives_nothing_without_input() {
        // No depth, or one under the cut (h < 0.01 below depth 6.6): exactly 0, no layer.
        let mut s = CrouchSpring::default();
        assert!((0..20).all(|_| s.step(0.) == 0.));
        assert!((0..20).all(|_| s.step(6. / CROUCH_DEPTH_FULL) == 0.));
        assert!(s.step(7. / CROUCH_DEPTH_FULL) > 0.01);
        let crouch = |f: &AnimFrame| layer(f, ClipId::StretchCrouch);
        // Standing, walking, stopping and a full jump with its landing.
        let frames = run(
            |t| Input {
                x: if (80..140).contains(&t) { 1. } else { 0. },
                jump: (5..=14).contains(&t),
                ..Default::default()
            },
            200,
        );
        assert!(frames.iter().all(|f| crouch(f).is_none()));
        // With a contact depth the layer is additive about the middle frame of the nine.
        let mut a = SackboyAnim::new();
        let s = moving(0.);
        a.set_contact_depth(350. * 0.25);
        let f = a.tick(&s, true);
        let l = crouch(&f).unwrap();
        assert!(close(l.frame, (0.5 * 0.2 + 0.5) * 8.) && l.weight == 1.);
        assert_eq!(l.blend, Blend::Additive { reference: 4. });
        // The depth is read once: the next tick is the spring's own second value.
        let l = crouch(&a.tick(&s, true)).unwrap();
        assert!(close(l.frame, (0.5 * 0.3 + 0.5) * 8.));
    }

    #[test]
    fn landing_tail_starts_on_the_touchdown_tick() {
        // D247, MEASURED (d234 `jump.bin`): mesh height 201.6 on the tick the feet reach
        // the floor, then 206.0, 202.9, 194.9; our jump poses at frames 14, 15, 16 are
        // 201.9, 206.3, 202.6. So the touchdown tick shows the tail's first frame
        // int((14 + 15) / 2) = 14 and the next tick 15 (it was 14, 14, 15 before).
        let frames = run(
            |t| Input {
                jump: (5..=10).contains(&t),
                ..Default::default()
            },
            60,
        );
        let jump: Vec<f32> = frames
            .iter()
            .filter_map(|f| layer(f, ClipId::JumpStand).map(|l| l.frame))
            .collect();
        let first_14 = jump.iter().position(|&f| f == 14.).unwrap();
        // One airborne tick at the full-fall frame 14 (vy -31.8 on the 6-tick jump), the
        // touchdown tick on 14 again, then one frame per tick.
        assert_eq!(jump[first_14..first_14 + 5], [14., 14., 15., 16., 17.]);
        assert!(jump[first_14 - 1] < 14.);
    }

    #[test]
    fn ledge_fall_eases_the_jump_layer_in_and_lands_at_full_weight() {
        // D257, MEASURED (ledgeR1, vx 32.2): fitted weights 0.00, 0.00, 0.06, 0.09, 0.23,
        // 0.41, 0.57, 0.73, 0.84, 1.00, 1.00 over the fall, then the landing tail at 1.00.
        let vys = [
            -1.4, -2.5, -5.2, -7.9, -10.6, -13.3, -16.0, -18.7, -21.4, -24.1, -26.8,
        ];
        let game = [0., 0., 0.06, 0.09, 0.23, 0.41, 0.57, 0.73, 0.84, 1., 1.];
        let mut a = SackboyAnim::new();
        let mut s = moving(32.2);
        for (vy, want) in vys.into_iter().zip(game) {
            s.vy = vy;
            let (w, _) = jump_parts(&a.tick(&s, false));
            assert!((w - want).abs() < 0.08, "vy {vy}: {w} vs {want}");
        }
        s.vy = 0.;
        let (w, _) = jump_parts(&a.tick(&s, true));
        assert!(close(w, 1.));
        // INFERRED: leaving the ground again under the tail keeps the weight.
        s.vy = -1.4;
        let (w, _) = jump_parts(&a.tick(&s, false));
        assert!(w > 0.8, "{w}");
    }

    #[test]
    fn late_contact_with_vy_zero_keeps_the_fall_frame() {
        // Falling at -25 (frame 12.96), then a tick that reports vy 0 before `grounded`
        // (a caller whose contact lags): the frame holds, and the tail starts at
        // int((12.96 + 15) / 2) = 13.
        let mut a = SackboyAnim::new();
        let mut s = moving(0.);
        s.vy = -25.;
        let f = a.tick(&s, false);
        // A fall has no hold count: jump_small_stand alone.
        assert!(layer(&f, ClipId::JumpStand).is_none());
        let fall = layer(&f, ClipId::JumpSmallStand).unwrap().frame;
        assert!(close(fall, 8. + 6. * 25. / 30.26));
        s.vy = 0.;
        let f = a.tick(&s, false);
        assert_eq!(layer(&f, ClipId::JumpSmallStand).unwrap().frame, fall);
        let f = a.tick(&s, true);
        assert_eq!(layer(&f, ClipId::JumpSmallStand).unwrap().frame, 13.);
    }

    #[test]
    fn running_jump_mixes_jump_walk_by_air_speed() {
        let frames = run(
            |t| Input {
                x: 1.,
                jump: (20..=24).contains(&t),
                ..Default::default()
            },
            40,
        );
        let f = &frames[22];
        // vx about 30 x 0.045 > 1: all walk jump clips; blend-in 1 + 4 x 30 / 20 = 7 ticks.
        assert!(weight_of(f, ClipId::JumpStand) < 1e-4);
        assert!(weight_of(f, ClipId::JumpSmallStand) < 1e-4);
        // Held 5 ticks: jump_walk's part of the layer is 0.1 per tick up to 0.5, then held.
        for (k, f) in frames[20..28].iter().enumerate() {
            let layer_w = ((k + 1) as f32 / 7.).min(1.);
            let (w, parts) = jump_parts(f);
            assert!(close(w, layer_w), "{k} {w}");
            assert!(
                close(parts[3], layer_w * (k + 1).min(5) as f32 / 10.),
                "{k}"
            );
        }
        // Full-speed walk under it, facing right.
        assert!(close(weight_of(&frames[19], ClipId::WalkFast), 1.));
        assert!(frames[30].yaw > 1.4);
    }

    /// Height (lowest to highest vertex) of the Sackboy mesh posed by
    /// `lerp(jump_small_stand, jump_stand, h)` at clip frames 1..=10, rows h = 0.1..=0.6
    /// (MEASURED on our own skinned mesh with the pose_fit example, `JUMP_TABLE=1`; D245,
    /// recomputed with the Biped scale rule in D246: the jump clips carry no body scale,
    /// so the rule moves these by 0.1 to 0.2 only).
    const JUMP_POSE_HEIGHT: [[f32; 10]; 6] = [
        [
            214.4, 217.7, 219.1, 219.1, 217.4, 214.9, 211.5, 203.9, 194.8, 189.6,
        ],
        [
            214.7, 217.8, 219.1, 218.0, 215.8, 212.9, 208.4, 200.9, 192.6, 189.0,
        ],
        [
            215.0, 218.5, 218.9, 216.8, 213.8, 210.3, 205.0, 198.2, 191.3, 187.6,
        ],
        [
            215.2, 219.2, 218.7, 215.3, 211.3, 207.5, 202.4, 195.1, 188.5, 185.3,
        ],
        [
            215.6, 219.8, 218.3, 213.7, 208.7, 205.1, 198.9, 190.9, 184.4, 181.3,
        ],
        [
            216.2, 220.4, 217.9, 211.8, 206.6, 202.0, 194.7, 186.7, 180.5, 176.0,
        ],
    ];
    /// The same for jump_stand alone (h = 1), which is what we drew before D245.
    const JUMP_STAND_HEIGHT: [f32; 10] = [
        218.4, 221.7, 215.3, 204.9, 195.9, 188.5, 178.7, 167.6, 161.4, 161.3,
    ];

    fn between(row: &[f32; 10], frame: f32) -> f32 {
        let i = (frame.floor() as usize).clamp(1, 9);
        let r = frame - i as f32;
        row[i - 1] * (1. - r) + row[i] * r
    }

    /// [`JUMP_POSE_HEIGHT`] between frames and rows.
    fn jump_pose_height(frame: f32, h: f32) -> f32 {
        let x = (h * 10. - 1.).clamp(0., 5.);
        let row = (x.floor() as usize).min(4);
        let r = x - row as f32;
        between(&JUMP_POSE_HEIGHT[row], frame) * (1. - r)
            + between(&JUMP_POSE_HEIGHT[row + 1], frame) * r
    }

    #[test]
    fn six_tick_jump_follows_the_games_mesh_height() {
        // D234 capture `jump.bin` (RPCS3, jump button held 200 ms = 6 ticks, standing, no
        // sideways drift): mesh height per tick from the take-off, one vertex buffer per
        // tick. MEASURED. Our own controller flies the same arc (feet within 1 unit of the
        // game's on every tick up to the apex at 242), so it drives the pose here.
        let game = [
            214.5, 217.9, 219.0, 216.1, 209.8, 206.9, 205.6, 204.1, 202.3, 200.4, 198.4, 196.2,
            193.8, 191.5, 189.1, 185.6, 181.6, 178.9, 176.9, 176.4,
        ];
        let lanes = |_: f32| true;
        let mut s = Sackboy::at_lane(0.);
        let mut a = SackboyAnim::new();
        let mut ours = Vec::new();
        let mut before = Vec::new();
        for t in 0..game.len() {
            s.step(
                Input {
                    jump: t < 6,
                    ..Default::default()
                },
                &Contact {
                    grounded: t == 0,
                    lane_has_floor: &lanes,
                },
            );
            let f = a.tick(&s, false);
            let (w, parts) = jump_parts(&f);
            // Standing jump: full weight, no walk clips, both stand clips on one frame.
            assert!(close(w, 1.), "tick {t}: {w}");
            assert!(parts[2] == 0. && parts[3] == 0.);
            let h = (t + 1).min(6) as f32 / 10.;
            assert!(close(parts[1], h), "tick {t}: {parts:?}");
            let frame = layer(&f, ClipId::JumpSmallStand).unwrap().frame;
            assert_eq!(layer(&f, ClipId::JumpStand).unwrap().frame, frame);
            ours.push(jump_pose_height(frame, h));
            before.push(between(&JUMP_STAND_HEIGHT, frame));
        }
        let worst = |v: &[f32]| {
            v.iter()
                .zip(&game)
                .map(|(a, b)| (a - b).abs())
                .fold(0f32, f32::max)
        };
        let rms = |v: &[f32]| {
            (v.iter()
                .zip(&game)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f32>()
                / v.len() as f32)
                .sqrt()
        };
        // Ticks 1..=19 within 2.4 (the viewer's own skinned heights: 2.3, RMS 1.4); tick 20
        // is past the table's last frame.
        assert!(worst(&ours) < 3.5, "{ours:?}");
        assert!(rms(&ours) < 1.8, "{ours:?}");
        // jump_stand alone tucks the legs early: 16 below the game on tick 10, 20 at worst.
        assert!(game[9] - before[9] > 15. && worst(&before) > 20.);
        assert!(ours[9] > 198.);
    }
}
