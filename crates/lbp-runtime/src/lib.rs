//! Rt: a level as the script VM's [`World`] (docs/runtime-research.md, item Rt;
//! docs/level-scripts.md). Separate from the parsing crates, which never depend on it.
//!
//! - [`LevelWorld`] builds the Thing tree from a read level: Thing handles are indices in
//!   the level's Thing list; each Thing with a PScript part gets its instance fields as
//!   script members (`GET/SET_SP_MEMBER` on that Thing), converted from the stored values.
//! - [`Runner`] runs a named function of each scripted Thing's class with the Thing as
//!   `this` in register 0 (the convention the test-script and Gooey hosts use, INFERRED for
//!   Thing scripts) and resumes yielded scripts once per tick.
//!
//! - [`LevelScripts`] is the per-tick schedule: [`CREATE`] once on every scripted Thing,
//!   then each tick resume suspended scripts and run [`UPDATE`] on the others.
//!
//! Entry names: `OnCreate__` and `Update__` are the create/update functions (PC D152:
//! across the 49 classes Rookie Test and the other levels use, `OnCreate__` is defined by
//! 24 and `Update__` by 11; the remaining names are UI, switch and enter/leave handlers).
use lbp_level::parts::{ScriptObject, ScriptPart, ScriptValue, ShapeColor, SwitchSignal};
use lbp_level::thing::ThingRecord;
use lbp_resource::translation::TranslationTable;
use lbp_script_vm::natives::{
    Bindings, CameraFromToUp, PadState, PlayerRecord, ResourceKind, SwitchInput, World, GS_GRAVITY,
    GS_LIGHTING, GS_WATER_BITS, GS_WATER_LEVEL, GS_WATER_MURKINESS, GS_WATER_TINT,
    GS_WATER_WAVE_MAGNITUDE, SCORE_BOARD_SCRIPT,
};
use lbp_script_vm::{ClassId, Host, MemberKind, ObjectId, Step, Task, Value, Vm};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

mod camera;
mod display;
mod global_settings;
mod media;
mod message_view;
mod messages;
mod pod;
mod sensors;
mod switches;
pub mod transform;
pub use camera::*;
pub use display::*;
pub use global_settings::*;
pub use media::*;
pub use message_view::*;
pub use messages::*;
pub use pod::*;
pub use switches::*;

/// One Thing as the runtime sees it.
#[derive(Debug, Clone, Default)]
pub struct ThingState {
    pub uid: Option<i32>,
    pub parent: Option<u32>,
    pub first_child: Option<u32>,
    pub next_sibling: Option<u32>,
    /// POS world matrix in stored order (translation at 12..14).
    pub world_matrix: Option<[f32; 16]>,
    /// POS local matrix, against the parent's world matrix. LBP2 level files do not store
    /// it; [`LevelWorld::derive_local_matrices`] works it out when the level loads.
    pub local_matrix: Option<[f32; 16]>,
    pub parts: Vec<&'static str>,
    /// GUID of the PScript resource, when the Thing has one.
    pub script_guid: Option<u32>,
    /// Names of the Thing's script class and its superclasses, once the class is loaded
    /// ([`LevelWorld::add_class_defaults`]); a script cast tests against them.
    pub script_classes: Vec<String>,
    /// Script members by field name; `None` when the Thing has no PScript layout.
    pub members: Option<BTreeMap<String, Value>>,
    /// PShape colour as a colour vector (`GetShapeEditorColour`).
    pub shape_colour: Option<[f32; 4]>,
    /// PShape colour tint a script set (`SetShapeEditorColourTint`); nothing draws it.
    pub shape_tint: Option<[f32; 4]>,
    /// Render-mesh editor colour tint a script set (`SetRenderMeshEditorColourTint`).
    pub render_mesh_tint: Option<[f32; 4]>,
    /// Animation playback rate a script set (`Native.SetAnimSpeed`, INFERRED multiplier).
    pub anim_speed: Option<f32>,
    /// Animation position a script set (`Native.SetAnimPos`, INFERRED time or fraction).
    pub anim_pos: Option<f32>,
    /// PTrigger, when the Thing has one.
    pub trigger: Option<TriggerState>,
    /// PSwitch, when the Thing has one.
    pub switch: Option<SwitchState>,
    /// The Thing's input table by port: the switch output wired to each port, `None`
    /// for a port nothing is wired to (see [`LevelWorld::wire_inputs`]).
    pub inputs: Vec<Option<InputWire>>,
    /// The game's "deactivated" byte (Thing+0x7f): while set, the switch pass forces the
    /// Thing's outputs off ([`LevelWorld::update_switch`]). FOUND: the game derives it
    /// from the microchip or circuit board holding the Thing
    /// ([`LevelWorld::recompute_deactivated`]).
    pub deactivated: bool,
    /// PMicrochip: the chip's circuit-board Thing. The components on the board are the
    /// board's children.
    pub circuit_board: Option<u32>,
    /// GUID of the mesh the Thing's PRenderMesh shows, when it has one with a GUID: the
    /// level file's until `Thing.SetRenderMeshMesh` sets another.
    pub mesh_guid: Option<u32>,
    /// The name of the Thing's PScriptName, when it has one: the bytes
    /// `Thing.FindThing` compares ([`LevelWorld::find_thing`]).
    pub script_name: Option<Vec<u8>>,
    /// The PRenderMesh's visibility byte, when the Thing has a PRenderMesh: the level
    /// file's until `Thing.HideRenderMesh` or `ShowRenderMesh` changes it
    /// ([`ThingDisplay::mesh_hidden`]).
    pub render_mesh_visibility: Option<u8>,
    /// The PRenderMesh's editor colour as a colour vector, when the Thing has a
    /// PRenderMesh: the level file's until `SetRenderMeshEditorColour` sets one.
    pub render_mesh_colour: Option<[f32; 4]>,
    /// Render-mesh editor texture a script set (`SetRenderMeshEditorTexture`), as passed
    /// (a resource handle, or null).
    pub render_mesh_texture: Option<Value>,
    /// Loop flag a script set (`Thing.SetAnimLoop__b`). The game sets it only on a Thing
    /// that has an animation; here it is kept on any Thing and nothing animates by it.
    pub anim_loop: Option<bool>,
    /// PSwitchKey, when the Thing has one: the tag a tag sensor finds it by.
    pub switch_key: Option<TagKey>,
}

/// What a tag sensor matches a tag by: the colour index and the name.
///
/// FOUND (01.33, the tag map lookup 0xceac0): an entry matches when the colour index and
/// the name handle are both the same. The handle is counted by reference, so equal
/// handles are equal strings (INFERRED); here `name` is the index of the string in
/// [`LevelWorld::tag_names`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagKey {
    pub colour: i32,
    pub name: u32,
}

/// One tag in the world's tag map ([`LevelWorld::refresh_tags`]): where it was when the
/// switch pass started, and its Thing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TagRecord {
    pub x: f32,
    pub y: f32,
    pub thing: u32,
}

/// A PSwitch at run time: the stored settings and its outputs. The switch pass
/// ([`LevelWorld::update_switch`]) rewrites the outputs' signals each tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchState {
    pub switch_type: i32,
    pub inverted: bool,
    pub behavior: i32,
    /// In stored order; `None` for an output the level file leaves null.
    pub outputs: Vec<Option<SwitchOutputState>>,
    /// ManualActivation as stored. The live value is [`SwitchMemory::manual`].
    pub manual_activation: SwitchInput,
    /// ActivationHoldTime as stored: a timer's length in ticks.
    pub activation_hold_time: i32,
    /// BulletsRequired: the port count of an AND or OR gate, a counter's target.
    pub bullets_required: i32,
    /// The AND / OR output mode (stored in the RandomOffTimeMin slot).
    pub gate_mode: i32,
    pub team_filter: i32,
    pub include_touching: i32,
    /// Radius and MinRadius: the outer and inner edge of a sensor's range.
    pub radius: f32,
    pub min_radius: f32,
    /// AngleRange: the half angle of a sensor's cone in degrees (180 = all round).
    pub angle_range: f32,
    /// The tag a tag sensor looks for.
    pub key: TagKey,
    /// KeySensorMode of a tag sensor: 1 hands on the tag's own signal.
    pub key_sensor_mode: i32,
    /// RequireAll of a player sensor.
    pub require_all: bool,
    pub detect_unspawned_players: bool,
    pub player_mode: i32,
    /// RetardedOldJoint: the output deadband of [`compute_output`].
    pub old_joint: bool,
    /// CrappyOldLbp1Switch as stored (census only; see `force`).
    pub old_lbp1_switch: bool,
    /// Re-fire flag (PSwitch+0xe3): the next update hands `true` to the targets of every
    /// output that is on, changed or not, and clears the flag. What sets it in the game
    /// is not established; nothing sets it here.
    pub force: bool,
    /// What the switch remembers between ticks ([`compute_output`]'s state slot).
    pub memory: SwitchMemory,
}

/// One output of a PSwitch: its signal and the (Thing, port) inputs it is wired to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchOutputState {
    /// The current signal, as the switch-input natives read it.
    pub signal: SwitchInput,
    /// The signal the level file stores.
    pub stored: SwitchInput,
    pub targets: Vec<(Option<u32>, i32)>,
}

/// What is wired to one input port: output `output` of the PSwitch on Thing `switch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputWire {
    pub switch: u32,
    pub output: usize,
}

/// How many chips inside chips [`LevelWorld::microchip_of`] follows outwards.
pub const MAX_CHIP_NESTING: usize = 64;

/// Input ports past this are not wired (a guard against a damaged file; the count of
/// targets it drops is [`LevelWorld::input_ports_dropped`]).
pub const MAX_INPUT_PORTS: usize = 1024;

/// A PTrigger at run time: the stored settings and the Things inside it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriggerState {
    /// 0 circle, 1 box, 4 sphere (see [`LevelWorld::update_triggers`]).
    pub trigger_type: i8,
    pub radius: f32,
    pub hysteresis: f32,
    pub all_z_layers: bool,
    pub enabled: bool,
    pub z_offset: f32,
    /// Things inside, in the order they entered (the stored list first).
    pub inside: Vec<u32>,
}

/// A player's Sackboy as a Thing of the level (see [`LevelWorld::add_player`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerState {
    pub thing: u32,
    /// His `PYellowHead.PlayerNumber`, the number of his [`PlayerRecord`].
    pub number: i32,
    /// Nearest and farthest z his body covers (the layers he stands in).
    pub layers: (f32, f32),
}

/// What a trigger did to a Thing this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TriggerEventKind {
    Enter,
    Inside,
    Leave,
}

impl TriggerEventKind {
    /// The script function the game calls for the event.
    pub fn function(self) -> &'static str {
        match self {
            Self::Enter => "Enter__Q5Thing",
            Self::Inside => "Inside__Q5Thing",
            Self::Leave => "Leave__Q5Thing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerEvent {
    /// The Thing with the PTrigger.
    pub trigger: u32,
    pub kind: TriggerEventKind,
    /// The Thing that entered, is inside or left.
    pub thing: u32,
}

/// `p` in the space of the stored-order matrix `m` (translation at 12..14, columns the
/// axes), or `None` when `m` cannot be inverted.
fn to_local(m: &[f32; 16], p: [f32; 3]) -> Option<[f32; 3]> {
    let (a, b, c) = (&m[0..3], &m[4..7], &m[8..11]);
    let cross = |u: &[f32], v: &[f32]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let dot = |u: &[f32], v: &[f32]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let (bc, ca, ab) = (cross(b, c), cross(c, a), cross(a, b));
    let det = dot(a, &bc);
    if det == 0. || !det.is_finite() {
        return None;
    }
    let d = [p[0] - m[12], p[1] - m[13], p[2] - m[14]];
    Some([dot(&bc, &d) / det, dot(&ca, &d) / det, dot(&ab, &d) / det])
}

/// An engine part a script reached through a native member (`thing.SpriteLight`), with
/// the `ref_ptr` members scripts wrote to it.
#[derive(Debug, Clone, Default)]
pub struct PartState {
    pub thing: u32,
    /// The reader's part name (`SPRITE_LIGHT`).
    pub part: &'static str,
    pub members: BTreeMap<String, Value>,
}

/// Packed PShape colour (head > 0x389) as a colour vector. INFERRED byte order
/// 0xAARRGGBB (white 0xffffffff is the common value; nothing yet tells R from B).
pub fn unpack_colour(c: u32) -> [f32; 4] {
    let ch = |shift: u32| ((c >> shift) & 0xff) as f32 / 255.;
    [ch(16), ch(8), ch(0), ch(24)]
}

/// A read level as the VM's world.
#[derive(Debug, Default)]
pub struct LevelWorld {
    pub things: Vec<ThingState>,
    pub world: Option<u32>,
    /// `WRITE` output of the scripts.
    pub written: Vec<String>,
    /// Stored non-null object references with no VM value: before [`Self::bind_objects`]
    /// every member that is not a string; after it, references (members, array elements,
    /// instance fields) whose object could not be built (resource, audio handle, instance
    /// of a class the VM has not loaded).
    pub unconverted: usize,
    /// Script object contents by the level's object number (first occurrence).
    pub stored_objects: BTreeMap<usize, ScriptObject>,
    /// Members waiting for a VM object: (Thing, field, object number).
    pub object_members: Vec<(u32, String, usize)>,
    /// Parts handed to scripts; a part handle is its index + 1 (`Raw(0)` is null).
    pub part_handles: Vec<PartState>,
    pub camera: CameraState,
    /// The PodComputer script object `GetPodComputer__` returns (see [`add_engine_objects`]).
    pub pod_computer: Option<ObjectId>,
    /// Gooey objects made ahead for `CreateGooey__i` (the host cannot allocate VM objects
    /// during a native call); each call hands out the next one.
    pub gooey_pool: Vec<ObjectId>,
    pub gooeys_created: usize,
    /// Interactive-music commands scripts sent, by name.
    pub audio_commands: BTreeMap<String, u64>,
    /// The profile's word of LBP2 game progression flags: one bit a flag. FOUND (01.33,
    /// 0x15d22c, 0x15d2a8): `Profile.SetLBP2GameProgressionFlag__i` ORs its argument
    /// into the word and `IsLBP2GameProgressionFlagSet__i` is true when every bit of its
    /// argument is set. 0 = a fresh profile, on which the planets level's world script
    /// starts its intro thread; the thread sets the mask 3 when it ends.
    pub progression_flags: u32,
    /// The video and voice-over requests of the scripts ([`MediaState`]). Nothing plays.
    pub media: MediaState,
    /// `PodComputer.GetPodComputerInUse__`: a player is at the pod computer ("PuterInUse").
    /// False by default (INFERRED: nobody at the computer when the Pod loads).
    pub pod_computer_in_use: bool,
    /// The Planets object `PodComputer.GetPlanets__` returns.
    pub planets: Option<ObjectId>,
    /// Planet objects by `GetCrafteroid__i` index - 1, with their Thing.
    pub crafteroids: Vec<(ObjectId, Option<u32>)>,
    /// The game state's player records, in join order (`Game.GetNumPlayers__` is their
    /// count). A level read with [`LevelWorld::from_things`] starts with the record of
    /// one local player, number 0, who has no Sackboy yet; [`LevelWorld::add_player`]
    /// gives the next record a Sackboy and makes the record when it is missing.
    ///
    /// INFERRED: one local player whose `Game.EPlayerNumber` is 0, there from the start
    /// (in the game a level is not played without a player, and his record is there
    /// before his Sackboy is spawned); his team starts at 0 (the start value was not
    /// read; the world script sets every player's team each tick).
    pub player_records: Vec<PlayerRecord>,
    /// Players' Sackboys ([`LevelWorld::add_player`]); none until a caller adds one.
    pub players: Vec<PlayerState>,
    /// Switch targets whose port already had an output wired (the first one is kept).
    pub input_collisions: usize,
    /// Switch targets with a negative port or one at or past [`MAX_INPUT_PORTS`].
    pub input_ports_dropped: usize,
    /// Script GUIDs of the loaded classes that are, or derive from, the scoreboard class
    /// (filled by [`LevelWorld::add_class_defaults`]).
    pub score_board_scripts: BTreeSet<u32>,
    /// The switch Things in pass order, once computed ([`LevelWorld::switch_order`]).
    pub switch_order: Option<Vec<u32>>,
    /// A test drive for the switch pass: while `Some(on)`, every output's computed signal
    /// is replaced by ternary 1 and activation 1.0 (`on`) or ternary 0 and activation 0.0,
    /// with the stored player. The deactivated forcing still applies. Not a game feature.
    pub switch_pulse: Option<bool>,
    /// Set once the deactivated flags have their start values
    /// ([`LevelWorld::prepare_switches`]).
    pub switches_prepared: bool,
    /// The level's PWorld values scripts read ([`WorldState`]).
    pub pworld: WorldState,
    /// The owner of each circuit board: board Thing -> the Thing whose PMicrochip names it
    /// (the first in Thing order). See [`LevelWorld::microchip_of`].
    pub board_owners: BTreeMap<u32, u32>,
    /// `Audio.PlaySound__Q5ThingQ7StringA` requests by sound name; nothing plays.
    pub sounds_played: BTreeMap<String, u64>,
    /// Calls the world only records ([`World::engine_call`]), by name.
    pub engine_calls: BTreeMap<&'static str, u64>,
    /// `Game.GotoScoreboard__b` requests, with the flag passed. The game ends the level
    /// on it; nothing here does.
    pub scoreboard_requests: Vec<bool>,
    /// The script SlotID objects `Game.GetLevelSlotID__` and `GetPrimaryLevelSlotID__`
    /// hand out (see [`add_engine_objects`]).
    pub level_slot: Option<ObjectId>,
    pub primary_level_slot: Option<ObjectId>,
    /// The SlotID object (0, 0) that `Slot.GetSlotGroup__Q6SlotID` hands out for a slot
    /// the game's slot database does not have (see [`add_engine_objects`]).
    pub no_group_slot: Option<ObjectId>,
    /// Set by [`LevelWorld::add_class_defaults`]: the Things' class names are known, so
    /// a script cast is tested against them. Before that every cast passes.
    pub classes_resolved: bool,
    /// `Game.IsTutorialLevel__`. False unless the caller sets it (INFERRED: which levels
    /// the game sets it for is not established).
    pub tutorial_level: bool,
    /// Type and SlotNumber of the level's slot and of the primary slot: what the two
    /// SlotID objects are made with and what the natives that read a SlotID answer from.
    /// INFERRED: (0, 0), an empty slot (see [`add_engine_objects`]).
    pub level_slot_id: (i32, i32),
    pub primary_level_slot_id: (i32, i32),
    /// The translation tables that are loaded, in the order they are searched
    /// (`LAMS.IsInLams__i`, `LAMS.Translate__i`). Empty unless the caller hands tables
    /// in: then no key is in LAMS. Which tables the game has loaded while a level plays
    /// was not read.
    pub translations: Vec<Arc<TranslationTable>>,
    /// The recorded calls of [`KEPT_REQUESTS`] with their arguments, oldest first, for a
    /// later stage to show (speech bubbles, prompts, audio settings). At most
    /// [`MAX_REQUESTS`]; the calls past that are counted in `requests_dropped`.
    pub requests: Vec<EngineRequest>,
    pub requests_dropped: u64,
    /// The speech bubbles, subtitles and prompts that are open, by the game's rules for
    /// its message list ([`Messages`]); the messaging natives feed it.
    pub messages: Messages,
    /// The global settings in force when the level was read, before any script ran
    /// ([`WorldState::settings_in_force`] of the stored values).
    pub settings_at_load: SettingsInForce,
    /// Whether the first global-settings pass has run
    /// ([`LevelWorld::global_settings_holder`]).
    pub global_settings_started: bool,
    /// The Things whose visibility, colours or world matrix changed, for a display
    /// ([`LevelWorld::take_display_dirty`]).
    pub display: DisplayState,
    /// What the Pod's world script asks for and sets ([`PodState`]).
    pub pod: PodState,
    /// The names of the level's tags and tag sensors; a [`TagKey`] holds an index.
    pub tag_names: Vec<String>,
    /// The world's tag map as the last switch pass left it
    /// ([`LevelWorld::refresh_tags`]): per key, the tags by ascending x.
    pub tags: BTreeMap<TagKey, Vec<TagRecord>>,
}

/// One recorded call kept with its arguments ([`LevelWorld::requests`]).
#[derive(Debug, Clone, PartialEq)]
pub struct EngineRequest {
    /// The name [`World::engine_call`] was given.
    pub name: &'static str,
    /// The Thing the call is about (a bubble, a level link), when it has one.
    pub thing: Option<u32>,
    /// The native's parameters as passed.
    pub args: Vec<Value>,
    /// `PWorld.Frame` when the call was made.
    pub frame: i32,
}

/// The recorded calls whose arguments are kept in [`LevelWorld::requests`].
pub const KEPT_REQUESTS: [&str; 14] = [
    "Messaging_NewTriggerMessage",
    "Messaging_NewTriggerMessageNoPrompt",
    "Messaging_NewTriggerMessageSubtitles",
    "Messaging_NewLevelLinkPrompt",
    "Messaging_NewWormHolePrompt",
    "Messaging_NewGameEvent",
    "RegisterOverlayCallback",
    "TrackSpeechKey",
    "NotifyNpcsOfMouthStart",
    "SetGlobalSettingsAudioTo",
    "Messaging_NewTriggerMessageCaption",
    "Messaging_NewTriggerMessageCaptionNoPrompt",
    "Messaging_NewTriggerMessageThink",
    "Messaging_NewTriggerMessageThinkNoPrompt",
];

/// Most requests [`LevelWorld::requests`] keeps.
pub const MAX_REQUESTS: usize = 4096;

/// A global setting as the PWorld keeps it: a start value, a target and a blend factor.
/// FOUND (01.33, the setters 0xce608, 0xce654, 0xce6a0, 0xcef3c, 0xd51b0 and the getters
/// 0x5ec6f8, 0x5ec4f8, 0x5ec340, 0x5ec25c).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blend {
    pub from: f32,
    pub to: f32,
    pub t: f32,
}

