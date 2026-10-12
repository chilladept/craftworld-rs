//! Rd: CPU pose evaluation and linear-blend skinning for the animated preview.
//! Renderer-free; matrices are column-major `[f32; 16]` like the RMesh skin poses.
//! Rules and provenance: docs/skinned-preview.md.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_mesh::{Skeleton, SkinStreams};
pub use lbp_resource::animation::Pose;
use lbp_resource::animation::{self, Animation};

pub type Mat4 = [f32; 16];

const IDENTITY: Mat4 = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum()
    })
}

/// Translation Ã— rotation (unit quaternion x, y, z, w) Ã— scale.
pub fn trs(t: [f32; 3], q: [f32; 4], s: [f32; 3]) -> Mat4 {
    let [x, y, z, w] = q;
    let r = [
        [
            1. - 2. * (y * y + z * z),
            2. * (x * y + z * w),
            2. * (x * z - y * w),
        ],
        [
            2. * (x * y - z * w),
            1. - 2. * (x * x + z * z),
            2. * (y * z + x * w),
        ],
        [
            2. * (x * z + y * w),
            2. * (y * z - x * w),
            1. - 2. * (x * x + y * y),
        ],
    ];
    let mut m = IDENTITY;
    for c in 0..3 {
        for row in 0..3 {
            m[c * 4 + row] = r[c][row] * s[c];
        }
        m[12 + c] = t[c];
    }
    m
}

/// One bone's local translation, rotation (unit quaternion x, y, z, w) and scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trs {
    pub t: [f32; 3],
    pub r: [f32; 4],
    pub s: [f32; 3],
}

impl Trs {
    pub fn matrix(&self) -> Mat4 {
        trs(self.t, self.r, self.s)
    }

    /// Splits a translation Ã— rotation Ã— scale matrix (no shear) back into its parts; a
    /// mirrored basis gets a negative x scale.
    pub fn from_matrix(m: &Mat4) -> Self {
        let col = |c: usize| [m[c * 4], m[c * 4 + 1], m[c * 4 + 2]];
        let len = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let (c0, c1, c2) = (col(0), col(1), col(2));
        let det = c0[0] * (c1[1] * c2[2] - c1[2] * c2[1]) - c1[0] * (c0[1] * c2[2] - c0[2] * c2[1])
            + c2[0] * (c0[1] * c1[2] - c0[2] * c1[1]);
        let mut s = [len(c0), len(c1), len(c2)];
        if det < 0. {
            s[0] = -s[0];
        }
        // r[row][col] of the pure rotation.
        let r = |row: usize, c: usize| {
            if s[c] == 0. {
                f32::from(u8::from(row == c))
            } else {
                m[c * 4 + row] / s[c]
            }
        };
        let trace = r(0, 0) + r(1, 1) + r(2, 2);
        let q = if trace > 0. {
            let k = (trace + 1.).sqrt() * 2.;
            [
                (r(2, 1) - r(1, 2)) / k,
                (r(0, 2) - r(2, 0)) / k,
                (r(1, 0) - r(0, 1)) / k,
                0.25 * k,
            ]
        } else if r(0, 0) > r(1, 1) && r(0, 0) > r(2, 2) {
            let k = (1. + r(0, 0) - r(1, 1) - r(2, 2)).max(0.).sqrt() * 2.;
            [
                0.25 * k,
                (r(0, 1) + r(1, 0)) / k,
                (r(0, 2) + r(2, 0)) / k,
                (r(2, 1) - r(1, 2)) / k,
            ]
        } else if r(1, 1) > r(2, 2) {
            let k = (1. + r(1, 1) - r(0, 0) - r(2, 2)).max(0.).sqrt() * 2.;
            [
                (r(0, 1) + r(1, 0)) / k,
                0.25 * k,
                (r(1, 2) + r(2, 1)) / k,
                (r(0, 2) - r(2, 0)) / k,
            ]
        } else {
            let k = (1. + r(2, 2) - r(0, 0) - r(1, 1)).max(0.).sqrt() * 2.;
            [
                (r(0, 2) + r(2, 0)) / k,
                (r(1, 2) + r(2, 1)) / k,
                0.25 * k,
                (r(1, 0) - r(0, 1)) / k,
            ]
        };
        Self {
            t: [m[12], m[13], m[14]],
            r: normalize(q),
            s,
        }
    }
}

fn normalize(q: [f32; 4]) -> [f32; 4] {
    let n = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    if n > 0. && n.is_finite() {
        q.map(|v| v / n)
    } else {
        [0., 0., 0., 1.]
    }
}

fn q_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

/// Normalised lerp along the shorter arc.
pub fn nlerp(a: [f32; 4], b: [f32; 4], w: f32) -> [f32; 4] {
    let dot: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
    let sign = if dot < 0. { -1. } else { 1. };
    normalize(std::array::from_fn(|i| a[i] + (b[i] * sign - a[i]) * w))
}

/// Per-axis `s / bind`, 1 where the bind scale is zero.
fn scale_ratio(s: [f32; 3], bind: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|c| {
        if bind[c].abs() > 1e-6 {
            s[c] / bind[c]
        } else {
            1.
        }
    })
}

fn lerp3(a: [f32; 3], b: [f32; 3], w: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * w)
}

fn lerp_morphs(dst: &mut Vec<f32>, src: &[f32], w: f32) {
    if dst.len() < src.len() {
        dst.resize(src.len(), 0.);
    }
    for (i, d) in dst.iter_mut().enumerate() {
        let s = src.get(i).copied().unwrap_or(0.);
        *d += (s - *d) * w;
    }
}

/// The pose a blend of several clips writes into: one local [`Trs`] per mesh bone plus
/// morph weights. Starts at the bind pose, so a bone no layer drives stays at bind.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendPose {
    pub bones: Vec<Trs>,
    pub morphs: Vec<f32>,
}

impl BlendPose {
    pub fn bind(rig: &Rig) -> Self {
        Self {
            bones: rig.bind_local.iter().map(Trs::from_matrix).collect(),
            morphs: Vec::new(),
        }
    }

    /// Moves each bone `rig` drives `w` of the way toward `pose` (translation and scale
    /// lerp, rotation nlerp; morph weights lerp). `rig` binds the pose's animation to the
    /// same mesh bones as this pose.
    pub fn lerp(&mut self, rig: &Rig, pose: &Pose, w: f32) {
        if w <= 0. || !w.is_finite() {
            return;
        }
        let w = w.min(1.);
        for (i, b) in self.bones.iter_mut().enumerate() {
            if let Some(p) = rig.pose_trs(pose, i) {
                b.t = lerp3(b.t, p.t, w);
                b.r = nlerp(b.r, p.r, w);
                b.s = lerp3(b.s, p.s, w);
            }
        }
        lerp_morphs(&mut self.morphs, &pose.morphs, w);
    }

    /// [`BlendPose::lerp`] on the bones flagged in `mask` only (one flag per mesh bone; a
    /// missing flag is false). Morph weights are left alone.
    pub fn lerp_masked(&mut self, rig: &Rig, pose: &Pose, w: f32, mask: &[bool]) {
        if w <= 0. || !w.is_finite() {
            return;
        }
        let w = w.min(1.);
        for (i, b) in self.bones.iter_mut().enumerate() {
            if !mask.get(i).copied().unwrap_or(false) {
                continue;
            }
            if let Some(p) = rig.pose_trs(pose, i) {
                b.t = lerp3(b.t, p.t, w);
                b.r = nlerp(b.r, p.r, w);
                b.s = lerp3(b.s, p.s, w);
            }
        }
    }

    /// [`BlendPose::lerp`] toward `pose` mirrored left/right ([`Rig::mirrored_trs`]).
    /// Morph weights lerp toward their mirror partners' ([`Rig::mirrored_morphs`]).
    pub fn lerp_mirrored(&mut self, rig: &Rig, pose: &Pose, w: f32) {
        if w <= 0. || !w.is_finite() {
            return;
        }
        let w = w.min(1.);
        for (b, p) in self.bones.iter_mut().zip(rig.mirrored_trs(pose)) {
            if let Some(p) = p {
                b.t = lerp3(b.t, p.t, w);
                b.r = nlerp(b.r, p.r, w);
                b.s = lerp3(b.s, p.s, w);
            }
        }
        lerp_morphs(&mut self.morphs, &rig.mirrored_morphs(&pose.morphs), w);
    }

    /// Adds `w` times the difference `pose` minus `reference` (both from `rig`'s
    /// animation): translation and morph deltas add, scale ratios multiply, and the local
    /// rotation delta `referenceâ»Â¹ Â· pose` is applied after the bone's rotation. The
    /// game's own composition (`0x1eafc0`) is not traced: INFERRED.
    pub fn add(&mut self, rig: &Rig, pose: &Pose, reference: &Pose, w: f32) {
        if w <= 0. || !w.is_finite() {
            return;
        }
        for (i, b) in self.bones.iter_mut().enumerate() {
            let (Some(p), Some(r)) = (rig.pose_trs(pose, i), rig.pose_trs(reference, i)) else {
                continue;
            };
            for c in 0..3 {
                b.t[c] += w * (p.t[c] - r.t[c]);
                if r.s[c].abs() > 1e-6 {
                    b.s[c] *= 1. + w * (p.s[c] / r.s[c] - 1.);
                }
            }
            let [x, y, z, qw] = r.r;
            let delta = q_mul([-x, -y, -z, qw], p.r);
            b.r = normalize(q_mul(b.r, nlerp([0., 0., 0., 1.], delta, w)));
        }
        let n = pose.morphs.len().max(reference.morphs.len());
        if self.morphs.len() < n {
            self.morphs.resize(n, 0.);
        }
        for (i, m) in self.morphs.iter_mut().enumerate().take(n) {
            let a = pose.morphs.get(i).copied().unwrap_or(0.);
            let b = reference.morphs.get(i).copied().unwrap_or(0.);
            *m += w * (a - b);
        }
    }

