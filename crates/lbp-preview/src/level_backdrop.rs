//! D119: level backdrop for the renderer-independent level scene (no Bevy types).
//!
//! How a level names its backdrop (docs/d119-level-backdrop.md; Toolkit d6a2ef80
//! `structs/things/parts/PWorld.java`, `PLevelSettings.java`, `resources/RLevel.java`,
//! CONFIRMED_FROM_COMMUNITY_CODE, and CONFIRMED_FROM_LOCAL_DATA on all 236 in-band 01.33
//! levels): PWorld `backdrop` references a Thing in the level (POS + LEVEL_SETTINGS, often
//! GROUP) whose planGUID equals PWorld `backdropPlan` (235/236 levels). That Thing's
//! PLevelSettings `backdropMesh` names a prebaked STATIC_MESH (185 levels); the plan's own
//! Things (props, lights, the settings Thing) are not stored in the level and are read
//! from the plan resource.
//!
//! Drawn here:
//! - the static mesh: primitives merged per gfx material into batches (adapter limits),
//!   placed at the backdrop Thing's POS world matrix (identity when it has no POS);
//! - the plan's RENDER_MESH Things, through the same `build_scene` path as level Things,
//!   placed at backdrop world x inverse(plan settings-Thing world) x Thing world
//!   (a placement approximation; both matrices are identity on the tested levels).
//!   Plan SHAPE Things and the plan copy of `backdropMesh` are counted, not drawn.
use crate::level_scene::{
    build_scene, world_matrix_check, LevelScene, MeshSource, SceneItem, SceneMesh,
    MAX_BATCH_TRIANGLES, MAX_BATCH_VERTICES, MAX_LEVEL_ITEMS,
};
use lbp_level::thing::ThingRecord;
use lbp_mesh::static_mesh::{StaticDescriptor, StaticMesh};
use std::collections::{BTreeMap, HashMap};

/// Merged static-mesh primitives sharing one gfx material (mesh space).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackdropBatch {
    pub material: Option<StaticDescriptor>,
    pub positions: Vec<[f32; 3]>,
    pub uv0: Vec<[f32; 2]>,
    pub uv1: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Static-mesh primitive indices merged into this batch.
    pub primitives: Vec<usize>,
}

/// What the level names as its backdrop (PWorld + backdrop Thing).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackdropRefs {
    pub thing: Option<usize>,
    /// Backdrop Thing POS world matrix (None = no POS).
    pub world: Option<[f32; 16]>,
    /// PLevelSettings backdropMesh: GUID, else SHA-1 (lowercase hex).
    pub mesh: Option<MeshSource>,
    /// PWorld backdropPlan GUID.
    pub plan: Option<u32>,
}

/// Counting units: static-mesh primitives, plan Things, batches; reasons keyed by text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackdropCounts {
    /// "none" when the level has no backdrop reference; else a short source line.
    pub source: Option<String>,
    /// Backdrop Thing index: every backdrop scene item carries it as `SceneItem::thing`.
    pub thing: Option<usize>,
    pub mesh_primitives: usize,
    pub mesh_primitives_drawn: usize,
    pub mesh_triangles_drawn: usize,
    pub mesh_batches: usize,
    pub mesh_primitives_skipped: BTreeMap<String, usize>,
    /// Drawn primitives whose gfx material did not select (flat neutral colour).
    pub mesh_primitives_fallback: BTreeMap<String, usize>,
    pub plan_things: usize,
    pub plan_render_mesh: usize,
    pub plan_drawn: usize,
    pub plan_skipped: BTreeMap<String, usize>,
    pub plan_meshes_loaded: usize,
    pub plan_meshes_failed: usize,
    /// Plan SHAPE Things (not drawn) and plan Things carrying their own backdropMesh.
    pub plan_shapes_not_drawn: usize,
    /// Whole-route failures (mesh or plan unavailable / unreadable).
    pub errors: Vec<String>,
}