impl Blend {
    /// A setting at rest on `value`: start = target = value, t = 1.0.
    pub fn at(value: f32) -> Self {
        Blend {
            from: value,
            to: value,
            t: 1.,
        }
    }
    /// The value in force: from + (to - from) * t.
    pub fn value(&self) -> f32 {
        (self.to - self.from) * self.t + self.from
    }
    /// `SetGlobalSettings<X>To(value, blend)`. With blend the start becomes the value in
    /// force and t 0, so the setting stays where it is until t is raised; without blend
    /// the setting jumps to `value`.
    pub fn set_to(&mut self, value: f32, blend: bool) {
        *self = if blend {
            Blend {
                from: self.value(),
                to: value,
                t: 0.,
            }
        } else {
            Blend::at(value)
        };
    }
}

/// `Game.GetGlobalSettingsWaterLevel__` gives the blended level only above this, else
/// 0.0. FOUND (01.33, 0xce438: the constant the level is compared with, 1.0e-4).
pub const WATER_LEVEL_MIN: f32 = 0.0001;

/// Slots of the PWorld global-settings arrays (FOUND: the natives index two arrays of
/// 8 x s32, PWorld+0x5d8 and +0x5f8).
pub const GLOBAL_SETTINGS: usize = 8;

/// PWorld state scripts read through `World.World` members, with the level file's values
/// at the start.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldState {
    /// `PWorld.GameMode` (declared `Game.EGameMode`, an s32): 0 normal, 1 competitive,
    /// 2 cut scene, 3 single player, 4 social, 5 teams (Toolkit `GameMode`).
    pub game_mode: i32,
    /// `PWorld.Frame`: the world's frame counter ([`LevelWorld::advance_frame`]).
    pub frame: i32,
    /// PWorld `simFrame`, advanced with `frame`.
    pub sim_frame: i32,
    /// PWorld `isPaused` as stored. Nothing reads it: see `Game.IsVcrPaused__` in
    /// [`LevelWorld::state_query`].
    pub is_paused: bool,
    /// `PWorld.WaterLevel`.
    pub water_level: f32,
    /// PWorld `GlobalSettingsThingUIDs` and `GlobalSettingsThingPriority`, one entry per
    /// global-settings type: the stored values at the start (padded with zeros to
    /// [`GLOBAL_SETTINGS`] entries), then what `Game.SetGlobalSettingUID` makes of them.
    pub global_settings_thing_uids: Vec<i32>,
    pub global_settings_thing_priority: Vec<i32>,
    /// D263: the one-float global settings (`Game.EGlobalSettingsType` 0, 1, 2, 4, 6).
    /// INFERRED start: each at rest on the value the level file stores (start = target =
    /// stored, t = 1.0); the stored start, target and blend factor are not read.
    pub water: Blend,
    pub gravity: Blend,
    pub water_wave_magnitude: Blend,
    pub water_murkiness: Blend,
    pub water_bits: Blend,
    /// Type 3, the water tint: one value (FOUND: no start and target pair) and its blend
    /// factor, which only `SetGlobalSettingsBlendFactor` writes here.
    pub water_tint: f32,
    pub water_tint_blend: f32,
    /// Type 5, the lighting group, in stored order: lighting, colour correction, fog,
    /// fog tint, darkness. Start values: the level file's `from` and `target` settings
    /// and the blend factor stored after them (INFERRED that this is the group's t).
    pub lighting_from: [f32; 5],
    pub lighting_to: [f32; 5],
    pub lighting_blend: f32,
    /// The five current values the level file stores next to the group (PWorld
    /// `lighting`, `colourCorrection`, `fog`, `fogTint`, `darkness`), in the same order.
    /// Nothing here reads them; a display that lit the level from them at load can
    /// compare them with [`WorldState::lighting`].
    pub lighting_stored: [f32; 5],
    /// Type 7, audio: only the blend factor lives in the PWorld.
    pub audio_blend: f32,
    /// PWorld `waterDrainSoundsEnabled`.
    pub water_drain_sounds: bool,
}

impl WorldState {
    /// The lighting group in force, in stored order: from + (to - from) * t per value.
    /// FOUND that the game takes "the blend in force" as the new start (0xd978c);
    /// INFERRED that it is this per-value blend.
    pub fn lighting(&self) -> [f32; 5] {
        std::array::from_fn(|i| {
            (self.lighting_to[i] - self.lighting_from[i]) * self.lighting_blend
                + self.lighting_from[i]
        })
    }
}

/// What [`LevelWorld::global_settings_holder`] found in one global-settings slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotHolder {
    /// No UID in the slot (or no PWorld).
    Empty,
    /// The slot held a UID of no Thing, or of a Thing without a script, and was cleared.
    Cleared,
    /// The scripted Thing that holds the slot.
    Thing(u32),
}

/// VM value for a stored script field; `None` for a field the instance does not store.
/// Object references other than strings with their text at hand become null and are
/// counted by the caller ([`LevelWorld::bind_objects`] builds them).
pub fn member_value(v: &ScriptValue) -> Option<(Value, bool)> {
    stored_value(v, &|_| None)
}

/// [`member_value`] with `object` giving the VM value for an object number.
fn stored_value(v: &ScriptValue, object: &dyn Fn(usize) -> Option<Value>) -> Option<(Value, bool)> {
    Some(match v {
        ScriptValue::Bool(b) => (Value::Bool(*b), true),
        ScriptValue::Char(c) => (Value::Char(*c as u16), true),
        ScriptValue::Int(i) => (Value::Int(*i), true),
        ScriptValue::Float(f) => (Value::Float(*f), true),
        ScriptValue::V4(v) => (Value::Vec4(*v), true),
        ScriptValue::M44(m) => (Value::Mat44(*m), true),
        ScriptValue::Thing(t) => (Value::Thing(t.and_then(|i| u32::try_from(i).ok())), true),
        ScriptValue::Object { kind: 0, .. } => (Value::Object(None), true),
        ScriptValue::Object { id, body, .. } => match (body.as_deref(), id.and_then(object)) {
            (Some(ScriptObject::Text(t)), _) => (Value::Str(t.as_str().into()), true),
            (_, Some(v)) => (v, true),
            _ => (Value::Object(None), false),
        },
        ScriptValue::NotStored => return None,
    })
}

/// Object contents by number, from a stored value and everything nested in it.
fn collect_objects(v: &ScriptValue, out: &mut BTreeMap<usize, ScriptObject>) {
    let ScriptValue::Object {
        id: Some(id),
        body: Some(body),
        ..
    } = v
    else {
        return;
    };
    match &**body {
        ScriptObject::Objects(items) => items.iter().for_each(|i| collect_objects(i, out)),
        ScriptObject::Instance(p) => p.fields.iter().for_each(|f| collect_objects(&f.value, out)),
        _ => {}
    }
    out.entry(*id).or_insert_with(|| (**body).clone());
}

fn signal(s: &SwitchSignal) -> SwitchInput {
    SwitchInput {
        activation: s.activation,
        ternary: s.ternary,
        player: s.player,
    }
}

/// The reader's part name for a script-side one: `"PSpriteLight"` (the `HasP...` natives)
/// or `"SpriteLight"` (a native member, `thing.SpriteLight`) -> `"SPRITE_LIGHT"`. `None`
/// when the name is no part.
///
/// The names are compared without case and underscores, so the reader names that do not
/// split at a capital match too (`MetaData`, `MicroChip`, `WormHole`, `YellowHead`). The
/// script side spells `TagSynchroniser` with an s. The whole name is tried first and the
/// leading `P` dropped only when that fails: `Pos`, `PhysicsTweak`, `PoppetPowerup`,
/// `PocketItem`, `PowerUp` and `Paint` keep their own `P`.
fn part_name(native: &str) -> Option<&'static str> {
    let find = |name: &str| {
        let want = name
            .to_ascii_uppercase()
            .replace('_', "")
            .replace("SYNCHRONISER", "SYNCHRONIZER");
        lbp_level::thing::PARTS
            .iter()
            .map(|p| p.0)
            .find(|p| p.replace('_', "") == want)
    };
    find(native).or_else(|| find(native.strip_prefix('P')?))
}