    /// Puts the root bones' (parent -1) rotation back to bind, keeping their translation
    /// and scale: the clips' root rotation is a facing channel (Sackboy: 0 in stand,
    /// jump_stand and the lane clips, +90 degrees about the model up axis in the walk, stop,
    /// jump_walk and idle-chain clips, a 90 to -90 sweep in walk_fast_turn) that the game
    /// replaces with its own facing angle (D232 fix, INFERRED from that data and the
    /// sin/cos built from the eased yaw at 0x201b18).
    pub fn hold_root_rotation(&mut self, rig: &Rig) {
        for (i, b) in self.bones.iter_mut().enumerate() {
            if rig.bones.get(i).is_some_and(|r| r.parent == -1) {
                b.r = Trs::from_matrix(&rig.bind_local[i]).r;
            }
        }
    }

    pub fn local_matrices(&self) -> Vec<Mat4> {
        self.bones.iter().map(Trs::matrix).collect()
    }

    /// Two-bone solver: moves the `end` joint of the chain `upper` -> `lower` -> `end` (a
    /// thigh, calf and foot) by `offset` in model space. The upper joint stays where it is,
    /// both bone lengths are kept, the middle joint stays in the plane it bends in now and
    /// the end bone keeps its model-space rotation (a level foot stays level). A target out
    /// of reach is pulled in to the nearest reachable point. Only the three rotations change.
    /// Returns false (pose untouched) when the bones are not such a chain or have no length.
    pub fn move_chain_end(
        &mut self,
        rig: &Rig,
        [upper, lower, end]: [usize; 3],
        offset: [f32; 3],
    ) -> bool {
        let parent = |i: usize| rig.bones.get(i).map(|b| b.parent);
        if end >= self.bones.len()
            || parent(end) != i32::try_from(lower).ok()
            || parent(lower) != i32::try_from(upper).ok()
        {
            return false;
        }
        let worlds = rig.worlds_from_trs(&self.bones);
        let at = |i: usize| [worlds[i][12], worlds[i][13], worlds[i][14]];
        let (a, b, c) = (at(upper), at(lower), at(end));
        let (ab, bc) = (sub3(b, a), sub3(c, b));
        let (l1, l2) = (len3(ab), len3(bc));
        let to = sub3(add3(c, offset), a);
        let reach = len3(to);
        if l1 < 1e-4 || l2 < 1e-4 || reach < 1e-4 || !reach.is_finite() {
            return false;
        }
        let d = reach.clamp((l1 - l2).abs() + 1e-3, l1 + l2 - 1e-3);
        let dir = scale3(to, 1. / reach);
        // The side the middle joint bends to: where it is now, seen from the new axis.
        let side = sub3(ab, scale3(dir, dot3(ab, dir)));
        let side = if len3(side) > 1e-4 {
            scale3(side, 1. / len3(side))
        } else {
            // Straight along the axis: bend toward the model's front (-Y).
            let f = [0., -1., 0.];
            let s = sub3(f, scale3(dir, dot3(f, dir)));
            if len3(s) < 1e-4 {
                return false;
            }
            scale3(s, 1. / len3(s))
        };
        let cos = ((l1 * l1 + d * d - l2 * l2) / (2. * l1 * d)).clamp(-1., 1.);
        let sin = (1. - cos * cos).max(0.).sqrt();
        let knee = add3(scale3(dir, l1 * cos), scale3(side, l1 * sin));
        let target = scale3(dir, d);
        // Model-space turns: the upper bone onto the new middle joint, then the lower bone
        // (already carried by the first turn) onto the target.
        let d1 = q_between(ab, knee);
        let d2 = q_between(q_rotate(d1, bc), sub3(target, knee));
        // Model-space rotation of each bone: the product of the local rotations above it.
        let model = |bones: &[Trs], i: usize| {
            rig.chains[i]
                .iter()
                .fold([0., 0., 0., 1.], |q, &j| q_mul(q, bones[j].r))
        };
        let (qu, ql, qe) = (
            model(&self.bones, upper),
            model(&self.bones, lower),
            model(&self.bones, end),
        );
        let qp = q_mul(qu, q_conj(self.bones[upper].r));
        let qu_new = q_mul(d1, qu);
        let ql_new = q_mul(d2, q_mul(d1, ql));
        self.bones[upper].r = normalize(q_mul(q_conj(qp), qu_new));
        self.bones[lower].r = normalize(q_mul(q_conj(qu_new), ql_new));
        self.bones[end].r = normalize(q_mul(q_conj(ql_new), qe));
        true
    }
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn scale3(a: [f32; 3], k: f32) -> [f32; 3] {
    a.map(|v| v * k)
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn len3(a: [f32; 3]) -> f32 {
    dot3(a, a).sqrt()
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn q_conj([x, y, z, w]: [f32; 4]) -> [f32; 4] {
    [-x, -y, -z, w]
}

fn q_rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let [x, y, z, _] = q_mul(q_mul(q, [v[0], v[1], v[2], 0.]), q_conj(q));
    [x, y, z]
}

/// The shortest turn taking the direction of `a` to the direction of `b` (identity for a
/// zero vector; opposite directions turn about an axis square to `a`).
fn q_between(a: [f32; 3], b: [f32; 3]) -> [f32; 4] {
    let (la, lb) = (len3(a), len3(b));
    if la < 1e-6 || lb < 1e-6 {
        return [0., 0., 0., 1.];
    }
    let (a, b) = (scale3(a, 1. / la), scale3(b, 1. / lb));
    let w = 1. + dot3(a, b);
    if w < 1e-6 {
        let other = if a[0].abs() < 0.9 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        };
        let [x, y, z] = cross3(a, other);
        return normalize([x, y, z, 0.]);
    }
    let [x, y, z] = cross3(a, b);
    normalize([x, y, z, w])
}

/// The pose at a fractional frame: the two neighbouring frames blended (translation and
/// scale lerp, rotation nlerp, morph lerp). The frame is clamped to the clip.
pub fn sample_pose(animation: &Animation, frame: f32) -> Option<Pose> {
    let last = animation.frame_count().checked_sub(1)?;
    let f = if frame.is_finite() {
        frame.clamp(0., f32::from(last))
    } else {
        0.
    };
    let f0 = f.floor() as u16;
    let t = f - f32::from(f0);
    let a = animation.pose(f0)?;
    if t <= 0. || f0 >= last {
        return Some(a);
    }
    let b = animation.pose(f0 + 1)?;
    Some(lerp_pose(&a, &b, t))
}

/// `a` moved `t` of the way toward `b` (same bone lists).
pub fn lerp_pose(a: &Pose, b: &Pose, t: f32) -> Pose {
    let mut morphs = a.morphs.clone();
    lerp_morphs(&mut morphs, &b.morphs, t);
    Pose {
        rotations: a
            .rotations
            .iter()
            .zip(&b.rotations)
            .map(|(x, y)| nlerp(*x, *y, t))
            .collect(),
        positions: a
            .positions
            .iter()
            .zip(&b.positions)
            .map(|(x, y)| lerp3(*x, *y, t))
            .collect(),
        scales: a
            .scales
            .iter()
            .zip(&b.scales)
            .map(|(x, y)| lerp3(*x, *y, t))
            .collect(),
        morphs,
    }
}

fn transform(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
}

/// How a bone's animated scale reaches the bones below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScaleRule {
    /// Plain matrix hierarchy: a bone's scale scales everything below it.
    #[default]
    Inherited,
    /// The bones named as 3ds Max Biped parts (`Bip...`) do not take their parent's scale:
    /// a scaled Biped bone stretches its own vertices and moves its children's joints
    /// (their offsets are scaled), but a Biped child keeps its own size and shape. Every
    /// other bone inherits as in [`ScaleRule::Inherited`]. MEASURED for Sackboy (D246,
    /// docs/d242-idle-stretch.md); the game's own composition is not traced.
    Biped,
}

/// One mesh bone as the rig needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct RigBone {
    pub parent: i32,
    pub anim_hash: u32,
    pub skin_pose: Mat4,
    pub inv_skin_pose: Mat4,
}