impl BackdropCounts {
    pub fn lines(&self) -> Vec<String> {
        let Some(source) = &self.source else {
            return vec!["Backdrop: not requested".into()];
        };
        let mut out = vec![format!("Backdrop: {source}")];
        out.push(format!(
            "  static mesh: {} of {} primitives drawn ({} triangles) in {} material batches",
            self.mesh_primitives_drawn,
            self.mesh_primitives,
            self.mesh_triangles_drawn,
            self.mesh_batches
        ));
        for (reason, n) in &self.mesh_primitives_skipped {
            out.push(format!("  backdrop primitive skipped {n}: {reason}"));
        }
        let fallback: usize = self.mesh_primitives_fallback.values().sum();
        out.push(format!(
            "  backdrop primitives with flat fallback colour: {fallback}"
        ));
        for (reason, n) in &self.mesh_primitives_fallback {
            out.push(format!("    fallback {n}: {reason}"));
        }
        out.push(format!(
            "  plan: {} Things, {} with RENDER_MESH, {} drawn; meshes {} loaded, {} failed; {} SHAPE Things not drawn",
            self.plan_things,
            self.plan_render_mesh,
            self.plan_drawn,
            self.plan_meshes_loaded,
            self.plan_meshes_failed,
            self.plan_shapes_not_drawn
        ));
        for (reason, n) in &self.plan_skipped {
            out.push(format!("  backdrop plan skipped {n}: {reason}"));
        }
        for e in &self.errors {
            out.push(format!("  backdrop unavailable: {e}"));
        }
        out
    }
}

/// Finds the backdrop references from the reader output (the Thing holding PWorld).
pub fn backdrop_refs(things: &[ThingRecord]) -> BackdropRefs {
    let Some(world) = things.iter().find_map(|t| t.data.world.as_ref()) else {
        return BackdropRefs::default();
    };
    let thing = world.backdrop.and_then(|i| things.get(i));
    let mesh = thing
        .and_then(|t| t.data.level_settings.as_ref())
        .and_then(|l| l.backdrop_mesh.as_ref())
        .and_then(|d| match (d.guid, d.sha1) {
            (Some(g), _) if g != 0 => Some(MeshSource::Guid(g)),
            (_, Some(h)) if h.iter().any(|&b| b != 0) => Some(MeshSource::Sha1(
                h.iter().map(|b| format!("{b:02x}")).collect(),
            )),
            _ => None,
        });
    BackdropRefs {
        thing: thing.map(|t| t.index),
        world: thing.and_then(|t| t.data.pos.as_ref()).map(|p| p.world),
        mesh,
        plan: world.backdrop_plan.as_ref().and_then(|d| d.guid),
    }
}

pub const IDENTITY: [f32; 16] = lbp_level::shape_mesh::IDENTITY;

/// Column-major 4x4 product a * b.
pub fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let (a, b) = (a.map(f64::from), b.map(f64::from));
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum::<f64>() as f32
    })
}

/// Inverse of an affine column-major matrix (checked by `world_matrix_check`).
pub fn affine_inverse(m: &[f32; 16]) -> Result<[f32; 16], String> {
    world_matrix_check(m)?;
    let a = m.map(f64::from);
    let at = |r: usize, c: usize| a[c * 4 + r];
    let det = at(0, 0) * (at(1, 1) * at(2, 2) - at(1, 2) * at(2, 1))
        - at(0, 1) * (at(1, 0) * at(2, 2) - at(1, 2) * at(2, 0))
        + at(0, 2) * (at(1, 0) * at(2, 1) - at(1, 1) * at(2, 0));
    let mut inv = [[0f64; 3]; 3];
    for (r, row) in inv.iter_mut().enumerate() {
        for (c, v) in row.iter_mut().enumerate() {
            // cofactor of (c, r) / det
            let (r1, r2) = ((c + 1) % 3, (c + 2) % 3);
            let (c1, c2) = ((r + 1) % 3, (r + 2) % 3);
            *v = (at(r1, c1) * at(r2, c2) - at(r1, c2) * at(r2, c1)) / det;
        }
    }
    let t = [at(0, 3), at(1, 3), at(2, 3)];
    let mut out = [0f32; 16];
    for r in 0..3 {
        for c in 0..3 {
            out[c * 4 + r] = inv[r][c] as f32;
        }
        out[12 + r] = -(0..3).map(|k| inv[r][k] * t[k]).sum::<f64>() as f32;
    }
    out[15] = 1.;
    Ok(out)
}

/// Triangulated primitives (index, absolute triangles) per gfx material.
type PrimitiveGroups = BTreeMap<Option<StaticDescriptor>, Vec<(usize, Vec<[u32; 3]>)>>;

