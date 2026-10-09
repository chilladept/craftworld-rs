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
pub mod scene;

pub use scene::{build_scene, build_scene_with, PhysicsScene};

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

/// A physics position in LBP world units.
pub fn to_level(p: Vector) -> [f32; 2] {
    [p.x / UNIT_SCALE, p.y / UNIT_SCALE]
}

fn sv(p: [f32; 2]) -> Vector {
    Vector::new(p[0] * UNIT_SCALE, p[1] * UNIT_SCALE)
}

/// MOTOR joint target angular velocity (rad/s) per unit of the stored speed. PLACEHOLDER:
/// the unit of PJoint speed is UNVERIFIED.
pub const MOTOR_SPEED_SCALE: f32 = 1.0;
/// Joint motors: share of the speed error removed per substep (avian `AccelerationBased`
/// damping; above 1 it overshoots). Our choice; the motor strength is not modelled.
pub const MOTOR_DAMPING: f32 = 0.5;
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

impl ZSlab {
    /// Slabs collide when their depth ranges overlap (touching is not overlapping).
    pub fn overlaps(&self, other: &ZSlab) -> bool {
        (self.z - other.z).abs() < self.half_thickness + other.half_thickness
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
        app.add_plugins(PhysicsPlugins::default().with_collision_hooks::<SlabHooks>())
            .insert_resource(Time::<Fixed>::from_hz(TICK_HZ))
            .insert_resource(SubstepCount(SUBSTEPS))
            .insert_resource(Gravity(Vector::new(0., -self.gravity * UNIT_SCALE)))
            .add_systems(
                FixedPostUpdate,
                put_to_sleep
                    .after(PhysicsSystems::StepSimulation)
                    .before(PhysicsSystems::Writeback),
            );
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
            if !b.fixed && b.frozen != 0 {
                e.insert(StartAsleep {
                    position: sv(b.frame.position),
                    angle: b.frame.angle,
                });
            }
            e.id()
        })
        .collect();
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
    let entity = |e: scene::Endpoint| match e {
        scene::Endpoint::Body(i) => bodies[i],
        scene::Endpoint::Static(i) => colliders[i],
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
        scene::JointKind::Motor => e.insert(
            pivot().with_motor(
                AngularMotor::new(MotorModel::AccelerationBased {
                    stiffness: 0.,
                    damping: MOTOR_DAMPING,
                })
                .with_target_velocity(j.params.motion[3] * MOTOR_SPEED_SCALE),
            ),
        ),
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
        }
        scene::JointKind::Rod | scene::JointKind::Chain => {
            e.insert(distance(j.length, j.length, 0.))
        }
        scene::JointKind::String => e.insert(distance(0., j.length, 0.)),
        scene::JointKind::Elastic => e.insert(distance(0., j.length, SPRING_COMPLIANCE)),
        scene::JointKind::Spring => e.insert(distance(j.length, j.length, SPRING_COMPLIANCE)),
    };
    e.id()
}
