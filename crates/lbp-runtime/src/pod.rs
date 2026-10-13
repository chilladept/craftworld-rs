//! D270 (docs/level-scripts.md, part 21): what the Pod's world script asks the engine for.
//! The Things a script finds by name ([`LevelWorld::find_thing`]), the pod Thing
//! ([`LevelWorld::pod_thing`]), the mesh a script gives a Thing's PRenderMesh, and the pod
//! computer's values that are only kept ([`PodState`]).
//!
//! D273 (part 24): the pod computer is entered. The pad zone
//! ([`LevelWorld::in_pad_zone`]), the request to use the pod computer and its answer
//! ([`LevelWorld::request_host_use_pod`], [`LevelWorld::begin_session_tick`]), and the
//! state the game keeps outside a level, handed from one level world to another
//! ([`LevelWorld::hand_session_to`], [`tick_session`]).
use crate::{LevelScripts, LevelWorld, StepReport};
use lbp_script_vm::natives::{
    Bindings, DEFAULT_PLANETS_BACKGROUND, PAD_ZONE_MIN_Z, POD_MESH, POD_PLATFORM_MESH,
};
use lbp_script_vm::Vm;
use std::collections::{BTreeMap, BTreeSet};

/// The script class of the pod computer's trigger. INFERRED: the game tests the script
/// instance of each entry of a PWorld list for one class and calls a function of it by
/// name without arguments (01.33, 0x620504, 0x6205b4); the class and the function names
/// were not read. `Trigger_Pod` has a function `IsPodComputerTrigger__` that returns
/// true and nothing else calls it.
pub const POD_TRIGGER_CLASS: &str = "Trigger_Pod";

/// The sounds the game plays at the primary player's avatar when "in use" changes by
/// the host's answer (FOUND, 01.33, 0x18ed30).
pub const POD_COMPUTER_STARTUP_SOUND: &str = "pod/pod_comps_startup";
pub const POD_COMPUTER_SHUTDOWN_SOUND: &str = "pod/pod_comps_shutdown";