impl LevelWorld {
    pub fn from_things(list: &[ThingRecord]) -> Self {
        let handle = |i: usize| u32::try_from(i).ok();
        // Equal names get one index, as the game's name handles.
        let mut tag_names = Vec::<String>::new();
        let mut names = BTreeMap::<String, u32>::new();
        let mut key = |colour: i32, name: &str| {
            let name = match names.get(name) {
                Some(&i) => i,
                None => {
                    let i = tag_names.len() as u32;
                    names.insert(name.to_owned(), i);
                    tag_names.push(name.to_owned());
                    i
                }
            };
            TagKey { colour, name }
        };
        let mut w = LevelWorld {
            things: list
                .iter()
                .map(|t| ThingState {
                    uid: t.uid,
                    parent: t.parent.and_then(handle),
                    world_matrix: t.data.pos.as_ref().map(|p| p.world),
                    parts: t.parts.clone(),
                    shape_colour: t.data.shape.as_ref().map(|s| match s.color {
                        ShapeColor::Packed(c) => unpack_colour(c),
                        ShapeColor::Vector(v) => v,
                    }),
                    trigger: t.data.trigger.as_ref().map(|g| TriggerState {
                        trigger_type: g.trigger_type,
                        radius: g.radius_multiplier,
                        hysteresis: g.hysteresis_multiplier,
                        all_z_layers: g.all_z_layers,
                        enabled: g.enabled,
                        z_offset: g.z_offset,
                        inside: g.in_things.iter().filter_map(|i| handle((*i)?)).collect(),
                    }),
                    switch: t.data.switch.as_ref().map(|s| SwitchState {
                        switch_type: s.switch_type,
                        inverted: s.inverted,
                        behavior: s.behavior,
                        outputs: s
                            .outputs
                            .iter()
                            .map(|o| {
                                o.as_ref().map(|o| SwitchOutputState {
                                    signal: signal(&o.signal),
                                    stored: signal(&o.signal),
                                    targets: o
                                        .targets
                                        .iter()
                                        .map(|&(t, port)| (t.and_then(handle), port))
                                        .collect(),
                                })
                            })
                            .collect(),
                        manual_activation: signal(&s.manual_activation),
                        activation_hold_time: s.activation_hold_time,
                        bullets_required: s.bullets_required,
                        gate_mode: s.random_off_time_min,
                        team_filter: s.team_filter,
                        include_touching: s.include_touching,
                        radius: s.radius,
                        min_radius: s.min_radius,
                        angle_range: s.angle_range,
                        key: key(s.colour_index, &s.name),
                        key_sensor_mode: s.key_sensor_mode,
                        require_all: s.require_all,
                        detect_unspawned_players: s.detect_unspawned_players,
                        player_mode: i32::from(s.player_mode),
                        old_joint: s.retarded_old_joint,
                        old_lbp1_switch: s.crappy_old_lbp1_switch,
                        force: false,
                        memory: SwitchMemory {
                            updates: 0,
                            manual: signal(&s.manual_activation),
                            count: s.bullets_detected,
                            timer: s.timer_count,
                        },
                    }),
                    circuit_board: t
                        .data
                        .microchip
                        .and_then(|m| m.circuit_board)
                        .and_then(handle),
                    mesh_guid: t
                        .data
                        .render_mesh
                        .as_ref()
                        .and_then(|m| m.mesh.as_ref()?.guid),
                    render_mesh_visibility: t
                        .data
                        .render_mesh
                        .as_ref()
                        .map(|m| m.visibility_flags as u8),
                    render_mesh_colour: t
                        .data
                        .render_mesh
                        .as_ref()
                        .map(|m| unpack_colour(m.editor_color)),
                    switch_key: t
                        .data
                        .switch_key
                        .as_ref()
                        .map(|k| key(k.colour_index, &k.name)),
                    script_name: t.data.script_name.as_ref().map(|n| n.name.clone()),
                    ..Default::default()
                })
                .collect(),
            player_records: vec![PlayerRecord::default()],
            ..Default::default()
        };
        w.tag_names = tag_names;
        // Children in Thing-list order.
        let mut last_child: Vec<Option<u32>> = vec![None; list.len()];
        for i in 0..list.len() {
            let (Some(me), Some(p)) = (handle(i), w.things[i].parent) else {
                continue;
            };
            let Some(pi) = usize::try_from(p).ok().filter(|&p| p < list.len()) else {
                continue;
            };
            match last_child[pi] {
                None => w.things[pi].first_child = Some(me),
                Some(prev) => w.things[prev as usize].next_sibling = Some(me),
            }
            last_child[pi] = Some(me);
        }
        w.world = list
            .iter()
            .position(|t| t.data.world.is_some())
            .and_then(handle);
        if let Some(p) = list.iter().find_map(|t| t.data.world.as_ref()) {
            w.pworld = WorldState {
                game_mode: p.game_mode,
                frame: p.frame,
                sim_frame: p.sim_frame,
                is_paused: p.is_paused,
                water_level: p.water_level,
                global_settings_thing_uids: p.global_settings_thing_uids.clone(),
                global_settings_thing_priority: p.global_settings_thing_priority.clone(),
                water: Blend::at(p.water_level),
                gravity: Blend::at(p.gravity),
                water_wave_magnitude: Blend::at(p.water_wave_magnitude),
                water_murkiness: Blend::at(p.water_murkiness),
                water_bits: Blend::at(p.water_bits),
                water_tint: p.water_tint,
                water_tint_blend: 1.,
                lighting_from: p.gs_from,
                lighting_to: p.gs_target,
                lighting_blend: p.gs_blend,
                lighting_stored: p.lighting,
                audio_blend: 1.,
                water_drain_sounds: p.water_drain_sounds_enabled,
            };
            for slots in [
                &mut w.pworld.global_settings_thing_uids,
                &mut w.pworld.global_settings_thing_priority,
            ] {
                if slots.len() < GLOBAL_SETTINGS {
                    slots.resize(GLOBAL_SETTINGS, 0);
                }
            }
        }
        w.settings_at_load = w.pworld.settings_in_force();
        for (i, t) in w.things.iter().enumerate() {
            if let Some(board) = t.circuit_board {
                w.board_owners.entry(board).or_insert(i as u32);
            }
        }
        w.derive_local_matrices();
        for t in list {
            if let Some(part) = &t.data.script {
                for f in &part.fields {
                    collect_objects(&f.value, &mut w.stored_objects);
                }
            }
        }
        for (i, t) in list.iter().enumerate() {
            if let Some(part) = &t.data.script {
                w.add_script(i, part);
            }
        }
        w.wire_inputs();
        w
    }

    /// Builds every Thing's input table from the PSwitch outputs' targets.
    ///
    /// FOUND in the 01.33 executable (the switch-input natives' readers, 0x7541e0 and
    /// its neighbours): a Thing holds a table with one entry per input port, each empty
    /// or the SwitchOutput wired to it. INFERRED: the table is what the outputs' stored
    /// targets say. A target (Thing, port) puts its output on that port of that Thing;
    /// the table is as long as the highest wired port + 1. When two outputs target one
    /// port the first in Thing order (then output and target order) is kept and the
    /// other is counted in `input_collisions`. Targets with a null Thing wire nothing.
    pub fn wire_inputs(&mut self) {
        let mut wires = Vec::new();
        for (i, t) in self.things.iter().enumerate() {
            let outputs = t.switch.iter().flat_map(|s| s.outputs.iter().enumerate());
            for (output, o) in outputs {
                for &(target, port) in o.iter().flat_map(|o| &o.targets) {
                    let wire = InputWire {
                        switch: i as u32,
                        output,
                    };
                    wires.push((target, port, wire));
                }
            }
        }
        for t in &mut self.things {
            t.inputs.clear();
        }
        (self.input_collisions, self.input_ports_dropped) = (0, 0);
        self.switch_order = None;
        for (target, port, wire) in wires {
            let Some(t) = target.and_then(|t| self.things.get_mut(t as usize)) else {
                continue;
            };
            let Some(port) = usize::try_from(port).ok().filter(|&p| p < MAX_INPUT_PORTS) else {
                self.input_ports_dropped += 1;
                continue;
            };
            if t.inputs.len() <= port {
                t.inputs.resize(port + 1, None);
            }
            match &mut t.inputs[port] {
                Some(_) => self.input_collisions += 1,
                empty => *empty = Some(wire),
            }
        }
    }

    /// The wire on input port `port` of a Thing, with the output it leads to.
    fn input(
        &self,
        thing: u32,
        port: i32,
    ) -> Result<Option<(InputWire, &SwitchOutputState)>, String> {
        let inputs = &self.state(thing)?.inputs;
        let wire = usize::try_from(port).ok().and_then(|p| *inputs.get(p)?);
        Ok(wire.and_then(|w| {
            let s = self.things.get(w.switch as usize)?.switch.as_ref()?;
            Some((w, s.outputs.get(w.output)?.as_ref()?))
        }))
    }

    fn add_script(&mut self, i: usize, part: &ScriptPart) {
        let stored = &self.stored_objects;
        let text = |id: usize| match stored.get(&id) {
            Some(ScriptObject::Text(t)) => Some(Value::Str(t.as_str().into())),
            _ => None,
        };
        let mut members = BTreeMap::new();
        for f in &part.fields {
            if let Some((v, converted)) = stored_value(&f.value, &text) {
                if !converted {
                    self.unconverted += 1;
                    if let ScriptValue::Object { id: Some(id), .. } = f.value {
                        self.object_members.push((i as u32, f.name.clone(), id));
                    }
                }
                members.insert(f.name.clone(), v);
            }
        }
        let state = &mut self.things[i];
        state.script_guid = part.script.and_then(|d| d.guid);
        if !part.fields.is_empty() {
            state.members = Some(members);
        }
    }

    /// Builds the level's stored script objects on the VM heap and points the members that
    /// reference them at those objects. Arrays become script arrays; an instance becomes
    /// an instance of the class with its script GUID (class defaults, then the stored
    /// fields); strings stay string values. Objects with one number are one heap object, so
    /// shared references stay shared. Resources, audio handles and instances of classes
    /// the VM has not loaded stay null and are counted in `unconverted`. Returns the
    /// number of heap objects created.
    pub fn bind_objects(&mut self, vm: &mut Vm) -> usize {
        let thing = |t: &Option<usize>| Value::Thing(t.and_then(|i| u32::try_from(i).ok()));
        let mut built = BTreeMap::<usize, Value>::new();
        let mut created = 0;
        // Allocate first so references in any order (and cycles) resolve.
        for (&id, body) in &self.stored_objects {
            let made = match body {
                ScriptObject::Text(t) => {
                    built.insert(id, Value::Str(t.as_str().into()));
                    continue;
                }
                ScriptObject::Bools(v) => vm.new_array(v.iter().map(|&b| Value::Bool(b)).collect()),
                ScriptObject::Ints(v) => vm.new_array(v.iter().map(|&i| Value::Int(i)).collect()),
                ScriptObject::Floats(v) => {
                    vm.new_array(v.iter().map(|&f| Value::Float(f)).collect())
                }
                ScriptObject::Vectors(v) => {
                    vm.new_array(v.iter().map(|&f| Value::Vec4(f)).collect())
                }
                ScriptObject::Things(v) => vm.new_array(v.iter().map(thing).collect()),
                ScriptObject::Objects(_) => vm.new_array(Vec::new()),
                ScriptObject::Instance(part) => {
                    match part
                        .script
                        .and_then(|d| d.guid)
                        .and_then(|g| vm.class_by_guid(g))
                    {
                        Some(class) => vm.instantiate(class),
                        None => continue,
                    }
                }
                ScriptObject::Resource { .. } | ScriptObject::AudioHandle => continue,
            };
            if let Ok(o) = made {
                built.insert(id, Value::Object(Some(o)));
                created += 1;
            }
        }
        let object = |i: usize| built.get(&i).cloned();
        let mut unconverted = 0;
        for (id, body) in &self.stored_objects {
            let Some(Value::Object(Some(o))) = built.get(id) else {
                continue;
            };
            match body {
                ScriptObject::Objects(items) => {
                    let values: Vec<Value> = items
                        .iter()
                        .map(|v| {
                            let (v, ok) =
                                stored_value(v, &object).unwrap_or((Value::Object(None), true));
                            unconverted += usize::from(!ok);
                            v
                        })
                        .collect();
                    if let Some(a) = vm.array_mut(*o) {
                        *a = values;
                    }
                }
                ScriptObject::Instance(part) => {
                    let mut stored = Vec::new();
                    for f in &part.fields {
                        if let Some((v, ok)) = stored_value(&f.value, &object) {
                            unconverted += usize::from(!ok);
                            stored.push((f.name.clone(), v));
                        }
                    }
                    if let Some(fields) = vm.fields_mut(*o) {
                        fields.extend(stored);
                    }
                }
                _ => {}
            }
        }
        for (t, name, id) in std::mem::take(&mut self.object_members) {
            match object(id) {
                Some(v) => {
                    if let Some(m) = self.things[t as usize].members.as_mut() {
                        m.insert(name, v);
                    }
                }
                None => unconverted += 1,
            }
        }
        self.unconverted = unconverted;
        created
    }

    /// Gives every scripted Thing whose class is loaded the fields of its class and
    /// superclasses that the instance does not store, at their type defaults. The stored
    /// layout only carries serialized fields: a superclass field such as
    /// `Tweakable.IsTweaking` (read by `Trigger_Speech_Bubble.Update`) is never stored but
    /// still exists on the instance (INFERRED, the way `instantiate` builds script objects).
    pub fn add_class_defaults(&mut self, vm: &Vm) {
        self.classes_resolved = true;
        for t in &mut self.things {
            let Some((guid, class)) = t.script_guid.and_then(|g| Some((g, vm.class_by_guid(g)?)))
            else {
                continue;
            };
            let chain = vm.class_chain(class);
            t.script_classes = chain.iter().map(|&c| vm.class_name(c).to_owned()).collect();
            if chain
                .iter()
                .any(|&c| vm.class_guid(c) == Some(SCORE_BOARD_SCRIPT))
            {
                self.score_board_scripts.insert(guid);
            }
            let Ok(defaults) = vm.field_defaults(class) else {
                continue;
            };
            let members = t.members.get_or_insert_with(BTreeMap::new);
            for (name, v) in defaults {
                members.entry(name).or_insert(v);
            }
        }
    }

    /// Adds a player's Sackboy as a new Thing (YELLOWHEAD, CREATURE, BODY, POS; no parent)
    /// at `position`, covering the z span `layers`. Triggers test players only. The n-th
    /// Sackboy belongs to player number n - 1; that player's record
    /// ([`LevelWorld::player_records`]) is made here when it is not there yet.
    pub fn add_player(&mut self, position: [f32; 3], layers: (f32, f32)) -> u32 {
        let thing = self.things.len() as u32;
        self.things.push(ThingState {
            parts: vec!["BODY", "POS", "YELLOWHEAD", "CREATURE"],
            ..Default::default()
        });
        // `PYellowHead.PlayerNumber`, read by scripts a player walks into: the player's
        // index (INFERRED: an s32 `Game.EPlayerNumber`, as `Game.GetPlayerNumber__i`).
        let number = self.players.len() as i32;
        let part = self.part_handle(thing, "YELLOWHEAD") as usize - 1;
        self.part_handles[part]
            .members
            .insert("PlayerNumber".into(), Value::Int(number));
        if !self.player_records.iter().any(|r| r.number == number) {
            self.player_records.push(PlayerRecord {
                number,
                ..Default::default()
            });
        }
        self.players.push(PlayerState {
            thing,
            number,
            layers,
        });
        self.move_player(thing, position, layers);
        thing
    }

    /// Puts a player's Sackboy at `position` (no rotation) with the z span `layers`.
    pub fn move_player(&mut self, thing: u32, position: [f32; 3], layers: (f32, f32)) {
        let mut m = [0.; 16];
        (m[0], m[5], m[10], m[15]) = (1., 1., 1., 1.);
        m[12..15].copy_from_slice(&position);
        if let Some(t) = self.things.get_mut(thing as usize) {
            // A Sackboy has no parent: his local matrix is his world matrix.
            t.world_matrix = Some(m);
            t.local_matrix = Some(m);
        }
        if let Some(p) = self.players.iter_mut().find(|p| p.thing == thing) {
            p.layers = layers;
        }
    }

    /// One tick of every PTrigger against the players, in Thing order: updates the
    /// triggers' `inside` lists and returns what happened.
    ///
    /// FOUND in the 01.33 executable (trigger pass 0xe3bc0, per trigger 0x1364f0, test
    /// 0x135fe0, add 0x135a18, remove 0x1346fc):
    /// - The position is taken into the trigger Thing's own space (inverse world matrix),
    ///   so the Thing's scale sizes the trigger. The radius is `radiusMultiplier`, times
    ///   `hysteresisMultiplier` while anything is inside. Type 0: x^2 + y^2 < r^2; type 1:
    ///   |x| < r and |y| < r; type 4: x^2 + y^2 + z^2 < r^2; other types hold nothing.
    /// - A trigger that is not enabled holds nothing.
    /// - Without `allZLayers`, a Sackboy only counts when the trigger's z plus `zOffset`
    ///   lies within the z span of his body.
    /// - A Thing newly inside is appended and raises Enter; one no longer inside is
    ///   removed and raises Leave; then every Thing in the list raises Inside.
    /// - Ordinary triggers test the players' Sackboys only.
    ///
    /// Not modelled: the seven engine object kinds whose triggers test other Things too,
    /// the Sackboy states that never count (creature state 5, a YELLOWHEAD flag), the
    /// world-bounds cull, and the object kinds the engine handles without a script.
    /// INFERRED: the player's point is his Thing position; triggers run in Thing order.
    pub fn update_triggers(&mut self) -> Vec<TriggerEvent> {
        let mut events = Vec::new();
        let players: Vec<(PlayerState, [f32; 3])> = self
            .players
            .iter()
            .filter_map(|p| {
                let m = self.things.get(p.thing as usize)?.world_matrix?;
                Some((*p, [m[12], m[13], m[14]]))
            })
            .collect();
        for (i, t) in self.things.iter_mut().enumerate() {
            let (Some(g), Some(m)) = (t.trigger.as_mut(), t.world_matrix) else {
                continue;
            };
            let trigger = i as u32;
            for (p, pos) in &players {
                let radius = if g.inside.is_empty() {
                    g.radius
                } else {
                    g.radius * g.hysteresis
                };
                let z = m[14] + g.z_offset;
                let inside = g.enabled
                    && (g.all_z_layers || (p.layers.0..=p.layers.1).contains(&z))
                    && to_local(&m, *pos).is_some_and(|l| match g.trigger_type {
                        0 => l[0] * l[0] + l[1] * l[1] < radius * radius,
                        1 => l[0].abs() < radius && l[1].abs() < radius,
                        4 => l[0] * l[0] + l[1] * l[1] + l[2] * l[2] < radius * radius,
                        _ => false,
                    });
                let at = g.inside.iter().position(|x| *x == p.thing);
                let kind = match (inside, at) {
                    (true, None) => {
                        g.inside.push(p.thing);
                        TriggerEventKind::Enter
                    }
                    (false, Some(at)) => {
                        g.inside.remove(at);
                        TriggerEventKind::Leave
                    }
                    _ => continue,
                };
                events.push(TriggerEvent {
                    trigger,
                    kind,
                    thing: p.thing,
                });
            }
            events.extend(g.inside.iter().map(|&thing| TriggerEvent {
                trigger,
                kind: TriggerEventKind::Inside,
                thing,
            }));
        }
        events
    }

