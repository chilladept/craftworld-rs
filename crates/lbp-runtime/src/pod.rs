//! D270 (docs/level-scripts.md, part 21): what the Pod's world script asks the engine for.
//! The Things a script finds by name ([`LevelWorld::find_thing`]), the pod Thing
//! ([`LevelWorld::pod_thing`]), the mesh a script gives a Thing's PRenderMesh, and the pod
//! computer's values that are only kept ([`PodState`]).
use crate::LevelWorld;
use lbp_script_vm::natives::{DEFAULT_PLANETS_BACKGROUND, POD_MESH, POD_PLATFORM_MESH};
use std::collections::BTreeSet;

/// One `PodComputer.RequestHostUsePod__b` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostUsePodRequest {
    /// True asks to use the pod computer, false to stop.
    pub in_use: bool,
    /// `PWorld.Frame` when the call was made.
    pub frame: i32,
}

/// The Pod state of a level world ([`LevelWorld::pod`]).
#[derive(Debug, Clone, Default)]
pub struct PodState {
    /// The backdrop plan `PodComputer.GetPlanetsBackgroundGUID__` answers with; none
    /// while the planets level is not loaded, and the native then gives
    /// [`DEFAULT_PLANETS_BACKGROUND`] (FOUND). Nothing sets it here: the runtime plays
    /// one level, and the field of the planets level's PWorld the game reads was not
    /// matched to a stored value.
    pub planets_background: Option<u32>,
    /// The resource handles `Resource.LoadMeshByFilename__g` handed out: the handles
    /// that are a mesh, for `Thing.SetRenderMeshMesh`.
    pub mesh_resources: BTreeSet<u32>,
    /// `Thing.FindThing__Q7StringA` calls, and those that found a Thing.
    pub find_thing_calls: u64,
    pub find_thing_hits: u64,
    /// `Thing.GetPodThing__b` calls, and those that found a Thing.
    pub pod_thing_calls: u64,
    pub pod_thing_hits: u64,
    /// `Thing.SetRenderMeshMesh` calls on a Thing with a PRenderMesh, and those that
    /// changed its mesh.
    pub mesh_sets: u64,
    pub mesh_changes: u64,
    /// The last value `PodComputer.SetFadePodDecorations__f` was given, and the calls.
    /// Nothing fades by it (the game's arithmetic was not read).
    pub fade: Option<f32>,
    pub fade_calls: u64,
    /// `PodComputer.RequestHostUsePod__b` calls, oldest first, at most
    /// [`MAX_HOST_USE_POD_REQUESTS`]. The game sends them to the host; here nothing
    /// answers them and [`LevelWorld::pod_computer_in_use`] does not change.
    pub host_use_pod_requests: Vec<HostUsePodRequest>,
}

/// Most requests [`PodState::host_use_pod_requests`] keeps.
pub const MAX_HOST_USE_POD_REQUESTS: usize = 256;

impl LevelWorld {
    /// The first Thing, in Thing order, whose PScriptName is exactly `name`.
    ///
    /// FOUND (01.33, 0xd3360): the game walks a list of the PWorld and compares each
    /// entry's byte string with the name by the C library's string compare: every byte,
    /// case sensitive. The first equal entry wins; none gives null. INFERRED: the list
    /// is the level's PScriptName parts in Thing order. A name with a NUL inside is
    /// kept up to the NUL by the reader, as a C string ends there.
    pub fn find_thing(&self, name: &[u8]) -> Option<u32> {
        let named = |t: &crate::ThingState| t.script_name.as_deref() == Some(name);
        self.things.iter().position(named).map(|i| i as u32)
    }

    /// How many Things have a PScriptName, and how many of those names are not empty.
    pub fn script_names(&self) -> (usize, usize) {
        let names = || self.things.iter().filter_map(|t| t.script_name.as_ref());
        (names().count(), names().filter(|n| !n.is_empty()).count())
    }

    /// The pod Thing: what `Thing.GetPodThing__b` answers.
    ///
    /// FOUND (01.33, 0xd32dc): the game walks a list of the PWorld whose entries have a
    /// Thing and a resource. The first entry whose resource is [`POD_MESH`] gives its
    /// Thing. An entry whose resource is [`POD_PLATFORM_MESH`] gives its Thing when
    /// `platform_too`, and ends the search with nothing when not. No such entry: nothing.
    /// INFERRED: the list is the level's PRenderMesh parts in Thing order. The mesh is
    /// the one the PRenderMesh shows now, so the answer follows `SetRenderMeshMesh`.
    pub fn pod_thing(&self, platform_too: bool) -> Option<u32> {
        for (i, t) in self.things.iter().enumerate() {
            if t.render_mesh_visibility.is_none() {
                continue;
            }
            match t.mesh_guid {
                Some(POD_MESH) => return Some(i as u32),
                Some(POD_PLATFORM_MESH) => return platform_too.then_some(i as u32),
                _ => {}
            }
        }
        None
    }

    /// What `PodComputer.GetPlanetsBackgroundGUID__` answers.
    pub fn planets_background(&self) -> u32 {
        self.pod
            .planets_background
            .unwrap_or(DEFAULT_PLANETS_BACKGROUND)
    }
}
