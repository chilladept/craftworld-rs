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
}

/// On a draw entity of the played level: the Thing it shows.
#[derive(Component)]
pub struct LevelThing {
    pub item: Item,
    /// The entity's row in the viewer's scene list (the list the lighting walks).
    pub scene_row: usize,
    /// The entity has a material of its own (made for its first tint).
    pub own_material: bool,
}

/// The Thing-to-entity map of the level scene.
#[derive(Resource)]
pub struct LevelThings {
    /// Level units to display units.
    pub normalize: Mat4,
    pub entities: HashMap<u32, Vec<Entity>>,
    /// A number no other scene has had.
    pub scene: u64,
}

static SCENES: AtomicU64 = AtomicU64::new(1);

impl LevelThings {
    pub fn new(normalize: Mat4) -> Self {
        Self {
            normalize,
            entities: HashMap::new(),
            scene: SCENES.fetch_add(1, Ordering::Relaxed),
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
    /// Things whose PRenderMesh shows another mesh than the level file's: the scene
    /// keeps the file's mesh.
    pub mesh_not_drawn: usize,
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
    }

    /// Whether anything in the scene was changed.
    pub fn any(&self) -> bool {
        self.hidden + self.shown + self.tinted + self.untinted + self.placed > 0
    }

    pub fn line(&self) -> String {
        format!(
            "listed {}, with a draw entity {}; hidden {}, shown {}, tinted {}, tint taken off {}, placed {} ({} entities); not followed: no draw entity {}, editor or shape colour {}, tint of alpha only {}, no placement {}, another mesh {}",
            self.listed,
            self.drawn,
            self.hidden,
            self.shown,
            self.tinted,
            self.untinted,
            self.placed,
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
/// - Mesh: a Thing whose PRenderMesh shows another mesh than the level file's keeps the
///   file's mesh in the scene, and is counted.
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
        done.colour_not_drawn += usize::from(change.colour && other_colour);
        done.mesh_not_drawn += usize::from(change.mesh && now.mesh != was.mesh);
        let Some(list) = things.entities.get(&thing).filter(|l| !l.is_empty()) else {
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
            if change.visibility {
                let want = if now.mesh_hidden() {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                };
                if *visibility != want {
                    hid |= want == Visibility::Hidden;
                    showed |= want == Visibility::Inherited;
                    *visibility = want;
                }
            }
            if change.colour && (tint.is_some() || tag.own_material) {
                if !tag.own_material {
                    let Some(copy) = materials.get(material.0.id()).cloned() else {
                        continue;
                    };
                    let handle = materials.add(copy);
                    if let Some(row) = scene.get_mut(tag.scene_row) {
                        row.2 = handle.clone();
                    }
                    material.0 = handle;
                    tag.own_material = true;
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
        done.not_placed += usize::from(moved == Some(None));
    }
    done
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
}
