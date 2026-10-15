//! D113 (level-research L2/L5 subset): renderer-independent level scene.
//!
//! Reads one level with `lbp-level` and turns every Thing that has a RENDER_MESH into a
//! scene item: (shared mesh payload, world matrix). Meshes load once per mesh reference
//! through the same bounded single-mesh path (`load_mesh_groups_with_options`) and are
//! reused by every item that names them. Things that cannot be drawn are skipped and
//! counted by reason. Placement is the Thing's own POS world matrix (static bind pose):
//! bones, animation, stickers and material overrides are not applied.
//!
//! D152: DECORATIONS entries are drawn as extra items at owner POS x stored offset (see
//! `add_decorations`); STICKERS decals are counted but not drawn.
//!
//! D120: POS Things are placed at POS world x inverse bind pose of mesh bone 0 by default
//! (`PosPlacement::InverseBind`, Toolkit StaticMeshBuilder/SceneExporter rule);
//! `PosPlacement::PosOnly` keeps the D113 placement (POS alone) for comparison.
//! RENDER_MESH Things without POS that carry a JOINT (bolts, strings, pistons...)
//! are placed from the joint's two endpoint Things (`joint_world`, preview
//! approximation, see docs/d120-bone-props.md). Every other Thing without POS stays
//! skipped with its reason.
//!
//! SHAPE Things with a visible GENERATED_MESH are extruded by D114
//! (`lbp_level::shape_mesh`, world-space geometry, preview approximation) and added by
//! `add_shapes`: shapes sharing one gfx material are merged into world-space batches
//! (`MeshSource::Shapes`, identity item matrix) so each material loads once per batch
//! through the same diffuse route as model meshes, with the generated UV0 as input.
//! The renderer consumes only `meshes` + `items`.
//!
//! D122 (opt-in, `LevelSceneOptions::shape_bevels`, default off): shapes are extruded with
//! their RBevel profile (`lbp_level::shape_mesh::extrude_thing_bevelled`; bevel geometry
//! inferred, unverified) and each shape is split by bevel-slot material before batching.
//! With the option off the scene is built exactly as in D114.
//!
//! No GUI or GPU types. The level reader recurses natively: call `load_level_scene` on
//! a thread with at least `LEVEL_STACK_BYTES` of stack.
use crate::{load_mesh_groups_with_options, resolve, GroupedMesh, MaterialAsset, PreviewOptions};
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_level::bevel::Bevel;
pub use lbp_level::parts::LevelSettings;
use lbp_level::serial::Descriptor;
use lbp_level::shape_mesh::{self, BevelOptions, MaterialPart};
use lbp_level::thing::ThingRecord;
use std::collections::{BTreeMap, HashMap};

/// D114 shape label, re-exported so renderers need not depend on `lbp-level`.
pub const SHAPE_PREVIEW_LABEL: &str = shape_mesh::PREVIEW_LABEL;
/// D122 bevelled-shape label (shown when `shape_bevels` is on).
pub const SHAPE_BEVEL_LABEL: &str = shape_mesh::BEVEL_LABEL;
/// Distinct bevel resources loaded per level.
pub const MAX_LEVEL_BEVELS: usize = 1024;

/// Level-scene options beyond the material preview options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelSceneOptions {
    /// D122: extrude shapes with their bevel profile (inferred, unverified). Default off.
    pub shape_bevels: bool,
    /// D120: POS placement rule (default inverse bind of bone 0).
    pub placement: PosPlacement,
    /// P2e `--runtime-texture MESH:SLOT:TEXTURE` rules (docs/p2e-runtime-texture.md).
    /// Empty (default): materials draw as loaded.
    pub runtime_textures: Vec<crate::runtime_texture::RuntimeTexture>,
    /// P2m `--per-bone-placement`: POS models with boneThings place each vertex by its
    /// bone's Thing (docs/p2m-per-bone-placement.md). Default off (bone 0 rule, D120).
    pub per_bone: bool,
    /// D191 (docs/d191-level-thing-defaults.md): `instance_white` gives each RENDER_MESH
    /// Thing (and decoration) its own `PRenderMesh.editorColor` as the instance colour
    /// (link to THING_COLOR inferred) and the backdrop the D188 white; `animation_base`
    /// draws every level material at its animation base values. Explicit
    /// [`PreviewOptions`] win. Default on; `StandaloneDefaults::OFF` restores the old
    /// source-only scene.
    pub defaults: crate::StandaloneDefaults,
    /// D276: pairs of mesh GUIDs a script swaps on a Thing's RENDER_MESH. A Thing drawn
    /// with one mesh of a pair also gets an item for the other at the same POS
    /// ([`add_alternate_meshes`]; `SceneItem::alternate`), for a renderer to keep hidden
    /// until the swap. Empty (default): no such items.
    pub mesh_alternates: Vec<[u32; 2]>,
}

/// D276: the other mesh GUID of the first pair in `pairs` that holds `guid`.
pub fn alternate_mesh(pairs: &[[u32; 2]], guid: u32) -> Option<u32> {
    pairs
        .iter()
        .find(|pair| pair.contains(&guid))
        .map(|&[a, b]| if guid == a { b } else { a })
        .filter(|&other| other != guid)
}

/// D191: `PRenderMesh.editorColor` (packed ARGB, -1 = opaque white) as an RGBA tint.
pub const EDITOR_COLOR_WHITE: u32 = 0xffff_ffff;
/// Default opaque white ARGB (`PRenderMesh.editorColor` and `PShape.color` default -1).
pub const WHITE_ARGB: u32 = 0xffff_ffff;

/// D194: `PShape.color` as packed ARGB. Floats (revisions up to 0x389) are clamped to
/// 0..1 and rounded to bytes; non-finite components count as 1.
pub fn shape_color_argb(color: &lbp_level::parts::ShapeColor) -> u32 {
    match color {
        lbp_level::parts::ShapeColor::Packed(argb) => *argb,
        lbp_level::parts::ShapeColor::Vector([r, g, b, a]) => {
            let byte = |v: f32| {
                let v = if v.is_finite() { v.clamp(0., 1.) } else { 1. };
                (v * 255.).round() as u32
            };
            byte(*a) << 24 | byte(*r) << 16 | byte(*g) << 8 | byte(*b)
        }
    }
}

pub fn editor_color_tint(argb: u32) -> crate::DiffuseTint {
    let channel = |shift: u32| (((argb >> shift) & 0xff) as f32 / 255.).to_bits();
    crate::DiffuseTint {
        bits: [channel(16), channel(8), channel(0), channel(24)],
    }
}

/// D307 FOUND (01.33, the checkpoint's per-frame update 0x5aa9c): a checkpoint sets its
/// render colour's alpha to `1 - 0.5 x` (stored as a byte, x 255), where `x` approaches the
/// share of spawns left (`spawnsLeft / maxSpawnsLeft`, at 0.1 a frame) when the checkpoint
/// holds spawns and some are left, and 0 otherwise. The ring's lifebar material (gmat
/// `neon_lifebar`) samples its texture at UV1 minus that alpha (the graph's thing-colour
/// alpha on sampler port 2), so a full checkpoint (alpha 0.5) shows the white half of the
/// texture and an empty one (alpha 1) the black half. D308 FOUND (same update): an infinite
/// checkpoint (`instanceInfiniteSpawns`, INFERRED from its use as "can activate without
/// spawns left") targets `x` = 1, so its ring is white. INFERRED: the level's stored counts
/// are the start of play, the update has converged, and the active checkpoint's out-of-lives
/// pulse (red, `x` = 1) is not drawn.
pub fn checkpoint_light_alpha(c: &lbp_level::parts::Checkpoint) -> f32 {
    let x = if c.infinite_spawns {
        1.
    } else if c.max_spawns_left > 0 && c.spawns_left != 0 {
        (c.spawns_left as f32 / c.max_spawns_left as f32).clamp(0., 1.)
    } else {
        0.
    };
    1. - 0.5 * x
}
/// D307: the checkpoint each render-mesh Thing lights, by the Thing's index. The update
/// writes the colour of the checkpoint Thing's parent (its `+0x50`, whose render mesh it
/// checks first; INFERRED the parent pointer): in the level files the CHECKPOINT part sits
/// on a child of the mesh Thing (Bravery Test: Thing 4805 under the spawn door's 4801).
pub fn checkpoint_lights(things: &[ThingRecord]) -> HashMap<usize, &lbp_level::parts::Checkpoint> {
    things
        .iter()
        .filter_map(|t| Some((t.parent?, t.data.checkpoint.as_ref()?)))
        .collect()
}
/// D307: the editorColor a Thing's render mesh is drawn with: a checkpoint's alpha byte is
/// [`checkpoint_light_alpha`]; every other Thing keeps its stored colour. D308 FOUND (0x5aa9c
/// packs the whole word, base colour 0xfc2d0): in normal play the RGB is white, whatever the
/// stored colour (team and player-colour modes are not modelled); the byte is truncated.
pub fn checkpoint_editor_color(argb: u32, c: Option<&lbp_level::parts::Checkpoint>) -> u32 {
    match c {
        Some(c) => {
            let byte = (checkpoint_light_alpha(c) * 255.) as u32 & 0xff;
            0x00ff_ffff | byte << 24
        }
        None => argb,
    }
}
/// Parsed bevels by descriptor identity (GUID, SHA-1).
pub type BevelTable = HashMap<(Option<u32>, Option<[u8; 20]>), Bevel>;
/// Worker stack for `load_level_scene` (the D107 examples use the same size).
pub const LEVEL_STACK_BYTES: usize = 512 * 1024 * 1024;
/// Distinct mesh references loaded per level.
pub const MAX_LEVEL_MESHES: usize = 1024;
/// Drawn items per level.
pub const MAX_LEVEL_ITEMS: usize = 100_000;
/// Aggregate retained material bytes over all loaded meshes of one level. Each mesh keeps
/// its own D66 limits (64 material loads, 640 MiB); this caps the sum.
pub const MAX_LEVEL_RETAINED_BYTES: usize = 1536 * 1024 * 1024;
/// Shape Things extruded per level and output vertices over all shapes.
pub const MAX_LEVEL_SHAPES: usize = 50_000;
pub const MAX_LEVEL_SHAPE_VERTICES: usize = 8_000_000;
/// Per shape batch: the display adapter's per-mesh limits (positions / triangles).
pub const MAX_BATCH_VERTICES: usize = 100_000;
pub const MAX_BATCH_TRIANGLES: usize = 50_000;
const PAGE: usize = 4096;

/// Where a scene mesh comes from. Shapes (D114) will add a variant here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeshSource {
    /// Model mesh named by GUID in a RENDER_MESH descriptor.
    Guid(u32),
    /// Model mesh named only by SHA-1 (lowercase hex).
    Sha1(String),
    /// Merged world-space shape extrusions sharing one gfx material GUID (None = no
    /// GUID) and `PShape.color` (D194, ARGB); `batch` numbers the chunks of that pair.
    Shapes {
        material: Option<u32>,
        color: u32,
        batch: usize,
    },
    /// D160: STICKERS decal overlays merged per decal texture (GUID hex or SHA-1 hex).
    Decals { texture: String, batch: usize },
    /// D119: static backdrop mesh primitives (mesh GUID) merged per gfx material GUID.
    Backdrop {
        mesh: Option<u32>,
        material: Option<u32>,
        batch: usize,
    },
    /// D147: per-item skinned copy of a span-JOINT mesh (Thing index in reader order).
    JointSkin { mesh: Box<MeshSource>, thing: usize },
    /// P2m: per-item copy of a POS model mesh placed per bone by its boneThings.
    BoneSkin { mesh: Box<MeshSource>, thing: usize },
}

impl MeshSource {
    /// Mesh GUID behind this source, looking through per-item skinned copies
    /// (JointSkin/BoneSkin keep the original mesh identity).
    pub fn base_guid(&self) -> Option<u32> {
        match self {
            Self::Guid(g) => Some(*g),
            Self::JointSkin { mesh, .. } | Self::BoneSkin { mesh, .. } => mesh.base_guid(),
            _ => None,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Guid(g) => format!("{g:08x}"),
            Self::Sha1(h) => format!("sha1 {h}"),
            Self::Shapes {
                material,
                color,
                batch,
            } => format!(
                "shapes gfx {}{} batch {batch}",
                material.map_or("none".into(), |g| format!("{g:08x}")),
                if *color == WHITE_ARGB {
                    String::new()
                } else {
                    format!(" colour {color:08x}")
                }
            ),
            Self::Backdrop {
                mesh,
                material,
                batch,
            } => format!(
                "backdrop {} gfx {} batch {batch}",
                mesh.map_or("none".into(), |g| format!("{g:08x}")),
                material.map_or("none".into(), |g| format!("{g:08x}"))
            ),
            Self::Decals { texture, batch } => format!("sticker decals {texture} batch {batch}"),
            Self::JointSkin { mesh, thing } => {
                format!("{} skinned for joint Thing {thing}", mesh.label())
            }
            Self::BoneSkin { mesh, thing } => {
                format!("{} placed per bone for Thing {thing}", mesh.label())
            }
        }
    }
}

pub struct SceneMesh<T> {
    pub source: MeshSource,
    pub payload: T,
    /// Local axis-aligned bounds of the payload geometry (None when empty/nonfinite).
    pub bounds: Option<[[f32; 3]; 2]>,
    /// True when at least one non-empty primitive group has no supported material
    /// (the renderer shows it in the neutral diagnostic colour, as the single-mesh view).
    pub neutral_groups: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    /// Index into `LevelScene::meshes`.
    pub mesh: usize,
    /// Column-major world matrix (translation in elements 12..14), as stored in POS.
    pub world: [f32; 16],
    /// Thing index in reader (tag) order and its UID.
    pub thing: usize,
    pub uid: Option<i32>,
    /// Shape Things merged into this item (0 for a model item).
    pub shapes: usize,
    /// The item draws an entry of `thing`'s DECORATIONS part, not its RENDER_MESH part.
    pub decoration: bool,
    /// D276: the item draws a mesh the level file does not name on `thing`: the other
    /// mesh of a `LevelSceneOptions::mesh_alternates` pair. Not counted as drawn, left
    /// out of the scene bounds, and for a renderer to hide until a script swaps to it.
    pub alternate: bool,
}

/// Merged world-space shape geometry for one gfx material (D114 extrusions).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShapeBatch {
    pub material: Option<u32>,
    /// D194: the merged shapes' `PShape.color` (packed ARGB); batches split per colour.
    pub color: u32,
    pub positions: Vec<[f32; 3]>,
    pub uv0: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Thing indices of the merged shapes.
    pub things: Vec<usize>,
}

/// Counting units: Things (tag-table entries), items (= drawn Things), distinct mesh
/// references. Skip reasons count Things.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneCounts {
    pub things_total: usize,
    pub with_render_mesh: usize,
    pub drawn: usize,
    pub skipped: BTreeMap<String, usize>,
    /// Drawn Things whose POS names another Thing as bone owner (placed by own POS).
    pub drawn_bone_attached: usize,
    /// D120: POS placement rule used for this scene.
    pub pos_placement: PosPlacement,
    /// D120: drawn POS Things whose matrix differs from POS alone (D113), and those
    /// unchanged (inverse bind = identity) or kept at POS because the mesh has no bones.
    pub pos_bind_changed: usize,
    pub pos_bind_unchanged: usize,
    pub pos_bind_no_bones: usize,
    /// D120: per mesh source label: (changed Things, max translation shift in level
    /// units, max |log2 scale ratio|, max rotation in degrees) versus POS alone.
    pub pos_bind_by_mesh: BTreeMap<String, (usize, f64, f64, f64)>,
    /// D120: drawn JOINT Things without POS, placed at contact A (angular joint types).
    pub drawn_joint_pivot: usize,
    /// D120: drawn JOINT Things without POS, fitted between contacts A and B (linear).
    pub drawn_joint_span: usize,
    /// D147: span-JOINT items drawn skinned (rigid end groups, stretched bridge), the
    /// ones kept at the D120 stretch by reason, and skinned vertices without a usable
    /// weight (placed with contact A).
    pub joint_span_skinned: usize,
    pub joint_span_skin_failed: BTreeMap<String, usize>,
    pub joint_skin_unweighted_vertices: usize,
    /// P2m (per_bone on): POS model items with two or more boneThings drawn per bone,
    /// those whose bones all match the bone-0 rule (left as they were), the ones kept at
    /// the bone-0 rule by reason, and vertices without a usable weight (bone-0 rule).
    pub per_bone: bool,
    pub per_bone_skinned: usize,
    pub per_bone_unchanged: usize,
    pub per_bone_failed: BTreeMap<String, usize>,
    pub per_bone_unweighted_vertices: usize,
    /// Drawn Things whose mesh has neutral (unsupported material) groups.
    pub drawn_with_neutral_groups: usize,
    /// D191: drawn Things and decorations whose mesh uses the instance colour and whose
    /// editorColor is not white, and the extra mesh loads (one per mesh and colour).
    pub drawn_editor_colored: usize,
    pub editor_color_variants: usize,
    pub meshes_loaded: usize,
    pub meshes_failed: usize,
    /// Shape Things (SHAPE part present) and their extrusion outcomes.
    pub shapes_total: usize,
    pub shapes_extruded: usize,
    pub shapes_placed: usize,
    /// Selection outcomes (no GENERATED_MESH, edit-only, no POS): not drawn by design.
    pub shapes_not_selected: BTreeMap<String, usize>,
    /// Geometry rejections from the extrusion (by stable reason key) and budgets.
    pub shapes_rejected: BTreeMap<String, usize>,
    /// Placed shapes whose gfx material did not select (flat neutral colour), by reason.
    pub shapes_material_fallback: BTreeMap<String, usize>,
    pub shape_batches: usize,
    /// D122 (shape_bevels on): bevel outcome per extruded shape (`BevelOutcome::key`, with
    /// the fallback reason) and distinct bevel resources loaded / failed.
    pub shape_bevel_outcomes: BTreeMap<String, usize>,
    pub bevels_loaded: usize,
    pub bevels_failed: usize,
    /// D152: DECORATIONS entries (counting unit: entries), those drawn as scene items at
    /// owner POS x offset (x inverse bind of the decoration mesh's bone 0 under the
    /// inverse-bind rule), and the rest by reason.
    pub decorations_total: usize,
    pub decorations_drawn: usize,
    /// Drawn decorations whose mesh has neutral (unsupported material) groups.
    pub decorations_drawn_neutral: usize,
    pub decorations_skipped: BTreeMap<String, usize>,
    /// D160: STICKERS decals (counting unit: decals, main plus costume arrays), those
    /// drawn as projected overlays, and the rest by reason (when the pass ran).
    pub sticker_decals_total: usize,
    pub sticker_decals_drawn: usize,
    pub sticker_decals_skipped: BTreeMap<String, usize>,
    /// Reader stop (None = whole level read).
    pub reader_stop: Option<String>,
    /// D119 backdrop (static mesh + plan Things).
    pub backdrop: crate::level_backdrop::BackdropCounts,
    /// P2e opt-in runtime texture rules: per rule, primitive groups of distinct loaded
    /// model meshes drawn with the rule's texture, and those left by reason.
    pub runtime_textures: crate::runtime_texture::RuntimeCounts,
    /// D276 (`LevelSceneOptions::mesh_alternates`): Things given an item for the other
    /// mesh of a pair, and those whose other mesh could not be loaded or placed.
    pub alternate_meshes: usize,
    pub alternate_meshes_failed: usize,
}

