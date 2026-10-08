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

/// Translation × rotation (unit quaternion x, y, z, w) × scale.
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

fn transform(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
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
}

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
        Ok(Self {
            bones,
            chains,
            bind_local,
            driven_by,
            unmatched_animation_bones,
        })
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
        Self::new(bones, animation_hashes)
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    pub fn matched_bones(&self) -> usize {
        self.driven_by.iter().flatten().count()
    }

    /// Per mesh bone: world(pose) × inverse skin pose. Undriven bones keep their bind
    /// local transform, so a pose without matches reproduces the bind pose.
    pub fn skin_matrices(&self, pose: Option<&Pose>) -> Vec<Mat4> {
        let local = self.local_matrices(pose);
        self.chains
            .iter()
            .zip(&self.bones)
            .map(|(chain, bone)| {
                let world = chain.iter().fold(IDENTITY, |acc, &j| mul(&acc, &local[j]));
                mul(&world, &bone.inv_skin_pose)
            })
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
        if rig.matched_bones() == 0 {
            return Err("Unsupported: animation drives none of this mesh's bones".into());
        }
        let p = &animation.inspection.prefix;
        let report = ClipReport {
            mesh_bones: rig.bone_count(),
            animation_bones: hashes.len(),
            matched_bones: rig.matched_bones(),
            unmatched_animation_bones: rig.unmatched_animation_bones,
            mesh_morphs: skin.morphs.len(),
            animation_morphs: usize::from(p.morph_count_raw_u8),
            frames: animation.frame_count(),
            fps: p.fps_raw_u16,
            looped: p.loop_start_raw_u16 < animation.frame_count(),
        };
        Ok(Self {
            rig,
            skin,
            animation,
            bind_positions: geometry.positions.clone(),
            report,
        })
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
    SkinnedClip::new(&geometry, skin, animation)
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

    /// The viewer's GPU path: joint world = parent world × local, matrix × inverse bind.
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
