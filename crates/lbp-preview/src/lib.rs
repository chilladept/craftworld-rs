//! Shared bounded CPU preview preparation. No GUI, GPU, decryption or executable tools.
//! Supported format/predicate evidence remains docs/textured-mesh.md.
mod add;
pub use add::DiffuseAdd;
/// D309: the glow alpha's form, read from the material's program (`Surface::glow_alpha`).
pub use lbp_resource::material::GlowAlphaForm;
mod affine;
mod alpha_mix;
pub use alpha_mix::{AlphaDiffuseMix, TextureAlphaMix};
mod animation_base;
pub use animation_base::AnimationBaseSnapshot;
mod blend;
mod blend_mask;
pub use blend_mask::{
    blend_factor, blend_mask_value, blend_thresholds, BlendDiffuseMask, BlendPort,
    TextureBlendMask, BLEND_THRESHOLD_BITS,
};
mod border_domain;
mod fragment;
pub use fragment::{
    evaluate as evaluate_fragment, FragmentGraph, FragmentStep, FragmentTexture, FRAGMENT_ADD,
    FRAGMENT_BLEND, FRAGMENT_MIX, FRAGMENT_MULTIPLY, FRAGMENT_SAMPLE, FRAGMENT_VALUE,
    FRAGMENT_WEIGHT, MAX_FRAGMENT_STEPS, MAX_FRAGMENT_TEXTURES,
};
mod grid_graph;
mod offgrid;
pub use grid_graph::{
    grid_graph_program, GraphInput, GraphNode, GraphOp, GraphProgram, GridGraph, MixFactor,
};
pub use offgrid::OffGrid;
pub mod census;
pub mod level_backdrop;
pub mod level_list;
pub mod level_scene;
pub mod popit;
pub mod runtime_texture;
pub mod story;
pub mod surface;
pub mod text;
pub use blend::DiffuseBlend;
pub use border_domain::{BorderDomain, INTERIOR_MARGIN};
mod matrix;
mod product;
mod product_tree;
pub use matrix::{SamplingTransform, UvMatrix};
pub use product_tree::ProductTree;
mod instance;
mod instance_fold;
mod instance_graph;
pub use instance_graph::{instance_graph_refusal, project_instance_graph, select_evaluated};
mod instance_sampler;
pub use instance_sampler::SamplerPortFold;
mod instance_textured;
pub use instance::{
    project_instance_color, AppliedDefaults, InstanceProjection, PreviewOptions, StandaloneDefaults,
};
pub use instance_textured::select_instance_diffuse;
mod linked;
pub mod skinned;
mod small_family;
pub use small_family::{select_small_solid, SamplerScale};
mod solid;
mod tint;
mod uv_channel;
pub use uv_channel::sampler_uv_channel;
mod wrapping;
pub use affine::UvTransform;
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
pub use linked::{
    load, resolve, select_diffuse, select_diffuse_with_animation_base, Asset, DiffuseSelection,
    MaterialAsset, RelativeSampling, SolidAsset,
};
pub use solid::{select_solid_diffuse, SolidSelection};
pub use tint::DiffuseTint;
pub use wrapping::{TextureAddress, TextureWrapping};
pub struct LoadedMesh {
    pub geometry: lbp_mesh::Geometry,
    pub linked: Option<Result<MaterialAsset, String>>,
}
pub fn load_mesh(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    textured: bool,
    control: &LoadControl,
) -> Result<LoadedMesh, String> {
    let (geometry, data) = read_mesh(source, key, control)?;
    let linked = textured.then(|| load(source, catalog, &geometry, &data, control));
    control.check()?;
    source.check_stable()?;
    Ok(LoadedMesh { geometry, linked })
}
pub(crate) fn read_mesh(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<(lbp_mesh::Geometry, Vec<u8>), String> {
    control.check()?;
    let mut data = source
        .read_hex_checked(key, 0, key.size.min(32) as usize, control)?
        .bytes;
    lbp_mesh::preflight(&data).map_err(|e| e.to_string())?;
    if key.size > lbp_mesh::MAX_INPUT as u64 {
        return Err("Mesh budget exceeded".into());
    }
    data.reserve(key.size as usize - data.len());
    while data.len() < key.size as usize {
        control.check()?;
        let count = (key.size as usize - data.len()).min(4096);
        let page = source
            .read_hex_checked(key, data.len() as u64, count, control)?
            .bytes;
        if page.len() != count {
            return Err("Short mesh page".into());
        }
        data.extend(page);
    }
    let geometry =
        lbp_mesh::decode_geometry(&data, || control.check().is_err()).map_err(|e| e.to_string())?;
    if geometry.triangles.len() > MAX_TRIANGLES {
        return Err(format!("Preview budget: more than {MAX_TRIANGLES} triangles; inspect metadata with the diagnostic tool"));
    }
    Ok((geometry, data))
}

/// Preview geometry budgets (P2: raised from 50,000 triangles / 100,000 vertices so the
/// Pod globe, cloth_earth 000027bd, draws; the PC reported it over the old budget).
pub const MAX_TRIANGLES: usize = 500_000;
/// Vertex (and per-vertex coordinate) budget for preview derivations.
pub const MAX_VERTICES: usize = 1_000_000;

/// Per-primitive outcomes share cached assets; no GPU handles or serialized asset data.
pub struct GroupedMesh {
    pub geometry: lbp_mesh::Geometry,
    pub bindings: Vec<Result<std::sync::Arc<MaterialAsset>, String>>,
    /// P2v: the mesh's decoded per-vertex normals (one per position, mesh space), when
    /// the skin streams decode. `None` after per-bone skinning (positions moved, normals
    /// not rotated) or when the stream is unusable; the viewer then generates normals.
    pub normals: Option<Vec<[f32; 3]>>,
}
/// D66: at most 64 distinct material loads per mesh, and the retained display bytes
/// of successful loads (decoded + derived pixels and coordinate arrays) may total at
/// most 640 MiB, the same aggregate the earlier eight-material count implied
/// (512 MiB raw + 128 MiB derived). A load that would exceed it fails explicitly
/// for its groups and is not retained. Each load keeps its own per-material read,
/// decompression and pixel limits, so worst-case stored reads and expansion scale
/// with the 64-load cap (docs/material-budget-preview.md). Derived graph metadata
/// clones are bounded by 1024 boxes / 4096 wires per material (no key arrays
/// copied). The parser's 4096 primitive cap is retained; failures stay explicit.
pub fn load_mesh_groups(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<GroupedMesh, String> {
    load_mesh_groups_with_animation_base(source, catalog, key, control, false)
}
/// Opt-in diagnostic snapshot of stored local sampler animation base values.
pub fn load_mesh_groups_with_animation_base(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
    animation_base: bool,
) -> Result<GroupedMesh, String> {
    load_mesh_groups_with_options(
        source,
        catalog,
        key,
        control,
        PreviewOptions {
            animation_base,
            ..Default::default()
        },
    )
}
/// Explicit diagnostic inputs; defaults retain static source-only behavior.
pub fn load_mesh_groups_with_options(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
    options: PreviewOptions,
) -> Result<GroupedMesh, String> {
    load_mesh_groups_with_overrides(source, catalog, key, control, options, &[])
}

/// D331: one PMaterialOverride region swap: mesh primitives whose `region` equals
/// `region` take this gmat descriptor (GUID and/or lowercase SHA-1 hex) instead of their
/// own. The override's uvScale is not applied (INFERRED/left out: no mesh-level UV-scale
/// input exists in the material loader).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegionMaterial {
    pub region: u32,
    pub guid: Option<u32>,
    pub hash: Option<String>,
}

/// D331: replaces the material descriptor of every primitive whose region is listed
/// (first listed entry wins); returns the number of primitives changed.
pub fn apply_region_materials(
    primitives: &mut [lbp_mesh::Primitive],
    overrides: &[RegionMaterial],
) -> usize {
    let mut changed = 0;
    for p in primitives {
        if let Some(o) = overrides.iter().find(|o| o.region == p.region) {
            p.material_flags = 0;
            p.material_kind = u8::from(o.guid.is_some()) * 2 + u8::from(o.hash.is_some());
            p.material_guid = o.guid;
            p.material_hash = o.hash.clone();
            changed += 1;
        }
    }
    changed
}

/// D331: [`load_mesh_groups_with_options`] with the primitives of the listed regions
/// drawn with the override's gmat (see [`RegionMaterial`]).
pub fn load_mesh_groups_with_overrides(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
    options: PreviewOptions,
    overrides: &[RegionMaterial],
) -> Result<GroupedMesh, String> {
    if let Some(color) = options.instance_color {
        color.validate()?;
    }
    let (mut geometry, data) = read_mesh(source, key, control)?;
    apply_region_materials(&mut geometry.metadata.primitives, overrides);
    let mut cache: std::collections::HashMap<
        LocationKey,
        Result<std::sync::Arc<MaterialAsset>, String>,
    > = std::collections::HashMap::new();
    let mut retained = 0usize;
    let mut bindings = Vec::with_capacity(geometry.metadata.primitives.len());
    for primitive in &geometry.metadata.primitives {
        control.check()?;
        let result = if is_materialless(primitive) {
            Err(MATERIALLESS_REASON.into())
        } else if primitive.material_flags != 0
            || primitive.alternatives_kind != 0
            || primitive.alternatives_flags != 0
        {
            Err("Unsupported: material flags or alternative materials".into())
        } else {
            resolve(
                catalog,
                primitive.material_guid,
                primitive.material_hash.as_deref(),
            )
            .and_then(|material_key| {
                cached_binding(
                    &mut cache,
                    &mut retained,
                    material_key,
                    MaterialAsset::retained_bytes,
                    || {
                        linked::load_primitive(
                            source, catalog, &geometry, &data, primitive, control, options,
                        )
                    },
                )
            })
        };
        bindings.push(result);
    }
    control.check()?;
    source.check_stable()?;
    let normals = lbp_mesh::decode_skin(&data, || control.check().is_err())
        .ok()
        .map(|skin| skin.normals)
        .filter(|n| n.len() == geometry.positions.len());
    Ok(GroupedMesh {
        geometry,
        bindings,
        normals,
    })
}

/// D188: neutral reason for a material-less primitive (see [`is_materialless`]).
pub const MATERIALLESS_REASON: &str =
    "Material-less: no material descriptor (game skips its colour draw, D192; not drawn, no diffuse invented)";

/// D188: a primitive whose material and alternatives descriptors are both entirely
/// absent (no GUID, no hash, kind 0, flags 0). Descriptor shape
/// `CONFIRMED_FROM_LOCAL_DATA` (01.33: 59 meshes, one such primitive each); what the
/// game draws for it is `UNKNOWN`; Toolkit `MeshExporter` emits such a primitive with
/// the glTF default (white) material (`CONFIRMED_FROM_COMMUNITY_CODE`, export only).
/// No diffuse binding is invented; censuses tally these groups separately
/// (docs/d188-absent-material-primitives.md). Any other unresolved descriptor stays
/// an ordinary blocking group.
pub fn is_materialless(p: &lbp_mesh::Primitive) -> bool {
    p.material_flags == 0
        && p.material_kind == 0
        && p.material_guid.is_none()
        && p.material_hash.is_none()
        && p.alternatives_flags == 0
        && p.alternatives_kind == 0
        && p.alternatives_guid.is_none()
        && p.alternatives_hash.is_none()
}

impl GroupedMesh {
    /// Per primitive: true when the group is material-less (D188).
    pub fn materialless(&self, index: usize) -> bool {
        self.geometry
            .metadata
            .primitives
            .get(index)
            .is_some_and(is_materialless)
    }
}

const MAX_MATERIAL_LOADS: usize = 64;
const MAX_RETAINED_BYTES: usize = 640 * 1024 * 1024;

fn cached_binding<T>(
    cache: &mut std::collections::HashMap<LocationKey, Result<std::sync::Arc<T>, String>>,
    retained: &mut usize,
    key: LocationKey,
    size: impl Fn(&T) -> usize,
    load: impl FnOnce() -> Result<T, String>,
) -> Result<std::sync::Arc<T>, String> {
    if let Some(result) = cache.get(&key) {
        return result.clone();
    }
    if cache.len() >= MAX_MATERIAL_LOADS {
        return Err("Unsupported: 64 distinct material preview budget".into());
    }
    let result = load().and_then(|value| {
        let total = retained
            .checked_add(size(&value))
            .filter(|&t| t <= MAX_RETAINED_BYTES)
            .ok_or("Unsupported: mesh preview memory budget (640 MiB retained)")?;
        *retained = total;
        Ok(std::sync::Arc::new(value))
    });
    cache.insert(key, result.clone());
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_bindings_share_success_failure_and_budget_without_new_reads() {
        let mut cache = std::collections::HashMap::new();
        let mut retained = 0;
        let size = |_: &usize| 1;
        let key = |n| LocationKey {
            path: format!("authored-{n}"),
            offset: 0,
            size: 1,
        };
        let first = cached_binding(&mut cache, &mut retained, key(0), size, || Ok(42)).unwrap();
        let repeat = cached_binding(&mut cache, &mut retained, key(0), size, || {
            panic!("duplicate read")
        })
        .unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &repeat));
        assert_eq!(
            cached_binding(&mut cache, &mut retained, key(1), size, || Err(
                "Unsupported graph".into()
            ))
            .unwrap_err(),
            "Unsupported graph"
        );
        assert_eq!(
            cached_binding(&mut cache, &mut retained, key(1), size, || panic!(
                "duplicate failed read"
            ))
            .unwrap_err(),
            "Unsupported graph"
        );
        for n in 2..MAX_MATERIAL_LOADS {
            cached_binding(&mut cache, &mut retained, key(n), size, || Ok(n)).unwrap();
        }
        assert_eq!(retained, MAX_MATERIAL_LOADS - 1); // failures retain nothing
        assert!(cached_binding(
            &mut cache,
            &mut retained,
            key(MAX_MATERIAL_LOADS),
            size,
            || panic!("count budget must precede read")
        )
        .unwrap_err()
        .contains("budget"));
        assert!(
            cached_binding(&mut cache, &mut retained, key(0), size, || panic!(
                "cached after budget"
            ))
            .is_ok()
        );
        // Aggregate retained bytes: an over-budget load fails, is cached as failed,
        // and leaves the total unchanged; later small loads still fit.
        let mut cache = std::collections::HashMap::new();
        let mut retained = MAX_RETAINED_BYTES - 10;
        let big = |v: &usize| *v;
        assert!(
            cached_binding(&mut cache, &mut retained, key(0), big, || Ok(11))
                .unwrap_err()
                .contains("memory budget")
        );
        assert_eq!(retained, MAX_RETAINED_BYTES - 10);
        assert!(
            cached_binding(&mut cache, &mut retained, key(0), big, || panic!("cached")).is_err()
        );
        assert!(cached_binding(&mut cache, &mut retained, key(1), big, || Ok(10)).is_ok());
        assert_eq!(retained, MAX_RETAINED_BYTES);
    }
    #[test]
    fn materialless_needs_every_descriptor_field_absent() {
        let none = lbp_mesh::Primitive {
            material_flags: 0,
            material_kind: 0,
            material_guid: None,
            material_hash: None,
            alternatives_flags: 0,
            alternatives_kind: 0,
            alternatives_guid: None,
            alternatives_hash: None,
            min_vertex_raw: 0,
            max_vertex_raw: 2,
            first_index: 0,
            index_count: 3,
            region: 0,
        };
        assert!(is_materialless(&none));
        let variants = [
            lbp_mesh::Primitive {
                material_guid: Some(1),
                ..none.clone()
            },
            lbp_mesh::Primitive {
                material_hash: Some("00".repeat(20)),
                ..none.clone()
            },
            lbp_mesh::Primitive {
                material_kind: 2,
                ..none.clone()
            },
            lbp_mesh::Primitive {
                material_flags: 1,
                ..none.clone()
            },
            lbp_mesh::Primitive {
                alternatives_kind: 1,
                ..none.clone()
            },
            lbp_mesh::Primitive {
                alternatives_flags: 1,
                ..none.clone()
            },
            lbp_mesh::Primitive {
                alternatives_guid: Some(7),
                ..none.clone()
            },
        ];
        for p in variants {
            assert!(!is_materialless(&p), "{p:?}");
        }
    }
    #[test]
    fn standalone_defaults_fill_only_unset_inputs() {
        let (o, a) = StandaloneDefaults::ON.apply(PreviewOptions::default());
        assert!(o.animation_base && a.animation_base && a.instance_white);
        assert_eq!(o.instance_color.unwrap().values(), [1.; 4]);
        let explicit = PreviewOptions {
            animation_base: true,
            instance_color: Some(DiffuseTint {
                bits: [0.5f32; 4].map(f32::to_bits),
            }),
            surface: true,
        };
        let (o, a) = StandaloneDefaults::ON.apply(explicit);
        assert!(!a.any() && o.surface);
        assert_eq!(o.instance_color, explicit.instance_color);
        let (o, a) = StandaloneDefaults::OFF.apply(PreviewOptions::default());
        assert!(!a.any() && !o.animation_base && o.instance_color.is_none());
    }
}