/// Mesh bone hierarchy bound to one animation's bone list by anim hash.
#[derive(Debug, Clone, PartialEq)]
pub struct Rig {
    bones: Vec<RigBone>,
    /// Root-first chain per bone (validated once; bounded by the bone count).
    chains: Vec<Vec<usize>>,
    bind_local: Vec<Mat4>,
    /// Animation bone index driving each mesh bone, if any.
    pub driven_by: Vec<Option<usize>>,
    /// Animation bones with no mesh bone of the same hash.
    pub unmatched_animation_bones: usize,
    /// Each bone's left/right partner (the skeleton's mirror table; itself on the centre
    /// line, and for every bone when the table is missing or out of range).
    pub mirror: Vec<usize>,
    /// The skeleton's mirror flip type per bone ([`flip_local`]); empty without the table.
    flip: Vec<u8>,
    /// The skeleton's mirror morph table: the morph each morph takes its weight from in a
    /// mirrored pose; empty without the table.
    mirror_morphs: Vec<i16>,
    /// Bones named as 3ds Max Biped parts (`Bip...`); all false without a skeleton.
    biped: Vec<bool>,
    /// How scale passes down the hierarchy (default [`ScaleRule::Inherited`]).
    pub scale_rule: ScaleRule,
}

/// One bone's local transform mirrored by its flip type (the game's pose mirror, EBOOT
/// `0x1e42b4`; type names from the Toolkit): the parent's frame and the bone's own frame
/// are each reflected on the axis the type names, scale is kept, and types 6 and up copy.
fn flip_local(kind: u8, p: Trs) -> Trs {
    let ([x, y, z], [qx, qy, qz, qw]) = (p.t, p.r);
    let (t, r) = match kind {
        0 => ([-x, y, z], [qx, -qy, -qz, qw]),     // MAX
        1 => ([-x, y, z], [qy, qx, -qw, -qz]),     // BIP_ROOT
        2 => ([x, -y, z], [-qw, qz, qy, -qx]),     // BIP_PELVIS
        3 | 4 => ([x, y, -z], [-qx, -qy, qz, qw]), // BIP_BONE, PARENT_WAS_BIP_BONE
        5 => ([x, y, -z], [-qz, qw, -qx, qy]),     // GRANDPARENT_WAS_BIP_BONE
        _ => (p.t, p.r),
    };
    Trs { t, r, s: p.s }
}

