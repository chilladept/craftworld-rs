//! Bevy-free physics description of a read level: which SHAPE Things collide, which move,
//! their 2D frames, triangles and z-slabs. [`crate::spawn_scene`] turns it into avian2d
//! entities.
//!
//! Rules (labels from docs/formats.md; evidence in docs/runtime-evidence/physics.md and
//! docs/sim-physics.md):
//! - A Thing with BODY is a rigid body at its POS frame (INFERRED: PBody holds the
//!   velocities, so it is the moving piece).
//! - A SHAPE Thing belongs to the topmost Thing with BODY on its parent chain, itself
//!   included (INFERRED, A1: pieces glued in one parent tree form one body; a BODY under
//!   another BODY is counted as `nested_bodies`). A SHAPE with no BODY on the chain is
//!   static.
//! - A body is fixed (static) when its owner is the level's WORLD Thing, or when any of its
//!   shapes uses a physical material the caller marks fixed (PUBLIC behaviour: dark matter
//!   is immovable and anything glued to it stays put; which descriptor is dark matter comes
//!   from the caller, see [`build_scene_with`]). INFERRED until a PC run confirms it.
//! - A shape whose material rule is [`MaterialRule::NoCollide`] (hologram) is not a collider
//!   (PUBLIC behaviour: hologram has no physical presence; INFERRED mapping).
//! - A body with GAMEPLAY_DATA (score bubbles, pick-ups) is fixed: it is collected through
//!   its trigger, not pushed (INFERRED; the Rookie run had 71 of them falling).
//! - A dynamic body with PBody.frozen != 0 starts asleep and wakes when touched or pulled
//!   by a joint (INFERRED: every dynamic body in the Rookie run has frozen 1, so a saved
//!   level is at rest; see `crate::spawn_scene`).
//! - Collision is 2D; a shape's slab is world z ± thickness (INFERRED, A3/A4).
//! - A shape without the COLLIDABLE_GAME flag (0x1) does not collide (COMMUNITY, ShapeFlags).
//! - PJoints join the entities their endpoint Things belong to (a body, or a static
//!   shape); contact points are in each endpoint Thing's POS frame (COMMUNITY field
//!   names; frame INFERRED). The type mapping is [`JointKind`].
//! - Mass = area × massDepth × density (INFERRED, A2); density is [`DENSITY`] until RMaterial
//!   is read (PLACEHOLDER).
use lbp_level::serial::Descriptor;
use lbp_level::shape_mesh::outline_triangles_lenient;
use lbp_level::thing::ThingRecord;
use std::collections::BTreeMap;

/// RMaterial default density (Toolkit RMaterial field default, COMMUNITY). Every shape uses
/// it until physical materials are read: PLACEHOLDER.
pub const DENSITY: f32 = 20.0;
/// RMaterial default sliding friction (COMMUNITY default). PLACEHOLDER for every shape.
pub const FRICTION: f32 = 0.8;
/// RMaterial default restitution (COMMUNITY default). PLACEHOLDER for every shape.
pub const RESTITUTION: f32 = 0.3;
/// ShapeFlags.COLLIDABLE_GAME.
pub const COLLIDABLE_GAME: i16 = 0x1;
/// Parent-chain depth limit (untrusted references may cycle).
pub const MAX_PARENT_DEPTH: usize = 256;
/// Triangles below this area (world units²) after the transform are dropped.
pub const MIN_TRIANGLE_AREA: f32 = 1e-4;
/// World triangles and the z slab (centre, half depth) of a [`sideways`] shape.
type Sideways = (Vec<[[f32; 2]; 3]>, (f32, f32));

/// Side view of a SHAPE whose world matrix flattens its outline plane in xy: the outline
/// extruded over local z in [-t, t], projected onto world xy. For a connected outline that
/// projects to a line this is exactly the convex hull of the projected prism corners (a
/// parallelogram). Returns CCW world triangles and the world z slab (centre, half depth)
/// the outline spans. INFERRED from d198 samples (Things rotated 90 degrees about x in DLC
/// levels): the game evidently collides with them, as Sackboy stands on such floors.
fn sideways(m: &[f32; 16], points: &[[f32; 2]], t: f32) -> Option<Sideways> {
    let scale = m[0].abs().max(m[1].abs()).max(m[4].abs()).max(m[5].abs());
    let det = m[0] * m[5] - m[1] * m[4];
    if det.abs() > 1e-6 * scale * scale || t <= 0. {
        return None;
    }
    let mut corners = Vec::with_capacity(points.len() * 2);
    let (mut zlo, mut zhi) = (f32::MAX, f32::MIN);
    for p in points {
        for z in [-t, t] {
            let w = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * z + m[12 + r];
            corners.push([w(0), w(1)]);
            let wz = w(2);
            zlo = zlo.min(wz);
            zhi = zhi.max(wz);
        }
    }
    let hull = convex_hull(corners);
    let mut tris = Vec::new();
    for k in 1..hull.len().saturating_sub(1) {
        let tri = [hull[0], hull[k], hull[k + 1]];
        if area(&tri) >= MIN_TRIANGLE_AREA {
            tris.push(tri);
        }
    }
    let slab = (0.5 * (zlo + zhi), 0.5 * (zhi - zlo));
    (!tris.is_empty() && slab.0.is_finite() && slab.1.is_finite()).then_some((tris, slab))
}

