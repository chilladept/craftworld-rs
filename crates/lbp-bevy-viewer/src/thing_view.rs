//! D268 (docs/level-scripts.md, part 19): the played level's Things as draw entities, so
//! that what the level's scripts hide, tint and move shows in the scene.
//!
//! [`LevelThings`] is the Thing-to-entity map, built when the level scene is spawned:
//! every draw entity of a Thing's render mesh or of one of its decorations carries a
//! [`LevelThing`]. [`apply`] takes the Things the script world lists as changed and sets
//! each entity's visibility, tint and place from the world's values.
//!
//! Followed: the render mesh's visibility, its tint, and the Thing's world matrix (the
//! mesh and the Thing's decorations). Not followed, and counted: a Thing that has no
//! draw entity of its own (a shape, which is merged with others into a shared mesh; a
//! Thing with no mesh), a new editor colour of a mesh or colour of a shape, which the
//! scene builds into the material when the level is loaded, and (D272) another mesh on a
//! Thing's PRenderMesh, which would need that mesh loaded while the scene is up.
//!
//! D276 (part 27): the meshes of [`SWAPPED_MESHES`] are loaded with the scene. A Thing
//! drawn with one of a pair has a second, hidden set of draw entities for the other
//! (`Item::alternate`), and [`apply`] shows the set of the mesh the PRenderMesh has now.
//!
//! D278 (part 29): in a map with a [`LevelThings::dissolve_mesh`] (the Pod's shell
//! layer) the pod Thing's entities of that mesh are drawn with a top-down dissolve while
//! the alpha of the Thing's shape colour is between 0 and 1. [`apply`] keeps the value
//! on the tag ([`LevelThing::dissolve`]) and gives the entity a material of its own;
//! `level_play::follow_dissolve` writes the line into that material each frame.
use crate::dual::{self, ViewerMaterial};
use bevy::prelude::*;
use lbp_runtime::{pack_colour, DisplayChange, LevelWorld, WHITE};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// A scene item that draws a Thing's own model: found when the level is planned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Item {
    /// The Thing's index in the level (the script world's Thing handle).
    pub thing: u32,
    /// A decoration of the Thing, not its render mesh.
    pub decoration: bool,
    /// The item's matrix in level units, as the scene placed it from the level file.
    pub world: Mat4,
    /// GUID of the mesh the item draws (none for a mesh named by its hash only).
    pub mesh: Option<u32>,
    /// D276: the item draws the other mesh of a [`SWAPPED_MESHES`] pair, not the one the
    /// level file names: hidden until a script sets that mesh on the Thing.
    pub alternate: bool,
}

/// D276: pairs of mesh GUIDs a script swaps on a Thing with `Thing.SetRenderMeshMesh`.
/// FOUND: the one call site in the 426 scripts is `Pod.SetPodTransition__b`, which puts
/// `pod.mol` (0x2f6c) on the pod Thing while the change-over is under 1 and
/// `pod_platform.mol` (0x23959) at 1; `Thing.GetPodThing__b` tests for the same two
/// (docs/level-scripts.md, parts 21 and 27). Any other mesh a script sets is not loaded.
pub const SWAPPED_MESHES: [[u32; 2]; 1] = [[0x2f6c, 0x0002_3959]];

/// On a draw entity of the played level: the Thing it shows.
#[derive(Component)]
pub struct LevelThing {
    pub item: Item,
    /// The entity's row in the viewer's scene list (the list the lighting walks).
    pub scene_row: usize,
    /// The entity has a material of its own (made for its first tint or dissolve).
    pub own_material: bool,
    /// D278: the entity is drawn with the dissolve, at this value and flat colour.
    pub dissolve: Option<Dissolving>,
    /// D284: GUID of the material of the entity's draw bucket (none for a neutral one).
    pub material: Option<u32>,
}

/// D278: the dissolve of one draw entity: the pod's change-over value, strictly between
/// 0 and 1, and the red, green and blue of the pod Thing's shape colour as stored. D284:
/// or 1, on the entities of [`LevelThings::dissolve_edge`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dissolving {
    pub value: f32,
    pub colour: [f32; 3],
}

/// The Thing-to-entity map of the level scene.
#[derive(Resource)]
pub struct LevelThings {
    /// Level units to display units.
    pub normalize: Mat4,
    pub entities: HashMap<u32, Vec<Entity>>,
    /// A number no other scene has had.
    pub scene: u64,
    /// D278: the mesh GUID whose entities of the level's pod Thing are drawn with the
    /// dissolve; none (the default) for a level without it. The viewer sets it on the
    /// Pod shell layer's map only.
    pub dissolve_mesh: Option<u32>,
    /// D284: the mesh and the material GUIDs whose entities of the pod Thing keep the
    /// dissolve at value 1 (the platform's card edge); none for a level without it.
    pub dissolve_edge: Option<(u32, u32)>,
}

static SCENES: AtomicU64 = AtomicU64::new(1);

impl LevelThings {
    pub fn new(normalize: Mat4) -> Self {
        Self {
            normalize,
            entities: HashMap::new(),
            scene: SCENES.fetch_add(1, Ordering::Relaxed),
            dissolve_mesh: None,
            dissolve_edge: None,
        }
    }

    pub fn add(&mut self, thing: u32, entity: Entity) {
        self.entities.entry(thing).or_default().push(entity);
    }

    /// Things with a draw entity, and the draw entities.
    pub fn counts(&self) -> (usize, usize) {
        (
            self.entities.len(),
            self.entities.values().map(Vec::len).sum(),
        )
    }
}

/// D277 (part 28): the Thing-to-entity map of the Pod's shell layer, when the shell
/// level has a script world of its own. A resource type of its own beside the played
/// level's [`LevelThings`]: [`apply`] reaches entities through the map it is given, so
/// the same Thing number in both levels does not collide and the tag needs no level.
#[derive(Resource)]
pub struct ShellThings(pub LevelThings);

