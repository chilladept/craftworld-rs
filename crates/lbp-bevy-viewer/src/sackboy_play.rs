//! `--play-sackboy` (docs/sim-physics.md, Part 4): with a level, the level's shapes as an
//! avian world (lbp-sim, material rules by MAP path) and Sackboy as the kinematic capsule
//! driven by the RPCS3-measured movement rules at 30 Hz. Arrow keys or the left stick walk
//! and change lanes (Up = lane behind, Down = lane in front), Space or the South button
//! jumps; the camera follows him as the game's default camera does (Moon level measurement).
//! He is drawn as his own body mesh ([`SACKBOY_MESH`], or the `--select-guid` mesh) standing
//! on the capsule (feet at its bottom), turned by the facing angle and posed from the
//! weighted clip layers `lbp_sim::sackboy_anim::SackboyAnim` gives each tick (EBOOT rules,
//! D232, docs/d231-sackboy-animations.md), blended per bone from a clip bank sharing one
//! rig. With `--play-animation` that one clip cycles at the walk speed instead (facing
//! still from the picker); without a mesh, a placeholder capsule. The level's own dynamic objects are simulated but not redrawn.
//!
//! `--play-sackboy-at X,Y,Z` sets the spawn (level units, his position: 69.9 above the feet); otherwise the
//! first CHECKPOINT's POS plus [`SPAWN_OFFSET`]. `--play-sackboy-input STEPS` replaces the pad with a script of
//! `keys:ticks` steps (keys from `l r j b f`, `-` for none; e.g. `r:40,rj:12,-:30`) and prints
//! Sackboy's position every tick, for comparing paths against RPCS3.
//!
//! Moods (D233, docs/d233-sackboy-moods-headlook.md): I / K / L / J are the mood d-pad up
//! (happy) / down (sad) / right (angry) / left (scared); in a script the capitals `U D R L`
//! hold that direction (one press per step: `U:1,-:40,U:1`).
//!
//! In the Pod (D281, docs/level-scripts.md part 32): with `--level-scripts --pod-camera N
//! --pod-shell HEX` the level he plays in is the shell level (pod.bin), not the planets
//! level given: the physics scene is pod.bin's ([`pod_scene`]), he arrives by the game's
//! checkpoint spawn rule and falls to its floor ([`checkpoint_arrival`], D282), his figure
//! hangs under the shell's frame root, the camera is the script's, and `level_play` hands
//! his place and the pad to the script worlds ([`PodSackboy`]). In a script `e` is the
//! engage button (Square) on the step's first tick. While the pod computer is in use his
//! movement input is not applied, he faces into the screen and his figure is drawn toward
//! the engage target (D282, docs/level-scripts.md part 33). D320 (part 71): the scripted Pod
//! has him without `--play-sackboy` too (`--no-pod-sackboy` leaves him out), and at the pod
//! computer he plays the game's into / idle / out clips by its rule ([`PodUserAnim`]).
use bevy::prelude::*;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::level_scene;
use lbp_preview::skinned::{sample_pose, BlendPose, BoundAnimation, ScaleRule, SkinnedClip};
use lbp_sim::avian2d::prelude::*;
use lbp_sim::character::{
    spawn_sackboy, SackboyBody, SackboyPlugin, CENTRE_ABOVE, FLOOR_REST, HALF_SEGMENT, HEIGHT,
    RADIUS,
};
use lbp_sim::sackboy::{Input, Mood, FOOT_PLANT_X, WALK_SPEED};
use lbp_sim::sackboy_anim::{AnimFrame, Blend, ClipId, SackboyAnim};
use lbp_sim::scene::{material_rule_for_path, MaterialRule};
use lbp_sim::{
    build_scene_with, build_spawn_scene_with, spawn_scene, to_level, PhysicsScene, SimPlugin,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Level units to display units, set when the level is drawn (`LevelPlan::normalize`).
#[derive(Resource, Clone, Copy)]
pub(crate) struct LevelDisplay(pub Mat4);

#[derive(Resource)]
struct Play {
    scene: Option<PhysicsScene>,
    spawn: [f32; 3],
    script: Option<Vec<(Input, usize)>>,
    /// D281: the 0-based ticks on which a script step with `e` starts ([`engage_ticks`]).
    engage: Vec<usize>,
    tick: usize,
}

/// D281: the Sackboy in the Pod, between this module and `level_play`. There only with
/// `--play-sackboy --level-scripts --pod-camera N --pod-shell HEX` ([`pod_shell`]).
#[derive(Resource, Default)]
pub(crate) struct PodSackboy {
    /// His position in the shell level's units after the last physics tick, and whether
    /// he stood on a floor then; none before he is spawned. Written in `FixedFirst`, so a
    /// script tick reads where the tick before left him.
    pub at: Option<([f32; 3], bool)>,
    /// The engage button of the input script is down on this tick; taken by
    /// `level_play::tick`.
    pub engage: bool,
    /// `--level-scripts-hold-at`: the script ticks are held (written by
    /// `level_play::tick`). His input and his pose are held with them.
    pub held: bool,
    /// His movement state after each tick that ran before his figure was loaded, for
    /// the animation picker to catch up on ([`tick_anim`]): his pose on a tick then does
    /// not depend on how long the load took.
    backlog: Vec<(lbp_sim::sackboy::Sackboy, bool)>,
    /// D282: the pod computer is in use by him (written by `level_play::tick`): his
    /// movement input is not applied ([`pod_input`]) and he faces into the screen
    /// ([`pod_facing`]).
    pub in_use: bool,
    /// D282: how far his figure is drawn toward [`PodSackboy::target`], 0..1
    /// (D320: the pod computer layer's offset blend, [`PodUserAnim`]); 0 when the layer
    /// is off.
    pub blend: f32,
    /// D282: the engage target in the shell level's units ([`engage_target`]).
    pub target: [f32; 3],
    /// D320: the pod computer layer's state, stepped once a script tick by `level_play`.
    pub anim: PodUserAnim,
    /// D320: the shell's script tick the layer was last stepped on.
    pub stepped: Option<u64>,
    /// D320: the pod computer layer on the latest script tick ([`PodUserAnim::step`]).
    pub user: Option<PodUserFrame>,
    /// D320: the frame counts of [`POD_CLIPS`]: the loaded clips' once his figure is
    /// there, before that [`POD_CLIP_FRAMES`].
    pub frames: [u32; 3],
}

/// D320: the frame counts of [`POD_CLIPS`] (MEASURED, the clip files of 01.33); the loaded
/// clips' replace them ([`PodSackboy::frames`]).
pub(crate) const POD_CLIP_FRAMES: [u32; 3] = [31, 21, 31];

/// D282: the place his figure is drawn toward at the pod computer: FOUND (01.33, the
/// Sackboy animation code for the pod computer user) a constant, whose x the game
/// replaces with the x of the Thing named "pod_controller" ([`engage_target`]).
pub(crate) const POD_ENGAGE_TARGET: [f32; 3] = [-12.469, -482.979, -93.170];

/// D282: [`POD_ENGAGE_TARGET`] with x from the "pod_controller" Thing when the level
/// has one.
pub(crate) fn engage_target(controller_x: Option<f32>) -> [f32; 3] {
    let [x, y, z] = POD_ENGAGE_TARGET;
    [controller_x.unwrap_or(x), y, z]
}

/// D282: where his figure is drawn: his physics position moved by (target - position)
/// times the blend. FOUND the offset rule (a root offset of his drawn body); INFERRED
/// that it is drawn only, since no write to his physics position was found.
///
/// D320: in depth the game draws him where this rule does not: MEASURED (his head on two
/// captured frames of the game, 1.21 times smaller than the viewer's at the target's
/// depth, with the DualShock the same size) about 300 units deeper than the target's z,
/// just in front of the DualShock (the "pod_controller" Thing, z -354 in the game's
/// memory). INFERRED: his z is [`POD_USER_LANE_Z`] while he is the user (the game's
/// memory, 2026-10-09) and the z part of the offset is applied reversed, z - (target z -
/// z) x blend: the root offset goes to his model in the axes (x, -z, y) (FOUND), and the
/// depth's sign is not established. At blend 1 that is -306.8 (the size gives about -300).
/// x and y are the FOUND rule (his x on the frames is the target's within 5 px).
pub(crate) fn engage_drawn(at: [f32; 3], target: [f32; 3], blend: f32) -> [f32; 3] {
    let z = POD_USER_LANE_Z;
    [
        at[0] + (target[0] - at[0]) * blend,
        at[1] + (target[1] - at[1]) * blend,
        z - (target[2] - z) * blend,
    ]
}

/// D320: his z while he is the pod computer's user: MEASURED -200 in the game's memory
/// with the pod computer open (2026-10-09, his x unchanged). INFERRED one lane back; what
/// moves him there is not established (no walk back is drawn: he is at this z from the
/// first tick of the pod computer layer to its last).
pub(crate) const POD_USER_LANE_Z: f32 = -200.;

/// D320: the pod computer's clips of Sackboy's animation set, in [`PodClip`] order: FOUND
/// (01.33, the Sackboy animation object's loader and the GUID table it reads, which the
/// inventory names `sack_boy_pinched_anims/pod_computer_into`, `_idle` and `_out`).
pub(crate) const POD_CLIPS: [u32; 3] = [0x42b3, 0x42b2, 0x42e9];

/// D320: which of [`POD_CLIPS`] his pod computer layer plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PodClip {
    Into,
    Idle,
    Out,
}

impl PodClip {
    pub(crate) fn index(self) -> usize {
        self as usize
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            PodClip::Into => "pod_computer_into",
            PodClip::Idle => "pod_computer_idle",
            PodClip::Out => "pod_computer_out",
        }
    }
}