impl SceneCounts {
    pub fn skipped_total(&self) -> usize {
        self.skipped.values().sum()
    }
    /// Display / QA-log lines with explicit units.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![
            format!("Things total: {} (tag-table Things)", self.things_total),
            format!("Things with RENDER_MESH: {}", self.with_render_mesh),
            format!("Placed (scene items): {} Things", self.drawn),
            format!("Skipped: {} Things with RENDER_MESH", self.skipped_total()),
        ];
        for (reason, n) in &self.skipped {
            out.push(format!("  skipped {n}: {reason}"));
        }
        out.push(format!(
            "Distinct meshes: {} loaded, {} failed (mesh references)",
            self.meshes_loaded, self.meshes_failed
        ));
        out.push(format!(
            "Drawn with neutral (unsupported material) groups: {} Things",
            self.drawn_with_neutral_groups
        ));
        out.push(format!(
            "Drawn with a non-white editorColor instance colour: {} Things/decorations, {} mesh+colour variants (editorColor to THING_COLOR inferred, D191)",
            self.drawn_editor_colored, self.editor_color_variants
        ));
        out.push(format!(
            "Drawn bone-attached at own POS (approximation): {} Things",
            self.drawn_bone_attached
        ));
        out.push(match self.pos_placement {
            PosPlacement::InverseBind => {
                "POS placement: POS x inverse bind pose of mesh bone 0 (D120 default, Toolkit StaticMeshBuilder rule)".into()
            }
            PosPlacement::PosOnly => "POS placement: POS only (D113, --level-placement pos-only)".into(),
        });
        if self.pos_placement == PosPlacement::InverseBind {
            out.push(format!(
                "Changed vs D113 POS-only: {} Things ({} unchanged, {} meshes without bones kept at POS)",
                self.pos_bind_changed, self.pos_bind_unchanged, self.pos_bind_no_bones
            ));
            let mut most: Vec<_> = self.pos_bind_by_mesh.iter().collect();
            most.sort_by(|a, b| b.1 .1.total_cmp(&a.1 .1).then(a.0.cmp(b.0)));
            for (mesh, (n, shift, scale, rot)) in most.into_iter().take(5) {
                out.push(format!(
                    "  most moved: mesh {mesh} ({n} Things): shift {shift:.1} units, scale x{:.3}, rotation {rot:.1} deg",
                    scale.exp2()
                ));
            }
        }
        out.push(format!(
            "Placed from JOINT endpoints (D120 preview approximation): {} Things ({} pivot at contact A, {} span A to B; angle/renderScale not applied)",
            self.drawn_joint_pivot + self.drawn_joint_span,
            self.drawn_joint_pivot,
            self.drawn_joint_span
        ));
        let kept: usize = self.joint_span_skin_failed.values().sum();
        out.push(format!(
            "Span joints skinned (D147, inferred: rigid end groups, stretched bridge): {} of {} Things; {} kept stretched; {} vertices without usable weight",
            self.joint_span_skinned,
            self.drawn_joint_span,
            kept,
            self.joint_skin_unweighted_vertices
        ));
        for (reason, n) in &self.joint_span_skin_failed {
            out.push(format!("  kept stretched {n}: {reason}"));
        }
        if self.per_bone {
            let kept: usize = self.per_bone_failed.values().sum();
            out.push(format!(
                "Per-bone placement (P2m, inferred rigid skinning by boneThings): {} items placed per bone; {} unchanged (bones match bone 0); {} kept at bone 0; {} vertices without usable weight",
                self.per_bone_skinned, self.per_bone_unchanged, kept, self.per_bone_unweighted_vertices
            ));
            for (reason, n) in &self.per_bone_failed {
                out.push(format!("  kept at bone 0 {n}: {reason}"));
            }
        }
        out.push(format!(
            "Shapes: {} SHAPE Things; {} extruded; {} placed in {} material batches",
            self.shapes_total, self.shapes_extruded, self.shapes_placed, self.shape_batches
        ));
        for (reason, n) in &self.shapes_not_selected {
            out.push(format!("  shape not selected {n}: {reason}"));
        }
        for (reason, n) in &self.shapes_rejected {
            out.push(format!("  shape rejected {n}: {reason}"));
        }
        let fallback: usize = self.shapes_material_fallback.values().sum();
        out.push(format!(
            "Shapes with flat fallback colour (gfx material not selected): {fallback} Things"
        ));
        for (reason, n) in &self.shapes_material_fallback {
            out.push(format!("  fallback {n}: {reason}"));
        }
        if self.shapes_placed > 0 {
            if self.shape_bevel_outcomes.is_empty() {
                out.push(format!("Shapes are a {}", shape_mesh::PREVIEW_LABEL));
            } else {
                out.push(format!(
                    "Shapes without an applied bevel are a {}",
                    shape_mesh::PREVIEW_LABEL
                ));
            }
        }
        if !self.shape_bevel_outcomes.is_empty() || self.bevels_loaded + self.bevels_failed > 0 {
            out.push(format!(
                "Shape bevels (opt-in): {} bevel resources loaded, {} failed; {}",
                self.bevels_loaded,
                self.bevels_failed,
                shape_mesh::BEVEL_LABEL
            ));
            for (k, n) in &self.shape_bevel_outcomes {
                out.push(format!("  shape bevel {n}: {k}"));
            }
        }
        out.push(format!(
            "Decorations drawn: {} / {} (DECORATIONS entries, owner POS x offset; inferred); {} with neutral material groups",
            self.decorations_drawn, self.decorations_total, self.decorations_drawn_neutral
        ));
        for (reason, n) in &self.decorations_skipped {
            out.push(format!("  decoration skipped {n}: {reason}"));
        }
        out.push(format!(
            "Sticker decals drawn: {} / {} (STICKERS decals projected on the owner shape's front face; frame inferred)",
            self.sticker_decals_drawn, self.sticker_decals_total
        ));
        let listed: usize = self.sticker_decals_skipped.values().sum();
        for (reason, n) in &self.sticker_decals_skipped {
            out.push(format!("  sticker decal not drawn {n}: {reason}"));
        }
        let rest = self
            .sticker_decals_total
            .saturating_sub(self.sticker_decals_drawn + listed);
        if rest > 0 {
            out.push(format!(
                "  sticker decal not drawn {rest}: decal pass not run"
            ));
        }
        out.extend(self.backdrop.lines());
        out.extend(crate::runtime_texture::lines(&self.runtime_textures));
        if self.alternate_meshes + self.alternate_meshes_failed > 0 {
            out.push(format!(
                "Alternate meshes (a script swaps them in): {} Things with a second item, kept hidden; {} not loaded or placed",
                self.alternate_meshes, self.alternate_meshes_failed
            ));
        }
        out.push(match &self.reader_stop {
            None => "Level reader: complete".into(),
            Some(s) => format!("Level reader stopped: {s}"),
        });
        out
    }
}

pub struct LevelScene<T> {
    pub meshes: Vec<SceneMesh<T>>,
    pub items: Vec<SceneItem>,
    pub counts: SceneCounts,
    /// D121: stored PLevelSettings (world holder first), for opt-in preview lighting.
    pub settings: Option<LevelSettings>,
    /// P2l: the same holder's stored presets (for `--lighting-preset`).
    pub settings_presets: Vec<LevelSettings>,
    /// P3m: PWorld `lighting`, `colourCorrection`, `fog`, `fogTint`, `darkness` (the game
    /// blends the presets by `lighting` and dims by `darkness`; `LevelSettings::at_lighting`).
    pub world_lighting: Option<[f32; 5]>,
    /// P3b: presets of the PLevelSettings holder inside the backdrop plan. The Pod's live
    /// lighting is this holder's preset 0 (probe #105), not the level's own holder.
    pub backdrop_presets: Vec<LevelSettings>,
    /// P2c: Things with a CHECKPOINT part and a valid POS (Thing index, world matrix), in
    /// reader order. The player spawns at a checkpoint.
    pub checkpoints: Vec<(usize, [f32; 16])>,
}

impl<T> Default for LevelScene<T> {
    fn default() -> Self {
        Self {
            meshes: Vec::new(),
            items: Vec::new(),
            counts: SceneCounts::default(),
            settings: None,
            settings_presets: Vec::new(),
            world_lighting: None,
            backdrop_presets: Vec::new(),
            checkpoints: Vec::new(),
        }
    }
}

impl<T> LevelScene<T> {
    /// Adds a drawable item; extension point for later mesh sources (shapes).
    pub fn push_item(&mut self, item: SceneItem) -> Result<(), String> {
        if item.mesh >= self.meshes.len() {
            return Err("Invalid: scene item mesh index".into());
        }
        if self.items.len() >= MAX_LEVEL_ITEMS {
            return Err("Unsupported: level item budget".into());
        }
        world_matrix_check(&item.world)?;
        self.items.push(item);
        Ok(())
    }

    /// World-space bounds of all items (mesh local bounds through each world matrix).
    pub fn bounds(&self) -> Option<[[f32; 3]; 2]> {
        self.bounds_where(|_| true)
    }

    /// World-space bounds of the items `keep` accepts (e.g. those a renderer displays).
    pub fn bounds_where(&self, keep: impl Fn(&SceneItem) -> bool) -> Option<[[f32; 3]; 2]> {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for item in self.items.iter().filter(|i| !i.alternate && keep(i)) {
            let Some([a, b]) = self.meshes[item.mesh].bounds else {
                continue;
            };
            for corner in 0..8 {
                let p = [
                    if corner & 1 == 0 { a[0] } else { b[0] },
                    if corner & 2 == 0 { a[1] } else { b[1] },
                    if corner & 4 == 0 { a[2] } else { b[2] },
                ];
                let w = transform_point(&item.world, p);
                for i in 0..3 {
                    lo[i] = lo[i].min(w[i]);
                    hi[i] = hi[i].max(w[i]);
                }
            }
        }
        (lo.iter().chain(&hi).all(|v| v.is_finite()))
            .then(|| [lo.map(|v| v as f32), hi.map(|v| v as f32)])
    }
}

/// Column-major point transform in f64.
pub fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f64; 3] {
    let m = m.map(f64::from);
    let [x, y, z] = p.map(f64::from);
    std::array::from_fn(|r| m[r] * x + m[4 + r] * y + m[8 + r] * z + m[12 + r])
}

/// Finite, affine (last row 0,0,0,1) and invertible upper 3x3.
pub fn world_matrix_check(m: &[f32; 16]) -> Result<(), String> {
    if m.iter().any(|v| !v.is_finite()) {
        return Err("non-finite world matrix".into());
    }
    let tol = 1e-4;
    if m[3].abs() > tol || m[7].abs() > tol || m[11].abs() > tol || (m[15] - 1.).abs() > tol {
        return Err("projective world matrix (last row not 0,0,0,1)".into());
    }
    let a = m.map(f64::from);
    let det = a[0] * (a[5] * a[10] - a[9] * a[6]) - a[4] * (a[1] * a[10] - a[9] * a[2])
        + a[8] * (a[1] * a[6] - a[5] * a[2]);
    if det.is_nan() || det.abs() <= 1e-12 {
        return Err("non-invertible world matrix".into());
    }
    Ok(())
}

/// D120 placement of POS Things.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PosPlacement {
    /// POS world x inverse bind (skin) pose of mesh bone 0 (CONFIRMED_FROM_COMMUNITY_CODE:
    /// Toolkit d6a2ef80 `cwlib/io/imports/StaticMeshBuilder.java` matrices[0] = wpos x
    /// bones[0].invSkinPoseMatrix; `cwlib-gl/.../SceneExporter.java` registerThing uses
    /// POS x invSkinPoseMatrix for bone 0). Meshes without bones keep POS.
    #[default]
    InverseBind,
    /// D113: POS world alone.
    PosOnly,
}

impl PosPlacement {
    pub fn parse(v: &str) -> Result<Self, String> {
        match v {
            "inverse-bind" => Ok(Self::InverseBind),
            "pos-only" => Ok(Self::PosOnly),
            _ => Err("--level-placement takes inverse-bind or pos-only".into()),
        }
    }
}

/// Bone bind pose and its stored inverse (column-major, as read by lbp-mesh).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BindPose {
    pub skin: [f32; 16],
    pub inv: [f32; 16],
}

/// Column-major 4x4 product a x b, accumulated in f64.
pub fn mul44(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        (0..4)
            .map(|k| f64::from(a[4 * k + r]) * f64::from(b[4 * c + k]))
            .sum::<f64>() as f32
    })
}

/// Change of `new` versus `old`: translation shift, |log2| of the volume scale ratio
/// (cube root of the determinant ratio), and rotation angle between the column-normalized
/// linear parts (degrees).
fn placement_change(old: &[f32; 16], new: &[f32; 16]) -> (f64, f64, f64) {
    let lin = |m: &[f32; 16]| -> M3 {
        std::array::from_fn(|c| std::array::from_fn(|r| f64::from(m[4 * c + r])))
    };
    let det = |m: &M3| dot(m[0], cross(m[1], m[2]));
    let (a, b) = (lin(old), lin(new));
    let shift = dot(
        sub(
            [new[12], new[13], new[14]].map(f64::from),
            [old[12], old[13], old[14]].map(f64::from),
        ),
        sub(
            [new[12], new[13], new[14]].map(f64::from),
            [old[12], old[13], old[14]].map(f64::from),
        ),
    )
    .sqrt();
    let ratio = (det(&b) / det(&a)).abs().cbrt();
    let scale = if ratio.is_finite() && ratio > 0. {
        ratio.log2()
    } else {
        f64::NAN
    };
    let norm = |m: &M3| -> Option<M3> { Some([unit(m[0])?, unit(m[1])?, unit(m[2])?]) };
    let rot = match (norm(&a), norm(&b)) {
        (Some(na), Some(nb)) => {
            let rel = mul_mm(&nb, &transpose(&na));
            (((rel[0][0] + rel[1][1] + rel[2][2] - 1.) / 2.).clamp(-1., 1.))
                .acos()
                .to_degrees()
        }
        _ => f64::NAN,
    };
    (shift, scale, rot)
}

/// How a drawn RENDER_MESH Thing is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointPlacement {
    /// Angular joint (BOLT, SPRING_ANGULAR, MOTOR, QUANTIZED): bone 0 at contact A.
    Pivot,
    /// Linear joint (ELASTIC, SPRING, CHAIN, PISTON, STRING, ROD): fitted A to B.
    Span,
}

/// Joint type classes (PJoint.type, Toolkit EJointType comment; INFERRED for the
/// angular/linear split, which follows the comment's "start of angular" marker).
pub fn joint_placement_kind(joint_type: i32) -> Option<JointPlacement> {
    match joint_type {
        1..=6 => Some(JointPlacement::Span),
        7..=10 => Some(JointPlacement::Pivot),
        _ => None,
    }
}

type M3 = [[f64; 3]; 3];

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn scaled(a: [f64; 3], k: f64) -> [f64; 3] {
    a.map(|v| v * k)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot(a, a).sqrt();
    (n.is_finite() && n > 1e-6).then(|| scaled(a, 1. / n))
}
/// Column vectors -> matrix applying them (m[col][row]).
fn mul_mv(m: &M3, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|r| m[0][r] * v[0] + m[1][r] * v[1] + m[2][r] * v[2])
}
fn mul_mm(a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|c| mul_mv(a, b[c]))
}
fn transpose(m: &M3) -> M3 {
    std::array::from_fn(|c| std::array::from_fn(|r| m[r][c]))
}
/// Orthonormal basis (columns) whose first axis is `x` and whose second axis is world
/// +Z (the layer depth axis) made orthogonal to `x` (+Y when `x` is nearly along Z).
fn frame(x: [f64; 3]) -> Option<M3> {
    let reference = if x[2].abs() > 0.99 {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let y = unit(sub(reference, scaled(x, dot(reference, x))))?;
    Some([x, y, cross(x, y)])
}

/// D120 joint placement (INFERRED_FROM_LOCAL_DATA, preview approximation; rules and
/// evidence in docs/d120-bone-props.md). Contact points: pA = world(a) x aContact,
/// pB = world(b) x bContact (contacts are in each endpoint's local frame). The joint
/// mesh's bone-0 bind rotation is undone (bone axes aligned with world axes, bone local
/// Z = layer depth), and:
/// - Pivot: bone 0's bind position lands on pA.
/// - Span: the mesh's extent along its bind bone0->bone1 axis is fitted onto pA..pB,
///   rotated so that axis follows pA->pB (cross-section rigid, local Z kept nearest
///   world Z). Static stretch; the real-data loader replaces it with D147 skinning
///   (`skin_span_joints`) where the mesh's skin streams decode.
///
/// `bind` = bone skin-pose matrices of the joint mesh (column-major, bone order);
/// `bounds` = model-space bounds of the mesh.
pub fn joint_world(
    things: &[ThingRecord],
    joint: &lbp_level::parts::Joint,
    bind: &[[f32; 16]],
    bounds: Option<[[f32; 3]; 2]>,
) -> Result<([f32; 16], JointPlacement), String> {
    let base = joint_base(things, joint, bind)?;
    let (linear, translation) = match base.kind {
        JointPlacement::Pivot => (base.p0_inv, sub(base.pa, mul_mv(&base.p0_inv, base.t0))),
        JointPlacement::Span => {
            let fit = span_fit(&base, bind, bounds)?;
            let k = fit.length / (fit.hi - fit.lo);
            // Stretch along `axis` by k: I + (k - 1) axis axis^T.
            let stretch: M3 = std::array::from_fn(|c| {
                std::array::from_fn(|r| {
                    f64::from(u8::from(r == c)) + (k - 1.) * fit.axis[r] * fit.axis[c]
                })
            });
            let linear = mul_mm(&fit.rot, &mul_mm(&stretch, &base.p0_inv));
            let translation = sub(
                sub(base.pa, mul_mv(&linear, base.t0)),
                scaled(fit.dir, k * fit.lo),
            );
            (linear, translation)
        }
    };
    let m = affine(&linear, translation, [0.; 3]);
    world_matrix_check(&m).map_err(|e| format!("JOINT placement {e}"))?;
    Ok((m, base.kind))
}

/// Contact points and the de-rotating bone-0 bind frame shared by both placements.
struct JointBase {
    kind: JointPlacement,
    pa: [f64; 3],
    pb: [f64; 3],
    t0: [f64; 3],
    p0_inv: M3,
}

fn joint_base(
    things: &[ThingRecord],
    joint: &lbp_level::parts::Joint,
    bind: &[[f32; 16]],
) -> Result<JointBase, String> {
    let kind = joint_placement_kind(joint.joint_type).ok_or_else(|| {
        format!(
            "JOINT type {} without POS (placement not implemented)",
            joint.joint_type
        )
    })?;
    let endpoint = |e: Option<usize>, contact: [f32; 3]| -> Result<[f64; 3], String> {
        let pos = e
            .and_then(|i| things.get(i))
            .and_then(|t| t.data.pos.as_ref())
            .ok_or("JOINT endpoint without POS")?;
        world_matrix_check(&pos.world).map_err(|e| format!("JOINT endpoint {e}"))?;
        let p = transform_point(&pos.world, contact);
        p.iter()
            .all(|v| v.is_finite())
            .then_some(p)
            .ok_or_else(|| "JOINT contact not finite".to_string())
    };
    let pa = endpoint(joint.a, joint.a_contact)?;
    let pb = endpoint(joint.b, joint.b_contact)?;
    let skin0 = bind
        .first()
        .ok_or("JOINT mesh without bones")?
        .map(f64::from);
    let t0 = [skin0[12], skin0[13], skin0[14]];
    // Bone-0 bind rotation: skin-pose columns normalized; must be orthonormal.
    let mut p0: M3 = [[0.; 3]; 3];
    for (c, col) in p0.iter_mut().enumerate() {
        *col = unit([skin0[4 * c], skin0[4 * c + 1], skin0[4 * c + 2]])
            .ok_or("JOINT mesh bone 0 bind pose degenerate")?;
    }
    if (0..3).any(|i| (i + 1..3).any(|j| dot(p0[i], p0[j]).abs() > 1e-3)) {
        return Err("JOINT mesh bone 0 bind pose not a scaled rotation".into());
    }
    Ok(JointBase {
        kind,
        pa,
        pb,
        t0,
        p0_inv: transpose(&p0),
    })
}

/// Span geometry in the de-rotated bind frame (bone 0 at the origin): the bind axis
/// bone0 -> bone1, the mesh extent [lo, hi] along it, and the rotation taking the axis
/// onto the world direction pA -> pB (local Z kept nearest world Z).
struct SpanFit {
    axis: [f64; 3],
    dir: [f64; 3],
    length: f64,
    lo: f64,
    hi: f64,
    rot: M3,
}

fn span_fit(
    base: &JointBase,
    bind: &[[f32; 16]],
    bounds: Option<[[f32; 3]; 2]>,
) -> Result<SpanFit, String> {
    let skin1 = bind
        .get(1)
        .ok_or("span JOINT mesh with fewer than 2 bones")?
        .map(f64::from);
    let t1 = [skin1[12], skin1[13], skin1[14]];
    let axis = unit(mul_mv(&base.p0_inv, sub(t1, base.t0)))
        .ok_or("JOINT mesh bones coincide in bind pose")?;
    let d = sub(base.pb, base.pa);
    let length = dot(d, d).sqrt();
    let dir = unit(d).ok_or("span JOINT contacts coincide")?;
    let [lo_b, hi_b] = bounds.ok_or("JOINT mesh without bounds")?;
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for corner in 0..8 {
        let v = [
            if corner & 1 == 0 { lo_b[0] } else { hi_b[0] },
            if corner & 2 == 0 { lo_b[1] } else { hi_b[1] },
            if corner & 4 == 0 { lo_b[2] } else { hi_b[2] },
        ]
        .map(f64::from);
        let s = dot(mul_mv(&base.p0_inv, sub(v, base.t0)), axis);
        lo = lo.min(s);
        hi = hi.max(s);
    }
    if (hi - lo).is_nan() || hi - lo <= 1e-6 {
        return Err("JOINT mesh has no extent along its bone axis".into());
    }
    let (Some(f), Some(w)) = (frame(axis), frame(dir)) else {
        return Err("JOINT frame degenerate".into());
    };
    Ok(SpanFit {
        axis,
        dir,
        length,
        lo,
        hi,
        rot: mul_mm(&w, &transpose(&f)),
    })
}

/// Column-major affine matrix from a linear part and a translation minus `origin`.
fn affine(linear: &M3, translation: [f64; 3], origin: [f64; 3]) -> [f32; 16] {
    let mut m = [0f32; 16];
    for c in 0..3 {
        for r in 0..3 {
            m[4 * c + r] = linear[c][r] as f32;
        }
    }
    for r in 0..3 {
        m[12 + r] = (translation[r] - origin[r]) as f32;
    }
    m[15] = 1.;
    m
}

/// D147 span-joint skinning (INFERRED_FROM_LOCAL_DATA; docs/d147-joint-skinning.md).
/// Every bone of a linear joint's mesh is rigid (bind scale, no stretch) and follows
/// the endpoint on its side of the mesh: a bone whose bind position lies on the
/// bone-0 half of the mesh extent (along the bind axis bone0 -> bone1) follows contact
/// A, the others contact B. The A group's mesh end (axial `lo`) lands on pA and the B
/// group's end (`hi`) on pB; both are rotated so the bind axis follows pA -> pB. Only
/// vertices weighted to both groups (bridging cord / piston rod) stretch, by linear
/// blend skinning.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanSkin {
    /// World position the matrices are relative to (contact A, f32-rounded).
    pub origin: [f32; 3],
    /// Per mesh bone: model -> (world - origin), column-major.
    pub matrices: Vec<[f32; 16]>,
    /// Per mesh bone: true when the bone follows contact B.
    pub follows_b: Vec<bool>,
}