/// What [`apply`] did. Units: Thing, except `entities`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Followed {
    /// Things on the list, and those of them with a draw entity.
    pub listed: usize,
    pub drawn: usize,
    /// Render meshes hidden and shown again.
    pub hidden: usize,
    pub shown: usize,
    /// Render meshes given a tint, and given back their own colour.
    pub tinted: usize,
    pub untinted: usize,
    /// Things put at a new place.
    pub placed: usize,
    /// Draw entities touched.
    pub entities: usize,
    /// Things with a change and no draw entity (a shape in a shared mesh, no mesh).
    pub not_drawn: usize,
    /// Changes of a mesh's editor colour or of a shape's colour or tint, not drawn.
    pub colour_not_drawn: usize,
    /// Moved Things whose stored matrix has no inverse: left where they were.
    pub not_placed: usize,
    /// Tints that change only the alpha (red, green and blue stay white): not drawn.
    pub tint_alpha_not_drawn: usize,
    /// Things whose PRenderMesh shows another mesh than the level file's, one that was
    /// not loaded with the scene: the scene keeps the file's mesh.
    pub mesh_not_drawn: usize,
    /// D276: Things whose draw entities were changed over to the mesh the PRenderMesh
    /// has now (to the other mesh of a pair, or back to the file's).
    pub mesh_swapped: usize,
    /// D278: Things whose dissolve was set to a new value or taken off.
    pub dissolved: usize,
}

impl Followed {
    pub fn absorb(&mut self, o: &Self) {
        self.listed += o.listed;
        self.drawn += o.drawn;
        self.hidden += o.hidden;
        self.shown += o.shown;
        self.tinted += o.tinted;
        self.untinted += o.untinted;
        self.placed += o.placed;
        self.entities += o.entities;
        self.not_drawn += o.not_drawn;
        self.colour_not_drawn += o.colour_not_drawn;
        self.not_placed += o.not_placed;
        self.tint_alpha_not_drawn += o.tint_alpha_not_drawn;
        self.mesh_not_drawn += o.mesh_not_drawn;
        self.mesh_swapped += o.mesh_swapped;
        self.dissolved += o.dissolved;
    }

    /// Whether anything in the scene was changed.
    pub fn any(&self) -> bool {
        self.hidden
            + self.shown
            + self.tinted
            + self.untinted
            + self.placed
            + self.mesh_swapped
            + self.dissolved
            > 0
    }

    pub fn line(&self) -> String {
        format!(
            "listed {}, with a draw entity {}; hidden {}, shown {}, tinted {}, tint taken off {}, placed {}, mesh swapped {}, dissolve set {} ({} entities); not followed: no draw entity {}, editor or shape colour {}, tint of alpha only {}, no placement {}, another mesh {}",
            self.listed,
            self.drawn,
            self.hidden,
            self.shown,
            self.tinted,
            self.untinted,
            self.placed,
            self.mesh_swapped,
            self.dissolved,
            self.entities,
            self.not_drawn,
            self.colour_not_drawn,
            self.tint_alpha_not_drawn,
            self.not_placed,
            self.mesh_not_drawn
        )
    }
}

/// The list for a scene that was just spawned: every mapped Thing whose values are not
/// the level file's (scripts ran while the scene loaded), and every mapped Thing whose
/// render mesh is hidden, in the file or by a script. The scene itself is built from the
/// level file and draws every mesh.
pub fn first_list(world: &LevelWorld, things: &LevelThings) -> Vec<(u32, DisplayChange)> {
    let mut list: Vec<(u32, DisplayChange)> = things
        .entities
        .keys()
        .filter_map(|&thing| {
            let now = world.thing_display(thing)?;
            let mut change = now.unlike(&world.display_at_load(thing)?);
            change.visibility |= now.mesh_hidden();
            change.any().then_some((thing, change))
        })
        .collect();
    list.sort_by_key(|(thing, _)| *thing);
    list
}

pub type Parts<'a> = (
    &'a mut LevelThing,
    &'a mut Transform,
    &'a mut Visibility,
    &'a mut MeshMaterial3d<ViewerMaterial>,
);