/// Counter-clockwise convex hull (monotone chain), collinear points dropped.
fn convex_hull(mut p: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    p.retain(|q| q[0].is_finite() && q[1].is_finite());
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: [f32; 2], a: [f32; 2], b: [f32; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut h: Vec<[f32; 2]> = Vec::with_capacity(2 * p.len());
    for pass in [p.clone(), p.into_iter().rev().collect()] {
        let start = h.len();
        for q in pass {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0. {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
    }
    h
}

/// Flat-transform samples kept per scene (diagnostics).
pub const FLAT_SAMPLES: usize = 4;

/// Signed area of a triangle (positive when counter-clockwise).
pub fn area([a, b, c]: &[[f32; 2]; 3]) -> f32 {
    0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))
}

/// A 2D frame: world position and rotation about z (radians).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    pub position: [f32; 2],
    pub angle: f32,
}

impl Frame {
    /// Frame of a POS world matrix (column-major, translation at 12..14): translation xy
    /// and the angle of the x axis. Scale and shear stay in the collider points.
    pub fn of_matrix(m: &[f32; 16]) -> Option<Self> {
        if m.iter().any(|v| !v.is_finite()) || (m[0] == 0. && m[1] == 0.) {
            return None;
        }
        Some(Self {
            position: [m[12], m[13]],
            angle: m[1].atan2(m[0]),
        })
    }

    /// World point to this frame's local coordinates.
    pub fn to_local(&self, p: [f32; 2]) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        let d = [p[0] - self.position[0], p[1] - self.position[1]];
        [c * d[0] + s * d[1], -s * d[0] + c * d[1]]
    }
}

/// A rigid body: the BODY Thing's frame and stored velocities.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneBody {
    pub thing: usize,
    pub frame: Frame,
    pub velocity: [f32; 2],
    pub angular_velocity: f32,
    /// PBody.frozen as stored (meaning UNVERIFIED; not used yet).
    pub frozen: i32,
    /// Static body: WORLD owner or a fixed material in the glue tree (see module rules).
    pub fixed: bool,
}

/// One SHAPE Thing as a collider.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneCollider {
    pub thing: usize,
    /// Index in [`PhysicsScene::bodies`]; None = static.
    pub body: Option<usize>,
    /// Frame the triangles are in: the body's frame, or the shape's own for a static one.
    pub frame: Frame,
    /// Counter-clockwise triangles in `frame` coordinates.
    pub triangles: Vec<[[f32; 2]; 3]>,
    /// Slab centre (world z) and half depth (PShape.thickness).
    pub z: f32,
    pub half_thickness: f32,
    /// Mass per unit area: massDepth × [`DENSITY`].
    pub density: f32,
    /// Built by [`sideways`] (a shape turned on its side).
    pub sideways: bool,
}

/// Counts for the PC report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneReport {
    pub shapes: usize,
    pub dynamic: usize,
    pub static_: usize,
    pub not_collidable: usize,
    /// Bodies made static (counted in `static_` by shape, here by body).
    pub fixed_bodies: usize,
    /// BODY Things with another BODY above them on the parent chain (merged into it).
    pub nested_bodies: usize,
    /// Shapes left out, by reason.
    pub rejected: BTreeMap<&'static str, usize>,
    /// Shapes kept by the even-odd fill, by the reason the strict outline refused them.
    pub filled: BTreeMap<&'static str, usize>,
    /// Up to [`FLAT_SAMPLES`] flat-transform shapes: Thing and the 3x3 linear part of the
    /// world matrix (columns x, y, z).
    pub flat_samples: Vec<(usize, [f32; 9])>,
    pub joints: usize,
    /// PJoints left out, by reason.
    pub rejected_joints: BTreeMap<&'static str, usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhysicsScene {
    pub bodies: Vec<SceneBody>,
    pub colliders: Vec<SceneCollider>,
    pub joints: Vec<SceneJoint>,
    pub report: SceneReport,
}