pub fn joint_span_skin(
    things: &[ThingRecord],
    joint: &lbp_level::parts::Joint,
    bind: &[[f32; 16]],
    bounds: Option<[[f32; 3]; 2]>,
) -> Result<SpanSkin, String> {
    let base = joint_base(things, joint, bind)?;
    if base.kind != JointPlacement::Span {
        return Err("not a span joint".into());
    }
    let fit = span_fit(&base, bind, bounds)?;
    let origin = base.pa.map(|v| v as f32);
    let origin64 = origin.map(f64::from);
    let linear = mul_mm(&fit.rot, &base.p0_inv);
    // Model point v -> anchor + rot (P0^T (v - t0) - end * axis).
    let side = |anchor: [f64; 3], end: f64| {
        let t = sub(
            sub(anchor, mul_mv(&linear, base.t0)),
            mul_mv(&fit.rot, scaled(fit.axis, end)),
        );
        affine(&linear, t, origin64)
    };
    let (ma, mb) = (side(base.pa, fit.lo), side(base.pb, fit.hi));
    let mid = (fit.lo + fit.hi) / 2.;
    let follows_b: Vec<bool> = bind
        .iter()
        .map(|s| {
            let t = [s[12], s[13], s[14]].map(f64::from);
            dot(mul_mv(&base.p0_inv, sub(t, base.t0)), fit.axis) > mid
        })
        .collect();
    if follows_b.first() != Some(&false) || !follows_b.iter().any(|&b| b) {
        return Err("span JOINT bones do not split into an A and a B group".into());
    }
    for m in [&ma, &mb] {
        world_matrix_check(m).map_err(|e| format!("JOINT skin {e}"))?;
    }
    Ok(SpanSkin {
        origin,
        matrices: follows_b.iter().map(|&b| if b { mb } else { ma }).collect(),
        follows_b,
    })
}

/// Linear blend skinning with `SpanSkin` matrices. Influences past the bone list or
/// with non-positive weight are dropped and the rest normalized; a vertex with no usable
/// influence follows bone 0 (contact A). Returns positions and that fallback count.
pub fn skin_span_positions(
    positions: &[[f32; 3]],
    joints: &[[u8; 4]],
    weights: &[[f32; 4]],
    matrices: &[[f32; 16]],
) -> Result<(Vec<[f32; 3]>, usize), String> {
    if joints.len() != positions.len() || weights.len() != positions.len() {
        return Err("skin streams do not match the vertex count".into());
    }
    let first = matrices.first().ok_or("no bone matrices")?;
    let mut unweighted = 0;
    let out = positions
        .iter()
        .zip(joints.iter().zip(weights))
        .map(|(&p, (j, w))| {
            let mut acc = [0f64; 3];
            let mut total = 0f64;
            for k in 0..4 {
                let Some(m) = matrices.get(usize::from(j[k])) else {
                    continue;
                };
                if !(w[k] > 0. && w[k].is_finite()) {
                    continue;
                }
                let q = transform_point(m, p);
                for c in 0..3 {
                    acc[c] += q[c] * f64::from(w[k]);
                }
                total += f64::from(w[k]);
            }
            if total > 0. {
                acc.map(|v| (v / total) as f32)
            } else {
                unweighted += 1;
                transform_point(first, p).map(|v| v as f32)
            }
        })
        .collect();
    Ok((out, unweighted))
}

/// Per-vertex bone indices and weights of a mesh (skin stream 1).
pub type SkinWeights = (Vec<[u8; 4]>, Vec<[f32; 4]>);

/// D147: replaces the D120 stretched placement of drawn span-JOINT items with skinned
/// copies of their mesh (one scene mesh per item, item matrix = translation to contact
/// A). `skin_of` returns a mesh's per-vertex bone indices and weights; `skinned` makes
/// the payload copy with new positions. Items that cannot be skinned keep the D120
/// stretch and are counted by reason.
pub fn skin_span_joints<T>(
    scene: &mut LevelScene<T>,
    things: &[ThingRecord],
    mut skin_of: impl FnMut(&SceneMesh<T>) -> Result<SkinWeights, String>,
    positions_of: impl Fn(&T) -> Option<&[[f32; 3]]>,
    bind_poses: impl Fn(&T) -> Vec<BindPose>,
    skinned: impl Fn(&T, Vec<[f32; 3]>) -> T,
) {
    let mut skins: HashMap<usize, Result<SkinWeights, String>> = HashMap::new();
    for item_index in 0..scene.items.len() {
        let item = &scene.items[item_index];
        let Some(t) = things.get(item.thing) else {
            continue;
        };
        let (None, Some(joint)) = (&t.data.pos, &t.data.joint) else {
            continue;
        };
        if joint_placement_kind(joint.joint_type) != Some(JointPlacement::Span) {
            continue;
        }
        let mesh_index = item.mesh;
        let result = (|| {
            if scene.meshes.len() >= MAX_LEVEL_MESHES {
                return Err("Unsupported: level distinct-mesh budget".to_string());
            }
            let m = &scene.meshes[mesh_index];
            let skins_bind: Vec<[f32; 16]> =
                bind_poses(&m.payload).iter().map(|b| b.skin).collect();
            let skin = joint_span_skin(things, joint, &skins_bind, m.bounds)?;
            let (joints, weights) = skins
                .entry(mesh_index)
                .or_insert_with(|| skin_of(m))
                .clone()?;
            let positions = positions_of(&m.payload).ok_or("mesh without positions")?;
            let (moved, unweighted) =
                skin_span_positions(positions, &joints, &weights, &skin.matrices)?;
            if moved.iter().flatten().any(|v| !v.is_finite()) {
                return Err("skinned position not finite".into());
            }
            let payload = skinned(&m.payload, moved);
            Ok((
                SceneMesh {
                    source: MeshSource::JointSkin {
                        mesh: Box::new(m.source.clone()),
                        thing: t.index,
                    },
                    bounds: None,
                    neutral_groups: m.neutral_groups,
                    payload,
                },
                skin.origin,
                unweighted,
            ))
        })();
        let c = &mut scene.counts;
        match result {
            Ok((mut mesh, origin, unweighted)) => {
                mesh.bounds = positions_of(&mesh.payload).and_then(points_bounds);
                scene.meshes.push(mesh);
                let item = &mut scene.items[item_index];
                item.mesh = scene.meshes.len() - 1;
                let mut world = [0f32; 16];
                for i in 0..4 {
                    world[5 * i] = 1.;
                }
                world[12..15].copy_from_slice(&origin);
                item.world = world;
                c.joint_span_skinned += 1;
                c.joint_skin_unweighted_vertices += unweighted;
            }
            Err(e) => *c.joint_span_skin_failed.entry(e).or_default() += 1,
        }
    }
}

/// P2m: draws POS model items whose RENDER_MESH lists two or more boneThings with each
/// vertex placed by its own bone: POS(boneThings[j]) x inverse bind(j), expressed in the
/// item's frame so the item matrix stays. Bones without a placed Thing keep the item
/// matrix. Items whose bones all match it are left unchanged. Same callbacks as
/// `skin_span_joints`.
pub fn skin_bone_things<T>(
    scene: &mut LevelScene<T>,
    things: &[ThingRecord],
    mut skin_of: impl FnMut(&SceneMesh<T>) -> Result<SkinWeights, String>,
    positions_of: impl Fn(&T) -> Option<&[[f32; 3]]>,
    bind_poses: impl Fn(&T) -> Vec<BindPose>,
    skinned: impl Fn(&T, Vec<[f32; 3]>) -> T,
) {
    const ITEM: [f32; 16] = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    scene.counts.per_bone = true;
    let pos_of = |i: usize| {
        things
            .get(i)
            .filter(|t| t.index == i)
            .and_then(|t| t.data.pos.as_ref())
            .map(|p| p.world)
    };
    let mut skins: HashMap<usize, Result<SkinWeights, String>> = HashMap::new();
    for item_index in 0..scene.items.len() {
        let item = &scene.items[item_index];
        let Some(t) = things.get(item.thing) else {
            continue;
        };
        let (Some(_), Some(rm)) = (&t.data.pos, &t.data.render_mesh) else {
            continue;
        };
        if rm.bone_things.len() < 2 {
            continue;
        }
        let (mesh_index, world) = (item.mesh, item.world);
        let result = (|| {
            let m = &scene.meshes[mesh_index];
            if !matches!(m.source, MeshSource::Guid(_) | MeshSource::Sha1(_)) {
                return Ok(None);
            }
            let to_item = crate::level_backdrop::affine_inverse(&world)?;
            let matrices: Vec<[f32; 16]> = bind_poses(&m.payload)
                .iter()
                .enumerate()
                .map(
                    |(j, b)| match rm.bone_things.get(j).copied().flatten().and_then(pos_of) {
                        Some(w) => mul44(&to_item, &mul44(&w, &b.inv)),
                        None => ITEM,
                    },
                )
                .collect();
            let near_item = |m: &[f32; 16]| m.iter().zip(ITEM).all(|(a, b)| (a - b).abs() < 1e-4);
            if matrices.iter().all(near_item) {
                return Ok(None);
            }
            if matrices.iter().flatten().any(|v| !v.is_finite()) {
                return Err("bone matrix not finite".to_string());
            }
            if scene.meshes.len() >= MAX_LEVEL_MESHES {
                return Err("Unsupported: level distinct-mesh budget".to_string());
            }
            let (joints, weights) = skins
                .entry(mesh_index)
                .or_insert_with(|| skin_of(m))
                .clone()?;
            let positions = positions_of(&m.payload).ok_or("mesh without positions")?;
            let (moved, unweighted) = skin_span_positions(positions, &joints, &weights, &matrices)?;
            if moved.iter().flatten().any(|v| !v.is_finite()) {
                return Err("skinned position not finite".into());
            }
            Ok(Some((
                SceneMesh {
                    source: MeshSource::BoneSkin {
                        mesh: Box::new(m.source.clone()),
                        thing: t.index,
                    },
                    bounds: None,
                    neutral_groups: m.neutral_groups,
                    payload: skinned(&m.payload, moved),
                },
                unweighted,
            )))
        })();
        let c = &mut scene.counts;
        match result {
            Ok(Some((mut mesh, unweighted))) => {
                mesh.bounds = positions_of(&mesh.payload).and_then(points_bounds);
                scene.meshes.push(mesh);
                scene.items[item_index].mesh = scene.meshes.len() - 1;
                c.per_bone_skinned += 1;
                c.per_bone_unweighted_vertices += unweighted;
            }
            Ok(None) => c.per_bone_unchanged += 1,
            Err(e) => *c.per_bone_failed.entry(e).or_default() += 1,
        }
    }
}

/// Builds the scene from reader Things. `load` is called once per distinct mesh
/// reference; failures are cached and count every Thing that names that mesh.
/// `bind_poses` returns a loaded mesh's bone bind poses (D120 placement).
pub fn build_scene<T>(
    things: &[ThingRecord],
    mut load: impl FnMut(&MeshSource) -> Result<T, String>,
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: impl Fn(&T) -> bool,
    bind_poses: impl Fn(&T) -> Vec<BindPose>,
    placement: PosPlacement,
) -> LevelScene<T> {
    build_scene_with_colors(
        things,
        |source, _| load(source),
        local_bounds,
        neutral_groups,
        bind_poses,
        |_| false,
        placement,
    )
}

/// D191: [`build_scene`] where `load(source, Some(editorColor))` loads a mesh for one
/// Thing colour. Each source first loads with `None` (the base inputs); a Thing with a
/// non-white editorColor gets its own (mesh, colour) load only when `uses_instance` says
/// the base load actually used the instance colour, so colour-blind meshes stay shared.
pub fn build_scene_with_colors<T>(
    things: &[ThingRecord],
    mut load: impl FnMut(&MeshSource, Option<u32>) -> Result<T, String>,
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: impl Fn(&T) -> bool,
    bind_poses: impl Fn(&T) -> Vec<BindPose>,
    uses_instance: impl Fn(&T) -> bool,
    placement: PosPlacement,
) -> LevelScene<T> {
    let mut scene = LevelScene::<T>::default();
    let mut by_source: MeshCache = HashMap::new();
    let c = &mut scene.counts;
    c.things_total = things.len();
    c.pos_placement = placement;
    let skip = |c: &mut SceneCounts, reason: String| *c.skipped.entry(reason).or_default() += 1;
    let lights = checkpoint_lights(things);
    for t in things {
        add_decorations(
            &mut scene,
            &mut by_source,
            &mut load,
            &local_bounds,
            &neutral_groups,
            &bind_poses,
            &uses_instance,
            placement,
            t,
        );
        let c = &mut scene.counts;
        let Some(rm) = &t.data.render_mesh else {
            continue;
        };
        c.with_render_mesh += 1;
        let source = match &rm.mesh {
            Some(d) => match descriptor_source(d) {
                Some(s) => s,
                None => {
                    skip(c, "RENDER_MESH with null mesh reference".into());
                    continue;
                }
            },
            None => {
                skip(c, "RENDER_MESH without mesh reference".into());
                continue;
            }
        };
        let pos = match (&t.data.pos, &t.data.joint) {
            (Some(pos), _) => {
                if let Err(e) = world_matrix_check(&pos.world) {
                    skip(c, e);
                    continue;
                }
                Some(pos)
            }
            (None, Some(_)) => None,
            (None, None) => {
                skip(c, "no POS and no JOINT (no placement source)".into());
                continue;
            }
        };
        let index = scene_mesh_index(
            &mut scene,
            &mut by_source,
            &mut load,
            &local_bounds,
            &neutral_groups,
            &uses_instance,
            &source,
            checkpoint_editor_color(rm.editor_color, lights.get(&t.index).copied()),
        );
        let c = &mut scene.counts;
        let (mesh, colored) = match index {
            Ok(i) => i,
            Err(e) => {
                skip(c, format!("mesh unavailable: {e}"));
                continue;
            }
        };
        if scene.items.len() >= MAX_LEVEL_ITEMS {
            skip(c, "Unsupported: level item budget".into());
            continue;
        }
        let world = match (pos, &t.data.joint) {
            (Some(pos), _) => {
                let world = match placement {
                    PosPlacement::PosOnly => pos.world,
                    PosPlacement::InverseBind => {
                        let m = &scene.meshes[mesh];
                        match bind_poses(&m.payload).first() {
                            None => {
                                c.pos_bind_no_bones += 1;
                                pos.world
                            }
                            Some(b0) => {
                                let w = mul44(&pos.world, &b0.inv);
                                if let Err(e) = world_matrix_check(&w) {
                                    skip(c, format!("POS x inverse bind: {e}"));
                                    continue;
                                }
                                if w == pos.world {
                                    c.pos_bind_unchanged += 1;
                                } else {
                                    c.pos_bind_changed += 1;
                                    let (shift, scale, rot) = placement_change(&pos.world, &w);
                                    let e = c
                                        .pos_bind_by_mesh
                                        .entry(m.source.label())
                                        .or_insert((0, 0., 0., 0.));
                                    e.0 += 1;
                                    e.1 = e.1.max(shift);
                                    e.2 = e.2.max(scale.abs());
                                    e.3 = e.3.max(rot);
                                }
                                w
                            }
                        }
                    }
                };
                c.drawn_bone_attached += usize::from(pos.bone_of.is_some_and(|b| b != t.index));
                world
            }
            (None, Some(joint)) => {
                let m = &scene.meshes[mesh];
                let skins: Vec<[f32; 16]> = bind_poses(&m.payload).iter().map(|b| b.skin).collect();
                match joint_world(things, joint, &skins, m.bounds) {
                    Ok((world, kind)) => {
                        match kind {
                            JointPlacement::Pivot => c.drawn_joint_pivot += 1,
                            JointPlacement::Span => c.drawn_joint_span += 1,
                        }
                        world
                    }
                    Err(e) => {
                        skip(c, e);
                        continue;
                    }
                }
            }
            (None, None) => unreachable!("skipped above"),
        };
        c.drawn += 1;
        c.drawn_with_neutral_groups += usize::from(scene.meshes[mesh].neutral_groups);
        c.drawn_editor_colored += usize::from(colored);
        scene.items.push(SceneItem {
            mesh,
            world,
            thing: t.index,
            uid: t.uid,
            shapes: 0,
            decoration: false,
            alternate: false,
        });
    }
    scene
}