/// Sets the draw entities of the listed Things from the script world's values.
///
/// - Visibility: the render mesh's entities are hidden while both hide bits of its
///   visibility byte are clear (`ThingDisplay::mesh_hidden`). Decorations stay.
/// - Tint: the render mesh's entities get a material of their own with the tint's red,
///   green and blue as a multiplier (INFERRED: the game's use of the tint was not read).
///   The tint's alpha is not used, so a white tint with an alpha draws as no tint.
///   The new material takes the place of the shared one in `scene`, so lighting and
///   clean-up reach it.
/// - Place: every entity of the Thing is moved by the matrix that takes the Thing from
///   its stored world matrix to the one it has now.
/// - Mesh (D276): a Thing with a second set of entities for the other mesh of a
///   [`SWAPPED_MESHES`] pair shows the set of the mesh its PRenderMesh has now and hides
///   the other; the visibility rule above still hides the set that is shown, whichever
///   of the two changes comes first. A mesh that was not loaded with the scene leaves
///   the file's mesh in the scene, and is counted.
/// - Dissolve (D278): in a map with a [`LevelThings::dissolve_mesh`], the pod Thing's
///   entities of that mesh (not its decorations, not the entities of another mesh) get
///   the change-over value the alpha of its shape colour stands for
///   (`pod_dissolve::value`) and the colour's red, green and blue on their tag, and a
///   material of their own on the first value. At alpha 1 nothing is touched. The shape
///   colour's change is then not counted as "not drawn". D284: at alpha 0 the entities
///   of [`LevelThings::dissolve_edge`] (the platform's card edge) get value 1 the same
///   way, and lose it at any other alpha.
pub fn apply(
    world: &LevelWorld,
    list: &[(u32, DisplayChange)],
    things: &LevelThings,
    entities: &mut Query<Parts>,
    materials: &mut Assets<ViewerMaterial>,
    scene: &mut [crate::OwnedScene],
) -> Followed {
    let mut done = Followed {
        listed: list.len(),
        ..Default::default()
    };
    for &(thing, change) in list {
        let (Some(now), Some(was)) = (world.thing_display(thing), world.display_at_load(thing))
        else {
            continue;
        };
        // What the scene cannot show, with or without a draw entity.
        let packed = |c: Option<[f32; 4]>| c.map(pack_colour);
        let tint_of = |c: Option<[f32; 4]>| pack_colour(c.unwrap_or(WHITE));
        let other_colour = packed(now.mesh_colour) != packed(was.mesh_colour)
            || packed(now.shape_colour) != packed(was.shape_colour)
            || tint_of(now.shape_tint) != tint_of(was.shape_tint);
        let list = things.entities.get(&thing).filter(|l| !l.is_empty());
        // D276: the Thing has a second set of entities for another mesh, and the mesh
        // it has now is that set's.
        let sets = |keep: &dyn Fn(&Item) -> bool| {
            list.is_some_and(|l| {
                l.iter()
                    .any(|&e| entities.get(e).is_ok_and(|(tag, ..)| keep(&tag.item)))
            })
        };
        // D278: the pod Thing of a map with the dissolve, with entities of that mesh.
        let dissolves = |item: &Item| {
            things.dissolve_mesh.is_some() && !item.alternate && item.mesh == things.dissolve_mesh
        };
        let pod = things.dissolve_mesh.is_some()
            && world.pod_thing(true) == Some(thing)
            && sets(&|item| !item.decoration && dissolves(item));
        let dissolving_by = |value: fn(f32) -> Option<f32>| {
            now.shape_colour.filter(|_| pod).and_then(|[r, g, b, a]| {
                Some(Dissolving {
                    value: value(a)?,
                    colour: [r, g, b],
                })
            })
        };
        let dissolving = dissolving_by(crate::pod_dissolve::value);
        // D284: the platform's card edge keeps the rule at value 1.
        let at_one = dissolving_by(crate::pod_dissolve::value_at_one);
        let edge_of = |tag: &LevelThing| {
            things.dissolve_edge.is_some_and(|(mesh, material)| {
                tag.item.mesh == Some(mesh) && tag.material == Some(material)
            })
        };
        // A shape colour that is drawn as the dissolve is not "not drawn".
        let only_shape_colour = packed(now.mesh_colour) == packed(was.mesh_colour)
            && tint_of(now.shape_tint) == tint_of(was.shape_tint);
        let as_dissolve = pod && only_shape_colour;
        done.colour_not_drawn += usize::from(change.colour && other_colour && !as_dissolve);
        let other_mesh = now.mesh != was.mesh;
        let alternates = sets(&|item| item.alternate);
        let preloaded = other_mesh && sets(&|item| item.alternate && item.mesh == now.mesh);
        done.mesh_not_drawn += usize::from(change.mesh && other_mesh && !preloaded);
        // The entities change sets: to the other mesh, or back to the file's.
        let swap = change.mesh && alternates && (preloaded || !other_mesh);
        done.mesh_swapped += usize::from(swap);
        let Some(list) = list else {
            done.not_drawn += 1;
            continue;
        };
        done.drawn += 1;
        let moved = change.transform.then(|| now.moved_from(&was));
        let opaque = |t: [f32; 4]| pack_colour([t[0], t[1], t[2], 1.]);
        let tint = now
            .mesh_tint
            .filter(|t| opaque(*t) != pack_colour(WHITE))
            .map(|[r, g, b, _]| [r, g, b]);
        let alpha_only = tint.is_none() && tint_of(now.mesh_tint) != pack_colour(WHITE);
        done.tint_alpha_not_drawn += usize::from(change.colour && alpha_only);
        let (mut hid, mut showed, mut tinted, mut untinted, mut placed) =
            (false, false, false, false, false);
        let mut dissolved = false;
        for &entity in list {
            let Ok((mut tag, mut transform, mut visibility, mut material)) =
                entities.get_mut(entity)
            else {
                continue;
            };
            done.entities += 1;
            if let Some(Some(by)) = moved {
                let matrix = things.normalize * Mat4::from_cols_array(&by) * tag.item.world;
                *transform = Transform::from_matrix(matrix);
                placed = true;
            }
            if tag.item.decoration {
                continue;
            }
            if change.visibility || (change.mesh && alternates) {
                // The set of the mesh the Thing has now; a mesh that was not loaded
                // leaves the file's set in the scene.
                let in_set = if preloaded {
                    tag.item.alternate && tag.item.mesh == now.mesh
                } else {
                    !tag.item.alternate
                };
                let want = if in_set && !now.mesh_hidden() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != want {
                    // A change of sets is counted as the swap, not as hidden or shown.
                    let sets_change = change.mesh && alternates;
                    hid |= !sets_change && want == Visibility::Hidden;
                    showed |= !sets_change && want == Visibility::Inherited;
                    *visibility = want;
                }
            }
            let wanted = if dissolves(&tag.item) {
                Some(dissolving)
            } else if edge_of(&tag) {
                Some(at_one)
            } else {
                None
            };
            if let Some(wanted) = wanted.filter(|w| pod && tag.dissolve != *w) {
                // The line itself is written each frame (`level_play::follow_dissolve`);
                // taken off, it is cleared here.
                let ready = match wanted {
                    Some(_) => own_material(&mut tag, &mut material, materials, scene),
                    None => {
                        if let Some(mut m) = materials.get_mut(material.0.id()) {
                            dual::set_dissolve(&mut m, None);
                        }
                        true
                    }
                };
                if ready {
                    tag.dissolve = wanted;
                    dissolved = true;
                }
            }
            // A material of the entity's own may be there for a dissolve alone (D278):
            // a tint is taken off only where one is on.
            let has_tint =
                tag.own_material && materials.get(material.0.id()).is_some_and(dual::tinted);
            if change.colour && (tint.is_some() || has_tint) {
                if !own_material(&mut tag, &mut material, materials, scene) {
                    continue;
                }
                if let Some(mut m) = materials.get_mut(material.0.id()) {
                    dual::set_thing_tint(&mut m, tint);
                }
                tinted |= tint.is_some();
                untinted |= tint.is_none();
            }
        }
        done.hidden += usize::from(hid);
        done.shown += usize::from(showed);
        done.tinted += usize::from(tinted);
        done.untinted += usize::from(untinted);
        done.placed += usize::from(placed);
        done.dissolved += usize::from(dissolved);
        done.not_placed += usize::from(moved == Some(None));
    }
    done
}