/// Reflection across the model's x = 0 plane (left/right; Z-up, front -Y).
const MIRROR_X: Mat4 = [
    -1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

impl Rig {
    pub fn new(bones: Vec<RigBone>, animation_hashes: &[u32]) -> Result<Self, String> {
        if bones.is_empty() {
            return Err("Unsupported: mesh has no bones".into());
        }
        let mut chains = Vec::with_capacity(bones.len());
        for i in 0..bones.len() {
            let mut chain = vec![i];
            let mut at = bones[i].parent;
            while at != -1 {
                let p = usize::try_from(at)
                    .ok()
                    .filter(|&p| p < bones.len())
                    .ok_or("Invalid: bone parent out of range")?;
                if chain.len() >= bones.len() {
                    return Err("Invalid: bone parent cycle".into());
                }
                chain.push(p);
                at = bones[p].parent;
            }
            chain.reverse();
            chains.push(chain);
        }
        let bind_local = bones
            .iter()
            .map(|b| match b.parent {
                -1 => b.skin_pose,
                p => mul(&bones[p as usize].inv_skin_pose, &b.skin_pose),
            })
            .collect();
        let driven_by: Vec<_> = bones
            .iter()
            .map(|b| animation_hashes.iter().position(|&h| h == b.anim_hash))
            .collect();
        let unmatched_animation_bones = animation_hashes
            .iter()
            .filter(|&&h| !bones.iter().any(|b| b.anim_hash == h))
            .count();
        let mirror = (0..bones.len()).collect();
        Ok(Self {
            biped: vec![false; bones.len()],
            scale_rule: ScaleRule::default(),
            bones,
            chains,
            bind_local,
            driven_by,
            unmatched_animation_bones,
            mirror,
            flip: Vec::new(),
            mirror_morphs: Vec::new(),
        })
    }

    /// Sets the left/right partner table (`Skeleton::mirror_bones`); ignored unless it has
    /// one in-range entry per bone and pairs up both ways.
    pub fn with_mirror(mut self, table: &[i16]) -> Self {
        let n = self.bones.len();
        let map: Option<Vec<usize>> = (table.len() == n)
            .then(|| {
                table
                    .iter()
                    .map(|&m| usize::try_from(m).ok().filter(|&m| m < n))
                    .collect()
            })
            .flatten();
        if let Some(map) = map.filter(|m: &Vec<usize>| (0..n).all(|i| m[m[i]] == i)) {
            self.mirror = map;
        }
        self
    }

    /// Sets the mirror flip types (`Skeleton::mirror_flip_types`); ignored unless there is
    /// one per bone.
    pub fn with_flip_types(mut self, table: &[u8]) -> Self {
        if table.len() == self.bones.len() {
            self.flip = table.to_vec();
        }
        self
    }

    /// Sets the mirror morph table (`Skeleton::mirror_morphs`).
    pub fn with_mirror_morphs(mut self, table: &[i16]) -> Self {
        self.mirror_morphs = table.to_vec();
        self
    }

    /// `morphs` as a mirrored pose carries them (the game's pose mirror, `0x1e42b4`, FOUND):
    /// morph i takes the weight of morph `mirror_morphs[i]`. An entry outside the weights
    /// (or no entry) keeps the morph's own weight.
    pub fn mirrored_morphs(&self, morphs: &[f32]) -> Vec<f32> {
        (0..morphs.len())
            .map(|i| {
                self.mirror_morphs
                    .get(i)
                    .and_then(|&j| morphs.get(usize::try_from(j).ok()?))
                    .copied()
                    .unwrap_or(morphs[i])
            })
            .collect()
    }

    pub fn from_skeleton(skeleton: &Skeleton, animation_hashes: &[u32]) -> Result<Self, String> {
        let bones = skeleton
            .bones
            .iter()
            .map(|b| RigBone {
                parent: b.parent,
                anim_hash: b.anim_hash,
                skin_pose: b.skin_pose,
                inv_skin_pose: b.inv_skin_pose,
            })
            .collect();
        let mut rig = Self::new(bones, animation_hashes)?
            .with_mirror(&skeleton.mirror_bones)
            .with_flip_types(&skeleton.mirror_flip_types)
            .with_mirror_morphs(&skeleton.mirror_morphs);
        rig.biped = skeleton
            .bones
            .iter()
            .map(|b| b.name.starts_with("Bip"))
            .collect();
        Ok(rig)
    }

    /// The same mesh bones bound to another animation's bone list (clip banks share one
    /// mesh rig across clips).
    pub fn rebind(&self, animation_hashes: &[u32]) -> Result<Self, String> {
        let mut rig = Self::new(self.bones.clone(), animation_hashes)?;
        rig.mirror = self.mirror.clone();
        rig.flip = self.flip.clone();
        rig.mirror_morphs = self.mirror_morphs.clone();
        rig.biped = self.biped.clone();
        rig.scale_rule = self.scale_rule;
        Ok(rig)
    }

    fn worlds(&self, local: &[Mat4]) -> Vec<Mat4> {
        self.chains
            .iter()
            .map(|chain| {
                chain.iter().fold(IDENTITY, |acc, &j| {
                    mul(&acc, local.get(j).unwrap_or(&self.bind_local[j]))
                })
            })
            .collect()
    }

    /// `pose` mirrored left/right (the game's layer mirror flag, `0x1ebb48` mode 5); None
    /// where neither a bone nor its partner is driven. With the skeleton's flip types
    /// (D254, FOUND in the game's routine) bone i takes its partner's local transform
    /// through [`flip_local`], scale copied. Without them bone i takes its partner j's
    /// motion reflected across x = 0, so its skinning matrix is `X * S_j * X` (S = world x
    /// inverse bind), which assumes a left/right symmetric bind.
    /// Under [`ScaleRule::Biped`] the compensating bones are mirrored at bind scale and
    /// then take their partner's scale ratio per axis (partners share their local axes
    /// up to sign), so the result composes under the same rule.
    pub fn mirrored_trs(&self, pose: &Pose) -> Vec<Option<Trs>> {
        let posed = self.pose_locals(Some(pose));
        if !self.flip.is_empty() {
            return (0..self.bones.len())
                .map(|i| {
                    let j = self.mirror[i];
                    (self.driven_by[i].is_some() || self.driven_by[j].is_some())
                        .then(|| flip_local(self.flip[i], posed[j]))
                })
                .collect();
        }
        let bind = self.bind_trs();
        let rigid: Vec<Mat4> = (0..self.bones.len())
            .map(|i| {
                if self.compensates(i) {
                    trs(posed[i].t, posed[i].r, bind[i].s)
                } else {
                    posed[i].matrix()
                }
            })
            .collect();
        // Plain inheritance keeps the bind matrices as they are (no TRS round trip).
        let world = if self.scale_rule == ScaleRule::Inherited {
            self.worlds(&self.local_matrices(Some(pose)))
        } else {
            self.worlds(&rigid)
        };
        let mirrored: Vec<Mat4> = (0..self.bones.len())
            .map(|i| {
                let j = self.mirror[i];
                let skin = mul(&world[j], &self.bones[j].inv_skin_pose);
                mul(
                    &mul(&mul(&MIRROR_X, &skin), &MIRROR_X),
                    &self.bones[i].skin_pose,
                )
            })
            .collect();
        (0..self.bones.len())
            .map(|i| {
                let j = self.mirror[i];
                if self.driven_by[i].is_none() && self.driven_by[j].is_none() {
                    return None;
                }
                let local = match self.bones[i].parent {
                    -1 => mirrored[i],
                    p => {
                        let inv =
                            crate::level_backdrop::affine_inverse(&mirrored[p as usize]).ok()?;
                        mul(&inv, &mirrored[i])
                    }
                };
                let mut out = Trs::from_matrix(&local);
                if self.compensates(i) {
                    let k = scale_ratio(posed[j].s, bind[j].s);
                    out.s = std::array::from_fn(|c| bind[i].s[c] * k[c]);
                }
                Some(out)
            })
            .collect()
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    pub fn matched_bones(&self) -> usize {
        self.driven_by.iter().flatten().count()
    }

    /// Per mesh bone: world(pose) Ã— inverse skin pose. Undriven bones keep their bind
    /// local transform, so a pose without matches reproduces the bind pose.
    pub fn skin_matrices(&self, pose: Option<&Pose>) -> Vec<Mat4> {
        if self.scale_rule != ScaleRule::Inherited {
            return self.skin_matrices_from_trs(&self.pose_locals(pose));
        }
        self.skin_matrices_from_locals(&self.local_matrices(pose))
    }

    /// Per mesh bone: world (from these local transforms) Ã— inverse skin pose.
    pub fn skin_matrices_from_locals(&self, local: &[Mat4]) -> Vec<Mat4> {
        self.chains
            .iter()
            .zip(&self.bones)
            .map(|(chain, bone)| {
                let world = chain.iter().fold(IDENTITY, |acc, &j| {
                    mul(&acc, local.get(j).unwrap_or(&self.bind_local[j]))
                });
                mul(&world, &bone.inv_skin_pose)
            })
            .collect()
    }

    /// Marks the bones [`ScaleRule::Biped`] treats as Biped parts (one flag per bone;
    /// [`Rig::from_skeleton`] sets them from the bone names). Ignored on a length mismatch.
    pub fn set_biped_bones(&mut self, flags: &[bool]) {
        if flags.len() == self.bones.len() {
            self.biped = flags.to_vec();
        }
    }

    /// True when bone `j` does not take its parent's scale under the rig's rule.
    fn compensates(&self, j: usize) -> bool {
        match self.scale_rule {
            ScaleRule::Inherited => false,
            ScaleRule::Biped => self.biped[j],
        }
    }

    fn bind_trs(&self) -> Vec<Trs> {
        self.bind_local.iter().map(Trs::from_matrix).collect()
    }

    /// World matrix per mesh bone from local transforms under the rig's [`ScaleRule`]
    /// (missing entries are bind). A compensating bone sits in its parent's unscaled
    /// frame, its offset multiplied by the parent's scale over bind scale.
    pub fn worlds_from_trs(&self, local: &[Trs]) -> Vec<Mat4> {
        let bind = self.bind_trs();
        self.chains
            .iter()
            .map(|chain| {
                // frame: the bone's world with its bind scale; world: with its pose scale.
                let (mut frame, mut world, mut k) = (IDENTITY, IDENTITY, [1f32; 3]);
                for &j in chain {
                    let l = local.get(j).copied().unwrap_or(bind[j]);
                    if self.compensates(j) {
                        let t = std::array::from_fn(|c| l.t[c] * k[c]);
                        world = mul(&frame, &trs(t, l.r, l.s));
                        frame = mul(&frame, &trs(t, l.r, bind[j].s));
                        k = scale_ratio(l.s, bind[j].s);
                    } else {
                        world = mul(&world, &l.matrix());
                        frame = world;
                        k = [1.; 3];
                    }
                }
                world
            })
            .collect()
    }

    /// Per mesh bone: world under the rig's [`ScaleRule`] x inverse skin pose.
    pub fn skin_matrices_from_trs(&self, local: &[Trs]) -> Vec<Mat4> {
        self.worlds_from_trs(local)
            .iter()
            .zip(&self.bones)
            .map(|(w, b)| mul(w, &b.inv_skin_pose))
            .collect()
    }

    /// For a joint hierarchy that reproduces [`Rig::worlds_from_trs`] with plain
    /// parent-child transforms: each bone is a frame node with a scale node under it
    /// (the scale node is the skinning joint). Returns per bone whether its frame node
    /// hangs under the parent's scale node (an inheriting bone) or the parent's frame
    /// node (a compensating bone). All true under [`ScaleRule::Inherited`].
    pub fn joint_inherits(&self) -> Vec<bool> {
        (0..self.bones.len())
            .map(|j| !self.compensates(j))
            .collect()
    }

    /// The frame node's local matrix and the scale node's scale per bone, for the
    /// hierarchy of [`Rig::joint_inherits`].
    pub fn joint_locals(&self, local: &[Trs]) -> Vec<(Mat4, [f32; 3])> {
        let bind = self.bind_trs();
        let at = |j: usize| local.get(j).copied().unwrap_or(bind[j]);
        (0..self.bones.len())
            .map(|j| {
                let l = at(j);
                if !self.compensates(j) {
                    return (l.matrix(), [1.; 3]);
                }
                let k = usize::try_from(self.bones[j].parent)
                    .ok()
                    .filter(|&p| self.compensates(p))
                    .map_or([1.; 3], |p| scale_ratio(at(p).s, bind[p].s));
                (
                    trs(std::array::from_fn(|c| l.t[c] * k[c]), l.r, bind[j].s),
                    scale_ratio(l.s, bind[j].s),
                )
            })
            .collect()
    }

    /// Every bone's local transform in `pose` (bind where the animation does not drive it).
    pub fn pose_locals(&self, pose: Option<&Pose>) -> Vec<Trs> {
        let bind = self.bind_trs();
        (0..self.bones.len())
            .map(|i| pose.and_then(|p| self.pose_trs(p, i)).unwrap_or(bind[i]))
            .collect()
    }

    /// Per mesh bone local transform: the pose's TRS for driven bones, else bind local.
    /// A joint hierarchy fed these and `inverse_binds` reproduces `skin_matrices`.
    pub fn local_matrices(&self, pose: Option<&Pose>) -> Vec<Mat4> {
        (0..self.bones.len())
            .map(|i| {
                match (pose, self.driven_by[i]) {
                    (Some(p), Some(a)) => p
                        .rotations
                        .get(a)
                        .zip(p.positions.get(a))
                        .zip(p.scales.get(a))
                        .map(|((r, t), s)| trs(*t, *r, *s)),
                    _ => None,
                }
                .unwrap_or(self.bind_local[i])
            })
            .collect()
    }

    /// Mesh bone `i`'s local transform in `pose`, when this rig's animation drives it.
    pub fn pose_trs(&self, pose: &Pose, i: usize) -> Option<Trs> {
        let a = (*self.driven_by.get(i)?)?;
        Some(Trs {
            t: *pose.positions.get(a)?,
            r: *pose.rotations.get(a)?,
            s: *pose.scales.get(a)?,
        })
    }

    /// The mesh bone whose animation hash is that of `name` (`Bip01 L Thigh`).
    pub fn bone_named(&self, name: &str) -> Option<usize> {
        let hash = lbp_mesh::anim_hash(name);
        self.bones.iter().position(|b| b.anim_hash == hash)
    }

    /// One flag per bone: true for bone `root` and every bone below it, except bone `cut`
    /// and the bones below that.
    pub fn subtree_mask(&self, root: usize, cut: Option<usize>) -> Vec<bool> {
        self.chains
            .iter()
            .map(|chain| chain.contains(&root) && cut.is_none_or(|c| !chain.contains(&c)))
            .collect()
    }

    pub fn parents(&self) -> Vec<i32> {
        self.bones.iter().map(|b| b.parent).collect()
    }

    pub fn inverse_binds(&self) -> Vec<Mat4> {
        self.bones.iter().map(|b| b.inv_skin_pose).collect()
    }
}

/// GPU skin attributes with the same semantics as `skin_positions`: weights on joints
/// past `bone_count` or non-positive are dropped, the rest normalised; a vertex with no
/// usable weight is bound fully to the extra static joint `bone_count`.
pub fn gpu_skin_attributes(
    joints: &[[u8; 4]],
    weights: &[[f32; 4]],
    bone_count: usize,
) -> (Vec<[u16; 4]>, Vec<[f32; 4]>) {
    let fixed = u16::try_from(bone_count).unwrap_or(u16::MAX);
    joints
        .iter()
        .zip(weights)
        .map(|(j, w)| {
            let mut out_j = [0u16; 4];
            let mut out_w = [0f32; 4];
            for k in 0..4 {
                if usize::from(j[k]) < bone_count && w[k] > 0. && w[k].is_finite() {
                    out_j[k] = u16::from(j[k]);
                    out_w[k] = w[k];
                }
            }
            let total: f32 = out_w.iter().sum();
            if total > 0. {
                (out_j, out_w.map(|v| v / total))
            } else {
                ([fixed, 0, 0, 0], [1., 0., 0., 0.])
            }
        })
        .unzip()
}

/// Bind positions plus weighted morph offsets (morph index = animation morph index).
/// Returns how many morphs were applied (the shorter of the two lists).
pub fn apply_morphs(positions: &mut [[f32; 3]], skin: &SkinStreams, weights: &[f32]) -> usize {
    let count = skin.morphs.len().min(weights.len());
    for (morph, &w) in skin.morphs.iter().zip(weights) {
        if w == 0. || !w.is_finite() {
            continue;
        }
        for (p, o) in positions.iter_mut().zip(&morph.offsets) {
            for c in 0..3 {
                p[c] += o[c] * w;
            }
        }
    }
    count
}

/// Linear-blend skinning. Joints past the matrix list and zero-weight vertices keep the
/// input position; weights are normalised by their sum.
pub fn skin_positions(
    positions: &[[f32; 3]],
    joints: &[[u8; 4]],
    weights: &[[f32; 4]],
    matrices: &[Mat4],
) -> Vec<[f32; 3]> {
    positions
        .iter()
        .zip(joints.iter().zip(weights))
        .map(|(&p, (j, w))| {
            let mut out = [0f32; 3];
            let mut total = 0f32;
            for k in 0..4 {
                let Some(m) = matrices.get(usize::from(j[k])) else {
                    continue;
                };
                if w[k] <= 0. || !w[k].is_finite() {
                    continue;
                }
                let q = transform(m, p);
                for c in 0..3 {
                    out[c] += q[c] * w[k];
                }
                total += w[k];
            }
            if total > 0. {
                out.map(|v| v / total)
            } else {
                p
            }
        })
        .collect()
}

/// What the viewer shows about a bound clip (counts only; no game data).
#[derive(Debug, Clone, PartialEq)]
pub struct ClipReport {
    pub mesh_bones: usize,
    pub animation_bones: usize,
    pub matched_bones: usize,
    pub unmatched_animation_bones: usize,
    pub mesh_morphs: usize,
    pub animation_morphs: usize,
    pub frames: u16,
    pub fps: u16,
    pub looped: bool,
}

/// One animation bound to a [`SkinnedClip`]'s mesh rig, ready to [`SkinnedClip::swap`] in.
pub struct BoundAnimation {
    pub rig: Rig,
    pub animation: Animation,
    pub report: ClipReport,
}

fn bound_report(
    rig: &Rig,
    skin: &SkinStreams,
    animation: &Animation,
) -> Result<ClipReport, String> {
    if rig.matched_bones() == 0 {
        return Err("Unsupported: animation drives none of this mesh's bones".into());
    }
    let p = &animation.inspection.prefix;
    Ok(ClipReport {
        mesh_bones: rig.bone_count(),
        animation_bones: animation.bones.len(),
        matched_bones: rig.matched_bones(),
        unmatched_animation_bones: rig.unmatched_animation_bones,
        mesh_morphs: skin.morphs.len(),
        animation_morphs: usize::from(p.morph_count_raw_u8),
        frames: animation.frame_count(),
        fps: p.fps_raw_u16,
        looped: p.loop_start_raw_u16 < animation.frame_count(),
    })
}

/// A mesh's skin streams bound to one animation.
pub struct SkinnedClip {
    pub rig: Rig,
    pub skin: SkinStreams,
    pub animation: Animation,
    pub bind_positions: Vec<[f32; 3]>,
    pub report: ClipReport,
}

impl SkinnedClip {
    pub fn new(
        geometry: &lbp_mesh::Geometry,
        skin: SkinStreams,
        animation: Animation,
    ) -> Result<Self, String> {
        if skin.joints.len() != geometry.positions.len() {
            return Err("Invalid: skin stream and position counts differ".into());
        }
        let hashes: Vec<u32> = animation.bones.iter().map(|b| b.anim_hash).collect();
        let rig = Rig::from_skeleton(&geometry.skeleton, &hashes)?;
        let report = bound_report(&rig, &skin, &animation)?;
        Ok(Self {
            rig,
            skin,
            animation,
            bind_positions: geometry.positions.clone(),
            report,
        })
    }

    /// Binds another animation to this clip's mesh rig and skin (clip bank entry).
    pub fn bind(&self, animation: Animation) -> Result<BoundAnimation, String> {
        let hashes: Vec<u32> = animation.bones.iter().map(|b| b.anim_hash).collect();
        let rig = self.rig.rebind(&hashes)?;
        let report = bound_report(&rig, &self.skin, &animation)?;
        Ok(BoundAnimation {
            rig,
            animation,
            report,
        })
    }

    /// Plays `next` from now on and returns the animation it replaces. The mesh, skin
    /// and bind positions stay (a GPU joint hierarchy built from the rig stays valid).
    pub fn swap(&mut self, next: BoundAnimation) -> BoundAnimation {
        BoundAnimation {
            rig: std::mem::replace(&mut self.rig, next.rig),
            animation: std::mem::replace(&mut self.animation, next.animation),
            report: std::mem::replace(&mut self.report, next.report),
        }
    }

    /// Resolved frame and pose for a running frame number (see `frame_positions`).
    pub fn frame_pose(&self, frame: i64) -> Option<Pose> {
        let frames = i64::from(self.animation.frame_count());
        let frame = if self.report.looped {
            self.animation.looped_frame(frame, true)?
        } else {
            u16::try_from(frame.checked_rem_euclid(frames)?).ok()?
        };
        self.animation.pose(frame)
    }

    /// Bind positions with this pose's morph weights applied (before skinning).
    pub fn morphed_bind(&self, pose: &Pose) -> Vec<[f32; 3]> {
        let mut base = self.bind_positions.clone();
        apply_morphs(&mut base, &self.skin, &pose.morphs);
        base
    }

    /// Bind positions with these morph weights applied (a [`BlendPose`]'s morphs).
    pub fn morphed_bind_weights(&self, morphs: &[f32]) -> Vec<[f32; 3]> {
        let mut base = self.bind_positions.clone();
        apply_morphs(&mut base, &self.skin, morphs);
        base
    }

    /// Skinned positions for a blended pose (CPU path).
    pub fn blended_positions(&self, pose: &BlendPose) -> Vec<[f32; 3]> {
        let base = self.morphed_bind_weights(&pose.morphs);
        let matrices = if self.rig.scale_rule == ScaleRule::Inherited {
            self.rig.skin_matrices_from_locals(&pose.local_matrices())
        } else {
            self.rig.skin_matrices_from_trs(&pose.bones)
        };
        skin_positions(&base, &self.skin.joints, &self.skin.weights, &matrices)
    }

    /// Skinned positions for a running frame number. Clips with a loop range wrap into
    /// it; others restart from frame 0 (preview policy: the game decides when a
    /// non-looping clip ends).
    pub fn frame_positions(&self, frame: i64) -> Option<Vec<[f32; 3]>> {
        let pose = self.frame_pose(frame)?;
        let base = self.morphed_bind(&pose);
        let matrices = self.rig.skin_matrices(Some(&pose));
        Some(skin_positions(
            &base,
            &self.skin.joints,
            &self.skin.weights,
            &matrices,
        ))
    }
}

/// Reads the mesh and the animation (read-only, budgeted) and binds them.
pub fn load_clip(
    source: &AttachedSource,
    mesh_key: &LocationKey,
    animation_key: &LocationKey,
    control: &LoadControl,
) -> Result<SkinnedClip, String> {
    let (geometry, mesh_bytes) = crate::read_mesh(source, mesh_key, control)?;
    let skin = lbp_mesh::decode_skin(&mesh_bytes, || control.check().is_err())
        .map_err(|e| format!("Skin streams: {e}"))?;
    let animation = read_animation(source, animation_key, control)?;
    SkinnedClip::new(&geometry, skin, animation)
}

/// Clip bank (Sackboy animation slice 2): the mesh read once and bound to the first
/// animation, plus each further animation bound to the same rig (per-GUID result, so one
/// unreadable clip does not drop the rest).
pub fn load_clip_bank(
    source: &AttachedSource,
    catalog: &Catalog,
    mesh_key: &LocationKey,
    guids: &[u32],
    control: &LoadControl,
) -> Result<(SkinnedClip, Vec<Result<BoundAnimation, String>>), String> {
    let (&first, rest) = guids.split_first().ok_or("Empty clip bank")?;
    let key = resolve_animation(catalog, first)?;
    let clip = load_clip(source, mesh_key, &key, control)?;
    let bound = rest
        .iter()
        .map(|&g| {
            control.check()?;
            let key = resolve_animation(catalog, g)?;
            clip.bind(read_animation(source, &key, control)?)
                .map_err(|e| format!("{g:08x}: {e}"))
        })
        .collect();
    Ok((clip, bound))
}

/// Reads and decodes one animation (read-only, budgeted).
pub fn read_animation(
    source: &AttachedSource,
    animation_key: &LocationKey,
    control: &LoadControl,
) -> Result<Animation, String> {
    if animation_key.size > animation::MAX_INPUT as u64 {
        return Err("Animation budget exceeded".into());
    }
    let mut data = Vec::with_capacity(animation_key.size as usize);
    while data.len() < animation_key.size as usize {
        control.check()?;
        let count = (animation_key.size as usize - data.len()).min(4096);
        let page = source
            .read_hex_checked(animation_key, data.len() as u64, count, control)?
            .bytes;
        if page.len() != count {
            return Err("Short animation page".into());
        }
        data.extend(page);
    }
    let animation = animation::decode(&data, || control.check().is_err())
        .map_err(|e| format!("Animation: {e}"))?;
    control.check()?;
    source.check_stable()?;
    Ok(animation)
}

/// Resolves an animation GUID and checks its confirmed type.
pub fn resolve_animation(catalog: &Catalog, guid: u32) -> Result<LocationKey, String> {
    let rows = catalog
        .by_guid
        .get(&format!("{guid:08x}"))
        .ok_or("Animation GUID not in the snapshot")?;
    if !rows
        .iter()
        .any(|&i| catalog.snapshot.resources[i].display_type() == "animation")
    {
        return Err("GUID is not a confirmed animation".into());
    }
    crate::resolve(catalog, Some(guid), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translate(t: [f32; 3]) -> Mat4 {
        trs(t, [0., 0., 0., 1.], [1.; 3])
    }

    /// Two bones: root at origin, child at y = 1 (world).
    fn rig(hashes: &[u32]) -> Rig {
        Rig::new(
            vec![
                RigBone {
                    parent: -1,
                    anim_hash: 10,
                    skin_pose: IDENTITY,
                    inv_skin_pose: IDENTITY,
                },
                RigBone {
                    parent: 0,
                    anim_hash: 20,
                    skin_pose: translate([0., 1., 0.]),
                    inv_skin_pose: translate([0., -1., 0.]),
                },
            ],
            hashes,
        )
        .unwrap()
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    #[test]
    fn rebind_keeps_the_mesh_bones_and_matches_the_new_order() {
        let a = rig(&[10, 20]);
        let b = a.rebind(&[20, 99]).unwrap();
        assert_eq!(b.bone_count(), 2);
        assert_eq!(b.parents(), a.parents());
        assert_eq!(b.inverse_binds(), a.inverse_binds());
        assert_eq!(b.driven_by, [None, Some(0)]);
        assert_eq!(b.unmatched_animation_bones, 1);
    }

    #[test]
    fn bind_pose_is_identity_skinning() {
        let r = rig(&[]);
        for m in r.skin_matrices(None) {
            assert!(m.iter().zip(IDENTITY).all(|(a, b)| (a - b).abs() < 1e-6));
        }
    }

    #[test]
    fn rotating_the_root_moves_the_child_vertex() {
        // Animation lists the child first: matching is by hash, not order.
        let r = rig(&[20, 10, 99]);
        assert_eq!(r.driven_by, [Some(1), Some(0)]);
        assert_eq!(r.unmatched_animation_bones, 1);
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let pose = Pose {
            // Root: 90 degrees about z. Child: bind local translation (0, 1, 0).
            rotations: vec![[0., 0., 0., 1.], [0., 0., half, half], [0.; 4]],
            positions: vec![[0., 1., 0.], [0.; 3], [0.; 3]],
            scales: vec![[1.; 3]; 3],
            morphs: vec![],
        };
        let m = r.skin_matrices(Some(&pose));
        let out = skin_positions(
            &[[0., 2., 0.], [0., 2., 0.]],
            &[[1, 0, 0, 0], [0, 1, 0, 0]],
            &[[1., 0., 0., 0.], [0.5, 0.5, 0., 0.]],
            &m,
        );
        // Child-weighted vertex at (0, 2) rotates about the origin to (-2, 0).
        assert!(close(out[0], [-2., 0., 0.]), "{:?}", out[0]);
        assert!(close(out[1], [-2., 0., 0.]), "{:?}", out[1]);
    }

    #[test]
    fn scale_and_unweighted_vertices() {
        let r = rig(&[10]);
        let pose = Pose {
            rotations: vec![[0., 0., 0., 1.]],
            positions: vec![[1., 0., 0.]],
            scales: vec![[2.; 3]],
            morphs: vec![],
        };
        let m = r.skin_matrices(Some(&pose));
        let out = skin_positions(
            &[[1., 1., 1.], [5., 5., 5.], [3., 3., 3.]],
            &[[0, 0, 0, 0], [0, 0, 0, 0], [7, 0, 0, 0]],
            &[[1., 0., 0., 0.], [0.; 4], [1., 0., 0., 0.]],
            &m,
        );
        assert!(close(out[0], [3., 2., 2.]));
        assert_eq!(out[1], [5., 5., 5.]); // no weight
        assert_eq!(out[2], [3., 3., 3.]); // joint past the bone list
    }

    /// The viewer's GPU path: joint world = parent world Ã— local, matrix Ã— inverse bind.
    #[test]
    fn hierarchy_of_locals_reproduces_skin_matrices() {
        let r = rig(&[10, 20]);
        let pose = Pose {
            rotations: vec![[0., 0., 0.6, 0.8], [0.6, 0., 0., 0.8]],
            positions: vec![[1., 2., 3.], [0., 1., 0.]],
            scales: vec![[1.; 3], [0.5; 3]],
            morphs: vec![],
        };
        let local = r.local_matrices(Some(&pose));
        let parents = r.parents();
        let mut world = local.clone();
        for i in 0..world.len() {
            if let Ok(p) = usize::try_from(parents[i]) {
                world[i] = mul(&world[p], &local[i]);
            }
        }
        let inverse = r.inverse_binds();
        for (i, m) in r.skin_matrices(Some(&pose)).iter().enumerate() {
            let gpu = mul(&world[i], &inverse[i]);
            assert!(m.iter().zip(gpu).all(|(a, b)| (a - b).abs() < 1e-5));
        }
    }

    #[test]
    fn gpu_attributes_match_cpu_weight_rules() {
        let (j, w) = gpu_skin_attributes(
            &[[0, 1, 9, 0], [0, 0, 0, 0], [9, 0, 0, 0]],
            &[[0.5, 0.25, 0.25, 0.], [0.; 4], [1., 0., 0., 0.]],
            2,
        );
        assert_eq!(j[0], [0, 1, 0, 0]);
        assert!((w[0][0] - 2. / 3.).abs() < 1e-6 && (w[0][1] - 1. / 3.).abs() < 1e-6);
        assert_eq!((j[1], w[1]), ([2, 0, 0, 0], [1., 0., 0., 0.]));
        assert_eq!((j[2], w[2]), ([2, 0, 0, 0], [1., 0., 0., 0.]));
    }

    fn pose1(r: [f32; 4], t: [f32; 3], s: [f32; 3], morphs: Vec<f32>) -> Pose {
        Pose {
            rotations: vec![r],
            positions: vec![t],
            scales: vec![s],
            morphs,
        }
    }

    #[test]
    fn trs_round_trips_through_a_matrix() {
        let q = normalize([0.3, -0.5, 0.2, 0.7]);
        for s in [[1., 2., 0.5], [-1., 1., 1.]] {
            let back = Trs::from_matrix(&trs([1., -2., 3.], q, s));
            assert!(close(back.t, [1., -2., 3.]));
            assert!(close(back.s, s), "{:?}", back.s);
            let m = back.matrix();
            assert!(m
                .iter()
                .zip(trs([1., -2., 3.], q, s))
                .all(|(a, b)| (a - b).abs() < 1e-5));
        }
        // Bind pose of the two-bone rig: identity root, child up 1.
        let b = BlendPose::bind(&rig(&[10, 20]));
        assert!(close(b.bones[1].t, [0., 1., 0.]));
        assert!(close(b.bones[1].s, [1.; 3]));
    }

    #[test]
    fn lerp_layers_blend_per_bone_and_skip_undriven_bones() {
        // The animation drives only the child (hash 20).
        let r = rig(&[20]);
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let quarter = pose1([0., 0., half, half], [2., 1., 0.], [3.; 3], vec![1., 0.5]);
        let mut b = BlendPose::bind(&r);
        b.lerp(&r, &quarter, 0.5);
        // Root untouched; child halfway in translation, scale and rotation (45 degrees).
        assert_eq!(b.bones[0], BlendPose::bind(&r).bones[0]);
        assert!(close(b.bones[1].t, [1., 1., 0.]));
        assert!(close(b.bones[1].s, [2.; 3]));
        let a = std::f32::consts::FRAC_PI_8;
        let want = [0., 0., a.sin(), a.cos()];
        assert!(b.bones[1]
            .r
            .iter()
            .zip(want)
            .all(|(x, y)| (x - y).abs() < 1e-5));
        assert_eq!(b.morphs, [0.5, 0.25]);
        // A full-weight layer replaces; a zero weight changes nothing.
        let mut c = b.clone();
        c.lerp(&r, &quarter, 0.);
        assert_eq!(c, b);
        c.lerp(&r, &quarter, 1.);
        assert!(close(c.bones[1].t, [2., 1., 0.]));
        // Opposite-sign quaternions take the short way.
        let mut d = BlendPose::bind(&r);
        d.lerp(
            &r,
            &pose1([0., 0., 0., -1.], [0., 1., 0.], [1.; 3], vec![]),
            0.5,
        );
        assert!((d.bones[1].r[3].abs() - 1.).abs() < 1e-5);
    }

    #[test]
    fn additive_layer_adds_the_difference_from_its_reference() {
        let r = rig(&[20]);
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let reference = pose1([0., 0., 0., 1.], [0., 1., 0.], [1.; 3], vec![0.2]);
        let blink = pose1([0., 0., half, half], [0.5, 1., 0.], [2.; 3], vec![0.7]);
        let mut b = BlendPose::bind(&r);
        b.lerp(
            &r,
            &pose1([0., 0., 0., 1.], [1., 1., 0.], [1.; 3], vec![]),
            1.,
        );
        b.add(&r, &blink, &reference, 1.);
        assert!(close(b.bones[1].t, [1.5, 1., 0.]));
        assert!(close(b.bones[1].s, [2.; 3]));
        assert!(b.bones[1]
            .r
            .iter()
            .zip([0., 0., half, half])
            .all(|(x, y)| (x - y).abs() < 1e-5));
        assert!((b.morphs[0] - 0.5).abs() < 1e-6);
        // Frame equal to its reference: no change.
        let before = b.clone();
        b.add(&r, &reference, &reference, 1.);
        assert!(close(b.bones[1].t, before.bones[1].t));
    }

    #[test]
    fn blended_locals_match_skin_matrices_for_a_single_pose() {
        let r = rig(&[10, 20]);
        let pose = Pose {
            rotations: vec![[0., 0., 0.6, 0.8], [0.6, 0., 0., 0.8]],
            positions: vec![[1., 2., 3.], [0., 1., 0.]],
            scales: vec![[1.; 3], [0.5; 3]],
            morphs: vec![],
        };
        let mut b = BlendPose::bind(&r);
        b.lerp(&r, &pose, 1.);
        let blended = r.skin_matrices_from_locals(&b.local_matrices());
        for (m, n) in blended.iter().zip(r.skin_matrices(Some(&pose))) {
            assert!(m.iter().zip(n).all(|(a, b)| (a - b).abs() < 1e-5));
        }
    }

    #[test]
    fn hold_root_rotation_keeps_only_the_roots_translation() {
        let r = rig(&[10, 20]);
        let pose = Pose {
            rotations: vec![[0., 0., 0.6, 0.8], [0.6, 0., 0., 0.8]],
            positions: vec![[1., 2., 3.], [0., 1., 0.]],
            scales: vec![[1.; 3], [1.; 3]],
            morphs: vec![],
        };
        let mut b = BlendPose::bind(&r);
        b.lerp(&r, &pose, 1.);
        b.hold_root_rotation(&r);
        assert_eq!(b.bones[0].r, [0., 0., 0., 1.]);
        assert!(close(b.bones[0].t, [1., 2., 3.]));
        assert_eq!(b.bones[1].r, [0.6, 0., 0., 0.8]);
    }

    #[test]
    fn mirrored_pose_swaps_partners_and_reflects_x() {
        // Root at the origin, a left bone at x = +1 and a right bone at x = -1.
        let side = |x: f32| RigBone {
            parent: 0,
            anim_hash: if x > 0. { 20 } else { 30 },
            skin_pose: translate([x, 0., 0.]),
            inv_skin_pose: translate([-x, 0., 0.]),
        };
        let root = RigBone {
            parent: -1,
            anim_hash: 10,
            skin_pose: IDENTITY,
            inv_skin_pose: IDENTITY,
        };
        let r = Rig::new(vec![root.clone(), side(1.), side(-1.)], &[10, 20, 30])
            .unwrap()
            .with_mirror(&[0, 2, 1]);
        assert_eq!(r.mirror, [0, 2, 1]);
        // A broken table is ignored.
        assert_eq!(
            Rig::new(vec![root], &[]).unwrap().with_mirror(&[5]).mirror,
            [0]
        );
        // Root turned +z, the left bone raised and turned about y, the right at bind.
        let (s, c) = (0.3f32.sin(), 0.3f32.cos());
        let pose = Pose {
            rotations: vec![[0., 0., s, c], [0., s, 0., c], [0., 0., 0., 1.]],
            positions: vec![[0.5, 0., 0.], [1., 0., 2.], [-1., 0., 0.]],
            scales: vec![[1.; 3]; 3],
            morphs: vec![],
        };
        let m = r.mirrored_trs(&pose);
        let near = |a: [f32; 4], b: [f32; 4]| {
            let d: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
            (d.abs() - 1.).abs() < 1e-5
        };
        let root = m[0].unwrap();
        assert!(close(root.t, [-0.5, 0., 0.]) && near(root.r, [0., 0., -s, c]));
        // The right bone now does what the left did, reflected; the left rests at bind.
        let right = m[2].unwrap();
        assert!(close(right.t, [-1., 0., 2.]), "{:?}", right.t);
        assert!(near(right.r, [0., -s, 0., c]), "{:?}", right.r);
        let left = m[1].unwrap();
        assert!(close(left.t, [1., 0., 0.]) && near(left.r, [0., 0., 0., 1.]));
        // Mirroring twice gives the pose back.
        let back = Pose {
            rotations: m.iter().map(|t| t.unwrap().r).collect(),
            positions: m.iter().map(|t| t.unwrap().t).collect(),
            scales: m.iter().map(|t| t.unwrap().s).collect(),
            morphs: vec![],
        };
        let twice = r.mirrored_trs(&back);
        for (t, (q, p)) in twice.iter().zip(pose.rotations.iter().zip(&pose.positions)) {
            let t = t.unwrap();
            assert!(close(t.t, *p) && near(t.r, *q));
        }
        // D246: under the Biped rule a scaled bone's partner takes the scale, and only it.
        let mut biped = r.clone();
        biped.set_biped_bones(&[true; 3]);
        biped.scale_rule = ScaleRule::Biped;
        let scaled = Pose {
            scales: vec![[1.; 3], [2., 1., 3.], [1.; 3]],
            ..pose.clone()
        };
        let m = biped.mirrored_trs(&scaled);
        let (left, right) = (m[1].unwrap(), m[2].unwrap());
        assert!(close(right.s, [2., 1., 3.]) && close(left.s, [1.; 3]));
        assert!(close(right.t, [-1., 0., 2.]) && near(right.r, [0., -s, 0., c]));
    }

    #[test]
    fn mirrored_pose_follows_the_flip_types() {
        let bone = |parent: i32, hash: u32| RigBone {
            parent,
            anim_hash: hash,
            skin_pose: IDENTITY,
            inv_skin_pose: IDENTITY,
        };
        // A pelvis-type root, a left/right pair of Biped bones and a copied centre bone.
        let r = Rig::new(
            vec![bone(-1, 10), bone(0, 20), bone(0, 30), bone(0, 40)],
            &[10, 20, 30, 40],
        )
        .unwrap()
        .with_mirror(&[0, 2, 1, 3])
        .with_flip_types(&[2, 3, 3, 6]);
        assert!(r.clone().with_flip_types(&[3]).flip == r.flip);
        let (s, c) = (0.3f32.sin(), 0.3f32.cos());
        let pose = Pose {
            rotations: vec![
                [0., 0., 0., 1.],
                [s, 0., 0., c],
                [0., 0., s, c],
                [0., s, 0., c],
            ],
            positions: vec![[1., 2., 3.], [4., 5., 6.], [7., 8., 9.], [1., 1., 1.]],
            scales: vec![[1.; 3], [2., 1., 3.], [1.; 3], [1.; 3]],
            morphs: vec![],
        };
        let m: Vec<Trs> = r.mirrored_trs(&pose).into_iter().flatten().collect();
        assert_eq!((m[0].t, m[0].r), ([1., -2., 3.], [-1., 0., 0., 0.]));
        // A Biped bone takes its partner's z turn as it is and its x turn negated, with
        // the offset reflected on z and the scale copied.
        assert_eq!(
            (m[1].t, m[1].r, m[1].s),
            ([7., 8., -9.], [0., 0., s, c], [1.; 3])
        );
        assert_eq!(
            (m[2].t, m[2].r, m[2].s),
            ([4., 5., -6.], [-s, 0., 0., c], [2., 1., 3.])
        );
        assert_eq!((m[3].t, m[3].r), ([1., 1., 1.], [0., s, 0., c]));
    }

    #[test]
    fn mirrored_pose_swaps_morph_weights() {
        let bone = RigBone {
            parent: -1,
            anim_hash: 10,
            skin_pose: IDENTITY,
            inv_skin_pose: IDENTITY,
        };
        let r = Rig::new(vec![bone], &[10]).unwrap();
        assert_eq!(r.mirrored_morphs(&[0.1, 0.2, 0.3]), [0.1, 0.2, 0.3]);
        // Morphs 1 and 2 are a pair; entry 3 has no weight to take, entry 4 is unset.
        let r = r.with_mirror_morphs(&[0, 2, 1, 9, -1]);
        assert_eq!(
            r.mirrored_morphs(&[0.1, 0.2, 0.3, 0.4, 0.5, 0.6]),
            [0.1, 0.3, 0.2, 0.4, 0.5, 0.6]
        );
        let pose = Pose {
            rotations: vec![[0., 0., 0., 1.]],
            positions: vec![[0.; 3]],
            scales: vec![[1.; 3]],
            morphs: vec![0., 1., 0.],
        };
        let mut b = BlendPose::bind(&r);
        b.lerp_mirrored(&r, &pose, 0.5);
        assert_eq!(b.morphs, [0., 0., 0.5]);
    }

    /// Root, a bone at y = 1 and its child at y = 2 (world), all driven.
    fn chain() -> Rig {
        let at = |parent: i32, hash: u32, y: f32| RigBone {
            parent,
            anim_hash: hash,
            skin_pose: translate([0., y, 0.]),
            inv_skin_pose: translate([0., -y, 0.]),
        };
        Rig::new(
            vec![at(-1, 10, 0.), at(0, 20, 1.), at(1, 30, 2.)],
            &[10, 20, 30],
        )
        .unwrap()
    }

    #[test]
    fn move_chain_end_reaches_the_target_and_keeps_the_end_rotation() {
        // A leg hanging down z from a pelvis: hip at z 3, knee bent forward (-y), ankle at 1.
        let at = |parent: i32, p: [f32; 3]| RigBone {
            parent,
            anim_hash: 0,
            skin_pose: translate(p),
            inv_skin_pose: translate([-p[0], -p[1], -p[2]]),
        };
        let rig = Rig::new(
            vec![
                at(-1, [0., 0., 3.5]),
                at(0, [0.5, 0., 3.]),
                at(1, [0.5, -0.2, 2.]),
                at(2, [0.5, 0., 1.]),
            ],
            &[],
        )
        .unwrap();
        let joint = |p: &BlendPose, i: usize| {
            let w = rig.worlds_from_trs(&p.bones)[i];
            [w[12], w[13], w[14]]
        };
        let foot_rotation = |p: &BlendPose| Trs::from_matrix(&rig.worlds_from_trs(&p.bones)[3]).r;
        for rule in [ScaleRule::Inherited, ScaleRule::Biped] {
            let mut rig = rig.clone();
            rig.set_biped_bones(&[true; 4]);
            rig.scale_rule = rule;
            let mut pose = BlendPose::bind(&rig);
            // Not a chain: nothing happens.
            assert!(!pose.move_chain_end(&rig, [0, 2, 3], [0., 0., 0.5]));
            assert_eq!(pose, BlendPose::bind(&rig));
            assert!(pose.move_chain_end(&rig, [1, 2, 3], [0.1, 0., 0.5]));
            assert!(close(joint(&pose, 1), [0.5, 0., 3.]));
            assert!(
                close(joint(&pose, 3), [0.6, 0., 1.5]),
                "{:?}",
                joint(&pose, 3)
            );
            // Lengths kept, the knee further forward, the foot not turned.
            let knee = joint(&pose, 2);
            assert!((len3(sub3(knee, [0.5, 0., 3.])) - len3([0., 0.2, 1.])).abs() < 1e-4);
            assert!(knee[1] < -0.2, "{knee:?}");
            let r = foot_rotation(&pose);
            assert!(r[3].abs() > 0.9999, "{r:?}");
            // Out of reach: the leg ends up straight toward the target.
            let mut far = BlendPose::bind(&rig);
            assert!(far.move_chain_end(&rig, [1, 2, 3], [0., 0., -5.]));
            let reach = 2. * len3([0., 0.2, 1.]);
            assert!((joint(&far, 3)[2] - (3. - reach)).abs() < 0.01);
        }
    }

    #[test]
    fn masked_lerp_moves_only_the_flagged_subtree() {
        // Root -> a -> b -> c: the subtree of a without c's.
        let at = |parent: i32, hash: u32| RigBone {
            parent,
            anim_hash: hash,
            skin_pose: IDENTITY,
            inv_skin_pose: IDENTITY,
        };
        let r = Rig::new(vec![at(-1, 1), at(0, 2), at(1, 3), at(2, 4)], &[1, 2, 3, 4]).unwrap();
        let mask = r.subtree_mask(1, Some(3));
        assert_eq!(mask, [false, true, true, false]);
        assert_eq!(r.subtree_mask(2, None), [false, false, true, true]);
        let pose = Pose {
            rotations: vec![[0., 0., 0., 1.]; 4],
            positions: vec![[4., 0., 0.]; 4],
            scales: vec![[1.; 3]; 4],
            morphs: vec![1.],
        };
        let mut p = BlendPose::bind(&r);
        p.lerp_masked(&r, &pose, 0.5, &mask);
        let x: Vec<f32> = p.bones.iter().map(|b| b.t[0]).collect();
        assert_eq!(x, [0., 2., 2., 0.]);
        assert!(p.morphs.is_empty());
    }

    #[test]
    fn biped_scale_stays_on_the_scaled_bone() {
        // The middle bone doubles along y; its child turns 90 degrees about z.
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let pose = Pose {
            rotations: vec![[0., 0., 0., 1.], [0., 0., 0., 1.], [0., 0., h, h]],
            positions: vec![[0.; 3], [0., 1., 0.], [0., 1., 0.]],
            scales: vec![[1.; 3], [1., 2., 1.], [1.; 3]],
            morphs: vec![],
        };
        let positions = [[0., 1.5, 0.], [0.5, 2.5, 0.]];
        let joints = [[1, 0, 0, 0], [2, 0, 0, 0]];
        let weights = [[1., 0., 0., 0.]; 2];
        let skinned =
            |r: &Rig| skin_positions(&positions, &joints, &weights, &r.skin_matrices(Some(&pose)));
        // Plain inheritance: the child is stretched and sheared by its parent's scale.
        let plain = chain();
        let out = skinned(&plain);
        assert!(close(out[0], [0., 2., 0.]) && close(out[1], [-0.5, 4., 0.]));
        let mut wide = plain.clone();
        wide.set_biped_bones(&[true; 3]);
        // The flags do nothing until the rule is chosen.
        assert_eq!(skinned(&wide), out);
        wide.scale_rule = ScaleRule::Biped;
        // The scaled bone's own vertex doubles its offset; the child's joint moves out to
        // y = 3 and its vertex keeps its bind offset (0.5, 0.5), turned by the child only.
        let out = skinned(&wide);
        assert!(close(out[0], [0., 2., 0.]) && close(out[1], [-0.5, 3.5, 0.]));
        // Same child, a quarter turn the other way round an x-scaled parent: no shear.
        let across = Pose {
            scales: vec![[1.; 3], [3., 1., 1.], [1.; 3]],
            ..pose.clone()
        };
        let w = wide.worlds_from_trs(&wide.pose_locals(Some(&across)));
        let col = |m: &Mat4, c: usize| [m[c * 4], m[c * 4 + 1], m[c * 4 + 2]];
        assert!(close(col(&w[1], 0), [3., 0., 0.]));
        assert!(close(col(&w[2], 0), [0., 1., 0.]) && close(col(&w[2], 1), [-1., 0., 0.]));
        assert!(close(col(&w[2], 3), [0., 2., 0.]));
        let inherited = plain.worlds_from_trs(&plain.pose_locals(Some(&across)));
        assert!(close(col(&inherited[2], 1), [-3., 0., 0.]));
        // A bone that is not a Biped part still inherits (Sackboy's zip pull under the chest).
        let mut mixed = wide.clone();
        mixed.set_biped_bones(&[true, true, false]);
        let w = mixed.worlds_from_trs(&mixed.pose_locals(Some(&across)));
        assert!(close(col(&w[2], 1), [-3., 0., 0.]));
        // Bind stays bind under either rule.
        for r in [&wide, &mixed] {
            for m in r.skin_matrices(None) {
                assert!(m.iter().zip(IDENTITY).all(|(a, b)| (a - b).abs() < 1e-6));
            }
        }
    }

    #[test]
    fn frame_and_scale_nodes_reproduce_the_biped_worlds() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let pose = Pose {
            rotations: vec![[0., 0.6, 0., 0.8], [0.6, 0., 0., 0.8], [0., 0., h, h]],
            positions: vec![[0.5, 0., 0.], [0., 1., 0.2], [0.3, 1., 0.]],
            scales: vec![[1.2, 1., 0.8], [0.9, 2., 1.], [1., 0.5, 1.5]],
            morphs: vec![],
        };
        for flags in [[true; 3], [true, true, false], [false, true, true]] {
            let mut r = chain();
            r.set_biped_bones(&flags);
            r.scale_rule = ScaleRule::Biped;
            let local = r.pose_locals(Some(&pose));
            let inherits = r.joint_inherits();
            assert_eq!(inherits, flags.map(|f| !f));
            // The joint hierarchy a renderer builds: frame node, then its scale node.
            let (mut frames, mut scaled) = (Vec::<Mat4>::new(), Vec::<Mat4>::new());
            for (i, (m, k)) in r.joint_locals(&local).into_iter().enumerate() {
                let above = match (r.parents()[i], inherits[i]) {
                    (-1, _) => IDENTITY,
                    (p, true) => scaled[p as usize],
                    (p, false) => frames[p as usize],
                };
                frames.push(mul(&above, &m));
                scaled.push(mul(&frames[i], &trs([0.; 3], [0., 0., 0., 1.], k)));
            }
            for (a, b) in scaled.iter().zip(r.worlds_from_trs(&local)) {
                assert!(
                    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5),
                    "{flags:?}"
                );
            }
        }
        // Plain inheritance: one node per bone does it (unit scale nodes, all inheriting).
        let r = chain();
        let local = r.pose_locals(Some(&pose));
        assert!(r.joint_inherits().iter().all(|&i| i));
        for ((m, k), l) in r.joint_locals(&local).iter().zip(&local) {
            assert_eq!((*m, *k), (l.matrix(), [1.; 3]));
        }
    }

    #[test]
    fn rejects_bad_hierarchies() {
        let bone = |parent| RigBone {
            parent,
            anim_hash: 0,
            skin_pose: IDENTITY,
            inv_skin_pose: IDENTITY,
        };
        assert!(Rig::new(vec![], &[]).is_err());
        assert!(Rig::new(vec![bone(5)], &[]).is_err());
        assert!(Rig::new(vec![bone(1), bone(0)], &[]).is_err());
        assert!(Rig::new(vec![bone(-1), bone(0), bone(1)], &[]).is_ok());
    }
}