/// D276: for every RENDER_MESH item of a POS Thing whose mesh is one of a pair in
/// `pairs`, adds an item for the pair's other mesh (`SceneItem::alternate`): the same
/// Thing and POS, under the other mesh's own bone-0 inverse bind (the placement rule of
/// [`build_scene`]). `load` is called once for a mesh the scene does not hold yet. The
/// new items follow the scene's items; they are not counted as drawn and stay out of
/// the bounds. Call it right after [`build_scene`], before shapes and decals are added.
#[allow(clippy::too_many_arguments)]
pub fn add_alternate_meshes<T>(
    scene: &mut LevelScene<T>,
    things: &[ThingRecord],
    pairs: &[[u32; 2]],
    mut load: impl FnMut(&MeshSource) -> Result<T, String>,
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: impl Fn(&T) -> bool,
    bind_poses: impl Fn(&T) -> Vec<BindPose>,
    placement: PosPlacement,
) {
    let wanted: Vec<(usize, Option<i32>, u32)> = scene
        .items
        .iter()
        .filter(|i| !i.decoration && !i.alternate && i.shapes == 0)
        .filter_map(|i| match scene.meshes[i.mesh].source {
            MeshSource::Guid(g) => Some((i.thing, i.uid, alternate_mesh(pairs, g)?)),
            _ => None,
        })
        .collect();
    for (thing, uid, other) in wanted {
        let source = MeshSource::Guid(other);
        let placed = (|| {
            let pos = things
                .iter()
                .find(|t| t.index == thing)
                .and_then(|t| t.data.pos.as_ref())?;
            let mesh = match scene.meshes.iter().position(|m| m.source == source) {
                Some(mesh) => mesh,
                None if scene.meshes.len() >= MAX_LEVEL_MESHES => return None,
                None => {
                    let payload = load(&source).ok()?;
                    scene.meshes.push(SceneMesh {
                        source: source.clone(),
                        bounds: local_bounds(&payload),
                        neutral_groups: neutral_groups(&payload),
                        payload,
                    });
                    scene.meshes.len() - 1
                }
            };
            let bind = bind_poses(&scene.meshes[mesh].payload);
            let world = match (placement, bind.first()) {
                (PosPlacement::InverseBind, Some(b0)) => mul44(&pos.world, &b0.inv),
                _ => pos.world,
            };
            world_matrix_check(&world).ok()?;
            (scene.items.len() < MAX_LEVEL_ITEMS).then_some((mesh, world))
        })();
        match placed {
            Some((mesh, world)) => {
                scene.counts.alternate_meshes += 1;
                scene.items.push(SceneItem {
                    mesh,
                    world,
                    thing,
                    uid,
                    shapes: 0,
                    decoration: false,
                    alternate: true,
                });
            }
            None => scene.counts.alternate_meshes_failed += 1,
        }
    }
}

/// Mesh source named by a RENDER_MESH-style descriptor (None for a null reference).
fn descriptor_source(d: &lbp_level::serial::Descriptor) -> Option<MeshSource> {
    match (d.guid, d.sha1) {
        (Some(g), _) if g != 0 => Some(MeshSource::Guid(g)),
        (_, Some(h)) if h.iter().any(|&b| b != 0) => Some(MeshSource::Sha1(
            h.iter().map(|b| format!("{b:02x}")).collect(),
        )),
        _ => None,
    }
}

/// Loads (once per source) a mesh into the scene; Err carries the unavailable reason.
/// Loaded scene meshes by (source, editorColor variant); `None` is the base load.
type MeshCache = HashMap<(MeshSource, Option<u32>), Result<usize, String>>;

/// D191: the scene mesh for a Thing with `editor_color`, and whether it is a non-white
/// colour variant (see [`build_scene_with_colors`]).
#[allow(clippy::too_many_arguments)]
fn scene_mesh_index<T>(
    scene: &mut LevelScene<T>,
    by_source: &mut MeshCache,
    load: &mut impl FnMut(&MeshSource, Option<u32>) -> Result<T, String>,
    local_bounds: &impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: &impl Fn(&T) -> bool,
    uses_instance: &impl Fn(&T) -> bool,
    source: &MeshSource,
    editor_color: u32,
) -> Result<(usize, bool), String> {
    let base = cached_mesh(
        scene,
        by_source,
        load,
        local_bounds,
        neutral_groups,
        source,
        None,
    )?;
    if editor_color == EDITOR_COLOR_WHITE || !uses_instance(&scene.meshes[base].payload) {
        return Ok((base, false));
    }
    let loaded_before = scene.counts.meshes_loaded;
    let variant = cached_mesh(
        scene,
        by_source,
        load,
        local_bounds,
        neutral_groups,
        source,
        Some(editor_color),
    )?;
    scene.counts.editor_color_variants += scene.counts.meshes_loaded - loaded_before;
    Ok((variant, true))
}

fn cached_mesh<T>(
    scene: &mut LevelScene<T>,
    by_source: &mut MeshCache,
    load: &mut impl FnMut(&MeshSource, Option<u32>) -> Result<T, String>,
    local_bounds: &impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: &impl Fn(&T) -> bool,
    source: &MeshSource,
    color: Option<u32>,
) -> Result<usize, String> {
    let key = (source.clone(), color);
    if let Some(r) = by_source.get(&key) {
        return r.clone();
    }
    let r = if scene.meshes.len() >= MAX_LEVEL_MESHES {
        Err("Unsupported: level distinct-mesh budget".to_string())
    } else {
        load(source, color).map(|payload| {
            scene.meshes.push(SceneMesh {
                source: source.clone(),
                bounds: local_bounds(&payload),
                neutral_groups: neutral_groups(&payload),
                payload,
            });
            scene.meshes.len() - 1
        })
    };
    if r.is_ok() {
        scene.counts.meshes_loaded += 1;
    } else {
        scene.counts.meshes_failed += 1;
    }
    by_source.insert(key, r.clone());
    r
}

/// D152: draws Thing `t`'s DECORATIONS entries. Each entry's `offset` is read as a
/// matrix in the owner Thing's POS frame (inferred from the stored translations: tens to
/// hundreds of units, z at the owner's front face), so world = POS x offset, then the
/// decoration mesh's bone-0 inverse bind under the inverse-bind rule (as POS Things).
#[allow(clippy::too_many_arguments)]
fn add_decorations<T>(
    scene: &mut LevelScene<T>,
    by_source: &mut MeshCache,
    load: &mut impl FnMut(&MeshSource, Option<u32>) -> Result<T, String>,
    local_bounds: &impl Fn(&T) -> Option<[[f32; 3]; 2]>,
    neutral_groups: &impl Fn(&T) -> bool,
    bind_poses: &impl Fn(&T) -> Vec<BindPose>,
    uses_instance: &impl Fn(&T) -> bool,
    placement: PosPlacement,
    t: &ThingRecord,
) {
    scene.counts.sticker_decals_total += t.data.sticker_decals;
    let skip = |c: &mut SceneCounts, reason: &str| {
        *c.decorations_skipped.entry(reason.to_string()).or_default() += 1
    };
    for d in &t.data.decorations {
        scene.counts.decorations_total += 1;
        let Some(rm) = &d.render_mesh else {
            skip(
                &mut scene.counts,
                "render mesh not read (null or unresolved reference)",
            );
            continue;
        };
        let Some(source) = rm.mesh.as_ref().and_then(descriptor_source) else {
            skip(&mut scene.counts, "null mesh reference");
            continue;
        };
        let Some(pos) = &t.data.pos else {
            skip(&mut scene.counts, "owner Thing has no POS");
            continue;
        };
        if world_matrix_check(&pos.world).is_err() || world_matrix_check(&d.offset).is_err() {
            skip(
                &mut scene.counts,
                "non-finite or degenerate POS/offset matrix",
            );
            continue;
        }
        let (mesh, colored) = match scene_mesh_index(
            scene,
            by_source,
            load,
            local_bounds,
            neutral_groups,
            uses_instance,
            &source,
            rm.editor_color,
        ) {
            Ok(i) => i,
            Err(e) => {
                skip(&mut scene.counts, &format!("mesh unavailable: {e}"));
                continue;
            }
        };
        let mut world = mul44(&pos.world, &d.offset);
        if placement == PosPlacement::InverseBind {
            if let Some(b0) = bind_poses(&scene.meshes[mesh].payload).first() {
                world = mul44(&world, &b0.inv);
            }
        }
        if world_matrix_check(&world).is_err() {
            skip(&mut scene.counts, "non-finite or degenerate placed matrix");
            continue;
        }
        if scene.items.len() >= MAX_LEVEL_ITEMS {
            skip(&mut scene.counts, "Unsupported: level item budget");
            continue;
        }
        scene.counts.decorations_drawn += 1;
        scene.counts.decorations_drawn_neutral += usize::from(scene.meshes[mesh].neutral_groups);
        scene.counts.drawn_editor_colored += usize::from(colored);
        scene.items.push(SceneItem {
            mesh,
            world,
            thing: t.index,
            uid: t.uid,
            shapes: 0,
            decoration: true,
            alternate: false,
        });
    }
}

/// Extrudes every SHAPE Thing (D114 selection rules) and adds the results as merged
/// world-space batches per gfx material, in material-GUID order then Thing order.
/// `wrap` turns a batch into the payload and reports whether its material selected.
pub fn add_shapes<T>(
    scene: &mut LevelScene<T>,
    things: &[ThingRecord],
    wrap: impl FnMut(ShapeBatch) -> (T, Result<(), String>),
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
) {
    add_shapes_with(scene, things, None, wrap, local_bounds)
}

/// [`add_shapes`] with optional bevels (D122): `Some(table)` extrudes every shape with the
/// bevel its GENERATED_MESH names (looked up in `table`; missing = straight walls) and
/// splits it by bevel-slot material; `None` is the D114 straight extrusion.
pub fn add_shapes_with<T>(
    scene: &mut LevelScene<T>,
    things: &[ThingRecord],
    bevels: Option<&BevelTable>,
    mut wrap: impl FnMut(ShapeBatch) -> (T, Result<(), String>),
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
) {
    // (gfx material GUID, PShape.color ARGB) -> (Thing index, extruded part).
    type ShapeKey = (Option<u32>, u32);
    let mut groups: BTreeMap<ShapeKey, Vec<(usize, MaterialPart)>> = BTreeMap::new();
    let bevel_options = BevelOptions::default();
    let mut vertices = 0usize;
    let roles = shape_mesh::ColliderRoles::new(things);
    for (position, t) in things
        .iter()
        .enumerate()
        .filter(|(_, t)| t.data.shape.is_some())
    {
        let c = &mut scene.counts;
        c.shapes_total += 1;
        if c.shapes_extruded >= MAX_LEVEL_SHAPES {
            *c.shapes_rejected
                .entry("level_shape_budget".into())
                .or_default() += 1;
            continue;
        }
        let extruded = match bevels {
            None => shape_mesh::extrude_thing(t),
            Some(table) => shape_mesh::extrude_thing_bevelled(
                t,
                |d: &Descriptor| table.get(&(d.guid, d.sha1)),
                &bevel_options,
            ),
        };
        match extruded {
            Ok(m) => {
                if vertices + m.positions.len() > MAX_LEVEL_SHAPE_VERTICES
                    || m.positions.len() > MAX_BATCH_VERTICES
                    || m.triangle_count() > MAX_BATCH_TRIANGLES
                {
                    *c.shapes_rejected
                        .entry("level_shape_vertex_budget".into())
                        .or_default() += 1;
                    continue;
                }
                vertices += m.positions.len();
                c.shapes_extruded += 1;
                if bevels.is_some() {
                    let key = match m.bevel {
                        shape_mesh::BevelOutcome::Fallback(r) => format!("bevel_fallback: {r}"),
                        o => o.key().to_string(),
                    };
                    *c.shape_bevel_outcomes.entry(key).or_default() += 1;
                }
                let color = t
                    .data
                    .shape
                    .as_ref()
                    .map_or(WHITE_ARGB, |s| shape_color_argb(&s.color));
                for part in m.material_parts() {
                    groups
                        .entry((part.material_guid(), color))
                        .or_default()
                        .push((t.index, part));
                }
            }
            Err(r) if r.is_selection() => {
                // D153: split no_generated_mesh by what stands in for the collider.
                let key = match r {
                    shape_mesh::ShapeReject::NoGeneratedMesh => roles.role(things, position).key(),
                    _ => r.reason(),
                };
                *c.shapes_not_selected.entry(key.into()).or_default() += 1;
            }
            Err(r) => *c.shapes_rejected.entry(r.reason().into()).or_default() += 1,
        }
    }
    let mut placed = std::collections::HashSet::new();
    for ((material, color), shapes) in groups {
        let mut batches = vec![ShapeBatch {
            material,
            color,
            ..Default::default()
        }];
        for (thing, m) in shapes {
            let current = batches.last_mut().expect("one batch");
            if current.positions.len() + m.positions.len() > MAX_BATCH_VERTICES
                || current.triangles.len() + m.triangle_count() > MAX_BATCH_TRIANGLES
            {
                batches.push(ShapeBatch {
                    material,
                    color,
                    ..Default::default()
                });
            }
            let b = batches.last_mut().expect("one batch");
            let base = b.positions.len() as u32;
            b.positions.extend_from_slice(&m.positions);
            b.uv0.extend_from_slice(&m.uv0);
            b.triangles.extend(
                m.indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| t.map(|i| i + base)),
            );
            b.things.push(thing);
        }
        for (batch_index, batch) in batches.into_iter().enumerate() {
            let n = batch.things.len();
            let first = batch.things.first().copied().unwrap_or_default();
            let things_in = batch.things.clone();
            let (payload, selected) = wrap(batch);
            let c = &mut scene.counts;
            if let Err(e) = &selected {
                *c.shapes_material_fallback.entry(e.clone()).or_default() += n;
            }
            if scene.items.len() >= MAX_LEVEL_ITEMS {
                *c.shapes_rejected
                    .entry("level_item_budget".into())
                    .or_default() += n;
                continue;
            }
            // A bevelled shape can span several material batches (D122): count each Thing
            // once, in the first batch that holds it (equals n without bevels).
            let newly = things_in.iter().filter(|&&t| placed.insert(t)).count();
            c.shapes_placed += newly;
            c.shape_batches += 1;
            scene.meshes.push(SceneMesh {
                source: MeshSource::Shapes {
                    material,
                    color,
                    batch: batch_index,
                },
                bounds: local_bounds(&payload),
                neutral_groups: selected.is_err(),
                payload,
            });
            scene.items.push(SceneItem {
                mesh: scene.meshes.len() - 1,
                world: shape_mesh::IDENTITY,
                thing: first,
                uid: None,
                shapes: newly,
                decoration: false,
                alternate: false,
            });
        }
    }
}

/// Real-data payload: a model mesh or a merged shape batch with its material outcome.
pub enum ScenePayload {
    Model(Box<GroupedMesh>),
    Shapes {
        batch: ShapeBatch,
        material: Result<std::sync::Arc<MaterialAsset>, String>,
    },
    /// D119: static backdrop mesh batch (mesh space, placed by the item matrix).
    Backdrop {
        batch: crate::level_backdrop::BackdropBatch,
        material: Result<std::sync::Arc<MaterialAsset>, String>,
        /// P3k: the static mesh's baked-AO lightmap (shared by every batch of the mesh); the
        /// batch's `uv1` is its unwrap.
        lightmap: Option<std::sync::Arc<Lightmap>>,
    },
    /// D160: world-space sticker decal overlays sharing one decal texture (drawn with
    /// alpha blending; `material` is a plain clamped texture, no gfx material graph).
    Decals {
        batch: DecalBatch,
        material: std::sync::Arc<MaterialAsset>,
    },
}

/// P3k: a backdrop static mesh's baked-AO lightmap, one byte per texel (top mip only; the
/// GTF is B8, read as the byte before the remap).
#[derive(Debug, PartialEq, Eq)]
pub struct Lightmap {
    pub width: u32,
    pub height: u32,
    pub r8: Vec<u8>,
}

/// P3k: decodes the static mesh's lightmap texture (B8 GTF, <= 16 Mi texels) to one byte per
/// texel. A texture that is not a plain 2D map of at most 4096 x 4096 is refused.
fn load_lightmap(
    source: &AttachedSource,
    catalog: &Catalog,
    descriptor: &lbp_mesh::static_mesh::StaticDescriptor,
    control: &LoadControl,
) -> Result<Lightmap, String> {
    let hash = descriptor
        .sha1
        .map(|h| h.iter().map(|b| format!("{b:02x}")).collect::<String>());
    let key = resolve(catalog, descriptor.guid, hash.as_deref())?;
    let texture = lbp_texture::decode_reader(
        key.size,
        |offset, count| {
            source
                .read_hex_checked(&key, offset, count, control)
                .map(|p| p.bytes)
                .map_err(lbp_texture::Error::Input)
        },
        || control.check().is_err(),
    )
    .map_err(|e| e.to_string())?;
    let (width, height) = (texture.header.width, texture.header.height);
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(format!("Unsupported: lightmap extent {width}x{height}"));
    }
    let r8: Vec<u8> = texture
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[0])
        .collect();
    if r8.len() != width as usize * height as usize {
        return Err("Invalid: lightmap pixel count".into());
    }
    Ok(Lightmap { width, height, r8 })
}

/// D160: merged world-space decal overlay triangles for one decal texture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecalBatch {
    pub texture: (Option<u32>, Option<[u8; 20]>),
    pub positions: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Owner Thing indices (one per drawn decal).
    pub things: Vec<usize>,
    pub decals: usize,
}

impl DecalBatch {
    pub fn texture_label(&self) -> String {
        match self.texture {
            (Some(g), _) => format!("{g:08x}"),
            (None, Some(h)) => h.iter().map(|b| format!("{b:02x}")).collect(),
            (None, None) => "none".into(),
        }
    }
}

/// D160: projects every main-array STICKERS decal of a SHAPE owner onto its front face
/// (`lbp_level::decal_mesh`), merges them per owner Thing and decal texture (so a QA focus
/// on the owner frames its decals) and adds one scene item per batch. `load` turns a texture key into the display material (or an error, counted per
/// decal). Costume decal arrays and non-shape owners are counted by reason, not drawn.
pub fn add_sticker_decals<T>(
    scene: &mut LevelScene<T>,
    things: &[ThingRecord],
    mut load: impl FnMut(&DecalBatch) -> Result<T, String>,
    local_bounds: impl Fn(&T) -> Option<[[f32; 3]; 2]>,
) {
    type Key = (usize, Option<u32>, Option<[u8; 20]>);
    let mut groups: BTreeMap<Key, Vec<(usize, lbp_level::decal_mesh::DecalMesh)>> = BTreeMap::new();
    let skip = |c: &mut SceneCounts, reason: &str, n: usize| {
        if n > 0 {
            *c.sticker_decals_skipped.entry(reason.into()).or_default() += n;
        }
    };
    let mut vertices = 0usize;
    for t in things {
        let main = t.data.stickers.len();
        skip(
            &mut scene.counts,
            "costume decal array (costume pieces not projected)",
            t.data.sticker_decals.saturating_sub(main),
        );
        if main == 0 {
            continue;
        }
        if t.data.shape.is_none() {
            let reason = if t.data.render_mesh.is_some() {
                "owner is a mesh (mesh UV decal projection unsupported)"
            } else {
                "owner has no surface (logic / microchip / switch Thing)"
            };
            skip(&mut scene.counts, reason, main);
            continue;
        }
        let face = match shape_mesh::front_face(t) {
            Ok(f) => f,
            Err(r) => {
                skip(
                    &mut scene.counts,
                    &format!("owner shape not drawn: {}", r.reason()),
                    main,
                );
                continue;
            }
        };
        for (order, d) in t.data.stickers.iter().enumerate() {
            let Some(tex) = &d.texture else {
                skip(&mut scene.counts, "decal: no texture reference", 1);
                continue;
            };
            match lbp_level::decal_mesh::project(&face, d, order) {
                Ok(m) => {
                    if vertices + m.positions.len() > MAX_LEVEL_SHAPE_VERTICES
                        || m.positions.len() > MAX_BATCH_VERTICES
                    {
                        skip(&mut scene.counts, "decal: level vertex budget", 1);
                        continue;
                    }
                    vertices += m.positions.len();
                    groups
                        .entry((t.index, tex.guid, tex.sha1))
                        .or_default()
                        .push((t.index, m));
                }
                Err(r) => skip(&mut scene.counts, r.reason(), 1),
            }
        }
    }
    for ((_, guid, sha1), decals) in groups {
        let texture = (guid, sha1);
        let mut batches = vec![DecalBatch {
            texture,
            ..Default::default()
        }];
        for (thing, m) in decals {
            let current = batches.last_mut().expect("one batch");
            if current.positions.len() + m.positions.len() > MAX_BATCH_VERTICES
                || current.triangles.len() + m.indices.len() / 3 > MAX_BATCH_TRIANGLES
            {
                batches.push(DecalBatch {
                    texture,
                    ..Default::default()
                });
            }
            let b = batches.last_mut().expect("one batch");
            let base = b.positions.len() as u32;
            b.positions.extend_from_slice(&m.positions);
            b.uv.extend_from_slice(&m.uv);
            b.triangles.extend(
                m.indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| t.map(|i| i + base)),
            );
            b.things.push(thing);
            b.decals += 1;
        }
        for (index, batch) in batches.into_iter().enumerate() {
            let n = batch.decals;
            let first = batch.things.first().copied().unwrap_or_default();
            let label = batch.texture_label();
            let payload = match load(&batch) {
                Ok(p) => p,
                Err(e) => {
                    skip(&mut scene.counts, &format!("decal texture: {e}"), n);
                    continue;
                }
            };
            if scene.items.len() >= MAX_LEVEL_ITEMS {
                skip(&mut scene.counts, "level_item_budget", n);
                continue;
            }
            scene.counts.sticker_decals_drawn += n;
            scene.meshes.push(SceneMesh {
                source: MeshSource::Decals {
                    texture: label,
                    batch: index,
                },
                bounds: local_bounds(&payload),
                neutral_groups: false,
                payload,
            });
            scene.items.push(SceneItem {
                mesh: scene.meshes.len() - 1,
                world: shape_mesh::IDENTITY,
                thing: first,
                uid: None,
                shapes: 0,
                decoration: false,
                alternate: false,
            });
        }
    }
}

