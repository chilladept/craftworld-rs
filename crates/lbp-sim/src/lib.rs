//! R2 physics sandbox (docs/runtime-research.md §4 and §5; docs/sim-physics.md): a read
//! level's shapes as one avian2d world stepped at the game's 30 Hz tick, with a z-slab
//! pair filter standing in for layers. Runtime only; the parsing crates never depend on it.
//!
//! - [`scene`] (no Bevy) decides bodies, colliders, frames, triangles and slabs.
//! - [`SimPlugin`] adds avian2d with [`SlabHooks`], the fixed tick, substeps and gravity.
//! - [`spawn_scene`] spawns one entity per body and one collider entity per shape.
//!
//! Constants marked PLACEHOLDER wait on RPCS3 measurements or RMaterial reads.
use avian2d::math::Vector;
use avian2d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub mod character;
pub mod sackboy;
pub mod sackboy_anim;
pub mod scene;

pub use avian2d;
pub use scene::{build_scene, build_scene_with, build_spawn_scene_with, PhysicsScene};

/// Game ticks per second (X1, CONFIRMED_FROM_LOCAL_DATA, emulated; docs/runtime-research.md).
pub const TICK_HZ: f64 = 30.0;
/// Solver substeps per tick: our choice (the game's solver is not public).
pub const SUBSTEPS: u32 = 6;
/// Gravity in world units per second² along -y: 2,424 (2.693 per tick²). S9 jump
/// measurement in RPCS3 (Rookie Test, three runs, Sackboy's descent fits one parabola with
/// error under 0.1%): CONFIRMED_FROM_LOCAL_DATA (emulated) for Sackboy falling; using it for
/// every body is INFERRED (material gravity multipliers are not read yet).
pub const GRAVITY: f32 = 2424.0;
/// Physics units per LBP world unit: the solver works in "metres" of 100 world units
/// (our choice). In world units a 100-unit box has an angular inertia near 1e8, and
/// avian skips motors whose inverse inertia is below f32 epsilon. Everything spawned is
/// scaled by this; read positions back with [`to_level`].
pub const UNIT_SCALE: f32 = 0.01;

/// D330: the world's gravity per tick² at the normal gravity setting (1.0), the value
/// [`gravity_per_tick`] gives there. FOUND (01.33): the game's gravity-ratio getter (0xce6f0)
/// divides the world gravity by -2.7.
pub const NORMAL_GRAVITY_PER_TICK: f32 = 2.7;

/// D330: the world's gravity per tick² (downwards) for the level's gravity global setting
/// (`Game.SetGlobalSettingsGravityTo`, the PWorld's blended type-1 value): `2.3 x setting +
/// 0.4`, so 2.7 at the normal 1.0 and 0.4 at 0.0. FOUND (01.33): the gravity setter (0xcef3c)
/// and the blend-factor setter (0xd623c) write `setting x -2.3 - 0.4` into the y lane of the
/// physics world's gravity vector each time they run, with the setting as now blended; the
/// world gravity native (0x5fd4c8) reads that vector's length.
pub fn gravity_per_tick(setting: f32) -> f32 {
    2.3 * setting + 0.4
}

/// D330: the level's gravity global setting in force, which scales the world's gravity
/// ([`SimPlugin`]) and Sackboy's ([`character`]) against the normal setting. A display that
/// runs the level's scripts writes it; without them it stays at the normal 1.0.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WorldGravity {
    pub setting: f32,
}

impl Default for WorldGravity {
    fn default() -> Self {
        Self { setting: 1.0 }
    }
}

impl WorldGravity {
    /// The world's gravity over the normal one: 1.0 at the normal setting. A setting that is
    /// not a number counts as the normal one (our choice: the game would carry the NaN).
    pub fn scale(&self) -> f32 {
        if self.setting.is_finite() {
            gravity_per_tick(self.setting) / NORMAL_GRAVITY_PER_TICK
        } else {
            1.0
        }
    }
}

/// A physics position in LBP world units.
pub fn to_level(p: Vector) -> [f32; 2] {
    [p.x / UNIT_SCALE, p.y / UNIT_SCALE]
}

fn sv(p: [f32; 2]) -> Vector {
    Vector::new(p[0] * UNIT_SCALE, p[1] * UNIT_SCALE)
}

/// Joint motors: share of the speed error removed per substep (avian `AccelerationBased`
/// damping; above 1 it overshoots). Our choice, for a motor at full strength
/// ([`motor_damping`]).
pub const MOTOR_DAMPING: f32 = 0.5;

/// D1173: FOUND (01.33) a motor's strength is PJoint strength `+0xc0` clamped to [0, 1]
/// (0xca760), times max(1, 4 × its speed 0xc746c) over the sum of its moving ends'
/// `*(body+0x110)+0x38` (0xca2a4, type 9). The motor row 0x95f494 bounds its push by the
/// strength times the ends' speed error (0x95fab8, skipped only for a strength of
/// FLT_MAX, which a motor never has), so a motor at strength 0 does not turn its ends.
/// INFERRED: the clamped strength is the share of the speed error the motor can remove,
/// so it scales [`MOTOR_DAMPING`].
pub fn motor_damping(params: &lbp_level::parts::JointParams) -> f32 {
    MOTOR_DAMPING * params.strength.clamp(0., 1.)
}

/// D1178: FOUND (01.33) the solver's motor case 0x1c9eac passes the motor row 0x95f494 a
/// k (f1) of 5 × the motor's strength (0xca760, TOC 0xd984d8) for pattern 2, or 0 for any
/// other pattern or a pattern-2 motor wired in mode 1 or 4. The row (0x95f740) turns it
/// into a force cap of k × max(|err|, 0.1) (TOC -0x202c) on the band edge at 1.5 × `err`
/// ([`motor_band`]).
pub fn motor_k(params: &lbp_level::parts::JointParams, wired: bool) -> f32 {
    if params.animation_pattern != 2 || (wired && matches!(params.mode, 1 | 4)) {
        return 0.;
    }
    5. * params.strength.clamp(0., 1.)
}

/// D1178: FOUND (01.33, the motor row 0x95f494) the band the row holds a motor's turn per
/// frame to: `delta` cut to between 0 and `err` (0x95fa28, 0x95fb10, as
/// [`PistonDrive::turn_to`] has it) and 1.5 × `err` (TOC -0x1d88, 0x95f734), sorted
/// (0x95f778), with the strength cap on the `delta` edge and k × max(|err|, 0.1) on the
/// 1.5 × `err` edge (INFERRED pairing; the row is solved off the PPU). Returns (lo, hi,
/// the 1.5 × `err` edge's cap). INFERRED: with k 0 (every motor but pattern 2) that edge
/// has no force, so only a pattern-2 motor is braked once it turns past 1.5 × `err`.
pub fn motor_band(delta: f32, err: f32, k: f32) -> (f32, f32, f32) {
    row_band(delta, err, k, 0.1)
}