/// Adds the static mesh as per-material batches placed at `world`. `wrap` turns a batch
/// into the payload and reports whether its material selected.
#[allow(clippy::too_many_arguments)]
pub fn add_backdrop_mesh<T>(
    scene: &mut LevelScene<T>,
    counts: &mut BackdropCounts,
    mesh_guid: Option<u32>,
    mesh: &StaticMesh,
    world: [f32; 16],
    thing: usize,
    mut wrap: impl FnMut(BackdropBatch) -> (T, Result<(), String>),
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
) {
    let skip = |c: &mut BackdropCounts, reason: String| {
        *c.mesh_primitives_skipped.entry(reason).or_default() += 1
    };
    counts.mesh_primitives += mesh.primitives.len();
    if let Err(e) = world_matrix_check(&world) {
        *counts
            .mesh_primitives_skipped
            .entry(format!("backdrop Thing: {e}"))
            .or_default() += mesh.primitives.len();
        return;
    }
    let mut groups: PrimitiveGroups = BTreeMap::new();
    for (i, p) in mesh.primitives.iter().enumerate() {
        match mesh.primitive_triangles(i) {
            Ok(t) if t.is_empty() => skip(counts, "no nondegenerate triangles".into()),
            Ok(t) if t.len() > MAX_BATCH_TRIANGLES || t.len() * 3 > MAX_BATCH_VERTICES => {
                skip(counts, "primitive exceeds display batch limits".into())
            }
            Ok(t) => groups.entry(p.material.clone()).or_default().push((i, t)),
            Err(e) => skip(counts, e.to_string()),
        }
    }
    for (material, prims) in groups {
        let mut batches = vec![BackdropBatch {
            material: material.clone(),
            ..Default::default()
        }];
        let mut remap: HashMap<u32, u32> = HashMap::new();
        for (index, tris) in prims {
            let fresh = {
                let mut seen = std::collections::HashSet::new();
                tris.iter()
                    .flatten()
                    .filter(|v| !remap.contains_key(v) && seen.insert(**v))
                    .count()
            };
            let b = batches.last().expect("one batch");
            if b.positions.len() + fresh > MAX_BATCH_VERTICES
                || b.triangles.len() + tris.len() > MAX_BATCH_TRIANGLES
            {
                batches.push(BackdropBatch {
                    material: material.clone(),
                    ..Default::default()
                });
                remap.clear();
            }
            let b = batches.last_mut().expect("one batch");
            for t in tris {
                let local = t.map(|v| {
                    *remap.entry(v).or_insert_with(|| {
                        let k = v as usize;
                        b.positions.push(mesh.positions[k]);
                        b.uv0.push(mesh.uv0[k]);
                        b.uv1.push(mesh.uv1[k]);
                        (b.positions.len() - 1) as u32
                    })
                });
                b.triangles.push(local);
            }
            b.primitives.push(index);
        }
        for (batch_index, batch) in batches.into_iter().enumerate() {
            let n = batch.primitives.len();
            let tris = batch.triangles.len();
            let (payload, selected) = wrap(batch);
            if scene.items.len() >= MAX_LEVEL_ITEMS {
                *counts
                    .mesh_primitives_skipped
                    .entry("level item budget".into())
                    .or_default() += n;
                continue;
            }
            if let Err(e) = &selected {
                *counts
                    .mesh_primitives_fallback
                    .entry(e.clone())
                    .or_default() += n;
            }
            counts.mesh_primitives_drawn += n;
            counts.mesh_triangles_drawn += tris;
            counts.mesh_batches += 1;
            scene.meshes.push(SceneMesh {
                source: MeshSource::Backdrop {
                    mesh: mesh_guid,
                    material: material.as_ref().and_then(|d| d.guid),
                    batch: batch_index,
                },
                bounds: local_bounds(&payload),
                neutral_groups: selected.is_err(),
                payload,
            });
            scene.items.push(SceneItem {
                mesh: scene.meshes.len() - 1,
                world,
                thing,
                uid: None,
                shapes: 0,
                decoration: false,
                alternate: false,
            });
        }
    }
}