/// One change of [`LevelWorld::pod_computer_in_use`] by the host's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InUseChange {
    pub in_use: bool,
    /// `PWorld.Frame` of this world at the start of the tick that took the answer.
    pub frame: i32,
}

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
    /// The `PodComputer.RequestHostUsePod__b` calls that passed the game's test (our
    /// Pod, owned by the local user), oldest first, at most
    /// [`MAX_HOST_USE_POD_REQUESTS`]; the calls that did not pass are counted in
    /// `host_use_pod_refused`.
    pub host_use_pod_requests: Vec<HostUsePodRequest>,
    pub host_use_pod_refused: u64,
    /// The request the host has not answered yet
    /// ([`LevelWorld::begin_session_tick`]).
    pub host_use_pod_pending: Option<bool>,
    /// The answers that changed "in use" in this world, oldest first, at most
    /// [`MAX_HOST_USE_POD_REQUESTS`].
    pub in_use_changes: Vec<InUseChange>,
    /// The last GUID `PodComputer.SetPodTransitionTexture__g` was given, and the calls.
    /// Nothing is textured by it.
    pub transition_texture: Option<u32>,
    pub transition_texture_calls: u64,
    /// `Native.GetIsInPadZone__Q5Thing` calls with a Thing, and those that were true.
    pub pad_zone_calls: u64,
    pub pad_zone_hits: u64,
    /// `Input.IsJustClickedByID__iiQ6String` calls, and those that were true.
    pub action_click_calls: u64,
    pub action_clicks: u64,
    /// The calls with an action whose test was not read (answered false), by action.
    pub actions_not_read: BTreeMap<i32, u64>,
    /// The on-screen prompts those calls asked for, by the key of the prompt.
    pub action_prompts: BTreeMap<String, u64>,
    /// `Native.GetPlayerColour__Q5Thing` calls with a Thing that has a PYellowHead.
    pub player_colour_calls: u64,
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

    /// The number of the primary local player, if there is a player.
    ///
    /// FOUND (01.33, 0x240a88): the game picks one entry of the session's player list.
    /// INFERRED: with one local player that is him; here the first player record.
    pub fn primary_player(&self) -> Option<i32> {
        self.player_records.first().map(|r| r.number)
    }

    /// What `Native.GetIsInPadZone__Q5Thing` answers for a Thing: the player stands
    /// where he can engage the pod computer.
    ///
    /// FOUND (01.33, 0x13bac0): false unless the level is our Pod
    /// ([`LevelWorld::our_pod`]) and the Thing's PYellowHead has the primary player's
    /// number. Then the game walks a list of the PWorld; an entry counts when its
    /// Thing's script instance passes a class test and a call by name, and the Thing
    /// asked about is in the entry's list of Things inside. The first such entry
    /// decides: false when the Thing has no PPos, else his world position z is at least
    /// [`PAD_ZONE_MIN_Z`]. No entry: false. There is no distance test: the zone is the
    /// trigger's own.
    ///
    /// INFERRED: the list is the level's PTriggers in Thing order and the class is
    /// [`POD_TRIGGER_CLASS`]. The call by name is not made here (the class's one
    /// function of that kind returns true). The Things with a player number are the
    /// Sackboys [`LevelWorld::add_player`] made. The classes are known after
    /// [`LevelWorld::add_class_defaults`]; before it no trigger counts.
    pub fn in_pad_zone(&self, thing: u32) -> bool {
        let Some(player) = self.players.iter().find(|p| p.thing == thing) else {
            return false;
        };
        if !self.our_pod || Some(player.number) != self.primary_player() {
            return false;
        }
        let holds = |t: &crate::ThingState| {
            t.script_classes.iter().any(|c| c == POD_TRIGGER_CLASS)
                && t.trigger
                    .as_ref()
                    .is_some_and(|g| g.inside.contains(&thing))
        };
        if !self.things.iter().any(holds) {
            return false;
        }
        let matrix = self.things.get(thing as usize).and_then(|t| t.world_matrix);
        matrix.is_some_and(|m| m[14] >= PAD_ZONE_MIN_Z)
    }

    /// `PodComputer.RequestHostUsePod__b`.
    ///
    /// FOUND (01.33, 0x3fa974, 0x3fa9cc): only when the level is our Pod and the Pod
    /// belongs to the local user, the game stores the request (use or stop) with the
    /// time and sends a message to the host. "In use" does not change here: the
    /// receiver of the message sets it (0x3fa868).
    ///
    /// INFERRED: the one local player owns the Pod he starts and is the host, so only
    /// [`LevelWorld::our_pod`] is tested, and the host's answer is the request itself,
    /// taken at the start of the next tick ([`LevelWorld::begin_session_tick`]; the
    /// delay in the game was not measured). A second request before the answer
    /// replaces the first (the game keeps one request state).
    pub fn request_host_use_pod(&mut self, in_use: bool) {
        if !self.our_pod {
            self.pod.host_use_pod_refused += 1;
            return;
        }
        self.pod.host_use_pod_pending = Some(in_use);
        if self.pod.host_use_pod_requests.len() < MAX_HOST_USE_POD_REQUESTS {
            self.pod.host_use_pod_requests.push(HostUsePodRequest {
                in_use,
                frame: self.pworld.frame,
            });
        }
    }

    /// The start of a tick for the state outside the level: the host's answer to a
    /// pending [`LevelWorld::request_host_use_pod`] arrives. [`LevelScripts::tick_level`]
    /// calls it; [`tick_session`] calls it on every level before any of them ticks.
    ///
    /// FOUND (01.33, 0x3fa868 -> 0x18ed30): the receiver of the host's message sets "in
    /// use" to the message's value with sound, and clears the request. With sound, and
    /// when the primary player's avatar exists, a change to true plays
    /// [`POD_COMPUTER_STARTUP_SOUND`] and a change to false
    /// [`POD_COMPUTER_SHUTDOWN_SOUND`]. Nothing plays here: the sound is counted in
    /// [`LevelWorld::sounds_played`] of the world that has the avatar.
    /// Left out: on turning off the game also tells the pod computer object (0x402478).
    pub fn begin_session_tick(&mut self) {
        let Some(in_use) = self.pod.host_use_pod_pending.take() else {
            return;
        };
        if self.pod_computer_in_use == in_use {
            return;
        }
        self.pod_computer_in_use = in_use;
        if self.pod.in_use_changes.len() < MAX_HOST_USE_POD_REQUESTS {
            self.pod.in_use_changes.push(InUseChange {
                in_use,
                frame: self.pworld.frame,
            });
        }
        let primary = self.primary_player();
        if self.players.iter().any(|p| Some(p.number) == primary) {
            let sound = if in_use {
                POD_COMPUTER_STARTUP_SOUND
            } else {
                POD_COMPUTER_SHUTDOWN_SOUND
            };
            *self.sounds_played.entry(sound.to_owned()).or_default() += 1;
        }
    }

    /// Copies the state the game keeps outside a level from this world to `other`, a
    /// level played in the same session: the profile's progression flags, "our Pod",
    /// "pod computer in use" with a request that waits for its answer, the video and
    /// voice-over requests, and per player (records with the same number) the colour
    /// and the buttons that were used up.
    ///
    /// FOUND (01.33): "our Pod" and "in use" are two bytes of the one game object, the
    /// flags are the profile's, and the player records hang off the game object: none
    /// of them belongs to a level. The game holds the Pod and the planets level at once.
    /// The rest of a level world (Things, the camera a script wrote, the counts) is the
    /// level's own and is not copied.
    pub fn hand_session_to(&self, other: &mut LevelWorld) {
        other.progression_flags = self.progression_flags;
        other.our_pod = self.our_pod;
        other.pod_computer_in_use = self.pod_computer_in_use;
        other.pod.host_use_pod_pending = self.pod.host_use_pod_pending;
        other.media.clone_from(&self.media);
        for mine in &self.player_records {
            let theirs = other.player_records.iter_mut();
            let Some(theirs) = theirs.into_iter().find(|r| r.number == mine.number) else {
                continue;
            };
            theirs.colour = mine.colour;
            if let (Some(a), Some(b)) = (mine.pad, theirs.pad.as_mut()) {
                b.use_up(a.held_off);
            }
        }
    }
}