/// D1180: FOUND (01.33) the piston's length row 0x96178c (called first by the type-4 row
/// 0x962590 with the same k, strength cap and limits) holds the length's change per frame
/// to the motor row's band, in game units: `delta` = target(t + 1) − target(t) from the
/// length rule 0xc95ec (0x961950, 0x961984, 0x9619ac), `err` = target(t + 1) − the
/// measured anchor distance (0x961d80), `delta` cut to between 0 and `err` (0x961de4,
/// 0x9624bc; skipped when wired in mode 1 or 4, and pattern 1 drives at 0xc7370), and
/// 1.5 × `err` (TOC -0x1d88, 0x961d9c) with a cap of k × max(|err|, 10) (TOC -0x1cdc =
/// 10, 0x961c84 to 0x961cc8), sorted (0x961df0) with the strength cap on the `delta`
/// edge (row+0x40, unscaled for a joint with a type, 0x961cfc). Returns (lo, hi, the
/// 1.5 × `err` edge's cap). The length limits sit in the same row (row+0x30, lo − L and
/// hi − L), eased back by 0.2 × (past − 1) when the length is outside them (TOC -0x1fe0,
/// 0x962390, 0x962408). Not applied: the piston is held at its length ([`drive_pistons`]).
pub fn piston_band(delta: f32, err: f32, k: f32) -> (f32, f32, f32) {
    row_band(delta, err, k, 10.)
}

fn row_band(delta: f32, err: f32, k: f32, floor: f32) -> (f32, f32, f32) {
    let d = cut(delta, err);
    let e = 1.5 * err;
    (d.min(e), d.max(e), k * err.abs().max(floor))
}

/// D1174: FOUND (01.33) the solver's motor case 0x1c9eac gives the motor row its angle
/// limits: for pattern 0 with a range `+0xc8` under 2π, the centre `+0xe0` ± range / 2
/// (sorted); for pattern 2 with a range under 2π, the centre to the centre + range;
/// otherwise none (±FLT_MAX).
/// D1175: FOUND (01.33, 0xc95ec's motor branch 0xc9860 to 0xc996c) a motor's pattern-0
/// rule is centred, the centre − range / 2 × cos(π s), so its sweep is exactly the limits.
/// Our rule ([`scene::SceneJoint::motor_angle`]) is the game's + range / 2, which the sync
/// cancels; avian counts the angle from the spawn, where ours is `start`, so the limits
/// are -start to -start + range (sorted).
/// D1177: FOUND (01.33, 0xc98d0) a free-running pattern-2 motor has no motor branch: its
/// target is the centre + range × (1 − u / time), u the clock fmod time + pause, so it
/// sweeps the centre to the centre + range, which is 0x1ca6f0's limits. Our pattern-2
/// rule sweeps the same [0, range] from `start`, so it takes the same limits as pattern 0;
/// they hold in our own turn sign whatever Part 74's sign turns out to be.
pub fn motor_angle_limits(
    params: &lbp_level::parts::JointParams,
    start: f32,
) -> Option<(f32, f32)> {
    let range = params.motion[0];
    (matches!(params.animation_pattern, 0 | 2) && range < std::f32::consts::TAU)
        .then(|| (-start + range.min(0.), -start + range.max(0.)))
}
/// Angular springs (SPRING_ANGULAR, wobble): avian spring-damper. PLACEHOLDER until
/// PJoint.strength is calibrated.
pub const SPRING_FREQUENCY: f32 = 2.0;
pub const SPRING_DAMPING_RATIO: f32 = 0.5;
/// ELASTIC and SPRING joint compliance (inverse stiffness). PLACEHOLDER.
pub const SPRING_COMPLIANCE: f32 = 1e-4;

/// Thing index of the entity in the level.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThingIndex(pub usize);

/// A collider's layer slab: world z centre and half depth.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ZSlab {
    pub z: f32,
    pub half_thickness: f32,
}

/// Depth overlap a slab pair needs before it collides. INFERRED (D1153): the
/// game's pairwise depth test runs on the SPU and was not found; level z values
/// carry float rounding (99.99994 for 100), so slabs that only touch must not
/// collide by a few ulps.
pub const SLAB_TOUCH_EPSILON: f32 = 0.01;

impl ZSlab {
    /// Slabs collide when their depth ranges overlap by more than
    /// [`SLAB_TOUCH_EPSILON`] (touching is not overlapping).
    pub fn overlaps(&self, other: &ZSlab) -> bool {
        (self.z - other.z).abs() < self.half_thickness + other.half_thickness - SLAB_TOUCH_EPSILON
    }
}

/// Collision hooks: shapes in non-overlapping slabs never touch.
#[derive(SystemParam)]
pub struct SlabHooks<'w, 's> {
    slabs: Query<'w, 's, &'static ZSlab>,
}

impl CollisionHooks for SlabHooks<'_, '_> {
    fn filter_pairs(&self, collider1: Entity, collider2: Entity, _: &mut Commands) -> bool {
        match self.slabs.get_many([collider1, collider2]) {
            Ok([a, b]) => a.overlaps(b),
            Err(_) => true,
        }
    }
}

/// avian2d with [`SlabHooks`], stepped in `FixedPostUpdate` at [`TICK_HZ`].
pub struct SimPlugin {
    pub gravity: f32,
}

impl Default for SimPlugin {
    fn default() -> Self {
        Self { gravity: GRAVITY }
    }
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        let base = self.gravity;
        // D330: the world's gravity follows the level's gravity setting.
        let follow = move |g: Res<WorldGravity>, mut gravity: ResMut<Gravity>| {
            if g.is_changed() {
                gravity.0 = Vector::new(0., -base * g.scale() * UNIT_SCALE);
            }
        };
        app.add_plugins(PhysicsPlugins::default().with_collision_hooks::<SlabHooks>())
            .insert_resource(Time::<Fixed>::from_hz(TICK_HZ))
            .insert_resource(SubstepCount(SUBSTEPS))
            .insert_resource(Gravity(Vector::new(0., -self.gravity * UNIT_SCALE)))
            .init_resource::<WorldGravity>()
            .add_systems(FixedPreUpdate, follow)
            .add_systems(
                FixedPostUpdate,
                put_to_sleep
                    .after(PhysicsSystems::StepSimulation)
                    .before(PhysicsSystems::Writeback),
            );
    }
}

/// D1101: a body's anti-gravity tweakers, for [`follow_tweaks`].
#[derive(Component, Debug, Clone)]
pub struct AntiGravityTweaks(pub scene::AntiGravity);

/// D1101: sets each tweaked body's gravity scale from the tweaks' current activations and
/// wakes a body whose scale changed (a sleeping body would not feel the new gravity).
pub fn follow_tweaks(
    commands: &mut Commands,
    q: &mut Query<(Entity, &AntiGravityTweaks, &mut GravityScale)>,
    activation: impl Fn(&scene::AntiGravityTweak) -> f32,
) {
    for (e, tweaks, mut scale) in q.iter_mut() {
        let s = tweaks.0.gravity_scale(&activation);
        if s.to_bits() != scale.0.to_bits() {
            scale.0 = s;
            commands.queue(avian2d::dynamics::solver::islands::WakeBody(e));
        }
    }
}