/// Adds the plan's RENDER_MESH Things through `build_scene` (same loader, skip rules and
/// budgets), placed by `backdrop_world` x inverse(settings-Thing world).
#[allow(clippy::too_many_arguments)]
pub fn add_backdrop_plan<T>(
    scene: &mut LevelScene<T>,
    counts: &mut BackdropCounts,
    plan_things: &[ThingRecord],
    backdrop_world: [f32; 16],
    thing: usize,
    load: impl FnMut(&MeshSource) -> Result<T, String>,
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: impl Fn(&T) -> bool,
    bind_poses: impl Fn(&T) -> Vec<crate::level_scene::BindPose>,
    placement: crate::level_scene::PosPlacement,
) {
    counts.plan_things += plan_things.len();
    counts.plan_shapes_not_drawn += plan_things
        .iter()
        .filter(|t| t.data.shape.is_some())
        .count();
    let root = plan_things
        .iter()
        .find(|t| t.data.level_settings.is_some() && t.data.pos.is_some())
        .and_then(|t| t.data.pos.as_ref())
        .map_or(IDENTITY, |p| p.world);
    let to_level = match affine_inverse(&root) {
        Ok(inv) => mul(&backdrop_world, &inv),
        Err(e) => {
            counts
                .errors
                .push(format!("plan settings Thing matrix: {e}"));
            return;
        }
    };
    let sub = build_scene(
        plan_things,
        load,
        local_bounds,
        neutral_groups,
        bind_poses,
        placement,
    );
    let c = &sub.counts;
    counts.plan_render_mesh += c.with_render_mesh;
    counts.plan_meshes_loaded += c.meshes_loaded;
    counts.plan_meshes_failed += c.meshes_failed;
    for (k, n) in &c.skipped {
        *counts.plan_skipped.entry(k.clone()).or_default() += n;
    }
    let base = scene.meshes.len();
    scene.meshes.extend(sub.meshes);
    for item in sub.items {
        let world = mul(&to_level, &item.world);
        if scene.items.len() >= MAX_LEVEL_ITEMS {
            *counts
                .plan_skipped
                .entry("level item budget".into())
                .or_default() += 1;
            continue;
        }
        if let Err(e) = world_matrix_check(&world) {
            *counts.plan_skipped.entry(e).or_default() += 1;
            continue;
        }
        counts.plan_drawn += 1;
        scene.items.push(SceneItem {
            mesh: base + item.mesh,
            world,
            thing,
            uid: item.uid,
            shapes: 0,
            decoration: false,
            alternate: false,
        });
    }
}