/// Resolves a level by GUID or by MAP path (exact, or a `/`-suffix of the stored path,
/// case-insensitive). Returns the location and the matching MAP path(s).
pub fn resolve_level(
    catalog: &Catalog,
    guid: Option<u32>,
    path: Option<&str>,
) -> Result<(LocationKey, String), String> {
    let mut guids = std::collections::BTreeSet::new();
    let mut paths = std::collections::BTreeSet::new();
    let wanted = path.map(|p| p.trim().replace('\\', "/").to_ascii_lowercase());
    for r in &catalog.snapshot.resources {
        let row_path = r.map_row.path.replace('\\', "/").to_ascii_lowercase();
        let row_guid = u32::from_str_radix(r.map_row.guid.trim(), 16).ok();
        let hit = match (&wanted, guid) {
            (Some(w), _) => row_path == *w || row_path.ends_with(&format!("/{w}")),
            (None, Some(g)) => row_guid == Some(g),
            (None, None) => return Err("Level GUID or path required".into()),
        };
        if hit {
            if r.display_type() != "level" {
                return Err(format!(
                    "{} is a {} resource, not a magic-confirmed level",
                    r.map_row.path,
                    r.display_type()
                ));
            }
            paths.insert(r.map_row.path.clone());
            if let Some(g) = row_guid {
                guids.insert(g);
            }
        }
    }
    if paths.is_empty() {
        return Err("Unresolved: no level MAP row matches".into());
    }
    if guids.len() != 1 {
        return Err(format!(
            "Ambiguous level: {} GUIDs match ({})",
            guids.len(),
            paths.into_iter().collect::<Vec<_>>().join(" | ")
        ));
    }
    let key = resolve(catalog, guids.first().copied(), None)?;
    Ok((key, paths.into_iter().collect::<Vec<_>>().join(" | ")))
}

/// Reads a level's stored bytes (bounded by the level input budget).
pub fn read_level_bytes(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    read_resource(source, key, control, lbp_level::level::MAX_INPUT)
}