/// D1109: a body's gyroscopes and rotators, for [`drive_spins`].
#[derive(Component, Debug, Clone)]
pub struct SpinTweaks(pub Vec<scene::SpinTweak>);

/// D1109: each gyroscope and rotator moves its body's turn speed towards its target
/// ([`scene::SpinTweak::turn`], per tick, in order) with the tweak's current activation,
/// and wakes a body whose speed changed. D1144: configurations 3 and 5 aim along the
/// body's velocity or at the nearest target they see ([`scene::SpinTweak::aim`]): with
/// followKey a tag in `tags` with its key not on its own body, else `player`. D1164: a
/// tag on a body leads by that body's point velocity ([`scene::SpinTweak::lead`]), from
/// `motion`: the body's velocity and turn speed (game units and radians per tick) and its
/// centre. D1165: the player is a Thing with a body too (+0x64), so `player` carries
/// Sackboy's velocity (game units per tick) and he is led by it (INFERRED: his body does
/// not turn and his u16 +0xe is 0).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn drive_spins(
    commands: &mut Commands,
    q: &mut Query<(
        Entity,
        &SpinTweaks,
        &Position,
        &Rotation,
        &LinearVelocity,
        &mut AngularVelocity,
    )>,
    player: Option<(Vec2, [f32; 2])>,
    tags: &[TagSpot],
    activation: impl Fn(&scene::SpinTweak) -> f32,
    stick: impl Fn(&scene::SpinTweak) -> [f32; 2],
    scale: impl Fn(&scene::SpinTweak) -> f32,
    motion: impl Fn(Entity) -> Option<([f32; 2], f32, [f32; 2])>,
) {
    let hz = TICK_HZ as f32;
    let per_tick = 1. / (hz * UNIT_SCALE);
    for (e, tweaks, pos, rot, v, mut w) in q.iter_mut() {
        let angle = rot.as_radians();
        let up = [-rot.sin, rot.cos];
        let vel = [v.0.x * per_tick, v.0.y * per_tick];
        let before = w.0 / hz;
        let after = tweaks.0.iter().fold(before, |s, t| {
            // D1158: configuration 4 reads its stick this tick.
            let t = &scene::SpinTweak {
                stick: stick(t),
                scale: scale(t),
                ..*t
            };
            // D1156: measured from the tweak's parent point.
            let me = [
                pos.0.x / UNIT_SCALE + rot.cos * t.at[0] - rot.sin * t.at[1],
                pos.0.y / UNIT_SCALE + rot.sin * t.at[0] + rot.cos * t.at[1],
            ];
            let (d, lead) = match t.key {
                Some(k) => {
                    let g = tags
                        .iter()
                        .filter(|g| g.key == k && g.body != Some(e))
                        .map(|g| (g, [g.at[0] - me[0], g.at[1] - me[1]]))
                        .filter(|&(_, d)| t.sees(d, up))
                        .min_by(|a, b| a.1[0].hypot(a.1[1]).total_cmp(&b.1[0].hypot(b.1[1])));
                    let lead = g.and_then(|(g, _)| Some((g.at, motion(g.body?)?))).map_or(
                        [0., 0.],
                        |(at, (v, w, c))| {
                            scene::SpinTweak::lead(v, w, [at[0] - c[0], at[1] - c[1]])
                        },
                    );
                    (g.map(|g| g.1), lead)
                }
                None => (
                    player
                        .map(|(p, _)| [p.x / UNIT_SCALE - me[0], p.y / UNIT_SCALE - me[1]])
                        .filter(|&d| t.sees(d, up)),
                    player.map_or([0., 0.], |(_, v)| v),
                ),
            };
            let lead = if d.is_some() { lead } else { [0., 0.] };
            t.turn_led(angle, s, activation(t), t.aim(vel, d), lead)
        });
        if (after - before).abs() > 1e-6 {
            w.0 = after * hz;
            commands.queue(avian2d::dynamics::solver::islands::WakeBody(e));
        }
    }
}

/// D1110: a body's movers and followers, for [`drive_moves`].
#[derive(Component, Debug, Clone)]
pub struct MoveTweaks(pub Vec<scene::MoveTweak>);

/// D1111: the tags riding on a body (Thing, key, point in the body's frame, game units;
/// D1168: then the world z),
/// or, on the one entity [`spawn_scene`] makes for them, the tags on no body (world
/// points).
#[derive(Component, Debug, Clone)]
pub struct TagMarks(pub Vec<(u32, u32, [f32; 2], f32)>);

/// D1111: a tag where it is this tick: its Thing, its key, the body it rides (None: on no
/// body) and its world position in game units. D1168: `z` is its world z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TagSpot {
    pub thing: u32,
    pub key: u32,
    pub body: Option<Entity>,
    pub at: [f32; 2],
    pub z: f32,
}

/// D1111: every tag's [`TagSpot`] from the [`TagMarks`] entities, a body's with its
/// position and rotation.
pub fn tag_spots<'a>(
    marks: impl IntoIterator<Item = (Entity, &'a TagMarks, Option<(&'a Position, &'a Rotation)>)>,
) -> Vec<TagSpot> {
    let mut spots = Vec::new();
    for (e, m, body) in marks {
        let frame = body.map(|(p, r)| scene::Frame {
            position: [p.0.x / UNIT_SCALE, p.0.y / UNIT_SCALE],
            angle: r.as_radians(),
        });
        spots.extend(m.0.iter().map(|&(thing, key, at, z)| TagSpot {
            thing,
            key,
            body: frame.map(|_| e),
            at: frame.map_or(at, |f| f.world(at)),
            z,
        }));
    }
    spots
}

/// D1155: the tags a follower or an aiming tweak can find. FOUND (0x1c3e04): the keyed
/// search takes each tag's signal (0x12f6f4) and passes over a tag whose ternary is 0, so
/// a tag switched off by its wired input is not there; `ternary` gives a tag Thing's.
pub fn lit_tags(mut spots: Vec<TagSpot>, ternary: impl Fn(u32) -> i32) -> Vec<TagSpot> {
    spots.retain(|g| ternary(g.thing) != 0);
    spots
}

/// D1157: the tag synchronisers riding on a body ([`scene::SceneSync`], in the body's
/// frame), or, on the one entity [`spawn_scene`] makes for them, those on no body.
#[derive(Component, Debug, Clone)]
pub struct SyncMarks(pub Vec<scene::SceneSync>);

/// D1157: a tag synchroniser where it is this tick, as [`scene::SceneSync`] with `at` and
/// `axes` in the world and `body` its body's entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncSpot {
    pub thing: u32,
    pub kind: i32,
    pub key: u32,
    pub radius: f32,
    pub min_radius: f32,
    pub scale: [f32; 2],
    pub use_layers: bool,
    pub body: Option<Entity>,
    pub at: [f32; 2],
    pub axes: [[f32; 2]; 2],
    pub z: f32,
}

