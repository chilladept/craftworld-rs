//! D331 (docs/level-scripts/part-82.md): the Story view's planet world.
//!
//! FOUND (01.33, 0x43cd64 -> 0x42324c): when a story group is chosen the planet clears
//! Earth's world and pastes the group's planet decorations, a level of its own (The
//! Cosmos, group (4, 27): 0x19049), at Earth's centre and radius. That level holds an
//! Earth with the same mesh as the Pod's (its continents in other materials by a
//! PMaterialOverride), the Earth's decoration and a moon.
//!
//! The viewer loads that level with the Pod's planets level and pastes the models of
//! its Earth and the Things under it onto the Pod's Earth Thing ([`merge`]; the level's
//! moon is left out, INFERRED: it is not in Earth's world): each item is moved by the matrix that takes the
//! decorations level's Earth model onto the Pod's, and is kept as an alternate item (out
//! of the scene bounds, spawned hidden). While the Story search is up
//! ([`story_open`]) those items are shown and the Pod's own Earth and Moon are hidden.
use bevy::prelude::*;
use lbp_preview::level_scene::{self, MeshSource, ScenePayload};

type LevelScene = level_scene::LevelScene<ScenePayload>;

/// The Pod's Earth and the decorations level's Earth are drawn by this mesh.
pub const EARTH_MESH: u32 = 0x0000_27bd;

/// What [`merge`] pasted, for the load's printed line.
#[derive(Debug, Clone, PartialEq)]
pub struct Merged {
    /// The decorations level's Earth Thing.
    pub story_earth: usize,
    /// Items pasted onto the Pod's Earth, and the meshes they draw.
    pub items: usize,
    pub meshes: usize,
    /// Pasted items by the decorations level's Thing and mesh label.
    pub by_thing: std::collections::BTreeMap<(usize, String), usize>,
}

/// The first model item of `thing` drawing [`EARTH_MESH`].
fn earth_item(scene: &LevelScene, thing: Option<usize>) -> Option<&level_scene::SceneItem> {
    scene.items.iter().find(|i| {
        thing.is_none_or(|t| i.thing == t)
            && i.shapes == 0
            && !i.alternate
            && scene.meshes.get(i.mesh).is_some_and(|m| {
                matches!(m.payload, ScenePayload::Model(_))
                    && m.source.base_guid() == Some(EARTH_MESH)
            })
    })
}

/// Pastes the model items of the decorations level `story` onto Thing `globe` of
/// `scene` (the Pod's Earth), as alternate items. INFERRED: "at Earth's centre and
/// radius" is the matrix taking the decorations level's Earth model onto the Pod's.
pub fn merge(scene: &mut LevelScene, story: LevelScene, globe: usize) -> Result<Merged, String> {
    let pod = earth_item(scene, Some(globe)).ok_or("the Pod's Earth has no Earth model")?;
    let deco = earth_item(&story, None).ok_or("the decorations level has no Earth model")?;
    let story_earth = deco.thing;
    let to = Mat4::from_cols_array(&pod.world) * Mat4::from_cols_array(&deco.world).inverse();
    if !to.is_finite() {
        return Err("the Earth placement matrix is not finite".into());
    }
    let backdrop = story.counts.backdrop.thing;
    // Earth's world: the Earth Thing and the Things under it (not the level's moon).
    let in_earth = |mut thing: usize| {
        for _ in 0..=story.parents.len() {
            if thing == story_earth {
                return true;
            }
            match story.parents.get(thing).copied().flatten() {
                Some(p) => thing = p,
                None => return false,
            }
        }
        false
    };
    let model = |m: &level_scene::SceneMesh<ScenePayload>| {
        matches!(m.payload, ScenePayload::Model(_))
            && matches!(
                m.source,
                MeshSource::Guid(_) | MeshSource::Sha1(_) | MeshSource::BoneSkin { .. }
            )
    };
    let kept: Vec<_> = story
        .items
        .iter()
        .filter(|i| {
            i.shapes == 0
                && !i.alternate
                && in_earth(i.thing)
                && Some(i.thing) != backdrop
                && story.meshes.get(i.mesh).is_some_and(model)
        })
        .cloned()
        .collect();
    let mut remap = std::collections::HashMap::new();
    let labels: Vec<String> = story.meshes.iter().map(|m| m.source.label()).collect();
    let mut meshes: Vec<_> = story.meshes.into_iter().map(Some).collect();
    let mut items = 0;
    let mut by_thing = std::collections::BTreeMap::new();
    for mut item in kept {
        *by_thing
            .entry((item.thing, labels[item.mesh].clone()))
            .or_insert(0) += 1;
        let next = scene.meshes.len();
        let index = *remap.entry(item.mesh).or_insert_with(|| {
            scene
                .meshes
                .push(meshes[item.mesh].take().expect("each mesh moved once"));
            next
        });
        item.mesh = index;
        item.world = (to * Mat4::from_cols_array(&item.world)).to_cols_array();
        item.thing = globe;
        item.uid = None;
        item.alternate = true;
        scene.items.push(item);
        items += 1;
    }
    Ok(Merged {
        story_earth,
        items,
        meshes: remap.len(),
        by_thing,
    })
}

/// The Story search is up ([`lbp_runtime::LevelWorld::story_open`]).
pub fn story_open(world: &lbp_runtime::LevelWorld) -> bool {
    world.story_open()
}

/// The visibility of a draw entity of the Pod's Earth or Moon: the pasted items while
/// the Story search is up, the Pod's own otherwise.
pub fn wanted(item: &crate::thing_view::Item, globe: u32, open: bool) -> Option<Visibility> {
    let moon = item.thing == crate::pod_menu::MOON_THING as u32;
    if item.thing != globe && !moon {
        return None;
    }
    let pasted = item.thing == globe && item.alternate;
    Some(if pasted == open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(thing: u32, alternate: bool) -> crate::thing_view::Item {
        crate::thing_view::Item {
            thing,
            decoration: false,
            world: Mat4::IDENTITY,
            mesh: Some(EARTH_MESH),
            alternate,
        }
    }

    #[test]
    fn d331_story_shows_the_pasted_world_and_hides_the_pods_planets() {
        let shown = Some(Visibility::Inherited);
        let hidden = Some(Visibility::Hidden);
        // Earth is Thing 1, the Moon Thing 4 (`pod_menu::MOON_THING`).
        assert_eq!(wanted(&item(1, true), 1, true), shown);
        assert_eq!(wanted(&item(1, false), 1, true), hidden);
        assert_eq!(wanted(&item(4, false), 1, true), hidden);
        assert_eq!(wanted(&item(1, true), 1, false), hidden);
        assert_eq!(wanted(&item(1, false), 1, false), shown);
        assert_eq!(wanted(&item(4, false), 1, false), shown);
        assert_eq!(wanted(&item(7, false), 1, true), None);
    }
}