/// Reads a whole stored resource in pages, refusing stored sizes above `max`.
pub fn read_resource(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
    max: usize,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > max {
        return Err("Unsupported: stored resource exceeds the input budget".into());
    }
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let page = source.read_hex_checked(
            key,
            bytes.len() as u64,
            (size - bytes.len()).min(PAGE),
            control,
        )?;
        if page.bytes.is_empty() {
            return Err("Invalid: truncated level read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

/// D147: a mesh's geometry and its decoded skin streams (stream 1 joints/weights).
pub fn read_mesh_skin(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<(lbp_mesh::Geometry, lbp_mesh::SkinStreams), String> {
    let (geometry, bytes) = crate::read_mesh(source, key, control)?;
    let skin = lbp_mesh::decode_skin(&bytes, || control.check().is_err())
        .map_err(|e| format!("Skin streams: {e}"))?;
    if skin.joints.len() != geometry.positions.len() || skin.weights.len() != skin.joints.len() {
        return Err("Skin streams: vertex count differs from geometry".into());
    }
    Ok((geometry, skin))
}

fn model_bind_poses(p: &ScenePayload) -> Vec<BindPose> {
    match p {
        ScenePayload::Model(m) => m
            .geometry
            .skeleton
            .bones
            .iter()
            .map(|b| BindPose {
                skin: b.skin_pose,
                inv: b.inv_skin_pose,
            })
            .collect(),
        ScenePayload::Shapes { .. }
        | ScenePayload::Backdrop { .. }
        | ScenePayload::Decals { .. } => Vec::new(),
    }
}

fn grouped_bounds(g: &GroupedMesh) -> Option<[[f32; 3]; 2]> {
    points_bounds(&g.geometry.positions)
}

fn payload_bounds(p: &ScenePayload) -> Option<[[f32; 3]; 2]> {
    match p {
        ScenePayload::Model(g) => grouped_bounds(g),
        ScenePayload::Shapes { batch, .. } => points_bounds(&batch.positions),
        ScenePayload::Backdrop { batch, .. } => points_bounds(&batch.positions),
        ScenePayload::Decals { batch, .. } => points_bounds(&batch.positions),
    }
}

fn points_bounds(points: &[[f32; 3]]) -> Option<[[f32; 3]; 2]> {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in points {
        if p.iter().any(|v| !v.is_finite()) {
            return None;
        }
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo[0] <= hi[0]).then_some([lo, hi])
}

/// D191: some group's material took the supplied instance colour.
fn uses_instance_color(g: &GroupedMesh) -> bool {
    g.bindings
        .iter()
        .flatten()
        .any(|b| b.instance_projection().is_some())
}

/// `--level-report`: the distinct binding errors behind a mesh's neutral groups, with
/// how many non-empty groups each one leaves neutral. A shape or backdrop batch is one
/// group: its gfx material error, when the batch has triangles (empty for decals).
pub fn neutral_reasons(p: &ScenePayload) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    match p {
        ScenePayload::Model(g) => {
            for (range, b) in g.geometry.primitive_triangles.iter().zip(&g.bindings) {
                if let (false, Err(e)) = (range.is_empty(), b) {
                    match out.iter_mut().find(|(r, _)| r == e) {
                        Some((_, n)) => *n += 1,
                        None => out.push((e.clone(), 1)),
                    }
                }
            }
        }
        ScenePayload::Shapes {
            batch,
            material: Err(e),
        } if !batch.triangles.is_empty() => out.push((e.clone(), 1)),
        ScenePayload::Backdrop {
            batch,
            material: Err(e),
            ..
        } if !batch.triangles.is_empty() => out.push((e.clone(), 1)),
        _ => {}
    }
    out
}

/// D192: material-less groups are hidden (the game skips them), so they are not neutral.
fn grouped_neutral(g: &GroupedMesh) -> bool {
    g.geometry
        .primitive_triangles
        .iter()
        .zip(&g.bindings)
        .enumerate()
        .any(|(i, (range, b))| !range.is_empty() && b.is_err() && !g.materialless(i))
}

fn retained(g: &GroupedMesh) -> usize {
    let mut seen = Vec::new();
    let mut total = 0usize;
    for b in g.bindings.iter().flatten() {
        if !seen.iter().any(|s| std::sync::Arc::ptr_eq(s, b)) {
            seen.push(b.clone());
            total = total.saturating_add(b.retained_bytes());
        }
    }
    total
}

/// Reads the level and loads every referenced mesh (default D120 POS placement). Run on
/// a `LEVEL_STACK_BYTES` stack.
pub fn load_level_scene(
    source: &AttachedSource,
    catalog: &Catalog,
    level: &LocationKey,
    control: &LoadControl,
    options: PreviewOptions,
) -> Result<LevelScene<ScenePayload>, String> {
    load_level_scene_with_options(
        source,
        catalog,
        level,
        control,
        options,
        LevelSceneOptions::default(),
    )
}

/// Loads (once each) every bevel the level's GENERATED_MESH parts name (D122).
fn load_bevels(
    source: &AttachedSource,
    catalog: &Catalog,
    things: &[ThingRecord],
    control: &LoadControl,
    counts: &mut SceneCounts,
) -> Result<BevelTable, String> {
    let mut wanted = std::collections::BTreeSet::new();
    for t in things {
        if let Some(d) = t.data.generated_mesh.as_ref().and_then(|g| g.bevel) {
            wanted.insert((d.guid, d.sha1));
        }
    }
    let mut table = BevelTable::new();
    for (i, (guid, sha1)) in wanted.into_iter().enumerate() {
        control.check()?;
        if i >= MAX_LEVEL_BEVELS {
            counts.bevels_failed += 1;
            continue;
        }
        let loaded = (|| {
            let hash = sha1.map(|h| lbp_level::serial::hex(&h));
            let key = match (guid, &hash) {
                (Some(g), _) => resolve(catalog, Some(g), None)?,
                (None, Some(h)) => resolve(catalog, None, Some(h))?,
                _ => return Err("no identifier".to_string()),
            };
            if key.size > lbp_level::bevel::MAX_INPUT as u64 {
                return Err("bevel input budget".into());
            }
            let bytes = read_resource(source, &key, control, lbp_level::bevel::MAX_INPUT)?;
            lbp_level::bevel::read(&bytes, &|| control.check().is_err()).map_err(|e| e.to_string())
        })();
        match loaded {
            Ok(b) => {
                counts.bevels_loaded += 1;
                table.insert((guid, sha1), b);
            }
            Err(_) => counts.bevels_failed += 1,
        }
    }
    Ok(table)
}

/// [`load_level_scene`] with level-scene options (D122 `shape_bevels`).
pub fn load_level_scene_with_options(
    source: &AttachedSource,
    catalog: &Catalog,
    level: &LocationKey,
    control: &LoadControl,
    options: PreviewOptions,
    scene_options: LevelSceneOptions,
) -> Result<LevelScene<ScenePayload>, String> {
    let bytes = read_level_bytes(source, level, control)?;
    // Default depth bound (256) fits the 512 MiB stack; deepest local level is 162.
    let limits = lbp_level::Limits::default();
    let report = lbp_level::read(&bytes, limits, &|| control.check().is_err())
        .map_err(|e| format!("Level wrapper: {e}"))?;
    control.check()?;
    if report.things.is_empty() {
        return Err(format!(
            "Level not read: {}",
            report
                .stop
                .as_ref()
                .map_or("no Things".into(), |s| format!("{s:?}"))
        ));
    }
    let mut retained_total = 0usize;
    let mut charge = |bytes: usize| -> Result<(), String> {
        retained_total = retained_total
            .checked_add(bytes)
            .filter(|&t| t <= MAX_LEVEL_RETAINED_BYTES)
            .ok_or("Unsupported: level material memory budget (1536 MiB retained)")?;
        Ok(())
    };
    crate::runtime_texture::validate(&scene_options.runtime_textures)?;
    let mut runtime_counts: crate::runtime_texture::RuntimeCounts = scene_options
        .runtime_textures
        .iter()
        .map(|r| (*r, Default::default()))
        .collect();
    // D191: explicit inputs win; the backdrop takes the standalone white. D194: shapes
    // take their own PShape.color when the instance default is on.
    let explicit_color = options.instance_color;
    let (options, _) = scene_options.defaults.apply(options);
    let thing_colors = scene_options.defaults.instance_white && explicit_color.is_none();
    let shape_colors = thing_colors;
    let shape_options = PreviewOptions {
        instance_color: explicit_color,
        ..options
    };
    let mut load_model =
        |src: &MeshSource, editor_color: Option<u32>| -> Result<ScenePayload, String> {
            control.check()?;
            let key = match src {
                MeshSource::Guid(g) => resolve(catalog, Some(*g), None)?,
                MeshSource::Sha1(h) => resolve(catalog, None, Some(h))?,
                MeshSource::Shapes { .. }
                | MeshSource::Backdrop { .. }
                | MeshSource::Decals { .. }
                | MeshSource::JointSkin { .. }
                | MeshSource::BoneSkin { .. } => return Err("Invalid: shape source".into()),
            };
            let mut options = options;
            if let Some(c) = editor_color.filter(|_| thing_colors) {
                options.instance_color = Some(editor_color_tint(c));
            }
            let mut mesh = load_mesh_groups_with_options(source, catalog, &key, control, options)?;
            if let MeshSource::Guid(g) = src {
                apply_runtime_textures(
                    *g,
                    &mut mesh,
                    &scene_options.runtime_textures,
                    &mut runtime_counts,
                    source,
                    catalog,
                    &key,
                    control,
                );
                control.check()?;
            }
            charge(retained(&mesh))?;
            Ok(ScenePayload::Model(Box::new(mesh)))
        };
    let neutral = |p: &ScenePayload| matches!(p, ScenePayload::Model(m) if grouped_neutral(m));
    let mut scene = build_scene_with_colors(
        &report.things,
        &mut load_model,
        payload_bounds,
        neutral,
        model_bind_poses,
        |p| thing_colors && matches!(p, ScenePayload::Model(m) if uses_instance_color(m)),
        scene_options.placement,
    );
    if !scene_options.mesh_alternates.is_empty() {
        // D276: the other mesh of a pair a script swaps, loaded with the base inputs.
        add_alternate_meshes(
            &mut scene,
            &report.things,
            &scene_options.mesh_alternates,
            |source| load_model(source, None),
            payload_bounds,
            neutral,
            model_bind_poses,
            scene_options.placement,
        );
    }
    scene.counts.runtime_textures = runtime_counts;
    control.check()?;
    skin_span_joints(
        &mut scene,
        &report.things,
        |m| {
            control.check()?;
            let key = match &m.source {
                MeshSource::Guid(g) => resolve(catalog, Some(*g), None)?,
                MeshSource::Sha1(h) => resolve(catalog, None, Some(h))?,
                _ => return Err("Invalid: joint mesh source".into()),
            };
            let (_, skin) = read_mesh_skin(source, &key, control)?;
            Ok((skin.joints, skin.weights))
        },
        |p| match p {
            ScenePayload::Model(m) => Some(&m.geometry.positions[..]),
            _ => None,
        },
        model_bind_poses,
        |p, positions| match p {
            ScenePayload::Model(m) => {
                let mut copy = GroupedMesh {
                    geometry: m.geometry.clone(),
                    bindings: m.bindings.clone(),
                    // Positions moved by the bone matrices; the stored normals no longer match.
                    normals: None,
                };
                copy.geometry.positions = positions;
                ScenePayload::Model(Box::new(copy))
            }
            _ => unreachable!("positions_of returns None for non-model payloads"),
        },
    );
    if scene_options.per_bone {
        skin_bone_things(
            &mut scene,
            &report.things,
            |m| {
                control.check()?;
                let key = match &m.source {
                    MeshSource::Guid(g) => resolve(catalog, Some(*g), None)?,
                    MeshSource::Sha1(h) => resolve(catalog, None, Some(h))?,
                    _ => return Err("Invalid: bone mesh source".into()),
                };
                let (_, skin) = read_mesh_skin(source, &key, control)?;
                Ok((skin.joints, skin.weights))
            },
            |p| match p {
                ScenePayload::Model(m) => Some(&m.geometry.positions[..]),
                _ => None,
            },
            model_bind_poses,
            |p, positions| match p {
                ScenePayload::Model(m) => {
                    let mut copy = GroupedMesh {
                        geometry: m.geometry.clone(),
                        bindings: m.bindings.clone(),
                        normals: None,
                    };
                    copy.geometry.positions = positions;
                    ScenePayload::Model(Box::new(copy))
                }
                _ => unreachable!("positions_of returns None for non-model payloads"),
            },
        );
    }
    control.check()?;
    let bevels = if scene_options.shape_bevels {
        Some(load_bevels(
            source,
            catalog,
            &report.things,
            control,
            &mut scene.counts,
        )?)
    } else {
        None
    };
    add_shapes_with(
        &mut scene,
        &report.things,
        bevels.as_ref(),
        |batch| {
            let material = (|| {
                control.check()?;
                let guid = batch
                    .material
                    .ok_or("Unsupported: GENERATED_MESH without gfx material GUID")?;
                let key = resolve(catalog, Some(guid), None)?;
                // D194: with the level defaults on, a shape's own PShape.color is its
                // instance colour (THING_COLOR link INFERRED, docs/d194-shape-color.md).
                let mut shape_options = shape_options;
                if shape_colors {
                    shape_options.instance_color = Some(editor_color_tint(batch.color));
                }
                let asset = crate::linked::load_material(
                    source,
                    catalog,
                    key,
                    batch.positions.len(),
                    crate::linked::UvInput::Generated(&batch.uv0),
                    control,
                    shape_options,
                )?;
                charge(asset.retained_bytes())?;
                Ok(std::sync::Arc::new(asset))
            })();
            let selected = material.as_ref().map(|_| ()).map_err(Clone::clone);
            (ScenePayload::Shapes { batch, material }, selected)
        },
        payload_bounds,
    );
    control.check()?;
    // Decoded decal textures shared by every owner batch that uses them.
    let mut decoded = HashMap::new();
    add_sticker_decals(
        &mut scene,
        &report.things,
        |batch| {
            control.check()?;
            let asset = load_decal_texture(source, catalog, batch, &mut decoded, control)?;
            charge(asset.retained_bytes())?;
            Ok(ScenePayload::Decals {
                batch: batch.clone(),
                material: std::sync::Arc::new(asset),
            })
        },
        payload_bounds,
    );
    control.check()?;
    load_backdrop(
        &mut scene,
        &report.things,
        source,
        catalog,
        control,
        options,
        &mut charge,
    )?;
    control.check()?;
    source.check_stable()?;
    scene.counts.reader_stop = report.stop.as_ref().map(|s| format!("{s:?}"));
    scene.settings = LevelSettings::find(&report.things).map(|(_, s)| *s);
    scene.settings_presets = LevelSettings::find_presets(&report.things).to_vec();
    scene.world_lighting = report
        .things
        .iter()
        .find_map(|t| t.data.world.as_ref())
        .map(|w| w.lighting);
    scene.checkpoints = checkpoints(&report.things);
    Ok(scene)
}

/// P2e: applies `--runtime-texture` rules to one loaded model mesh. The mesh bytes are
/// re-read (UV0) and the texture decoded only when a group matches.
#[allow(clippy::too_many_arguments)]
fn apply_runtime_textures(
    guid: u32,
    mesh: &mut GroupedMesh,
    rules: &[crate::runtime_texture::RuntimeTexture],
    counts: &mut crate::runtime_texture::RuntimeCounts,
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
) {
    let vertices = mesh.geometry.positions.len();
    crate::runtime_texture::apply(
        guid,
        crate::runtime_texture::Groups {
            ranges: &mesh.geometry.primitive_triangles,
            vertices,
            bindings: &mut mesh.bindings,
        },
        rules,
        counts,
        |channel| {
            let (_, data) = crate::read_mesh(source, key, control)?;
            let coords = lbp_mesh::decode_uvs(&data, || control.check().is_err())
                .map_err(|e| e.to_string())?;
            let uv = crate::linked::selected_uv(&coords, channel, vertices)?;
            Ok((uv, coords.metadata))
        },
        |texture| {
            control.check()?;
            let texture_key = resolve(catalog, Some(texture), None)?;
            let decoded = lbp_texture::decode_reader(
                texture_key.size,
                |offset, count| {
                    source
                        .read_hex_checked(&texture_key, offset, count, control)
                        .map(|p| p.bytes)
                        .map_err(lbp_texture::Error::Input)
                },
                || control.check().is_err(),
            )
            .map_err(|e| e.to_string())?;
            Ok((decoded, texture_key))
        },
        |material, slot| {
            let (inspection, _) = crate::linked::read_material_graph(source, material, control)?;
            crate::runtime_texture::slot_sampler(&inspection.graph, slot)
        },
    );
}

/// P2c: CHECKPOINT Things with a usable POS world matrix (spawn candidates).
pub fn checkpoints(things: &[ThingRecord]) -> Vec<(usize, [f32; 16])> {
    things
        .iter()
        .filter(|t| t.parts.contains(&"CHECKPOINT"))
        .filter_map(|t| {
            let pos = t.data.pos.as_ref()?;
            world_matrix_check(&pos.world).ok()?;
            Some((t.index, pos.world))
        })
        .collect()
}

/// D160: decodes a decal texture as a plain clamped texture material sampled with the
/// batch's decal coordinates (no gfx material graph; colour field not applied).
fn load_decal_texture(
    source: &AttachedSource,
    catalog: &Catalog,
    batch: &DecalBatch,
    decoded: &mut HashMap<LocationKey, Result<lbp_texture::Texture, String>>,
    control: &LoadControl,
) -> Result<MaterialAsset, String> {
    let hash = batch
        .texture
        .1
        .map(|h| h.iter().map(|b| format!("{b:02x}")).collect::<String>());
    let key = resolve(catalog, batch.texture.0, hash.as_deref())?;
    let texture = decoded
        .entry(key.clone())
        .or_insert_with(|| {
            lbp_texture::decode_reader(
                key.size,
                |offset, count| {
                    source
                        .read_hex_checked(&key, offset, count, control)
                        .map(|p| p.bytes)
                        .map_err(lbp_texture::Error::Input)
                },
                || control.check().is_err(),
            )
            .map_err(|e| e.to_string())
        })
        .as_ref()
        .map(|t| lbp_texture::Texture {
            header: t.header.clone(),
            expanded_bytes: t.expanded_bytes,
            top_mip_block_bytes: t.top_mip_block_bytes,
            unparsed_after_top_mip: t.unparsed_after_top_mip,
            rgba: t.rgba.clone(),
        })
        .map_err(Clone::clone)?;
    Ok(MaterialAsset::Texture(Box::new(crate::linked::Asset {
        surface: None,
        instance_projection: None,
        additional_texture_slots: Vec::new(),
        border_domain: None,
        fourth: None,
        third: None,
        dual: None,
        product: None,
        tinted_rgba: None,
        runtime_texture: None,
        fragment: None,
        sampling_uv: batch.uv.clone(),
        uv: batch.uv.clone(),
        uv_metadata: lbp_mesh::UvMetadata {
            vertex_count: u32::try_from(batch.uv.len()).map_err(|_| "decal vertex count")?,
            channel_count: 1,
            channels: Vec::new(),
        },
        texture,
        material_key: key.clone(),
        texture_key: key,
        wrap: crate::TextureWrapping::uniform(false),
        selection: crate::DiffuseSelection::default(),
    })))
}

/// D119: draws the backdrop static mesh and the backdrop plan's RENDER_MESH Things.
/// Route failures are counted in `counts.backdrop.errors`; only cancellation propagates.
fn load_backdrop(
    scene: &mut LevelScene<ScenePayload>,
    things: &[ThingRecord],
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
    options: PreviewOptions,
    charge: &mut dyn FnMut(usize) -> Result<(), String>,
) -> Result<(), String> {
    use crate::level_backdrop as bd;
    let refs = bd::backdrop_refs(things);
    let mut counts = bd::BackdropCounts::default();
    let Some(thing) = refs.thing else {
        counts.source = Some("none (PWorld backdrop Thing is null)".into());
        scene.counts.backdrop = counts;
        return Ok(());
    };
    let world = refs.world.unwrap_or(bd::IDENTITY);
    counts.thing = Some(thing);
    counts.source = Some(format!(
        "Thing {thing}{}; static mesh {}; plan {}",
        if refs.world.is_none() {
            " (no POS, identity)"
        } else {
            ""
        },
        refs.mesh.as_ref().map_or("none".into(), MeshSource::label),
        refs.plan.map_or("none".into(), |g| format!("{g:08x}"))
    ));
    if let Some(mesh_source) = &refs.mesh {
        control.check()?;
        let loaded = (|| {
            let key = match mesh_source {
                MeshSource::Guid(g) => resolve(catalog, Some(*g), None)?,
                MeshSource::Sha1(h) => resolve(catalog, None, Some(h))?,
                _ => return Err("Invalid: backdrop mesh source".to_string()),
            };
            let bytes = read_resource(source, &key, control, lbp_mesh::static_mesh::MAX_INPUT)?;
            lbp_mesh::static_mesh::read(&bytes, || control.check().is_err())
                .map_err(|e| e.to_string())
        })();
        control.check()?;
        match loaded {
            Ok(mesh) => {
                let guid = match mesh_source {
                    MeshSource::Guid(g) => Some(*g),
                    _ => None,
                };
                // P3k: baked AO. A failure leaves the mesh unlit by AO and is counted.
                let lightmap = match mesh.lightmaps[0].as_ref() {
                    Some(d) => match control
                        .check()
                        .and_then(|_| load_lightmap(source, catalog, d, control))
                    {
                        Ok(l) => match charge(l.r8.len()) {
                            Ok(()) => Some(std::sync::Arc::new(l)),
                            Err(e) => {
                                counts.errors.push(format!("lightmap: {e}"));
                                None
                            }
                        },
                        Err(e) => {
                            counts.errors.push(format!("lightmap: {e}"));
                            None
                        }
                    },
                    None => None,
                };
                control.check()?;
                bd::add_backdrop_mesh(
                    scene,
                    &mut counts,
                    guid,
                    &mesh,
                    world,
                    thing,
                    |batch| {
                        let material = (|| {
                            control.check()?;
                            let d = batch
                                .material
                                .as_ref()
                                .ok_or("Unsupported: backdrop primitive without gfx material")?;
                            let hash = d
                                .sha1
                                .map(|h| h.iter().map(|b| format!("{b:02x}")).collect::<String>());
                            let key = resolve(catalog, d.guid, hash.as_deref())?;
                            let asset = crate::linked::load_material(
                                source,
                                catalog,
                                key,
                                batch.positions.len(),
                                crate::linked::UvInput::Static {
                                    uv0: &batch.uv0,
                                    uv1: &batch.uv1,
                                },
                                control,
                                options,
                            )?;
                            charge(asset.retained_bytes())?;
                            Ok(std::sync::Arc::new(asset))
                        })();
                        let selected = material.as_ref().map(|_| ()).map_err(Clone::clone);
                        (
                            ScenePayload::Backdrop {
                                batch,
                                material,
                                lightmap: lightmap.clone(),
                            },
                            selected,
                        )
                    },
                    payload_bounds,
                );
            }
            Err(e) => counts
                .errors
                .push(format!("static mesh {}: {e}", mesh_source.label())),
        }
    }
    if let Some(plan) = refs.plan {
        control.check()?;
        let report = (|| {
            let key = resolve(catalog, Some(plan), None)?;
            let bytes = read_resource(source, &key, control, lbp_level::plan::MAX_INPUT)?;
            lbp_level::read_plan(&bytes, lbp_level::Limits::default(), &|| {
                control.check().is_err()
            })
            .map_err(|e| format!("plan wrapper: {e}"))
        })();
        control.check()?;
        if let Ok(r) = &report {
            scene.backdrop_presets = LevelSettings::find_presets(&r.things).to_vec();
        }
        match report {
            Ok(r) if r.stop.is_none() => bd::add_backdrop_plan(
                scene,
                &mut counts,
                &r.things,
                world,
                thing,
                |src| {
                    control.check()?;
                    let key = match src {
                        MeshSource::Guid(g) => resolve(catalog, Some(*g), None)?,
                        MeshSource::Sha1(h) => resolve(catalog, None, Some(h))?,
                        _ => return Err("Invalid: plan mesh source".into()),
                    };
                    let mesh =
                        load_mesh_groups_with_options(source, catalog, &key, control, options)?;
                    charge(retained(&mesh))?;
                    Ok(ScenePayload::Model(Box::new(mesh)))
                },
                payload_bounds,
                |p| matches!(p, ScenePayload::Model(m) if grouped_neutral(m)),
                |p| match p {
                    ScenePayload::Model(m) => m
                        .geometry
                        .skeleton
                        .bones
                        .iter()
                        .map(|b| BindPose {
                            skin: b.skin_pose,
                            inv: b.inv_skin_pose,
                        })
                        .collect(),
                    _ => Vec::new(),
                },
                scene.counts.pos_placement,
            ),
            Ok(r) => counts.errors.push(format!(
                "plan {plan:08x} not read: {:?} ({} Things before the stop)",
                r.stop,
                r.things.len()
            )),
            Err(e) => counts.errors.push(format!("plan {plan:08x}: {e}")),
        }
    }
    scene.counts.backdrop = counts;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two one-triangle primitives: 0 material-less, 1 with an unresolved GUID.
    fn two_group_mesh(second: Result<std::sync::Arc<MaterialAsset>, String>) -> GroupedMesh {
        let primitive = |guid: Option<u32>| lbp_mesh::Primitive {
            material_flags: 0,
            material_kind: if guid.is_some() { 2 } else { 0 },
            material_guid: guid,
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
        GroupedMesh {
            geometry: lbp_mesh::Geometry {
                metadata: lbp_mesh::GeometryMetadata {
                    prefix: lbp_mesh::MeshPrefix {
                        revision: 950,
                        compression_flags: 7,
                        counts: [3, 6, 0, 0, 2, 0, 0],
                        morph_names_hex: Vec::new(),
                        min_uv_bits: Vec::new(),
                        max_uv_bits: Vec::new(),
                        area_scale_bits: None,
                        stream_offsets: Vec::new(),
                        position_bounds: None,
                        position_bounds_bits: None,
                        position_bytes_sha256: String::new(),
                        attribute_bytes: 0,
                        attribute_sha256: String::new(),
                        index_bytes: 12,
                        index_sha256: String::new(),
                        expanded_bytes: 0,
                        parsed_prefix_bytes: 0,
                        unparsed_bytes: 0,
                    },
                    primitives: vec![primitive(None), primitive(Some(0x1234))],
                    bone_count: 1,
                    primitive_type: 5,
                    triangle_count: 2,
                    triangle_indices_sha256: String::new(),
                    parsed_geometry_bytes: 0,
                    unparsed_after_geometry: 0,
                },
                positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
                triangles: vec![[0, 1, 2], [0, 2, 1]],
                primitive_triangles: vec![0..1, 1..2],
                skeleton: Default::default(),
            },
            bindings: vec![Err(crate::MATERIALLESS_REASON.into()), second],
            normals: None,
        }
    }

    #[test]
    fn materialless_groups_are_hidden_not_neutral_d192() {
        let other = two_group_mesh(Err("Unresolved: missing".into()));
        assert!(grouped_neutral(&other));
        let reasons = neutral_reasons(&ScenePayload::Model(Box::new(other)));
        assert_eq!(
            reasons,
            vec![
                (crate::MATERIALLESS_REASON.to_string(), 1),
                ("Unresolved: missing".to_string(), 1)
            ]
        );
        let mut only = two_group_mesh(Err("x".into()));
        only.geometry.primitive_triangles = vec![0..2, 2..2];
        assert!(!grouped_neutral(&only));
    }

    #[test]
    fn shape_batch_neutral_reason_is_its_material_error() {
        let batch = ShapeBatch {
            material: Some(0x12ba7),
            color: WHITE_ARGB,
            positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            uv0: vec![[0., 0.]; 3],
            triangles: vec![[0, 1, 2]],
            things: vec![7],
        };
        let failed = ScenePayload::Shapes {
            batch: batch.clone(),
            material: Err("Unsupported: x".into()),
        };
        assert_eq!(
            neutral_reasons(&failed),
            vec![("Unsupported: x".to_string(), 1)]
        );
        let empty = ScenePayload::Shapes {
            batch: ShapeBatch {
                triangles: Vec::new(),
                ..batch
            },
            material: Err("Unsupported: x".into()),
        };
        assert!(neutral_reasons(&empty).is_empty());
    }
    use lbp_level::parts::{GeneratedMesh, Joint, PartData, Pos, RenderMesh, Shape, ShapeColor};
    use lbp_level::serial::Descriptor;

    fn identity_at(x: f32) -> [f32; 16] {
        let mut m = [0.; 16];
        m[0] = 1.;
        m[5] = 1.;
        m[10] = 1.;
        m[15] = 1.;
        m[12] = x;
        m
    }

    fn thing(index: usize, guid: Option<u32>, world: Option<[f32; 16]>) -> ThingRecord {
        ThingRecord {
            index,
            uid: Some(index as i32 + 1),
            data: PartData {
                pos: world.map(|world| Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world,
                }),
                render_mesh: Some(RenderMesh {
                    mesh: guid.map(|g| Descriptor {
                        flags: None,
                        kind_byte: 2,
                        guid: Some(g),
                        sha1: None,
                    }),
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
        }
    }

    fn decoration(mesh: Option<u32>, offset: [f32; 16]) -> lbp_level::parts::Decoration {
        let rm = thing(0, mesh, None).data.render_mesh;
        lbp_level::parts::Decoration {
            render_mesh: mesh.and(rm),
            offset,
            parent_bone: 0,
            parent_tri_vert: -1,
            bary_u: 0.,
            bary_v: 0.,
            angle: 0.,
            on_costume_piece: -1,
            earth_decoration: 0,
            scale: 1.,
            placed_by: 0,
            reversed: false,
            play_mode_frame: 0,
            plan_guid: None,
        }
    }

    #[test]
    fn decorations_draw_at_owner_pos_times_offset() {
        let mut offset = identity_at(5.);
        offset[13] = 7.;
        offset[0] = 2.;
        let mut owner = thing(0, None, Some(identity_at(100.)));
        owner.data.render_mesh = None;
        owner.data.sticker_decals = 3;
        owner.data.decorations = vec![
            decoration(Some(0x40), offset),
            decoration(None, offset),
            decoration(Some(0x41), offset), // load fails
        ];
        let mut no_pos = thing(1, None, None);
        no_pos.data.render_mesh = None;
        no_pos.data.decorations = vec![decoration(Some(0x40), offset)];
        let scene = build_scene(
            &[owner, no_pos],
            |s| match s {
                MeshSource::Guid(0x41) => Err("Unresolved: GUID".into()),
                MeshSource::Guid(g) => Ok(*g),
                _ => unreachable!(),
            },
            |_| None,
            |_| false,
            |_| Vec::new(),
            PosPlacement::InverseBind,
        );
        let c = &scene.counts;
        assert_eq!((c.decorations_total, c.decorations_drawn), (4, 1));
        assert_eq!(c.sticker_decals_total, 3);
        assert_eq!(c.drawn, 0);
        assert_eq!(
            c.decorations_skipped["render mesh not read (null or unresolved reference)"],
            1
        );
        assert_eq!(
            c.decorations_skipped["mesh unavailable: Unresolved: GUID"],
            1
        );
        assert_eq!(c.decorations_skipped["owner Thing has no POS"], 1);
        assert_eq!(c.decorations_skipped.values().sum::<usize>(), 3);
        assert_eq!(scene.items.len(), 1);
        let w = scene.items[0].world;
        assert_eq!(
            (w[0], w[12], w[13], scene.items[0].thing),
            (2., 105., 7., 0)
        );
    }

    /// D276. A Thing drawn with one mesh of a pair gets a second item for the other
    /// mesh, at its POS under that mesh's own inverse bind; other Things get none; the
    /// second item is not counted as drawn and stays out of the bounds.
    #[test]
    fn d276_a_thing_with_a_mesh_of_a_pair_gets_an_item_for_the_other() {
        let pairs = [[0x10, 0x11]];
        assert_eq!(alternate_mesh(&pairs, 0x10), Some(0x11));
        assert_eq!(alternate_mesh(&pairs, 0x11), Some(0x10));
        assert_eq!(alternate_mesh(&pairs, 0x20), None);
        assert_eq!(alternate_mesh(&[], 0x10), None);
        assert_eq!(alternate_mesh(&[[7, 7]], 7), None);
        let mut swapped = thing(0, Some(0x10), Some(identity_at(100.)));
        swapped.data.decorations = vec![decoration(Some(0x10), identity_at(1.))];
        let things = [swapped, thing(1, Some(0x20), Some(identity_at(0.)))];
        let load = |s: &MeshSource| match s {
            MeshSource::Guid(0x12) => Err("Unresolved: GUID".to_string()),
            MeshSource::Guid(g) => Ok(*g),
            _ => unreachable!(),
        };
        // The bounds grow with the GUID; mesh 0x11 is bound 3 units along x.
        let bounds = |g: &u32| Some([[-1.; 3], [*g as f32; 3]]);
        let bind = |g: &u32| {
            let skin = identity_at(if *g == 0x11 { 3. } else { 0. });
            let inv = identity_at(-skin[12]);
            vec![BindPose { skin, inv }]
        };
        let build = || {
            build_scene(
                &things,
                load,
                bounds,
                |_| false,
                bind,
                PosPlacement::InverseBind,
            )
        };
        let plain = build();
        let mut scene = build();
        let mut loads = Vec::new();
        let mut add = |scene: &mut LevelScene<u32>, pairs: &[[u32; 2]]| {
            add_alternate_meshes(
                scene,
                &things,
                pairs,
                |s| {
                    loads.push(s.clone());
                    load(s)
                },
                bounds,
                |_| false,
                bind,
                PosPlacement::InverseBind,
            )
        };
        add(&mut scene, &pairs);
        assert_eq!(scene.items.len(), plain.items.len() + 1);
        assert_eq!(scene.items[..plain.items.len()], plain.items[..]);
        let extra = scene.items.last().unwrap();
        assert!(extra.alternate && !extra.decoration);
        assert_eq!((extra.thing, extra.uid, extra.world[12]), (0, Some(1), 97.));
        assert_eq!(scene.meshes[extra.mesh].source, MeshSource::Guid(0x11));
        assert_eq!(scene.items.iter().filter(|i| i.alternate).count(), 1);
        let c = &scene.counts;
        assert_eq!((c.alternate_meshes, c.alternate_meshes_failed), (1, 0));
        assert_eq!((c.drawn, c.meshes_loaded), (plain.counts.drawn, 2));
        assert_eq!(scene.bounds(), plain.bounds());
        assert!(c.lines().iter().any(|l| l.starts_with("Alternate meshes")));
        assert!(!plain.counts.lines().iter().any(|l| l.contains("Alternate")));
        // A mesh that does not load: counted, no item. A mesh the scene already holds
        // is not loaded again.
        add(&mut scene, &[[0x20, 0x12], [0x10, 0x11]]);
        let c = &scene.counts;
        assert_eq!((c.alternate_meshes, c.alternate_meshes_failed), (2, 1));
        assert_eq!(loads, [MeshSource::Guid(0x11), MeshSource::Guid(0x12)]);
    }

    #[test]
    fn synthetic_scene_shares_meshes_and_counts_skips() {
        let mut nan = identity_at(0.);
        nan[13] = f32::NAN;
        let mut flat = identity_at(0.);
        flat[10] = 0.;
        let plain = ThingRecord {
            index: 7,
            ..Default::default()
        };
        let things = vec![
            thing(0, Some(0x10), Some(identity_at(0.))),
            thing(1, Some(0x10), Some(identity_at(10.))),
            thing(2, Some(0x20), Some(identity_at(-5.))),
            thing(3, Some(0x30), Some(identity_at(3.))), // load fails
            thing(4, Some(0x30), Some(identity_at(4.))), // cached failure
            thing(5, None, Some(identity_at(0.))),
            thing(6, Some(0x10), None),
            thing(8, Some(0x10), Some(nan)),
            thing(9, Some(0x10), Some(flat)),
            plain,
        ];
        let mut calls = Vec::new();
        let scene = build_scene(
            &things,
            |s| {
                calls.push(s.clone());
                match s {
                    MeshSource::Guid(0x30) => Err("Unresolved: GUID".into()),
                    MeshSource::Guid(g) => Ok(*g),
                    _ => unreachable!(),
                }
            },
            |_| Some([[-1., -1., -1.], [1., 1., 1.]]),
            |&g| g == 0x20,
            |_| Vec::new(),
            PosPlacement::PosOnly,
        );
        assert_eq!(
            calls,
            vec![
                MeshSource::Guid(0x10),
                MeshSource::Guid(0x20),
                MeshSource::Guid(0x30)
            ]
        );
        let c = &scene.counts;
        assert_eq!((c.things_total, c.with_render_mesh, c.drawn), (10, 9, 3));
        assert_eq!((c.meshes_loaded, c.meshes_failed), (2, 1));
        assert_eq!(c.drawn_with_neutral_groups, 1);
        assert_eq!(c.skipped_total(), 6);
        assert_eq!(c.skipped["mesh unavailable: Unresolved: GUID"], 2);
        assert_eq!(c.skipped["RENDER_MESH without mesh reference"], 1);
        assert_eq!(c.skipped["no POS and no JOINT (no placement source)"], 1);
        assert_eq!((c.drawn_joint_pivot, c.drawn_joint_span), (0, 0));
        assert_eq!(c.skipped["non-finite world matrix"], 1);
        assert_eq!(c.skipped["non-invertible world matrix"], 1);
        assert_eq!(scene.meshes.len(), 2);
        assert_eq!(
            scene.items.iter().map(|i| i.mesh).collect::<Vec<_>>(),
            vec![0, 0, 1]
        );
        assert_eq!(scene.bounds(), Some([[-6., -1., -1.], [11., 1., 1.]]));
        assert!(c
            .lines()
            .iter()
            .any(|l| l == "Placed (scene items): 3 Things"));
    }

    fn joint_thing(index: usize, guid: u32, joint_type: i32, a: usize, b: usize) -> ThingRecord {
        let mut t = thing(index, Some(guid), None);
        t.data.joint = Some(Joint {
            a: Some(a),
            b: Some(b),
            a_contact: [1., 2., 0.],
            b_contact: [0., 0., 0.],
            length: 0.,
            angle: 0.,
            settings: None,
            bone_idx: vec![-1, -1],
            bone_lengths: [1., 1., 1., 1.],
            joint_type,
            render_scale: 1.,
            hide_in_play_mode: false,
            params: Default::default(),
        });
        t
    }

    /// Bind pose like the local joint meshes: scale s, local Y -> model +Z, local Z ->
    /// model -Y (column-major), translation `t`.
    fn joint_bone(s: f32, t: [f32; 3]) -> [f32; 16] {
        [
            s, 0., 0., 0., 0., 0., s, 0., 0., -s, 0., 0., t[0], t[1], t[2], 1.,
        ]
    }

    fn apply(m: &[f32; 16], p: [f32; 3]) -> [f64; 3] {
        transform_point(m, p)
    }

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
    }

    fn skins_only(skins: &[[f32; 16]]) -> Vec<BindPose> {
        skins
            .iter()
            .map(|&skin| BindPose { skin, inv: skin })
            .collect()
    }

    #[test]
    fn pos_things_use_inverse_bind_of_bone_zero_by_default() {
        // Bone 0 bind: scale 2, local Y -> model +Z, local Z -> model -Y, at (0, 0, 10).
        let skin = joint_bone(2., [0., 0., 10.]);
        // Stored inverse: model -> bone local.
        let inv = [
            0.5, 0., 0., 0., 0., 0., -0.5, 0., 0., 0.5, 0., 0., 0., -5., 0., 1.,
        ];
        assert_eq!(mul44(&skin, &inv), identity_at(0.));
        let mut pos = identity_at(100.);
        pos[0] = 2.;
        pos[5] = 2.;
        pos[10] = 2.;
        let things = vec![
            thing(0, Some(0x10), Some(pos)),             // bones: changes
            thing(1, Some(0x20), Some(identity_at(7.))), // identity bind: unchanged
            thing(2, Some(0x30), Some(identity_at(9.))), // no bones: keeps POS
        ];
        let build = |placement| {
            build_scene(
                &things,
                |s| match s {
                    MeshSource::Guid(g) => Ok(*g),
                    _ => unreachable!(),
                },
                |_| Some([[-1.; 3], [1.; 3]]),
                |_| false,
                |&g| match g {
                    0x10 => vec![BindPose { skin, inv }],
                    0x20 => vec![BindPose {
                        skin: identity_at(0.),
                        inv: identity_at(0.),
                    }],
                    _ => Vec::new(),
                },
                placement,
            )
        };
        // Old D113 path: POS alone.
        let old = build(PosPlacement::PosOnly);
        assert_eq!(
            old.items.iter().map(|i| i.world).collect::<Vec<_>>(),
            vec![pos, identity_at(7.), identity_at(9.)]
        );
        assert_eq!(old.counts.pos_bind_changed, 0);
        assert!(old
            .counts
            .lines()
            .iter()
            .any(|l| l.starts_with("POS placement: POS only (D113")));
        // New rule: POS x inverse bind of bone 0.
        let new = build(PosPlacement::default());
        let w = new.items[0].world;
        assert_eq!(w, mul44(&pos, &inv));
        // Model point at the bone's bind position lands on POS translation; model -Y
        // (local +Z) maps to world +Z with the POS scale only.
        assert!(close(apply(&w, [0., 0., 10.]), [100., 0., 0.]));
        assert!(close(apply(&w, [0., -1., 10.]), [100., 0., 1.]));
        assert!(close(apply(&w, [0., 0., 11.]), [100., 1., 0.]));
        assert_eq!(new.items[1].world, identity_at(7.));
        assert_eq!(new.items[2].world, identity_at(9.));
        let c = &new.counts;
        assert_eq!(
            (
                c.pos_bind_changed,
                c.pos_bind_unchanged,
                c.pos_bind_no_bones
            ),
            (1, 1, 1)
        );
        let (n, shift, _, rot) = c.pos_bind_by_mesh["00000010"];
        assert_eq!(n, 1);
        assert!((shift - 10.).abs() < 1e-4 && (rot - 90.).abs() < 1e-3);
        assert!(c
            .lines()
            .iter()
            .any(|l| l == "Changed vs D113 POS-only: 1 Things (1 unchanged, 1 meshes without bones kept at POS)"));
    }

    #[test]
    fn joint_things_place_from_endpoints() {
        // Endpoints: A at (100, 0, 0), B at (100, 0, -100) for the bolt; C at (0, 50, 0)
        // for the string from A's contact (101, 2, 0) to C.
        let things = vec![
            thing(0, Some(0x10), Some(identity_at(100.))),
            {
                let mut m = identity_at(100.);
                m[14] = -100.;
                let mut t = thing(1, Some(0x10), Some(m));
                t.data.pos.as_mut().unwrap().world[12] = 101.;
                t.data.pos.as_mut().unwrap().world[13] = 2.;
                t
            },
            {
                let mut m = identity_at(0.);
                m[13] = 50.;
                thing(2, Some(0x10), Some(m))
            },
            joint_thing(3, 0x40, 7, 0, 1), // bolt: pivot at A contact
            joint_thing(4, 0x50, 5, 0, 2), // string: span A -> C
            joint_thing(5, 0x40, 0, 0, 1), // legacy type: skipped
            {
                let mut t = joint_thing(6, 0x40, 7, 0, 1);
                t.data.joint.as_mut().unwrap().b = None; // endpoint missing
                t
            },
            joint_thing(7, 0x60, 5, 0, 2), // one-bone mesh: span impossible
        ];
        // POS Things keep their exact matrices.
        let bolt_bind = vec![
            joint_bone(0.1, [0., 0., 0.]),
            joint_bone(0.08, [0., 0., 0.]),
        ];
        let string_bind = vec![
            joint_bone(1.4, [0., 0., 26.]),
            joint_bone(1.7, [0., 0., -26.]),
        ];
        let scene = build_scene(
            &things,
            |s| match s {
                MeshSource::Guid(g) => Ok(*g),
                _ => unreachable!(),
            },
            |&g| match g {
                0x50 => Some([[-20., -10., -88.], [20., 10., 88.]]),
                _ => Some([[-40., -32., -40.], [40., 0., 40.]]),
            },
            |_| false,
            |&g| match g {
                0x40 => skins_only(&bolt_bind),
                0x50 => skins_only(&string_bind),
                0x60 => skins_only(&[joint_bone(1., [0.; 3])]),
                _ => Vec::new(),
            },
            PosPlacement::InverseBind,
        );
        let c = &scene.counts;
        assert_eq!(
            (c.drawn, c.drawn_joint_pivot, c.drawn_joint_span),
            (5, 1, 1)
        );
        assert_eq!(
            c.skipped["JOINT type 0 without POS (placement not implemented)"],
            1
        );
        assert_eq!(c.skipped["JOINT endpoint without POS"], 1);
        assert_eq!(c.skipped["span JOINT mesh with fewer than 2 bones"], 1);
        for (item, t) in scene.items.iter().zip(&things).take(3) {
            assert_eq!(item.world, t.data.pos.as_ref().unwrap().world);
        }
        // Bolt: model origin (bone 0) at contact A (101, 2, 0); model -Y -> world +Z,
        // model +Z -> world +Y, unit scale.
        let bolt = &scene.items[3];
        assert_eq!(bolt.thing, 3);
        assert!(close(apply(&bolt.world, [0., 0., 0.]), [101., 2., 0.]));
        assert!(close(apply(&bolt.world, [0., -32., 0.]), [101., 2., 32.]));
        assert!(close(apply(&bolt.world, [0., 0., 40.]), [101., 42., 0.]));
        assert!(close(apply(&bolt.world, [40., 0., 0.]), [141., 2., 0.]));
        // String: model extent along the bone0 -> bone1 axis (model z from 88 to -88)
        // fitted onto A (101, 2, 0) .. C (0, 50, 0); cross-section kept at unit scale.
        let string = &scene.items[4];
        assert_eq!(string.thing, 4);
        assert!(close(apply(&string.world, [0., 0., 88.]), [101., 2., 0.]));
        assert!(close(apply(&string.world, [0., 0., -88.]), [0., 50., 0.]));
        let mid = apply(&string.world, [0., 0., 0.]);
        let side = apply(&string.world, [0., -10., 0.]);
        assert!(close(sub(side, mid), [0., 0., 10.]));
        let across = sub(apply(&string.world, [20., 0., 0.]), mid);
        assert!((dot(across, across).sqrt() - 20.).abs() < 1e-3);
        assert!(dot(across, sub([0., 50., 0.], [101., 2., 0.])).abs() < 1e-2);
        assert!(c.lines().iter().any(|l| l.starts_with(
            "Placed from JOINT endpoints (D120 preview approximation): 2 Things (1 pivot"
        )));
    }

    /// D187: skinned per-item copies keep the mesh GUID for lookups (the Pod shell's
    /// DualShock lookup failed under --per-bone-placement).
    #[test]
    fn d187_base_guid_looks_through_skinned_copies() {
        let bone = MeshSource::BoneSkin {
            mesh: Box::new(MeshSource::Guid(0x42af)),
            thing: 3,
        };
        let joint = MeshSource::JointSkin {
            mesh: Box::new(bone),
            thing: 1,
        };
        assert_eq!(joint.base_guid(), Some(0x42af));
        assert_eq!(MeshSource::Sha1("ab".into()).base_guid(), None);
    }

    /// D147: span joints skin with rigid A/B bone groups; only blended vertices stretch.
    #[test]
    fn p2m_bone_things_place_each_vertex_by_its_bone() {
        let mut squash = identity_at(0.);
        squash[10] = 0.5;
        let mut earth = thing(0, Some(0x10), Some(identity_at(0.)));
        let mut moon = thing(2, Some(0x10), Some(identity_at(100.)));
        earth.data.render_mesh.as_mut().unwrap().bone_things = vec![Some(0), Some(1)];
        moon.data.render_mesh.as_mut().unwrap().bone_things = vec![Some(2), Some(2)];
        let things = vec![earth, thing(1, None, Some(squash)), moon];
        let mesh: Vec<[f32; 3]> = vec![[0., 0., 10.], [0., 0., 10.], [1., 0., 10.]];
        let joints = vec![[0, 0, 0, 0], [1, 0, 0, 0], [0; 4]];
        let weights = vec![[1., 0., 0., 0.], [1., 0., 0., 0.], [0.; 4]];
        let id = identity_at(0.);
        let binds = || vec![BindPose { skin: id, inv: id }; 2];
        let mut scene = build_scene(
            &things,
            |_| Ok(mesh.clone()),
            |p| points_bounds(p),
            |_| false,
            |_| binds(),
            PosPlacement::InverseBind,
        );
        let worlds: Vec<_> = scene.items.iter().map(|i| i.world).collect();
        skin_bone_things(
            &mut scene,
            &things,
            |_| Ok((joints.clone(), weights.clone())),
            |p| Some(&p[..]),
            |_| binds(),
            |_, positions| positions,
        );
        let c = &scene.counts;
        assert!(c.per_bone);
        assert_eq!(
            (
                c.per_bone_skinned,
                c.per_bone_unchanged,
                c.per_bone_unweighted_vertices
            ),
            (1, 1, 1)
        );
        assert!(c.per_bone_failed.is_empty());
        // Item matrices stay; the Earth gets its own copy with bone 1 squashed in z.
        assert_eq!(
            scene.items.iter().map(|i| i.world).collect::<Vec<_>>(),
            worlds
        );
        let item = scene.items.iter().find(|i| i.thing == 0).unwrap();
        assert_eq!(
            scene.meshes[item.mesh].source,
            MeshSource::BoneSkin {
                mesh: Box::new(MeshSource::Guid(0x10)),
                thing: 0
            }
        );
        assert_eq!(
            scene.meshes[item.mesh].payload,
            vec![[0., 0., 10.], [0., 0., 5.], [1., 0., 10.]]
        );
        let moon = scene.items.iter().find(|i| i.thing == 2).unwrap();
        assert_eq!(scene.meshes[moon.mesh].source, MeshSource::Guid(0x10));
        assert!(c
            .lines()
            .iter()
            .any(|l| l.starts_with("Per-bone placement")));
    }

    #[test]
    fn span_joints_skin_rigid_ends_and_stretch_the_bridge() {
        let things = vec![
            thing(0, Some(0x10), Some(identity_at(0.))),
            thing(1, Some(0x10), Some(identity_at(500.))),
            joint_thing(2, 0x70, 4, 0, 1), // piston A (1, 2, 0) -> B (500, 0, 0)
            joint_thing(3, 0x40, 7, 0, 1), // bolt: pivot, untouched
        ];
        // Piston-like: bones 0/2 on the +Z (A) end, bones 1/3 on the -Z (B) end.
        let piston_bind = vec![
            joint_bone(1.15, [0., 0., 26.]),
            joint_bone(1.7, [0., 0., -26.]),
            joint_bone(0.55, [0., 0., 26.]),
            joint_bone(0.5, [0., 0., -26.]),
        ];
        let piston: Vec<[f32; 3]> = vec![
            [0., 0., 88.],  // A tip, bone 0
            [0., 0., 9.],   // A inner end, bone 0
            [10., 0., 88.], // A tip, across
            [0., 0., -88.], // B tip, bone 1
            [0., 0., -9.],  // B inner end, bone 1
            [0., 0., 0.],   // rod middle, bones 2/3 50/50
            [0., -5., 50.], // unweighted: follows A
        ];
        let joints = vec![
            [0, 0, 0, 0],
            [0; 4],
            [0; 4],
            [1, 0, 0, 0],
            [1, 0, 0, 0],
            [2, 3, 0, 0],
            [0; 4],
        ];
        let weights = vec![
            [1., 0., 0., 0.],
            [1., 0., 0., 0.],
            [1., 0., 0., 0.],
            [1., 0., 0., 0.],
            [1., 0., 0., 0.],
            [0.5, 0.5, 0., 0.],
            [0.; 4],
        ];
        let bolt: Vec<[f32; 3]> = vec![[0.; 3], [40., 0., 0.]];
        let mut scene = build_scene(
            &things,
            |s| match s {
                MeshSource::Guid(0x70) => Ok(piston.clone()),
                MeshSource::Guid(_) => Ok(bolt.clone()),
                _ => unreachable!(),
            },
            |p| points_bounds(p),
            |_| false,
            |p| {
                if p.len() == piston.len() {
                    skins_only(&piston_bind)
                } else {
                    skins_only(&[joint_bone(0.1, [0.; 3]), joint_bone(0.08, [0.; 3])])
                }
            },
            PosPlacement::InverseBind,
        );
        let stretched = scene.items[2].world;
        let bolt_world = scene.items[3].world;
        skin_span_joints(
            &mut scene,
            &things,
            |_| Ok((joints.clone(), weights.clone())),
            |p| Some(&p[..]),
            |p| {
                if p.len() == piston.len() {
                    skins_only(&piston_bind)
                } else {
                    Vec::new()
                }
            },
            |_, positions| positions,
        );
        let c = &scene.counts;
        assert_eq!(
            (c.joint_span_skinned, c.joint_skin_unweighted_vertices),
            (1, 1)
        );
        assert!(c.joint_span_skin_failed.is_empty());
        assert_eq!(scene.items[3].world, bolt_world);
        let item = &scene.items[2];
        assert_ne!(item.world, stretched);
        assert_eq!(scene.meshes.len(), 4); // 0x10, 0x70, 0x40 + one skinned copy
        assert_eq!(
            scene.meshes[item.mesh].source,
            MeshSource::JointSkin {
                mesh: Box::new(MeshSource::Guid(0x70)),
                thing: 2
            }
        );
        let world: Vec<[f64; 3]> = scene.meshes[item.mesh]
            .payload
            .iter()
            .map(|&p| apply(&item.world, p))
            .collect();
        let (pa, pb) = ([1., 2., 0.], [500., 0., 0.]);
        let dir = unit(sub(pb, pa)).unwrap();
        let along = |p: [f64; 3], from: [f64; 3]| dot(sub(p, from), dir);
        // Rigid ends: tips on the contacts, inner ends 79 units in, width kept.
        assert!(close(world[0], pa));
        assert!((along(world[1], pa) - 79.).abs() < 1e-3);
        assert!((dot(sub(world[2], world[0]), sub(world[2], world[0])).sqrt() - 10.).abs() < 1e-3);
        assert!(close(world[3], pb));
        assert!((along(world[4], pb) + 79.).abs() < 1e-3);
        // Rod middle: halfway between its A-group (88 in from A) and B-group positions.
        let span = along(pb, pa);
        assert!((along(world[5], pa) - span / 2.).abs() < 1e-3);
        // Unweighted vertex follows the A group: 38 in from A, 5 towards the camera.
        assert!((along(world[6], pa) - 38.).abs() < 1e-3);
        assert!((world[6][2] - 5.).abs() < 1e-3);
        assert!(c
            .lines()
            .iter()
            .any(|l| l.starts_with("Span joints skinned (D147") && l.contains(": 1 of 1 Things")));
        // A mesh whose bones all sit on one side keeps the D120 stretch.
        let bind = [joint_bone(1., [0., 0., 26.]), joint_bone(1., [0., 0., 25.])];
        assert_eq!(
            joint_span_skin(
                &things,
                things[2].data.joint.as_ref().unwrap(),
                &bind,
                Some([[-1., -1., -88.], [1., 1., 88.]])
            ),
            Err("span JOINT bones do not split into an A and a B group".into())
        );
    }

    #[test]
    fn matrix_checks_and_push_item_extension_point() {
        let mut m = identity_at(1.);
        assert!(world_matrix_check(&m).is_ok());
        m[3] = 0.5;
        assert!(world_matrix_check(&m)
            .unwrap_err()
            .starts_with("projective"));
        let mut scene = LevelScene::<u8>::default();
        assert!(scene
            .push_item(SceneItem {
                mesh: 0,
                world: identity_at(0.),
                thing: 0,
                uid: None,
                shapes: 0,
                decoration: false,
                alternate: false,
            })
            .is_err());
        scene.meshes.push(SceneMesh {
            source: MeshSource::Sha1("ab".into()),
            payload: 1,
            bounds: None,
            neutral_groups: false,
        });
        scene
            .push_item(SceneItem {
                mesh: 0,
                world: identity_at(2.),
                thing: 0,
                uid: None,
                shapes: 0,
                decoration: false,
                alternate: false,
            })
            .unwrap();
        assert_eq!(scene.bounds(), None);
        // Mirror (negative determinant) is invertible and accepted.
        let mut mirror = identity_at(0.);
        mirror[0] = -1.;
        assert!(world_matrix_check(&mirror).is_ok());
        assert_eq!(
            transform_point(&identity_at(2.), [1., 2., 3.]),
            [3., 2., 3.]
        );
    }

    fn shape_thing(index: usize, gfx: Option<u32>, visible: bool, x: f32) -> ThingRecord {
        let square = [[0., 0.], [100., 0.], [100., 100.], [0., 100.]];
        let d = |g| Descriptor {
            flags: Some(0),
            kind_byte: 2,
            guid: Some(g),
            sha1: None,
        };
        ThingRecord {
            index,
            data: PartData {
                pos: Some(Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world: identity_at(x),
                }),
                shape: Some(Shape {
                    vertex_count: 4,
                    requires_z: false,
                    vertices: square.iter().map(|p: &[f32; 2]| [p[0], p[1], 0.]).collect(),
                    loops: vec![4],
                    material: None,
                    old_material: None,
                    thickness: 10.,
                    mass_depth: 1.,
                    color: ShapeColor::Packed(0xffff_ffff),
                    brightness: 0.,
                    bevel_size: 0.,
                    com: None,
                    behavior: 0,
                    flags: 0,
                    contacts: 0,
                }),
                generated_mesh: Some(GeneratedMesh {
                    gfx_material: gfx.map(d),
                    bevel: None,
                    uv_offset: [0.; 4],
                    plan_guid: None,
                    play_mode_visible: visible,
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn colliders_without_generated_mesh_split_by_parent_tree() {
        // D153: 0 = mesh root; 1 = shape with its own RENDER_MESH; 2 = collider parented to
        // 0; 3 = root collider; 4/5 = colliders in a parent cycle (bounded walk).
        let collider = |index: usize, parent: Option<usize>| {
            let mut t = shape_thing(index, Some(7), true, 0.);
            t.data.generated_mesh = None;
            t.parent = parent;
            t
        };
        let mut own = collider(1, None);
        own.data.render_mesh = thing(1, Some(2), None).data.render_mesh;
        let things = vec![
            thing(0, Some(1), Some(identity_at(0.))),
            own,
            collider(2, Some(0)),
            collider(3, None),
            collider(4, Some(5)),
            collider(5, Some(4)),
        ];
        let mut scene = LevelScene::<ShapeBatch>::default();
        add_shapes(
            &mut scene,
            &things,
            |b| (b, Ok(())),
            |b| points_bounds(&b.positions),
        );
        let n = &scene.counts.shapes_not_selected;
        assert_eq!(n["no_generated_mesh: collision hull of own RENDER_MESH"], 1);
        assert_eq!(
            n["no_generated_mesh: collider in a RENDER_MESH parent tree"],
            1
        );
        assert_eq!(
            n["no_generated_mesh: invisible collider, no mesh in parent tree"],
            3
        );
        assert_eq!(scene.counts.shapes_extruded, 0);
    }

    #[test]
    fn sticker_decals_project_per_texture_and_count_reasons() {
        // D160: two decals (textures 0x11, 0x22) on a shape, one off its face, one on an
        // edit-only shape, one on a logic Thing, plus two costume decals.
        let decal = |tex: u32, u: f32| lbp_level::parts::Decal {
            texture: Some(Descriptor {
                flags: Some(0),
                kind_byte: 2,
                guid: Some(tex),
                sha1: None,
            }),
            u,
            v: 0.9,
            xvec: [0.05, 0.],
            yvec: [0., 0.05],
            color: 0,
            kind: 0,
            metadata_index: 0,
            num_metadata: None,
            placed_by: 0,
            play_mode_frame: 0,
            scorch_mark: false,
            plan: None,
        };
        let mut owner = shape_thing(0, Some(7), true, 0.);
        owner.data.stickers = vec![decal(0x11, 0.1), decal(0x22, 0.1), decal(0x11, 5.)];
        owner.data.sticker_decals = 5;
        let mut hidden = shape_thing(1, Some(7), false, 0.);
        hidden.data.stickers = vec![decal(0x11, 0.1)];
        hidden.data.sticker_decals = 1;
        let mut logic = thing(2, None, Some(identity_at(0.)));
        logic.data.render_mesh = None;
        logic.data.stickers = vec![decal(0x11, 0.1)];
        logic.data.sticker_decals = 1;
        let mut scene = LevelScene::<DecalBatch>::default();
        scene.counts.sticker_decals_total = 7;
        add_sticker_decals(
            &mut scene,
            &[owner, hidden, logic],
            |b| {
                if b.texture.0 == Some(0x22) {
                    Err("Unresolved: GUID".into())
                } else {
                    Ok(b.clone())
                }
            },
            |b| points_bounds(&b.positions),
        );
        let c = &scene.counts;
        assert_eq!(c.sticker_decals_drawn, 1);
        assert_eq!(scene.meshes.len(), 1);
        let b = &scene.meshes[0].payload;
        assert_eq!((b.decals, b.texture.0), (1, Some(0x11)));
        // Square 0..100 at scale 1: u 0.1 -> x 50, v 0.9 -> y 50; front face z = 10 + lift.
        assert!(b
            .positions
            .iter()
            .all(|p| (p[0] - 50.).abs() <= 25.01 && (p[1] - 50.).abs() <= 25.01 && p[2] > 10.));
        let skipped = |k: &str| c.sticker_decals_skipped.get(k).copied().unwrap_or(0);
        assert_eq!(skipped("decal texture: Unresolved: GUID"), 1);
        assert_eq!(skipped("decal: outside the owner's front face"), 1);
        assert_eq!(
            skipped("costume decal array (costume pieces not projected)"),
            2
        );
        assert_eq!(
            skipped("owner has no surface (logic / microchip / switch Thing)"),
            1
        );
        assert_eq!(c.sticker_decals_skipped.values().sum::<usize>() + 1, 7);
        assert!(c
            .lines()
            .iter()
            .any(|l| l.starts_with("Sticker decals drawn: 1 / 7")));
    }

    #[test]
    fn shapes_split_per_shape_color_d194() {
        let mut grey = shape_thing(1, Some(7), true, 500.);
        grey.data.shape.as_mut().unwrap().color = ShapeColor::Packed(0xff4d_4d4d);
        let things = vec![
            shape_thing(0, Some(7), true, 0.),
            grey,
            shape_thing(2, Some(7), true, 1000.),
        ];
        let mut scene = LevelScene::<ShapeBatch>::default();
        add_shapes(
            &mut scene,
            &things,
            |b| (b, Ok(())),
            |b| points_bounds(&b.positions),
        );
        assert_eq!(scene.counts.shape_batches, 2);
        // Colour order within one material: 0xff4d4d4d before white.
        let colors: Vec<_> = scene
            .meshes
            .iter()
            .map(|m| (m.payload.color, m.payload.things.clone()))
            .collect();
        assert_eq!(
            colors,
            vec![(0xff4d_4d4d, vec![1]), (WHITE_ARGB, vec![0, 2])]
        );
        assert_eq!(
            scene.meshes[0].source.label(),
            "shapes gfx 00000007 colour ff4d4d4d batch 0"
        );
        assert_eq!(
            scene.meshes[1].source.label(),
            "shapes gfx 00000007 batch 0"
        );
    }

    #[test]
    fn shape_color_argb_packs_and_clamps_vectors() {
        assert_eq!(
            shape_color_argb(&ShapeColor::Packed(0x8011_2233)),
            0x8011_2233
        );
        assert_eq!(
            shape_color_argb(&ShapeColor::Vector([1., 0.5, -1., f32::NAN])),
            0xffff_8000
        );
        let tint = editor_color_tint(0xff4d_4d4d);
        assert_eq!(f32::from_bits(tint.bits[0]), 0x4d as f32 / 255.);
        assert_eq!(f32::from_bits(tint.bits[3]), 1.);
    }

    #[test]
    fn shapes_merge_per_material_and_count_outcomes() {
        let mut collider = shape_thing(4, Some(7), true, 0.);
        collider.data.generated_mesh = None;
        let mut bad = shape_thing(5, Some(7), true, 0.);
        bad.data.shape.as_mut().unwrap().loops = vec![3];
        let things = vec![
            shape_thing(0, Some(7), true, 0.),
            shape_thing(1, Some(9), true, 500.),
            shape_thing(2, Some(7), true, 1000.),
            shape_thing(3, Some(7), false, 0.),
            collider,
            bad,
        ];
        let mut scene = LevelScene::<ShapeBatch>::default();
        add_shapes(
            &mut scene,
            &things,
            |b| {
                let ok = if b.material == Some(9) {
                    Err("Unresolved: GUID".to_string())
                } else {
                    Ok(())
                };
                (b, ok)
            },
            |b| points_bounds(&b.positions),
        );
        let c = &scene.counts;
        assert_eq!(
            (c.shapes_total, c.shapes_extruded, c.shapes_placed),
            (6, 3, 3)
        );
        assert_eq!(c.shape_batches, 2);
        assert_eq!(c.shapes_not_selected["edit_only"], 1);
        assert_eq!(
            c.shapes_not_selected["no_generated_mesh: invisible collider, no mesh in parent tree"],
            1
        );
        assert_eq!(c.shapes_rejected["loop_sum_mismatch"], 1);
        assert_eq!(c.shapes_material_fallback["Unresolved: GUID"], 1);
        // Material 7 first (BTreeMap order): two shapes merged, indices rebased.
        let b = &scene.meshes[0].payload;
        assert_eq!(b.things, vec![0, 2]);
        let half = b.positions.len() / 2;
        assert!(b.triangles[b.triangles.len() / 2..]
            .iter()
            .flatten()
            .all(|&i| i as usize >= half));
        assert_eq!(scene.items[0].shapes, 2);
        assert!(scene.meshes[1].neutral_groups);
        let [lo, hi] = scene.bounds().unwrap();
        assert_eq!((lo[0], hi[0]), (0., 1100.));
        assert!(c
            .lines()
            .iter()
            .any(|l| l.contains(shape_mesh::PREVIEW_LABEL)));
    }

    #[test]
    fn bevelled_shapes_split_by_slot_material_and_count_once() {
        use lbp_level::bevel::BevelVertex;
        let d = |g| Descriptor {
            flags: Some(0),
            kind_byte: 2,
            guid: Some(g),
            sha1: None,
        };
        let v = |y, z, slot| BevelVertex {
            y,
            z,
            rigidity: 1.,
            smooth_with_previous: 1,
            gmat_slot: slot,
            mapping_mode_raw: 1,
        };
        let bevel = Bevel {
            revision: lbp_level::Revision {
                head: 0x3f8,
                branch_id: 0,
                branch_revision: 0,
                compression: 7,
            },
            materials: [None, Some(d(11)), None, None],
            uv_scales: [1.; 4],
            auto_smooth_cutoff_angle: 45.,
            vertices: vec![v(0., -1., 1), v(0., 0.5, 1), v(-0.5, 1., 0)],
            include_backface: false,
            smooth_with_front: true,
            relax_strength: 0.,
            sub_div_radius: 0.3,
            fixed_bevel_size: 10.,
            spongy: false,
            soft_physics_settings: None,
            texture_repeats: 1.,
            consumed_bytes: 0,
            trailing_bytes: 0,
        };
        let mut a = shape_thing(0, Some(7), true, 0.);
        a.data.generated_mesh.as_mut().unwrap().bevel = Some(d(5));
        let mut missing = shape_thing(1, Some(7), true, 500.);
        missing.data.generated_mesh.as_mut().unwrap().bevel = Some(d(6));
        let plain = shape_thing(2, Some(7), true, 1000.);
        let mut table = BevelTable::new();
        table.insert((Some(5), None), bevel);
        let things = vec![a, missing, plain];
        let run = |bevels: Option<&BevelTable>| {
            let mut scene = LevelScene::<ShapeBatch>::default();
            add_shapes_with(
                &mut scene,
                &things,
                bevels,
                |b| (b, Ok(())),
                |b| points_bounds(&b.positions),
            );
            scene
        };
        let scene = run(Some(&table));
        let c = &scene.counts;
        assert_eq!((c.shapes_extruded, c.shapes_placed), (3, 3));
        assert_eq!(c.shape_bevel_outcomes["bevelled"], 1);
        assert_eq!(c.shape_bevel_outcomes["bevel_unavailable"], 1);
        assert_eq!(c.shape_bevel_outcomes["no_bevel"], 1);
        // Material 7 (front cap + both straight shapes) and slot-1 material 11 (sides, back).
        assert_eq!(c.shape_batches, 2);
        assert_eq!(scene.meshes[0].payload.things, vec![0, 1, 2]);
        assert_eq!(scene.meshes[1].payload.material, Some(11));
        assert_eq!(scene.meshes[1].payload.things, vec![0]);
        assert_eq!(scene.items.iter().map(|i| i.shapes).sum::<usize>(), 3);
        assert!(c.lines().iter().any(|l| l.contains(SHAPE_BEVEL_LABEL)));
        // Option off: the D114 scene, no bevel lines.
        let plain_scene = run(None);
        assert_eq!(plain_scene.counts.shape_batches, 1);
        assert!(plain_scene.counts.shape_bevel_outcomes.is_empty());
        assert!(!plain_scene
            .counts
            .lines()
            .iter()
            .any(|l| l.contains(SHAPE_BEVEL_LABEL)));
    }

    #[test]
    fn checkpoints_need_the_part_and_a_valid_pos() {
        let thing = |index, parts: Vec<&'static str>, world: Option<[f32; 16]>| ThingRecord {
            index,
            parts,
            data: PartData {
                pos: world.map(|world| Pos {
                    bone_of: None,
                    anim_hash: 0,
                    world,
                }),
                ..PartData::default()
            },
            ..ThingRecord::default()
        };
        let mut singular = identity_at(0.);
        singular[0] = 0.;
        let things = [
            thing(0, vec!["POS"], Some(identity_at(1.))),
            thing(1, vec!["POS", "CHECKPOINT"], Some(identity_at(2.))),
            thing(2, vec!["CHECKPOINT"], None),
            thing(3, vec!["POS", "CHECKPOINT"], Some(singular)),
            thing(4, vec!["POS", "CHECKPOINT"], Some(identity_at(5.))),
        ];
        let found = checkpoints(&things);
        assert_eq!(
            found.iter().map(|c| (c.0, c.1[12])).collect::<Vec<_>>(),
            [(1, 2.), (4, 5.)]
        );
    }

    #[test]
    fn d191_editor_color_variants_only_for_instance_meshes() {
        let colored = |i, mesh, color| {
            let mut t = thing(i, Some(mesh), Some(identity_at(i as f32)));
            t.data.render_mesh.as_mut().unwrap().editor_color = color;
            t
        };
        let things = [
            colored(0, 0x10, EDITOR_COLOR_WHITE),
            colored(1, 0x10, 0xffff_0000),
            colored(2, 0x10, 0xffff_0000),
            colored(3, 0x10, 0x8000_00ff),
            colored(4, 0x20, 0xffff_0000), // colour-blind mesh: stays shared
        ];
        let mut calls = Vec::new();
        let scene = build_scene_with_colors(
            &things,
            |s, c| {
                calls.push((s.clone(), c));
                match s {
                    MeshSource::Guid(g) => Ok((*g, c)),
                    _ => unreachable!(),
                }
            },
            |_| None,
            |_| false,
            |_| Vec::new(),
            |&(g, _)| g == 0x10,
            PosPlacement::PosOnly,
        );
        assert_eq!(
            calls,
            [
                (MeshSource::Guid(0x10), None),
                (MeshSource::Guid(0x10), Some(0xffff_0000)),
                (MeshSource::Guid(0x10), Some(0x8000_00ff)),
                (MeshSource::Guid(0x20), None),
            ]
        );
        let payloads: Vec<_> = scene
            .items
            .iter()
            .map(|i| scene.meshes[i.mesh].payload)
            .collect();
        assert_eq!(
            payloads,
            [
                (0x10, None),
                (0x10, Some(0xffff_0000)),
                (0x10, Some(0xffff_0000)),
                (0x10, Some(0x8000_00ff)),
                (0x20, None),
            ]
        );
        let c = &scene.counts;
        assert_eq!((c.drawn_editor_colored, c.editor_color_variants), (3, 2));
        assert_eq!(c.meshes_loaded, 4);
        // The plain build_scene ignores colours (one load per source).
        let plain = build_scene(
            &things,
            |s| match s {
                MeshSource::Guid(g) => Ok(*g),
                _ => unreachable!(),
            },
            |_| None,
            |_| false,
            |_| Vec::new(),
            PosPlacement::PosOnly,
        );
        assert_eq!(
            (
                plain.counts.meshes_loaded,
                plain.counts.drawn_editor_colored
            ),
            (2, 0)
        );
    }

    #[test]
    fn d191_editor_color_is_packed_argb() {
        assert_eq!(editor_color_tint(EDITOR_COLOR_WHITE).values(), [1.; 4]);
        assert_eq!(
            editor_color_tint(0x80ff_0000).values(),
            [1., 0., 0., 128. / 255.]
        );
        assert_eq!(editor_color_tint(0xff00_ff00).values(), [0., 1., 0., 1.]);
        assert_eq!(editor_color_tint(0xff00_00ff).values(), [0., 0., 1., 1.]);
        assert!(editor_color_tint(0x1234_5678).validate().is_ok());
    }

    /// D308: an infinite checkpoint's ring is white (x = 1, alpha 0.5) whatever its counts,
    /// and every checkpoint's RGB is white in normal play.
    #[test]
    fn d308_infinite_checkpoint_ring_white() {
        use lbp_level::parts::Checkpoint;
        let c = |left, max, inf| Checkpoint {
            spawns_left: left,
            max_spawns_left: max,
            infinite_spawns: inf,
        };
        assert_eq!(checkpoint_light_alpha(&c(-1, 0, true)), 0.5);
        assert_eq!(checkpoint_light_alpha(&c(0, 3, true)), 0.5);
        assert_eq!(checkpoint_light_alpha(&c(-1, 0, false)), 1.);
        assert_eq!(
            checkpoint_editor_color(0xff00_0000, Some(&c(-1, 0, true))),
            0x7fff_ffff
        );
    }

    /// D307: a checkpoint's ring alpha, `1 - 0.5 x` with `x` the share of spawns left:
    /// full 0.5 (byte 127, the white half of the lifebar texture), empty or without a
    /// count 1 (the black half); the colour's RGB stays the stored editorColor.
    #[test]
    fn d307_checkpoint_light_alpha() {
        use lbp_level::parts::Checkpoint;
        let c = |left, max| Checkpoint {
            spawns_left: left,
            max_spawns_left: max,
            infinite_spawns: false,
        };
        assert_eq!(checkpoint_light_alpha(&c(3, 3)), 0.5);
        assert_eq!(checkpoint_light_alpha(&c(1, 2)), 0.75);
        assert_eq!(checkpoint_light_alpha(&c(0, 2)), 1.);
        assert_eq!(checkpoint_light_alpha(&c(4, 0)), 1.);
        assert_eq!(checkpoint_light_alpha(&c(5, 2)), 0.5);
        assert_eq!(
            checkpoint_editor_color(0xffff_ffff, Some(&c(1, 1))),
            0x7fff_ffff
        );
        assert_eq!(
            checkpoint_editor_color(0xff12_3456, Some(&c(0, 1))),
            0xffff_ffff
        );
        assert_eq!(checkpoint_editor_color(0x8012_3456, None), 0x8012_3456);
        // The ring's UV1 u lies in 0.0017..0.4921 (mesh 00007a06, primitive 4); the
        // texture's left half is black, its right half white. Minus alpha 127/255 every
        // u lands in the white half; minus 1 every u stays in the black half.
        // The light goes to the checkpoint Thing's parent (the mesh Thing).
        let things = [
            ThingRecord {
                index: 0,
                ..Default::default()
            },
            ThingRecord {
                index: 1,
                parent: Some(0),
                data: PartData {
                    checkpoint: Some(c(3, 3)),
                    ..Default::default()
                },
                ..Default::default()
            },
        ];
        let lights = checkpoint_lights(&things);
        assert_eq!(lights.len(), 1);
        assert_eq!(checkpoint_light_alpha(lights[&0]), 0.5);
        let a = (checkpoint_editor_color(0xffff_ffff, Some(&c(1, 1))) >> 24) as f32 / 255.;
        for u in [0.0017f32, 0.25, 0.4921] {
            assert!((u - a).rem_euclid(1.) >= 0.5, "{u}");
            assert!((u - 1.).rem_euclid(1.) < 0.5, "{u}");
        }
    }
}