/// One level of a session ([`tick_session`]): its scripts, the VM they run on and its
/// world.
pub struct SessionLevel<'a> {
    pub scripts: &'a mut LevelScripts,
    pub vm: &'a mut Vm,
    pub host: &'a mut Bindings<LevelWorld>,
}

/// One tick of several levels that are loaded at once, as the game holds the Pod and
/// the planets level: every world takes the host's answer
/// ([`LevelWorld::begin_session_tick`]), then the levels tick in the order given, and
/// after each one its session state goes to all the others
/// ([`LevelWorld::hand_session_to`]). So a flag one level's script sets is seen by the
/// levels after it on the same tick and by the levels before it on the next.
///
/// Before the first tick the caller makes the worlds agree (`hand_session_to` once), and
/// again after it changes one world's session state itself between two ticks.
/// INFERRED: the order of the levels in the game's frame was not read. Each level has
/// its own VM here (a Thing handle is an index into one level world), so script
/// statics and the PodComputer script object are per level; the game has one of each.
pub fn tick_session(levels: &mut [SessionLevel<'_>], budget: u64) -> Vec<StepReport> {
    for level in levels.iter_mut() {
        level.host.world.begin_session_tick();
    }
    let mut reports = Vec::with_capacity(levels.len());
    for i in 0..levels.len() {
        let (before, rest) = levels.split_at_mut(i);
        let Some((level, after)) = rest.split_first_mut() else {
            break;
        };
        reports.push(
            level
                .scripts
                .tick_level_in_session(level.vm, level.host, budget),
        );
        for other in before.iter_mut().chain(after) {
            level.host.world.hand_session_to(&mut other.host.world);
        }
    }
    reports
}