/// D1157: every synchroniser's [`SyncSpot`] from the [`SyncMarks`] entities.
pub fn sync_spots<'a>(
    marks: impl IntoIterator<Item = (Entity, &'a SyncMarks, Option<(&'a Position, &'a Rotation)>)>,
) -> Vec<SyncSpot> {
    let mut spots = Vec::new();
    for (e, m, body) in marks {
        let frame = body.map(|(p, r)| scene::Frame {
            position: [p.0.x / UNIT_SCALE, p.0.y / UNIT_SCALE],
            angle: r.as_radians(),
        });
        spots.extend(m.0.iter().map(|y| {
            let turn = |a: [f32; 2]| match frame {
                Some(f) => {
                    let (s, c) = f.angle.sin_cos();
                    [c * a[0] - s * a[1], s * a[0] + c * a[1]]
                }
                None => a,
            };
            SyncSpot {
                thing: y.thing as u32,
                kind: y.kind,
                key: y.key,
                radius: y.radius,
                min_radius: y.min_radius,
                scale: y.scale,
                use_layers: y.use_layers,
                body: frame.map(|_| e),
                at: frame.map_or(y.at, |f| f.world(y.at)),
                axes: [turn(y.axes[0]), turn(y.axes[1])],
                z: y.z,
            }
        }));
    }
    spots
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

/// D1157: the tags the receivers place this tick, from the found tags `tags` (pass
/// [`lit_tags`]). FOUND (EBOOT 01.33, the world list +0x2b8 tick 0x132800): a sender
/// (kind 0, 0x13187c) takes the nearest tag of its key with minRadius² <= d² <= radius²
/// (no distance test for a radius below 0.0, DAT_00d96820), passing over tags on its own
/// root (INFERRED here: its own body) and the synchronisers' own tags, and keeps the tag's
/// offset along its unit axes. A receiver (kind 1, 0x131f88) takes a sender of its key
/// with an offset (0x130c90; the nearest, INFERRED) and places its tag at its point plus
/// the offset times `scale` along its own unit axes; with no sender it places none
/// (its tag leaves the tag map, 0xd1b58). The tag has no input, so it is lit (0x12f6f4).
/// D1167: the receiver's second source 0x2a8538 is the online one: it reads offsets that
/// other consoles' senders packed (0x29ff3c) and returns nothing unless the network
/// context has bit 0x4000, so offline a receiver needs a sender in the level. D1168
/// (0x13187c, 0x131f88 at 0x132190): the sent offset's z lane is the tag's world z, not
/// a difference, and a receiver with useLayers (+0x2c) gives its tag that z; without it
/// the tag keeps the receiver's own z.
pub fn synced_tags(syncs: &[SyncSpot], tags: &[TagSpot]) -> Vec<TagSpot> {
    let sent: Vec<(&SyncSpot, [f32; 2], f32)> = syncs
        .iter()
        .filter(|y| y.kind == 0)
        .filter_map(|y| {
            let (lo, hi) = (y.min_radius * y.min_radius, y.radius * y.radius);
            tags.iter()
                .filter(|g| g.key == y.key && (g.body.is_none() || g.body != y.body))
                .map(|g| ([g.at[0] - y.at[0], g.at[1] - y.at[1]], g.z))
                .map(|(d, z)| (d, dot(d, d), z))
                .filter(|&(_, d2, _)| y.radius < 0. || (lo <= d2 && d2 <= hi))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(d, _, z)| (y, [dot(d, y.axes[0]), dot(d, y.axes[1])], z))
        })
        .collect();
    syncs
        .iter()
        .filter(|y| y.kind == 1)
        .filter_map(|y| {
            let (_, off, z) = sent
                .iter()
                .filter(|(s, _, _)| s.key == y.key)
                .min_by(|a, b| {
                    let d = |s: &SyncSpot| (s.at[0] - y.at[0]).hypot(s.at[1] - y.at[1]);
                    d(a.0).total_cmp(&d(b.0))
                })?;
            let (u, v) = (off[0] * y.scale[0], off[1] * y.scale[1]);
            Some(TagSpot {
                thing: y.thing,
                key: y.key,
                body: None,
                at: [
                    y.at[0] + u * y.axes[0][0] + v * y.axes[1][0],
                    y.at[1] + u * y.axes[0][1] + v * y.axes[1][1],
                ],
                z: if y.use_layers { *z } else { y.z },
            })
        })
        .collect()
}

/// D1110: each mover and follower moves its body's velocity towards its rows
/// ([`scene::MoveTweak::push`], per tick, in order) with the tweak's current activation; a
/// follower (configuration 11) follows `player` when it sees it. D1111: a follower with
/// followKey follows instead the nearest tag in `tags` with its key that it sees and that
/// is not on its own body (FOUND, 0x1c3e04; D1155: the caller passes [`lit_tags`]).
/// D1154: a follower measures from its parent's point ([`scene::MoveTweak::at`]). Wakes a body whose velocity changed.
pub fn drive_moves(
    commands: &mut Commands,
    q: &mut Query<(
        Entity,
        &MoveTweaks,
        &Position,
        &Rotation,
        &mut LinearVelocity,
    )>,
    player: Option<Vec2>,
    tags: &[TagSpot],
    activation: impl Fn(&scene::MoveTweak) -> f32,
    stick: impl Fn(&scene::MoveTweak) -> [f32; 2],
    scale: impl Fn(&scene::MoveTweak) -> f32,
) {
    // The rows are in game units per tick; the bodies move in sim units per second.
    let hz = TICK_HZ as f32;
    let per_tick = 1. / (hz * UNIT_SCALE);
    for (e, tweaks, pos, rot, mut v) in q.iter_mut() {
        let up = [-rot.sin, rot.cos];
        let before = [v.0.x * per_tick, v.0.y * per_tick];
        let after = tweaks.0.iter().fold(before, |vel, t| {
            // D1158: stick movers read their stick this tick.
            let t = &scene::MoveTweak {
                stick: stick(t),
                scale: scale(t),
                ..*t
            };
            // D1154: from the follower's parent, carried by the body.
            let me = [
                pos.0.x / UNIT_SCALE + rot.cos * t.at[0] - rot.sin * t.at[1],
                pos.0.y / UNIT_SCALE + rot.sin * t.at[0] + rot.cos * t.at[1],
            ];
            let d = match t.key {
                Some(k) => tags
                    .iter()
                    .filter(|g| g.key == k && g.body != Some(e))
                    .map(|g| [g.at[0] - me[0], g.at[1] - me[1]])
                    .filter(|&d| t.sees(d, up))
                    .min_by(|a, b| a[0].hypot(a[1]).total_cmp(&b[0].hypot(b[1]))),
                None => player
                    .map(|p| [p.x / UNIT_SCALE - me[0], p.y / UNIT_SCALE - me[1]])
                    .filter(|&d| t.configuration == 11 && t.sees(d, up)),
            };
            t.push(vel, up, d, activation(t))
        });
        if (after[0] - before[0]).abs() + (after[1] - before[1]).abs() > 1e-4 {
            v.0 = Vec2::new(after[0], after[1]) / per_tick;
            commands.queue(avian2d::dynamics::solver::islands::WakeBody(e));
        }
    }
}