/// True for scene meshes that belong to the backdrop (static mesh batches; plan meshes
/// are recognised by the caller through item ranges if needed).
pub fn is_backdrop_mesh(source: &MeshSource) -> bool {
    matches!(source, MeshSource::Backdrop { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_level::parts::{LevelSettings, PartData, Pos, RenderMesh, World};
    use lbp_level::serial::Descriptor;
    use lbp_mesh::static_mesh::StaticPrimitive;

    fn at(x: f32) -> [f32; 16] {
        let mut m = IDENTITY;
        m[12] = x;
        m
    }

    fn prim(material: Option<u32>, index_start: u32, count: u32, kind: u8) -> StaticPrimitive {
        StaticPrimitive {
            min: [0.; 4],
            max: [0.; 4],
            material: material.map(|g| StaticDescriptor {
                flags: 0,
                guid: Some(g),
                sha1: None,
            }),
            vertex_start: 0,
            index_start,
            index_count: count,
            kind,
        }
    }

    fn mesh() -> StaticMesh {
        StaticMesh {
            revision: 0x3ac,
            lightmaps: [None, None, None],
            primitives: vec![
                prim(Some(7), 0, 3, 5),
                prim(Some(9), 3, 4, 6),
                prim(Some(7), 7, 3, 5),
                prim(Some(7), 10, 3, 5), // degenerate
                prim(None, 0, 3, 4),     // unsupported type
            ],
            node_count: 1,
            positions: (0..6).map(|i| [i as f32, 0., 0.]).collect(),
            uv0: vec![[0.; 2]; 6],
            uv1: vec![[1.; 2]; 6],
            indices: vec![0, 1, 2, 2, 3, 4, 5, 3, 4, 5, 1, 1, 2],
            expanded_bytes: 0,
        }
    }

    #[test]
    fn static_mesh_batches_per_material_with_counts() {
        let mut scene = LevelScene::<BackdropBatch>::default();
        let mut counts = BackdropCounts {
            source: Some("test".into()),
            ..Default::default()
        };
        add_backdrop_mesh(
            &mut scene,
            &mut counts,
            Some(0x190ca),
            &mesh(),
            at(10.),
            3,
            |b| {
                let ok = if b.material.as_ref().and_then(|d| d.guid) == Some(9) {
                    Err("Unsupported: route".to_string())
                } else {
                    Ok(())
                };
                (b, ok)
            },
            |b| {
                let mut lo = [f32::INFINITY; 3];
                let mut hi = [f32::NEG_INFINITY; 3];
                for p in &b.positions {
                    for i in 0..3 {
                        lo[i] = lo[i].min(p[i]);
                        hi[i] = hi[i].max(p[i]);
                    }
                }
                Some([lo, hi])
            },
        );
        assert_eq!(counts.mesh_primitives, 5);
        assert_eq!(counts.mesh_primitives_drawn, 3);
        assert_eq!(counts.mesh_batches, 2);
        assert_eq!(
            counts.mesh_primitives_skipped["no nondegenerate triangles"],
            1
        );
        assert_eq!(
            counts.mesh_primitives_skipped["Unsupported(\"static mesh primitive type\")"],
            1
        );
        assert_eq!(counts.mesh_primitives_fallback["Unsupported: route"], 1);
        // Material 7: primitives 0 and 2 merged; vertices compacted (0,1,2 then 3,4,5).
        let b = &scene.meshes[0].payload;
        assert_eq!(b.primitives, vec![0, 2]);
        assert_eq!(b.positions.len(), 6);
        assert_eq!(b.triangles, vec![[0, 1, 2], [3, 4, 5]]);
        assert_eq!(b.uv1[0], [1., 1.]);
        // Strip primitive (material 9): 2,3,4 then 3,5,4 (odd flip), 4 vertices.
        let s = &scene.meshes[1].payload;
        assert_eq!(s.triangles, vec![[0, 1, 2], [1, 3, 2]]);
        assert!(scene.meshes[1].neutral_groups);
        assert!(is_backdrop_mesh(&scene.meshes[0].source));
        assert_eq!(scene.items[0].world, at(10.));
        assert_eq!(scene.bounds().unwrap()[0][0], 10.);
        assert!(counts
            .lines()
            .iter()
            .any(|l| l.contains("3 of 5 primitives")));
    }

    #[test]
    fn refs_plan_placement_and_matrix_helpers() {
        let d = |g| Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(g),
            sha1: None,
        };
        let world_thing = ThingRecord {
            index: 0,
            data: PartData {
                world: Some(World {
                    backdrop: Some(1),
                    backdrop_plan: Some(d(0x195eb)),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let backdrop = ThingRecord {
            index: 1,
            data: PartData {
                pos: Some(Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world: at(5.),
                }),
                level_settings: Some(LevelSettings {
                    backdrop_mesh: Some(d(0x190ca)),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let refs = backdrop_refs(&[world_thing, backdrop]);
        assert_eq!(refs.thing, Some(1));
        assert_eq!(refs.mesh, Some(MeshSource::Guid(0x190ca)));
        assert_eq!(refs.plan, Some(0x195eb));
        assert_eq!(refs.world, Some(at(5.)));
        assert_eq!(backdrop_refs(&[]), BackdropRefs::default());

        // Plan: settings Thing at x=2, a prop at x=3 -> level x = 5 - 2 + 3 = 6.
        let settings = ThingRecord {
            index: 0,
            data: PartData {
                pos: Some(Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world: at(2.),
                }),
                level_settings: Some(LevelSettings::default()),
                ..Default::default()
            },
            ..Default::default()
        };
        let prop = ThingRecord {
            index: 1,
            uid: Some(42),
            data: PartData {
                pos: Some(Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world: at(3.),
                }),
                render_mesh: Some(RenderMesh {
                    mesh: Some(d(0x150c3)),
                    bone_things: Vec::new(),
                    anim: None,
                    anim_pos: 0.,
                    anim_speed: 0.,
                    anim_loop: false,
                    editor_color: 0,
                    cast_shadows: 0,
                    visibility_flags: 0,
                    poppet_render_scale: 1.,
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut scene = LevelScene::<u32>::default();
        let mut counts = BackdropCounts::default();
        add_backdrop_plan(
            &mut scene,
            &mut counts,
            &[settings, prop],
            at(5.),
            9,
            |s| match s {
                MeshSource::Guid(g) => Ok(*g),
                _ => Err("x".into()),
            },
            |_| Some([[0.; 3], [1.; 3]]),
            |_| false,
            |_| Vec::new(),
            crate::level_scene::PosPlacement::PosOnly,
        );
        assert_eq!(
            (
                counts.plan_things,
                counts.plan_render_mesh,
                counts.plan_drawn
            ),
            (2, 1, 1)
        );
        assert_eq!(counts.plan_meshes_loaded, 1);
        assert_eq!(scene.items[0].world[12], 6.);
        assert_eq!(scene.items[0].thing, 9);
        assert_eq!(scene.items[0].uid, Some(42));

        let mut m = at(4.);
        m[0] = 2.;
        m[5] = -1.;
        let inv = affine_inverse(&m).unwrap();
        let id = mul(&m, &inv);
        for (a, b) in id.iter().zip(IDENTITY.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
        let mut flat = IDENTITY;
        flat[10] = 0.;
        assert!(affine_inverse(&flat).is_err());
    }
}