/// Nearest Thing with BODY on the parent chain of `i`, itself included.
/// Topmost Thing with BODY on the parent chain of `i` (itself included), and the BODY
/// Things below it on the chain.
fn body_owner(things: &[ThingRecord], mut i: usize) -> (Option<usize>, Vec<usize>) {
    let mut found = Vec::new();
    for _ in 0..MAX_PARENT_DEPTH {
        let Some(t) = things.get(i) else {
            break;
        };
        if t.data.body.is_some() {
            found.push(i);
        }
        match t.parent {
            Some(p) if p != i => i = p,
            _ => break,
        }
    }
    let top = found.pop();
    (top, found)
}

fn is_world(t: &ThingRecord) -> bool {
    t.data.world.is_some() || t.parts.contains(&"WORLD")
}

/// Score bubbles and other pick-ups (GAMEPLAY_DATA) hang where they are placed.
fn is_pickup(t: &ThingRecord) -> bool {
    t.parts.contains(&"GAMEPLAY_DATA")
}

/// How a physical material behaves in the sandbox (the caller decides per descriptor until
/// RMaterial is read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialRule {
    #[default]
    Normal,
    /// Immovable: the whole glue tree is static (dark matter, infinite mass).
    Fixed,
    /// No physical presence: the shape is not a collider (hologram).
    NoCollide,
}

/// Builds the physics description of a level (`things[i]` must be Thing index `i`) with every
/// material normal: only WORLD-owned and pick-up bodies are static.
pub fn build_scene(things: &[ThingRecord]) -> PhysicsScene {
    build_scene_with(things, &|_| MaterialRule::Normal)
}