/// D1102: a piston joint, kept so its length can follow the game's piston rule, with its
/// clock (D1103).
#[derive(Component, Debug, Clone)]
pub struct PistonDrive {
    pub joint: scene::SceneJoint,
    pub clock: scene::PistonClock,
    /// D1171: a motor's angle rule less the measured relative angle of its ends, taken
    /// when it first runs ([`Self::turn_to`]; the game's centre sync 0xc9af4).
    pub sync: Option<f32>,
}

impl PistonDrive {
    /// D1103: at tick `now` sets the clock rate, 1 with no input, else from the `input`
    /// signal (ternary, activation, whether its source is signed, D1170) by mode ([`scene::piston_scale`], D1104; mode 4
    /// [`scene::SceneJoint::servo_scale`], D1106), and returns the length (in sim units)
    /// from the clock.
    pub fn step(&mut self, now: f32, input: Option<(i32, f32, bool)>) -> f32 {
        let p = self.joint.params;
        let s = match input {
            None => 1.,
            Some(_) if p.held => 0.,
            Some((_, a, _)) if p.mode == 4 => {
                let l = self.joint.length(self.clock.frame(now), true);
                self.joint.servo_scale(a, l)
            }
            Some((t, a, signed)) => scene::piston_scale(p.mode, p.held, t, a, signed),
        };
        self.clock.set_scale(now, s);
        self.joint.length(self.clock.frame(now), input.is_some()) * UNIT_SCALE
    }

    /// D1108: steps the clock as [`Self::step`] does and returns the motor's turn rate in
    /// rad/s: the change of its angle ([`scene::SceneJoint::motor_angle`]) over the next
    /// frame of its clock, as the motor row builder 0x95f494 takes it (0xc99d0 at frame and
    /// frame + 1), times [`TICK_HZ`] (one game frame per tick). Pattern 1 gives speed (+0xd4,
    /// rad per frame) × clock rate, as 0xc7370 returns, in every mode (D1172: wired in mode
    /// 1 or 4 it no longer follows the length ramp); wired in mode 2, 0 once the clock's
    /// frame × |speed| passes 2π (0xc73dc to 0xc7410), so it stops after one turn.
    pub fn turn(&mut self, now: f32, input: Option<(i32, f32, bool)>) -> f32 {
        self.step(now, input);
        let wired = input.is_some();
        let f = self.clock.frame(now);
        let p = self.joint.params;
        if p.animation_pattern == 1 {
            let speed = p.motion[3];
            if wired && p.mode == 2 && f * speed.abs() > std::f32::consts::TAU {
                return 0.;
            }
            return speed * self.clock.scale * TICK_HZ as f32;
        }
        let a = |f| self.joint.motor_angle(f, wired);
        (a(f + self.clock.scale) - a(f)) * TICK_HZ as f32
    }

    /// D1171: FOUND (01.33, the motor row 0x95f494) the turn rate as [`Self::turn`] gives
    /// it, for a motor whose ends are at relative angle `measured` (body 2 less body 1,
    /// 0xc9250). A motor with pattern 1, or wired with mode 1 or 4, is driven at its rate
    /// (0x95fa44, 0xc7370). Any other takes the change of its angle over the next frame
    /// (`delta`) and the error from the measure to the angle at the next frame (`err`,
    /// both wrapped to [-π, π)), and turns by `delta` cut to between 0 and `err`
    /// (0x95fa28, 0x95fb10): it never runs past its target and never turns back. The
    /// angle rule is synced to the measure when it first runs (0xc9af4). The row's
    /// strength cap scales the motor's push ([`motor_damping`], D1173). The row's second
    /// edge, 1.5 × `err` with a cap of k × max(|err|, 0.1) ([`motor_band`], [`motor_k`],
    /// D1178), is applied by [`Self::band_to`] (D1179).
    pub fn turn_to(&mut self, now: f32, input: Option<(i32, f32, bool)>, measured: f32) -> f32 {
        match self.edges(now, input, measured) {
            Ok(w) => w,
            Err((delta, err)) => cut(delta, err) * TICK_HZ as f32,
        }
    }

    /// D1171: the motor's rate in rad/s as `Ok` when the row drives it at its rate
    /// (pattern 1, or wired in mode 1 or 4), else as `Err` its `delta` and `err` per frame.
    fn edges(
        &mut self,
        now: f32,
        input: Option<(i32, f32, bool)>,
        measured: f32,
    ) -> Result<f32, (f32, f32)> {
        let w = self.turn(now, input);
        let wired = input.is_some();
        let p = self.joint.params;
        if p.animation_pattern == 1 || (wired && matches!(p.mode, 1 | 4)) {
            return Ok(w);
        }
        let f = self.clock.frame(now);
        let a = |f| self.joint.motor_angle(f, wired);
        let sync = *self.sync.get_or_insert(a(f) - measured);
        let delta = wrap_pi(w / TICK_HZ as f32);
        Err((delta, wrap_pi(a(f + self.clock.scale) - sync - measured)))
    }

    /// D1179: the row's band ([`motor_band`], D1178) as the joint motor's target turn rate
    /// (rad/s) and push ([`MotorModel::AccelerationBased`] damping), for ends at relative
    /// angle `measured` turning at `rate` rad/s. Below the band the motor drives to its low
    /// edge, above it to its high edge, each with that edge's cap: the strength cap
    /// ([`motor_damping`]) on the cut `delta` edge, and on the 1.5 × `err` edge the k cap
    /// k × max(|err|, 0.1) scaled as the strength cap is (MOTOR_DAMPING × cap, at most
    /// MOTOR_DAMPING; INFERRED, the row is solved off the PPU). Inside the band it does not
    /// push. So a motor with k 0 (any pattern but 2) drives forward to its target but is
    /// never braked, and a pattern-2 motor is braked once it turns past 1.5 × `err`. A
    /// motor driven at its rate keeps [`motor_damping`] both ways.
    pub fn band_to(
        &mut self,
        now: f32,
        input: Option<(i32, f32, bool)>,
        measured: f32,
        rate: f32,
    ) -> (f32, f32) {
        let push = motor_damping(&self.joint.params);
        let (delta, err) = match self.edges(now, input, measured) {
            Ok(w) => return (w, push),
            Err(e) => e,
        };
        let hz = TICK_HZ as f32;
        let k = motor_k(&self.joint.params, input.is_some());
        let (lo, hi, cap) = motor_band(delta, err, k);
        let d = cut(delta, err);
        let brake = MOTOR_DAMPING * cap.min(1.);
        let at = |edge: f32| (edge * hz, if edge == d { push } else { brake });
        let r = rate / hz;
        if r < lo {
            at(lo)
        } else if r > hi {
            at(hi)
        } else {
            (d * hz, 0.)
        }
    }
}