    /// Gives every Thing with a PPos its local matrix: inverse(parent's world) x world when
    /// its parent has a PPos, else its world matrix.
    ///
    /// FOUND (01.33, 0xf26bc and 0xf1d58): this is how the game keeps the local matrix
    /// when a world matrix is written. LBP2 level files store the world matrix only.
    /// INFERRED: a parent whose matrix cannot be inverted counts as no parent.
    pub fn derive_local_matrices(&mut self) {
        for i in 0..self.things.len() {
            let Some(world) = self.things[i].world_matrix else {
                continue;
            };
            let local = self
                .parent_world(i as u32)
                .and_then(|p| transform::inverse(&p))
                .map_or(world, |inv| transform::mul(&inv, &world));
            self.things[i].local_matrix = Some(local);
        }
    }

    /// World matrix of the Thing's parent, when it has a parent with a PPos.
    fn parent_world(&self, thing: u32) -> Option<[f32; 16]> {
        let parent = self.things.get(thing as usize)?.parent?;
        self.things.get(parent as usize)?.world_matrix
    }

    /// After a Thing moved: every descendant with a PPos gets world = its parent's world x
    /// its own local, parents before children (FOUND, 0xf250c / 0xf20f0). A descendant
    /// whose parent has no PPos keeps its matrix (INFERRED), and its children are still
    /// visited.
    fn move_descendants(&mut self, thing: u32) {
        let first = |w: &Self, t: u32| w.things.get(t as usize).and_then(|t| t.first_child);
        let mut stack: Vec<u32> = first(self, thing).into_iter().collect();
        // A damaged tree cannot make this run for ever.
        let mut left = self.things.len();
        while let Some(t) = stack.pop().filter(|_| left > 0) {
            left -= 1;
            let Some(state) = self.things.get(t as usize) else {
                continue;
            };
            let (sibling, local) = (state.next_sibling, state.local_matrix);
            if let (Some(parent), Some(local)) = (self.parent_world(t), local) {
                let world = transform::mul(&parent, &local);
                self.change_display(t, |s| s.world_matrix = Some(world));
            }
            stack.extend(sibling);
            stack.extend(first(self, t));
        }
    }

    /// The Thing's microchip Thing (the game's Thing+0x5c), if it has one.
    ///
    /// INFERRED: the Thing whose PMicrochip names the circuit board this Thing is a child
    /// of, and for a chip that itself sits on a board, the outermost chip. The level
    /// scripts read the PPos of this Thing without a null test, so it must have one; in
    /// the levels a chip on another chip's board has no PPos and the outermost chip has
    /// (PC census, 378 levels: 1,107 scripted Things sit on such an inner chip).
    pub fn microchip_of(&self, thing: u32) -> Option<u32> {
        let owner = |t: u32| {
            let parent = self.things.get(t as usize)?.parent?;
            self.board_owners.get(&parent).copied()
        };
        let mut chip = owner(thing)?;
        // Bounded, so a damaged tree cannot loop.
        for _ in 0..MAX_CHIP_NESTING {
            match owner(chip) {
                Some(outer) if outer != chip => chip = outer,
                _ => break,
            }
        }
        Some(chip)
    }

    /// Writes the buttons that are down this frame on a player's pad and returns whether
    /// the player has a record. The first write gives the record its pad, with both mode
    /// masks 1 (INFERRED: mode 1 is gameplay, the mode the level scripts ask with, and
    /// gameplay owns the pad). Call it before the tick; [`LevelWorld::end_pad_frame`]
    /// then makes these the buttons of the frame before.
    ///
    /// `buttons` is the game's button word (see [`PadState`]). A player who was never
    /// given buttons has no pad, and every `Pad` query about him is false.
    pub fn set_pad_buttons(&mut self, number: i32, buttons: u32) -> bool {
        let Some(r) = self.player_records.iter_mut().find(|r| r.number == number) else {
            return false;
        };
        let pad = r.pad.get_or_insert(PadState {
            mask_a: 1,
            mask_b: 1,
            ..Default::default()
        });
        pad.now = buttons;
        true
    }

    /// The end of a frame for the pads: this frame's buttons become the old ones. FOUND
    /// (01.33, 0x3866e8) that the game copies them once a frame; INFERRED where in the
    /// frame. Here it is the end of [`LevelScripts::tick_level`], so every script of a
    /// tick sees the same edge.
    pub fn end_pad_frame(&mut self) {
        for pad in self
            .player_records
            .iter_mut()
            .filter_map(|r| r.pad.as_mut())
        {
            pad.old = pad.now;
        }
    }

    /// The Thing a global-settings slot hands its per-frame call to, for the slot
    /// `kind` at its turn in the pass ([`LevelScripts::apply_global_settings`]).
    ///
    /// FOUND (01.33, the PWorld's per-frame function 0xd5e84):
    /// - On its first run (a PWorld flag byte is still 0) it writes every slot again with
    ///   its own UID and priority through the UID setter with the priority test off
    ///   (0xce554, last argument 1). That setter clears the slot first (0xce520) and
    ///   writes the priority back only for a UID other than 0, so the one change is that
    ///   an empty slot loses a stored priority.
    /// - Then, slot by slot: an empty slot is passed over. For a slot with a UID the
    ///   Thing is looked up (0xce860, a search of the PWorld's sorted UID list). A Thing
    ///   that has a PScript part gets the call; when no Thing has the UID, or the Thing
    ///   has no PScript, the slot's UID and priority are set to 0.
    ///
    /// INFERRED: the flag byte is 0 for a level just loaded; UIDs are unique, so the
    /// first Thing with the UID is the one the game's search finds; a Thing has a PScript
    /// part when the reader kept a script resource for it. Not modelled, same function:
    /// default audio settings while slot 7 is empty, and the water level and wave values
    /// it refreshes for the renderer.
    pub fn global_settings_holder(&mut self, kind: usize) -> SlotHolder {
        if self.world.is_none() {
            return SlotHolder::Empty;
        }
        let p = &mut self.pworld;
        if !self.global_settings_started {
            self.global_settings_started = true;
            for (uid, priority) in p
                .global_settings_thing_uids
                .iter()
                .zip(&mut p.global_settings_thing_priority)
            {
                if *uid == 0 {
                    *priority = 0;
                }
            }
        }
        let uid = match p.global_settings_thing_uids.get(kind) {
            Some(&uid) if uid != 0 => uid,
            _ => return SlotHolder::Empty,
        };
        let holder = self
            .things
            .iter()
            .position(|t| t.uid == Some(uid))
            .filter(|&t| self.things[t].script_guid.is_some())
            .and_then(|t| u32::try_from(t).ok());
        match holder {
            Some(thing) => SlotHolder::Thing(thing),
            None => {
                p.global_settings_thing_uids[kind] = 0;
                if let Some(priority) = p.global_settings_thing_priority.get_mut(kind) {
                    *priority = 0;
                }
                SlotHolder::Cleared
            }
        }
    }

    /// Per global-settings type, whether the value in force is not the one in force when
    /// the level was read ([`LevelWorld::settings_at_load`]).
    pub fn settings_unlike_load(&self) -> [bool; GLOBAL_SETTINGS] {
        self.pworld
            .settings_in_force()
            .unlike(&self.settings_at_load)
    }

    /// One world frame has passed: `PWorld.Frame` (and `simFrame`) go up by one.
    ///
    /// INFERRED: the game's increment was not located. Here it happens at the end of
    /// [`LevelScripts::tick_level`], so the scripts of the first tick read the stored
    /// frame and those of tick n the stored frame + n - 1.
    pub fn advance_frame(&mut self) {
        self.pworld.frame = self.pworld.frame.wrapping_add(1);
        self.pworld.sim_frame = self.pworld.sim_frame.wrapping_add(1);
    }

    /// A member of an engine part that the runtime keeps as real state, or `None` for one
    /// it does not know (those live in the part handle's own map).
    ///
    /// FOUND (01.33): the PWorld serializer (0xb3f3c0..0xb3fd20) names `Frame` (s32),
    /// `GameMode` (i32) and `WaterLevel` (f32) and the level file stores them; the PPos
    /// readers (0x755874, 0x755a98) give the world matrix its place in the part.
    /// INFERRED: that the members scripts name are these fields, and that the PTrigger
    /// members are the stored `radiusMultiplier`, `allZLayers` and `hysteresisMultiplier`
    /// the trigger test uses ([`LevelWorld::update_triggers`]).
    fn engine_member(&self, thing: u32, part: &str, field: &str) -> Result<Option<Value>, String> {
        let t = self.state(thing)?;
        let trigger = || {
            t.trigger
                .as_ref()
                .ok_or_else(|| format!("thing {thing} has no trigger state"))
        };
        Ok(Some(match (part, field) {
            ("WORLD", "GameMode") => Value::Int(self.pworld.game_mode),
            ("WORLD", "Frame") => Value::Int(self.pworld.frame),
            ("WORLD", "WaterLevel") => Value::Float(self.pworld.water_level),
            ("POS", "WorldPosition") => Value::Mat44(
                t.world_matrix
                    .ok_or_else(|| format!("thing {thing} has no position"))?,
            ),
            ("TRIGGER", "RadiusMultiplier") => Value::Float(trigger()?.radius),
            ("TRIGGER", "HysteresisMultiplier") => Value::Float(trigger()?.hysteresis),
            ("TRIGGER", "AllZLayers") => Value::Bool(trigger()?.all_z_layers),
            _ => return Ok(None),
        }))
    }

    /// Writes a member [`Self::engine_member`] reads; `Ok(false)` for any other member.
    ///
    /// `PPos.WorldPosition`: INFERRED a plain field write: the world matrix changes, the
    /// local matrix and the Thing's descendants do not (the member accessor was not
    /// located; the natives `SetWorldPosition__m` / `SetLocalPosition__m` do move them).
    /// No script of the levels writes the PWorld members; a write is kept.
    fn set_engine_member(
        &mut self,
        thing: u32,
        part: &str,
        field: &str,
        value: &Value,
    ) -> Result<bool, String> {
        let bad = || format!("{part} member {field} set to {value:?}");
        if let ("POS", "WorldPosition", Value::Mat44(m)) = (part, field, value) {
            self.state(thing)?;
            self.change_display(thing, |t| t.world_matrix = Some(*m));
            return Ok(true);
        }
        let pworld = &mut self.pworld;
        let t = self
            .things
            .get_mut(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))?;
        match (part, field, value) {
            ("WORLD", "GameMode", Value::Int(i)) => pworld.game_mode = *i,
            ("WORLD", "Frame", Value::Int(i)) => pworld.frame = *i,
            ("WORLD", "WaterLevel", Value::Float(f)) => pworld.water_level = *f,
            ("TRIGGER", "RadiusMultiplier" | "HysteresisMultiplier" | "AllZLayers", v) => {
                let g = t
                    .trigger
                    .as_mut()
                    .ok_or_else(|| format!("thing {thing} has no trigger state"))?;
                match (field, v) {
                    ("RadiusMultiplier", Value::Float(f)) => g.radius = *f,
                    ("HysteresisMultiplier", Value::Float(f)) => g.hysteresis = *f,
                    ("AllZLayers", Value::Bool(b)) => g.all_z_layers = *b,
                    _ => return Err(bad()),
                }
            }
            ("WORLD", "GameMode" | "Frame" | "WaterLevel", _) | ("POS", "WorldPosition", _) => {
                return Err(bad())
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Handle for a part a Thing carries (index + 1), created on first use.
    fn part_handle(&mut self, thing: u32, part: &'static str) -> u32 {
        if let Some(i) = self
            .part_handles
            .iter()
            .position(|p| p.thing == thing && p.part == part)
        {
            return i as u32 + 1;
        }
        self.part_handles.push(PartState {
            thing,
            part,
            members: BTreeMap::new(),
        });
        self.part_handles.len() as u32
    }

    fn part_of(&mut self, base: &Value) -> Result<&mut PartState, String> {
        match base {
            Value::Raw(0) => Err("null part".into()),
            Value::Raw(h) => self
                .part_handles
                .get_mut(*h as usize - 1)
                .ok_or_else(|| format!("no part handle {h}")),
            other => Err(format!("member base is not a part: {other:?}")),
        }
    }

    /// `thing.SpriteLight` and the like: the part as a raw pointer, null when the Thing
    /// lacks it. Only names of the reader's part list are parts.
    fn native_member(&mut self, base: &Value, class: &str, field: &str) -> Result<Value, String> {
        let Some(part) = part_name(field) else {
            return Err(format!(
                "world does not provide native member {class}.{field}"
            ));
        };
        let t = Self::thing_of(base)?;
        Ok(if self.state(t)?.parts.contains(&part) {
            Value::Raw(self.part_handle(t, part))
        } else {
            Value::Raw(0)
        })
    }

    fn state(&self, thing: u32) -> Result<&ThingState, String> {
        self.things
            .get(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))
    }

    fn thing_mut(&mut self, thing: u32) -> Result<&mut ThingState, String> {
        self.things
            .get_mut(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))
    }

    fn thing_of(base: &Value) -> Result<u32, String> {
        match base {
            Value::Thing(Some(t)) => Ok(*t),
            Value::Thing(None) => Err("null thing".into()),
            other => Err(format!("member base is not a thing: {other:?}")),
        }
    }
}