/// Builds the physics description of a level with a rule per physical material.
pub fn build_scene_with(
    things: &[ThingRecord],
    material_rule: &dyn Fn(&Descriptor) -> MaterialRule,
) -> PhysicsScene {
    let mut scene = PhysicsScene::default();
    let mut body_of = BTreeMap::<usize, Option<usize>>::new();
    let mut nested = std::collections::BTreeSet::new();
    let mut fixed_shape = Vec::new();
    for (i, t) in things.iter().enumerate() {
        let Some(shape) = &t.data.shape else {
            continue;
        };
        scene.report.shapes += 1;
        let mut reject = |why| *scene.report.rejected.entry(why).or_default() += 1;
        let rule = shape
            .material
            .as_ref()
            .map_or(MaterialRule::Normal, material_rule);
        if shape.flags & COLLIDABLE_GAME == 0 || rule == MaterialRule::NoCollide {
            scene.report.not_collidable += 1;
            continue;
        }
        let Some(pos) = &t.data.pos else {
            reject("no POS");
            continue;
        };
        let Some(own) = Frame::of_matrix(&pos.world) else {
            reject("bad transform");
            continue;
        };
        let outline = match outline_triangles_lenient(shape) {
            Ok((v, why)) => {
                if let Some(why) = why {
                    *scene.report.filled.entry(why.reason()).or_default() += 1;
                }
                v
            }
            Err(e) => {
                reject(e.reason());
                continue;
            }
        };
        let (owner, below) = body_owner(things, i);
        nested.extend(below);
        let body = match owner {
            None => None,
            Some(b) => {
                let slot = *body_of.entry(b).or_insert_with(|| {
                    let thing = &things[b];
                    let frame = thing
                        .data
                        .pos
                        .as_ref()
                        .and_then(|p| Frame::of_matrix(&p.world));
                    frame.zip(thing.data.body).map(|(frame, pb)| {
                        scene.bodies.push(SceneBody {
                            thing: b,
                            frame,
                            velocity: [pb.velocity[0], pb.velocity[1]],
                            angular_velocity: pb.angular_velocity,
                            frozen: pb.frozen,
                            fixed: is_world(thing) || is_pickup(thing),
                        });
                        scene.bodies.len() - 1
                    })
                });
                if slot.is_none() {
                    *scene.report.rejected.entry("body without POS").or_default() += 1;
                    continue;
                }
                slot
            }
        };
        let frame = body.map_or(own, |b| scene.bodies[b].frame);
        let m = &pos.world;
        let world = |p: [f32; 2]| {
            [
                m[0] * p[0] + m[4] * p[1] + m[12],
                m[1] * p[0] + m[5] * p[1] + m[13],
            ]
        };
        let mut triangles = Vec::with_capacity(outline.triangles.len());
        for t in &outline.triangles {
            let [a, b, c] = t.map(|k| frame.to_local(world(outline.points[k])));
            let area = area(&[a, b, c]);
            if !area.is_finite() || area.abs() < MIN_TRIANGLE_AREA {
                continue;
            }
            // A mirrored transform reverses the winding.
            triangles.push(if area > 0. { [a, b, c] } else { [a, c, b] });
        }
        let (mut z, mut half_thickness) = (m[14], shape.thickness.abs());
        let mut turned = false;
        if triangles.is_empty() {
            // A shape turned on its side (its outline plane contains the z axis, e.g. rotated
            // 90 degrees about x): the 2D collider is the side view of its extruded slab.
            if let Some((tris, slab)) = sideways(m, &outline.points, shape.thickness.abs()) {
                triangles = tris
                    .into_iter()
                    .map(|t| t.map(|p| frame.to_local(p)))
                    .collect();
                (z, half_thickness) = slab;
                *scene.report.filled.entry("sideways prism").or_default() += 1;
                turned = true;
            }
        }
        if triangles.is_empty() {
            // Split by cause: a world matrix that flattens the xy plane (seen edge-on, or
            // scaled to nothing), a tiny outline, or neither.
            let det = m[0] * m[5] - m[1] * m[4];
            let scale = m[0].abs().max(m[1].abs()).max(m[4].abs()).max(m[5].abs());
            let axis = |x: f32, y: f32| x.hypot(y) <= 1e-6 * scale;
            let flat = scale <= 0. || det.abs() <= 1e-6 * scale * scale;
            if flat && scene.report.flat_samples.len() < FLAT_SAMPLES {
                let l = [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]];
                scene.report.flat_samples.push((i, l));
            }
            reject(if scale <= 0. {
                "no triangles: flat transform: zero xy scale"
            } else if axis(m[0], m[1]) || axis(m[4], m[5]) {
                "no triangles: flat transform: one axis zero"
            } else if det.abs() <= 1e-6 * scale * scale {
                // Both axes present but parallel: turned edge-on in 3D, or sheared flat.
                "no triangles: flat transform: axes parallel"
            } else if outline.area.abs() * f64::from(det.abs()) < 1e-3 {
                "no triangles: tiny outline"
            } else {
                "no triangles: other"
            });
            continue;
        }
        fixed_shape.push(rule == MaterialRule::Fixed);
        scene.colliders.push(SceneCollider {
            thing: i,
            body,
            frame,
            triangles,
            z,
            half_thickness,
            sideways: turned,
            density: shape.mass_depth.max(0.) * DENSITY,
        });
    }
    for (c, fixed) in scene.colliders.iter().zip(fixed_shape) {
        if let (Some(b), true) = (c.body, fixed) {
            scene.bodies[b].fixed = true;
        }
    }
    for c in &scene.colliders {
        if c.body.is_some_and(|b| !scene.bodies[b].fixed) {
            scene.report.dynamic += 1;
        } else {
            scene.report.static_ += 1;
        }
    }
    scene.report.fixed_bodies = scene.bodies.iter().filter(|b| b.fixed).count();
    scene.report.nested_bodies = nested.len();
    scene.add_joints(things);
    scene
}

/// PJoint type as the sandbox maps it (docs/runtime-research.md §4 joint table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    /// ELASTIC (1): pulls back towards its length, never pushes.
    Elastic,
    /// SPRING (2): holds its length, compliant.
    Spring,
    /// CHAIN (3, winch INFERRED): fixed length for now (animation not modelled).
    Chain,
    /// PISTON (4): slides along the contact line between its min and max length.
    Piston,
    /// STRING (5): at most its length.
    String,
    /// ROD (6): exactly its length.
    Rod,
    /// BOLT (7): shared pivot.
    Bolt,
    /// SPRING_ANGULAR (8) and QUANTIZED (10, wobble bolt INFERRED): pivot pulled back to
    /// its rest angle.
    SpringAngular,
    /// MOTOR (9): pivot turning at the stored speed.
    Motor,
}

impl JointKind {
    /// None for LEGACY (0) and unknown types.
    pub fn of(joint_type: i32) -> Option<Self> {
        Some(match joint_type {
            1 => Self::Elastic,
            2 => Self::Spring,
            3 => Self::Chain,
            4 => Self::Piston,
            5 => Self::String,
            6 => Self::Rod,
            7 => Self::Bolt,
            8 | 10 => Self::SpringAngular,
            9 => Self::Motor,
            _ => return None,
        })
    }
}