impl PistonDrive {
    /// D1181: FOUND (01.33, the piston row 0x96178c) the row's band as the slider motor's
    /// target rate (sim units/s) and push ([`MotorModel::AccelerationBased`] damping), for
    /// ends `measured` game units apart along the slider (L, 0x961b30) moving apart at
    /// `rate` game units/s. Its `delta` is the change of the length rule over the next frame
    /// of the clock (0xc95ec at t and t + 1, 0x9619ac) and its `err` the length at the next
    /// frame less L (0x961d80). The low edge is `delta` cut to between 0 and `err` (any
    /// piston but one wired in mode 1 or 4, 0x961d88) and the other 1.5 × `err`, with a
    /// cap of k × max(|err|, 10) ([`piston_band`], D1180). As [`Self::band_to`] does, below
    /// the band it drives to the low edge and above it to the high edge, with the strength
    /// cap ([`motor_damping`]) on the `delta` edge and MOTOR_DAMPING × the k cap (at most
    /// MOTOR_DAMPING; INFERRED) on the `err` edge; inside the band it does not push.
    pub fn slide_to(
        &mut self,
        now: f32,
        input: Option<(i32, f32, bool)>,
        measured: f32,
        rate: f32,
    ) -> (f32, f32) {
        self.step(now, input);
        let wired = input.is_some();
        let p = self.joint.params;
        let f = self.clock.frame(now);
        let next = self.joint.length(f + self.clock.scale, wired);
        let delta = next - self.joint.length(f, wired);
        let err = next - measured;
        let k = motor_k(&p, wired);
        let (mut lo, mut hi, cap) = piston_band(delta, err, k);
        let mut d = cut(delta, err);
        if wired && matches!(p.mode, 1 | 4) {
            d = delta;
            (lo, hi) = (d.min(1.5 * err), d.max(1.5 * err));
        }
        let push = motor_damping(&p);
        let brake = MOTOR_DAMPING * cap.min(1.);
        let hz = TICK_HZ as f32;
        let at = |edge: f32| {
            let w = if edge == d { push } else { brake };
            (edge * hz * UNIT_SCALE, w)
        };
        let r = rate / hz;
        if r < lo {
            at(lo)
        } else if r > hi {
            at(hi)
        } else {
            (d * hz * UNIT_SCALE, 0.)
        }
    }
}

/// D1171: `delta` cut to between 0 and `err` (0x95fa28, 0x95fb10).
fn cut(delta: f32, err: f32) -> f32 {
    if delta > 0. {
        delta.min(err).max(0.)
    } else {
        delta.max(err).min(0.)
    }
}

/// D1171: `x` wrapped to [-π, π), as the motor row wraps its angles (0x95f688).
fn wrap_pi(x: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    x - ((x + PI) / TAU).floor() * TAU
}

/// D1181/D1182: whether a piston is driven by its band ([`PistonDrive::slide_to`]) rather
/// than held at its length. FOUND (01.33, D1182): the solver's piston case 0x1c9988 builds
/// the prismatic row 0x962590 (the band of [`PistonDrive::slide_to`]) only for a stiff
/// piston (PJoint +0x28, with the byte at +0x29 clear, 0x1c9998/0x1c99a4); any other
/// piston gets row 0x960988 (0x1c9b08) and an extra block (0x1c90b8). Both rows take the
/// strength cap 0xca760, clamp(strength, 0, 1) × a per-type scale, so strength 0 gives a
/// cap of 0. FOUND (D1183): row 0x960988 has the same band, edges `delta` and
/// 1.5 × `err` (0x960f98) with the k cap k × max(|err|, 10) (0x960ec0) and the strength
/// cap on `delta` (0x960f78); it adds the length limits f3..f4 eased by 0.2 past a slop
/// of 1 (0x96158c/0x961604), as the slider's limits here. So the flag does not change
/// the length drive. D1184: FOUND the block 0x1c90b8 builds, through the one-sided
/// contact-row builder 0x1c58d0, a row at the end of each body's angle less its offset
/// (+0x54 at 0x1c9510 always, +0x50 at 0x1c92ec unless it is 100, 0x1c96a8). INFERRED
/// (+0x50/+0x54 are aAngleOffset/bAngleOffset, `motion[5]`/`motion[6]`): those rows hold
/// a non-stiff piston near its saved angle on each body, so both kinds stay prismatic
/// (angle locked). INFERRED: a piston also needs a k ([`motor_k`], pattern 2) to
/// close the error from its saved length to its rule, which the band does not push
/// inside; without one it is held. D1185: FOUND the byte at +0x29 has one writer, the
/// setter 0xcacec, called only by the joint-editing gadget (0x5a6bb4, 0x55d720), and the
/// PJoint serialiser 0xb41850 never reads or writes it, so in play it is 0 and the stiff
/// test is +0x28 alone. D1186: FOUND those rows come from pool solver+0x10 (filler
/// 0x95ba80, 0x1c5e8c), frictionless (0x95bc58), with no strength term, bias 0.4 × the
/// separation past a slop of 1 (0x95bef4): hard positional holds of each anchor along a
/// direction turned by the saved angle, which the prismatic joint here matches.
/// D1188: FOUND (Part 88) the row's `delta` edge and its push do not depend on k, only
/// the 1.5 × `err` edge's cap does, and its limits are the rule's range from the saved
/// length. So a piston with a strength but no k (any pattern but 2, or wired in mode 1
/// or 4) also slides: it is pushed by its rule's change each frame within that range but,
/// with nothing to close the error, is not pulled back to the rule. Only strength 0 is
/// held. `wired` no longer matters here.
pub fn slides(params: &lbp_level::parts::JointParams, wired: bool) -> bool {
    let _ = wired;
    motor_damping(params) > 0.
}

/// D1103: at tick `now` steps each piston with the `signal` on its input and holds its
/// slider at the length, or (D1181, [`slides`]) drives its slider motor within the row's
/// band ([`PistonDrive::slide_to`]) between the limits of its length rule, waking both
/// ends when it changed. The ends' poses and velocities come from `bodies`.
pub fn drive_pistons(
    commands: &mut Commands,
    q: &mut Query<(&mut PistonDrive, &mut PrismaticJoint)>,
    bodies: &Query<(&Position, &Rotation, &LinearVelocity)>,
    now: f32,
    signal: impl Fn(&scene::SceneJoint) -> Option<(i32, f32, bool)>,
) {
    for (mut drive, mut joint) in q.iter_mut() {
        let input = signal(&drive.joint);
        if !slides(&drive.joint.params, input.is_some()) {
            let l = drive.step(now, input);
            if joint.limits.is_some_and(|d| d.min == l && d.max == l) {
                continue;
            }
            joint.limits = Some(DistanceLimit::new(l, l));
            wake_ends(commands, joint.body1, joint.body2);
            continue;
        }
        let at = |e| bodies.get(e).ok();
        let (Some((p1, r1, v1)), Some((p2, r2, v2))) = (at(joint.body1), at(joint.body2)) else {
            continue;
        };
        let a1 = *r1 * joint.local_anchor1().unwrap_or_default();
        let a2 = *r2 * joint.local_anchor2().unwrap_or_default();
        let axis = *r1 * joint.slider_axis;
        let l = (p2.0 + a2 - p1.0 - a1).dot(axis) / UNIT_SCALE;
        let rate = (v2.0 - v1.0).dot(axis) / UNIT_SCALE;
        let (w, damping) = drive.slide_to(now, input, l, rate);
        let model = MotorModel::AccelerationBased {
            stiffness: 0.,
            damping,
        };
        if joint.motor.enabled
            && joint.motor.target_velocity == w
            && joint.motor.motor_model == model
        {
            continue;
        }
        joint.limits = Some(drive.joint.range());
        joint.motor.enabled = true;
        joint.motor.target_velocity = w;
        joint.motor.motor_model = model;
        wake_ends(commands, joint.body1, joint.body2);
    }
}