impl World for LevelWorld {
    fn thing_uid(&mut self, thing: u32) -> Result<i32, String> {
        self.state(thing)?
            .uid
            .ok_or_else(|| format!("thing {thing} has no UID"))
    }
    fn parent(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.parent)
    }
    fn first_child(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.first_child)
    }
    fn next_sibling(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.next_sibling)
    }
    fn world_thing(&mut self) -> Result<Option<u32>, String> {
        Ok(self.world)
    }
    fn world_transform(&mut self, thing: u32) -> Result<Option<[f32; 16]>, String> {
        Ok(self.state(thing)?.world_matrix)
    }
    fn local_transform(&mut self, thing: u32) -> Result<Option<[f32; 16]>, String> {
        Ok(self.state(thing)?.local_matrix)
    }
    /// FOUND (01.33, 0xf2e80): see `Thing.SetLocalPosition__m` in the native table.
    fn set_local_transform(&mut self, thing: u32, m: [f32; 16]) -> Result<(), String> {
        if self.state(thing)?.world_matrix.is_none() {
            return Ok(());
        }
        let world = self
            .parent_world(thing)
            .map_or(m, |p| transform::mul(&p, &m));
        self.change_display(thing, |t| {
            (t.local_matrix, t.world_matrix) = (Some(m), Some(world));
        });
        self.move_descendants(thing);
        Ok(())
    }
    /// The "deactivated" flag the switch pass keeps (FOUND: the byte the native reads,
    /// Thing+0x7f). The flags get their start values first if no switch pass ran yet.
    fn is_inactive(&mut self, thing: u32) -> Result<bool, String> {
        self.prepare_switches();
        Ok(self.state(thing)?.deactivated)
    }
    fn microchip_proxy(&mut self, thing: u32) -> Result<u32, String> {
        self.state(thing)?;
        Ok(self.microchip_of(thing).unwrap_or(thing))
    }
    fn is_on_sequencer(&mut self, thing: u32) -> Result<bool, String> {
        self.state(thing)?;
        Ok(self.on_sequencer(thing))
    }
    fn has_part(&mut self, thing: u32, part: &str) -> Result<bool, String> {
        let want = part_name(part);
        Ok(self.state(thing)?.parts.iter().any(|p| Some(*p) == want))
    }
    fn trigger_count(&mut self, thing: u32) -> Result<i32, String> {
        let t = self.state(thing)?;
        Ok(t.trigger.as_ref().map_or(0, |g| g.inside.len() as i32))
    }
    fn trigger_thing(&mut self, thing: u32, index: i32) -> Result<Option<u32>, String> {
        let inside = self.state(thing)?.trigger.as_ref().map(|g| &g.inside);
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| inside?.get(i).copied()))
    }
    /// The current signal of the output wired to the port ([`LevelWorld::wire_inputs`]),
    /// as the switch pass last wrote it.
    fn switch_input(&mut self, thing: u32, port: i32) -> Result<Option<SwitchInput>, String> {
        Ok(self.input(thing, port)?.map(|(_, o)| o.signal))
    }
    fn switch_input_thing(&mut self, thing: u32, port: i32) -> Result<Option<u32>, String> {
        Ok(self.input(thing, port)?.map(|(w, _)| w.switch))
    }
    fn switch_output_target(
        &mut self,
        thing: u32,
        output: i32,
        target: i32,
    ) -> Result<Option<u32>, String> {
        let switch = self.state(thing)?.switch.as_ref();
        let (Ok(o), Ok(t)) = (usize::try_from(output), usize::try_from(target)) else {
            return Ok(None);
        };
        Ok(switch
            .and_then(|s| s.outputs.get(o)?.as_ref())
            .and_then(|o| o.targets.get(t)?.0))
    }
    /// The Thing's script is the scoreboard script, or a loaded class derived from it
    /// (known once [`LevelWorld::add_class_defaults`] ran).
    fn is_score_board(&mut self, thing: u32) -> Result<bool, String> {
        Ok(self
            .state(thing)?
            .script_guid
            .is_some_and(|g| g == SCORE_BOARD_SCRIPT || self.score_board_scripts.contains(&g)))
    }
    /// The first loaded table that has the key gives the text (FOUND, 01.33, 0x159c1c:
    /// the search `IsInLams` makes also hands out the string). With no table loaded, a
    /// key in no table, or a row whose text did not decode: an error (what the game
    /// returns for a missing key was not read).
    fn translate(&mut self, key: i32) -> Result<String, String> {
        let key = key as u32;
        let Some(table) = self.translations.iter().find(|t| t.contains_key(key)) else {
            return Err(format!(
                "LAMS key {key:#010x} is in none of the {} loaded tables",
                self.translations.len()
            ));
        };
        table
            .get(key)
            .map(str::to_owned)
            .ok_or_else(|| format!("LAMS key {key:#010x} has no text"))
    }
    /// FOUND (01.33, 0x159c1c): true when a loaded table has a row with the key.
    fn in_lams(&mut self, key: i32) -> Result<bool, String> {
        let key = key as u32;
        Ok(self.translations.iter().any(|t| t.contains_key(key)))
    }
    /// FOUND (01.33, 0x156b18 and the pad readers): PYellowHead -> player number ->
    /// record -> pad. Here the Things with a known player number are the Sackboys
    /// [`LevelWorld::add_player`] made; a level file's own PYellowHead Things have none
    /// (INFERRED, as [`World::yellowhead`]).
    fn pad(&mut self, thing: u32) -> Result<Option<PadState>, String> {
        self.state(thing)?;
        let number = self.players.iter().find(|p| p.thing == thing);
        Ok(number.and_then(|p| {
            let record = self.player_records.iter().find(|r| r.number == p.number);
            record?.pad
        }))
    }
    /// A script cast of a Thing to a script class (`CASTsp`): the Thing carries a script
    /// of that class or of a class derived from it. A Thing without a script, or whose
    /// class is not loaded, is no instance of any class. Until
    /// [`LevelWorld::add_class_defaults`] has run the classes are not known and every
    /// cast passes (INFERRED stand-in for a world without a VM).
    fn thing_is(&mut self, thing: u32, class: &str) -> bool {
        if !self.classes_resolved {
            return true;
        }
        self.things
            .get(thing as usize)
            .is_some_and(|t| t.script_classes.iter().any(|c| c == class))
    }
    /// FOUND (01.33, 0x7477d8 -> 0x1a6cd8): the slot lookup finds nothing for (0, 0), and
    /// the group of a slot that is not found is (0, 0), as a new SlotID. Here the object
    /// made ahead for that answer. Any other slot needs the game's slot database, which
    /// the world does not have: an error.
    fn slot_group(&mut self, slot: &Value) -> Result<Value, String> {
        match self.slot_id(slot)? {
            (0, 0) => self
                .no_group_slot
                .map(|o| Value::Object(Some(o)))
                .ok_or_else(|| "no SlotID object (class not loaded)".into()),
            (kind, number) => Err(format!(
                "the group of slot ({kind}, {number}) needs the slot database"
            )),
        }
    }
    /// The contents of the SlotID objects the world hands out. A native cannot read
    /// a script object, so any other SlotID (one a script built) is an error. INFERRED:
    /// the scripts do not change the objects they were handed.
    fn slot_id(&mut self, slot: &Value) -> Result<(i32, i32), String> {
        match slot {
            Value::Object(Some(o)) if Some(*o) == self.no_group_slot => Ok((0, 0)),
            Value::Object(Some(o)) if Some(*o) == self.level_slot => Ok(self.level_slot_id),
            Value::Object(Some(o)) if Some(*o) == self.primary_level_slot => {
                Ok(self.primary_level_slot_id)
            }
            other => Err(format!("SlotID {other:?} is not one the world made")),
        }
    }
    /// FOUND (01.33): see the `SetGlobalSettings<X>To` entries in the native table.
    /// Nothing for a level without a PWorld.
    fn set_global_setting_to(&mut self, kind: i32, value: f32, blend: bool) -> Result<(), String> {
        if self.world.is_none() {
            return Ok(());
        }
        let p = &mut self.pworld;
        let setting = match kind {
            GS_WATER_LEVEL => &mut p.water,
            GS_GRAVITY => &mut p.gravity,
            GS_WATER_WAVE_MAGNITUDE => &mut p.water_wave_magnitude,
            GS_WATER_MURKINESS => &mut p.water_murkiness,
            GS_WATER_BITS => &mut p.water_bits,
            GS_WATER_TINT => {
                p.water_tint = value;
                return Ok(());
            }
            _ => return Err(format!("global setting {kind} is not one float")),
        };
        setting.set_to(value, blend);
        Ok(())
    }
    /// FOUND (01.33, 0xd9dac): see `Game.SetGlobalSettingsTo__fffffb` in the native table.
    fn set_lighting_to(&mut self, values: [f32; 5], blend: bool) -> Result<(), String> {
        if self.world.is_none() {
            return Ok(());
        }
        // Arguments: Lighting, Darkness, Fog, FogTint, ColourCorrection. Stored order:
        // lighting, colour correction, fog, fog tint, darkness.
        let [lighting, darkness, fog, fog_tint, colour_correction] = values;
        let target = [lighting, colour_correction, fog, fog_tint, darkness];
        let p = &mut self.pworld;
        if blend {
            p.lighting_from = p.lighting();
            p.lighting_blend = 0.;
        } else {
            p.lighting_from = target;
            p.lighting_blend = 1.;
        }
        p.lighting_to = target;
        Ok(())
    }
    /// FOUND (01.33, 0xd623c): the blend factor of that type. A type outside the 8 is an
    /// error (the game writes past the array).
    fn set_global_setting_blend(&mut self, kind: i32, t: f32) -> Result<(), String> {
        if self.world.is_none() {
            return Ok(());
        }
        let p = &mut self.pworld;
        let slot = match kind {
            GS_WATER_LEVEL => &mut p.water.t,
            GS_GRAVITY => &mut p.gravity.t,
            GS_WATER_WAVE_MAGNITUDE => &mut p.water_wave_magnitude.t,
            GS_WATER_TINT => &mut p.water_tint_blend,
            GS_WATER_MURKINESS => &mut p.water_murkiness.t,
            GS_LIGHTING => &mut p.lighting_blend,
            GS_WATER_BITS => &mut p.water_bits.t,
            7 => &mut p.audio_blend,
            _ => {
                return Err(format!(
                    "global setting {kind} outside 0..{GLOBAL_SETTINGS}"
                ))
            }
        };
        *slot = t;
        Ok(())
    }
    /// FOUND (01.33, the getters): the blended value, for the tint the stored value.
    /// 0.0 for a level without a PWorld, and for a water level that is not above
    /// [`WATER_LEVEL_MIN`] (a level without water). Not modelled: an editor flag that
    /// also makes the water level 0.0.
    fn global_setting_value(&mut self, kind: i32) -> Result<f32, String> {
        if self.world.is_none() {
            return Ok(0.);
        }
        let p = &self.pworld;
        Ok(match kind {
            // The game's own comparison: the level must be above the constant.
            GS_WATER_LEVEL => Some(p.water.value())
                .filter(|level| WATER_LEVEL_MIN < *level)
                .unwrap_or(0.),
            GS_WATER_WAVE_MAGNITUDE => p.water_wave_magnitude.value(),
            GS_WATER_TINT => p.water_tint,
            GS_WATER_MURKINESS => p.water_murkiness.value(),
            GS_WATER_BITS => p.water_bits.value(),
            _ => return Err(format!("global setting {kind} has no getter")),
        })
    }
    /// FOUND (01.33, 0x5ec908): one byte of the PWorld; nothing without one.
    fn set_water_drain_sounds(&mut self, enabled: bool) -> Result<(), String> {
        if self.world.is_some() {
            self.pworld.water_drain_sounds = enabled;
        }
        Ok(())
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        match kind {
            // Through `Thing` or a script class of a Thing (`World.World`).
            MemberKind::Native => return self.native_member(base, class, field),
            MemberKind::RefPtr => {
                let (thing, part) = {
                    let p = self.part_of(base)?;
                    (p.thing, p.part)
                };
                if let Some(v) = self.engine_member(thing, part, field)? {
                    return Ok(v);
                }
                let p = self.part_of(base)?;
                return p
                    .members
                    .get(field)
                    .cloned()
                    .ok_or_else(|| format!("{} member {class}.{field} not read yet", p.part));
            }
            MemberKind::Thing => {}
        }
        let t = Self::thing_of(base)?;
        self.state(t)?
            .members
            .as_ref()
            .and_then(|m| m.get(field))
            .cloned()
            .ok_or_else(|| format!("thing {t} has no script member {class}.{field}"))
    }
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        if kind == MemberKind::RefPtr {
            let (thing, part) = {
                let p = self.part_of(base)?;
                (p.thing, p.part)
            };
            if !self.set_engine_member(thing, part, field, &value)? {
                self.part_of(base)?.members.insert(field.to_owned(), value);
            }
            return Ok(());
        }
        if kind != MemberKind::Thing {
            return Err(format!(
                "world does not provide {kind:?} member {class}.{field}"
            ));
        }
        let t = Self::thing_of(base)?;
        let state = self
            .things
            .get_mut(t as usize)
            .ok_or_else(|| format!("no thing {t}"))?;
        state
            .members
            .get_or_insert_with(BTreeMap::new)
            .insert(field.to_owned(), value);
        Ok(())
    }
    /// Counted by name in [`LevelWorld::sounds_played`]; nothing plays (audio natives are
    /// recorded no-ops until there is an audio system, docs/rscript-plan.md).
    fn play_sound(&mut self, _thing: Option<u32>, name: &str) -> Result<(), String> {
        *self.sounds_played.entry(name.to_owned()).or_default() += 1;
        Ok(())
    }
    /// FOUND (01.33, 0x5fadd4 -> 0x5e885c): the count of player records
    /// ([`LevelWorld::player_records`]).
    fn num_players(&mut self) -> Result<i32, String> {
        Ok(self.player_records.len() as i32)
    }
    /// Play mode (0); levels run as played, not edited.
    fn play_or_edit_mode(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    /// No dialogue list is loaded (0).
    fn dialogue_list(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    fn shape_editor_colour(&mut self, thing: u32) -> Result<[f32; 4], String> {
        self.state(thing)?
            .shape_colour
            .ok_or_else(|| format!("thing {thing} has no shape"))
    }
    /// FOUND (01.33): nothing for a Thing without a PShape.
    fn set_shape_editor_colour(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        if self.state(thing)?.shape_colour.is_some() {
            self.change_display(thing, |t| t.shape_colour = Some(colour));
        }
        Ok(())
    }
    /// FOUND (01.33): nothing for a Thing without a PShape. Kept; nothing draws it.
    fn set_shape_tint(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        if self.state(thing)?.shape_colour.is_some() {
            self.change_display(thing, |t| t.shape_tint = Some(colour));
        }
        Ok(())
    }
    /// FOUND (01.33): nothing for a Thing without a PRenderMesh.
    fn set_render_mesh_tint(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        if self.state(thing)?.render_mesh_visibility.is_some() {
            self.change_display(thing, |t| t.render_mesh_tint = Some(colour));
        }
        Ok(())
    }
    /// FOUND (01.33): nothing for a Thing without a PRenderMesh.
    fn set_render_mesh_colour(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        if self.state(thing)?.render_mesh_visibility.is_some() {
            self.change_display(thing, |t| t.render_mesh_colour = Some(colour));
        }
        Ok(())
    }
    /// FOUND (01.33): the play-mode bit of the PRenderMesh's visibility byte; false for
    /// a Thing without a PRenderMesh.
    fn render_mesh_visible_in_play_mode(&mut self, thing: u32) -> Result<bool, String> {
        let byte = self.state(thing)?.render_mesh_visibility;
        Ok(byte.is_some_and(|v| v & MESH_VISIBLE_IN_PLAY_MODE != 0))
    }
    /// See [`LevelWorld::show_render_mesh`].
    fn set_render_mesh_visible(&mut self, thing: u32, visible: bool) -> Result<(), String> {
        self.state(thing)?;
        self.show_render_mesh(thing, visible);
        Ok(())
    }
    /// Recorded on the Thing; nothing animates by it yet.
    fn set_anim_speed(&mut self, thing: u32, speed: f32) -> Result<(), String> {
        self.thing_mut(thing)?.anim_speed = Some(speed);
        Ok(())
    }
    /// FOUND (01.33, 0x5e917c): the number of record `index`, -1 outside the records.
    fn player_number(&mut self, index: i32) -> Result<i32, String> {
        let record = usize::try_from(index)
            .ok()
            .and_then(|i| self.player_records.get(i));
        Ok(record.map_or(-1, |r| r.number))
    }
    fn player(&mut self, number: i32) -> Result<Option<&mut PlayerRecord>, String> {
        Ok(self.player_records.iter_mut().find(|r| r.number == number))
    }
    /// FOUND (01.33, 0x18ec14): the game walks the PWorld's list of PYellowHead parts for
    /// the one with this player number. Here those are the Sackboys
    /// [`LevelWorld::add_player`] made; a level file's own PYellowHead Things are not
    /// looked at (INFERRED: a stored level has no player in it).
    fn yellowhead(&mut self, number: i32) -> Result<Option<u32>, String> {
        let player = self.players.iter().find(|p| p.number == number);
        Ok(player.map(|p| p.thing))
    }
    /// INFERRED: one machine, so every player with a record is local.
    fn player_is_local(&mut self, number: i32) -> Result<bool, String> {
        Ok(self.player_records.iter().any(|r| r.number == number))
    }
    /// FOUND (01.33, 0x5eb810): 0 for a level without a PWorld. A `kind` outside the
    /// slots is an error (the game reads past the array).
    fn global_setting_uid(&mut self, kind: i32) -> Result<i32, String> {
        if self.world.is_none() {
            return Ok(0);
        }
        let slots = &self.pworld.global_settings_thing_uids;
        usize::try_from(kind)
            .ok()
            .and_then(|k| slots.get(k).copied())
            .ok_or_else(|| format!("global setting {kind} outside 0..{}", slots.len()))
    }
    /// FOUND (01.33, 0xce554): see `Game.SetGlobalSettingUID` in the native table.
    /// Nothing for a level without a PWorld (0x5eb718 tests for one first).
    fn set_global_setting_uid(&mut self, kind: i32, uid: i32, priority: i32) -> Result<(), String> {
        if self.world.is_none() {
            return Ok(());
        }
        let p = &mut self.pworld;
        let n = p
            .global_settings_thing_uids
            .len()
            .min(p.global_settings_thing_priority.len());
        let k = usize::try_from(kind)
            .ok()
            .filter(|&k| k < n)
            .ok_or_else(|| format!("global setting {kind} outside 0..{n}"))?;
        let current = p.global_settings_thing_priority[k];
        if priority < current || current == 1 {
            return Ok(());
        }
        p.global_settings_thing_uids[k] = uid;
        p.global_settings_thing_priority[k] = if uid != 0 { priority } else { 0 };
        Ok(())
    }
    /// The SlotID object made ahead ([`add_engine_objects`]).
    fn level_slot_id(&mut self, primary: bool) -> Result<Value, String> {
        let slot = if primary {
            self.primary_level_slot
        } else {
            self.level_slot
        };
        slot.map(|o| Value::Object(Some(o)))
            .ok_or_else(|| "no SlotID object (class not loaded)".into())
    }
    /// Depth first, a Thing before its children, children in Thing-list order.
    fn find_mesh_descendant(&mut self, thing: u32, mesh: u32) -> Result<Option<u32>, String> {
        let mut at = self.state(thing)?.first_child;
        // Bounded, so a damaged tree cannot loop.
        for _ in 0..self.things.len() {
            let Some(t) = at else { break };
            let state = self.state(t)?;
            if state.mesh_guid == Some(mesh) {
                return Ok(Some(t));
            }
            at = state.first_child;
            let mut up = t;
            while at.is_none() && up != thing {
                let s = self.state(up)?;
                at = s.next_sibling;
                match s.parent {
                    Some(p) if at.is_none() => up = p,
                    _ => break,
                }
            }
        }
        Ok(None)
    }
    /// Counted by name in [`LevelWorld::engine_calls`]. The render-mesh texture is also
    /// kept on its Thing (nothing draws by it yet) and a scoreboard request in
    /// [`LevelWorld::scoreboard_requests`]. A null Thing is counted and nothing more, as
    /// the game does nothing for it. The calls of [`KEPT_REQUESTS`] are kept with their
    /// arguments in [`LevelWorld::requests`], and `SetAnimLoop` on its Thing. The
    /// messaging calls also go to the message list ([`LevelWorld::messages`]), the
    /// pod computer's two calls leave their value in [`LevelWorld::pod`], and the video
    /// and voice-over calls are kept in [`LevelWorld::media`].
    fn engine_call(
        &mut self,
        name: &'static str,
        thing: Option<u32>,
        args: &[Value],
    ) -> Result<(), String> {
        *self.engine_calls.entry(name).or_default() += 1;
        if let ("GotoScoreboard", Some(Value::Bool(b))) = (name, args.first()) {
            self.scoreboard_requests.push(*b);
        }
        if KEPT_REQUESTS.contains(&name) {
            if self.requests.len() < MAX_REQUESTS {
                self.requests.push(EngineRequest {
                    name,
                    thing,
                    args: args.to_vec(),
                    frame: self.pworld.frame,
                });
            } else {
                self.requests_dropped += 1;
            }
        }
        self.message_call(name, thing, args);
        self.media_call(name, thing, args);
        match (name, args.first()) {
            ("SetFadePodDecorations", Some(Value::Float(f))) => {
                self.pod.fade = Some(*f);
                self.pod.fade_calls += 1;
            }
            ("RequestHostUsePod", Some(Value::Bool(b)))
                if self.pod.host_use_pod_requests.len() < MAX_HOST_USE_POD_REQUESTS =>
            {
                self.pod.host_use_pod_requests.push(HostUsePodRequest {
                    in_use: *b,
                    frame: self.pworld.frame,
                });
            }
            _ => {}
        }
        let Some(thing) = thing else { return Ok(()) };
        let t = self.thing_mut(thing)?;
        match (name, args.first()) {
            ("SetRenderMeshEditorTexture", Some(v)) => t.render_mesh_texture = Some(v.clone()),
            ("SetAnimLoop", Some(Value::Bool(b))) => t.anim_loop = Some(*b),
            _ => {}
        }
        Ok(())
    }
    /// The handle is the GUID itself (INFERRED stand-in: nothing is loaded; scripts only
    /// keep the handle and pass it back to natives). A mesh handle is remembered as one
    /// ([`PodState::mesh_resources`]).
    fn load_resource(&mut self, kind: ResourceKind, guid: u32) -> Result<u32, String> {
        if kind == ResourceKind::Mesh {
            self.pod.mesh_resources.insert(guid);
        }
        Ok(guid)
    }
    /// [`LevelWorld::find_thing`], counted.
    fn find_thing(&mut self, name: &[u8]) -> Result<Option<u32>, String> {
        let found = LevelWorld::find_thing(self, name);
        self.pod.find_thing_calls += 1;
        self.pod.find_thing_hits += u64::from(found.is_some());
        Ok(found)
    }
    /// [`LevelWorld::pod_thing`], counted.
    fn pod_thing(&mut self, platform_too: bool) -> Result<Option<u32>, String> {
        let found = LevelWorld::pod_thing(self, platform_too);
        self.pod.pod_thing_calls += 1;
        self.pod.pod_thing_hits += u64::from(found.is_some());
        Ok(found)
    }
    /// FOUND (01.33, 0x755580): nothing for a Thing without a PRenderMesh; else the mesh
    /// becomes the resource when the resource is a mesh, and none when it is not. Here a
    /// handle is a mesh when `Resource.LoadMeshByFilename__g` handed it out.
    fn set_render_mesh_mesh(&mut self, thing: u32, mesh: Option<u32>) -> Result<(), String> {
        if self.state(thing)?.render_mesh_visibility.is_none() {
            return Ok(());
        }
        let mesh = mesh.filter(|g| self.pod.mesh_resources.contains(g));
        let before = self.things[thing as usize].mesh_guid;
        self.change_display(thing, |t| t.mesh_guid = mesh);
        self.pod.mesh_sets += 1;
        self.pod.mesh_changes += u64::from(before != mesh);
        Ok(())
    }
    /// [`LevelWorld::planets_background`].
    fn planets_background(&mut self) -> Result<u32, String> {
        Ok(LevelWorld::planets_background(self))
    }
    /// FOUND (01.33, 0xf26bc, 0xf1d58): see `Thing.SetWorldPosition__m` in the native
    /// table. INFERRED: a parent whose matrix cannot be inverted counts as no parent.
    fn set_world_transform(&mut self, thing: u32, m: [f32; 16]) -> Result<(), String> {
        if self.state(thing)?.world_matrix.is_none() {
            return Ok(());
        }
        let local = self
            .parent_world(thing)
            .and_then(|p| transform::inverse(&p))
            .map_or(m, |inv| transform::mul(&inv, &m));
        self.change_display(thing, |t| {
            (t.local_matrix, t.world_matrix) = (Some(local), Some(m));
        });
        self.move_descendants(thing);
        Ok(())
    }
    fn set_anim_pos(&mut self, thing: u32, pos: f32) -> Result<(), String> {
        self.thing_mut(thing)?.anim_pos = Some(pos);
        Ok(())
    }
    fn pod_computer(&mut self) -> Result<Value, String> {
        self.pod_computer
            .map(|o| Value::Object(Some(o)))
            .ok_or_else(|| "no PodComputer object (class not loaded)".into())
    }
    fn planets(&mut self) -> Result<Value, String> {
        self.planets
            .map(|o| Value::Object(Some(o)))
            .ok_or_else(|| "no Planets object (class not loaded)".into())
    }
    fn crafteroid(&mut self, index: i32) -> Result<Value, String> {
        usize::try_from(index)
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.crafteroids.get(i))
            .map(|(o, _)| Value::Object(Some(*o)))
            .ok_or_else(|| format!("no crafteroid {index}"))
    }
    fn planet_thing(&mut self, planet: &Value) -> Result<Option<u32>, String> {
        match planet {
            Value::Object(Some(o)) => self
                .crafteroids
                .iter()
                .find(|(p, _)| p == o)
                .map(|(_, t)| *t)
                .ok_or_else(|| format!("object {o:?} is not a planet")),
            other => Err(format!("expected a Planet, got {other:?}")),
        }
    }
    /// INFERRED defaults for the PC run: interactive, no pending links, no avatars spawned
    /// (there is no Sackboy yet), no video, a 16:9 screen, player 0 leads, no music track
    /// (GUID 0), no user music (stopped, not paused), interactive music audible, no
    /// attraction (demo) loop.
    ///
    /// Stage 4, all INFERRED:
    /// - `Game.IsVcrPaused__` is false. The game returns the PWorld's IsPaused byte, but
    ///   the level files store it as true in 220 of the 236 levels that have a PWorld
    ///   (PC census), story levels included, and those levels play: the stored value is
    ///   an editor state the game must clear when play starts. Where it clears it was not
    ///   located.
    /// - `Game.IsTutorialLevel__` is [`LevelWorld::tutorial_level`].
    /// - `Game.IsPSNDemoBuild__` is false: the full game, not the demo.
    /// - `Game.GetPartyLeadersPlayerNumber__` is the first player record's number, -1
    ///   with no record.
    ///
    /// D263. From state the world keeps (the rules are FOUND, see the native table):
    /// - `Messaging_ShouldFilterMessageStrings__`: true unless the level slot's type is
    ///   2. For type 2 the game asks the network session about the local player; here
    ///   false (INFERRED: that test holds for the one local player).
    /// - `Game.GetGameFrame__`: `PWorld.Frame`, 0 for a level without a PWorld.
    /// - `Game.GetNumYellowHeadAvatars__`: the Sackboys [`LevelWorld::add_player`] made.
    /// - `Score_Board.GetGameScoreDataType__`: 6 in game mode 1, else the number of
    ///   player records - 1, not below 0.
    ///
    /// D263, constants that hold for one local player in a level (INFERRED):
    /// - `OffScreenIndicator.GetPlayerIndicatorActivityType__i` is 0, no indicator: the
    ///   game gives 1 or 2 for a Sackboy off the screen or a second session entry. Not
    ///   exact for a player whose Sackboy is not spawned yet.
    /// - `Game.GetNumKeyNames__` is 0 and `Game.GetKeyIndex__` is -1: the reader keeps no
    ///   Sackbot names, so the name list is empty. Exact for a level without a named
    ///   Sackbot.
    /// - `ModalOverlay.IsPartyLeaderOverlayActive__` is false: nothing here opens a
    ///   modal overlay (start menu, Popit, prompt).
    /// - `Game.GetNumKnownItemsCollectedAtStart__` is 0: a fresh profile, as the empty
    ///   progression flags.
    ///
    /// D270, INFERRED: `Game.IsStartMenuOpen__` and `Pod.IsTakingPhoto__Q5Thing` are
    /// false. Nothing here opens the start menu, and no player has a Popit to take a
    /// photo with.
    ///
    /// D271: `Audio.IsVoiceOverPlaying__` is false: no line ever starts
    /// ([`LevelWorld::media`]), and false is the game's answer while none plays (FOUND).
    /// `Profile.HasPlayedLBP1AndUnlockedPod__` and `...UnlockedMoon__` are false
    /// (INFERRED: a profile without save data of the first game, as the fresh profile
    /// of the progression flags).
    fn state_query(&mut self, native: &str) -> Result<Value, String> {
        Ok(match native {
            "GameMessagingManager.Messaging_ShouldFilterMessageStrings__" => {
                Value::Bool(self.level_slot_id.0 != 2)
            }
            "Game.GetGameFrame__" => Value::Int(if self.world.is_some() {
                self.pworld.frame
            } else {
                0
            }),
            "Game.GetNumYellowHeadAvatars__" => Value::Int(self.players.len() as i32),
            "Score_Board.GetGameScoreDataType__" => Value::Int(if self.pworld.game_mode == 1 {
                6
            } else {
                (self.player_records.len() as i32 - 1).max(0)
            }),
            "OffScreenIndicator.GetPlayerIndicatorActivityType__i"
            | "Game.GetNumKeyNames__Q25Gooey.EGooeyNetworkAction"
            | "Game.GetNumKnownItemsCollectedAtStart__" => Value::Int(0),
            "Game.GetKeyIndex__Q25Gooey.EGooeyNetworkActionQ6String" => Value::Int(-1),
            "ModalOverlay.IsPartyLeaderOverlayActive__"
            | "Game.IsStartMenuOpen__"
            | "Pod.IsTakingPhoto__Q5Thing"
            | "Audio.IsVoiceOverPlaying__"
            | "Profile.HasPlayedLBP1AndUnlockedPod__"
            | "Profile.HasPlayedLBP1AndUnlockedMoon__" => Value::Bool(false),
            "Game.IsVcrPaused__" | "Game.IsPSNDemoBuild__" => Value::Bool(false),
            "Game.IsTutorialLevel__" => Value::Bool(self.tutorial_level),
            "Game.GetPartyLeadersPlayerNumber__" => {
                Value::Int(self.player_records.first().map_or(-1, |r| r.number))
            }
            "Game.IsNonInteractive__"
            | "Video.IsVideoPlaying__"
            | "System.Is4to3__"
            | "Audio.UserMusicIsPaused__"
            | "Input.IsInAttractionLoop__i" => Value::Bool(false),
            "Audio.UserMusicIsStopped__" | "Audio.IsInteractiveMusicAudible__" => Value::Bool(true),
            "Game.GetNumPendingLinks__"
            | "Game.GetNumAvatars__"
            | "Game.GetLocalLeadersPlayerNumber__"
            | "Audio.GetCurrMusicGuid__" => Value::Int(0),
            _ => return Err(format!("world does not provide {native}")),
        })
    }
    /// Counted by name; nothing plays.
    fn audio_command(&mut self, name: &str, _args: &[Value]) -> Result<(), String> {
        *self.audio_commands.entry(name.to_owned()).or_default() += 1;
        Ok(())
    }
    /// Every bit of the mask is set in [`LevelWorld::progression_flags`]. A mask of 0 is
    /// set on any profile, as in the game's test.
    fn progression_flag(&mut self, mask: u32) -> Result<bool, String> {
        Ok(self.progression_flags & mask == mask)
    }
    /// ORs the mask into [`LevelWorld::progression_flags`]. The game also marks the
    /// profile as changed; nothing is saved here.
    fn set_progression_flags(&mut self, mask: u32) -> Result<(), String> {
        self.progression_flags |= mask;
        Ok(())
    }
    /// [`LevelWorld::pod_computer_in_use`].
    fn pod_computer_in_use(&mut self) -> Result<bool, String> {
        Ok(self.pod_computer_in_use)
    }
    /// No crash zoom in the PC run (INFERRED default).
    fn crash_zoom(&mut self) -> Result<bool, String> {
        Ok(false)
    }
    fn create_gooey(&mut self, _arg: i32) -> Result<Value, String> {
        let o = self
            .gooey_pool
            .get(self.gooeys_created)
            .copied()
            .ok_or_else(|| format!("Gooey pool of {} used up", self.gooey_pool.len()))?;
        self.gooeys_created += 1;
        Ok(Value::Object(Some(o)))
    }
    fn set_camera(&mut self, camera: CameraFromToUp) -> Result<(), String> {
        self.camera.from_to_up = Some(camera);
        self.camera.from_to_up_calls += 1;
        Ok(())
    }
    fn set_dof_and_fog(&mut self, values: [f32; 3]) -> Result<(), String> {
        self.camera.dof_and_fog = Some(values);
        Ok(())
    }
    fn clear_motion_blur(&mut self) -> Result<(), String> {
        self.camera.motion_blur_clears += 1;
        Ok(())
    }
    /// The two vectors as passed; nothing in a level without a PWorld, which has no
    /// level camera (FOUND: the native does nothing without one).
    fn set_override_camera(
        &mut self,
        position: [f32; 4],
        direction: [f32; 4],
    ) -> Result<(), String> {
        let o = &mut self.camera.override_camera;
        o.set_calls += 1;
        if self.world.is_some() {
            (o.position, o.direction) = (position, direction);
        }
        Ok(())
    }
    /// The blend's target and rate; the blend itself moves in
    /// [`LevelWorld::step_camera`]. Nothing in a level without a PWorld.
    fn set_override_camera_activation(&mut self, on: bool, rate: f32) -> Result<(), String> {
        let o = &mut self.camera.override_camera;
        o.activation_calls += 1;
        o.activations_on += u64::from(on);
        if self.world.is_some() {
            (o.target, o.rate) = (if on { 1. } else { 0. }, rate);
        }
        Ok(())
    }
    /// [`LevelWorld::camera_matrix`].
    fn camera_matrix(&mut self) -> Result<[f32; 16], String> {
        self.camera.matrix_queries += 1;
        Ok(LevelWorld::camera_matrix(self))
    }
    /// [`LevelWorld::screen_position`].
    fn screen_position(&mut self, point: [f32; 4]) -> Result<[f32; 4], String> {
        Ok(LevelWorld::screen_position(self, point))
    }
    /// [`SCREEN`], whatever a display's window measures.
    fn screen_size(&mut self) -> Result<[i32; 2], String> {
        Ok(SCREEN)
    }
    /// No cinema mode in the PC run (INFERRED default).
    fn cinema_mode_count(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    fn write(&mut self, text: &str) {
        if self.written.len() < 1000 {
            self.written.push(text.to_owned());
        }
    }
}

/// Gooey objects made ahead for `Gooey.CreateGooey__i`.
pub const GOOEY_POOL: usize = 16;

/// Runs `function` (`.init__`, `.ctor__`) on a heap object with it as `this` in register 0;
/// a fault comes back as text with its site.
fn run_on_object(
    vm: &mut Vm,
    host: &mut dyn Host,
    class: ClassId,
    object: ObjectId,
    function: &str,
    budget: u64,
) -> Result<(), String> {
    if !vm.has_code(class, function) {
        return Ok(());
    }
    let mut b = budget;
    let regs = [(0, Value::Object(Some(object)))];
    match vm.start_registers(host, class, function, &regs, &mut b) {
        Ok(Step::Done(_)) => Ok(()),
        Ok(Step::Yielded(_)) => Err(format!("{function} yielded")),
        Err(f) => {
            let site = vm.last_site().map_or(String::new(), |s| s.to_string());
            Err(format!("{function}: {f:?} at {site}"))
        }
    }
}

/// Makes the engine-owned script objects natives hand out: [`GOOEY_POOL`] Gooey objects
/// (`.init__`) and the PodComputer (`.init__`, then `.ctor__`). INFERRED: the engine holds
/// one PodComputer and creates Gooeys on demand. The PodComputer is registered before its
/// `.init__` runs, because that code already asks for it (`GetPodComputer__`). Returns the
/// faults, if any; objects whose `.init__`/`.ctor__` faulted are still kept.
pub fn add_engine_objects(
    vm: &mut Vm,
    host: &mut Bindings<LevelWorld>,
    budget: u64,
) -> Vec<String> {
    let mut faults = Vec::new();
    let make = |vm: &mut Vm, host: &mut Bindings<LevelWorld>, class, pod: bool| {
        let o = vm.instantiate(class).map_err(|f| format!("{f:?}"))?;
        if pod {
            host.world.pod_computer = Some(o);
        } else {
            host.world.gooey_pool.push(o);
        }
        run_on_object(vm, host, class, o, ".init__", budget)?;
        if pod {
            run_on_object(vm, host, class, o, ".ctor__", budget)?;
        }
        Ok::<(), String>(())
    };
    if let Some(class) = vm.class_by_name("Gooey") {
        for _ in 0..GOOEY_POOL {
            if let Err(e) = make(vm, host, class, false) {
                faults.push(format!("Gooey {e}"));
            }
        }
    }
    // Planets and its crafteroids: INFERRED index 1 = the level's Earth (a PlanetThing),
    // 2 = its OrbitingMoon, from PlanetsView, which reads crafteroids 1 and 2 and returns
    // early when either Thing is null.
    if let Some(class) = vm.class_by_name("Planets") {
        match vm.instantiate(class) {
            Ok(o) => {
                host.world.planets = Some(o);
                if let Err(e) = run_on_object(vm, host, class, o, ".init__", budget) {
                    faults.push(format!("Planets {e}"));
                }
            }
            Err(f) => faults.push(format!("Planets {f:?}")),
        }
    }
    if let Some(class) = vm.class_by_name("Planet") {
        for name in ["Earth", "OrbitingMoon"] {
            let thing = host.world.things.iter().position(|t| {
                t.script_guid
                    .and_then(|g| vm.class_by_guid(g))
                    .is_some_and(|c| vm.class_name(c) == name)
            });
            match vm.instantiate(class) {
                Ok(o) => host
                    .world
                    .crafteroids
                    .push((o, thing.and_then(|t| u32::try_from(t).ok()))),
                Err(f) => faults.push(format!("Planet {f:?}")),
            }
        }
    }
    if let Some(class) = vm.class_by_name("PodComputer") {
        if let Err(e) = make(vm, host, class, true) {
            faults.push(format!("PodComputer {e}"));
        }
    }
    // The SlotID objects of `Game.GetLevelSlotID__` and `GetPrimaryLevelSlotID__`. FOUND:
    // the game builds a new SlotID (fields Type and SlotNumber) on every call, from the
    // slot the game state holds. Here one object each is made ahead and handed out every
    // time (the scripts of the levels only read it). INFERRED: Type 0 and SlotNumber 0,
    // an empty slot: the level's own slot is in the game's slot list, not in the level
    // file, and is not known here. A caller that knows the slot sets
    // `LevelWorld::level_slot_id` / `primary_level_slot_id` before this runs.
    if let Some(class) = vm.class_by_name("SlotID") {
        for primary in [false, true] {
            let (kind, number) = if primary {
                host.world.primary_level_slot_id
            } else {
                host.world.level_slot_id
            };
            match vm.instantiate(class) {
                Ok(o) => {
                    if let Some(fields) = vm.fields_mut(o) {
                        fields.insert("Type".into(), Value::Int(kind));
                        fields.insert("SlotNumber".into(), Value::Int(number));
                    }
                    if primary {
                        host.world.primary_level_slot = Some(o);
                    } else {
                        host.world.level_slot = Some(o);
                    }
                }
                Err(f) => faults.push(format!("SlotID {f:?}")),
            }
        }
        // The (0, 0) SlotID of `Slot.GetSlotGroup__Q6SlotID` (the game builds a new one
        // on every call; the scripts only read it).
        match vm.instantiate(class) {
            Ok(o) => {
                if let Some(fields) = vm.fields_mut(o) {
                    fields.insert("Type".into(), Value::Int(0));
                    fields.insert("SlotNumber".into(), Value::Int(0));
                }
                host.world.no_group_slot = Some(o);
            }
            Err(f) => faults.push(format!("SlotID {f:?}")),
        }
    }
    faults.dedup();
    faults
}

/// A scripted Thing and its class, when the class is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instance {
    pub thing: u32,
    pub guid: u32,
    pub class: Option<ClassId>,
}