/// What a joint end is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    /// Index in [`PhysicsScene::bodies`].
    Body(usize),
    /// Index in [`PhysicsScene::colliders`] of a static shape.
    Static(usize),
}

/// One PJoint between two scene entities.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneJoint {
    pub thing: usize,
    pub kind: JointKind,
    pub a: Endpoint,
    pub b: Endpoint,
    /// Contact points in world xy and in each endpoint's frame.
    pub world_a: [f32; 2],
    pub world_b: [f32; 2],
    pub local_a: [f32; 2],
    pub local_b: [f32; 2],
    /// Rest length (PJoint.length; the contact distance when that is not usable).
    pub length: f32,
    /// Piston travel (tweakTargetMin/MaxLength when usable, else the rest length).
    pub min_length: f32,
    pub max_length: f32,
    pub params: lbp_level::parts::JointParams,
}

/// World xy of a point given in a Thing's local frame (its POS matrix).
fn world_point(t: &ThingRecord, p: [f32; 3]) -> Option<[f32; 2]> {
    let m = &t.data.pos.as_ref()?.world;
    let w = [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
    ];
    w.iter().all(|v| v.is_finite()).then_some(w)
}

impl PhysicsScene {
    fn frame_of(&self, e: Endpoint) -> Frame {
        match e {
            Endpoint::Body(i) => self.bodies[i].frame,
            Endpoint::Static(i) => self.colliders[i].frame,
        }
    }

    fn endpoint(&self, things: &[ThingRecord], t: usize) -> Option<Endpoint> {
        if let (Some(owner), _) = body_owner(things, t) {
            if let Some(i) = self.bodies.iter().position(|b| b.thing == owner) {
                return Some(Endpoint::Body(i));
            }
        }
        self.colliders
            .iter()
            .position(|c| c.thing == t && c.body.is_none())
            .map(Endpoint::Static)
    }

    /// Adds the level's PJoints between scene entities ([`build_scene`] calls it).
    /// Counted in the report: `joints` added, `rejected_joints` by reason.
    fn add_joints(&mut self, things: &[ThingRecord]) {
        for (i, t) in things.iter().enumerate() {
            let Some(j) = &t.data.joint else {
                continue;
            };
            match self.joint(things, i, j) {
                Ok(joint) => {
                    self.report.joints += 1;
                    self.joints.push(joint);
                }
                Err(why) => *self.report.rejected_joints.entry(why).or_default() += 1,
            }
        }
    }

    fn joint(
        &self,
        things: &[ThingRecord],
        thing: usize,
        j: &lbp_level::parts::Joint,
    ) -> Result<SceneJoint, &'static str> {
        let kind = JointKind::of(j.joint_type).ok_or("legacy or unknown type")?;
        let (Some(ta), Some(tb)) = (j.a, j.b) else {
            return Err("missing endpoint");
        };
        let (Some(a), Some(b)) = (self.endpoint(things, ta), self.endpoint(things, tb)) else {
            return Err("endpoint without a collider");
        };
        if a == b {
            return Err("both ends on one body");
        }
        let fixed = |e: Endpoint| match e {
            Endpoint::Static(_) => true,
            Endpoint::Body(i) => self.bodies[i].fixed,
        };
        // avian2d needs a dynamic body on at least one end (both static panics: "neither
        // body is in an island").
        if fixed(a) && fixed(b) {
            return Err("both ends static");
        }
        let (Some(world_a), Some(world_b)) = (
            things.get(ta).and_then(|t| world_point(t, j.a_contact)),
            things.get(tb).and_then(|t| world_point(t, j.b_contact)),
        ) else {
            return Err("endpoint without POS");
        };
        let gap = ((world_b[0] - world_a[0]).powi(2) + (world_b[1] - world_a[1]).powi(2)).sqrt();
        let length = if j.length.is_finite() && j.length > 0. {
            j.length
        } else {
            gap
        };
        let p = &j.params;
        let (lo, hi) = (p.tweak_target_min_length, p.tweak_target_max_length);
        let (min_length, max_length) =
            if lo.is_finite() && hi.is_finite() && 0. <= lo && lo <= hi && hi > 0. {
                (lo, hi)
            } else {
                (length, length)
            };
        Ok(SceneJoint {
            thing,
            kind,
            a,
            b,
            world_a,
            world_b,
            local_a: self.frame_of(a).to_local(world_a),
            local_b: self.frame_of(b).to_local(world_b),
            length,
            min_length,
            max_length,
            params: *p,
        })
    }
}