/// D320: the pod computer layer on one tick: the clip and its frame, the layer's weight
/// over his own pose, and the drawn offset's blend toward [`PodSackboy::target`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PodUserFrame {
    pub clip: PodClip,
    pub frame: u32,
    pub weight: f32,
    pub offset: f32,
}

/// D320: the pod computer user's animation state, one step per script tick. FOUND (01.33,
/// the Sackboy animation code for the pod computer user; the rule, not its code):
///
/// - The tick he becomes the user the into clip starts at frame 0 and counts one frame a
///   tick; the layer's weight is (n + 1) / 3 and the drawn offset's blend (n - 5) / 11,
///   each clamped to 0..1 (n the frames played). When the clip has played out the idle
///   clip loops from frame 0, weight and blend 1.
/// - The tick he stops being the user the out clip starts at frame 0: blend 1 - (n - 5) /
///   10, weight (frames - n) / 3, clamped; when it has played out the layer is off.
/// - Not modelled: the button clips the pad's cross and circle start while holding (the
///   viewer's pad writes only the engage bit), the stick clip's layers, and a creature
///   state the into clip waits for (INFERRED: a standing Sackboy's; the viewer's stands).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PodUserAnim {
    phase: Option<PodClip>,
    n: u32,
}

impl PodUserAnim {
    /// One tick: `user` whether he is the pod computer's user on it, `frames` the frame
    /// counts of the three clips ([`PodClip`] order). None when the layer is off.
    pub(crate) fn step(&mut self, user: bool, frames: [u32; 3]) -> Option<PodUserFrame> {
        let ramp = |x: f32| x.clamp(0., 1.);
        match (user, self.phase) {
            (true, None | Some(PodClip::Out)) => {
                *self = Self {
                    phase: Some(PodClip::Into),
                    n: 0,
                }
            }
            (false, Some(PodClip::Into | PodClip::Idle)) => {
                *self = Self {
                    phase: Some(PodClip::Out),
                    n: 0,
                }
            }
            _ => {}
        }
        let n = self.n;
        let out = match self.phase? {
            PodClip::Into => {
                let offset = ramp((n as f32 - 5.) / 11.);
                let weight = ramp((n as f32 + 1.) / 3.);
                if n >= frames[0] {
                    self.phase = Some(PodClip::Idle);
                    self.n = 0;
                    PodUserFrame {
                        clip: PodClip::Idle,
                        frame: 0,
                        weight,
                        offset,
                    }
                } else {
                    PodUserFrame {
                        clip: PodClip::Into,
                        frame: n,
                        weight,
                        offset,
                    }
                }
            }
            PodClip::Idle => PodUserFrame {
                clip: PodClip::Idle,
                frame: n % frames[1].max(1),
                weight: 1.,
                offset: 1.,
            },
            PodClip::Out => {
                let offset = 1. - ramp((n as f32 - 5.) / 10.);
                let weight = ramp((frames[2] as f32 - n as f32) / 3.);
                if n >= frames[2] {
                    self.phase = None;
                }
                PodUserFrame {
                    clip: PodClip::Out,
                    frame: n.min(frames[2].saturating_sub(1)),
                    weight,
                    offset,
                }
            }
        };
        self.n += 1;
        Some(out)
    }
}

/// D282: his pad input while the pod computer is in use: none. FOUND: the game switches
/// his input context to the pod computer's; INFERRED that his walking, jumping and lane
/// changes are not applied then.
pub(crate) fn pod_input(input: Input, in_use: bool) -> Input {
    if in_use {
        Input::default()
    } else {
        input
    }
}

/// D282: the facing angle the pod computer's user turns to, into the screen (FOUND, the
/// facing function returns pi for him).
pub(crate) const POD_USER_FACING: f32 = std::f32::consts::PI;

/// D282: the facing shown for a tick: `last` the angle shown on the tick before, `picked`
/// the picker's for this tick, `turned` whether the last one was not the picker's. In use
/// it eases toward [`POD_USER_FACING`] by the picker's own rate
/// (`lbp_sim::sackboy_anim::FACING_EASE`, FOUND); afterwards back to the picker's angle at
/// the same rate (INFERRED). Returns the angle and whether it is still not the picker's.
pub(crate) fn pod_facing(last: f32, picked: f32, in_use: bool, turned: bool) -> (f32, bool) {
    use lbp_sim::sackboy_anim::FACING_EASE;
    if in_use {
        (last + (POD_USER_FACING - last) * FACING_EASE, true)
    } else if turned {
        let y = last + (picked - last) * FACING_EASE;
        if (y - picked).abs() < 1e-3 {
            (picked, false)
        } else {
            (y, true)
        }
    } else {
        (picked, false)
    }
}

/// D282: the per-player x offsets of a checkpoint spawn by spawn slot (FOUND, 01.33): the
/// first player takes slot 0.
pub(crate) const SPAWN_SLOT_X: [f32; 5] = [0., 80., -40., 40., -80.];

/// D282: how far in front of the checkpoint (world z) a Sackboy spawns (FOUND, 01.33).
pub(crate) const SPAWN_Z_FORWARD: f32 = 50.;

/// D282: where a checkpoint puts a Sackboy, FOUND (01.33): the checkpoint's world
/// translation, z the larger of the checkpoint's z and a box's z (`box_z`; INFERRED a box
/// of the checkpoint Thing, not identified, so the viewer passes none and the checkpoint's
/// z is taken) then 50 forward, x moved by the slot's offset ([`SPAWN_SLOT_X`]). No
/// height offset: he appears at the checkpoint's height (INFERRED: and falls, no floor
/// search was seen).
pub(crate) fn checkpoint_arrival(m: &[f32; 16], box_z: Option<f32>, slot: usize) -> [f32; 3] {
    let z = box_z.map_or(m[14], |b| m[14].max(b)) + SPAWN_Z_FORWARD;
    [m[12] + SPAWN_SLOT_X[slot % SPAWN_SLOT_X.len()], m[13], z]
}

/// D281: the shell level's GUID when the Sackboy plays in the Pod: `--play-sackboy` with
/// `--level-scripts`, `--pod-camera N` and `--pod-shell HEX` (the condition under which
/// `level_play` has the shell level in its session, plus the play mode).
///
/// D320: the scripted Pod has him without `--play-sackboy` too (the game's Pod always has
/// its player); `--no-pod-sackboy` leaves him out there (part 30's undrawn record).
pub(crate) fn pod_shell(all: &[String]) -> Result<Option<u32>, String> {
    let has = |flag: &str| all.iter().any(|a| a == flag);
    let wanted = has("--play-sackboy") || !has("--no-pod-sackboy");
    if !(wanted && has("--level-scripts") && has("--pod-camera")) {
        return Ok(None);
    }
    value(all, "--pod-shell")
        .map(|g| u32::from_str_radix(&g, 16).map_err(|_| "--pod-shell requires hex digits".into()))
        .transpose()
}

/// True when the Sackboy plays in the Pod ([`pod_shell`]).
pub(crate) fn in_pod() -> bool {
    let all: Vec<String> = std::env::args().collect();
    matches!(pod_shell(&all), Ok(Some(_)))
}

/// Where the game has Sackboy standing in the Pod on arrival: (x, y) of his position, in
/// lane 0. MEASURED (RPCS3 memory read, 2026-10-09, one arrival; after a walk right he
/// stood at (338.10, -475.53)). Only printed beside the viewer's own start, never used.
pub(crate) const POD_ARRIVAL_MEASURED: [f32; 2] = [-5.37, -477.97];

/// D281: the physics scene of the Pod's shell level.
///
/// - Shapes whose outline has depth under a tilting matrix are projected through the
///   whole matrix (`build_spawn_scene_with`). pod.bin's pod Thing is such a shape: its
///   matrix lays the outline's z along the level's y, so its in-plane outline is a thin
///   slab at y -167 to -152, and projected it is the pod's cross-section from y -1525 to
///   1282 with the floor inside. MEASURED: that floor is at y -546.8 under x -4.2 and
///   -544.3 under x 338; with his rest height 69.9 he stands at -476.9 and -474.4, where
///   the game has him at -477.97 and -475.53 ([`POD_ARRIVAL_MEASURED`]). So the game's
///   floor is pod.bin's own collision, 1.1 off; no height is fitted.
/// - Every body is held where the file has it. PLACEHOLDER: the pod hangs on two strings
///   and is a moving body in the game (MEASURED 3.6 units from its stored x); the viewer
///   does not draw the shell from the physics, so a pod that swung would leave him
///   standing beside the picture. The two string joints are left out with that.
pub(crate) fn pod_scene(
    things: &[lbp_level::thing::ThingRecord],
    rule: &dyn Fn(&lbp_level::serial::Descriptor) -> MaterialRule,
) -> PhysicsScene {
    let mut scene = build_spawn_scene_with(things, rule);
    for body in &mut scene.bodies {
        body.fixed = true;
    }
    // Between held bodies a joint has nothing to do (and the solver refuses one).
    scene.joints.clear();
    scene
}

/// The 0-based ticks on which a step of an input script that holds `e` (the engage
/// button) starts: the button is down on that tick alone, a click.
fn engage_ticks(text: &str) -> Vec<usize> {
    let mut at = 0;
    let mut out = Vec::new();
    for step in text.split(',') {
        let Some((keys, ticks)) = step.split_once(':') else {
            continue;
        };
        if keys.contains('e') {
            out.push(at);
        }
        at += ticks.trim().parse::<usize>().unwrap_or(0);
    }
    out
}