/// Every Thing with a PScript resource GUID, in Thing order.
pub fn instances(vm: &Vm, world: &LevelWorld) -> Vec<Instance> {
    world
        .things
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            let guid = t.script_guid?;
            Some(Instance {
                thing: u32::try_from(i).ok()?,
                guid,
                class: vm.class_by_guid(guid),
            })
        })
        .collect()
}

/// Result of starting or resuming one script.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The class (or its superclasses) has no code for the function.
    NoFunction,
    /// The script's class is not loaded.
    NoClass,
    Done,
    Yielded,
    Fault(String),
}

/// Runs entry functions on scripted Things and keeps yielded ones for later ticks.
#[derive(Default)]
pub struct Runner {
    tasks: Vec<(u32, Task)>,
}

impl Runner {
    /// Scripts suspended by `System.Yield`, waiting for the next tick.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }

    /// Things with a suspended script.
    pub fn pending_things(&self) -> BTreeSet<u32> {
        self.tasks.iter().map(|(t, _)| *t).collect()
    }

    /// Runs `function` on one instance with the Thing as `this` in register 0.
    pub fn start(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        inst: Instance,
        function: &str,
        budget: &mut u64,
    ) -> Outcome {
        self.start_with(vm, host, inst, function, &[], budget)
    }

    /// [`Runner::start`] with argument registers (byte offsets) staged after `this`.
    pub fn start_with(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        inst: Instance,
        function: &str,
        args: &[(u16, Value)],
        budget: &mut u64,
    ) -> Outcome {
        let Some(class) = inst.class else {
            return Outcome::NoClass;
        };
        if !vm.has_code(class, function) {
            return Outcome::NoFunction;
        }
        let mut regs = vec![(0, Value::Thing(Some(inst.thing)))];
        regs.extend_from_slice(args);
        let step = vm.start_registers(host, class, function, &regs, budget);
        self.settle(vm, inst.thing, step)
    }