/// D1107: winches (CHAIN, type 3) run the piston's clock and length rule (0xc95ec sends
/// types 3, 4 and 9 to 0xc9790, the solver gives types 3 and 4 the same limit rows), so
/// at tick `now` each one is stepped like a piston and its rope held to at most the length.
/// The solver builds the winch row (0x960988) with a flag (set at 0x1c9964) that the piston
/// and rod rows clear; INFERRED from that, the rope only pulls and goes slack when shorter.
pub fn drive_winches(
    commands: &mut Commands,
    q: &mut Query<(&mut PistonDrive, &mut DistanceJoint)>,
    now: f32,
    signal: impl Fn(&scene::SceneJoint) -> Option<(i32, f32, bool)>,
) {
    for (mut drive, mut joint) in q.iter_mut() {
        let input = signal(&drive.joint);
        let l = drive.step(now, input);
        if joint.limits.min == 0. && joint.limits.max == l {
            continue;
        }
        joint.limits = DistanceLimit::new(0., l);
        wake_ends(commands, joint.body1, joint.body2);
    }
}

/// D1108: at tick `now` sets each motor's (MOTOR, type 9) target turn rate from its clock
/// and angle rule, cut so it never runs past its target ([`PistonDrive::turn_to`], D1171),
/// within the row's band ([`PistonDrive::band_to`], D1179), waking both ends when it
/// changed. The ends' relative angle and turn rate come from `bodies`.
pub fn drive_motors(
    commands: &mut Commands,
    q: &mut Query<(&mut PistonDrive, &mut RevoluteJoint)>,
    bodies: &Query<(&Rotation, &AngularVelocity)>,
    now: f32,
    signal: impl Fn(&scene::SceneJoint) -> Option<(i32, f32, bool)>,
) {
    for (mut drive, mut joint) in q.iter_mut() {
        let input = signal(&drive.joint);
        let at = |e| {
            bodies
                .get(e)
                .map_or((0., 0.), |(r, w)| (r.as_radians(), w.0))
        };
        let ((a1, w1), (a2, w2)) = (at(joint.body1), at(joint.body2));
        let (w, damping) = drive.band_to(now, input, a2 - a1, w2 - w1);
        let model = MotorModel::AccelerationBased {
            stiffness: 0.,
            damping,
        };
        if joint.motor.target_velocity == w && joint.motor.motor_model == model {
            continue;
        }
        joint.motor.target_velocity = w;
        joint.motor.motor_model = model;
        wake_ends(commands, joint.body1, joint.body2);
    }
}

fn wake_ends(commands: &mut Commands, a: Entity, b: Entity) {
    for e in [a, b] {
        commands.queue(move |world: &mut World| wake_if_island(world, e));
    }
}

/// Wakes a body's island; a static anchor (no island) is left alone.
fn wake_if_island(world: &mut World, e: Entity) {
    use avian2d::dynamics::solver::islands::{BodyIslandNode, WakeBody};
    if world.get::<BodyIslandNode>(e).is_some() {
        let _ = WakeBody(e).apply(world);
    }
}

/// A body that starts asleep (PBody.frozen != 0, INFERRED: a saved level is at rest). avian
/// builds islands during the first step, so after it the body goes back to its stored pose
/// with no velocity and its island is put to sleep; contacts and joints wake it.
#[derive(Component, Debug, Clone, Copy)]
pub struct StartAsleep {
    position: Vector,
    angle: f32,
}

fn put_to_sleep(
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &StartAsleep,
        &mut Position,
        &mut Rotation,
        &mut LinearVelocity,
        &mut AngularVelocity,
    )>,
) {
    for (e, start, mut p, mut r, mut v, mut w) in &mut q {
        // Without change detection: a changed pose or velocity would wake the island again.
        p.bypass_change_detection().0 = start.position;
        *r.bypass_change_detection() = Rotation::radians(start.angle);
        v.bypass_change_detection().0 = Vector::ZERO;
        w.bypass_change_detection().0 = 0.;
        commands.entity(e).remove::<StartAsleep>();
        commands.queue(avian2d::dynamics::solver::islands::SleepBody(e));
    }
}

fn transform(frame: &scene::Frame) -> Transform {
    let p = sv(frame.position);
    Transform::from_xyz(p.x, p.y, 0.).with_rotation(Quat::from_rotation_z(frame.angle))
}

/// One convex hull when the shape is convex (hull area equals the triangle area), else a
/// compound of its triangles. A single hull has no inner edges for contacts to catch on.
fn collider(c: &scene::SceneCollider) -> Collider {
    let area: f32 = c.triangles.iter().map(scene::area).sum::<f32>() * UNIT_SCALE * UNIT_SCALE;
    let points: Vec<Vector> = c.triangles.iter().flatten().map(|p| sv(*p)).collect();
    if let Some(hull) = Collider::convex_hull(points) {
        let hull_area = hull.mass_properties(1.).mass;
        if (hull_area - area).abs() <= 1e-3 * area {
            return hull;
        }
    }
    Collider::compound(
        c.triangles
            .iter()
            .map(|[a, b, t]| {
                (
                    Position::default(),
                    Rotation::default(),
                    Collider::triangle(sv(*a), sv(*b), sv(*t)),
                )
            })
            .collect(),
    )
}

/// Entities [`spawn_scene`] made, in scene order.
#[derive(Debug, Clone, Default)]
pub struct Spawned {
    pub bodies: Vec<Entity>,
    pub colliders: Vec<Entity>,
    pub joints: Vec<Entity>,
}