#[derive(Component)]
pub(crate) struct Marker;

/// The Sackboy mesh standing on the capsule: `upright` turns the raw Z-up model to the
/// display (feet at the origin, level-unit scale); `facing` is the turn about the display Y
/// axis, eased between the picker's last two ticks (`yaw`); `phase` the `--play-animation`
/// clip's frame. With the clip bank (`banked`), stand is `Playing`'s own clip and `bank`
/// holds the others by [`ClipId::index`] (None for stand or a clip that failed to load);
/// `anim` is the picker's output for the latest tick, `picked`/`posed` count the ticks
/// picked and blended.
#[derive(Component)]
pub(crate) struct Figure {
    upright: Mat4,
    facing: f32,
    phase: f32,
    banked: bool,
    bank: Vec<Option<BoundAnimation>>,
    picker: SackboyAnim,
    anim: Option<AnimFrame>,
    yaw: [f32; 2],
    picked: u64,
    posed: u64,
    /// D281: level units to the space of the entity the figure hangs under (the Pod's
    /// shell root and the shell level's display matrix); none for a figure without a
    /// parent, which is placed by [`LevelDisplay`].
    display: Option<Mat4>,
    /// D282: the shown facing is not the picker's (turned to the pod computer, or on the
    /// way back from it; [`pod_facing`]).
    turned: bool,
    /// D320: [`POD_CLIPS`] bound to the rig, in [`PodClip`] order (None for a clip that
    /// failed to load).
    pod_bank: Vec<Option<BoundAnimation>>,
    /// D320: the pod computer layer the shown pose was blended with.
    pod_posed: Option<PodUserFrame>,
}

/// Sackboy's body mesh, the play figure when `--select-guid` is not given: `0000043f`
/// `gamedata/mesh_library/player_character/sack_boy.mol` (FOUND, MAP and the EBOOT GUID
/// block). The default "naked" costume pieces add no geometry (INFERRED; plan section 1).
pub(crate) const SACKBOY_MESH: &str = "0000043f";

/// Reads the mesh once and binds every [`ClipId::ALL`] clip to its rig; the first
/// (stand) is the playing clip, the rest come back in order, then (D320) the three
/// [`POD_CLIPS`]. Runs on the load worker.
pub(crate) fn load_bank(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    mesh_key: &lbp_catalog::attachment::LocationKey,
    control: &LoadControl,
) -> Result<(SkinnedClip, Vec<Result<BoundAnimation, String>>), String> {
    // D320: the pod computer's clips follow the picker's ([`POD_CLIPS`]).
    let guids: Vec<u32> = ClipId::ALL
        .map(ClipId::guid)
        .into_iter()
        .chain(POD_CLIPS)
        .collect();
    let (mut clip, mut bank) =
        lbp_preview::skinned::load_clip_bank(source, catalog, mesh_key, &guids, control)?;
    // D246: Sackboy's Biped bones do not pass their scale on (the stand clip breathes by
    // scaling the spine; inherited, that stretched the whole upper body).
    clip.rig.scale_rule = ScaleRule::Biped;
    for bound in bank.iter_mut().flatten() {
        bound.rig.scale_rule = ScaleRule::Biped;
    }
    Ok((clip, bank))
}

/// True when the viewer was started with `--play-sackboy`, or (D320) when the scripted Pod
/// has its Sackboy ([`in_pod`]).
pub(crate) fn enabled() -> bool {
    std::env::args().any(|a| a == "--play-sackboy") || in_pod()
}

/// The pose for one tick's layers: from bind, each lerp layer moves the bones it drives
/// toward its clip's (sub-frame) pose by its weight, each additive layer adds its frame
/// minus its reference frame. Stand is the playing clip; a clip missing from the bank is
/// skipped. A layer with the game's mirror flag is mirrored left/right through the
/// skeleton's mirror table (moving left: walk, jump_walk, stop/turn). Last, `squash` raises
/// both ankles toward the pelvis by that much (the short legs after a landing, D249) and
/// `plant` (0..1) moves each Foot joint that far onto its standing spot, [`FOOT_PLANT_X`] to
/// its side of the body (D256).
fn blend_frame(
    stand: &SkinnedClip,
    bank: &[Option<BoundAnimation>],
    frame: &AnimFrame,
    squash: f32,
    plant: f32,
    pod: Option<(&BoundAnimation, f32, f32)>,
) -> BlendPose {
    let mut pose = BlendPose::bind(&stand.rig);
    // The game's upper-body mask: Spine1 and below, without the head and what hangs on it.
    let upper = stand.rig.bone_named("Bip01 Spine1").map(|spine1| {
        stand
            .rig
            .subtree_mask(spine1, stand.rig.bone_named("Bip01 Head"))
    });
    for l in &frame.layers {
        let (rig, animation) = if l.clip == ClipId::Stand {
            (&stand.rig, &stand.animation)
        } else {
            match bank.get(l.clip.index()) {
                Some(Some(b)) => (&b.rig, &b.animation),
                _ => continue,
            }
        };
        let Some(p) = sample_pose(animation, l.frame) else {
            continue;
        };
        match l.blend {
            Blend::Lerp => {
                if l.mirror {
                    pose.lerp_mirrored(rig, &p, l.weight);
                } else {
                    pose.lerp(rig, &p, l.weight);
                }
            }
            Blend::Additive { reference } => {
                if let Some(r) = sample_pose(animation, reference) {
                    pose.add(rig, &p, &r, l.weight);
                }
            }
            Blend::UpperBody => {
                if let Some(mask) = &upper {
                    pose.lerp_masked(rig, &p, l.weight, mask);
                }
            }
        }
    }
    // D320: the pod computer layer (clip, frame, weight) over his own pose, the whole body.
    if let Some((b, f, w)) = pod {
        if let Some(p) = sample_pose(&b.animation, f) {
            pose.lerp(&b.rig, &p, w);
        }
    }
    // The clips' root rotation is the facing channel the game overwrites (stand 0, walk
    // and idle-chain clips +90 degrees): kept, it doubled the picker's yaw (walking right
    // showed his back) and snapped 90 degrees wherever stand met another clip.
    pose.hold_root_rotation(&stand.rig);
    if squash != 0. || plant != 0. {
        let worlds = stand.rig.worlds_from_trs(&pose.bones);
        // Raw model space: his left at +X, depth Y, Z up.
        for (side, x) in [("L", FOOT_PLANT_X), ("R", -FOOT_PLANT_X)] {
            let bone = |part: &str| stand.rig.bone_named(&format!("Bip01 {side} {part}"));
            if let (Some(thigh), Some(calf), Some(foot)) =
                (bone("Thigh"), bone("Calf"), bone("Foot"))
            {
                // The standing spot is on the body's depth line (MEASURED against the
                // game's standing feet); the joint's height stays the clip's.
                let to = [
                    plant * (x - worlds[foot][12]),
                    plant * -worlds[foot][13],
                    squash,
                ];
                pose.move_chain_end(&stand.rig, [thigh, calf, foot], to);
            }
        }
    }
    pose
}