    /// Resumes every yielded script once, in the order they yielded.
    pub fn tick(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        budget: &mut u64,
    ) -> Vec<(u32, Outcome)> {
        let tasks = std::mem::take(&mut self.tasks);
        let mut out = Vec::with_capacity(tasks.len());
        for (thing, task) in tasks {
            let step = vm.resume(host, task, budget);
            out.push((thing, self.settle(vm, thing, step)));
        }
        out
    }

    fn settle(&mut self, vm: &Vm, thing: u32, step: Result<Step, lbp_script_vm::Fault>) -> Outcome {
        match step {
            Ok(Step::Done(_)) => Outcome::Done,
            Ok(Step::Yielded(task)) => {
                self.tasks.push((thing, task));
                Outcome::Yielded
            }
            Err(f) => {
                let site = vm.last_site().map_or(String::new(), |s| s.to_string());
                Outcome::Fault(format!("{f:?} at {site}"))
            }
        }
    }
}

/// Function the game runs once when a scripted Thing is created.
pub const CREATE: &str = "OnCreate__";
/// Function the game runs on every scripted Thing each tick.
pub const UPDATE: &str = "Update__";
/// Function the game runs each frame on the Thing that holds a global-settings slot
/// ([`LevelScripts::apply_global_settings`]).
pub const APPLY_GLOBAL_SETTINGS: &str = "ApplyGlobalSettings__";
/// What a started script thread runs (`Thread.ThreadMain__`: OnStart, Run, OnFinish).
pub const THREAD_MAIN: &str = "ThreadMain__";
/// Thread starts handled in one step (threads can start threads).
pub const MAX_THREAD_STARTS: usize = 256;
/// Game logic rate (X1, CONFIRMED_FROM_LOCAL_DATA in RPCS3: 30 ticks per second).
pub const TICK_HZ: u32 = 30;

/// What a [`FrameEntry`] builds its arguments from.
pub struct FrameContext<'a> {
    pub vm: &'a Vm,
    pub instances: &'a [Instance],
    /// Ticks run before this one.
    pub tick: u64,
}

/// A per-tick function the engine calls with arguments on instances of one class, in
/// place of [`UPDATE`] (an `export` entry no script calls).
pub struct FrameEntry {
    /// Exact script class name of the instance.
    pub class: &'static str,
    pub function: &'static str,
    /// Argument registers (byte offsets) and values; `this` is register 0.
    pub args: fn(&FrameContext<'_>) -> Vec<(u16, Value)>,
}

/// PlanetsWorld (the Pod's planet view): no script calls the export
/// `UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb`, and it is the only caller of
/// `UpdateState__` (private), `UpdatePodCamera__Q11PlanetThingbb` and
/// `UpdateInteractiveMusic__`, so the engine calls it each frame (INFERRED from the
/// listings; `OnEnterPod__`/`OnExitPod__` are called by PodComputer on pod entry/exit).
/// Arguments (INFERRED from how the function uses them): +4 bool, passed on to
/// `UpdatePodCamera` (false); +8 the Earth (a PlanetThing; it gets `EarthSpinOffset`); +12 the
/// moon (`MoonSpinOffset`); +16 bool "snap the view" (true on the first tick only); +17
/// bool "take the zoom from the pod computer's state" (true).
fn planets_view_args(cx: &FrameContext<'_>) -> Vec<(u16, Value)> {
    let find = |name: &str| {
        cx.instances
            .iter()
            .find(|i| i.class.is_some_and(|c| cx.vm.class_name(c) == name))
            .map(|i| i.thing)
    };
    vec![
        (4, Value::Bool(false)),
        (8, Value::Thing(find("Earth"))),
        (12, Value::Thing(find("OrbitingMoon"))),
        (16, Value::Bool(cx.tick == 0)),
        (17, Value::Bool(true)),
    ]
}

/// Per-tick entries other than [`UPDATE`], by class (docs/level-scripts.md "Part 6").
pub const FRAME_ENTRIES: &[FrameEntry] = &[FrameEntry {
    class: "PlanetsWorld",
    function: "UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb",
    args: planets_view_args,
}];

/// The per-tick entry for a class, if it has one other than [`UPDATE`].
pub fn frame_entry(vm: &Vm, class: ClassId) -> Option<&'static FrameEntry> {
    let name = vm.class_name(class);
    FRAME_ENTRIES.iter().find(|e| e.class == name)
}

/// Outcomes of one schedule step, by kind, with the faults.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepReport {
    pub done: usize,
    pub yielded: usize,
    pub resumed: usize,
    pub no_function: usize,
    pub no_class: usize,
    pub faults: Vec<(u32, String)>,
    /// Script threads started, finished (`ThreadMain__` returned) and faulted this step.
    pub threads_started: usize,
    pub threads_done: usize,
    pub thread_faults: Vec<String>,
    /// Trigger events raised, by [`TriggerEventKind`] (Enter, Inside, Leave).
    pub trigger_events: [usize; 3],
    /// Trigger events whose script function exists and was run, by kind.
    pub events_run: [usize; 3],
    /// Trigger events with no script to call (none on the trigger's Thing or its parent).
    pub events_without_script: usize,
    /// Trigger events for a Thing already stopped by a fault.
    pub events_to_stopped: usize,
    /// What the switch pass did ([`LevelScripts::switch_reactions`]).
    pub reactions: ReactionReport,
    /// What the global-settings pass did ([`LevelScripts::apply_global_settings`]).
    pub global_settings: GlobalSettingsReport,
}

/// Counts of one or more switch passes. Unit: a visited output target (one Thing on one
/// port, once per edge); the maps are by script resource GUID of the target.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReactionReport {
    /// Output targets the pass visited ([`output_edge`]).
    pub targets_visited: usize,
    /// Visits with `on` true.
    pub targets_on: usize,
    /// `SwitchPerformReaction__b` calls made.
    pub calls_b: BTreeMap<u32, usize>,
    /// `SwitchPerformReaction__bf` calls made.
    pub calls_bf: BTreeMap<u32, usize>,
    /// Visited targets with no script.
    pub without_script: usize,
    /// Visited targets whose script is not on the list ([`reaction_for_script`]).
    pub not_whitelisted: BTreeMap<u32, usize>,
    /// Listed scripts whose Thing gets its object kind from a part tested before the
    /// script ([`kind_before_script`]); no call.
    pub shadowed_by_switch: BTreeMap<u32, usize>,
    /// Listed scripts whose class is not loaded or has no code for the function.
    pub no_function: BTreeMap<u32, usize>,
    /// Listed scripts already stopped by a fault.
    pub to_stopped: BTreeMap<u32, usize>,
}

impl ReactionReport {
    /// Adds `other`'s counts to this report.
    pub fn absorb(&mut self, other: ReactionReport) {
        self.targets_visited += other.targets_visited;
        self.targets_on += other.targets_on;
        self.without_script += other.without_script;
        for (mine, theirs) in [
            (&mut self.calls_b, other.calls_b),
            (&mut self.calls_bf, other.calls_bf),
            (&mut self.not_whitelisted, other.not_whitelisted),
            (&mut self.shadowed_by_switch, other.shadowed_by_switch),
            (&mut self.no_function, other.no_function),
            (&mut self.to_stopped, other.to_stopped),
        ] {
            for (guid, n) in theirs {
                *mine.entry(guid).or_default() += n;
            }
        }
    }
}

impl StepReport {
    /// Adds `other`'s counts and faults to this report.
    pub fn absorb(&mut self, other: StepReport) {
        self.done += other.done;
        self.yielded += other.yielded;
        self.resumed += other.resumed;
        self.no_function += other.no_function;
        self.no_class += other.no_class;
        self.faults.extend(other.faults);
        self.threads_started += other.threads_started;
        self.threads_done += other.threads_done;
        self.thread_faults.extend(other.thread_faults);
        for (a, b) in self.trigger_events.iter_mut().zip(other.trigger_events) {
            *a += b;
        }
        for (a, b) in self.events_run.iter_mut().zip(other.events_run) {
            *a += b;
        }
        self.events_without_script += other.events_without_script;
        self.events_to_stopped += other.events_to_stopped;
        self.reactions.absorb(other.reactions);
        self.global_settings.absorb(other.global_settings);
    }

    fn add(&mut self, thing: u32, o: Outcome) {
        match o {
            Outcome::Done => self.done += 1,
            Outcome::Yielded => self.yielded += 1,
            Outcome::NoFunction => self.no_function += 1,
            Outcome::NoClass => self.no_class += 1,
            Outcome::Fault(f) => self.faults.push((thing, f)),
        }
    }
}

