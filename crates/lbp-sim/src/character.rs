//! Sackboy in the avian world: a kinematic capsule driven by [`crate::sackboy`] once per game
//! tick. Ground, walls, ceilings and lane floors come from shape casts against colliders whose
//! z-slab overlaps Sackboy's lane; the controller turns them and the pad state into a move,
//! which is applied axis by axis and stopped at the first hit (move and stop, no slide along
//! slopes yet).
use crate::sackboy::{Contact, Input, Sackboy};
use crate::{sv, ZSlab, UNIT_SCALE};
use avian2d::math::Vector;
use avian2d::prelude::*;
use bevy::prelude::*;

/// Capsule radius and half segment in world units. INFERRED: Sackboy's position rests 69
/// units above the floor in Rookie Test (floor y 5905, standing y 5973.8), so the capsule
/// spans 69 below its centre; the width is a PLACEHOLDER.
pub const RADIUS: f32 = 35.0;
pub const HALF_SEGMENT: f32 = 34.0;
/// Half depth of Sackboy's slab: thin, so he only meets the lane he is in. PLACEHOLDER.
pub const HALF_THICKNESS: f32 = 5.0;
/// Gap kept from surfaces after a move, and how far below the feet counts as standing.
pub const SKIN: f32 = 0.5;
pub const GROUND_PROBE: f32 = 2.0;
/// How far below Sackboy a lane must have floor for a lane change.
pub const LANE_PROBE: f32 = 400.0;

/// Sackboy's movement state and this tick's pad input (set it before `FixedUpdate`).
#[derive(Component, Debug, Clone, Default)]
pub struct SackboyBody {
    pub controller: Sackboy,
    pub input: Input,
    pub grounded: bool,
    /// Position computed this tick (physics units), written by `apply_moves`.
    next: Option<Vector>,
}

/// Spawns Sackboy standing at `position` (world units, capsule centre) in lane `z`.
pub fn spawn_sackboy(commands: &mut Commands, position: [f32; 2], z: f32) -> Entity {
    let p = sv(position);
    commands
        .spawn((
            SackboyBody {
                controller: Sackboy::at_lane(z),
                ..Default::default()
            },
            RigidBody::Kinematic,
            Collider::capsule(RADIUS * UNIT_SCALE, 2. * HALF_SEGMENT * UNIT_SCALE),
            ZSlab {
                z,
                half_thickness: HALF_THICKNESS,
            },
            Transform::from_xyz(p.x, p.y, 0.),
        ))
        .id()
}

/// Adds the per-tick Sackboy system.
pub struct SackboyPlugin;

impl Plugin for SackboyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (drive_sackboy, apply_moves).chain());
    }
}

fn drive_sackboy(
    spatial: SpatialQuery,
    slabs: Query<&ZSlab, Without<SackboyBody>>,
    mut q: Query<(Entity, &mut SackboyBody, &Position, &mut ZSlab, &Collider)>,
) {
    for (me, mut body, pos, mut slab, shape) in &mut q {
        let mut at = pos.0;
        let filter = SpatialQueryFilter::default().with_excluded_entities([me]);
        let in_lane = |z: f32| {
            let lane = ZSlab {
                z,
                half_thickness: HALF_THICKNESS,
            };
            move |e: Entity| slabs.get(e).is_ok_and(|s| s.overlaps(&lane))
        };
        // Distance (world units) to the first collider in lane `z` along `dir`, up to `max`.
        // Distance (world units) to the nearest surface in lane `z` that faces against `dir`
        // (a floor under a walking capsule touches it but does not block a sideways move).
        let cast = |origin: Vector, dir: Dir2, max: f32, z: f32| {
            let lane = in_lane(z);
            let mut best: Option<f32> = None;
            spatial.shape_hits_callback(
                shape,
                origin,
                0.,
                dir,
                &ShapeCastConfig::from_max_distance(max * UNIT_SCALE),
                &filter,
                |h| {
                    if lane(h.entity) && h.normal1.dot(Vector::from(dir)) < -0.5 {
                        let d = h.distance / UNIT_SCALE;
                        best = Some(best.map_or(d, |b: f32| b.min(d)));
                    }
                    true
                },
            );
            best
        };
        let lane_z = body.controller.lane;
        let grounded = body.controller.vy <= 0.
            && cast(at, Dir2::NEG_Y, GROUND_PROBE + SKIN, lane_z).is_some();
        let here = at;
        let has_floor = |z: f32| cast(here, Dir2::NEG_Y, LANE_PROBE, z).is_some();
        let input = body.input;
        let d = body.controller.step(
            input,
            &Contact {
                grounded,
                lane_has_floor: &has_floor,
            },
        );
        body.grounded = grounded;
        let lane_z = body.controller.lane;
        // x, then y, each stopped at the first hit.
        for (axis, delta) in [(0, d[0]), (1, d[1])] {
            if delta == 0. {
                continue;
            }
            let dir = match (axis, delta > 0.) {
                (0, true) => Dir2::X,
                (0, false) => Dir2::NEG_X,
                (_, true) => Dir2::Y,
                (_, false) => Dir2::NEG_Y,
            };
            let mut step = delta.abs();
            if let Some(hit) = cast(at, dir, step + SKIN, lane_z) {
                step = (hit - SKIN).max(0.);
                if axis == 0 {
                    body.controller.vx = 0.;
                } else {
                    body.controller.vy = 0.;
                }
            }
            at += Vector::from(dir) * step * UNIT_SCALE;
        }
        body.next = Some(at);
        slab.z = body.controller.z;
    }
}

/// Writes the positions `drive_sackboy` computed (spatial queries read every Position, so
/// the move is applied in a second system).
fn apply_moves(mut q: Query<(&mut SackboyBody, &mut Position)>) {
    for (mut body, mut pos) in &mut q {
        if let Some(p) = body.next.take() {
            pos.0 = p;
        }
    }
}