/// Spawns the Sackboy mesh (`--select-guid`) as the play figure and, with a clip, its rig
/// and playback (frames chosen by [`show_sackboy`]). Model units are level units (as for
/// the Pod figure), so `k` is the level display scale. D281: with `under` the figure is a
/// child of that entity and is placed by that matrix (level units to the parent's space;
/// `k` is then its scale).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_figure(
    v: &mut crate::Viewer,
    (commands, meshes, materials, images): (
        &mut Commands,
        &mut Assets<Mesh>,
        &mut Assets<crate::ViewerMaterial>,
        &mut Assets<Image>,
    ),
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    k: f32,
    preview: &lbp_preview::GroupedMesh,
    (clip, bank): (
        Option<crate::ClipResult>,
        Option<Vec<Result<BoundAnimation, String>>>,
    ),
    under: Option<(Entity, Mat4)>,
) -> Result<String, String> {
    let [lo, hi] =
        crate::adapter_bounds(&preview.geometry.positions).ok_or("Unsupported: empty figure")?;
    // Raw Z-up model: feet are the lowest z, centred in x and y (depth).
    let origin = Mat4::from_translation(Vec3::new(
        -(lo[0] + hi[0]) / 2.,
        -(lo[1] + hi[1]) / 2.,
        -lo[2],
    ));
    let upright =
        Mat4::from_scale(Vec3::splat(k)) * Mat4::from_quat(crate::model_up(true)) * origin;
    // Clip bank: stand is the playing clip; the others are blended in by ClipId index.
    let banked = bank.is_some() && matches!(clip, Some(Ok(_)));
    let mut slots: Vec<Option<BoundAnimation>> = ClipId::ALL.iter().map(|_| None).collect();
    let mut pod_slots: Vec<Option<BoundAnimation>> = POD_CLIPS.iter().map(|_| None).collect();
    let mut bank_line = String::new();
    if let (true, Some(mut bank)) = (banked, bank) {
        let mut missing = Vec::new();
        let mut odd = Vec::new();
        // D320: the pod computer's clips come after the picker's.
        let pod = bank.split_off((ClipId::ALL.len() - 1).min(bank.len()));
        let mut pod_line = Vec::new();
        for ((slot, bound), name) in pod_slots
            .iter_mut()
            .zip(pod)
            .zip([PodClip::Into, PodClip::Idle, PodClip::Out].map(PodClip::name))
        {
            match bound {
                Ok(b) => {
                    pod_line.push(format!(
                        "{name} {} frames at {} fps",
                        b.report.frames, b.report.fps
                    ));
                    *slot = Some(b);
                }
                Err(e) => pod_line.push(format!("{name} not loaded: {e}")),
            }
        }
        if !pod_line.is_empty() {
            bank_line += &format!("; pod computer clips: {}", pod_line.join(", "));
        }
        for (id, bound) in ClipId::ALL[1..].iter().zip(bank) {
            match bound {
                Ok(b) => {
                    if b.report.frames != id.frames() {
                        odd.push(format!("{} {} frames", id.name(), b.report.frames));
                    }
                    slots[id.index()] = Some(b);
                }
                Err(e) => missing.push(format!("{} {e}", id.name())),
            }
        }
        bank_line = format!(
            "; clip bank {}/{} bound to one rig, blended by the EBOOT layer rules{bank_line}",
            ClipId::ALL.len() - missing.len(),
            ClipId::ALL.len()
        );
        if !missing.is_empty() {
            bank_line += &format!("; not loaded: {}", missing.join(", "));
        }
        if !odd.is_empty() {
            bank_line += &format!("; frame counts differ from the census: {}", odd.join(", "));
        }
    }
    let figure = commands
        .spawn((
            Transform::from_matrix(upright),
            Visibility::default(),
            Figure {
                upright,
                // Standing faces the camera (FOUND, rules note section 2).
                facing: 0.,
                phase: 0.,
                banked,
                bank: slots,
                picker: SackboyAnim::new(),
                anim: None,
                yaw: [0.; 2],
                picked: 0,
                posed: 0,
                display: under.map(|u| u.1),
                turned: false,
                pod_bank: pod_slots,
                pod_posed: None,
            },
        ))
        .id();
    if let Some((parent, _)) = under {
        commands.entity(parent).add_child(figure);
    }
    let mut evidence = crate::browser::PreviewEvidence::default();
    let mut details = Vec::new();
    let first = v.scene.len();
    for p in crate::prepare(preview, false, &mut evidence, &mut details)? {
        let (mesh, material, image) = crate::upload(p, meshes, materials, images);
        let entity = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                ChildOf(figure),
            ))
            .id();
        v.scene.push((entity, mesh, material, image));
    }
    let mut line = format!(
        "Play figure: model height {:.0} level units (capsule {:.0})",
        hi[2] - lo[2],
        HEIGHT
    );
    let scene: Vec<_> = v.scene[first..]
        .iter()
        .map(|s| (s.0, s.1.clone()))
        .collect();
    match clip {
        None => line += "; no clip",
        Some(Err(e)) => line += &format!("; clip not played: {e}"),
        Some(Ok(clip)) => {
            // Raw model units: the identity display frame.
            let frame = ([0.; 3], 1.);
            let gpu = if v.cpu_skinning {
                None
            } else {
                let rig = crate::gpu_rig(
                    commands,
                    meshes,
                    bindposes,
                    &clip.rig,
                    (&clip.skin.joints, &clip.skin.weights),
                    (frame, Quat::IDENTITY),
                    &scene,
                )?;
                commands.entity(rig.root).insert(ChildOf(figure));
                Some(crate::GpuRig {
                    morphs: !clip.skin.morphs.is_empty() && clip.report.animation_morphs > 0,
                    ..rig
                })
            };
            line += &format!(
                "; {} clip {} frames at {} fps{bank_line}",
                if banked { "stand" } else { "--play-animation" },
                clip.report.frames,
                clip.report.fps
            );
            v.animation_frame = Some(0);
            v.playing = Some(crate::Playing {
                clip,
                frame,
                started: -1.,
                shown: None,
                gpu,
                meshes: scene.into_iter().map(|s| s.1).collect(),
                alt: None,
                blended: None,
                layered: banked,
            });
        }
    }
    Ok(line)
}

/// The walk clip's next frame position: advances at the clip rate scaled by walking speed
/// (full rate at [`WALK_SPEED`]) while grounded and moving; otherwise back to frame 0.
/// INFERRED pacing (RPCS3 stride timing to check).
fn walk_phase(phase: f32, dt: f32, fps: f32, vx: f32, grounded: bool) -> f32 {
    let speed = (vx.abs() / WALK_SPEED).min(1.);
    if grounded && speed > 0.05 {
        phase + dt * fps * speed
    } else {
        0.
    }
}

/// Sackboy's start relative to the first CHECKPOINT POS. RPCS3, Rookie Test (PC, d-run
/// 2026-10-09): checkpoint (-13335, 6971, -250), Sackboy appears at (-13335.7, 6763, 0) and
/// lands at y 5974. One level only: INFERRED for other levels.
pub(crate) const SPAWN_OFFSET: [f32; 3] = [0., -208., 250.];

/// Sackboy's x relative to the first CHECKPOINT POS when he spawns there; he appears at the
/// checkpoint's height and falls to the floor below. RPCS3, Moon level play-mode spawn (PC,
/// 2026-10-09, local-analysis/results/2026-10-09-moon-movement.md, "Addendum: play-mode
/// checkpoint spawn"): checkpoint (-18904.74, 1798.11, -470), first sample at +(-0.36,
/// +0.07), settles at +(-1.11, -73.82) after 23 ticks. The Pod (shell 000027b9,
/// local-analysis/results/2026-10-09-pod-spawn-memory.md): checkpoint (-3.05, 540.76, -50),
/// Sackboy at (-5.37, -477.97) in lane 0, 1018 below it, so only x and the drop carry over,
/// not the Moon's y. INFERRED for other levels.
pub(crate) const DATA_SPAWN_X: f32 = -1.11;

/// Lane centres (world z): front 0, middle -200, back -400 (Moon level, same results file).
const LANES: [f32; 3] = [0., -200., -400.];

/// The lane Sackboy spawns in for a checkpoint at depth `z`: the nearest lane centre. Moon:
/// checkpoint z -470 -> back lane -400. INFERRED (one checkpoint; Rookie Test's z 0 from a
/// -250 checkpoint does not fit this rule and was not taken in play mode).
pub(crate) fn spawn_lane(z: f32) -> f32 {
    LANES
        .into_iter()
        .min_by(|a, b| (a - z).abs().total_cmp(&(b - z).abs()))
        .unwrap_or(0.)
}

/// The default game camera's eye relative to Sackboy's position (level units), looking
/// straight down -z (no pitch), vertical FOV [`FOLLOW_FOV_DEG`]. RPCS3 memory read in the
/// owner's blank Moon level (PC run d208, idle, one pose): CONFIRMED_FROM_LOCAL_DATA
/// (emulated) for that pose; whether the x lead follows his facing or lags while walking is
/// not measured (INFERRED: held constant). The runtime keeps the value, for the camera it
/// tells the level scripts of when nothing draws.
const FOLLOW_EYE: [f32; 3] = lbp_runtime::FOLLOW_EYE;
const FOLLOW_FOV_DEG: f32 = 60.;

fn value(all: &[String], flag: &str) -> Option<String> {
    all.iter()
        .position(|a| a == flag)
        .and_then(|i| all.get(i + 1))
        .cloned()
}

fn parse_script(text: &str) -> Result<Vec<(Input, usize)>, String> {
    text.split(',')
        .map(|step| {
            let (keys, ticks) = step
                .split_once(':')
                .ok_or_else(|| format!("--play-sackboy-input: {step:?} needs keys:ticks"))?;
            let ticks = ticks
                .trim()
                .parse::<usize>()
                .map_err(|e| format!("--play-sackboy-input: {step:?}: {e}"))?;
            let mut input = Input::default();
            for c in keys.trim().chars() {
                match c {
                    'l' => input.x = -1.,
                    'r' => input.x = lbp_sim::sackboy::STICK_FULL_RIGHT,
                    'j' => input.jump = true,
                    'b' => input.back = true,
                    'f' => input.front = true,
                    // D281: the engage button; not a movement key ([`engage_ticks`]).
                    'e' => {}
                    'U' => input.mood = Some(Mood::Happy),
                    'D' => input.mood = Some(Mood::Sad),
                    'R' => input.mood = Some(Mood::Angry),
                    'L' => input.mood = Some(Mood::Scared),
                    '-' => {}
                    _ => return Err(format!("--play-sackboy-input: unknown key {c:?}")),
                }
            }
            Ok((input, ticks))
        })
        .collect()
}

/// Non-normal material rules by descriptor GUID, from the MAP paths.
fn material_rules(catalog: &lbp_catalog::Catalog) -> BTreeMap<u32, MaterialRule> {
    let mut rules = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
            let rule = material_rule_for_path(&r.map_row.path);
            if rule != MaterialRule::Normal {
                rules.entry(g).or_insert(rule);
            }
        }
    }
    rules
}

/// Where a level puts Sackboy's feet, from its data alone: the first CHECKPOINT's POS
/// moved by [`DATA_SPAWN_X`] into the [`spawn_lane`] lane, dropped from the checkpoint's
/// height to the highest collider top below in that slab (the Pod figure, in place of the
/// screenshot fit). Pod: feet at y -547, i.e. the measured position row -478 less the 69
/// from feet to Sackboy's position (`lbp_sim::character`). Level units; and a line for the status. Runs on the
/// level worker (deep reader stack).
pub(crate) fn level_spawn_feet(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    key: &lbp_catalog::attachment::LocationKey,
    control: &LoadControl,
) -> Result<([f32; 3], String), String> {
    let bytes = level_scene::read_level_bytes(source, key, control)?;
    let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let (thing, m) = *level_scene::checkpoints(&report.things)
        .first()
        .ok_or("no CHECKPOINT")?;
    let rules = material_rules(catalog);
    let scene = build_spawn_scene_with(&report.things, &|d| {
        d.guid
            .and_then(|g| rules.get(&g).copied())
            .unwrap_or_default()
    });
    data_spawn(&scene, thing, &m)
}