/// Per-tick script schedule for one level.
///
/// INFERRED (docs/level-scripts.md): `create` runs [`CREATE`] on every instance in Thing
/// order; each `tick` first resumes scripts suspended by `System.Yield` (in the order
/// they yielded), then runs [`UPDATE`] on every instance that was not suspended at the
/// start of the tick, in Thing order. A Thing whose script faults is not run again.
/// A class listed in [`FRAME_ENTRIES`] runs that entry with its arguments instead of
/// [`UPDATE`].
///
/// Script threads (`Thread.Start__` -> `Thread.StartImpl__(this)`, docs/script-vm.md):
/// a thread started during a step runs `ThreadMain__` on the thread object at the end of
/// that step, until its first `System.Yield`; each later tick resumes it once, after the
/// Things' suspended scripts and before the updates (order INFERRED). A faulted thread
/// is dropped.
pub struct LevelScripts {
    pub instances: Vec<Instance>,
    runner: Runner,
    /// Suspended script threads, in start order.
    threads: Vec<(ObjectId, Task)>,
    /// Things stopped by a fault.
    pub stopped: BTreeSet<u32>,
    /// Ticks run so far.
    pub ticks: u64,
}

impl LevelScripts {
    pub fn new(vm: &Vm, world: &LevelWorld) -> Self {
        Self {
            instances: instances(vm, world),
            runner: Runner::default(),
            threads: Vec::new(),
            stopped: BTreeSet::new(),
            ticks: 0,
        }
    }

    /// Scripts suspended by `System.Yield`.
    pub fn pending(&self) -> usize {
        self.runner.pending()
    }

    /// Script threads waiting for the next tick.
    pub fn threads(&self) -> usize {
        self.threads.len()
    }

    /// Runs [`CREATE`] on every instance; `budget` is per call.
    pub fn create(&mut self, vm: &mut Vm, host: &mut dyn Host, budget: u64) -> StepReport {
        let mut report = self.run_all(vm, host, Some(CREATE), budget, &BTreeSet::new());
        self.start_threads(vm, host, budget, &mut report);
        report
    }

    fn settle_thread(
        &mut self,
        vm: &Vm,
        id: ObjectId,
        step: Result<Step, lbp_script_vm::Fault>,
        report: &mut StepReport,
    ) {
        match step {
            Ok(Step::Done(_)) => report.threads_done += 1,
            Ok(Step::Yielded(task)) => self.threads.push((id, task)),
            Err(f) => {
                let class = vm.class_of(id).map_or("?", |c| vm.class_name(c));
                let site = vm.last_site().map_or(String::new(), |s| s.to_string());
                report
                    .thread_faults
                    .push(format!("thread {class}: {f:?} at {site}"));
            }
        }
    }

    /// Runs threads started since the last step until their first yield; threads they
    /// start run too, up to [`MAX_THREAD_STARTS`] per step.
    fn start_threads(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        budget: u64,
        report: &mut StepReport,
    ) {
        let mut started = 0;
        loop {
            let new = vm.take_started_threads();
            if new.is_empty() {
                break;
            }
            for id in new {
                started += 1;
                if started > MAX_THREAD_STARTS {
                    report
                        .thread_faults
                        .push("thread start limit reached".into());
                    return;
                }
                report.threads_started += 1;
                let Some(class) = vm.class_of(id) else {
                    report
                        .thread_faults
                        .push(format!("thread object {} has no class", id.0));
                    continue;
                };
                let mut b = budget;
                let step = vm.start_registers(
                    host,
                    class,
                    THREAD_MAIN,
                    &[(0, Value::Object(Some(id)))],
                    &mut b,
                );
                self.settle_thread(vm, id, step, report);
            }
        }
    }

    /// Runs the level's triggers against the players ([`LevelWorld::update_triggers`]) and
    /// calls the script function of each event with the Thing as its argument.
    ///
    /// FOUND in the 01.33 executable (0x135a18, 0x1346fc, 0x1364f0): Enter and Leave go
    /// to the script of the trigger's Thing, or to its parent's when that Thing has no
    /// script; Inside goes to the parent's script. (There the engine calls Inside for four
    /// object kinds only; here every parent script that has the function gets it.)
    /// INFERRED: events are delivered in trigger order.
    pub fn trigger_events(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = StepReport::default();
        for e in host.world.update_triggers() {
            report.trigger_events[e.kind as usize] += 1;
            let instance = |thing: u32| {
                let i = self.instances.binary_search_by_key(&thing, |i| i.thing);
                i.ok().map(|i| self.instances[i])
            };
            let parent = host
                .world
                .things
                .get(e.trigger as usize)
                .and_then(|t| t.parent)
                .and_then(instance);
            let target = match e.kind {
                TriggerEventKind::Inside => parent,
                _ => instance(e.trigger).or(parent),
            };
            let Some(inst) = target else {
                report.events_without_script += 1;
                continue;
            };
            if self.stopped.contains(&inst.thing) {
                report.events_to_stopped += 1;
                continue;
            }
            let mut b = budget;
            let args = [(4, Value::Thing(Some(e.thing)))];
            let o = self
                .runner
                .start_with(vm, host, inst, e.kind.function(), &args, &mut b);
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(inst.thing);
            }
            if !matches!(o, Outcome::NoFunction | Outcome::NoClass) {
                report.events_run[e.kind as usize] += 1;
            }
            report.add(inst.thing, o);
        }
        self.start_threads(vm, host, budget, &mut report);
        report
    }

    /// The per-tick switch pass ([`LevelWorld::switch_order`], [`LevelWorld::update_switch`])
    /// with the script reaction of each visited target.
    ///
    /// FOUND in the 01.33 executable (0x1244a4 -> 0x11ddf0): a switch's targets get the
    /// signal while that switch is updated, before the next switch in the order, so a
    /// reaction script sees later switches' outputs as they were last tick. The call goes
    /// to the script of the target Thing itself (no parent walk; no script, no call), and
    /// only for the scripts [`reaction_for_script`] lists:
    /// `SwitchPerformReaction__b(on)` with `on` from [`output_edge`], or for one script
    /// `SwitchPerformReaction__bf(on, level)`, `level` 1.0 when the output's new ternary
    /// is not 0, else 0.0 (it is not the activation).
    /// INFERRED: the argument registers (+4 bool, +8 float), as for the other entries.
    pub fn switch_reactions(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = StepReport::default();
        host.world.prepare_switches();
        host.world.refresh_tags();
        for switch in host.world.switch_order().to_vec() {
            for v in host.world.update_switch(switch) {
                let r = &mut report.reactions;
                r.targets_visited += 1;
                r.targets_on += usize::from(v.on);
                let target = host.world.things.get(v.target as usize);
                let Some(guid) = target.and_then(|t| t.script_guid) else {
                    r.without_script += 1;
                    continue;
                };
                let Some(kind) = reaction_for_script(guid) else {
                    *r.not_whitelisted.entry(guid).or_default() += 1;
                    continue;
                };
                if target.is_some_and(|t| kind_before_script(t).is_some()) {
                    *r.shadowed_by_switch.entry(guid).or_default() += 1;
                    continue;
                }
                let found = self.instances.binary_search_by_key(&v.target, |i| i.thing);
                let Ok(inst) = found.map(|i| self.instances[i]) else {
                    *r.no_function.entry(guid).or_default() += 1;
                    continue;
                };
                if self.stopped.contains(&v.target) {
                    *r.to_stopped.entry(guid).or_default() += 1;
                    continue;
                }
                let level = f32::from(u8::from(v.signal.ternary != 0));
                let (function, args) = match kind {
                    Reaction::Bool => (REACTION_B, vec![(4, Value::Bool(v.on))]),
                    Reaction::BoolFloat => (
                        REACTION_BF,
                        vec![(4, Value::Bool(v.on)), (8, Value::Float(level))],
                    ),
                };
                let mut b = budget;
                let o = self
                    .runner
                    .start_with(vm, host, inst, function, &args, &mut b);
                let r = &mut report.reactions;
                let count = match (&o, kind) {
                    (Outcome::NoFunction | Outcome::NoClass, _) => &mut r.no_function,
                    (_, Reaction::Bool) => &mut r.calls_b,
                    (_, Reaction::BoolFloat) => &mut r.calls_bf,
                };
                *count.entry(guid).or_default() += 1;
                if matches!(o, Outcome::Fault(_)) {
                    self.stopped.insert(inst.thing);
                }
                report.add(inst.thing, o);
            }
        }
        self.start_threads(vm, host, budget, &mut report);
        report
    }

    /// The per-frame call of the global settings: for each of the eight slots in order,
    /// `ApplyGlobalSettings__()` on the scripted Thing that holds the slot
    /// ([`LevelWorld::global_settings_holder`], which also clears a slot whose Thing is
    /// gone).
    ///
    /// FOUND in the 01.33 executable (0xd5e84 -> 0x88ff8c -> 0x620bd8): the PWorld's
    /// per-frame function calls the script function of that name, with no arguments, on
    /// the PScript of each slot's Thing, one slot after the other, so a call sees what
    /// the calls for the lower slots did. A Thing that holds two slots is called twice.
    /// It is the only caller (the one reference to 0x88ff8c), and no script calls the
    /// function (script listings). The global-settings script steps its own blend there:
    /// from `Game.GetGameFrame` and the frame it began at it writes a smoothed factor
    /// with `Game.SetGlobalSettingsBlendFactor`, and 1.0 on every call once its
    /// transition time is over; while its `IsTweaking` member is set it writes its values
    /// without a blend instead.
    /// INFERRED: a script without the function is passed over and keeps its slot (the
    /// game's call by name was not followed into the VM); a Thing stopped by a fault is
    /// not called, as everywhere in this schedule.
    pub fn apply_global_settings(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = StepReport::default();
        for kind in 0..GLOBAL_SETTINGS {
            let thing = match host.world.global_settings_holder(kind) {
                SlotHolder::Empty => continue,
                SlotHolder::Cleared => {
                    report.global_settings.slots_cleared += 1;
                    continue;
                }
                SlotHolder::Thing(thing) => thing,
            };
            if self.stopped.contains(&thing) {
                report.global_settings.to_stopped += 1;
                continue;
            }
            let found = self.instances.binary_search_by_key(&thing, |i| i.thing);
            let Ok(inst) = found.map(|i| self.instances[i]) else {
                report.global_settings.no_function += 1;
                continue;
            };
            let mut b = budget;
            let o = self
                .runner
                .start(vm, host, inst, APPLY_GLOBAL_SETTINGS, &mut b);
            match o {
                Outcome::NoFunction | Outcome::NoClass => {
                    report.global_settings.no_function += 1;
                    continue;
                }
                Outcome::Fault(_) => {
                    self.stopped.insert(thing);
                }
                Outcome::Done | Outcome::Yielded => {}
            }
            report.global_settings.calls[kind] += 1;
            report.add(thing, o);
        }
        self.start_threads(vm, host, budget, &mut report);
        report
    }

    /// One game tick of a level: [`Self::tick`], then [`Self::trigger_events`], so a
    /// script an event suspends is resumed on the next tick (order INFERRED), then
    /// [`Self::switch_reactions`], then [`Self::apply_global_settings`], then the
    /// camera's step ([`LevelWorld::step_camera`], place INFERRED).
    ///
    /// FOUND in the 01.33 executable: the world update (0xeacd4) calls the switch pass
    /// (0xdd228, at 0xeb270) after 0xe0f0c, 0xcfc40 and 0xcfbb4 and before 0xcfed4, and
    /// not while the game-wide pause test (0x18cf18) holds. Which of those neighbours are
    /// the trigger pass and the script update is not established, so the place of the
    /// switch pass after both here is INFERRED. The global-settings pass (0xd5e84, at
    /// 0xeb2a4) comes after the switch pass in the same function, with 0xcfed4, 0xdad5c,
    /// 0xd50b4 and 0xe26b0 between them: FOUND. Its place after `Update__` is INFERRED:
    /// the call that runs `Update__` on every scripted Thing was looked for and not
    /// found (the one engine function that calls a script function of that name by its
    /// name object, 0x625780, is reached from a single game-state object outside the
    /// world update, 0x3f8d24).
    pub fn tick_level(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = self.tick(vm, host, budget);
        report.absorb(self.trigger_events(vm, host, budget));
        report.absorb(self.switch_reactions(vm, host, budget));
        report.absorb(self.apply_global_settings(vm, host, budget));
        host.world.step_camera();
        host.world.advance_frame();
        host.world.end_pad_frame();
        report
    }

    /// One game tick; `budget` is per call or resume.
    pub fn tick(&mut self, vm: &mut Vm, host: &mut dyn Host, budget: u64) -> StepReport {
        // A Thing suspended at the start of the tick is resumed, not updated again.
        let busy = self.runner.pending_things();
        let mut resume_budget = budget.saturating_mul(self.runner.pending() as u64);
        let mut report = StepReport::default();
        for (thing, o) in self.runner.tick(vm, host, &mut resume_budget) {
            if matches!(o, Outcome::Done | Outcome::Yielded) {
                report.resumed += 1;
            }
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(thing);
                report.add(thing, o);
            }
        }
        for (id, task) in std::mem::take(&mut self.threads) {
            let mut b = budget;
            let step = vm.resume(host, task, &mut b);
            self.settle_thread(vm, id, step, &mut report);
        }
        let update = self.run_all(vm, host, None, budget, &busy);
        report.done += update.done;
        report.yielded += update.yielded;
        report.no_function += update.no_function;
        report.no_class += update.no_class;
        report.faults.extend(update.faults);
        self.start_threads(vm, host, budget, &mut report);
        self.ticks += 1;
        report
    }

    fn run_all(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        function: Option<&str>,
        budget: u64,
        skip: &BTreeSet<u32>,
    ) -> StepReport {
        let mut report = StepReport::default();
        for inst in self.instances.clone() {
            if skip.contains(&inst.thing) || self.stopped.contains(&inst.thing) {
                continue;
            }
            let mut b = budget;
            // `None`: the tick's entry, a frame entry or `Update__`.
            let entry = match function {
                Some(_) => None,
                None => inst.class.and_then(|c| frame_entry(vm, c)),
            };
            let o = match entry {
                Some(e) => {
                    let args = (e.args)(&FrameContext {
                        vm,
                        instances: &self.instances,
                        tick: self.ticks,
                    });
                    self.runner
                        .start_with(vm, host, inst, e.function, &args, &mut b)
                }
                None => {
                    let f = function.unwrap_or(UPDATE);
                    self.runner.start(vm, host, inst, f, &mut b)
                }
            };
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(inst.thing);
            }
            report.add(inst.thing, o);
        }
        report
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn part_names() {
        use super::part_name;
        assert_eq!(part_name("PSpriteLight"), Some("SPRITE_LIGHT"));
        assert_eq!(part_name("PJoint"), Some("JOINT"));
        assert_eq!(part_name("PSwitch"), Some("SWITCH"));
        assert_eq!(part_name("PYellowHead"), Some("YELLOWHEAD"));
        assert_eq!(part_name("YellowHead"), Some("YELLOWHEAD"));
        // A name that starts with its own `P` keeps it.
        assert_eq!(part_name("Pos"), Some("POS"));
        assert_eq!(part_name("PPos"), Some("POS"));
        assert_eq!(part_name("World"), Some("WORLD"));
        assert_eq!(part_name("MetaData"), Some("METADATA"));
        assert_eq!(part_name("MicroChip"), Some("MICROCHIP"));
        assert_eq!(part_name("WormHole"), Some("WORMHOLE"));
        assert_eq!(part_name("TagSynchroniser"), Some("TAG_SYNCHRONIZER"));
        assert_eq!(part_name("PTagSynchroniser"), Some("TAG_SYNCHRONIZER"));
        assert_eq!(part_name("Os"), None);
        assert_eq!(part_name("P"), None);
        assert_eq!(part_name(""), None);
        assert_eq!(part_name("Thing"), None);
    }

    /// Every part the reader knows is found by its script-side spelling (`SPRITE_LIGHT`
    /// as `SpriteLight`), bare as a native member and with the `P` of the part class.
    #[test]
    fn every_part_name_maps_back() {
        for &(part, ..) in lbp_level::thing::PARTS {
            let camel: String = part
                .split('_')
                .map(|w| w[..1].to_owned() + &w[1..].to_ascii_lowercase())
                .collect();
            assert_eq!(super::part_name(&camel), Some(part), "{camel}");
            assert_eq!(
                super::part_name(&format!("P{camel}")),
                Some(part),
                "P{camel}"
            );
            assert_eq!(super::part_name(part), Some(part));
        }
    }
}