/// Gives a draw entity a material of its own, a copy of the shared one, on its first
/// tint or dissolve: the copy takes the shared one's place in the entity's scene row, so
/// lighting and clean-up reach it. False when the shared material is not there.
fn own_material(
    tag: &mut LevelThing,
    material: &mut MeshMaterial3d<ViewerMaterial>,
    materials: &mut Assets<ViewerMaterial>,
    scene: &mut [crate::OwnedScene],
) -> bool {
    if tag.own_material {
        return true;
    }
    let Some(copy) = materials.get(material.0.id()).cloned() else {
        return false;
    };
    let handle = materials.add(copy);
    if let Some(row) = scene.get_mut(tag.scene_row) {
        row.2 = handle.clone();
    }
    material.0 = handle;
    tag.own_material = true;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_level::parts::{Pos, RenderMesh, World as WorldPart};
    use lbp_level::thing::ThingRecord;
    use lbp_script_vm::natives::World;

    /// Thing 1: a mesh at x = 10, visible. Thing 2: a mesh the file stores hidden. Thing
    /// 3: a shape (no draw entity of its own).
    fn level() -> LevelWorld {
        let mut t: Vec<ThingRecord> = (0..4)
            .map(|index| ThingRecord {
                index,
                ..Default::default()
            })
            .collect();
        t[0].data.world = Some(WorldPart::default());
        for (n, flags) in [(1, 3), (2, 0)] {
            t[n].data.render_mesh = Some(RenderMesh {
                mesh: None,
                bone_things: vec![],
                anim: None,
                anim_pos: 0.,
                anim_speed: 0.,
                anim_loop: false,
                editor_color: 0xffff_ffff,
                cast_shadows: 0,
                visibility_flags: flags,
                poppet_render_scale: 1.,
            });
            let mut world = Mat4::IDENTITY.to_cols_array();
            world[12] = 10.;
            t[n].data.pos = Some(Pos {
                bone_of: None,
                anim_hash: 0,
                world,
            });
        }
        let mut w = LevelWorld::from_things(&t);
        w.derive_local_matrices();
        w.things[3].shape_colour = Some([1.; 4]);
        w
    }

    fn app() -> (App, [Entity; 3], Handle<ViewerMaterial>) {
        let mut app = App::new();
        app.insert_resource(Assets::<ViewerMaterial>::default());
        let shared = app
            .world_mut()
            .resource_mut::<Assets<ViewerMaterial>>()
            .add(dual::material(StandardMaterial::default()));
        // The item of Thing 1 sits 2 units from its Thing (a bind pose); its decoration
        // 5 units; display units are half level units.
        let item = |thing, decoration, x: f32| Item {
            thing,
            decoration,
            world: Mat4::from_translation(Vec3::new(10. + x, 0., 0.)),
            mesh: None,
            alternate: false,
        };
        let mut things = LevelThings::new(Mat4::from_scale(Vec3::splat(0.5)));
        let mut spawned = Vec::new();
        for (row, item) in [item(1, false, 2.), item(1, true, 5.), item(2, false, 0.)]
            .into_iter()
            .enumerate()
        {
            let entity = app
                .world_mut()
                .spawn((
                    Transform::from_matrix(things.normalize * item.world),
                    Visibility::default(),
                    MeshMaterial3d(shared.clone()),
                    LevelThing {
                        item,
                        scene_row: row,
                        own_material: false,
                        dissolve: None,
                        material: None,
                    },
                ))
                .id();
            things.add(item.thing, entity);
            spawned.push(entity);
        }
        assert_eq!(things.counts(), (2, 3));
        app.insert_resource(things);
        (app, [spawned[0], spawned[1], spawned[2]], shared)
    }

    /// Runs [`apply`] as a system would, with a scene list of three rows.
    fn run(
        app: &mut App,
        world: &LevelWorld,
        list: Vec<(u32, DisplayChange)>,
        scene: &mut [crate::OwnedScene],
    ) -> Followed {
        type Params<'w, 's> = (
            Res<'w, LevelThings>,
            Query<'w, 's, Parts<'static>>,
            ResMut<'w, Assets<ViewerMaterial>>,
        );
        let mut state: bevy::ecs::system::SystemState<Params> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let (things, mut entities, mut materials) = state.get_mut(app.world_mut()).unwrap();
        apply(world, &list, &things, &mut entities, &mut materials, scene)
    }

    #[test]
    fn hidden_tinted_and_moved_things_reach_their_entities() {
        let mut world = level();
        let (mut app, [mesh, decoration, stored_hidden], shared) = app();
        let mut scene: Vec<crate::OwnedScene> = [mesh, decoration, stored_hidden]
            .map(|e| (e, Handle::default(), shared.clone(), None))
            .into();
        let visible = |app: &App, e: Entity| *app.world().get::<Visibility>(e).unwrap();
        let x = |app: &App, e: Entity| app.world().get::<Transform>(e).unwrap().translation.x;

        // A new scene: the mesh the file stores hidden is hidden; nothing else changes.
        let things = app.world().resource::<LevelThings>();
        let first = first_list(&world, things);
        assert_eq!(first, [(2, DisplayChange::VISIBILITY)]);
        let done = run(&mut app, &world, first, &mut scene);
        assert_eq!(
            (done.listed, done.drawn, done.hidden, done.entities),
            (1, 1, 1, 1)
        );
        assert_eq!(visible(&app, stored_hidden), Visibility::Hidden);
        assert_eq!(visible(&app, mesh), Visibility::Inherited);

        // A script hides Thing 1, tints it and moves it 4 units; and colours the shape.
        world.set_render_mesh_visible(1, false).unwrap();
        world.set_render_mesh_tint(1, [1., 0.5, 0., 1.]).unwrap();
        let mut to = Mat4::IDENTITY.to_cols_array();
        to[12] = 14.;
        world.set_world_transform(1, to).unwrap();
        world.set_shape_editor_colour(3, [0., 0., 1., 1.]).unwrap();
        world.set_render_mesh_colour(2, [0., 1., 0., 1.]).unwrap();
        let list = world.take_display_dirty();
        assert_eq!(list.len(), 3);
        let done = run(&mut app, &world, list, &mut scene);
        let want = Followed {
            listed: 3,
            drawn: 2,
            hidden: 1,
            tinted: 1,
            placed: 1,
            entities: 3,
            not_drawn: 1,
            colour_not_drawn: 2,
            ..Default::default()
        };
        assert_eq!(done, want);
        assert!(done.any() && done.line().contains("hidden 1"));
        // The mesh is hidden, its decoration is not; both moved 4 level units = 2 here.
        assert_eq!(visible(&app, mesh), Visibility::Hidden);
        assert_eq!(visible(&app, decoration), Visibility::Inherited);
        assert!((x(&app, mesh) - 8.).abs() < 1e-5 && (x(&app, decoration) - 9.5).abs() < 1e-5);
        // The mesh has a material of its own now, in its scene row; the others share.
        let material = |app: &App, e: Entity| {
            app.world()
                .get::<MeshMaterial3d<ViewerMaterial>>(e)
                .unwrap()
                .0
                .clone()
        };
        let own = material(&app, mesh);
        assert_ne!(own, shared);
        assert_eq!(scene[0].2, own);
        assert_eq!(material(&app, decoration), shared);
        assert_eq!(
            (scene[1].2.clone(), scene[2].2.clone()),
            (shared.clone(), shared)
        );

        // Shown again with a white tint that has an alpha: the same own material, no new
        // one, and no tint drawn. Thing 2 gets such a tint too: no material of its own.
        world.set_render_mesh_visible(1, true).unwrap();
        world.set_render_mesh_tint(1, [1., 1., 1., 0.5]).unwrap();
        world.set_render_mesh_tint(2, [1., 1., 1., 0.5]).unwrap();
        let list = world.take_display_dirty();
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!(
            (done.shown, done.untinted, done.tinted, done.placed),
            (1, 1, 0, 0)
        );
        assert_eq!(done.tint_alpha_not_drawn, 2);
        assert_eq!(material(&app, stored_hidden), scene[2].2);
        assert_eq!(visible(&app, mesh), Visibility::Inherited);
        assert_eq!(material(&app, mesh), own);
        let mut sum = Followed::default();
        sum.absorb(&done);
        sum.absorb(&want);
        assert_eq!((sum.listed, sum.hidden, sum.shown), (5, 1, 1));
    }

    /// D276. One Thing with two sets of draw entities, the file's mesh and the other
    /// mesh of a pair (spawned hidden), and a decoration. A swap shows one set and hides
    /// the other, and back; a mesh the visibility byte hides stays hidden through a
    /// swap in either order; a mesh that was not loaded with the scene is counted and
    /// leaves the file's set in the scene.
    #[test]
    fn d276_a_swapped_mesh_shows_its_own_entities() {
        let [[tent, platform]] = SWAPPED_MESHES;
        assert_eq!((tent, platform), (0x2f6c, 145_753));
        // Thing 1 of the level has the first mesh of the pair in the file; the three
        // GUIDs are meshes a script loaded.
        let mut world = level();
        world.things[1].mesh_guid = Some(tent);
        world.pod.mesh_resources.extend([tent, platform, 0x77]);
        let mut app = App::new();
        app.insert_resource(Assets::<ViewerMaterial>::default());
        let shared = app
            .world_mut()
            .resource_mut::<Assets<ViewerMaterial>>()
            .add(dual::material(StandardMaterial::default()));
        let item = |mesh, alternate, decoration| Item {
            thing: 1,
            decoration,
            world: Mat4::IDENTITY,
            mesh,
            alternate,
        };
        let mut things = LevelThings::new(Mat4::IDENTITY);
        let mut spawned = Vec::new();
        // The file's mesh in two draw entities (two material groups), the other mesh in
        // one, hidden as the scene spawns it, and a decoration.
        for (row, item) in [
            item(Some(tent), false, false),
            item(Some(tent), false, false),
            item(Some(platform), true, false),
            item(Some(0x55), false, true),
        ]
        .into_iter()
        .enumerate()
        {
            let visibility = if item.alternate {
                Visibility::Hidden
            } else {
                Visibility::default()
            };
            let tag = LevelThing {
                item,
                scene_row: row,
                own_material: false,
                dissolve: None,
                material: None,
            };
            let parts = (
                Transform::IDENTITY,
                visibility,
                MeshMaterial3d(shared.clone()),
                tag,
            );
            let entity = app.world_mut().spawn(parts).id();
            things.add(1, entity);
            spawned.push(entity);
        }
        app.insert_resource(things);
        let mut scene: Vec<crate::OwnedScene> = spawned
            .iter()
            .map(|&e| (e, Handle::default(), shared.clone(), None))
            .collect();
        let shown = |app: &App| -> Vec<bool> {
            let visible = |e: &Entity| *app.world().get::<Visibility>(*e).unwrap();
            spawned
                .iter()
                .map(|e| visible(e) != Visibility::Hidden)
                .collect()
        };
        let mut step = |app: &mut App, world: &mut LevelWorld| {
            let list = world.take_display_dirty();
            run(app, world, list, &mut scene)
        };
        // A new scene with the file's mesh: nothing to do, the other set stays hidden.
        let things = app.world().resource::<LevelThings>();
        assert!(first_list(&world, things).is_empty());
        assert_eq!(shown(&app), [true, true, false, true]);

        // The swap: the other set is shown, the file's hidden, the decoration stays.
        world.set_render_mesh_mesh(1, Some(platform)).unwrap();
        let done = step(&mut app, &mut world);
        assert_eq!((done.mesh_swapped, done.mesh_not_drawn), (1, 0));
        assert_eq!((done.hidden, done.shown, done.entities), (0, 0, 4));
        assert!(done.any() && done.line().contains("mesh swapped 1"));
        assert_eq!(shown(&app), [false, false, true, true]);
        // A scene spawned now starts from the file's set and is told of the swap.
        let things = app.world().resource::<LevelThings>();
        assert_eq!(first_list(&world, things), [(1, DisplayChange::MESH)]);
        // The same mesh set again changes nothing and lists nothing.
        world.set_render_mesh_mesh(1, Some(platform)).unwrap();
        assert!(world.take_display_dirty().is_empty());

        // Hidden by the visibility byte while swapped: only the shown set is hidden.
        world.set_render_mesh_visible(1, false).unwrap();
        let done = step(&mut app, &mut world);
        assert_eq!((done.hidden, done.mesh_swapped), (1, 0));
        assert_eq!(shown(&app), [false, false, false, true]);
        // Swapped back while hidden: the swap is followed and nothing is shown.
        world.set_render_mesh_mesh(1, Some(tent)).unwrap();
        let done = step(&mut app, &mut world);
        assert_eq!((done.mesh_swapped, done.shown, done.hidden), (1, 0, 0));
        assert_eq!(shown(&app), [false, false, false, true]);
        // Shown again: the set of the mesh the Thing has now, not the other.
        world.set_render_mesh_visible(1, true).unwrap();
        let done = step(&mut app, &mut world);
        assert_eq!((done.shown, done.mesh_swapped), (1, 0));
        assert_eq!(shown(&app), [true, true, false, true]);

        // The other order: hidden first, then swapped, then shown.
        world.set_render_mesh_visible(1, false).unwrap();
        step(&mut app, &mut world);
        world.set_render_mesh_mesh(1, Some(platform)).unwrap();
        step(&mut app, &mut world);
        assert_eq!(shown(&app), [false, false, false, true]);
        world.set_render_mesh_visible(1, true).unwrap();
        step(&mut app, &mut world);
        assert_eq!(shown(&app), [false, false, true, true]);
        // Hidden and swapped back on one tick, then shown.
        world.set_render_mesh_visible(1, false).unwrap();
        world.set_render_mesh_mesh(1, Some(tent)).unwrap();
        let done = step(&mut app, &mut world);
        assert_eq!((done.mesh_swapped, done.hidden, done.shown), (1, 0, 0));
        assert_eq!(shown(&app), [false, false, false, true]);
        world.set_render_mesh_visible(1, true).unwrap();
        step(&mut app, &mut world);
        assert_eq!(shown(&app), [true, true, false, true]);

        // A mesh that was not loaded with the scene: counted, the file's set stays;
        // also when it comes after the other mesh of the pair.
        world.set_render_mesh_mesh(1, Some(0x77)).unwrap();
        let done = step(&mut app, &mut world);
        assert_eq!((done.mesh_not_drawn, done.mesh_swapped), (1, 0));
        assert!(!done.any());
        assert_eq!(shown(&app), [true, true, false, true]);
        world.set_render_mesh_mesh(1, Some(platform)).unwrap();
        step(&mut app, &mut world);
        assert_eq!(shown(&app), [false, false, true, true]);
        world.set_render_mesh_mesh(1, Some(0x77)).unwrap();
        let done = step(&mut app, &mut world);
        assert_eq!((done.mesh_not_drawn, done.mesh_swapped), (1, 0));
        assert_eq!(shown(&app), [true, true, false, true]);
        let mut sum = Followed::default();
        sum.absorb(&done);
        sum.absorb(&Followed {
            mesh_swapped: 2,
            ..Default::default()
        });
        assert_eq!((sum.mesh_not_drawn, sum.mesh_swapped), (1, 2));
    }

    /// D278. The pod Thing of a map with a dissolve mesh: the tent's two entities get
    /// the value and the colour and a material each on the first tick of the change-over;
    /// the platform's entity and the decoration get nothing; at alpha 1 nothing is
    /// cloned or counted; at alpha 0 and back at alpha 1 the dissolve is taken off. A map
    /// without a dissolve mesh counts the colour as not drawn, as before.
    #[test]
    fn d278_the_tent_entities_dissolve_by_the_shape_colours_alpha() {
        let [[tent, platform]] = SWAPPED_MESHES;
        let mut world = level();
        world.things[1].mesh_guid = Some(tent);
        world.things[1].shape_colour = Some([1.; 4]);
        world.pod.mesh_resources.extend([tent, platform]);
        assert_eq!(world.pod_thing(true), Some(1));
        let mut app = App::new();
        app.insert_resource(Assets::<ViewerMaterial>::default());
        let shared = app
            .world_mut()
            .resource_mut::<Assets<ViewerMaterial>>()
            .add(dual::material(StandardMaterial::default()));
        let item = |mesh, alternate, decoration| Item {
            thing: 1,
            decoration,
            world: Mat4::IDENTITY,
            mesh,
            alternate,
        };
        let mut things = LevelThings::new(Mat4::IDENTITY);
        things.dissolve_mesh = Some(tent);
        let mut spawned = Vec::new();
        for (row, item) in [
            item(Some(tent), false, false),
            item(Some(tent), false, false),
            item(Some(platform), true, false),
            item(Some(tent), false, true),
        ]
        .into_iter()
        .enumerate()
        {
            let tag = LevelThing {
                item,
                scene_row: row,
                own_material: false,
                dissolve: None,
                material: None,
            };
            let parts = (
                Transform::IDENTITY,
                Visibility::default(),
                MeshMaterial3d(shared.clone()),
                tag,
            );
            let entity = app.world_mut().spawn(parts).id();
            things.add(1, entity);
            spawned.push(entity);
        }
        app.insert_resource(things);
        let mut scene: Vec<crate::OwnedScene> = spawned
            .iter()
            .map(|&e| (e, Handle::default(), shared.clone(), None))
            .collect();
        let seen = |app: &App| -> Vec<(Option<f32>, bool)> {
            let tag = |e: &Entity| app.world().get::<LevelThing>(*e).unwrap();
            let own = |e: &Entity| {
                app.world()
                    .get::<MeshMaterial3d<ViewerMaterial>>(*e)
                    .unwrap()
                    .0
                    != shared
            };
            spawned
                .iter()
                .map(|e| (tag(e).dissolve.map(|d| d.value), own(e)))
                .collect()
        };
        let untouched = (None, false);
        let count = |app: &App| app.world().resource::<Assets<ViewerMaterial>>().len();

        // Alpha 1 with another colour (the script's first update, not in use): nothing
        // is cloned, nothing is set; the colour is not counted as not drawn.
        world.set_shape_editor_colour(1, [0., 0., 0., 1.]).unwrap();
        let list = world.take_display_dirty();
        assert_eq!(list, [(1, DisplayChange::COLOUR)]);
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!((done.dissolved, done.colour_not_drawn), (0, 0));
        assert!(!done.any());
        assert_eq!(seen(&app), [untouched; 4]);
        assert_eq!(count(&app), 1);

        // The first tick of the change-over: the two tent entities, each with its own
        // material in its scene row; the platform's entity and the decoration not.
        world
            .set_shape_editor_colour(1, [0.25, 0.5, 0.75, 0.95])
            .unwrap();
        let list = world.take_display_dirty();
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!((done.dissolved, done.colour_not_drawn), (1, 0));
        assert!(done.any() && done.line().contains("dissolve set 1"));
        assert_eq!((done.tinted, done.untinted), (0, 0));
        let now = seen(&app);
        assert!((now[0].0.unwrap() - 0.05).abs() < 1e-6 && now[0].1);
        assert_eq!(now[0], now[1]);
        assert_eq!(now[2..], [untouched; 2]);
        assert_eq!(count(&app), 3);
        assert_ne!(scene[0].2, shared);
        assert_ne!(scene[0].2, scene[1].2);
        assert_eq!(scene[2].2, shared);
        let tag = app.world().get::<LevelThing>(spawned[0]).unwrap();
        assert_eq!(tag.dissolve.unwrap().colour, [0.25, 0.5, 0.75]);
        // A scene spawned now is told through the first list.
        let things = app.world().resource::<LevelThings>();
        assert!(first_list(&world, things)[0].1.colour);

        // The next tick: a new value, no new material.
        world
            .set_shape_editor_colour(1, [0.25, 0.5, 0.75, 0.5])
            .unwrap();
        let list = world.take_display_dirty();
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!((done.dissolved, done.untinted), (1, 0));
        assert_eq!(seen(&app)[1], (Some(0.5), true));
        assert_eq!(count(&app), 3);
        // The line is in the material (as `level_play::follow_dissolve` writes it).
        let own = scene[0].2.clone();
        {
            let mut materials = app.world_mut().resource_mut::<Assets<ViewerMaterial>>();
            let mut m = materials.get_mut(own.id()).unwrap();
            let d = dual::Dissolve {
                plane: Vec4::Y,
                ..Default::default()
            };
            dual::set_dissolve(&mut m, Some(&d));
        }

        // Alpha 0 with the platform's mesh: the swap, and the dissolve is taken off
        // the tag and the material; the material stays the entity's own.
        world
            .set_shape_editor_colour(1, [0.25, 0.5, 0.75, 0.])
            .unwrap();
        world.set_render_mesh_mesh(1, Some(platform)).unwrap();
        let list = world.take_display_dirty();
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!((done.dissolved, done.mesh_swapped), (1, 1));
        assert_eq!(
            seen(&app),
            [(None, true), (None, true), untouched, untouched]
        );
        let materials = app.world().resource::<Assets<ViewerMaterial>>();
        assert!(dual::dissolve(materials.get(own.id()).unwrap()).is_none());
        // The same list again changes nothing.
        let done = run(
            &mut app,
            &world,
            vec![(1, DisplayChange::COLOUR)],
            &mut scene,
        );
        assert_eq!(done.dissolved, 0);

        // Back: the tent's mesh and a value under 1 on one tick, then closed.
        world
            .set_shape_editor_colour(1, [0.25, 0.5, 0.75, 0.05])
            .unwrap();
        world.set_render_mesh_mesh(1, Some(tent)).unwrap();
        let list = world.take_display_dirty();
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!((done.dissolved, done.mesh_swapped), (1, 1));
        assert!((seen(&app)[0].0.unwrap() - 0.95).abs() < 1e-6);
        world
            .set_shape_editor_colour(1, [0.25, 0.5, 0.75, 1.])
            .unwrap();
        let list = world.take_display_dirty();
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!(done.dissolved, 1);
        assert_eq!(seen(&app)[..2], [(None, true); 2]);
        assert_eq!(count(&app), 3);

        // A map without a dissolve mesh (every other level): the colour is counted as
        // not drawn and nothing is set.
        app.world_mut().resource_mut::<LevelThings>().dissolve_mesh = None;
        world
            .set_shape_editor_colour(1, [0.25, 0.5, 0.75, 0.5])
            .unwrap();
        let list = world.take_display_dirty();
        let done = run(&mut app, &world, list, &mut scene);
        assert_eq!((done.dissolved, done.colour_not_drawn), (0, 1));
        assert_eq!(seen(&app)[0].0, None);
        let mut sum = Followed::default();
        sum.absorb(&Followed {
            dissolved: 2,
            ..Default::default()
        });
        assert!(sum.any() && sum.dissolved == 2);
    }

    /// D284. The pod Thing with the tent's three buckets and the platform's two: under 1
    /// the tent's entities dissolve and the platform's do not; at alpha 0 (the swap) the
    /// tent's dissolve is off, the platform's card edge gets value 1 and a material of
    /// its own, its other material and the decoration nothing; back under 1 the edge
    /// loses it, in the tag and in the material.
    #[test]
    fn d284_the_platform_edge_keeps_the_dissolve_at_one() {
        use crate::pod_dissolve::{CARD, CARD_EDGE, PLATFORM_MESH};
        let [[tent, platform]] = SWAPPED_MESHES;
        assert_eq!(platform, PLATFORM_MESH);
        let mut world = level();
        world.things[1].mesh_guid = Some(tent);
        world.things[1].shape_colour = Some([1.; 4]);
        world.pod.mesh_resources.extend([tent, platform]);
        let mut app = App::new();
        app.insert_resource(Assets::<ViewerMaterial>::default());
        let shared = app
            .world_mut()
            .resource_mut::<Assets<ViewerMaterial>>()
            .add(dual::material(StandardMaterial::default()));
        let mut things = LevelThings::new(Mat4::IDENTITY);
        things.dissolve_mesh = Some(tent);
        things.dissolve_edge = Some((PLATFORM_MESH, CARD_EDGE));
        let glass = 0x0001_d14c;
        let platform_card = 0x0002_52f3;
        let mut spawned = Vec::new();
        for (row, (mesh, alternate, decoration, material)) in [
            (tent, false, false, CARD),
            (tent, false, false, CARD_EDGE),
            (tent, false, false, glass),
            (platform, true, false, CARD_EDGE),
            (platform, true, false, platform_card),
            (platform, false, true, CARD_EDGE),
        ]
        .into_iter()
        .enumerate()
        {
            let tag = LevelThing {
                item: Item {
                    thing: 1,
                    decoration,
                    world: Mat4::IDENTITY,
                    mesh: Some(mesh),
                    alternate,
                },
                scene_row: row,
                own_material: false,
                dissolve: None,
                material: Some(material),
            };
            let parts = (
                Transform::IDENTITY,
                Visibility::default(),
                MeshMaterial3d(shared.clone()),
                tag,
            );
            let entity = app.world_mut().spawn(parts).id();
            things.add(1, entity);
            spawned.push(entity);
        }
        app.insert_resource(things);
        let mut scene: Vec<crate::OwnedScene> = spawned
            .iter()
            .map(|&e| (e, Handle::default(), shared.clone(), None))
            .collect();
        let values = |app: &App| -> Vec<Option<f32>> {
            let tag = |e: &Entity| app.world().get::<LevelThing>(*e).unwrap();
            spawned
                .iter()
                .map(|e| tag(e).dissolve.map(|d| d.value))
                .collect()
        };
        let step = |app: &mut App, world: &mut LevelWorld, scene: &mut [crate::OwnedScene]| {
            let list = world.take_display_dirty();
            run(app, world, list, scene)
        };

        // Under 1: the tent's three buckets, not the platform's.
        world
            .set_shape_editor_colour(1, [0.3, 0.7, 0.1, 0.5])
            .unwrap();
        let done = step(&mut app, &mut world, &mut scene);
        assert_eq!(done.dissolved, 1);
        let half = Some(0.5);
        assert_eq!(values(&app), [half, half, half, None, None, None]);

        // Alpha 0 and the swap: the card edge of the platform at 1, nothing else.
        world
            .set_shape_editor_colour(1, [0.3, 0.7, 0.1, 0.])
            .unwrap();
        world.set_render_mesh_mesh(1, Some(platform)).unwrap();
        let done = step(&mut app, &mut world, &mut scene);
        assert_eq!((done.dissolved, done.mesh_swapped), (1, 1));
        assert_eq!(values(&app), [None, None, None, Some(1.), None, None]);
        let tag = app.world().get::<LevelThing>(spawned[3]).unwrap();
        assert_eq!(tag.dissolve.unwrap().colour, [0.3, 0.7, 0.1]);
        assert!(tag.own_material);
        let edge = scene[3].2.clone();
        assert_ne!(edge, shared);
        assert_eq!(scene[4].2, shared);
        assert_eq!(scene[5].2, shared);
        // The same list again changes nothing.
        let again = run(
            &mut app,
            &world,
            vec![(1, DisplayChange::COLOUR)],
            &mut scene,
        );
        assert_eq!(again.dissolved, 0);
        // As `level_play::follow_dissolve` writes it.
        {
            let mut materials = app.world_mut().resource_mut::<Assets<ViewerMaterial>>();
            let mut m = materials.get_mut(edge.id()).unwrap();
            let d = dual::Dissolve {
                plane: Vec4::Y,
                at_one: true,
                ..Default::default()
            };
            dual::set_dissolve(&mut m, Some(&d));
        }

        // Back under 1 with the tent's mesh: the edge loses value 1, the tent has 0.95.
        world
            .set_shape_editor_colour(1, [0.3, 0.7, 0.1, 0.05])
            .unwrap();
        world.set_render_mesh_mesh(1, Some(tent)).unwrap();
        let done = step(&mut app, &mut world, &mut scene);
        assert_eq!(done.dissolved, 1);
        let now = values(&app);
        assert!((now[0].unwrap() - 0.95).abs() < 1e-6);
        assert_eq!(now[3..], [None; 3]);
        let materials = app.world().resource::<Assets<ViewerMaterial>>();
        assert!(dual::dissolve(materials.get(edge.id()).unwrap()).is_none());

        // A map without the edge (D278's): the platform is left alone at alpha 0.
        app.world_mut().resource_mut::<LevelThings>().dissolve_edge = None;
        world
            .set_shape_editor_colour(1, [0.3, 0.7, 0.1, 0.])
            .unwrap();
        world.set_render_mesh_mesh(1, Some(platform)).unwrap();
        step(&mut app, &mut world, &mut scene);
        assert_eq!(values(&app), [None; 6]);
    }

    /// D277. Two levels drawn at once, each with its own map and script world, and the
    /// same Thing number (1) in both: a change in one world reaches the entities of its
    /// own map only, placed by that map's display matrix.
    #[test]
    fn d277_two_maps_keep_two_levels_apart() {
        let (mut level_world, mut shell_world) = (level(), level());
        let mut app = App::new();
        app.insert_resource(Assets::<ViewerMaterial>::default());
        let shared = app
            .world_mut()
            .resource_mut::<Assets<ViewerMaterial>>()
            .add(dual::material(StandardMaterial::default()));
        let item = Item {
            thing: 1,
            decoration: false,
            world: Mat4::from_translation(Vec3::new(10., 0., 0.)),
            mesh: None,
            alternate: false,
        };
        // The level's display units are half its level units, the shell's a quarter.
        let mut maps =
            [0.5, 0.25].map(|scale| LevelThings::new(Mat4::from_scale(Vec3::splat(scale))));
        assert_ne!(maps[0].scene, maps[1].scene);
        let mut spawned = Vec::new();
        for (row, map) in maps.iter_mut().enumerate() {
            let tag = LevelThing {
                item,
                scene_row: row,
                own_material: false,
                dissolve: None,
                material: None,
            };
            let parts = (
                Transform::from_matrix(map.normalize * item.world),
                Visibility::default(),
                MeshMaterial3d(shared.clone()),
                tag,
            );
            let entity = app.world_mut().spawn(parts).id();
            map.add(1, entity);
            spawned.push(entity);
        }
        let [level_map, shell_map] = maps;
        let shell_map = ShellThings(shell_map);
        let mut scene: Vec<crate::OwnedScene> = spawned
            .iter()
            .map(|&e| (e, Handle::default(), shared.clone(), None))
            .collect();
        let mut step = |app: &mut App, world: &mut LevelWorld, map: &LevelThings| {
            let list = world.take_display_dirty();
            let mut state: bevy::ecs::system::SystemState<(
                Query<Parts<'static>>,
                ResMut<Assets<ViewerMaterial>>,
            )> = bevy::ecs::system::SystemState::new(app.world_mut());
            let (mut entities, mut materials) = state.get_mut(app.world_mut()).unwrap();
            apply(world, &list, map, &mut entities, &mut materials, &mut scene)
        };
        let seen = |app: &App| -> Vec<(Visibility, f32)> {
            let world = app.world();
            let one = |e: &Entity| {
                let x = world.get::<Transform>(*e).unwrap().translation.x;
                (*world.get::<Visibility>(*e).unwrap(), x)
            };
            spawned.iter().map(one).collect()
        };
        assert_eq!(
            seen(&app),
            [(Visibility::Inherited, 5.), (Visibility::Inherited, 2.5)]
        );

        // The shell's script hides its Thing 1 and moves it 4 level units.
        let mut to = Mat4::IDENTITY.to_cols_array();
        to[12] = 14.;
        shell_world.set_render_mesh_visible(1, false).unwrap();
        shell_world.set_world_transform(1, to).unwrap();
        let done = step(&mut app, &mut shell_world, &shell_map.0);
        assert_eq!((done.hidden, done.placed, done.entities), (1, 1, 1));
        assert_eq!(
            seen(&app),
            [(Visibility::Inherited, 5.), (Visibility::Hidden, 3.5)]
        );
        // The level's world has nothing to say, and its map leaves the shell alone.
        let done = step(&mut app, &mut level_world, &level_map);
        assert_eq!((done.listed, done.entities), (0, 0));
        // The other way round: the level's Thing 1 moves, the shell's stays.
        level_world.set_world_transform(1, to).unwrap();
        let done = step(&mut app, &mut level_world, &level_map);
        assert_eq!((done.hidden, done.placed, done.entities), (0, 1, 1));
        assert_eq!(
            seen(&app),
            [(Visibility::Inherited, 7.), (Visibility::Hidden, 3.5)]
        );
    }
}