/// [`level_spawn_feet`] for a scene and a checkpoint (Thing, world matrix).
fn data_spawn(
    scene: &PhysicsScene,
    thing: impl std::fmt::Display,
    m: &[f32; 16],
) -> Result<([f32; 3], String), String> {
    let [x, y, z] = [m[12] + DATA_SPAWN_X, m[13], spawn_lane(m[14])];
    let floor = scene
        .floor_below([x, y], z)
        .ok_or_else(|| format!("no floor below ({x:.0}, {y:.0}) in slab z {z:.0}"))?;
    Ok((
        [x, floor, z],
        format!(
            "data spawn: checkpoint Thing {thing} ({:.0}, {:.0}, {:.0}) x {DATA_SPAWN_X:+.2}, dropped in lane z {z:.0} to floor y {floor:.0}",
            m[12], m[13], m[14]
        ),
    ))
}

/// Reads the level and adds physics and Sackboy when `--play-sackboy` is given.
pub fn install(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    if !enabled() {
        return Ok(());
    }
    let snapshot = value(&all, "--snapshot").ok_or("--play-sackboy requires --snapshot")?;
    let game = value(&all, "--game-dir").ok_or("--play-sackboy requires --game-dir")?;
    let guid = value(&all, "--level-guid")
        .map(|g| u32::from_str_radix(&g, 16).map_err(|_| "--level-guid requires hex digits"))
        .transpose()?;
    let path = value(&all, "--level-path");
    if guid.is_none() && path.is_none() {
        return Err("--play-sackboy requires --level-guid or --level-path".into());
    }
    let at = value(&all, "--play-sackboy-at")
        .map(|v| {
            let p: Vec<f32> = v
                .split(',')
                .map(|x| x.trim().parse::<f32>())
                .collect::<Result<_, _>>()
                .map_err(|e| format!("--play-sackboy-at: {e}"))?;
            <[f32; 3]>::try_from(p).map_err(|_| "--play-sackboy-at needs X,Y,Z".to_string())
        })
        .transpose()?;
    let script = value(&all, "--play-sackboy-input")
        .map(|s| parse_script(&s))
        .transpose()?;
    let engage = value(&all, "--play-sackboy-input").map_or(Vec::new(), |s| engage_ticks(&s));
    // D281: in the Pod he plays in the shell level, not in the planets level given.
    let pod = pod_shell(&all)?;
    let (guid, path) = match pod {
        Some(shell) => (Some(shell), None),
        None => (guid, path),
    };
    if pod.is_none() && !engage.is_empty() {
        println!(
            "--play-sackboy-input: `e` (the engage button) is only read in the Pod (--level-scripts --pod-camera N --pod-shell HEX)"
        );
    }
    // The level reader recurses deeply: same worker stack as the level scene.
    let (scene, checkpoints, header, popit, controller) = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<_, String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(Path::new(&snapshot), &control)?;
            let source = AttachedSource::attach(Path::new(&game), &catalog, &control)?;
            let rules = material_rules(&catalog);
            let (key, identity) = level_scene::resolve_level(&catalog, guid, path.as_deref())?;
            let bytes = level_scene::read_level_bytes(&source, &key, &control)?;
            let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
                .map_err(|e| e.to_string())?;
            let rule = |d: &lbp_level::serial::Descriptor| {
                d.guid
                    .and_then(|g| rules.get(&g).copied())
                    .unwrap_or_default()
            };
            let scene = match pod {
                Some(_) => pod_scene(&report.things, &rule),
                None => build_scene_with(&report.things, &rule),
            };
            let r = &scene.report;
            let header = format!(
                "Play Sackboy: {identity}; shapes {}: dynamic {}, static {}, not collidable {}, rejected {}; joints {}",
                r.shapes,
                r.dynamic,
                r.static_,
                r.not_collidable,
                r.rejected.values().sum::<usize>(),
                r.joints
            );
            // D281: no Popit in the Pod (its Square is the pod computer's engage button).
            let popit = match pod {
                Some(_) => None,
                None => Some(crate::popit_play::load_assets(&source, &catalog, &control)?),
            };
            // D282: the pod computer's engage target takes its x from this Thing.
            let controller = report.things.iter().enumerate().find_map(|(i, t)| {
                let named = t.data.script_name.as_ref()?.name.as_slice() == b"pod_controller";
                named.then(|| (i, t.data.pos.as_ref().map(|p| p.world[12])))
            });
            Ok((
                scene,
                level_scene::checkpoints(&report.things),
                header,
                popit,
                controller,
            ))
        })
        .map_err(|e| format!("play sackboy thread: {e}"))?
        .join()
        .map_err(|_| "play sackboy thread panicked".to_string())??;
    let mut pod_line = None;
    let spawn = match (at, checkpoints.first()) {
        (Some(p), _) => p,
        // D282: in the Pod he arrives by the game's checkpoint spawn rule (FOUND, 01.33):
        // the shell level's first checkpoint, 50 forward in z, slot 0, at the
        // checkpoint's height; he falls to the floor from there.
        (None, Some((thing, m))) if pod.is_some() => {
            let p = checkpoint_arrival(m, None, 0);
            let floor = scene.floor_below([p[0], p[1]], p[2]).map_or(
                "no floor below in that lane".into(),
                |f| {
                    format!(
                        "the floor below is y {f:.1} (pod.bin's collision), where he would stand at y {:.2}",
                        f + FLOOR_REST
                    )
                },
            );
            pod_line = Some(format!(
                "Pod Sackboy start: checkpoint spawn (FOUND, 01.33): checkpoint Thing {thing} ({:.2}, {:.2}, {:.0}), z max(checkpoint z, box z) + {SPAWN_Z_FORWARD} with the box not identified (INFERRED: the checkpoint's z), slot 0 x {:+.0}, no height offset: he starts at ({:.2}, {:.2}, {:.0}), lane {:.0}, and falls; {floor}; MEASURED in the game on arrival: ({}, {}), lane 0",
                m[12], m[13], m[14], SPAWN_SLOT_X[0], p[0], p[1], p[2], p[2],
                POD_ARRIVAL_MEASURED[0], POD_ARRIVAL_MEASURED[1]
            ));
            p
        }
        (None, Some((_, m))) => {
            let mut p = [
                m[12] + SPAWN_OFFSET[0],
                m[13] + SPAWN_OFFSET[1],
                m[14] + SPAWN_OFFSET[2],
            ];
            // The Rookie Test offset drops him 208 below the checkpoint; where the floor
            // under the checkpoint is higher than that (blank level: top 1655, his feet at
            // 1520), he would start inside it (no lane changes, drawn sunk into the floor):
            // stand him on it instead.
            if let Some(top) = scene.floor_below([p[0], m[13]], p[2]) {
                p[1] = p[1].max(top + FLOOR_REST);
            }
            p
        }
        (None, None) => return Err("--play-sackboy: no CHECKPOINT; give --play-sackboy-at".into()),
    };
    let points: Vec<String> = checkpoints
        .iter()
        .map(|(t, m)| format!("Thing {t} ({:.0}, {:.0}, {:.0})", m[12], m[13], m[14]))
        .collect();
    println!(
        "{header}; checkpoints [{}]; spawn ({:.0}, {:.0}, {:.0})",
        points.join(", "),
        spawn[0],
        spawn[1],
        spawn[2]
    );
    if let Some(line) = pod_line {
        println!("{line}");
    }
    if pod.is_some() {
        println!(
            "Pod Sackboy: plays in the shell level (pod.bin), every body held where the file has it (placeholder); keys: arrows walk and change lanes, Space jumps, Z or gamepad Square is the engage button; no Popit"
        );
        let target = engage_target(controller.and_then(|c| c.1));
        println!(
            "Pod Sackboy engage target: ({:.3}, {:.3}, {:.3}): x from {} (the game's constant has {:.3}), y and z the game's constant (FOUND, 01.33); his figure is drawn toward it while the pod computer is in use",
            target[0],
            target[1],
            target[2],
            match controller {
                Some((i, Some(_))) => format!("Thing {i} \"pod_controller\""),
                Some((i, None)) => format!("the constant: Thing {i} \"pod_controller\" has no place"),
                None => "the constant: no Thing \"pod_controller\"".into(),
            },
            POD_ENGAGE_TARGET[0]
        );
        app.insert_resource(PodSackboy {
            target,
            frames: POD_CLIP_FRAMES,
            ..default()
        })
        .add_systems(FixedFirst, pod_report);
    }
    if let Some(popit) = popit {
        crate::popit_play::install(app, popit);
    }
    app.add_plugins((SimPlugin::default(), SackboyPlugin))
        .insert_resource(Play {
            scene: Some(scene),
            spawn,
            script,
            engage,
            tick: 0,
        })
        .add_systems(Update, (spawn_world, read_pad, show_sackboy).chain())
        .add_systems(FixedFirst, script_input)
        .add_systems(FixedLast, (tick_anim, log_scripted).chain());
    if let Some(spec) = value(&all, "--qa-record") {
        let parts: Vec<&str> = spec.split(',').collect();
        let num = |i: usize, d: f32| -> Result<f32, String> {
            parts.get(i).map_or(Ok(d), |s| {
                s.trim()
                    .parse::<f32>()
                    .map_err(|e| format!("--qa-record: {e}"))
            })
        };
        let dir = std::path::PathBuf::from(parts[0]);
        if !dir.starts_with("local-analysis") || dir.exists() {
            return Err("--qa-record DIR must be a new directory under local-analysis".into());
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        app.insert_resource(Record {
            dir,
            start: num(1, 1300.)? as usize,
            end: num(2, 1400.)? as usize,
            speed: num(3, 0.25)?,
            last: 0,
        })
        .add_systems(Update, record.after(show_sackboy));
    }
    Ok(())
}