/// Spawns the scene: a dynamic body per [`scene::SceneBody`] with its stored velocities,
/// a collider entity per shape (child of its body, or a static body of its own), and a
/// joint entity per [`scene::SceneJoint`].
pub fn spawn_scene(commands: &mut Commands, scene: &PhysicsScene) -> Spawned {
    let bodies: Vec<Entity> = scene
        .bodies
        .iter()
        .map(|b| {
            let mut e = commands.spawn((
                ThingIndex(b.thing),
                if b.fixed {
                    RigidBody::Static
                } else {
                    RigidBody::Dynamic
                },
                transform(&b.frame),
                LinearVelocity(sv(b.velocity)),
                AngularVelocity(b.angular_velocity),
            ));
            if let Some(a) = &b.anti_gravity {
                // FOUND (01.33, 0xc6308): gravity scale clamp(1 - Σ, 0, 1).
                e.insert((
                    GravityScale(a.gravity_scale(|t| t.saved)),
                    AntiGravityTweaks(a.clone()),
                ));
            }
            if !b.spins.is_empty() {
                e.insert(SpinTweaks(b.spins.clone()));
            }
            if !b.moves.is_empty() {
                e.insert(MoveTweaks(b.moves.clone()));
            }
            if !b.fixed && b.frozen != 0 {
                e.insert(StartAsleep {
                    position: sv(b.frame.position),
                    angle: b.frame.angle,
                });
            }
            e.id()
        })
        .collect();
    // D1111: tags ride their bodies; tags on no body share one entity.
    let mut marks = vec![Vec::new(); bodies.len() + 1];
    for g in &scene.tags {
        marks[g.body.unwrap_or(bodies.len())].push((g.thing as u32, g.key, g.at, g.z));
    }
    let loose = marks.pop().unwrap_or_default();
    for (b, m) in marks.into_iter().enumerate().filter(|(_, m)| !m.is_empty()) {
        commands.entity(bodies[b]).insert(TagMarks(m));
    }
    if !loose.is_empty() {
        commands.spawn(TagMarks(loose));
    }
    // D1157: synchronisers ride their bodies the same way.
    let mut syncs = vec![Vec::new(); bodies.len() + 1];
    for y in &scene.syncs {
        syncs[y.body.unwrap_or(bodies.len())].push(*y);
    }
    let loose = syncs.pop().unwrap_or_default();
    for (b, m) in syncs.into_iter().enumerate().filter(|(_, m)| !m.is_empty()) {
        commands.entity(bodies[b]).insert(SyncMarks(m));
    }
    if !loose.is_empty() {
        commands.spawn(SyncMarks(loose));
    }
    let mut colliders = Vec::with_capacity(scene.colliders.len());
    for c in &scene.colliders {
        let shape = (
            ThingIndex(c.thing),
            collider(c),
            ColliderDensity(c.density),
            Friction::new(scene::FRICTION),
            Restitution::new(scene::RESTITUTION),
            ZSlab {
                z: c.z,
                half_thickness: c.half_thickness,
            },
            ActiveCollisionHooks::FILTER_PAIRS,
        );
        // Placement goes in the spawn bundle: avian takes the initial position from it.
        colliders.push(
            match c.body {
                Some(b) => commands.spawn((shape, Transform::IDENTITY, ChildOf(bodies[b]))),
                None => commands.spawn((shape, RigidBody::Static, transform(&c.frame))),
            }
            .id(),
        );
    }
    let joints = scene
        .joints
        .iter()
        .map(|j| spawn_joint(commands, j, &bodies, &colliders))
        .collect();
    Spawned {
        bodies,
        colliders,
        joints,
    }
}

/// One joint entity. Jointed pieces do not collide with each other (INFERRED: a bolted
/// wheel overlapping its frame turns freely). Angular bases are global so the rest angle
/// is the stored pose.
fn spawn_joint(
    commands: &mut Commands,
    j: &scene::SceneJoint,
    bodies: &[Entity],
    colliders: &[Entity],
) -> Entity {
    let mut entity = |e: scene::Endpoint| match e {
        scene::Endpoint::Body(i) => bodies[i],
        scene::Endpoint::Static(i) => colliders[i],
        scene::Endpoint::Anchor { thing, frame } => commands
            .spawn((ThingIndex(thing), RigidBody::Static, transform(&frame)))
            .id(),
    };
    let (a, b) = (entity(j.a), entity(j.b));
    let v = sv;
    let pivot = || {
        RevoluteJoint::new(a, b)
            .with_anchor(v(j.world_a))
            .with_basis(Rotation::IDENTITY)
    };
    let distance = |min: f32, max: f32, compliance: f32| {
        let (min, max) = (min * UNIT_SCALE, max * UNIT_SCALE);
        DistanceJoint::new(a, b)
            .with_local_anchor1(v(j.local_a))
            .with_local_anchor2(v(j.local_b))
            .with_limits(min, max)
            .with_compliance(compliance)
    };
    let mut e = commands.spawn((ThingIndex(j.thing), JointCollisionDisabled));
    match j.kind {
        scene::JointKind::Bolt => e.insert(pivot()),
        scene::JointKind::Motor => {
            let mut drive = PistonDrive {
                joint: j.clone(),
                clock: scene::PistonClock::default(),
                sync: None,
            };
            let w = drive.turn(0., None);
            let start = j.motor_angle(drive.clock.frame(0.), false);
            let mut joint = pivot();
            if let Some((min, max)) = motor_angle_limits(&j.params, start) {
                joint = joint.with_angle_limits(min, max);
            }
            e.insert(
                joint.with_motor(
                    AngularMotor::new(MotorModel::AccelerationBased {
                        stiffness: 0.,
                        damping: motor_damping(&j.params),
                    })
                    .with_target_velocity(w),
                ),
            )
            .insert(drive)
        }
        scene::JointKind::SpringAngular => e.insert(
            pivot().with_motor(
                AngularMotor::new(MotorModel::SpringDamper {
                    frequency: SPRING_FREQUENCY,
                    damping_ratio: SPRING_DAMPING_RATIO,
                })
                .with_target_position(0.),
            ),
        ),
        scene::JointKind::Piston => {
            // D1169: the game's piston row (0x96178c, 0x962590) also locks the relative
            // angle its ends had when it first ran (0xcab7c syncs +0xe0). A global basis
            // keeps the spawn relative angle, which matches it.
            let d = [j.world_b[0] - j.world_a[0], j.world_b[1] - j.world_a[1]];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
            let axis = if len > 1e-3 {
                Vector::new(d[0] / len, d[1] / len)
            } else {
                Vector::Y
            };
            e.insert(
                PrismaticJoint::new(a, b)
                    .with_local_anchor1(v(j.local_a))
                    .with_local_anchor2(v(j.local_b))
                    .with_basis(Rotation::IDENTITY)
                    .with_slider_axis(axis)
                    .with_limits(j.min_length * UNIT_SCALE, j.max_length * UNIT_SCALE),
            )
            .insert(PistonDrive {
                joint: j.clone(),
                clock: scene::PistonClock::default(),
                sync: None,
            })
        }
        scene::JointKind::Rod => e.insert(distance(j.length, j.length, 0.)),
        scene::JointKind::Chain => {
            let l = j.length(0., false);
            e.insert(distance(0., l, 0.)).insert(PistonDrive {
                joint: j.clone(),
                clock: scene::PistonClock::default(),
                sync: None,
            })
        }
        scene::JointKind::String => e.insert(distance(0., j.length, 0.)),
        scene::JointKind::Elastic => e.insert(distance(0., j.length, SPRING_COMPLIANCE)),
        scene::JointKind::Spring => e.insert(distance(j.length, j.length, SPRING_COMPLIANCE)),
    };
    e.id()
}