/// `--qa-record DIR,START,END[,SPEED]` (QA only): one screenshot per scripted tick in
/// START..=END into DIR/tick_NNNN.png, game time slowed to SPEED meanwhile so every tick
/// is drawn, then exit.
#[derive(Resource)]
struct Record {
    dir: std::path::PathBuf,
    start: usize,
    end: usize,
    speed: f32,
    last: usize,
}

fn record(
    mut commands: Commands,
    play: Res<Play>,
    mut rec: ResMut<Record>,
    mut time: ResMut<Time<Virtual>>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let t = play.tick;
    if t + 30 >= rec.start && t <= rec.end {
        time.set_relative_speed(rec.speed);
    }
    if t > rec.end + 10 {
        exit.write(AppExit::Success);
        return;
    }
    if t >= rec.start && t <= rec.end && t != rec.last {
        rec.last = t;
        let path = rec.dir.join(format!("tick_{t:04}.png"));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

fn spawn_world(mut commands: Commands, mut play: ResMut<Play>) {
    let Some(scene) = play.scene.take() else {
        return;
    };
    spawn_scene(&mut commands, &scene);
    let [x, y, z] = play.spawn;
    spawn_sackboy(&mut commands, [x, y], z);
}

fn read_pad(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    play: Res<Play>,
    mut sackboy: Query<&mut SackboyBody>,
    pod: Option<Res<PodSackboy>>,
) {
    if play.script.is_some() {
        return;
    }
    // D282: at the pod computer his movement input is not applied.
    let in_use = pod.is_some_and(|p| p.in_use);
    let pad = pads.iter().next();
    let held = |k: KeyCode, b: GamepadButton| keys.pressed(k) || pad.is_some_and(|p| p.pressed(b));
    // A pad's stick is a signed byte over 128 in the game: full right stops short of 1.
    let mut x = pad.map_or(0., |p| {
        p.left_stick()
            .x
            .clamp(-1., lbp_sim::sackboy::STICK_FULL_RIGHT)
    });
    if x.abs() < 0.3 {
        x = 0.;
    }
    if held(KeyCode::ArrowLeft, GamepadButton::DPadLeft) {
        x = -1.;
    }
    if held(KeyCode::ArrowRight, GamepadButton::DPadRight) {
        x = lbp_sim::sackboy::STICK_FULL_RIGHT;
    }
    // Moods (D233): I/K/L/J as the mood d-pad up/down/right/left. The pad's own d-pad
    // already walks and changes lanes here, so it is not the mood pad.
    let mood = [
        (KeyCode::KeyI, Mood::Happy),
        (KeyCode::KeyK, Mood::Sad),
        (KeyCode::KeyL, Mood::Angry),
        (KeyCode::KeyJ, Mood::Scared),
    ]
    .into_iter()
    .find_map(|(k, m)| keys.pressed(k).then_some(m));
    let input = pod_input(
        Input {
            x,
            jump: held(KeyCode::Space, GamepadButton::South),
            back: held(KeyCode::ArrowUp, GamepadButton::DPadUp),
            front: held(KeyCode::ArrowDown, GamepadButton::DPadDown),
            mood,
        },
        in_use,
    );
    for mut body in &mut sackboy {
        body.input = input;
    }
}

/// Scripted input for this tick (none once the script ends). D281, in the Pod: none
/// while the script ticks are held, and the engage button on the first tick of a step
/// with `e`.
fn script_input(
    play: Res<Play>,
    mut sackboy: Query<&mut SackboyBody>,
    mut pod: Option<ResMut<PodSackboy>>,
) {
    let Some(script) = &play.script else {
        return;
    };
    let held = pod.as_ref().is_some_and(|p| p.held);
    let mut left = play.tick;
    let input = script
        .iter()
        .find_map(|(input, ticks)| {
            if left < *ticks {
                Some(*input)
            } else {
                left -= ticks;
                None
            }
        })
        .filter(|_| !held);
    if let Some(pod) = pod.as_mut() {
        if !held && !sackboy.is_empty() && play.engage.contains(&play.tick) {
            pod.engage = true;
        }
    }
    // D282: at the pod computer his movement input is not applied.
    let in_use = pod.as_ref().is_some_and(|p| p.in_use);
    for mut body in &mut sackboy {
        body.input = pod_input(input.unwrap_or_default(), in_use);
    }
}

/// D281: where the last physics tick left the Sackboy, for the script tick about to run
/// (`level_play::tick`).
fn pod_report(
    mut pod: ResMut<PodSackboy>,
    sackboy: Query<(&SackboyBody, &Position, &lbp_sim::ZSlab)>,
) {
    if let Ok((body, pos, slab)) = sackboy.single() {
        let [x, y] = to_level(pos.0);
        pod.at = Some(([x, y, slab.z], body.grounded));
    }
}

/// The picker's layers and facing for this tick, after the controller step.
fn tick_anim(
    sackboy: Query<&SackboyBody>,
    mut figure: Query<&mut Figure>,
    mut pod: Option<ResMut<PodSackboy>>,
) {
    // D320: the pod computer clips' own frame counts, once his figure has them.
    if let (Some(pod), Ok(f)) = (pod.as_mut(), figure.single()) {
        let mut frames = pod.frames;
        for (n, b) in frames.iter_mut().zip(&f.pod_bank) {
            if let Some(b) = b {
                *n = u32::from(b.report.frames);
            }
        }
        if pod.frames != frames {
            pod.frames = frames;
        }
    }
    // D281: his pose stands still while the script ticks are held for a capture.
    if pod.as_ref().is_some_and(|p| p.held) {
        return;
    }
    let Ok(body) = sackboy.single() else {
        return;
    };
    let Ok(mut f) = figure.single_mut() else {
        // D281, in the Pod: the ticks before the figure is there are kept for the picker.
        if let Some(pod) = pod.as_mut() {
            pod.backlog.push((body.controller, body.grounded));
        }
        return;
    };
    if let Some(pod) = pod.as_mut().filter(|p| !p.backlog.is_empty()) {
        for (controller, grounded) in std::mem::take(&mut pod.backlog) {
            let a = f.picker.tick(&controller, grounded);
            f.yaw = [f.yaw[1], a.yaw];
            f.picked += 1;
        }
    }
    let a = f.picker.tick(&body.controller, body.grounded);
    // D282: at the pod computer he turns to face into the screen.
    let in_use = pod.as_ref().is_some_and(|p| p.in_use);
    let (yaw, turned) = pod_facing(f.yaw[1], a.yaw, in_use, f.turned);
    f.turned = turned;
    f.yaw = [f.yaw[1], yaw];
    f.anim = Some(a);
    f.picked += 1;
}

/// The layers with a visible share, for the scripted log: `clip@frame*share` (additive
/// layers show their weight with a `+`).
fn layer_summary(a: &AnimFrame) -> String {
    let share = a.effective_weights();
    a.layers
        .iter()
        .zip(share)
        .filter_map(|(l, w)| match l.blend {
            Blend::Lerp if w > 0.01 => Some(format!("{}@{:.1}*{w:.2}", l.clip.name(), l.frame)),
            // A look layer on its neutral frame adds nothing: left out.
            Blend::Additive { reference } if l.frame == reference => None,
            Blend::Additive { .. } => {
                Some(format!("+{}@{:.1}*{:.2}", l.clip.name(), l.frame, l.weight))
            }
            Blend::UpperBody if l.weight > 0.01 => {
                Some(format!("^{}@{:.1}*{:.2}", l.clip.name(), l.frame, l.weight))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The mood for the scripted log: `happy 2 emote` (level, emote state), `happy 1 held`
/// once the emote holds, `happy off to_neutral` while an ended mood plays out, `-` for none.
fn mood_summary(picker: &SackboyAnim) -> String {
    let level = picker.mood().map(|(m, l)| format!("{} {l}", m.name()));
    match (level, picker.emote()) {
        (None, None) => "-".into(),
        (None, Some((m, state, _))) => format!("{} off {}", m.name(), state.name()),
        (Some(level), Some((_, state, _))) if !picker.emote_held() => {
            format!("{level} {}", state.name())
        }
        (Some(level), _) => format!("{level} held"),
    }
}

/// QA (`LBP_DUMP_VERTS=FILE`): appends each shown pose's skinned vertices (mesh space,
/// little-endian f32 xyz) so `pose_fit` can compare them with the game's buffer (D234).
/// Each dumped pose also prints a `sackboy pose` line: the shown pose is the one of the last
/// `sackboy tick` line above it, and a tick with no such line was never drawn (two ticks in
/// one rendered frame), so a comparison can be aligned per tick instead of by counting.
fn dump_pose(positions: &[[f32; 3]], picked: u64) {
    use std::io::Write;
    let Ok(path) = std::env::var("LBP_DUMP_VERTS") else {
        return;
    };
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let mut buf = Vec::with_capacity(positions.len() * 12);
    for p in positions {
        for c in p {
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }
    if f.write_all(&buf).is_ok() {
        println!("sackboy pose {picked}");
    }
}

/// With a script, Sackboy's position after each tick (1-based), and the picked clip.
fn log_scripted(
    mut play: ResMut<Play>,
    sackboy: Query<(&SackboyBody, &Position, &lbp_sim::ZSlab)>,
    figure: Query<&Figure>,
    pod: Option<Res<PodSackboy>>,
) {
    if play.script.is_none() || pod.is_some_and(|p| p.held) {
        return;
    }
    let Ok((body, pos, slab)) = sackboy.single() else {
        return;
    };
    play.tick += 1;
    let p = to_level(pos.0);
    let clip = figure
        .single()
        .ok()
        .and_then(|f| f.anim.as_ref().map(|a| (f, a)))
        .map(|(f, a)| {
            // D282: the shown facing when it is not the picker's (the pod computer's user).
            let shown = if f.turned {
                format!(" shown {:.1}", f.yaw[1].to_degrees())
            } else {
                String::new()
            };
            format!(
                " mood {} yaw {:.1}{shown} layers {}",
                mood_summary(&f.picker),
                a.yaw.to_degrees(),
                layer_summary(a)
            )
        })
        .unwrap_or_default();
    println!(
        "sackboy tick {} x {:.1} y {:.1} z {:.1} vx {:.2} vy {:.2} grounded {}{clip}",
        play.tick, p[0], p[1], slab.z, body.controller.vx, body.controller.vy, body.grounded
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn show_sackboy(
    mut commands: Commands,
    display: Option<Res<LevelDisplay>>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    mut v: ResMut<crate::Viewer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sackboy: Query<(&SackboyBody, &Position, &lbp_sim::ZSlab)>,
    mut marker: Query<&mut Transform, (With<Marker>, Without<Figure>)>,
    mut figure: Query<(&mut Transform, &mut Figure)>,
    mut orbit: ResMut<crate::Orbit>,
    mut cameras: Query<&mut Projection, With<Camera3d>>,
    mut framed: Local<bool>,
    pod: Option<Res<PodSackboy>>,
) {
    let (Some(display), Ok((body, pos, slab))) = (display, sackboy.single()) else {
        return;
    };
    let p = to_level(pos.0);
    let at = display.0.transform_point3(Vec3::new(p[0], p[1], slab.z));
    let s = display.0.x_axis.length();
    if let Ok((mut t, mut f)) = figure.single_mut() {
        let f = &mut *f;
        // D281: a figure under a parent is placed in the parent's space.
        let display = f.display.unwrap_or(display.0);
        let vx = body.controller.vx;
        // The picker eases facing per tick (FOUND 0.4); between ticks the shown angle
        // moves from the previous tick's to the latest by the fixed-step overstep.
        let alpha = fixed.overstep_fraction().clamp(0., 1.);
        f.facing = f.yaw[0] + (f.yaw[1] - f.yaw[0]) * alpha;
        // D282: at the pod computer his figure is drawn toward the engage target; his
        // physics place stays.
        // D320: while the pod computer layer is on (`PodSackboy::user`).
        let [dx, dy, dz] = match pod.as_deref() {
            Some(pod) if pod.user.is_some() => {
                engage_drawn([p[0], p[1], slab.z], pod.target, pod.blend)
            }
            _ => [p[0], p[1], slab.z],
        };
        let feet = display.transform_point3(Vec3::new(
            dx,
            dy - FLOOR_REST + body.controller.landing_dip(),
            dz,
        ));
        *t = Transform::from_matrix(
            Mat4::from_translation(feet) * Mat4::from_rotation_y(f.facing) * f.upright,
        );
        if f.banked {
            // Clip bank: one blended pose per picked tick (stand stays the playing clip).
            // D320: and again when the pod computer layer moves on.
            let user = pod.as_deref().and_then(|p| p.user);
            if f.posed != f.picked || f.pod_posed != user {
                if let (Some(a), Some(playing)) = (&f.anim, &mut v.playing) {
                    let c = &body.controller;
                    let layer = user.and_then(|u| {
                        let b = f.pod_bank.get(u.clip.index())?.as_ref()?;
                        Some((b, u.frame as f32, u.weight))
                    });
                    let pose = blend_frame(
                        &playing.clip,
                        &f.bank,
                        a,
                        c.landing_squash(),
                        c.foot_plant(),
                        layer,
                    );
                    dump_pose(&playing.clip.blended_positions(&pose), f.picked);
                    playing.blended = Some(pose);
                }
                f.posed = f.picked;
                f.pod_posed = user;
            }
        } else if let Some(fps) = v
            .playing
            .as_ref()
            .map(|p| f32::from(p.clip.report.fps.max(1)))
        {
            f.phase = walk_phase(f.phase, time.delta_secs(), fps, vx, body.grounded);
            v.animation_frame = Some(f.phase as i64);
        }
    } else if let Ok(mut t) = marker.single_mut() {
        t.translation = at + Vec3::Y * CENTRE_ABOVE * s;
    } else {
        commands.spawn((
            Marker,
            Mesh3d(meshes.add(Capsule3d::new(RADIUS * s, 2. * HALF_SEGMENT * s))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.55, 0.3),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(at + Vec3::Y * CENTRE_ABOVE * s),
        ));
    }
    // D281: in the Pod the camera is the one the planets level's script writes
    // (`level_play::script_camera`); the follow camera is not his there.
    if pod.is_some() {
        return;
    }
    let [ex, ey, ez] = FOLLOW_EYE;
    if !*framed {
        *framed = true;
        FRAMINGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // The game's default camera: straight on, at its distance and FOV (orbit and zoom
        // stay free afterwards).
        orbit.yaw = 0.;
        orbit.pitch = 0.;
        orbit.distance = ez * s;
        for mut projection in &mut cameras {
            if let Projection::Perspective(p) = projection.as_mut() {
                p.fov = FOLLOW_FOV_DEG.to_radians();
            }
        }
    }
    orbit.target = display
        .0
        .transform_point3(Vec3::new(p[0] + ex, p[1] + ey, slab.z));
}

/// Counts camera framings, so overlays that nudge `orbit.distance` re-apply after one.
pub(crate) static FRAMINGS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_script_steps() {
        let s = parse_script("r:40, rj:12,-:30,bl:1").unwrap();
        assert_eq!(s.len(), 4);
        assert_eq!((s[0].0.x, s[0].1), (lbp_sim::sackboy::STICK_FULL_RIGHT, 40));
        assert!(s[1].0.jump && s[1].0.x == lbp_sim::sackboy::STICK_FULL_RIGHT);
        assert_eq!(s[2].0, Input::default());
        assert!(s[3].0.back && s[3].0.x == -1.);
        assert!(parse_script("x:3").is_err());
        assert!(parse_script("r").is_err());
    }

    /// D281. The Sackboy plays in the Pod only with the play mode and the three options
    /// that put the shell level in the scripts' session.
    #[test]
    fn d281_the_pod_wants_the_play_mode_and_the_scripted_shell() {
        let args = |list: &[&str]| -> Vec<String> { list.iter().map(|a| a.to_string()).collect() };
        let all = [
            "--play-sackboy",
            "--level-scripts",
            "--pod-camera",
            "1",
            "--pod-shell",
            "000027b9",
        ];
        assert_eq!(pod_shell(&args(&all)), Ok(Some(0x27b9)));
        // Without any one of the three he plays in the level given, as before.
        for (skip, len) in [(1, 1), (2, 2), (4, 2)] {
            let mut less = all.to_vec();
            less.drain(skip..skip + len);
            assert_eq!(pod_shell(&args(&less)), Ok(None), "{less:?}");
        }
        // D320: the scripted Pod has him without the play mode's option too, unless
        // `--no-pod-sackboy`, which the play mode's option overrides.
        assert_eq!(pod_shell(&args(&all[1..])), Ok(Some(0x27b9)));
        let mut off = all[1..].to_vec();
        off.push("--no-pod-sackboy");
        assert_eq!(pod_shell(&args(&off)), Ok(None));
        off.push("--play-sackboy");
        assert_eq!(pod_shell(&args(&off)), Ok(Some(0x27b9)));
        let bad = args(&[
            "--play-sackboy",
            "--level-scripts",
            "--pod-camera",
            "1",
            "--pod-shell",
            "pod",
        ]);
        assert!(pod_shell(&bad).is_err());
    }

    /// D281. `e` in an input script is the engage button on the first tick of its step,
    /// and no movement key.
    #[test]
    fn d281_the_engage_key_of_a_script_is_a_click_on_its_steps_first_tick() {
        let text = "-:460,r:12,e:1,-:30,l:12,-:20,le:3,-:60";
        assert_eq!(engage_ticks(text), [472, 535]);
        assert_eq!(engage_ticks("r:40,j:2"), Vec::<usize>::new());
        assert_eq!(engage_ticks("e:1"), [0]);
        let s = parse_script(text).unwrap();
        assert_eq!(s[2], (Input::default(), 1));
        assert!(s[6].0.x == -1. && s[6].1 == 3);
    }

    /// D281. The Pod's physics scene: the pod Thing's shape is laid out as pod.bin's
    /// (the outline in local x and z, local z along the level's y), so its floor is
    /// found through the whole matrix, not in the plane; every body is held and no joint
    /// is left. The data spawn (the fitted figure's rule; D282: no longer the Pod's start)
    /// finds that floor, his rest height above it.
    #[test]
    fn d281_the_pod_scene_has_the_projected_floor_and_holds_every_body() {
        use lbp_level::parts::{Body, Pos, Shape, ShapeColor};
        use lbp_level::thing::ThingRecord;
        // A floor block 2000 wide whose top is 100 above its place (y -647): top -547.
        let shape = Shape {
            vertex_count: 4,
            requires_z: true,
            vertices: vec![
                [-1000., 0., -100.],
                [1000., 0., -100.],
                [1000., 0., 100.],
                [-1000., 0., 100.],
            ],
            loops: vec![4],
            material: None,
            old_material: None,
            thickness: 900.,
            mass_depth: 1.,
            color: ShapeColor::Packed(0xffff_ffff),
            brightness: 0.,
            bevel_size: 10.,
            com: None,
            behavior: 0,
            flags: 7,
            contacts: 0,
        };
        let mut world = [0.; 16];
        (world[0], world[6], world[9], world[15]) = (1., -1., 1., 1.);
        (world[12], world[13], world[14]) = (0., -647., 169.);
        let mut pod = ThingRecord::default();
        pod.data.pos = Some(Pos {
            bone_of: None,
            anim_hash: 0,
            world,
        });
        pod.data.shape = Some(shape);
        pod.data.body = Some(Body {
            velocity: [0.; 3],
            angular_velocity: 0.,
            frozen: 0,
        });
        let things = [pod];
        let rule = |_: &lbp_level::serial::Descriptor| MaterialRule::Normal;
        let scene = pod_scene(&things, &rule);
        assert_eq!(scene.bodies.len(), 1);
        assert!(scene.bodies.iter().all(|b| b.fixed) && scene.joints.is_empty());
        // The floor in the front lane and the two behind it (the slab is 900 deep).
        for lane in LANES {
            let top = scene.floor_below([-4.16, 541.], lane).unwrap();
            assert!((top + 547.).abs() < 1e-3, "lane {lane}: {top}");
        }
        // In the plane the same shape has no floor there: the in-plane scene would not do.
        let plain = build_scene_with(&things, &rule);
        assert!(!plain
            .floor_below([-4.16, 541.], 0.)
            .is_some_and(|top| (top + 547.).abs() < 1.));
        // The start: the checkpoint's x by the spawn offset, the nearest lane, the floor.
        let mut checkpoint = [0.; 16];
        (checkpoint[12], checkpoint[13], checkpoint[14]) = (-3.05, 540.76, -50.);
        let (feet, line) = data_spawn(&scene, 4, &checkpoint).unwrap();
        assert!((feet[0] + 4.16).abs() < 1e-3 && (feet[1] + 547.).abs() < 1e-3 && feet[2] == 0.);
        assert!(line.contains("checkpoint Thing 4"), "{line}");
        // He stands his rest height above it: within 1.1 of the game's measured y.
        let y = feet[1] + FLOOR_REST;
        assert!((y - POD_ARRIVAL_MEASURED[1]).abs() < 1.2, "{y}");
    }

    /// D282. The checkpoint spawn rule: the checkpoint's place, 50 forward in z (or from a
    /// box's z when that is in front), the slot's x offset, no height offset.
    #[test]
    fn d282_the_arrival_is_the_checkpoint_spawn_rule() {
        let mut checkpoint = [0.; 16];
        (checkpoint[0], checkpoint[5], checkpoint[10], checkpoint[15]) = (1., 1., 1., 1.);
        (checkpoint[12], checkpoint[13], checkpoint[14]) = (-3.05, 540.76, -50.);
        // The Pod: slot 0, lane 0, at the checkpoint's height.
        assert_eq!(
            checkpoint_arrival(&checkpoint, None, 0),
            [-3.05, 540.76, 0.]
        );
        // A box in front of the checkpoint wins; one behind it does not.
        assert_eq!(checkpoint_arrival(&checkpoint, Some(-10.), 0)[2], 40.);
        assert_eq!(checkpoint_arrival(&checkpoint, Some(-300.), 0)[2], 0.);
        // The slots' x offsets; the sixth player takes slot 0 again.
        let xs: Vec<f32> = (0..6)
            .map(|slot| checkpoint_arrival(&checkpoint, None, slot)[0] + 3.05)
            .collect();
        for (x, want) in xs.iter().zip([0., 80., -40., 40., -80., 0.]) {
            assert!((x - want).abs() < 1e-4, "{xs:?}");
        }
        // He falls from there onto a floor below (the pod scene of the d281 test is not
        // needed for the rule; his y is the checkpoint's whatever the slot).
        assert!((0..5).all(|s| checkpoint_arrival(&checkpoint, None, s)[1] == 540.76));
    }

    /// D282. At the pod computer: the drawn offset is 0 at blend 0 and the target at 1,
    /// the blend is 0 when not in use, his movement input is not applied while in use and
    /// returns afterwards, and he turns to face pi at the picker's rate and back.
    #[test]
    fn d282_the_pod_computer_user_is_held_turned_and_drawn_toward_the_target() {
        let at = [-5.37, -476.9, 0.];
        let target = engage_target(Some(-12.469));
        assert_eq!(target, POD_ENGAGE_TARGET);
        assert_eq!(engage_target(None), POD_ENGAGE_TARGET);
        assert_eq!(engage_target(Some(4.))[0], 4.);
        // D320: x and y by the found rule; z from his lane at the pod computer, reversed.
        assert_eq!(
            engage_drawn(at, target, 0.),
            [at[0], at[1], POD_USER_LANE_Z]
        );
        let full = engage_drawn(at, target, 1.);
        assert!(
            (0..2).all(|i| (full[i] - target[i]).abs() < 1e-4),
            "{full:?}"
        );
        assert!((full[2] + 306.83).abs() < 1e-2, "{full:?}");
        let half = engage_drawn(at, target, 0.5);
        assert!((half[2] + 253.415).abs() < 1e-2, "{half:?}");
        let walk = Input {
            x: -1.,
            jump: true,
            back: true,
            ..Input::default()
        };
        assert_eq!(pod_input(walk, true), Input::default());
        assert_eq!(pod_input(walk, false), walk);
        // Facing: eased toward pi while in use, then back to the picker's angle.
        let (mut yaw, mut turned) = (0., false);
        for _ in 0..40 {
            (yaw, turned) = pod_facing(yaw, 0., true, turned);
        }
        assert!(turned && (yaw - POD_USER_FACING).abs() < 1e-3, "{yaw}");
        assert_eq!(POD_USER_FACING, std::f32::consts::PI);
        let first = pod_facing(yaw, 0., false, turned);
        assert!(first.1 && first.0 < yaw && first.0 > 0.);
        for _ in 0..40 {
            (yaw, turned) = pod_facing(yaw, 0., false, turned);
        }
        assert_eq!((yaw, turned), (0., false));
        // Never in use: the picker's angle as it is.
        assert_eq!(pod_facing(0.3, 1.2, false, false), (1.2, false));
    }

    /// D320. The pod computer layer: off until he is the user; then the into clip from
    /// frame 0 with weight (n + 1) / 3 and offset blend (n - 5) / 11, the idle clip
    /// looping once it has played out; the out clip when he stops, offset 1 - (n - 5) / 10
    /// and weight (frames - n) / 3; off again after it.
    #[test]
    fn d320_the_pod_computer_layer_runs_into_idle_and_out() {
        let frames = [20, 8, 12];
        let mut a = PodUserAnim::default();
        assert_eq!(a.step(false, frames), None);
        let into: Vec<_> = (0..20).map(|_| a.step(true, frames).unwrap()).collect();
        assert!(into.iter().all(|u| u.clip == PodClip::Into));
        assert_eq!(
            into.iter().map(|u| u.frame).collect::<Vec<_>>(),
            (0..20).collect::<Vec<_>>()
        );
        assert!((into[0].weight - 1. / 3.).abs() < 1e-6 && into[2].weight == 1.);
        assert!(into[..=5].iter().all(|u| u.offset == 0.));
        assert!((into[6].offset - 1. / 11.).abs() < 1e-6 && into[16].offset == 1.);
        // Played out: the idle clip from frame 0, then looping.
        let idle: Vec<_> = (0..10).map(|_| a.step(true, frames).unwrap()).collect();
        assert!(idle
            .iter()
            .all(|u| u.clip == PodClip::Idle && u.weight == 1. && u.offset == 1.));
        assert_eq!(
            idle.iter().map(|u| u.frame).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5, 6, 7, 0, 1]
        );
        // Out.
        let out: Vec<_> = (0..13).map(|_| a.step(false, frames)).collect();
        assert!(out
            .iter()
            .all(|u| u.is_some_and(|u| u.clip == PodClip::Out)));
        let out: Vec<_> = out.into_iter().flatten().collect();
        assert!(out[..=5].iter().all(|u| u.offset == 1.));
        assert!((out[10].offset - 0.5).abs() < 1e-6);
        assert!(out[9].weight == 1. && (out[10].weight - 2. / 3.).abs() < 1e-6);
        assert_eq!((out[12].weight, out[12].frame), (0., 11));
        assert_eq!(a.step(false, frames), None);
        // He becomes the user again during the out clip: the into clip starts over.
        let mut b = PodUserAnim::default();
        b.step(true, frames);
        b.step(false, frames);
        assert_eq!(
            b.step(true, frames).map(|u| (u.clip, u.frame)),
            Some((PodClip::Into, 0))
        );
    }

    #[test]
    fn walk_cycle_paces_with_speed() {
        assert_eq!(walk_phase(4., 0.5, 30., WALK_SPEED, true), 19.);
        assert_eq!(walk_phase(4., 0.5, 30., -WALK_SPEED / 2., true), 11.5);
        assert_eq!(walk_phase(4., 0.5, 30., WALK_SPEED, false), 0.);
        assert_eq!(walk_phase(4., 0.5, 30., 0., true), 0.);
    }

    #[test]
    fn spawn_lane_is_the_nearest_lane() {
        // Moon level: checkpoint z -470 spawns in the back lane.
        assert_eq!(spawn_lane(-470.), -400.);
        assert_eq!(spawn_lane(-50.), 0.);
        assert_eq!(spawn_lane(-180.), -200.);
        assert_eq!(spawn_lane(300.), 0.);
    }
}
