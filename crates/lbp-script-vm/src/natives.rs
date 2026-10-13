//! Rs: host bindings for native functions, keyed by (declaring class, mangled name) as the
//! VM reports them. Written from recorded findings, never from game code
//! (docs/script-natives.md lists each binding with its evidence label).
//!
//! The engine side is a [`World`]; every method defaults to an explicit "not available"
//! error so a partial world (tests, the viewer) still runs the pure bindings. [`Bindings`]
//! adapts a world to the VM's [`Host`] and counts calls to natives that have no binding
//! yet, in the order the G10 census ranked them. Such a call faults the script, or, with
//! [`Bindings::default_unbound`], returns the default of the call's return type so the
//! script runs on; either way it is counted as unbound, never as bound.
use crate::signature;
use crate::{Host, MemberKind, Value};
use std::collections::BTreeMap;

/// The signal of the switch output wired to one input port of a Thing. FOUND in the 01.33
/// executable (readers 0x7541e0, 0x754308, 0x7542a4, 0x75437c, 0x754ccc): a Thing has a
/// table of input ports, each empty or pointing at a SwitchOutput, whose signal is an
/// activation (f32), a ternary (s32) and a player (s32, -1 = none).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SwitchInput {
    pub activation: f32,
    pub ternary: i32,
    pub player: i32,
}

/// Script resource GUID of the scoreboard class, which `BaseWorld.IsScoreBoard__Q5Thing`
/// tests against (FOUND, 01.33: the constant the test at 0x622658 loads its script by).
pub const SCORE_BOARD_SCRIPT: u32 = 11599;

/// One `Cam.SetCameraFromToUp__rrrffff` call: eye position, look-at point, up vector and
/// the four trailing floats as passed (PlanetsWorld passes 10, 100000, 0, 1; INFERRED near
/// and far planes then two unknowns).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CameraFromToUp {
    pub from: [f32; 4],
    pub to: [f32; 4],
    pub up: [f32; 4],
    pub params: [f32; 4],
}

/// One player's record in the game state. FOUND in the 01.33 executable (finder 0x156b18,
/// by index 0x156b74): the game-state object holds an array of records and their count;
/// a record is found by its player number. The fields below are the ones the score and
/// player natives read and write (record offsets +0x30 score, +0x38 counter, +0x3c timer,
/// +0x40 source, +0x60 visible, +0xdc number, +0xe0 team).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRecord {
    /// `Game.EPlayerNumber` (an s32).
    pub number: i32,
    pub team: i32,
    pub score: f32,
    /// The score multiplier is `counter / 5 + 1`.
    pub multiplier_counter: i32,
    /// Ticks left before the multiplier counter is lost (the scripts count it down).
    pub multiplier_timer: i32,
    /// The Thing the multiplier display is drawn at (null = none).
    pub multiplier_source: Option<u32>,
    pub multiplier_visible: bool,
    /// The record's pad object (record +0x6a0, a pointer); `None` = no pad, for which
    /// every `Pad` query is false.
    pub pad: Option<PadState>,
    /// The player's colour, 0xAARRGGBB: what `Native.GetPlayerColour__Q5Thing` unpacks.
    /// FOUND (01.33, 0x139308, 0x15894c): the first of four 32-bit colours of an object
    /// the record points at (record +0x16c, object +0x4c). INFERRED: that object is the
    /// player's profile. Who writes the colour and what a new profile holds were not
    /// read, so the start value here is [`PLACEHOLDER_PLAYER_COLOUR`].
    pub colour: u32,
}

/// PLACEHOLDER, not a game value: the colour a [`PlayerRecord`] starts with (opaque
/// white). The game's stored colour and its default per player were not established.
pub const PLACEHOLDER_PLAYER_COLOUR: u32 = 0xffff_ffff;

impl Default for PlayerRecord {
    fn default() -> Self {
        Self {
            number: 0,
            team: 0,
            score: 0.,
            multiplier_counter: 0,
            multiplier_timer: 0,
            multiplier_source: None,
            multiplier_visible: false,
            pad: None,
            colour: PLACEHOLDER_PLAYER_COLOUR,
        }
    }
}

/// A 32-bit colour whose bytes are A, R, G, B from the top as the vector (R, G, B, A),
/// each byte divided by 255. FOUND (01.33, 0x6d88b0): how `Native.GetPlayerColour`
/// turns the stored word into its answer.
pub fn colour_vector(argb: u32) -> [f32; 4] {
    let byte = |shift: u32| ((argb >> shift) & 0xff) as f32 / 255.;
    [byte(16), byte(8), byte(0), byte(24)]
}

/// The action number the Pod's world script passes to `Input.IsJustClickedByID` to
/// engage the pod computer. FOUND (01.33, 0x6d2f18, 0x3890a0): the "button" of that
/// native is a number of an action enumeration of 99 entries, not a pad mask.
pub const ACTION_ENGAGE_POD: i32 = 8;
/// The context mask of action 8. FOUND (01.33, table of 0x38677c): the only action with
/// this mask. INFERRED: bit 0x1 is gameplay (the mode 1 the level scripts pass to the
/// `Pad` natives) and bit 0x1000 the Pod.
pub const ACTION_ENGAGE_POD_CONTEXT: u32 = 0x1001;
/// The bit of the pad's button word that is action 8 on an ordinary pad. FOUND (01.33,
/// 0x38760c, case 8). The name of the button is not established. Left out: the same
/// case reads other words of the pad block when one of two bytes of it is set (INFERRED
/// other input devices); here those bytes are 0.
pub const ACTION_ENGAGE_POD_BUTTON: u32 = 0x80;
/// The bit `Video.PressedStartButton__` looks for. FOUND (01.33, 0x756520): bit 0x80000
/// of a word of the low-level pad records. INFERRED: the same numbering as the button
/// word of [`PadState`] (a script passes 0x80000 to a `Pad` native for Start).
pub const START_BUTTON: u32 = 0x80000;
/// How many pads `Video.PressedStartButton__` looks at (FOUND, 0x756520).
pub const START_BUTTON_PADS: i32 = 7;

/// A player's pad as the `Pad` natives read it. FOUND in the 01.33 executable (readers
/// 0x388358, 0x389400, 0x388e2c): two mode masks (pad +0x1c, +0x20), the button word of
/// this frame (+0x50) and the one of the frame before (+0xa0); the game copies the
/// current block over the old one once a frame (0x3866e8).
///
/// A button argument is a mask and all of its bits must be down. FOUND for four bits of
/// the word (the stick emulation at 0x688458 reads them): 0x100000 up, 0x200000 right,
/// 0x400000 down, 0x800000 left. The other bits are INFERRED at best and have no names
/// here. INFERRED: the masks say which input consumer owns the pad this frame (the
/// level scripts pass mode 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PadState {
    pub mask_a: u32,
    pub mask_b: u32,
    pub now: u32,
    pub old: u32,
    /// Buttons a consumer used up: they stay out of `now` until they are released.
    /// FOUND (01.33, 0x687c6c) that the use-up call of `Video.PressedStartButton__`
    /// takes the mask out of three words of the low-level pad record and ORs it into a
    /// fourth; INFERRED that the fourth means "ignore until released".
    pub held_off: u32,
}

impl PadState {
    /// The buttons of a new frame: a held-off button that is no longer down is free
    /// again, one that is still down stays out of the word.
    pub fn set_buttons(&mut self, buttons: u32) {
        self.held_off &= buttons;
        self.now = buttons & !self.held_off;
    }
    /// Uses a press up: the buttons leave this frame's word and stay out until released.
    pub fn use_up(&mut self, buttons: u32) {
        self.now &= !buttons;
        self.held_off |= buttons;
    }
    /// `Input.IsJustClickedByID` for one action: the action's context mask has a bit in
    /// both mode masks, the action is down now and was not down the frame before. FOUND
    /// (01.33, 0x3890a0). `None` for an action whose context and test are not read here
    /// (every action but [`ACTION_ENGAGE_POD`]).
    pub fn is_just_clicked(&self, action: i32) -> Option<bool> {
        (action == ACTION_ENGAGE_POD).then(|| {
            self.shows_prompt(action) == Some(true)
                && ACTION_ENGAGE_POD_CONTEXT & self.mask_b != 0
                && self.now & ACTION_ENGAGE_POD_BUTTON != 0
                && self.old & ACTION_ENGAGE_POD_BUTTON == 0
        })
    }
    /// Whether `Input.IsJustClickedByID` asks for its on-screen prompt: the action's
    /// context mask has a bit in the first mode mask (FOUND, 0x3890a0). `None` as
    /// [`PadState::is_just_clicked`].
    pub fn shows_prompt(&self, action: i32) -> Option<bool> {
        (action == ACTION_ENGAGE_POD).then_some(ACTION_ENGAGE_POD_CONTEXT & self.mask_a != 0)
    }
    /// `Pad.IsPressed`: the mode is in the first mask and every bit of `buttons` is down.
    pub fn is_pressed(&self, mode: u32, buttons: u32) -> bool {
        mode & self.mask_a != 0 && buttons & self.now == buttons
    }
    /// `Pad.IsJustPressed`: the mode is in both masks, every bit of `buttons` is down
    /// now and they were not all down the frame before.
    pub fn is_just_pressed(&self, mode: u32, buttons: u32) -> bool {
        mode & self.mask_a != 0
            && mode & self.mask_b != 0
            && buttons & self.now == buttons
            && buttons & self.old != buttons
    }
    /// `Pad.IsJustReleased`: the mode is in both masks, the bits of `buttons` were all
    /// down the frame before and are not all down now.
    pub fn is_just_released(&self, mode: u32, buttons: u32) -> bool {
        mode & self.mask_a != 0
            && mode & self.mask_b != 0
            && buttons & self.now != buttons
            && buttons & self.old == buttons
    }
}

/// `Game.EGlobalSettingsType` values. FOUND (01.33, 0xd623c): the type indexes the
/// PWorld's array of 8 blend factors; 5 is the lighting group (five values under one
/// factor) and 7 audio; the others are one blended float each.
pub const GS_WATER_LEVEL: i32 = 0;
pub const GS_GRAVITY: i32 = 1;
pub const GS_WATER_WAVE_MAGNITUDE: i32 = 2;
pub const GS_WATER_TINT: i32 = 3;
pub const GS_WATER_MURKINESS: i32 = 4;
pub const GS_LIGHTING: i32 = 5;
pub const GS_WATER_BITS: i32 = 6;

/// The highest score `Game.SetPlayerScore` stores. FOUND (01.33, 0x3970e4): the value is
/// clamped between two constants read from the executable, 0.0 and 4,000,000.0.
pub const MAX_PLAYER_SCORE: f32 = 4_000_000.;

/// Mesh GUID `TriggerLevelLink.FindLockMesh__` looks for (FOUND, 01.33, 0x1343dc: the
/// constant 0xeb1 compared with the GUID of a PRenderMesh's mesh).
pub const LOCK_MESH: u32 = 0xeb1;

/// The two mesh GUIDs `Thing.GetPodThing__b` looks for (FOUND, 01.33, 0xd32dc: the two
/// constants compared with the GUID of a PRenderMesh's mesh): the Pod, and the platform
/// the Pod's world script swaps in while the pod computer is in use.
pub const POD_MESH: u32 = 0x2f6c;
pub const POD_PLATFORM_MESH: u32 = 0x23959;

/// What `Video.StartVideo__Q5Thinggb` answers when no video starts (FOUND, 01.33,
/// 0x67c620). The pod computer's scripts keep the answer as their video handle and take
/// any other value as "a video is playing".
pub const NO_VIDEO: i32 = -1;

/// The lowest world z at which `Native.GetIsInPadZone__Q5Thing` is true. FOUND (01.33,
/// 0x13bcec): the Sackboy's world position z is compared "greater or equal" with -300.
pub const PAD_ZONE_MIN_Z: f32 = -300.;

/// What `PodComputer.GetPlanetsBackgroundGUID__` answers while the planets level is not
/// loaded (FOUND, 01.33, 0x3fcbb4: the constant returned without that level).
pub const DEFAULT_PLANETS_BACKGROUND: u32 = 0x1d054;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Texture,
    Plan,
    Mesh,
}

fn not_available(what: &str) -> String {
    format!("world does not provide {what}")
}

/// Engine state the bindings read and change. Thing handles are the host's `u32` ids.
pub trait World {
    fn thing_uid(&mut self, _thing: u32) -> Result<i32, String> {
        Err(not_available("thing UIDs"))
    }
    fn parent(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    fn first_child(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    fn next_sibling(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    /// The level's world Thing.
    fn world_thing(&mut self) -> Result<Option<u32>, String> {
        Err(not_available("the world thing"))
    }
    /// The Thing's PPos world matrix in game memory order (translation at 12..15); `None`
    /// for a Thing without a PPos.
    fn world_transform(&mut self, _thing: u32) -> Result<Option<[f32; 16]>, String> {
        Err(not_available("transforms"))
    }
    /// The Thing's PPos local matrix (against its parent), as [`World::world_transform`].
    fn local_transform(&mut self, _thing: u32) -> Result<Option<[f32; 16]>, String> {
        Err(not_available("transforms"))
    }
    fn centre_of_mass(&mut self, _thing: u32) -> Result<[f32; 4], String> {
        Err(not_available("centres of mass"))
    }
    /// Stores the local matrix, works out the world matrix from the parent's and moves the
    /// Thing's descendants along. Nothing for a Thing without a PPos.
    fn set_local_transform(&mut self, _thing: u32, _m: [f32; 16]) -> Result<(), String> {
        Err(not_available("transforms"))
    }
    /// The Thing's "deactivated" flag.
    fn is_inactive(&mut self, _thing: u32) -> Result<bool, String> {
        Err(not_available("activity state"))
    }
    /// The Thing's microchip Thing when it has one, else the Thing itself.
    fn microchip_proxy(&mut self, _thing: u32) -> Result<u32, String> {
        Err(not_available("microchips"))
    }
    /// Does the Thing sit on a circuit board that belongs to a sequencer?
    fn is_on_sequencer(&mut self, _thing: u32) -> Result<bool, String> {
        Err(not_available("sequencers"))
    }
    /// Does the Thing carry this part (`"PJoint"`, `"PSwitch"`, ...)?
    fn has_part(&mut self, _thing: u32, _part: &str) -> Result<bool, String> {
        Err(not_available("parts"))
    }
    /// The signal wired to input port `port` of the Thing; `None` when the port is out of
    /// range (negative included) or nothing is wired to it.
    fn switch_input(&mut self, _thing: u32, _port: i32) -> Result<Option<SwitchInput>, String> {
        Err(not_available("switch inputs"))
    }
    /// The Thing whose PSwitch has an output wired to input port `port` of the Thing;
    /// `None` as [`World::switch_input`].
    fn switch_input_thing(&mut self, _thing: u32, _port: i32) -> Result<Option<u32>, String> {
        Err(not_available("switch inputs"))
    }
    /// Target `target` of output `output` of the Thing's own PSwitch: the Thing that
    /// output is wired to. `None` without a PSwitch, out of range, or for a null target.
    fn switch_output_target(
        &mut self,
        _thing: u32,
        _output: i32,
        _target: i32,
    ) -> Result<Option<u32>, String> {
        Err(not_available("switch outputs"))
    }
    /// How many Things are inside the Thing's PTrigger; 0 without one.
    fn trigger_count(&mut self, _thing: u32) -> Result<i32, String> {
        Err(not_available("triggers"))
    }
    /// Entry `index` of the Thing's PTrigger in-list; `None` without one or out of range.
    fn trigger_thing(&mut self, _thing: u32, _index: i32) -> Result<Option<u32>, String> {
        Err(not_available("triggers"))
    }
    /// Is the Thing's script the scoreboard script ([`SCORE_BOARD_SCRIPT`]) or a class
    /// derived from it? False without a PScript.
    fn is_score_board(&mut self, _thing: u32) -> Result<bool, String> {
        Err(not_available("script classes of Things"))
    }
    /// LAMS text by key (the strings the U3 text browser reads).
    fn translate(&mut self, _key: i32) -> Result<String, String> {
        Err(not_available("translated text"))
    }
    fn play_sound(&mut self, _thing: Option<u32>, _name: &str) -> Result<(), String> {
        Err(not_available("audio"))
    }
    /// Handle for a resource loaded by GUID.
    fn load_resource(&mut self, _kind: ResourceKind, _guid: u32) -> Result<u32, String> {
        Err(not_available("resources"))
    }
    fn num_players(&mut self) -> Result<i32, String> {
        Err(not_available("players"))
    }
    /// `Game.EPlayOrEditMode`: 0 play, 1 edit (INFERRED from script use, see
    /// docs/level-scripts.md "Part 2").
    fn play_or_edit_mode(&mut self) -> Result<i32, String> {
        Err(not_available("the play/edit mode"))
    }
    /// File handle of the level's dialogue list (s32, builtin 9 like the scripts'
    /// `DialogueListFile` field); 0 = none loaded.
    fn dialogue_list(&mut self) -> Result<i32, String> {
        Err(not_available("dialogue lists"))
    }
    /// The colour a shape shows in the editor, as a colour vector.
    fn shape_editor_colour(&mut self, _thing: u32) -> Result<[f32; 4], String> {
        Err(not_available("shape colours"))
    }
    fn set_shape_editor_colour(&mut self, _thing: u32, _colour: [f32; 4]) -> Result<(), String> {
        Err(not_available("shape colours"))
    }
    /// Tint of a Thing's shape (`SetShapeEditorColourTint`). Nothing for a Thing without
    /// a PShape.
    fn set_shape_tint(&mut self, _thing: u32, _colour: [f32; 4]) -> Result<(), String> {
        Err(not_available("shape colours"))
    }
    /// Tint of a Thing's render mesh in the editor colour (`SetRenderMeshEditorColourTint`,
    /// called next to `SetShapeEditorColour` with a colour vector). Nothing for a Thing
    /// without a PRenderMesh.
    fn set_render_mesh_tint(&mut self, _thing: u32, _colour: [f32; 4]) -> Result<(), String> {
        Err(not_available("render mesh tints"))
    }
    /// Editor colour of a Thing's render mesh (`SetRenderMeshEditorColour`). Nothing for
    /// a Thing without a PRenderMesh.
    fn set_render_mesh_colour(&mut self, _thing: u32, _colour: [f32; 4]) -> Result<(), String> {
        Err(not_available("render mesh colours"))
    }
    /// Whether a Thing's render mesh is visible in play mode
    /// (`PostItNote.IsVisibleInPlayMode`); false without a PRenderMesh.
    fn render_mesh_visible_in_play_mode(&mut self, _thing: u32) -> Result<bool, String> {
        Err(not_available("render mesh visibility"))
    }
    /// Shows or hides a Thing's render mesh and those of the children the game's call
    /// goes on to (`Thing.ShowRenderMesh`, `Thing.HideRenderMesh`). Nothing for a Thing
    /// without a PRenderMesh.
    fn set_render_mesh_visible(&mut self, _thing: u32, _visible: bool) -> Result<(), String> {
        Err(not_available("render mesh visibility"))
    }
    /// Playback rate of a Thing's animation (`Native.SetAnimSpeed`; scripts pass 0 to stop
    /// and 1 to run, INFERRED multiplier of the normal rate).
    fn set_anim_speed(&mut self, _thing: u32, _speed: f32) -> Result<(), String> {
        Err(not_available("animation speeds"))
    }
    /// `Game.EPlayerNumber` of the `index`-th player record; -1 outside
    /// `0..num_players()` (scripts loop over that range).
    fn player_number(&mut self, _index: i32) -> Result<i32, String> {
        Err(not_available("players"))
    }
    /// The record of the player with this number; `None` when no record has it.
    fn player(&mut self, _number: i32) -> Result<Option<&mut PlayerRecord>, String> {
        Err(not_available("players"))
    }
    /// The Thing of the PYellowHead whose player number this is (the player's Sackboy);
    /// `None` when the level has no such Thing.
    fn yellowhead(&mut self, _number: i32) -> Result<Option<u32>, String> {
        Err(not_available("players"))
    }
    /// Does this player number belong to a player on this machine?
    fn player_is_local(&mut self, _number: i32) -> Result<bool, String> {
        Err(not_available("players"))
    }
    /// The Thing UID that holds global-settings slot `kind` (`PWorld`
    /// `GlobalSettingsThingUIDs`); 0 = nobody.
    fn global_setting_uid(&mut self, _kind: i32) -> Result<i32, String> {
        Err(not_available("global settings"))
    }
    /// A Thing asks for global-settings slot `kind` with a priority; the world applies the
    /// game's rule (see `Game.SetGlobalSettingUID` in the native table).
    fn set_global_setting_uid(
        &mut self,
        _kind: i32,
        _uid: i32,
        _priority: i32,
    ) -> Result<(), String> {
        Err(not_available("global settings"))
    }
    /// The script `SlotID` object of the level being played (`primary` = the slot the
    /// player entered through, for a level reached by a level link).
    fn level_slot_id(&mut self, _primary: bool) -> Result<Value, String> {
        Err(not_available("level slots"))
    }
    /// The first descendant of the Thing, depth first, whose PRenderMesh shows the mesh
    /// with this GUID.
    fn find_mesh_descendant(&mut self, _thing: u32, _mesh: u32) -> Result<Option<u32>, String> {
        Err(not_available("render meshes"))
    }
    /// The pad of the player whose Sackboy the Thing is (its PYellowHead's player number
    /// names the record); `None` for a Thing without a PYellowHead, a number without a
    /// record or a record without a pad.
    fn pad(&mut self, _thing: u32) -> Result<Option<PadState>, String> {
        Err(not_available("pads"))
    }
    /// Does a loaded translation table have this LAMS key (`LAMS.IsInLams__i`)?
    fn in_lams(&mut self, _key: i32) -> Result<bool, String> {
        Err(not_available("translated text"))
    }
    /// `Slot.GetSlotGroup__Q6SlotID`: a script `SlotID` object for the group of a slot.
    fn slot_group(&mut self, _slot: &Value) -> Result<Value, String> {
        Err(not_available("level slots"))
    }
    /// Type and SlotNumber of a script `SlotID` object.
    fn slot_id(&mut self, _slot: &Value) -> Result<(i32, i32), String> {
        Err(not_available("level slots"))
    }
    /// `Game.SetGlobalSettings<X>To__fb` for the one-float settings (`kind` is a
    /// `GS_*` type other than [`GS_LIGHTING`]): see the native table for the rule.
    fn set_global_setting_to(
        &mut self,
        _kind: i32,
        _value: f32,
        _blend: bool,
    ) -> Result<(), String> {
        Err(not_available("global settings"))
    }
    /// `Game.SetGlobalSettingsTo__fffffb`, the values in the script's argument order
    /// (Lighting, Darkness, Fog, FogTint, ColourCorrection).
    fn set_lighting_to(&mut self, _values: [f32; 5], _blend: bool) -> Result<(), String> {
        Err(not_available("global settings"))
    }
    /// `Game.SetGlobalSettingsBlendFactor`: the blend factor of global-settings type `kind`.
    fn set_global_setting_blend(&mut self, _kind: i32, _t: f32) -> Result<(), String> {
        Err(not_available("global settings"))
    }
    /// The value in force of a one-float setting (`Game.GetGlobalSettingsWater<X>__`).
    fn global_setting_value(&mut self, _kind: i32) -> Result<f32, String> {
        Err(not_available("global settings"))
    }
    fn set_water_drain_sounds(&mut self, _enabled: bool) -> Result<(), String> {
        Err(not_available("global settings"))
    }
    /// A call whose effect is display, HUD, network or game flow that nothing here models
    /// yet: the world records it (by `name`, with the Thing it is about and the arguments
    /// as passed) and nothing else happens. The natives bound this way are
    /// [`RECORDED_ONLY`]. The messaging natives ([`MESSAGING`]) come the same way, and a
    /// world with a message list opens and closes its messages by them.
    fn engine_call(
        &mut self,
        _name: &'static str,
        _thing: Option<u32>,
        _args: &[Value],
    ) -> Result<(), String> {
        Err(not_available("engine calls"))
    }
    /// Stores the world matrix (`Thing.SetWorldPosition__m`), works out the local matrix
    /// from the parent's and moves the Thing's descendants along. Nothing for a Thing
    /// without a PPos. Same matrix order as [`World::world_transform`].
    fn set_world_transform(&mut self, _thing: u32, _m: [f32; 16]) -> Result<(), String> {
        Err(not_available("transforms"))
    }
    /// Playback position of a Thing's animation (`Native.SetAnimPos`; scripts pass 0 or the
    /// value they just gave `SetAnimSpeed`, INFERRED a time or fraction).
    fn set_anim_pos(&mut self, _thing: u32, _pos: f32) -> Result<(), String> {
        Err(not_available("animation positions"))
    }
    /// The engine's PodComputer script object (`PodComputer.GetPodComputer__`).
    fn pod_computer(&mut self) -> Result<Value, String> {
        Err(not_available("the pod computer"))
    }
    /// The Planets object of the pod computer (`PodComputer.GetPlanets__`).
    fn planets(&mut self) -> Result<Value, String> {
        Err(not_available("planets"))
    }
    /// `Planets.GetCrafteroid__i`: a Planet object by index (scripts pass 1 and 2).
    fn crafteroid(&mut self, _index: i32) -> Result<Value, String> {
        Err(not_available("planets"))
    }
    /// `Planet.GetThing__`: the Thing of a Planet object (null when it has none).
    fn planet_thing(&mut self, _planet: &Value) -> Result<Option<u32>, String> {
        Err(not_available("planets"))
    }
    /// An interactive-music command (`Audio.SetInteractiveMusic*`, `StopInteractiveMusic`)
    /// with its arguments; nothing plays yet.
    fn audio_command(&mut self, _name: &str, _args: &[Value]) -> Result<(), String> {
        Err(not_available("audio"))
    }
    /// Engine state queries ([`STATE_QUERIES`]) by `Class.mangled` name; their arguments
    /// do not change the answer the world gives.
    fn state_query(&mut self, native: &str) -> Result<Value, String> {
        Err(not_available(native))
    }
    /// `Profile.IsLBP2GameProgressionFlagSet__i`: is every bit of `mask` set in the
    /// profile's word of LBP2 game progression flags?
    fn progression_flag(&mut self, _mask: u32) -> Result<bool, String> {
        Err(not_available("the profile"))
    }
    /// `Profile.SetLBP2GameProgressionFlag__i`: sets the bits of `mask` in that word.
    fn set_progression_flags(&mut self, _mask: u32) -> Result<(), String> {
        Err(not_available("the profile"))
    }
    /// `Native.GetIsInPadZone__Q5Thing` for a Thing that is not null: the level is our
    /// Pod, the Thing is the primary player's Sackboy, a pod-computer trigger holds him
    /// and his world z is at least [`PAD_ZONE_MIN_Z`].
    fn in_pad_zone(&mut self, _thing: u32) -> Result<bool, String> {
        Err(not_available("the pad zone"))
    }
    /// `Input.IsJustClickedByID__iiQ6String`: [`PadState::is_just_clicked`] on the pad
    /// of the player with that number; false without the player or his pad. `prompt` is
    /// the key of the on-screen hint and never changes the answer.
    fn action_just_clicked(
        &mut self,
        _player: i32,
        _action: i32,
        _prompt: Option<&str>,
    ) -> Result<bool, String> {
        Err(not_available("pads"))
    }
    /// `Native.GetPlayerColour__Q5Thing` for a Thing that is not null: the stored
    /// colour (0xAARRGGBB) of the Thing's player, `None` for a Thing without a
    /// PYellowHead.
    fn player_colour(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("players"))
    }
    /// `PodComputer.RequestHostUsePod__b`: asks the host to let the player use the pod
    /// computer (true) or to stop (false). "In use" changes when the host answers.
    fn request_host_use_pod(&mut self, _in_use: bool) -> Result<(), String> {
        Err(not_available("the pod computer"))
    }
    /// Is a player using the pod computer (`PodComputer.GetPodComputerInUse__`)?
    fn pod_computer_in_use(&mut self) -> Result<bool, String> {
        Err(not_available("the pod computer"))
    }
    /// `PodComputer.GetCrashZoom__` (bool; INFERRED a zoom-to-crash state of the planet view).
    fn crash_zoom(&mut self) -> Result<bool, String> {
        Err(not_available("the pod computer"))
    }
    /// A new Gooey (UI) script object (`Gooey.CreateGooey__i`, called by `Gooey.Create__i`
    /// with its s32 argument; INFERRED an id or flags).
    fn create_gooey(&mut self, _arg: i32) -> Result<Value, String> {
        Err(not_available("Gooey objects"))
    }
    fn set_camera(&mut self, _camera: CameraFromToUp) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    /// `Cam.SetDOFAndFog__fff` as passed (PlanetsWorld: DofMin, DofMax, FogBias, each x 2).
    fn set_dof_and_fog(&mut self, _values: [f32; 3]) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    fn clear_motion_blur(&mut self) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    /// `Cam.SetOverrideCamera__rr`: the position and the direction of the camera a script
    /// lays over the follow camera, as passed.
    fn set_override_camera(
        &mut self,
        _position: [f32; 4],
        _direction: [f32; 4],
    ) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    /// `Cam.SetOverrideCameraActivation__bf`: the override on or off, and how far its
    /// blend moves per camera update.
    fn set_override_camera_activation(&mut self, _on: bool, _rate: f32) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    /// `Cam.GetCameraMatrix__`: the render camera's matrix, camera to world (column W the
    /// eye, column Z pointing backward), in the order of [`World::world_transform`].
    fn camera_matrix(&mut self) -> Result<[f32; 16], String> {
        Err(not_available("the camera"))
    }
    /// `HUD.GetScreenSpacePosition__r`: a world point on the screen, (x, y) in pixels from
    /// the top left corner with y down, then the depth and 0.
    fn screen_position(&mut self, _point: [f32; 4]) -> Result<[f32; 4], String> {
        Err(not_available("the screen"))
    }
    /// `HUD.GetXRes__` and `HUD.GetYRes__`: the screen's width and height in pixels.
    fn screen_size(&mut self) -> Result<[i32; 2], String> {
        Err(not_available("the screen"))
    }
    /// `Thing.FindThing__Q7StringA`: the first Thing, in the order of the level's Things,
    /// whose PScriptName is exactly these bytes; `None` when no Thing has that name.
    fn find_thing(&mut self, _name: &[u8]) -> Result<Option<u32>, String> {
        Err(not_available("script names"))
    }
    /// `Thing.GetPodThing__b`: the first Thing, in the order of the level's Things, whose
    /// render mesh is [`POD_MESH`]. A Thing whose mesh is [`POD_PLATFORM_MESH`] comes
    /// first only for `platform_too`; without it the search ends there with `None`.
    fn pod_thing(&mut self, _platform_too: bool) -> Result<Option<u32>, String> {
        Err(not_available("the pod"))
    }
    /// `Thing.SetRenderMeshMesh__Q5ThingQ8Resource`: the mesh a Thing's PRenderMesh shows
    /// becomes this resource handle when the handle is a mesh, else none. Nothing for a
    /// Thing without a PRenderMesh.
    fn set_render_mesh_mesh(&mut self, _thing: u32, _mesh: Option<u32>) -> Result<(), String> {
        Err(not_available("render meshes"))
    }
    /// `PodComputer.GetPlanetsBackgroundGUID__`: the GUID of the backdrop plan of the
    /// planets level, [`DEFAULT_PLANETS_BACKGROUND`] while that level is not loaded.
    fn planets_background(&mut self) -> Result<u32, String> {
        Err(not_available("the pod computer"))
    }
    /// `HUD.GetCinemaModeCount__` (s32; scripts only test it against zero).
    fn cinema_mode_count(&mut self) -> Result<i32, String> {
        Err(not_available("the HUD"))
    }
    /// Outcome of a `Test.Check*` / `Test.Fail` native in the game's own test scripts.
    fn test_check(&mut self, _passed: bool, _message: &str, _line: i32) {}
    /// Uniform in [0, 1); only `NonDeterministic*` natives use it.
    fn random_unit(&mut self) -> f32 {
        0.
    }
    fn get_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        Err(not_available(&format!("member {class}.{field}")))
    }
    fn set_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        class: &str,
        field: &str,
        _value: Value,
    ) -> Result<(), String> {
        Err(not_available(&format!("member {class}.{field}")))
    }
    fn thing_is(&mut self, _thing: u32, _class: &str) -> bool {
        true
    }
    /// The GUID of the script resource a Thing carries ([`Host::thing_script`]).
    fn thing_script(&mut self, _thing: u32) -> Option<u32> {
        None
    }
    fn write(&mut self, _text: &str) {}
}

/// A native call split by the mangled signature: `receiver` is present when the VM staged
/// one more value than the signature has parameters (member call, INFERRED).
pub struct Call<'a> {
    pub receiver: Option<&'a Value>,
    pub params: &'a [Value],
}

pub type NativeFn = fn(&mut dyn World, &Call<'_>) -> Result<Value, String>;

/// Adapts a [`World`] to the VM's [`Host`].
pub struct Bindings<W> {
    pub world: W,
    /// Calls to natives without a binding, by `Class.mangled`.
    pub unbound: BTreeMap<String, u64>,
    /// Every native call that reached the host, bound or not, by `Class.mangled`.
    pub calls: BTreeMap<String, u64>,
    /// Run past natives without a binding: the call returns nothing, which the VM turns
    /// into the default of the call's return machine type (false, 0, null, zero vector).
    /// Off by default, so tests see a missing binding as a fault.
    pub default_unbound: bool,
}

impl<W: World> Bindings<W> {
    pub fn new(world: W) -> Self {
        Self {
            world,
            unbound: BTreeMap::new(),
            calls: BTreeMap::new(),
            default_unbound: false,
        }
    }

    /// [`Bindings::new`] with [`Bindings::default_unbound`] on (the viewer, `--play`).
    pub fn defaulting(world: W) -> Self {
        Self {
            default_unbound: true,
            ..Self::new(world)
        }
    }
}

/// The binding for a native, if one exists.
pub fn lookup(class: &str, mangled: &str) -> Option<NativeFn> {
    TABLE
        .iter()
        .find(|(c, m, _)| *c == class && *m == mangled)
        .map(|(_, _, f)| *f)
}

/// Every bound native as (class, mangled name), in table order.
pub fn bound() -> impl Iterator<Item = (&'static str, &'static str)> {
    TABLE.iter().map(|(c, m, _)| (*c, *m))
}

/// Calls a native through the table, checking arity against the mangled signature.
pub fn dispatch(
    world: &mut dyn World,
    class: &str,
    mangled: &str,
    args: &[Value],
) -> Option<Result<Value, String>> {
    let f = lookup(class, mangled)?;
    let n = signature::parse(mangled).map_or(0, |s| s.params.len());
    let call = if args.len() == n {
        Call {
            receiver: None,
            params: args,
        }
    } else if args.len() == n + 1 {
        Call {
            receiver: args.first(),
            params: &args[1..],
        }
    } else {
        return Some(Err(format!(
            "{class}.{mangled}: {} arguments for {n} parameters",
            args.len()
        )));
    };
    Some(f(world, &call))
}

impl<W: World> Host for Bindings<W> {
    fn call_native(
        &mut self,
        class: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        let name = format!("{class}.{function}");
        *self.calls.entry(name.clone()).or_default() += 1;
        match dispatch(&mut self.world, class, function, args) {
            Some(r) => r.map_err(|e| format!("{name}: {e}")),
            None => {
                *self.unbound.entry(name.clone()).or_default() += 1;
                if self.default_unbound {
                    Ok(Value::Void)
                } else {
                    Err(format!("unbound native {name}"))
                }
            }
        }
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        self.world.get_member(kind, base, class, field)
    }
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        self.world.set_member(kind, base, class, field, value)
    }
    fn thing_is(&mut self, thing: u32, class: &str) -> bool {
        self.world.thing_is(thing, class)
    }
    fn thing_script(&mut self, thing: u32) -> Option<u32> {
        self.world.thing_script(thing)
    }
    fn write(&mut self, text: &str) {
        self.world.write(text)
    }
}

// ---- argument helpers ------------------------------------------------------------

fn param<'a>(c: &'a Call<'_>, i: usize) -> Result<&'a Value, String> {
    c.params
        .get(i)
        .ok_or_else(|| format!("missing parameter {i}"))
}

fn float(v: &Value) -> Result<f32, String> {
    match v {
        Value::Float(f) => Ok(*f),
        other => Err(format!("expected f32, got {other:?}")),
    }
}

fn int(v: &Value) -> Result<i32, String> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(format!("expected s32, got {other:?}")),
    }
}

fn vec4(v: &Value) -> Result<[f32; 4], String> {
    match v {
        Value::Vec4(x) => Ok(*x),
        other => Err(format!("expected vector, got {other:?}")),
    }
}

fn text(v: &Value) -> Result<&str, String> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err(format!("expected string, got {other:?}")),
    }
}

/// A string parameter that may be null (a never-assigned `String` is a null object).
fn opt_text(v: &Value) -> Result<Option<&str>, String> {
    match v {
        Value::Object(None) => Ok(None),
        v => text(v).map(Some),
    }
}

/// First index of `needle` in `hay`, or -1 (`String.Find`, `StringA.Find`).
fn find_units<T: PartialEq>(hay: &[T], needle: &[T]) -> i32 {
    if needle.is_empty() {
        return 0;
    }
    hay.windows(needle.len())
        .position(|w| w == needle)
        .map_or(-1, |i| i as i32)
}

/// `SubStr(s, start, count)` on string units: from `start`, at most `count` units, empty
/// when `start` is at or past the end. Negative arguments are an error (not observed).
fn sub_units<T>(units: &[T], start: i32, count: i32) -> Result<&[T], String> {
    let (Ok(start), Ok(count)) = (usize::try_from(start), usize::try_from(count)) else {
        return Err(format!("SubStr({start}, {count})"));
    };
    let rest = units.get(start..).unwrap_or(&[]);
    Ok(&rest[..count.min(rest.len())])
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// GUID parameter (`g`): carried as an integer or pointer-sized value (INFERRED).
fn guid(v: &Value) -> Result<u32, String> {
    match v {
        Value::Int(i) => Ok(*i as u32),
        Value::Raw(u) | Value::Ref(u) => Ok(*u),
        other => Err(format!("expected GUID, got {other:?}")),
    }
}

/// A `Resource` parameter: the handle a `Resource.Load*ByFilename` native gave, or null
/// (a never-assigned resource is a zero handle or a null object).
fn resource(v: &Value) -> Result<Option<u32>, String> {
    match v {
        Value::Ref(u) | Value::Raw(u) => Ok(Some(*u).filter(|u| *u != 0)),
        Value::Object(None) => Ok(None),
        other => Err(format!("expected Resource, got {other:?}")),
    }
}

fn thing(v: &Value) -> Result<Option<u32>, String> {
    match v {
        Value::Thing(t) => Ok(*t),
        other => Err(format!("expected Thing, got {other:?}")),
    }
}

fn some_thing(v: &Value) -> Result<u32, String> {
    thing(v)?.ok_or_else(|| "null Thing".to_owned())
}

/// The Thing a member native acts on.
fn this(c: &Call<'_>) -> Result<u32, String> {
    some_thing(
        c.receiver
            .ok_or("member native called without a receiver")?,
    )
}

/// The Thing a member native acts on, when the native itself checks for null.
fn receiver(c: &Call<'_>) -> Result<Option<u32>, String> {
    thing(
        c.receiver
            .ok_or("member native called without a receiver")?,
    )
}

/// The matrix the position natives give for a null Thing. FOUND: a constant matrix is
/// copied out; INFERRED that it is the identity (the constants sit behind a TOC pointer
/// that was not resolved).
const IDENTITY: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

/// The record of the player a score native names (parameter 0, a `Game.EPlayerNumber`).
fn record<'a>(w: &'a mut dyn World, c: &Call<'_>) -> Result<Option<&'a mut PlayerRecord>, String> {
    w.player(int(param(c, 0)?)?)
}

/// A [`World::state_query`] that answers with a bool.
fn bool_query(w: &mut dyn World, native: &str) -> Result<Value, String> {
    match w.state_query(native)? {
        Value::Bool(b) => Ok(Value::Bool(b)),
        other => Err(format!("{native}: expected bool, got {other:?}")),
    }
}

/// A [`RECORDED_ONLY`] or [`MESSAGING`] native: hands the call to [`World::engine_call`].
fn recorded(
    w: &mut dyn World,
    name: &'static str,
    thing: Option<u32>,
    args: &[Value],
) -> Result<Value, String> {
    w.engine_call(name, thing, args)?;
    Ok(Value::Void)
}

/// `Pad.Is*(Thing, s32 mode, s32 buttons)`: `test` on the pad of the Thing's player. FOUND:
/// false for a null Thing and whenever the Thing leads to no pad.
fn pad_test(
    w: &mut dyn World,
    c: &Call<'_>,
    test: fn(&PadState, u32, u32) -> bool,
) -> Result<Value, String> {
    let (mode, buttons) = (int(param(c, 1)?)? as u32, int(param(c, 2)?)? as u32);
    let pad = match thing(param(c, 0)?)? {
        Some(t) => w.pad(t)?,
        None => None,
    };
    Ok(Value::Bool(pad.is_some_and(|p| test(&p, mode, buttons))))
}

/// `Game.SetGlobalSettings<X>To__fb(value, blend)` for global-settings type `kind`.
fn setting_to(w: &mut dyn World, c: &Call<'_>, kind: i32) -> Result<Value, String> {
    w.set_global_setting_to(kind, float(param(c, 0)?)?, boolean(param(c, 1)?)?)?;
    Ok(Value::Void)
}

/// A [`World::state_query`] that answers with an s32.
fn int_query(w: &mut dyn World, native: &str) -> Result<Value, String> {
    match w.state_query(native)? {
        Value::Int(i) => Ok(Value::Int(i)),
        other => Err(format!("{native}: expected s32, got {other:?}")),
    }
}

/// For the `Thing.*Switch*__i` natives: the signal wired to the receiver's input port.
fn wired(w: &mut dyn World, c: &Call<'_>) -> Result<Option<SwitchInput>, String> {
    w.switch_input(this(c)?, int(param(c, 0)?)?)
}

/// For `Get*__Q5Thing`: the Thing parameter (static form or member form alike).
fn subject(c: &Call<'_>) -> Result<u32, String> {
    some_thing(param(c, 0)?)
}

// ---- math (INFERRED: vector-library conventions, radians, column vectors) -----------

/// Column-major with column vectors: column `k` occupies indices `4k..4k+4`, so the
/// translation sits at 12..15 as in level-file world matrices.
fn columns(c0: [f32; 4], c1: [f32; 4], c2: [f32; 4], c3: [f32; 4]) -> Value {
    let mut m = [0.; 16];
    for (k, col) in [c0, c1, c2, c3].iter().enumerate() {
        m[k * 4..k * 4 + 4].copy_from_slice(col);
    }
    Value::Mat44(m)
}

const W: [f32; 4] = [0., 0., 0., 1.];

fn rotate(axis: usize, a: f32) -> Value {
    let (s, c) = a.sin_cos();
    match axis {
        0 => columns([1., 0., 0., 0.], [0., c, s, 0.], [0., -s, c, 0.], W),
        1 => columns([c, 0., -s, 0.], [0., 1., 0., 0.], [s, 0., c, 0.], W),
        _ => columns([c, s, 0., 0.], [-s, c, 0., 0.], [0., 0., 1., 0.], W),
    }
}

fn len3(v: [f32; 4]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Scales the first `k` components of `v` down to length `max` (`max <= 0` gives zero).
fn clamp_len(v: [f32; 4], k: usize, max: f32) -> [f32; 4] {
    let l = v[..k].iter().map(|x| x * x).sum::<f32>().sqrt();
    let s = if l > max { max.max(0.) / l } else { 1. };
    std::array::from_fn(|i| if i < k { v[i] * s } else { 0. })
}

/// `Smooth*(pos, vel, target, max_acc, max_vel)` on the first `k` components: the new
/// velocity. INFERRED: the parameter order comes from `PlanetThing`, which passes its
/// `MaxAcc` then `MaxVel` fields; the step is "head for the target at most `max_vel` per
/// tick, changing velocity by at most `max_acc` per tick". Callers add it to `pos`.
fn smooth(c: &Call<'_>, k: usize) -> Result<[f32; 4], String> {
    let get = |i| -> Result<[f32; 4], String> {
        match param(c, i)? {
            Value::Float(f) => Ok([*f, 0., 0., 0.]),
            v => vec4(v),
        }
    };
    let (pos, vel, target) = (get(0)?, get(1)?, get(2)?);
    let (max_acc, max_vel) = (float(param(c, 3)?)?, float(param(c, 4)?)?);
    let want = clamp_len(std::array::from_fn(|i| target[i] - pos[i]), k, max_vel);
    let dv = clamp_len(std::array::from_fn(|i| want[i] - vel[i]), k, max_acc);
    Ok(std::array::from_fn(
        |i| {
            if i < k {
                vel[i] + dv[i]
            } else {
                0.
            }
        },
    ))
}

/// `Math.ModAngleToWithinPI__f`: an angle in radians less whole turns, so that it lies
/// from -pi (included) up to pi.
pub fn mod_angle_to_within_pi(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let turns = (angle + PI) / TAU;
    (turns - turns.floor()) * TAU - PI
}

fn matrix(v: &Value) -> Result<[f32; 16], String> {
    match v {
        Value::Mat44(m) => Ok(*m),
        other => Err(format!("expected m44, got {other:?}")),
    }
}

/// `Test.*(..., message, line)`: reports one check; the message and line are the last two
/// parameters (INFERRED from the `Q7StringAi` suffix every Test native shares).
fn report(w: &mut dyn World, c: &Call<'_>, passed: bool) -> Result<Value, String> {
    let n = c.params.len();
    let message = text(param(c, n.wrapping_sub(2))?)?;
    let line = int(param(c, n.wrapping_sub(1))?)?;
    if passed {
        w.test_check(true, message, line);
    } else {
        // Failed checks carry the compared values for diagnosis.
        let values = &c.params[..n.saturating_sub(2)];
        w.test_check(false, &format!("{message} {values:?}"), line);
    }
    Ok(Value::Void)
}

/// `CheckEqual` on vectors compares the components the type has (V2: 2, V3: 3, V4: 4).
fn equal_components(w: &mut dyn World, c: &Call<'_>, k: usize) -> Result<Value, String> {
    let (a, b) = (vec4(param(c, 0)?)?, vec4(param(c, 1)?)?);
    report(w, c, a[..k] == b[..k])
}

fn equal_values(w: &mut dyn World, c: &Call<'_>) -> Result<Value, String> {
    let (a, b) = (param(c, 0)?, param(c, 1)?);
    if std::mem::discriminant(a) != std::mem::discriminant(b) {
        return Err(format!("CheckEqual on {a:?} and {b:?}"));
    }
    report(w, c, a == b)
}

fn boolean(v: &Value) -> Result<bool, String> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => Err(format!("expected bool, got {other:?}")),
    }
}

const TABLE: &[(&str, &str, NativeFn)] = &[
    // System.Yield__ is handled inside the VM (it suspends the call).
    ("Math", "floor__f", |_, c| {
        Ok(Value::Float(float(param(c, 0)?)?.floor()))
    }),
    ("Math", "clamp__fff", |_, c| {
        let (x, lo, hi) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        Ok(Value::Float(x.max(lo).min(hi)))
    }),
    ("Math", "atan2__ff", |_, c| {
        Ok(Value::Float(
            float(param(c, 0)?)?.atan2(float(param(c, 1)?)?),
        ))
    }),
    ("Math", "rotateX__f", |_, c| {
        Ok(rotate(0, float(param(c, 0)?)?))
    }),
    ("Math", "rotateY__f", |_, c| {
        Ok(rotate(1, float(param(c, 0)?)?))
    }),
    ("Math", "rotateZ__f", |_, c| {
        Ok(rotate(2, float(param(c, 0)?)?))
    }),
    ("Math", "translate__r", |_, c| {
        let t = vec4(param(c, 0)?)?;
        Ok(columns(
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [t[0], t[1], t[2], 1.],
        ))
    }),
    ("Math", "scale__r", |_, c| {
        let s = vec4(param(c, 0)?)?;
        Ok(columns(
            [s[0], 0., 0., 0.],
            [0., s[1], 0., 0.],
            [0., 0., s[2], 0.],
            W,
        ))
    }),
    ("Math", "mulPerElem__rr", |_, c| {
        let (a, b) = (vec4(param(c, 0)?)?, vec4(param(c, 1)?)?);
        Ok(Value::Vec4(std::array::from_fn(|i| a[i] * b[i])))
    }),
    ("Math", "length3__r", |_, c| {
        Ok(Value::Float(len3(vec4(param(c, 0)?)?)))
    }),
    ("Math", "normalize3__r", |_, c| {
        let v = vec4(param(c, 0)?)?;
        let l = len3(v);
        if l == 0. {
            return Err("normalize3 of a zero vector".into());
        }
        Ok(Value::Vec4([v[0] / l, v[1] / l, v[2] / l, v[3]]))
    }),
    ("Math", "length__p", |_, c| {
        let v = vec4(param(c, 0)?)?;
        Ok(Value::Float((v[0] * v[0] + v[1] * v[1]).sqrt()))
    }),
    ("Math", "NonDeterministicRandomBetween__ff", |w, c| {
        let (a, b) = (float(param(c, 0)?)?, float(param(c, 1)?)?);
        Ok(Value::Float(a + (b - a) * w.random_unit()))
    }),
    // ---- strings ----
    ("String", "Concat__Q6StringQ6String", |_, c| {
        let s = format!("{}{}", text(param(c, 0)?)?, text(param(c, 1)?)?);
        Ok(Value::Str(s.into()))
    }),
    ("String", "StringFromInt__i", |_, c| {
        Ok(Value::Str(int(param(c, 0)?)?.to_string().into()))
    }),
    ("String", "InfinitySymbol__", |_, _| {
        Ok(Value::Str("\u{221e}".into()))
    }),
    ("LAMS", "Translate__i", |w, c| {
        Ok(Value::Str(w.translate(int(param(c, 0)?)?)?.into()))
    }),
    // Stage 1 (docs/level-scripts.md part 9). `String` is a wide string: lengths and
    // indices count UTF-16 units; `StringA` counts bytes. A null string has length 0, is
    // "null or empty", is never found in and gives an empty substring.
    ("String", "Length__Q6String", |_, c| {
        let n = opt_text(param(c, 0)?)?.map_or(0, |s| s.encode_utf16().count());
        Ok(Value::Int(n as i32))
    }),
    ("String", "IsNullOrEmpty__Q6String", |_, c| {
        Ok(Value::Bool(
            opt_text(param(c, 0)?)?.is_none_or(str::is_empty),
        ))
    }),
    ("String", "Find__Q6StringQ6String", |_, c| {
        let (Some(s), Some(sub)) = (opt_text(param(c, 0)?)?, opt_text(param(c, 1)?)?) else {
            return Ok(Value::Int(-1));
        };
        Ok(Value::Int(find_units(&wide(s), &wide(sub))))
    }),
    ("String", "SubStr__Q6Stringii", |_, c| {
        let s = wide(opt_text(param(c, 0)?)?.unwrap_or(""));
        let part = sub_units(&s, int(param(c, 1)?)?, int(param(c, 2)?)?)?;
        Ok(Value::Str(String::from_utf16_lossy(part).into()))
    }),
    ("String", "ToUpper__Q6String", |_, c| {
        // Unit by unit: a letter whose upper case is not one character stays as it is.
        let upper = |ch: char| {
            let mut u = ch.to_uppercase();
            match (u.next(), u.next()) {
                (Some(one), None) if one.len_utf16() == ch.len_utf16() => one,
                _ => ch,
            }
        };
        Ok(Value::Str(
            text(param(c, 0)?)?
                .chars()
                .map(upper)
                .collect::<String>()
                .into(),
        ))
    }),
    ("String", "StringFromStringA__Q7StringA", |_, c| {
        Ok(Value::Str(text(param(c, 0)?)?.into()))
    }),
    // The StringA forms are INFERRED to follow the String forms on bytes.
    ("StringA", "Length__Q7StringA", |_, c| {
        Ok(Value::Int(
            opt_text(param(c, 0)?)?.map_or(0, str::len) as i32
        ))
    }),
    ("StringA", "IsNullOrEmpty__Q7StringA", |_, c| {
        Ok(Value::Bool(
            opt_text(param(c, 0)?)?.is_none_or(str::is_empty),
        ))
    }),
    ("StringA", "Find__Q7StringAQ7StringA", |_, c| {
        let (Some(s), Some(sub)) = (opt_text(param(c, 0)?)?, opt_text(param(c, 1)?)?) else {
            return Ok(Value::Int(-1));
        };
        Ok(Value::Int(find_units(s.as_bytes(), sub.as_bytes())))
    }),
    ("StringA", "SubStr__Q7StringAii", |_, c| {
        let s = opt_text(param(c, 0)?)?.unwrap_or("").as_bytes();
        let part = sub_units(s, int(param(c, 1)?)?, int(param(c, 2)?)?)?;
        Ok(Value::Str(String::from_utf8_lossy(part).into()))
    }),
    ("Math", "identity__", |_, _| {
        Ok(columns(
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            W,
        ))
    }),
    ("Math", "ceil__f", |_, c| {
        Ok(Value::Float(float(param(c, 0)?)?.ceil()))
    }),
    ("Math", "exp__f", |_, c| {
        Ok(Value::Float(float(param(c, 0)?)?.exp()))
    }),
    ("Math", "pow__ff", |_, c| {
        Ok(Value::Float(
            float(param(c, 0)?)?.powf(float(param(c, 1)?)?),
        ))
    }),
    ("Math", "sign__f", |_, c| {
        // 1 from zero up, -1 below; never 0.
        let x = float(param(c, 0)?)?;
        Ok(Value::Float(if x < 0. { -1. } else { 1. }))
    }),
    ("Math", "lerp__fff", |_, c| {
        let (t, a, b) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        Ok(Value::Float((b - a) * t + a))
    }),
    ("Math", "Delt__fff", |_, c| {
        // Moves `from` towards `to` by at most `step`, landing on `to`.
        let (from, to, step) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        let d = to - from;
        let dir = if d < 0. { -1. } else { 1. };
        Ok(Value::Float(if dir * d < step {
            to
        } else {
            dir * step + from
        }))
    }),
    ("Math", "BiasCosine__f", |_, c| {
        // 0 at 0, 1 at 1, cosine eased.
        let x = float(param(c, 0)?)?;
        Ok(Value::Float((x * std::f32::consts::PI).cos() * -0.5 + 0.5))
    }),
    // ---- thing tree and transforms ----
    ("Thing", "GetThingUID__", |w, c| {
        Ok(Value::Int(w.thing_uid(this(c)?)?))
    }),
    ("Thing", "GetParent__Q5Thing", |w, c| {
        Ok(Value::Thing(w.parent(subject(c)?)?))
    }),
    ("Thing", "GetFirstChild__Q5Thing", |w, c| {
        Ok(Value::Thing(w.first_child(subject(c)?)?))
    }),
    ("Thing", "GetNextSibling__Q5Thing", |w, c| {
        Ok(Value::Thing(w.next_sibling(subject(c)?)?))
    }),
    ("Thing", "GetWorld__", |w, _| {
        Ok(Value::Thing(w.world_thing()?))
    }),
    ("System", "GetWorld__", |w, _| {
        Ok(Value::Thing(w.world_thing()?))
    }),
    // FOUND (01.33, 0x643138 -> 0x755874). Declared `m44 GetWorldPosition()`: the whole
    // PPos world matrix, not a position (scripts take the translation column out of it
    // themselves). For a null Thing or one without a PPos the game leaves the result
    // untouched; INFERRED stand-in here: the identity.
    ("Thing", "GetWorldPosition__", |w, c| {
        Ok(Value::Mat44(match receiver(c)? {
            Some(t) => w.world_transform(t)?.unwrap_or(IDENTITY),
            None => IDENTITY,
        }))
    }),
    ("Thing", "GetCOM__", |w, c| {
        Ok(Value::Vec4(w.centre_of_mass(this(c)?)?))
    }),
    // FOUND (01.33, 0x642e48 -> 0x753bb0 -> 0xf2e80). Declared `void SetLocalPosition(m44)`.
    // Nothing for a null Thing or one without a PPos. The matrix becomes the local matrix;
    // world = parent's world x local when the parent has a PPos, else world = local; then
    // every descendant with a PPos gets world = its parent's world x its own local.
    // Not modelled: a Thing flag (byte +0x17 bit 0) that makes world = local under a
    // parent, and the refresh of bodies and shapes.
    ("Thing", "SetLocalPosition__m", |w, c| {
        let m = matrix(param(c, 0)?)?;
        if let Some(t) = receiver(c)? {
            w.set_local_transform(t, m)?;
        }
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x642ee0 -> 0x755a98). Declared `m44 GetLocalPosition()`: the PPos
    // local matrix; a constant matrix for a null Thing or one without a PPos (INFERRED:
    // the identity).
    ("Thing", "GetLocalPosition__", |w, c| {
        Ok(Value::Mat44(match receiver(c)? {
            Some(t) => w.local_transform(t)?.unwrap_or(IDENTITY),
            None => IDENTITY,
        }))
    }),
    // FOUND (01.33, 0x6422f0 -> 0x7533ac). Declared `bool IsInactive()`: the Thing's
    // "deactivated" byte (+0x7f), which the switch pass keeps for the Things on a
    // microchip that is off. The game does not test the Thing for null; a null Thing is
    // an error here.
    ("Thing", "IsInactive__", |w, c| {
        Ok(Value::Bool(w.is_inactive(this(c)?)?))
    }),
    // FOUND (01.33, 0x642bd4 -> 0x754498). Declared static
    // `Thing GetMicroChipProxyFromThing(Thing)`: null for null; the Thing's microchip
    // Thing (+0x5c) when it has one, else the Thing itself. INFERRED: what fills +0x5c
    // (the world's side: the outermost microchip whose circuit board the Thing is on).
    ("Thing", "GetMicroChipProxyFromThing__Q5Thing", |w, c| {
        Ok(Value::Thing(match thing(param(c, 0)?)? {
            Some(t) => Some(w.microchip_proxy(t)?),
            None => None,
        }))
    }),
    // FOUND (01.33, 0x643010 -> 0x7556a0). Declared `m44 GetWorldPositionForSensor()`: a
    // constant matrix for a null Thing (INFERRED: the identity); for a Thing that is not
    // on a sequencer, the world matrix of its microchip Thing when it has one, else its
    // own. Left out: the sequencer branch (0x40748, 0x3c594), where a Thing under a
    // sequencer gets a position along the sequencer; such a Thing gets the same matrix as
    // any other here. INFERRED: the identity when that Thing has no PPos.
    ("Thing", "GetWorldPositionForSensor__", |w, c| {
        Ok(Value::Mat44(match receiver(c)? {
            Some(t) => {
                let proxy = w.microchip_proxy(t)?;
                w.world_transform(proxy)?.unwrap_or(IDENTITY)
            }
            None => IDENTITY,
        }))
    }),
    // FOUND (01.33, 0x64b738 -> 0x5e7068 -> 0xf3d94). Declared static
    // `bool IsOnSequencer(Thing)`: the Thing's parent is a circuit board (object kind
    // 0x40) and that board's parent has a PSequencer. INFERRED: false for a null Thing.
    ("Game", "IsOnSequencer__Q5Thing", |w, c| {
        Ok(Value::Bool(match thing(param(c, 0)?)? {
            Some(t) => w.is_on_sequencer(t)?,
            None => false,
        }))
    }),
    // ---- switches (the Thing's own inputs, by port) ----
    // FOUND in the 01.33 executable through the Ghidra bridge; each native is a thin
    // wrapper (address first) around a reader of the Thing's input table (second).
    // Not modelled: the Binary, Analogue, Ternary and Player readers also answer "nothing
    // wired" while a flag byte of the Thing (+0x7f, meaning unknown) is set.
    //
    // 0x64228c -> 0x7541e0: true when an output is wired to the port. The signal's
    // value is not looked at.
    ("Thing", "IsSwitchTriggered__i", |w, c| {
        Ok(Value::Bool(wired(w, c)?.is_some()))
    }),
    // 0x642228 -> 0x754308: wired and the ternary is not 0.
    ("Thing", "GetSwitchActivationBinary__i", |w, c| {
        Ok(Value::Bool(wired(w, c)?.is_some_and(|s| s.ternary != 0)))
    }),
    // 0x6421c4 -> 0x7542a4: the activation; 0.0 when nothing is wired (the constant the
    // reader loads from its TOC, read as 0.0).
    ("Thing", "GetSwitchActivationAnalogue__i", |w, c| {
        Ok(Value::Float(wired(w, c)?.map_or(0., |s| s.activation)))
    }),
    // 0x642160 -> 0x75437c: the ternary as a float (declared f32 in the script tables,
    // as the analogue form); the same 0.0 when nothing is wired.
    ("Thing", "GetSwitchActivationTernary__i", |w, c| {
        Ok(Value::Float(wired(w, c)?.map_or(0., |s| s.ternary as f32)))
    }),
    // 0x6420fc -> 0x754ccc: the signal's player when it is not -1. Not modelled: with
    // -1 or nothing wired the game falls back to the player who owns the Thing (through
    // its root and parent); here that case is -1.
    ("Thing", "GetSwitchActivationPlayer__i", |w, c| {
        Ok(Value::Int(wired(w, c)?.map_or(-1, |s| s.player)))
    }),
    // 0x643f2c -> 0x754244: the Thing of the PSwitch whose output is wired to the port;
    // null for a null Thing, a port out of range or nothing wired.
    ("Thing", "GetInput__Q5Thingi", |w, c| {
        let port = int(param(c, 1)?)?;
        Ok(Value::Thing(match thing(param(c, 0)?)? {
            Some(t) => w.switch_input_thing(t, port)?,
            None => None,
        }))
    }),
    // 0x643ea0 -> 0x754af8: (thing, output, target): the Thing that target of that
    // output of the Thing's own PSwitch points at; null for a null Thing, no PSwitch or
    // a target index at or past the output's target count. The game does not check the
    // output index (nor a negative target index); those are null here.
    ("Thing", "GetOutput__Q5Thingii", |w, c| {
        let (output, target) = (int(param(c, 1)?)?, int(param(c, 2)?)?);
        Ok(Value::Thing(match thing(param(c, 0)?)? {
            Some(t) => w.switch_output_target(t, output, target)?,
            None => None,
        }))
    }),
    // ---- parts ----
    ("PartList", "HasPJoint__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PJoint")?))
    }),
    ("PartList", "HasPSwitch__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PSwitch")?))
    }),
    ("PartList", "HasPTrigger__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PTrigger")?))
    }),
    // INFERRED from the name, as the other `HasP` natives (its code was not read).
    ("PartList", "HasPYellowHead__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PYellowHead")?))
    }),
    // FOUND (01.33, 0x648770): the PTrigger's in-list count; 0 for a null Thing or a
    // Thing without the part.
    ("Native", "GetNumInThings__Q5Thing", |w, c| {
        Ok(Value::Int(match param(c, 0)? {
            Value::Thing(None) => 0,
            t => w.trigger_count(some_thing(t)?)?,
        }))
    }),
    // FOUND (01.33, 0x6486ec -> 0x6d558c): entry `i` of the PTrigger's in-list; null for
    // a null Thing or a Thing without the part. The game does not check `i` against the
    // count; out of range is null here.
    ("Native", "GetInThing__Q5Thingi", |w, c| {
        let index = int(param(c, 1)?)?;
        Ok(Value::Thing(match thing(param(c, 0)?)? {
            Some(t) => w.trigger_thing(t, index)?,
            None => None,
        }))
    }),
    // FOUND (01.33, 0x64bae0 -> 0x7569ec -> 0x622658 -> 0x164454): the Thing has a
    // PScript whose script is the scoreboard script ([`SCORE_BOARD_SCRIPT`]) or has it
    // on its superclass chain (by identity or by class name); false for a null Thing.
    ("BaseWorld", "IsScoreBoard__Q5Thing", |w, c| {
        Ok(Value::Bool(match thing(param(c, 0)?)? {
            Some(t) => w.is_score_board(t)?,
            None => false,
        }))
    }),
    ("PartList", "HasPShape__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PShape")?))
    }),
    ("PartList", "HasPSpriteLight__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PSpriteLight")?))
    }),
    ("PartList", "HasPRenderMesh__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PRenderMesh")?))
    }),
    // ---- audio, resources, game ----
    ("Audio", "PlaySound__Q5ThingQ7StringA", |w, c| {
        let t = thing(param(c, 0)?)?;
        w.play_sound(t, text(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    ("Resource", "LoadTextureByFilename__g", |w, c| {
        Ok(Value::Ref(w.load_resource(
            ResourceKind::Texture,
            guid(param(c, 0)?)?,
        )?))
    }),
    ("Resource", "LoadTextureByFilename__gb", |w, c| {
        // The `__g` loader with a flag handed to the load (its meaning is not settled; no
        // texture is loaded here either way).
        boolean(param(c, 1)?)?;
        Ok(Value::Ref(w.load_resource(
            ResourceKind::Texture,
            guid(param(c, 0)?)?,
        )?))
    }),
    ("Resource", "LoadPlanByFilename__g", |w, c| {
        Ok(Value::Ref(
            w.load_resource(ResourceKind::Plan, guid(param(c, 0)?)?)?,
        ))
    }),
    ("Game", "GetNumPlayers__", |w, _| {
        Ok(Value::Int(w.num_players()?))
    }),
    ("Game", "GetPlayOrEditMode__", |w, _| {
        Ok(Value::Int(w.play_or_edit_mode()?))
    }),
    ("Audio", "GetDialogueList__", |w, _| {
        Ok(Value::Int(w.dialogue_list()?))
    }),
    ("Thing", "GetShapeEditorColour__Q5Thing", |w, c| {
        Ok(Value::Vec4(w.shape_editor_colour(subject(c)?)?))
    }),
    // FOUND (01.33, 0x642a20 -> 0x75495c). Declared static
    // `void SetShapeEditorColour(Thing, v4)`: nothing for a null Thing or one without a
    // PShape; else the colour is packed to 32 bits and stored in the PShape.
    ("Thing", "SetShapeEditorColour__Q5Thingr", |w, c| {
        let colour = vec4(param(c, 1)?)?;
        if let Some(t) = thing(param(c, 0)?)? {
            w.set_shape_editor_colour(t, colour)?;
        }
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x6429cc -> 0x753a9c). Declared static
    // `void SetShapeEditorColourTint(Thing, v4)`: as `SetShapeEditorColour`, into the
    // PShape word before the colour's. Scripts of four classes call it (the race gates,
    // the scoreboard, the burning object); the measuring runs do not reach those calls.
    ("Thing", "SetShapeEditorColourTint__Q5Thingr", |w, c| {
        let colour = vec4(param(c, 1)?)?;
        if let Some(t) = thing(param(c, 0)?)? {
            w.set_shape_tint(t, colour)?;
        }
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x642978 -> 0x7549c8). Declared static
    // `void SetRenderMeshEditorColourTint(Thing, v4)`: nothing for a null Thing or one
    // without a PRenderMesh; else the colour is packed to 32 bits and stored in the
    // PRenderMesh, in a word of its own after the editor colour's two.
    (
        "Thing",
        "SetRenderMeshEditorColourTint__Q5Thingr",
        |w, c| {
            let colour = vec4(param(c, 1)?)?;
            if let Some(t) = thing(param(c, 0)?)? {
                w.set_render_mesh_tint(t, colour)?;
            }
            Ok(Value::Void)
        },
    ),
    ("Native", "SetAnimSpeed__Q5Thingf", |w, c| {
        w.set_anim_speed(subject(c)?, float(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    ("PartList", "HasPCameraTweak__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PCameraTweak")?))
    }),
    // FOUND (01.33, 0x600224 -> 0x5e917c). Declared static
    // `Game.EPlayerNumber GetPlayerNumber(s32)`: the number of the record at this index,
    // -1 for an index outside the records.
    ("Game", "GetPlayerNumber__i", |w, c| {
        Ok(Value::Int(w.player_number(int(param(c, 0)?)?)?))
    }),
    // ---- part 6: the Pod camera (PlanetsWorld) and the next Rookie Test natives ----
    ("Math", "lerp__frr", |_, c| {
        // Vector-library order lerp(t, a, b) = a + (b - a) t (INFERRED).
        let (t, a, b) = (
            float(param(c, 0)?)?,
            vec4(param(c, 1)?)?,
            vec4(param(c, 2)?)?,
        );
        Ok(Value::Vec4(std::array::from_fn(|i| {
            a[i] + (b[i] - a[i]) * t
        })))
    }),
    ("Math", "SmoothStep__fff", |_, c| {
        // smoothstep(edge0, edge1, x); PlanetsWorld passes (0, 1, ViewTransition).
        let (e0, e1, x) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        if e0 == e1 {
            return Ok(Value::Float(if x < e0 { 0. } else { 1. }));
        }
        let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
        Ok(Value::Float(t * t * (3. - 2. * t)))
    }),
    ("Math", "Smooth3D__rrrff", |_, c| {
        Ok(Value::Vec4(smooth(c, 3)?))
    }),
    ("Math", "Smooth2D__pppff", |_, c| {
        Ok(Value::Vec4(smooth(c, 2)?))
    }),
    ("Math", "Smooth__fffff", |_, c| {
        // The caller reads x as the new value and y as the new velocity.
        let pos = float(param(c, 0)?)?;
        let v = smooth(c, 1)?[0];
        Ok(Value::Vec4([pos + v, v, 0., 0.]))
    }),
    ("Cam", "SetCameraFromToUp__rrrffff", |w, c| {
        let f = |i| float(param(c, i)?);
        w.set_camera(CameraFromToUp {
            from: vec4(param(c, 0)?)?,
            to: vec4(param(c, 1)?)?,
            up: vec4(param(c, 2)?)?,
            params: [f(3)?, f(4)?, f(5)?, f(6)?],
        })?;
        Ok(Value::Void)
    }),
    ("Cam", "SetDOFAndFog__fff", |w, c| {
        let f = |i| float(param(c, i)?);
        w.set_dof_and_fog([f(0)?, f(1)?, f(2)?])?;
        Ok(Value::Void)
    }),
    ("Cam", "ClearMotionBlur__", |w, _| {
        w.clear_motion_blur()?;
        Ok(Value::Void)
    }),
    ("HUD", "GetCinemaModeCount__", |w, _| {
        Ok(Value::Int(w.cinema_mode_count()?))
    }),
    ("PodComputer", "GetPodComputer__", |w, _| w.pod_computer()),
    ("PodComputer", "GetPlanets__", |w, _| w.planets()),
    ("Game", "IsNonInteractive__", |w, _| {
        w.state_query("Game.IsNonInteractive__")
    }),
    ("Game", "GetNumPendingLinks__", |w, _| {
        w.state_query("Game.GetNumPendingLinks__")
    }),
    ("Game", "GetNumAvatars__", |w, _| {
        w.state_query("Game.GetNumAvatars__")
    }),
    ("Video", "IsVideoPlaying__", |w, _| {
        w.state_query("Video.IsVideoPlaying__")
    }),
    ("System", "Is4to3__", |w, _| {
        w.state_query("System.Is4to3__")
    }),
    ("Game", "GetLocalLeadersPlayerNumber__", |w, _| {
        w.state_query("Game.GetLocalLeadersPlayerNumber__")
    }),
    ("Audio", "GetCurrMusicGuid__", |w, _| {
        w.state_query("Audio.GetCurrMusicGuid__")
    }),
    ("Audio", "UserMusicIsStopped__", |w, _| {
        w.state_query("Audio.UserMusicIsStopped__")
    }),
    ("Audio", "UserMusicIsPaused__", |w, _| {
        w.state_query("Audio.UserMusicIsPaused__")
    }),
    ("Audio", "IsInteractiveMusicAudible__", |w, _| {
        w.state_query("Audio.IsInteractiveMusicAudible__")
    }),
    ("Input", "IsInAttractionLoop__i", |w, c| {
        int(param(c, 0)?)?;
        w.state_query("Input.IsInAttractionLoop__i")
    }),
    ("Audio", "SetInteractiveMusic__ggffiQ5Thing", |w, c| {
        w.audio_command("SetInteractiveMusic", c.params)?;
        Ok(Value::Void)
    }),
    (
        "Audio",
        "SetInteractiveMusicStereoPairVolume__giff",
        |w, c| {
            w.audio_command("SetInteractiveMusicStereoPairVolume", c.params)?;
            Ok(Value::Void)
        },
    ),
    ("Audio", "StopInteractiveMusic__", |w, c| {
        w.audio_command("StopInteractiveMusic", c.params)?;
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x5f6860 -> 0x73fa20 -> 0x161eac -> 0x15d2a8). Declared static
    // `bool IsLBP2GameProgressionFlagSet(s32)` on Profile. The argument is a bit mask:
    // true when every bit of it is set in one 32-bit word of the profile, the word
    // `SetLBP2GameProgressionFlag__i` writes. The core also answers true whatever the
    // word while a debug byte is set, or for the masks 1 and 2 while a build-type word
    // has one value. INFERRED: neither holds in a played retail game.
    ("Profile", "IsLBP2GameProgressionFlagSet__i", |w, c| {
        Ok(Value::Bool(w.progression_flag(int(param(c, 0)?)? as u32)?))
    }),
    ("Planets", "GetCrafteroid__i", |w, c| {
        w.crafteroid(int(param(c, 0)?)?)
    }),
    ("Planet", "GetThing__", |w, c| {
        let planet = c
            .receiver
            .ok_or("member native called without a receiver")?;
        Ok(Value::Thing(w.planet_thing(planet)?))
    }),
    ("PodComputer", "GetPodComputerInUse__", |w, _| {
        Ok(Value::Bool(w.pod_computer_in_use()?))
    }),
    ("PodComputer", "GetCrashZoom__", |w, _| {
        Ok(Value::Bool(w.crash_zoom()?))
    }),
    // FOUND (01.33, 0x6430a0 -> 0x753bfc -> 0xf26bc). Declared `void SetWorldPosition(m44)`.
    // Nothing for a null Thing or one without a PPos. The matrix becomes the world matrix;
    // local = inverse(parent's world) x world when the parent has a PPos, else local =
    // world; then the descendants follow as in `SetLocalPosition__m`.
    ("Thing", "SetWorldPosition__m", |w, c| {
        let m = matrix(param(c, 0)?)?;
        if let Some(t) = receiver(c)? {
            w.set_world_transform(t, m)?;
        }
        Ok(Value::Void)
    }),
    ("Native", "DebugPrint__i", |w, c| {
        w.write(&int(param(c, 0)?)?.to_string());
        Ok(Value::Void)
    }),
    ("Native", "SetAnimPos__Q5Thingf", |w, c| {
        w.set_anim_pos(subject(c)?, float(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    ("LAMS", "MakeLamsKeyID__Q6String", |_, c| {
        // Toolkit makeLamsKeyID (CONFIRMED_FROM_COMMUNITY_CODE, docs/u1-translation.md);
        // the key is the s32 that `Translate__i` and `IsInLams__i` take. A null string
        // hashes as the empty string.
        let key = lbp_resource::translation::lams_key(opt_text(param(c, 0)?)?.unwrap_or(""));
        Ok(Value::Int(key as i32))
    }),
    // FOUND (01.33, 0x5fe660 -> 0x5e84f0). Declared static
    // `void SetPlayerScoreMultiplierTimer(Game.EPlayerNumber, s32)`: the record's timer;
    // nothing for a player without a record.
    (
        "Game",
        "SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi",
        |w, c| {
            let timer = int(param(c, 1)?)?;
            if let Some(r) = record(w, c)? {
                r.multiplier_timer = timer;
            }
            Ok(Value::Void)
        },
    ),
    ("Gooey", "CreateGooey__i", |w, c| {
        w.create_gooey(int(param(c, 0)?)?)
    }),
    // ---- stage 4: players, score, global settings, game state ----
    // Each entry: handler -> reader in the 01.33 executable, the declared signature from
    // the script tables, and what is FOUND there or INFERRED. Player records are
    // [`PlayerRecord`]; "the record" is the one whose number is the first parameter.
    //
    // FOUND (0x6001e0 -> 0x5e68bc). Declared static `Game.EPlayerNumber
    // EPlayerNumberFromInt(s32)`: the callee is empty and the argument comes back.
    ("Game", "EPlayerNumberFromInt__i", |_, c| {
        Ok(Value::Int(int(param(c, 0)?)?))
    }),
    // FOUND (0x5fe0e8 -> 0x5e8728). Declared static `bool DoesPlayerExist(EPlayerNumber)`:
    // a record with this number exists.
    ("Game", "DoesPlayerExist__Q18Game.EPlayerNumber", |w, c| {
        Ok(Value::Bool(record(w, c)?.is_some()))
    }),
    // FOUND (0x5f85c4 -> 0x5e79f4). Declared static `bool IsPlayerConnected(s32)`: the same
    // test, a record with this number exists.
    ("Game", "IsPlayerConnected__i", |w, c| {
        Ok(Value::Bool(record(w, c)?.is_some()))
    }),
    // FOUND (0x600d40 -> 0x5e8774 -> 0x18ec14). Declared static
    // `Thing GetYellowheadFromPlayerNumber(EPlayerNumber)`: the Thing of the level's
    // PYellowHead whose PlayerNumber is the argument; null when there is none.
    (
        "Game",
        "GetYellowheadFromPlayerNumber__Q18Game.EPlayerNumber",
        |w, c| Ok(Value::Thing(w.yellowhead(int(param(c, 0)?)?)?)),
    ),
    // FOUND shape (0x5fe1f8 -> 0x5e6ac4 -> 0x243b54). Declared static
    // `bool PlayerNumberIsLocalPlayer(EPlayerNumber)`: a scan of the network session's
    // entry for this machine for a player with this number. INFERRED for the world: with
    // one machine, every player who has a record is local.
    (
        "Game",
        "PlayerNumberIsLocalPlayer__Q18Game.EPlayerNumber",
        |w, c| Ok(Value::Bool(w.player_is_local(int(param(c, 0)?)?)?)),
    ),
    // FOUND (0x5fe0a4 -> 0x5ea758). Declared static `bool IsLocalPlayerID(EPlayerNumber)`:
    // the player's yellowhead Thing exists, has a PYellowHead, and the network layer
    // calls its player number local.
    ("Game", "IsLocalPlayerID__Q18Game.EPlayerNumber", |w, c| {
        let n = int(param(c, 0)?)?;
        Ok(Value::Bool(
            w.yellowhead(n)?.is_some() && w.player_is_local(n)?,
        ))
    }),
    // FOUND shape (0x600268 -> 0x240a88). Declared static
    // `Game.EPlayerNumber GetPartyLeadersPlayerNumber()`: a scan of the session's players
    // for the leader, -1 with nobody. INFERRED for the world: the first record leads.
    ("Game", "GetPartyLeadersPlayerNumber__", |w, _| {
        w.state_query("Game.GetPartyLeadersPlayerNumber__")
    }),
    // FOUND (0x5fe520 -> 0x5e81e4). Declared static
    // `void SetPlayerTeamNumber(EPlayerNumber, s32)`: the record's team; nothing without a
    // record.
    (
        "Game",
        "SetPlayerTeamNumber__Q18Game.EPlayerNumberi",
        |w, c| {
            let team = int(param(c, 1)?)?;
            if let Some(r) = record(w, c)? {
                r.team = team;
            }
            Ok(Value::Void)
        },
    ),
    // FOUND (0x5fe55c -> 0x5e8234). Declared static `s32 GetPlayerTeamNumber(EPlayerNumber)`:
    // the record's team, -1 without a record.
    (
        "Game",
        "GetPlayerTeamNumber__Q18Game.EPlayerNumber",
        |w, c| Ok(Value::Int(record(w, c)?.map_or(-1, |r| r.team))),
    ),
    // FOUND (0x5fe6e0 -> 0x5e858c -> 0x3959e0). Declared static
    // `void SetPlayerScoreMultiplierCounter(EPlayerNumber, s32)`: the record's counter;
    // nothing without a record. Not modelled: the HUD and audio call that follows.
    (
        "Game",
        "SetPlayerScoreMultiplierCounter__Q18Game.EPlayerNumberi",
        |w, c| {
            let counter = int(param(c, 1)?)?;
            if let Some(r) = record(w, c)? {
                r.multiplier_counter = counter;
            }
            Ok(Value::Void)
        },
    ),
    // FOUND (0x5fe71c -> 0x5e8eb4). Declared static
    // `s32 GetPlayerScoreMultiplierCounter(EPlayerNumber)`: the counter, 0 without a record.
    (
        "Game",
        "GetPlayerScoreMultiplierCounter__Q18Game.EPlayerNumber",
        |w, c| {
            Ok(Value::Int(
                record(w, c)?.map_or(0, |r| r.multiplier_counter),
            ))
        },
    ),
    // FOUND (0x5fe7b4 -> 0x5e8178). Declared static
    // `s32 GetPlayerScoreMultiplier(EPlayerNumber)`: counter / 5 + 1, 0 without a record.
    (
        "Game",
        "GetPlayerScoreMultiplier__Q18Game.EPlayerNumber",
        |w, c| {
            Ok(Value::Int(
                record(w, c)?.map_or(0, |r| r.multiplier_counter / 5 + 1),
            ))
        },
    ),
    // FOUND (0x5fe760 -> 0x5e8e1c). Declared static
    // `s32 IncrementPlayerScoreMultiplierCounter(EPlayerNumber, s32 add, s32 cap)`: the
    // counter becomes min(counter + add, cap) and is returned; 0 without a record.
    (
        "Game",
        "IncrementPlayerScoreMultiplierCounter__Q18Game.EPlayerNumberii",
        |w, c| {
            let (add, cap) = (int(param(c, 1)?)?, int(param(c, 2)?)?);
            Ok(Value::Int(record(w, c)?.map_or(0, |r| {
                r.multiplier_counter = r.multiplier_counter.wrapping_add(add).min(cap);
                r.multiplier_counter
            })))
        },
    ),
    // FOUND (0x5fe69c -> 0x5e8540). Declared static
    // `s32 GetPlayerScoreMultiplierTimer(EPlayerNumber)`: the timer, 0 without a record.
    (
        "Game",
        "GetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumber",
        |w, c| Ok(Value::Int(record(w, c)?.map_or(0, |r| r.multiplier_timer))),
    ),
    // FOUND (0x5fe5a0 -> 0x5e9118). Declared static
    // `s32 DecrementPlayerScoreMultiplierTimer(EPlayerNumber)`: a timer above 0 loses 1;
    // the timer is returned; 0 without a record.
    (
        "Game",
        "DecrementPlayerScoreMultiplierTimer__Q18Game.EPlayerNumber",
        |w, c| {
            Ok(Value::Int(record(w, c)?.map_or(0, |r| {
                if r.multiplier_timer > 0 {
                    r.multiplier_timer -= 1;
                }
                r.multiplier_timer
            })))
        },
    ),
    // FOUND (0x5fe5e4 -> 0x5e82f0). Declared static
    // `void SetPlayerScoreMultiplierVisible(EPlayerNumber, bool)`: the record's flag.
    (
        "Game",
        "SetPlayerScoreMultiplierVisible__Q18Game.EPlayerNumberb",
        |w, c| {
            let visible = boolean(param(c, 1)?)?;
            if let Some(r) = record(w, c)? {
                r.multiplier_visible = visible;
            }
            Ok(Value::Void)
        },
    ),
    // FOUND (0x64b7f4 -> 0x5e838c). Declared static
    // `void SetPlayerScoreMultiplierSource(EPlayerNumber, Thing)`: the record keeps the
    // Thing (null allowed). Not modelled: for a Thing the game also takes its sensor
    // position for the HUD.
    (
        "Game",
        "SetPlayerScoreMultiplierSource__Q18Game.EPlayerNumberQ5Thing",
        |w, c| {
            let source = thing(param(c, 1)?)?;
            if let Some(r) = record(w, c)? {
                r.multiplier_source = source;
            }
            Ok(Value::Void)
        },
    ),
    // FOUND (0x5fe830 -> 0x5e86ac). Declared static `f32 GetPlayerScore(EPlayerNumber)`:
    // the record's score; without a record a constant, read as 0.0.
    ("Game", "GetPlayerScore__Q18Game.EPlayerNumber", |w, c| {
        Ok(Value::Float(record(w, c)?.map_or(0., |r| r.score)))
    }),
    // FOUND (0x5fe7f8 -> 0x5e85ec -> 0x3970e4). Declared static
    // `void SetPlayerScore(EPlayerNumber, f32)`: nothing without a record. In play mode a
    // negative value becomes 0.0, a value above [`MAX_PLAYER_SCORE`] becomes that, and
    // the result is stored. Not modelled: the score history the game appends to (frame
    // and score pairs on the record) and the HUD call. In edit mode the game takes a
    // branch that was not read: an error here.
    ("Game", "SetPlayerScore__Q18Game.EPlayerNumberf", |w, c| {
        let score = float(param(c, 1)?)?;
        if w.play_or_edit_mode()? == 1 {
            return Err("edit mode is not analysed".into());
        }
        if let Some(r) = record(w, c)? {
            let floored = if score >= 0. { score } else { 0. };
            r.score = if floored - MAX_PLAYER_SCORE >= 0. {
                MAX_PLAYER_SCORE
            } else {
                floored
            };
        }
        Ok(Value::Void)
    }),
    // FOUND (0x600640 -> 0x5eb810 -> 0xce4fc). Declared static
    // `s32 GetGlobalSettingUID(Game.EGlobalSettingsType)`: slot `type` of the PWorld's
    // GlobalSettingsThingUIDs (8 slots).
    (
        "Game",
        "GetGlobalSettingUID__Q24Game.EGlobalSettingsType",
        |w, c| Ok(Value::Int(w.global_setting_uid(int(param(c, 0)?)?)?)),
    ),
    // FOUND (0x600600 -> 0x5eb718 -> 0xce554). Declared static
    // `void SetGlobalSettingUID(Game.EGlobalSettingsType, s32 uid, s32 priority)`. With
    // cur = GlobalSettingsThingPriority[type]: nothing when priority < cur, nothing when
    // cur is 1; else the slot's UID becomes `uid`, its priority 0, and for a uid other
    // than 0 its priority becomes `priority`. Not modelled: a call (0xce520) made just
    // before the write.
    (
        "Game",
        "SetGlobalSettingUID__Q24Game.EGlobalSettingsTypeii",
        |w, c| {
            let (kind, uid, priority) =
                (int(param(c, 0)?)?, int(param(c, 1)?)?, int(param(c, 2)?)?);
            w.set_global_setting_uid(kind, uid, priority)?;
            Ok(Value::Void)
        },
    ),
    // FOUND (0x6023fc -> 0x5e7814 -> 0x16cb18). Declared static `SlotID GetLevelSlotID()`:
    // a new script SlotID object (Type, SlotNumber) made from the game state's current
    // slot; never null. Here the world hands out one object made ahead (a native cannot
    // allocate). INFERRED: its contents (see the world).
    ("Game", "GetLevelSlotID__", |w, _| w.level_slot_id(false)),
    // FOUND (0x602390 -> 0x16ca08). Declared static `SlotID GetPrimaryLevelSlotID()`: the
    // same from the game state's primary slot.
    ("Game", "GetPrimaryLevelSlotID__", |w, _| {
        w.level_slot_id(true)
    }),
    // FOUND (0x5f86bc -> 0x18cf18). Declared static `bool IsVcrPaused()`: with a level
    // loaded, the PWorld's IsPaused byte. INFERRED: what that byte holds in play (see the
    // world).
    ("Game", "IsVcrPaused__", |w, _| {
        bool_query(w, "Game.IsVcrPaused__")
    }),
    // FOUND (0x5f8a48 -> 0x156e44). Declared static `bool IsTutorialLevel()`: one byte of
    // the game state. What sets it is not established.
    ("Game", "IsTutorialLevel__", |w, _| {
        bool_query(w, "Game.IsTutorialLevel__")
    }),
    // FOUND (0x5f8510 -> 0x3abd8 -> 0x3a9a4). Declared static `bool IsPSNDemoBuild()`: a
    // query of the build.
    ("Game", "IsPSNDemoBuild__", |w, _| {
        bool_query(w, "Game.IsPSNDemoBuild__")
    }),
    // FOUND (0x5f9054 -> 0x73273c -> 0x2c6a14). Declared static
    // `bool IsSafeToTriggerMusic(s32)`: true at once in play mode. Edit mode makes more
    // tests, which were not read: an error here.
    ("Audio", "IsSafeToTriggerMusic__i", |w, c| {
        int(param(c, 0)?)?;
        match w.play_or_edit_mode()? {
            0 => Ok(Value::Bool(true)),
            mode => Err(format!("mode {mode} is not analysed")),
        }
    }),
    // FOUND (0x641570 -> 0x1343dc). Declared `Thing FindLockMesh()` on TriggerLevelLink:
    // walks the Thing's descendants depth first and returns the first whose PRenderMesh
    // shows mesh [`LOCK_MESH`]; null if none. Not modelled: the walk does not go below a
    // Thing with a certain part (bit 0 of the Thing's part flags; which part is not
    // established).
    ("TriggerLevelLink", "FindLockMesh__", |w, c| {
        Ok(Value::Thing(w.find_mesh_descendant(this(c)?, LOCK_MESH)?))
    }),
    // ---- stage 4: calls that are recorded and do nothing else ([`RECORDED_ONLY`]) ----
    // FOUND (0x5f5320 -> 0x756b70 -> 0x3f506c). Declared `void SetPieChartMultiplier(s32)`:
    // one write into a HUD object.
    ("World", "SetPieChartMultiplier__i", |w, c| {
        int(param(c, 0)?)?;
        recorded(w, "SetPieChartMultiplier", None, c.params)
    }),
    // FOUND (0x641fe4 -> 0x753a1c -> 0x123874). Declared `void UpdateScale()`: works the
    // Thing's scale out again against its parent and refreshes switch and shape data.
    // Nothing here depends on that until shapes are simulated.
    ("Thing", "UpdateScale__", |w, c| {
        recorded(w, "UpdateScale", receiver(c)?, c.params)
    }),
    // FOUND (0x642cd4 -> 0x754aac -> 0xebf98). Declared static `void HideRenderMesh(Thing)`:
    // nothing for a null Thing or one without a PRenderMesh; else the visibility call with
    // "off" and the mask 3: both bits are cleared in the PRenderMesh's visibility byte, and
    // the same is done down the Thing's children that have a PRenderMesh and no PShape
    // (not below a Thing with a PShape or with one of two globe meshes). The level-link
    // script hides its lock mesh with it.
    ("Thing", "HideRenderMesh__Q5Thing", |w, c| {
        if let Some(t) = thing(param(c, 0)?)? {
            w.set_render_mesh_visible(t, false)?;
        }
        Ok(Value::Void)
    }),
    // FOUND (0x644d7c -> 0x5e9260). Declared `bool IsVisibleInPlayMode()` on PostItNote:
    // bit 0 of the visibility byte of the Thing's PRenderMesh; false for a null Thing or
    // one without a PRenderMesh. This names the bit: a mesh without it is not visible
    // while a level is played. The note's script asks it once; the measuring runs do not
    // reach that call.
    ("PostItNote", "IsVisibleInPlayMode__", |w, c| {
        Ok(Value::Bool(match receiver(c)? {
            Some(t) => w.render_mesh_visible_in_play_mode(t)?,
            None => false,
        }))
    }),
    // FOUND (0x642c94 -> 0x753e3c -> 0xebf98). Declared static `void ShowRenderMesh(Thing)`:
    // as `HideRenderMesh` with "on": both bits are set. One script calls it (the level
    // link, when it shows its lock again); the measuring runs do not reach that call.
    ("Thing", "ShowRenderMesh__Q5Thing", |w, c| {
        if let Some(t) = thing(param(c, 0)?)? {
            w.set_render_mesh_visible(t, true)?;
        }
        Ok(Value::Void)
    }),
    // FOUND (0x6428ac -> 0x753e88). Declared static
    // `void SetRenderMeshEditorTexture(Thing, Resource)`: nothing for a null Thing; else
    // the texture is stored on its PRenderMesh. Display state.
    (
        "Thing",
        "SetRenderMeshEditorTexture__Q5ThingQ8Resource",
        |w, c| {
            let t = thing(param(c, 0)?)?;
            recorded(w, "SetRenderMeshEditorTexture", t, &c.params[1..])
        },
    ),
    // FOUND (0x642924 -> 0x753ed4 -> 0xecac4). Declared static
    // `void SetRenderMeshEditorColour(Thing, v4)`: nothing for a null Thing or one without
    // a PRenderMesh; else the colour is packed to 32 bits and stored in two words of the
    // PRenderMesh. INFERRED: those are the editor colour a level file stores.
    ("Thing", "SetRenderMeshEditorColour__Q5Thingr", |w, c| {
        let colour = vec4(param(c, 1)?)?;
        if let Some(t) = thing(param(c, 0)?)? {
            w.set_render_mesh_colour(t, colour)?;
        }
        Ok(Value::Void)
    }),
    // FOUND (0x642ac4 -> 0x7538dc). Declared `void RemoveFromAnimatingDivergentList()`:
    // takes the Thing off a PWorld list used for network divergence.
    ("Thing", "RemoveFromAnimatingDivergentList__", |w, c| {
        recorded(
            w,
            "RemoveFromAnimatingDivergentList",
            receiver(c)?,
            c.params,
        )
    }),
    // FOUND (0x641e04 -> 0x6d27bc). Declared `void NotifyNpcsOfMouthEnd()`: the callee is
    // an empty function. Recorded all the same, so the call is counted.
    ("Trigger_Speech_Bubble", "NotifyNpcsOfMouthEnd__", |w, c| {
        recorded(w, "NotifyNpcsOfMouthEnd", None, c.params)
    }),
    // FOUND (0x5f4648 -> 0x6d2e64). Declared `void SetGameNoJoin()` on NoJoinMarker: sets
    // a session byte and tells the network layer. Nothing offline.
    ("NoJoinMarker", "SetGameNoJoin__", |w, c| {
        recorded(w, "SetGameNoJoin", None, c.params)
    }),
    // FOUND (0x5fc2b0 -> 0x5ea5f4). Declared static `void GotoScoreboard(bool)`: sends the
    // game message that ends the level. Game flow nothing here acts on: the request is
    // recorded.
    ("Game", "GotoScoreboard__b", |w, c| {
        boolean(param(c, 0)?)?;
        recorded(w, "GotoScoreboard", None, c.params)
    }),
    // ---- D263: the natives the stage 4 runs reached in most levels ----
    // Same form as stage 4: handler -> reader in the 01.33 executable, the declared
    // signature, FOUND or INFERRED.
    //
    // FOUND (0x647f58 -> 0x6fa3dc -> 0x388358). Declared static
    // `bool IsJustPressed(Thing, s32 mode, s32 buttons)`: the Thing's PYellowHead gives a
    // player number, that player's record a pad; [`PadState::is_just_pressed`] on it.
    // False for a null Thing, no PYellowHead, no record or no pad. Not modelled: the
    // button hint the game can show for the call.
    ("Pad", "IsJustPressed__Q5Thingii", |w, c| {
        pad_test(w, c, PadState::is_just_pressed)
    }),
    // FOUND (0x647fc4 -> 0x6fa4f0 -> 0x388e2c). Declared static
    // `bool IsPressed(Thing, s32, s32)`: [`PadState::is_pressed`] on the same pad.
    ("Pad", "IsPressed__Q5Thingii", |w, c| {
        pad_test(w, c, PadState::is_pressed)
    }),
    // FOUND (0x647eec -> 0x6fa958 -> 0x389400). Declared static
    // `bool IsJustReleased(Thing, s32, s32)`: [`PadState::is_just_released`].
    ("Pad", "IsJustReleased__Q5Thingii", |w, c| {
        pad_test(w, c, PadState::is_just_released)
    }),
    // FOUND (0x617c0c). Declared static `Resource LoadAnimatedTextureByFilename(guid)`:
    // builds a resource descriptor from the GUID like `LoadTextureByFilename__g`, with
    // another resource type constant. The handle here is the GUID either way.
    ("Resource", "LoadAnimatedTextureByFilename__g", |w, c| {
        Ok(Value::Ref(w.load_resource(
            ResourceKind::Texture,
            guid(param(c, 0)?)?,
        )?))
    }),
    // FOUND (0x5f7c64 -> 0x751758 -> 0x51bc0 -> 0x159c1c). Declared static
    // `bool IsInLams(s32)`: the key (what `MakeLamsKeyID` returns, compared unsigned) is
    // searched in the sorted rows of every translation resource that is loaded; true
    // when one has it. The same search gives `Translate__i` its string.
    ("LAMS", "IsInLams__i", |w, c| {
        Ok(Value::Bool(w.in_lams(int(param(c, 0)?)?)?))
    }),
    // FOUND (0x5f7fe0 -> 0x736260). Declared static
    // `bool Messaging_ShouldFilterMessageStrings()`: true unless the level slot's type
    // is 2; for type 2 a test of the network session decides (see the world).
    (
        "GameMessagingManager",
        "Messaging_ShouldFilterMessageStrings__",
        |w, _| {
            bool_query(
                w,
                "GameMessagingManager.Messaging_ShouldFilterMessageStrings__",
            )
        },
    ),
    // FOUND (0x5fa9c8 -> 0x19f63c). Declared static `s32 GetGameFrame()`: the PWorld's
    // frame counter, 0 without a level.
    ("Game", "GetGameFrame__", |w, _| {
        int_query(w, "Game.GetGameFrame__")
    }),
    // FOUND (0x5faac8 -> 0x5ed9f8). Declared static `s32 GetNumYellowHeadAvatars()`: the
    // length of the PWorld's PYellowHead list, the list `GetYellowheadFromPlayerNumber`
    // walks.
    ("Game", "GetNumYellowHeadAvatars__", |w, _| {
        int_query(w, "Game.GetNumYellowHeadAvatars__")
    }),
    // FOUND (0x5f9710 -> 0x16aa4c). Declared `s32 GetGameScoreDataType()` on Score_Board:
    // 6 when `PWorld.GameMode` is 1, else max(n - 1, 0) for a count n kept in the game
    // state. INFERRED: n is the number of players the score table is kept for.
    ("Score_Board", "GetGameScoreDataType__", |w, _| {
        int_query(w, "Score_Board.GetGameScoreDataType__")
    }),
    // FOUND (0x63bef8 -> 0x743e64 -> 0x229bb8). Declared
    // `bool IsSlotIDValidForScoring(SlotID, s32)` on Score_Board: reads Type and
    // SlotNumber out of the script object; false for (0, 0), else true for type 0, 1 or
    // 8 and false for any other. Not modelled: a second test on a value looked up by
    // the s32 argument, which can still say no when a global byte is clear.
    (
        "Score_Board",
        "IsSlotIDValidForScoring__Q6SlotIDi",
        |w, c| {
            int(param(c, 1)?)?;
            let (kind, number) = w.slot_id(param(c, 0)?)?;
            Ok(Value::Bool(
                (kind, number) != (0, 0) && matches!(kind, 0 | 1 | 8),
            ))
        },
    ),
    // FOUND (0x640500 -> 0x7477d8 -> 0x1a6cd8, then 0x50c2cc). Declared static
    // `SlotID GetSlotGroup(SlotID)`: reads Type and SlotNumber out of the script object,
    // looks the slot up in the game's slot database and builds a new script SlotID from
    // the group pair the slot record holds, or from (0, 0) when the slot is not found.
    // Never null (the builder `GetLevelSlotID` uses). The lookup finds nothing for the
    // slot (0, 0); the world answers that case and has no database for any other.
    ("Slot", "GetSlotGroup__Q6SlotID", |w, c| {
        w.slot_group(param(c, 0)?)
    }),
    // FOUND shape (0x5f9ff0 -> 0x73a750). Declared static
    // `s32 GetPlayerIndicatorActivityType(s32 player)`: 0, 1 or 2; 0 = no off-screen
    // indicator. INFERRED for the world: 0 (see the world).
    (
        "OffScreenIndicator",
        "GetPlayerIndicatorActivityType__i",
        |w, c| {
            int(param(c, 0)?)?;
            int_query(w, "OffScreenIndicator.GetPlayerIndicatorActivityType__i")
        },
    ),
    // FOUND (0x5ffec8 -> 0x5e727c). Declared static
    // `s32 GetNumKeyNames(Gooey.EGooeyNetworkAction)`: the length of a sorted name list
    // the game rebuilds for the action (for actions 0x50 and 0x14b the names of the
    // level's Sackbots). INFERRED for the world: no names (see the world).
    (
        "Game",
        "GetNumKeyNames__Q25Gooey.EGooeyNetworkAction",
        |w, c| {
            int(param(c, 0)?)?;
            int_query(w, "Game.GetNumKeyNames__Q25Gooey.EGooeyNetworkAction")
        },
    ),
    // FOUND (0x62a48c -> 0x5e71a0). Declared static
    // `s32 GetKeyIndex(Gooey.EGooeyNetworkAction, String)`: the position of the string
    // in that list (a null string counts as empty), -1 when it is not there.
    (
        "Game",
        "GetKeyIndex__Q25Gooey.EGooeyNetworkActionQ6String",
        |w, c| {
            int(param(c, 0)?)?;
            opt_text(param(c, 1)?)?;
            int_query(w, "Game.GetKeyIndex__Q25Gooey.EGooeyNetworkActionQ6String")
        },
    ),
    // FOUND (0x5f99f8 -> 0x56a838). Declared static `s32 GetTextEntryType()` on Poppet:
    // the function returns the constant 2.
    ("Poppet", "GetTextEntryType__", |_, _| Ok(Value::Int(2))),
    // FOUND (0x5f827c -> 0x5e68d8). Declared static `bool IsTarsierDLCEnabled()`: the
    // function returns the constant 1.
    ("Game", "IsTarsierDLCEnabled__", |_, _| {
        Ok(Value::Bool(true))
    }),
    // FOUND (0x5f7b2c -> 0x7398a4 -> 0x3ea794). Declared static
    // `bool IsPartyLeaderOverlayActive()` on ModalOverlay: a byte of the party leader's
    // modal overlay entry, false when he has none. INFERRED for the world: no overlay.
    ("ModalOverlay", "IsPartyLeaderOverlayActive__", |w, _| {
        bool_query(w, "ModalOverlay.IsPartyLeaderOverlayActive__")
    }),
    // FOUND (0x5fac6c). Declared static `s32 GetNumKnownItemsCollectedAtStart()`: one
    // word of the game state; its writer was not located. INFERRED for the world: 0, a
    // fresh profile.
    ("Game", "GetNumKnownItemsCollectedAtStart__", |w, _| {
        int_query(w, "Game.GetNumKnownItemsCollectedAtStart__")
    }),
    // ---- D263: global settings ----
    // FOUND: the PWorld keeps, for each one-float setting, a start value, a target and a
    // blend factor t; the value in force is start + (target - start) * t. The scripts
    // drive t themselves through `SetGlobalSettingsBlendFactor`; no per-frame step of t
    // was seen. All of these do nothing without a level.
    //
    // FOUND (0x5fc13c -> 0xd9dac). Declared static
    // `void SetGlobalSettingsTo(f32, f32, f32, f32, f32, bool blend)`; the arguments are
    // Lighting, Darkness, Fog, FogTint, ColourCorrection (names from the script that
    // calls it). Without blend: start = target = the arguments, t = 1.0. With blend:
    // start = the blend in force (0xd978c), target = the arguments, t = 0. Not
    // modelled: the lighting block the game rebuilds afterwards.
    ("Game", "SetGlobalSettingsTo__fffffb", |w, c| {
        let f = |i| float(param(c, i)?);
        let values = [f(0)?, f(1)?, f(2)?, f(3)?, f(4)?];
        w.set_lighting_to(values, boolean(param(c, 5)?)?)?;
        Ok(Value::Void)
    }),
    // FOUND (0x5fbfd4 -> 0xcef3c). Declared static
    // `void SetGlobalSettingsGravityTo(f32, bool blend)`. Without blend: start = target
    // = value, t = 1.0. With blend: start = start + (target - start) * t, target =
    // value, t = 0. Not modelled: the physics world's gravity vector, written too.
    ("Game", "SetGlobalSettingsGravityTo__fb", |w, c| {
        setting_to(w, c, GS_GRAVITY)
    }),
    // FOUND (0x5fc108 -> 0xce608). `void SetGlobalSettingsWaterLevelTo(f32, bool)`: the
    // gravity rule on the water level's start, target and t.
    ("Game", "SetGlobalSettingsWaterLevelTo__fb", |w, c| {
        setting_to(w, c, GS_WATER_LEVEL)
    }),
    // FOUND (0x5fc0d4 -> 0xce654). `void SetGlobalSettingsWaterWaveMagnitudeTo(f32, bool)`:
    // the same rule.
    (
        "Game",
        "SetGlobalSettingsWaterWaveMagnitudeTo__fb",
        |w, c| setting_to(w, c, GS_WATER_WAVE_MAGNITUDE),
    ),
    // FOUND shape (0x5fc0a0 -> 0xd5d68 -> 0xd5c50).
    // `void SetGlobalSettingsWaterTintTo(f32, bool)`: the tint is one stored value (no
    // start and target pair), from which the game picks between 9 colours. INFERRED:
    // the stored value becomes the argument (the store itself was not read); the blend
    // flag and the colours are not modelled.
    ("Game", "SetGlobalSettingsWaterTintTo__fb", |w, c| {
        setting_to(w, c, GS_WATER_TINT)
    }),
    // FOUND (0x5fc06c -> 0xd51b0). `void SetGlobalSettingsWaterMurkinessTo(f32, bool)`:
    // the same rule. Not modelled: the renderer value written from the result.
    ("Game", "SetGlobalSettingsWaterMurkinessTo__fb", |w, c| {
        setting_to(w, c, GS_WATER_MURKINESS)
    }),
    // FOUND (0x5fc038 -> 0xce6a0). `void SetGlobalSettingsWaterBitsTo(f32, bool)`: the
    // same rule.
    ("Game", "SetGlobalSettingsWaterBitsTo__fb", |w, c| {
        setting_to(w, c, GS_WATER_BITS)
    }),
    // FOUND (0x5fc008 -> 0x5ec908). Declared static
    // `void SetGlobalSettingsWaterDrainSoundsEnabled(bool)`: one byte of the PWorld.
    (
        "Game",
        "SetGlobalSettingsWaterDrainSoundsEnabled__b",
        |w, c| {
            w.set_water_drain_sounds(boolean(param(c, 0)?)?)?;
            Ok(Value::Void)
        },
    ),
    // FOUND (0x600524 -> 0x5ecf34 -> 0xd623c). Declared static
    // `void SetGlobalSettingsBlendFactor(Game.EGlobalSettingsType, f32)`: t of that type
    // becomes the value. Not modelled: gravity, tint and murkiness are pushed on to
    // physics and the renderer.
    (
        "Game",
        "SetGlobalSettingsBlendFactor__Q24Game.EGlobalSettingsTypef",
        |w, c| {
            w.set_global_setting_blend(int(param(c, 0)?)?, float(param(c, 1)?)?)?;
            Ok(Value::Void)
        },
    ),
    // FOUND (0x5fd630 -> 0x5ec6f8 -> 0xce438). Declared static
    // `f32 GetGlobalSettingsWaterLevel()`: the blended level while it is above 0.0001
    // (and an editor flag is clear); else, and without a level, 0.0.
    ("Game", "GetGlobalSettingsWaterLevel__", |w, _| {
        Ok(Value::Float(w.global_setting_value(GS_WATER_LEVEL)?))
    }),
    // FOUND (0x5fd5b8 -> 0x5ec4f8). `f32 GetGlobalSettingsWaterWaveMagnitude()`: the
    // blended value; 0.0 without a level, as the three getters below.
    ("Game", "GetGlobalSettingsWaterWaveMagnitude__", |w, _| {
        Ok(Value::Float(
            w.global_setting_value(GS_WATER_WAVE_MAGNITUDE)?,
        ))
    }),
    // FOUND (0x5fd57c -> 0x5ec424). `f32 GetGlobalSettingsWaterTint()`: the stored tint
    // value, not blended.
    ("Game", "GetGlobalSettingsWaterTint__", |w, _| {
        Ok(Value::Float(w.global_setting_value(GS_WATER_TINT)?))
    }),
    // FOUND (0x5fd540 -> 0x5ec340). `f32 GetGlobalSettingsWaterMurkiness()`: blended.
    ("Game", "GetGlobalSettingsWaterMurkiness__", |w, _| {
        Ok(Value::Float(w.global_setting_value(GS_WATER_MURKINESS)?))
    }),
    // FOUND (0x5fd504 -> 0x5ec25c). `f32 GetGlobalSettingsWaterBits()`: blended.
    ("Game", "GetGlobalSettingsWaterBits__", |w, _| {
        Ok(Value::Float(w.global_setting_value(GS_WATER_BITS)?))
    }),
    // ---- D263, D265: messaging ([`MESSAGING`]) ----
    // The messaging natives go to one of the game's five message managers, which does
    // nothing when the Thing already has a message of that type; the world keeps each
    // request with its arguments and runs its message list by them.
    //
    // FOUND (0x64b010 -> 0x7382e0, message type 1). Declared static
    // `void Messaging_NewTriggerMessage(String, String, Thing, bool, s32, bool)`: text,
    // actor name, the bubble's Thing, triggered once, bubble position, prevent close
    // (names from the script that calls it).
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessage__Q6StringQ6StringQ5Thingbib",
        |w, c| {
            let t = thing(param(c, 2)?)?;
            recorded(w, "Messaging_NewTriggerMessage", t, c.params)
        },
    ),
    // FOUND (0x64af64 -> 0x7380f8, message type 2). The same six arguments.
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageNoPrompt__Q6StringQ6StringQ5Thingbib",
        |w, c| {
            let t = thing(param(c, 2)?)?;
            recorded(w, "Messaging_NewTriggerMessageNoPrompt", t, c.params)
        },
    ),
    // FOUND (0x64ac74 -> 0x737f64, message type 0x5d). The caption form: the same six
    // arguments, declared as `Messaging_NewTriggerMessage`.
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageCaption__Q6StringQ6StringQ5Thingbib",
        |w, c| {
            let t = thing(param(c, 2)?)?;
            recorded(w, "Messaging_NewTriggerMessageCaption", t, c.params)
        },
    ),
    // FOUND (0x64abc8 -> 0x737d7c, message type 0x5e).
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageCaptionNoPrompt__Q6StringQ6StringQ5Thingbib",
        |w, c| {
            let t = thing(param(c, 2)?)?;
            recorded(w, "Messaging_NewTriggerMessageCaptionNoPrompt", t, c.params)
        },
    ),
    // FOUND (0x64ab1c -> 0x737be8, message type 0x5f). The think form.
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageThink__Q6StringQ6StringQ5Thingbib",
        |w, c| {
            let t = thing(param(c, 2)?)?;
            recorded(w, "Messaging_NewTriggerMessageThink", t, c.params)
        },
    ),
    // FOUND (0x64aa70 -> 0x737a00, message type 0x60).
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageThinkNoPrompt__Q6StringQ6StringQ5Thingbib",
        |w, c| {
            let t = thing(param(c, 2)?)?;
            recorded(w, "Messaging_NewTriggerMessageThinkNoPrompt", t, c.params)
        },
    ),
    // FOUND (0x64aedc -> 0x737320, message type 3). Declared static
    // `void Messaging_NewTriggerMessageSubtitles(String, Thing, bool, bool)`: text, the
    // Thing, a flag, is dialogue.
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageSubtitles__Q6StringQ5Thingbb",
        |w, c| {
            let t = thing(param(c, 1)?)?;
            recorded(w, "Messaging_NewTriggerMessageSubtitles", t, c.params)
        },
    ),
    // FOUND (0x64add8 -> 0x735bd0, message type 0x2b). Declared static
    // `void Messaging_NewLevelLinkPrompt(Thing)`.
    (
        "GameMessagingManager",
        "Messaging_NewLevelLinkPrompt__Q5Thing",
        |w, c| {
            let t = thing(param(c, 0)?)?;
            recorded(w, "Messaging_NewLevelLinkPrompt", t, c.params)
        },
    ),
    // FOUND (0x64ad98 -> 0x735b20, message type 0x59). Declared static
    // `void Messaging_NewWormHolePrompt(Thing)`.
    (
        "GameMessagingManager",
        "Messaging_NewWormHolePrompt__Q5Thing",
        |w, c| {
            let t = thing(param(c, 0)?)?;
            recorded(w, "Messaging_NewWormHolePrompt", t, c.params)
        },
    ),
    // FOUND (0x64ad20 -> 0x7353ec). Declared static
    // `void Messaging_NewGameEvent(s32 manager, s32 event, Thing, bool)`: an event for
    // one of the five managers. A speech bubble closes its message with (4, 3, self,
    // false).
    (
        "GameMessagingManager",
        "Messaging_NewGameEvent__iiQ5Thingb",
        |w, c| {
            let t = thing(param(c, 2)?)?;
            recorded(w, "Messaging_NewGameEvent", t, c.params)
        },
    ),
    // ---- D263: recorded only ([`RECORDED_ONLY`]) ----
    // FOUND (0x64a948 -> 0x4b6d24). Declared static
    // `void RegisterOverlayCallback(Thing, String)` on HUD: the HUD pass will call the
    // named script function on the Thing. Nothing calls it here yet.
    ("HUD", "RegisterOverlayCallback__Q5ThingQ6String", |w, c| {
        opt_text(param(c, 1)?)?;
        let t = thing(param(c, 0)?)?;
        recorded(w, "RegisterOverlayCallback", t, c.params)
    }),
    // FOUND (0x62a258 -> 0x4ba780). Declared static `void PreloadGlyphs(Fonts.EFont,
    // String)` on HUD: warms a font's glyph cache with the string's characters.
    ("HUD", "PreloadGlyphs__Q11Fonts.EFontQ6String", |w, c| {
        recorded(w, "PreloadGlyphs", None, c.params)
    }),
    // FOUND (0x62a538 -> 0x5e67b0). Declared static `void TrackSpeechKey(String)`: the
    // callee is an empty function. The key is kept all the same.
    ("Game", "TrackSpeechKey__Q6String", |w, c| {
        recorded(w, "TrackSpeechKey", None, c.params)
    }),
    // FOUND (0x641e44 -> 0x6d3de0). Declared `void NotifyNpcsOfMouthStart(s32)` on
    // Trigger_Speech_Bubble: an event for every Sackbot, about the Sackbot with that
    // index or, for a negative index, about the bubble's own Thing.
    (
        "Trigger_Speech_Bubble",
        "NotifyNpcsOfMouthStart__i",
        |w, c| {
            int(param(c, 0)?)?;
            recorded(w, "NotifyNpcsOfMouthStart", receiver(c)?, c.params)
        },
    ),
    // FOUND (0x642b04 -> 0x7552d4). Declared `void AddToAnimatingDivergentList()`: the
    // pair of `RemoveFromAnimatingDivergentList`.
    ("Thing", "AddToAnimatingDivergentList__", |w, c| {
        recorded(w, "AddToAnimatingDivergentList", receiver(c)?, c.params)
    }),
    // FOUND (0x643438 -> 0x7548b0). Declared `void SetAnimLoop(bool)`: the loop flag of
    // the Thing's animation, when it has one. Kept on the Thing; nothing animates by it.
    ("Thing", "SetAnimLoop__b", |w, c| {
        boolean(param(c, 0)?)?;
        recorded(w, "SetAnimLoop", receiver(c)?, c.params)
    }),
    // FOUND (0x637f18 -> 0x4c0ad4). Declared `void SetLastNavSelectionMode(bool)` on a
    // Gooey object (found by its handle, no Thing): one byte of its UI state.
    ("Gooey", "SetLastNavSelectionMode__b", |w, c| {
        recorded(w, "SetLastNavSelectionMode", None, c.params)
    }),
    // FOUND (0x62b990 -> 0x4bfe20 -> 0x49c300). Declared
    // `void PerFrameUpdate(Game.EPlayerNumber, s32)` on a Gooey object: its layout pass.
    ("Gooey", "PerFrameUpdate__Q18Game.EPlayerNumberi", |w, c| {
        recorded(w, "PerFrameUpdate", None, c.params)
    }),
    // FOUND (0x6373cc -> 0x4ccd30). Declared `void SetStylesheetScalingFactor(f32)` on a
    // Gooey object: one float of its UI state.
    ("Gooey", "SetStylesheetScalingFactor__f", |w, c| {
        recorded(w, "SetStylesheetScalingFactor", None, c.params)
    }),
    // FOUND (0x5fb4a4 -> 0xd0df0 -> 0x2ce55c). Declared static
    // `void SetGlobalSettingsAudioTo(s32, f32 x 11)`: reverb, three reverb sends, four
    // volumes, three filters and a transition time, handed to the audio system. Kept
    // with its arguments; nothing plays.
    ("Game", "SetGlobalSettingsAudioTo__ifffffffffff", |w, c| {
        recorded(w, "SetGlobalSettingsAudioTo", None, c.params)
    }),
    // ---- D269: the override camera and the screen (docs/level-scripts.md part 20) ----
    // FOUND (01.33, 0x5fc45c -> 0x7349f8 -> 0x3760e4). Declared static
    // `void SetOverrideCamera(v4, v4)` on Cam (script tables). The scoreboard script
    // calls it every frame with a position and a direction it eases itself. The core
    // stores the two vectors in the level's camera and does nothing else; without a
    // level camera it does nothing.
    ("Cam", "SetOverrideCamera__rr", |w, c| {
        w.set_override_camera(vec4(param(c, 0)?)?, vec4(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x5fc348 -> 0x734bc4 -> 0x374cec). Declared static
    // `void SetOverrideCameraActivation(bool, f32)` on Cam. The scoreboard script calls
    // it with (true, 0.05) when its trigger fires and with (false, 0.05) when it is
    // destroyed. The core sets the override's blend target to 1.0 or 0.0 and its blend
    // rate to the float; the camera update steps the blend. No owner and no priority.
    ("Cam", "SetOverrideCameraActivation__bf", |w, c| {
        w.set_override_camera_activation(boolean(param(c, 0)?)?, float(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x609ed0 -> 0x7347e0). Declared static `m44 GetCameraMatrix()` on
    // Cam. The scoreboard script takes column W as the camera's position and minus
    // column Z as its direction. The core copies the render camera's camera matrix out;
    // the look-at builder that fills it (0x2e7990) writes right, up, backward, eye.
    ("Cam", "GetCameraMatrix__", |w, _| {
        Ok(Value::Mat44(w.camera_matrix()?))
    }),
    // FOUND (01.33, 0x601d30 -> 0x4b6e80 -> 0x2e7bdc, 0x2e74a0). Declared static
    // `GetScreenSpacePosition(v4)` on HUD, returning the 2D point type (held in a
    // vector). The
    // speech-bubble script passes its Thing's position each tick and reads x and y
    // against the screen size with a margin of 100. The core projects the point with
    // the render camera's view-projection matrix and maps it to pixels from the top
    // left; the native zeroes the fourth component. INFERRED: the third is the depth.
    ("HUD", "GetScreenSpacePosition__r", |w, c| {
        Ok(Value::Vec4(w.screen_position(vec4(param(c, 0)?)?)?))
    }),
    // FOUND (01.33, 0x5fa5e0 -> 0x4b6a94). Declared static `s32 GetXRes()` on HUD: one
    // global, which the screen-space projection reads too. The speech-bubble script
    // compares the screen position's x with it.
    ("HUD", "GetXRes__", |w, _| {
        Ok(Value::Int(w.screen_size()?[0]))
    }),
    // FOUND (01.33, 0x5fa5a4 -> 0x4b6aac). Declared static `s32 GetYRes()` on HUD: the
    // global after it; the same script compares y with it.
    ("HUD", "GetYRes__", |w, _| {
        Ok(Value::Int(w.screen_size()?[1]))
    }),
    // ---- D270: script names, the pod Thing, the avatar (docs/level-scripts.md part 21) ----
    // FOUND (01.33, 0x61b67c -> 0x753880 -> 0xd3360). Declared static
    // `Thing FindThing(StringA)` on Thing. One script calls it: the Pod's world script,
    // once in its create function, with the name of the Pod's checkpoint; it tests the
    // answer for null and then sets the checkpoint active. The core walks a list of the
    // PWorld of the script's world; an entry has a Thing and a byte string, which the C
    // library's string compare tests against the argument (whole string, case
    // sensitive); the Thing of the first equal entry is returned, null with no equal
    // entry or no world. INFERRED: the list is the level's PScriptName parts in the
    // order of the Things; a null string finds nothing.
    ("Thing", "FindThing__Q7StringA", |w, c| {
        Ok(Value::Thing(match opt_text(param(c, 0)?)? {
            Some(name) => w.find_thing(name.as_bytes())?,
            None => None,
        }))
    }),
    // FOUND (01.33, 0x6011d8 -> 0x7553b8 -> 0xd32dc). Declared static
    // `Thing GetPodThing(bool)` on Thing. One script calls it: the Pod's world script,
    // every tick, with true, and it reads the PShape of the answer without a null test.
    // The core walks a list of the PWorld of the level the game is in; an entry has a
    // Thing and a resource, and the resource's GUID is compared with two constants: the
    // first entry with [`POD_MESH`] gives its Thing; one with [`POD_PLATFORM_MESH`] gives
    // its Thing when the bool is true and ends the search with null when it is false.
    // Null with no such entry or no level. INFERRED: the list is the level's PRenderMesh
    // parts in the order of the Things (`SetRenderMeshMesh` writes the same field).
    ("Thing", "GetPodThing__b", |w, c| {
        Ok(Value::Thing(w.pod_thing(boolean(param(c, 0)?)?)?))
    }),
    // FOUND (01.33, 0x600ed0 -> 0x5e79b8 -> 0x18ec14). Declared static
    // `Thing GetAvatarFromPlayerID(s32)` on Game. The Pod's world script asks for the
    // party leader's avatar each tick and tests it for null. The core is the one
    // `GetYellowheadFromPlayerNumber` reaches: the Thing of the first entry of a PWorld
    // list whose number is the argument, null with none. INFERRED there too: the list is
    // the PYellowHead parts and the number the player number.
    ("Game", "GetAvatarFromPlayerID__i", |w, c| {
        Ok(Value::Thing(w.yellowhead(int(param(c, 0)?)?)?))
    }),
    // FOUND (01.33, 0x617df4 -> 0x50aa08). Declared static
    // `Resource LoadMeshByFilename(guid)` on Resource: asks the resource system for the
    // mesh with that GUID and hands the script a resource handle, as
    // `LoadTextureByFilename__g` does for a texture. The Pod's world script passes the
    // handle straight on to `SetRenderMeshMesh`. Nothing is loaded here; the world keeps
    // that the handle is a mesh.
    ("Resource", "LoadMeshByFilename__g", |w, c| {
        Ok(Value::Ref(
            w.load_resource(ResourceKind::Mesh, guid(param(c, 0)?)?)?,
        ))
    }),
    // FOUND (01.33, 0x642834 -> 0x755580). Declared static
    // `void SetRenderMeshMesh(Thing, Resource)` on Thing. The Pod's world script sets the
    // pod Thing's mesh each tick: the Pod, or the platform once its transition is at 1.
    // Nothing for a null Thing or one without a PRenderMesh; else the PRenderMesh's mesh
    // becomes the resource when the resource's type is a mesh, and none when it is not.
    // INFERRED: a null resource clears it too.
    ("Thing", "SetRenderMeshMesh__Q5ThingQ8Resource", |w, c| {
        let mesh = resource(param(c, 1)?)?;
        if let Some(t) = thing(param(c, 0)?)? {
            w.set_render_mesh_mesh(t, mesh)?;
        }
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x6016a8). Declared static `guid AsGUID(s32)` on FileList: the value
    // unchanged. The Pod's world script turns its two mesh numbers into GUIDs with it.
    ("FileList", "AsGUID__i", |_, c| {
        Ok(Value::Int(int(param(c, 0)?)?))
    }),
    // FOUND (01.33, 0x616c44). Declared static `s32 AsInt(guid)` on FileList: the value
    // unchanged. The speech-bubble script and the Pod's world script compare GUIDs as
    // numbers with it.
    ("FileList", "AsInt__g", |_, c| {
        Ok(Value::Int(guid(param(c, 0)?)? as i32))
    }),
    // FOUND (01.33, 0x601644 -> 0x503358 -> 0x3fcbb4). Declared static
    // `guid GetPlanetsBackgroundGUID()` on PodComputer: one field of the planets level's
    // PWorld, and [`DEFAULT_PLANETS_BACKGROUND`] while that level is not loaded. The
    // Pod's world script picks the texture of its change-over by it.
    ("PodComputer", "GetPlanetsBackgroundGUID__", |w, _| {
        Ok(Value::Int(w.planets_background()? as i32))
    }),
    // FOUND (01.33, 0x5f8588 -> 0x3aa5c). Declared static `bool IsKioskBuild()` on Game:
    // the constant false in this executable. The Pod's intro thread asks it.
    ("Game", "IsKioskBuild__", |_, _| Ok(Value::Bool(false))),
    // FOUND (01.33, 0x5f8644 -> 0x3ef1a4). Declared static `bool IsStartMenuOpen()` on
    // Game: one word of a UI object is not 0. The Pod's world script asks it before it
    // lets a player use the pod computer. A state query: nothing here opens that menu.
    ("Game", "IsStartMenuOpen__", |w, _| {
        bool_query(w, "Game.IsStartMenuOpen__")
    }),
    // FOUND (01.33, 0x644f3c -> 0x502f40). Declared static private
    // `bool IsTakingPhoto(Thing)` on Pod: false for a null Thing or one without a
    // PYellowHead; else an answer from that player's Popit. The Pod's world script asks
    // it about the leader's avatar. A state query: nothing here has a Popit.
    ("Pod", "IsTakingPhoto__Q5Thing", |w, c| {
        match thing(param(c, 0)?)? {
            Some(_) => bool_query(w, "Pod.IsTakingPhoto__Q5Thing"),
            None => Ok(Value::Bool(false)),
        }
    }),
    // ---- D270: recorded only ([`RECORDED_ONLY`]) ----
    // FOUND (01.33, 0x5fb36c -> 0x403ca4). Declared static
    // `void SetFadePodDecorations(f32)` on PodComputer. The Pod's world script passes its
    // transition value each tick. The core takes `GetPodThing(true)` and works over a
    // list of records on one of its parts with the float. INFERRED: the pod's
    // decorations fade; the arithmetic was not read. Display state.
    ("PodComputer", "SetFadePodDecorations__f", |w, c| {
        float(param(c, 0)?)?;
        recorded(w, "SetFadePodDecorations", None, c.params)
    }),
    // FOUND (01.33, 0x5fbd04 -> 0x3fa9cc). Declared static `void RequestHostUsePod(bool)`
    // on PodComputer. The Pod's world script calls it with true when the leader presses
    // the button in the pod computer's zone. Only in the player's own Pod: the core
    // stores the request (use or stop) and the time and sends a message to the host. It
    // does not set "in use" itself: the receiver of that message does (0x3fa868 ->
    // 0x18ed30). The world answers the request (D273).
    ("PodComputer", "RequestHostUsePod__b", |w, c| {
        w.request_host_use_pod(boolean(param(c, 0)?)?)?;
        Ok(Value::Void)
    }),
    // ---- D273: the pod computer is entered (docs/level-scripts.md part 24) ----
    // FOUND (01.33, 0x64a52c -> 0x6d426c -> 0x13bac0). Declared `bool GetIsInPadZone(Thing)`
    // on Native. The Pod's world script asks it about the leader's avatar before it lets
    // him engage the pod computer. False for a null Thing; the rule is the world's.
    ("Native", "GetIsInPadZone__Q5Thing", |w, c| {
        Ok(Value::Bool(match thing(param(c, 0)?)? {
            Some(t) => w.in_pad_zone(t)?,
            None => false,
        }))
    }),
    // FOUND (01.33, 0x629e18 -> 0x6d2f18 -> 0x3890a0). Declared static
    // `bool IsJustClickedByID(s32 player, s32 action, String prompt)` on Input. The
    // second argument is an action number; the string is the key of the on-screen hint.
    // The wrapper passes an action on unchanged when it is in a list filled at run time
    // and 0 when it is not; INFERRED that 8 is in the list (the list was not read).
    ("Input", "IsJustClickedByID__iiQ6String", |w, c| {
        let (player, action) = (int(param(c, 0)?)?, int(param(c, 1)?)?);
        let prompt = opt_text(param(c, 2)?)?;
        Ok(Value::Bool(w.action_just_clicked(player, action, prompt)?))
    }),
    // FOUND (01.33, 0x64a4dc -> 0x6d88b0 -> 0x139308). Declared `v4 GetPlayerColour(Thing)`
    // on Native: the zero vector for a null Thing or one without a PYellowHead; else the
    // player's stored 32-bit colour as (R, G, B, A) / 255. The Pod's world script tints
    // the pod with it and writes its own w.
    ("Native", "GetPlayerColour__Q5Thing", |w, c| {
        let colour = match thing(param(c, 0)?)? {
            Some(t) => w.player_colour(t)?,
            None => None,
        };
        Ok(Value::Vec4(colour.map_or([0.; 4], colour_vector)))
    }),
    // FOUND (01.33, 0x617428 -> 0x4015e8). Declared static
    // `void SetPodTransitionTexture(guid)` on PodComputer. The Pod's world script passes
    // the texture of the planets' backdrop each tick. Once that texture is loaded the
    // core sets it on three engine resources (GUIDs 12138, 12250, 119116; INFERRED the
    // materials of the pod's change-over). Nothing a script can read back. Display state.
    ("PodComputer", "SetPodTransitionTexture__g", |w, c| {
        guid(param(c, 0)?)?;
        recorded(w, "SetPodTransitionTexture", None, c.params)
    }),
    // FOUND (01.33, 0x5f8b38 -> 0x5e8024). Declared static `bool CurrentLevelIsOurPod()`
    // on Game: one byte of the game object, written in one place, when a level becomes
    // the current one (0x1ab508): the slot being entered is the Pod's slot. A state
    // query: the world is told how its level was started.
    ("Game", "CurrentLevelIsOurPod__", |w, _| {
        bool_query(w, "Game.CurrentLevelIsOurPod__")
    }),
    // FOUND (01.33, 0x5f61a4 -> 0x756520). Declared static `bool PressedStartButton()`
    // on Video: over seven low-level pad records in order, the first with bit 0x80000 in
    // one of its words has the press used up (0x687c6c) and the answer is true; none:
    // false. INFERRED: those records are the players' pads in the order of the player
    // records, and the word is the pad's button word ([`START_BUTTON`]).
    ("Video", "PressedStartButton__", |w, _| {
        for i in 0..w.num_players()?.min(START_BUTTON_PADS) {
            let number = w.player_number(i)?;
            let pad = w.player(number)?.and_then(|r| r.pad.as_mut());
            if let Some(pad) = pad.filter(|p| p.now & START_BUTTON != 0) {
                pad.use_up(START_BUTTON);
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }),
    // ---- D271: video, voice-over and progression flags (docs/level-scripts.md part 22) ----
    // Nothing here plays a video or a sound. A request is kept by the world with its
    // arguments, for a later audio or video system, and the script gets the answer the
    // game gives when nothing started ([`RECORDED_REQUESTS`]).
    //
    // FOUND (01.33, 0x642340 -> 0x75672c -> 0x67c620, 0x67c45c). Declared static
    // `s32 StartVideo(Thing, guid, bool)` on Video. Two script functions call it, both
    // of the pod computer, and keep the answer as the pod computer's video handle; the
    // scripts that wait for a video wait while that handle is not -1. The core tries to
    // start the video: started, it returns the video's handle; not started, -1. Here no
    // video starts, so the answer is always the game's -1.
    ("Video", "StartVideo__Q5Thinggb", |w, c| {
        let t = thing(param(c, 0)?)?;
        guid(param(c, 1)?)?;
        boolean(param(c, 2)?)?;
        w.engine_call("StartVideo", t, c.params)?;
        Ok(Value::Int(NO_VIDEO))
    }),
    // FOUND (01.33, 0x5fb5cc -> 0x7566ec -> 0x67c22c, 0x679438). Declared static
    // `bool UpdateVideo(s32, bool)` on Video. One script function calls it, with the pod
    // computer's video handle, and sets that handle to -1 on false. The core looks for
    // the video with the handle: none, false; else it steps the video and returns false
    // once it has ended. No video has a handle here (`StartVideo` hands none out), so
    // the answer is the game's false for every handle.
    ("Video", "UpdateVideo__ib", |_, c| {
        int(param(c, 0)?)?;
        boolean(param(c, 1)?)?;
        Ok(Value::Bool(false))
    }),
    // FOUND (01.33, 0x5f5434 -> 0x67a3d8). Declared static `void StopVideo(s32)` on
    // Video: stops the video with that handle, if there is one. Recorded only.
    ("Video", "StopVideo__i", |w, c| {
        int(param(c, 0)?)?;
        recorded(w, "StopVideo", None, c.params)
    }),
    // FOUND (01.33, 0x619430 -> 0x7324fc -> 0x2df200 -> 0x2de6b8). Declared static
    // `bool RequestVoiceOver(StringA, bool, bool)` on Audio: the name of a line and two
    // flags. Ten script functions call it; the ones the measuring runs reach drop the
    // answer. The core answers false, and starts nothing, while voice-over is switched
    // off (one global), for a line the profile has heard when the second flag is false,
    // and while a line plays that the first flag does not let it interrupt; else it
    // starts the line's sound. Here no line starts: the game's false. INFERRED: the
    // answer is true when a line starts (the end of the core was not read).
    ("Audio", "RequestVoiceOver__Q7StringAbb", |w, c| {
        opt_text(param(c, 0)?)?;
        boolean(param(c, 1)?)?;
        boolean(param(c, 2)?)?;
        w.engine_call("RequestVoiceOver", None, c.params)?;
        Ok(Value::Bool(false))
    }),
    // FOUND (01.33, 0x5f8fdc -> 0x2de218). Declared static `bool IsVoiceOverPlaying()`
    // on Audio: false unless voice-over is switched on, the kept sound handle is valid
    // and its sound is playing. The Pod's threads wait on it. A state query: false,
    // since no line ever starts.
    ("Audio", "IsVoiceOverPlaying__", |w, _| {
        bool_query(w, "Audio.IsVoiceOverPlaying__")
    }),
    // FOUND (01.33, 0x5f9018 -> 0x2de498). Declared static `bool StopVoiceOver()` on
    // Audio: stops the line that plays and returns true, always.
    ("Audio", "StopVoiceOver__", |w, c| {
        w.engine_call("StopVoiceOver", None, c.params)?;
        Ok(Value::Bool(true))
    }),
    // FOUND (01.33, 0x6194f4 -> 0x73255c -> 0x2cdb7c). Declared static
    // `bool SetMixOverlay(StringA, f32)` on Audio: the name of a mix and a float.
    // False while the audio system object (one global pointer) is null; else the
    // audio system's own answer, which was not read. Here there is no audio system: the
    // game's false.
    ("Audio", "SetMixOverlay__Q7StringAf", |w, c| {
        opt_text(param(c, 0)?)?;
        float(param(c, 1)?)?;
        w.engine_call("SetMixOverlay", None, c.params)?;
        Ok(Value::Bool(false))
    }),
    // FOUND (01.33, 0x619494 -> 0x73252c -> 0x2cdaa0). Declared static
    // `bool RemoveMixOverlay(StringA, f32)` on Audio: as `SetMixOverlay`, false without
    // the audio system object. The speech-bubble script calls it when a bubble closes
    // and drops the answer.
    ("Audio", "RemoveMixOverlay__Q7StringAf", |w, c| {
        opt_text(param(c, 0)?)?;
        float(param(c, 1)?)?;
        w.engine_call("RemoveMixOverlay", None, c.params)?;
        Ok(Value::Bool(false))
    }),
    // FOUND (01.33, 0x5f56cc -> 0x73fe44 -> 0x163a4c -> 0x15d22c). Declared static
    // `void SetLBP2GameProgressionFlag(s32)` on Profile. The argument is a bit mask:
    // the core ORs it into one 32-bit word of each local profile and marks the profile
    // changed. The Pod's intro thread sets a mask of two bits when it ends, and the
    // planets level's world script starts that thread only while one of them is clear.
    ("Profile", "SetLBP2GameProgressionFlag__i", |w, c| {
        w.set_progression_flags(int(param(c, 0)?)? as u32)?;
        Ok(Value::Void)
    }),
    // FOUND (01.33, 0x5f6958 -> 0x740084; 0x5f68e0 -> 0x7439dc). Declared static
    // `bool HasPlayedLBP1AndUnlockedPod()` and `...UnlockedMoon()` on Profile: false
    // without a loaded profile record in one of two states, else one byte the record
    // answers with. The Pod's intro thread picks its voice-over line by the first and
    // the flags it sets by the second. State queries. INFERRED: false, a profile
    // without save data of the first game.
    ("Profile", "HasPlayedLBP1AndUnlockedPod__", |w, _| {
        bool_query(w, "Profile.HasPlayedLBP1AndUnlockedPod__")
    }),
    ("Profile", "HasPlayedLBP1AndUnlockedMoon__", |w, _| {
        bool_query(w, "Profile.HasPlayedLBP1AndUnlockedMoon__")
    }),
    // ---- D274: the two Math natives of a planet's per-frame function ----
    // FOUND (01.33, 0x62ae30 -> 0x5ee288). Declared static `v4 transform(m44, v4)` on
    // Math: the matrix times the vector, all four components (column 0 times x plus
    // column 1 times y plus column 2 times z plus column 3 times w). A planet's script
    // passes its size matrix and a point with w 0, so the place of the matrix stays out.
    ("Math", "transform__mr", |_, c| {
        let (m, v) = (matrix(param(c, 0)?)?, vec4(param(c, 1)?)?);
        Ok(Value::Vec4(std::array::from_fn(|row| {
            (0..4).map(|col| m[col * 4 + row] * v[col]).sum()
        })))
    }),
    // FOUND (01.33, 0x5fcf60 -> 0x5eebb0). Declared static `f32 ModAngleToWithinPI(f32)`
    // on Math: with `t = (a + pi) / (2 pi)` the answer is `(t - r(t)) * 2 pi - pi`, the
    // angle less whole turns, from -pi up to pi. INFERRED: `r` rounds down (the call goes
    // through a stub that was not followed).
    ("Math", "ModAngleToWithinPI__f", |_, c| {
        Ok(Value::Float(mod_angle_to_within_pi(float(param(c, 0)?)?)))
    }),
    // ---- the game's own test scripts ----
    ("Test", "Check__bQ7StringAi", |w, c| {
        let ok = boolean(param(c, 0)?)?;
        report(w, c, ok)
    }),
    ("Test", "CheckFalse__bQ7StringAi", |w, c| {
        let ok = !boolean(param(c, 0)?)?;
        report(w, c, ok)
    }),
    ("Test", "Fail__Q7StringAi", |w, c| report(w, c, false)),
    ("Test", "Succeed__Q7StringAi", |w, c| report(w, c, true)),
    ("Test", "CheckEqual__iiQ7StringAi", equal_values),
    ("Test", "CheckEqual__bbQ7StringAi", equal_values),
    ("Test", "CheckEqual__jjQ7StringAi", equal_values),
    ("Test", "CheckEqual__ffQ7StringAi", equal_values),
    ("Test", "CheckEqual__ppQ7StringAi", |w, c| {
        equal_components(w, c, 2)
    }),
    ("Test", "CheckEqual__qqQ7StringAi", |w, c| {
        equal_components(w, c, 3)
    }),
    ("Test", "CheckEqual__rrQ7StringAi", |w, c| {
        equal_components(w, c, 4)
    }),
    ("Test", "CheckSimilar__fffQ7StringAi", |w, c| {
        let (a, b, eps) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        report(w, c, (a - b).abs() <= eps)
    }),
];

/// The natives bound through [`World::state_query`], with their return type.
pub const STATE_QUERIES: [(&str, &str); 30] = [
    ("Game.IsNonInteractive__", "bool"),
    ("Game.GetNumPendingLinks__", "s32"),
    ("Game.GetNumAvatars__", "s32"),
    ("Video.IsVideoPlaying__", "bool"),
    ("System.Is4to3__", "bool"),
    ("Game.GetLocalLeadersPlayerNumber__", "s32"),
    ("Audio.GetCurrMusicGuid__", "s32"),
    ("Audio.UserMusicIsStopped__", "bool"),
    ("Audio.UserMusicIsPaused__", "bool"),
    ("Audio.IsInteractiveMusicAudible__", "bool"),
    ("Input.IsInAttractionLoop__i", "bool"),
    ("Game.GetPartyLeadersPlayerNumber__", "s32"),
    ("Game.IsVcrPaused__", "bool"),
    ("Game.IsTutorialLevel__", "bool"),
    ("Game.IsPSNDemoBuild__", "bool"),
    (
        "GameMessagingManager.Messaging_ShouldFilterMessageStrings__",
        "bool",
    ),
    ("Game.GetGameFrame__", "s32"),
    ("Game.GetNumYellowHeadAvatars__", "s32"),
    ("Score_Board.GetGameScoreDataType__", "s32"),
    (
        "OffScreenIndicator.GetPlayerIndicatorActivityType__i",
        "s32",
    ),
    ("Game.GetNumKeyNames__Q25Gooey.EGooeyNetworkAction", "s32"),
    (
        "Game.GetKeyIndex__Q25Gooey.EGooeyNetworkActionQ6String",
        "s32",
    ),
    ("ModalOverlay.IsPartyLeaderOverlayActive__", "bool"),
    ("Game.GetNumKnownItemsCollectedAtStart__", "s32"),
    ("Game.IsStartMenuOpen__", "bool"),
    ("Pod.IsTakingPhoto__Q5Thing", "bool"),
    ("Audio.IsVoiceOverPlaying__", "bool"),
    ("Profile.HasPlayedLBP1AndUnlockedPod__", "bool"),
    ("Profile.HasPlayedLBP1AndUnlockedMoon__", "bool"),
    ("Game.CurrentLevelIsOurPod__", "bool"),
];

/// Bound natives that return nothing and whose whole effect here is a record in the world
/// ([`World::engine_call`], or a counter for the audio ones): the engine would draw, play
/// or send something that nothing models yet. Reports count calls to them apart from the
/// other bound natives.
pub const RECORDED_ONLY: [(&str, &str); 24] = [
    ("Audio", "PlaySound__Q5ThingQ7StringA"),
    ("Audio", "SetInteractiveMusic__ggffiQ5Thing"),
    ("Audio", "SetInteractiveMusicStereoPairVolume__giff"),
    ("Audio", "StopInteractiveMusic__"),
    ("World", "SetPieChartMultiplier__i"),
    ("Thing", "UpdateScale__"),
    ("Thing", "SetRenderMeshEditorTexture__Q5ThingQ8Resource"),
    ("Thing", "RemoveFromAnimatingDivergentList__"),
    ("Trigger_Speech_Bubble", "NotifyNpcsOfMouthEnd__"),
    ("NoJoinMarker", "SetGameNoJoin__"),
    ("Game", "GotoScoreboard__b"),
    ("HUD", "RegisterOverlayCallback__Q5ThingQ6String"),
    ("HUD", "PreloadGlyphs__Q11Fonts.EFontQ6String"),
    ("Game", "TrackSpeechKey__Q6String"),
    ("Trigger_Speech_Bubble", "NotifyNpcsOfMouthStart__i"),
    ("Thing", "AddToAnimatingDivergentList__"),
    ("Thing", "SetAnimLoop__b"),
    ("Gooey", "SetLastNavSelectionMode__b"),
    ("Gooey", "PerFrameUpdate__Q18Game.EPlayerNumberi"),
    ("Gooey", "SetStylesheetScalingFactor__f"),
    ("Game", "SetGlobalSettingsAudioTo__ifffffffffff"),
    ("PodComputer", "SetFadePodDecorations__f"),
    ("PodComputer", "SetPodTransitionTexture__g"),
    ("Video", "StopVideo__i"),
];

/// Bound natives that ask the engine to play something and return an answer. Nothing
/// plays here: the world keeps the call ([`World::engine_call`]) and the script gets the
/// answer the game gives when nothing started, or the one answer the game has. The third
/// entry is that answer as the script sees it. Reports count calls to them apart from
/// the other bound natives, as they do for [`RECORDED_ONLY`].
pub const RECORDED_REQUESTS: [(&str, &str, &str); 5] = [
    ("Video", "StartVideo__Q5Thinggb", "-1"),
    ("Audio", "RequestVoiceOver__Q7StringAbb", "false"),
    ("Audio", "StopVoiceOver__", "true"),
    ("Audio", "SetMixOverlay__Q7StringAf", "false"),
    ("Audio", "RemoveMixOverlay__Q7StringAf", "false"),
];

/// The `GameMessagingManager` natives that open or close a message. They return nothing
/// and reach the world through [`World::engine_call`], as the recorded-only natives do,
/// but a level world runs its message list by them (docs/level-scripts.md part 16).
pub const MESSAGING: [&str; 10] = [
    "Messaging_NewTriggerMessage__Q6StringQ6StringQ5Thingbib",
    "Messaging_NewTriggerMessageNoPrompt__Q6StringQ6StringQ5Thingbib",
    "Messaging_NewTriggerMessageCaption__Q6StringQ6StringQ5Thingbib",
    "Messaging_NewTriggerMessageCaptionNoPrompt__Q6StringQ6StringQ5Thingbib",
    "Messaging_NewTriggerMessageThink__Q6StringQ6StringQ5Thingbib",
    "Messaging_NewTriggerMessageThinkNoPrompt__Q6StringQ6StringQ5Thingbib",
    "Messaging_NewTriggerMessageSubtitles__Q6StringQ5Thingbb",
    "Messaging_NewLevelLinkPrompt__Q5Thing",
    "Messaging_NewWormHolePrompt__Q5Thing",
    "Messaging_NewGameEvent__iiQ5Thingb",
];

/// Natives the VM itself handles; they never reach a host.
pub const VM_HANDLED: [(&str, &str); 1] = [("System", "Yield__")];

#[cfg(test)]
mod tests {
    use super::*;

    struct Tree;
    impl World for Tree {
        fn parent(&mut self, t: u32) -> Result<Option<u32>, String> {
            Ok(t.checked_sub(1))
        }
        fn thing_uid(&mut self, t: u32) -> Result<i32, String> {
            Ok(t as i32 + 100)
        }
        /// Ports 0..=2 are wired; port 2 carries a player and a zero ternary.
        fn switch_input(&mut self, _t: u32, port: i32) -> Result<Option<SwitchInput>, String> {
            Ok((0..=2).contains(&port).then(|| SwitchInput {
                activation: port as f32 / 2.,
                ternary: if port == 2 { 0 } else { -1 },
                player: if port == 2 { 3 } else { -1 },
            }))
        }
        fn switch_input_thing(&mut self, t: u32, port: i32) -> Result<Option<u32>, String> {
            Ok((port == 0).then_some(t + 1))
        }
    }

    fn call(class: &str, m: &str, args: &[Value]) -> Result<Value, String> {
        dispatch(&mut Tree, class, m, args).expect("bound")
    }

    #[test]
    fn every_entry_has_a_signature_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (c, m) in bound() {
            let s = signature::parse(m).unwrap_or_else(|| panic!("{m}"));
            assert!(
                !s.params
                    .iter()
                    .any(|p| matches!(p, signature::Param::Unknown(_))),
                "{m}"
            );
            assert!(seen.insert((c, m)), "duplicate {c}.{m}");
        }
    }

    #[test]
    fn stage4_lists_name_bound_natives_and_a_world_without_players_says_so() {
        for (name, _) in STATE_QUERIES {
            let (class, mangled) = name.split_once('.').unwrap();
            assert!(lookup(class, mangled).is_some(), "{name}");
        }
        for (class, mangled) in RECORDED_ONLY {
            assert!(lookup(class, mangled).is_some(), "{mangled}");
        }
        for (class, mangled, _) in RECORDED_REQUESTS {
            assert!(lookup(class, mangled).is_some(), "{mangled}");
            assert!(!RECORDED_ONLY.contains(&(class, mangled)), "{mangled}");
        }
        for mangled in MESSAGING {
            assert!(
                lookup("GameMessagingManager", mangled).is_some(),
                "{mangled}"
            );
        }
        // The identity needs no world.
        assert_eq!(
            call("Game", "EPlayerNumberFromInt__i", &[Value::Int(-1)]),
            Ok(Value::Int(-1))
        );
        // A world without players, global settings or a record of engine calls: an
        // error, never a made-up answer.
        let player = [Value::Int(0)];
        for name in [
            "DoesPlayerExist__Q18Game.EPlayerNumber",
            "GetYellowheadFromPlayerNumber__Q18Game.EPlayerNumber",
            "GetPlayerScore__Q18Game.EPlayerNumber",
            "GetGlobalSettingUID__Q24Game.EGlobalSettingsType",
        ] {
            assert!(call("Game", name, &player).is_err(), "{name}");
        }
        assert!(call("Game", "IsVcrPaused__", &[]).is_err());
        assert!(call("Game", "GetLevelSlotID__", &[]).is_err());
        assert!(call("Thing", "UpdateScale__", &[Value::Thing(Some(1))]).is_err());
        // The display natives do nothing for a null Thing and need a world for any other.
        let colour = Value::Vec4([1.; 4]);
        for name in ["HideRenderMesh__Q5Thing", "ShowRenderMesh__Q5Thing"] {
            assert_eq!(call("Thing", name, &[Value::Thing(None)]), Ok(Value::Void));
            assert!(
                call("Thing", name, &[Value::Thing(Some(1))]).is_err(),
                "{name}"
            );
        }
        for name in [
            "SetRenderMeshEditorColour__Q5Thingr",
            "SetRenderMeshEditorColourTint__Q5Thingr",
            "SetShapeEditorColour__Q5Thingr",
            "SetShapeEditorColourTint__Q5Thingr",
        ] {
            let null = [Value::Thing(None), colour.clone()];
            assert_eq!(call("Thing", name, &null), Ok(Value::Void), "{name}");
            let some = [Value::Thing(Some(1)), colour.clone()];
            assert!(call("Thing", name, &some).is_err(), "{name}");
            // The colour is still checked for a null Thing.
            let bad = [Value::Thing(None), Value::Int(1)];
            assert!(call("Thing", name, &bad).is_err(), "{name}");
        }
    }

    #[test]
    fn d263_pad_rules_constants_and_a_world_without_the_state() {
        // The pad rule: every bit of the mask, the mode in the masks.
        let pad = PadState {
            mask_a: 1,
            mask_b: 1,
            now: 0b0110,
            old: 0b0100,
            held_off: 0,
        };
        assert!(pad.is_pressed(1, 0b0110) && pad.is_pressed(1, 0b0010));
        assert!(!pad.is_pressed(1, 0b0111) && !pad.is_pressed(2, 0b0010));
        // Newly down: the whole mask is down now and was not the frame before.
        assert!(pad.is_just_pressed(1, 0b0010) && pad.is_just_pressed(1, 0b0110));
        assert!(!pad.is_just_pressed(1, 0b0100) && !pad.is_just_pressed(1, 0b1000));
        let after = PadState {
            now: 0b0100,
            old: 0b0110,
            ..pad
        };
        assert!(after.is_just_released(1, 0b0010) && after.is_just_released(1, 0b0110));
        assert!(!after.is_just_released(1, 0b0100) && !pad.is_just_released(1, 0b0010));
        // The second mask gates the edge tests only.
        let held = PadState { mask_b: 2, ..pad };
        assert!(held.is_pressed(1, 0b0010) && !held.is_just_pressed(1, 0b0010));
        assert!(!PadState { mask_b: 2, ..after }.is_just_released(1, 0b0010));
        assert!(held.is_just_pressed(3, 0b0010));
        // A mask of no buttons is "all down" in both frames: pressed, never an edge.
        assert!(pad.is_pressed(1, 0) && !pad.is_just_pressed(1, 0));

        // A null Thing never reaches the world; a Thing does, and this world has no pads.
        let args = |t| [Value::Thing(t), Value::Int(1), Value::Int(4)];
        for name in [
            "IsJustPressed__Q5Thingii",
            "IsPressed__Q5Thingii",
            "IsJustReleased__Q5Thingii",
        ] {
            assert_eq!(call("Pad", name, &args(None)), Ok(Value::Bool(false)));
            assert!(call("Pad", name, &args(Some(1))).is_err(), "{name}");
        }
        // The two constants the executable returns as literals need no world.
        assert_eq!(call("Poppet", "GetTextEntryType__", &[]), Ok(Value::Int(2)));
        assert_eq!(
            call("Game", "IsTarsierDLCEnabled__", &[]),
            Ok(Value::Bool(true))
        );
        // Everything else asks the world, and a world without the state says so.
        assert!(call("LAMS", "IsInLams__i", &[Value::Int(1)]).is_err());
        assert!(call("Game", "GetGameFrame__", &[]).is_err());
        assert!(call("Game", "GetGlobalSettingsWaterLevel__", &[]).is_err());
        let to = [Value::Float(1.), Value::Bool(false)];
        assert!(call("Game", "SetGlobalSettingsGravityTo__fb", &to).is_err());
        assert!(call("Game", "TrackSpeechKey__Q6String", &[Value::Object(None)]).is_err());
    }

    #[test]
    fn d269_camera_and_screen_natives_check_their_arguments_and_ask_the_world() {
        let v = Value::Vec4([1., 2., 3., 1.]);
        // A world without a camera or a screen says so.
        let pair = [v.clone(), v.clone()];
        assert!(call("Cam", "SetOverrideCamera__rr", &pair).is_err());
        let on = [Value::Bool(true), Value::Float(0.05)];
        assert!(call("Cam", "SetOverrideCameraActivation__bf", &on).is_err());
        assert!(call("Cam", "GetCameraMatrix__", &[]).is_err());
        let point = std::slice::from_ref(&v);
        assert!(call("HUD", "GetScreenSpacePosition__r", point).is_err());
        assert!(call("HUD", "GetXRes__", &[]).is_err());
        assert!(call("HUD", "GetYRes__", &[]).is_err());

        /// A world with a screen and a camera that keeps what it is given.
        #[derive(Default)]
        struct Screen(Vec<String>);
        impl World for Screen {
            fn set_override_camera(&mut self, p: [f32; 4], d: [f32; 4]) -> Result<(), String> {
                self.0.push(format!("set {} {}", p[0], d[2]));
                Ok(())
            }
            fn set_override_camera_activation(&mut self, on: bool, r: f32) -> Result<(), String> {
                self.0.push(format!("activation {on} {r}"));
                Ok(())
            }
            fn camera_matrix(&mut self) -> Result<[f32; 16], String> {
                Ok(IDENTITY)
            }
            fn screen_position(&mut self, p: [f32; 4]) -> Result<[f32; 4], String> {
                Ok([p[0] * 2., p[1] * 2., 0.5, 0.])
            }
            fn screen_size(&mut self) -> Result<[i32; 2], String> {
                Ok([1280, 720])
            }
        }
        let mut w = Screen::default();
        let mut go =
            |class: &str, m: &str, args: &[Value]| dispatch(&mut w, class, m, args).unwrap();
        assert_eq!(go("Cam", "SetOverrideCamera__rr", &pair), Ok(Value::Void));
        assert_eq!(
            go("Cam", "SetOverrideCameraActivation__bf", &on),
            Ok(Value::Void)
        );
        assert_eq!(
            go("Cam", "GetCameraMatrix__", &[]),
            Ok(Value::Mat44(IDENTITY))
        );
        assert_eq!(
            go("HUD", "GetScreenSpacePosition__r", point),
            Ok(Value::Vec4([2., 4., 0.5, 0.]))
        );
        assert_eq!(go("HUD", "GetXRes__", &[]), Ok(Value::Int(1280)));
        assert_eq!(go("HUD", "GetYRes__", &[]), Ok(Value::Int(720)));
        // Wrong argument types and counts are errors, not defaults.
        let bad = [Value::Float(1.), v.clone()];
        assert!(go("Cam", "SetOverrideCamera__rr", &bad).is_err());
        let bad = [Value::Int(1), Value::Float(0.05)];
        assert!(go("Cam", "SetOverrideCameraActivation__bf", &bad).is_err());
        assert!(go("HUD", "GetScreenSpacePosition__r", &[Value::Int(1)]).is_err());
        assert!(go("HUD", "GetXRes__", &pair).is_err());
        assert_eq!(w.0, ["set 1 3", "activation true 0.05"]);
    }

    #[test]
    fn d270_pod_natives_check_their_arguments_and_ask_the_world() {
        let name = [Value::Str("made_up".into())];
        // A world without the state says so; a null string and a null Thing need none.
        assert!(call("Thing", "FindThing__Q7StringA", &name).is_err());
        assert_eq!(
            call("Thing", "FindThing__Q7StringA", &[Value::Object(None)]),
            Ok(Value::Thing(None))
        );
        assert!(call("Thing", "GetPodThing__b", &[Value::Bool(true)]).is_err());
        assert!(call("Game", "GetAvatarFromPlayerID__i", &[Value::Int(0)]).is_err());
        assert!(call("Resource", "LoadMeshByFilename__g", &[Value::Int(7)]).is_err());
        let set = "SetRenderMeshMesh__Q5ThingQ8Resource";
        let null = [Value::Thing(None), Value::Ref(7)];
        assert_eq!(call("Thing", set, &null), Ok(Value::Void));
        assert!(call("Thing", set, &[Value::Thing(Some(1)), Value::Ref(7)]).is_err());
        assert!(call("Thing", set, &[Value::Thing(None), Value::Bool(true)]).is_err());
        assert!(call("PodComputer", "GetPlanetsBackgroundGUID__", &[]).is_err());
        assert!(call("Game", "IsStartMenuOpen__", &[]).is_err());
        let photo = "IsTakingPhoto__Q5Thing";
        assert_eq!(
            call("Pod", photo, &[Value::Thing(None)]),
            Ok(Value::Bool(false))
        );
        assert!(call("Pod", photo, &[Value::Thing(Some(1))]).is_err());
        let fade = [Value::Float(0.5)];
        assert!(call("PodComputer", "SetFadePodDecorations__f", &fade).is_err());
        let use_pod = [Value::Bool(true)];
        assert!(call("PodComputer", "RequestHostUsePod__b", &use_pod).is_err());
        // No world is asked: the identities and the build constant.
        assert_eq!(
            call("FileList", "AsGUID__i", &[Value::Int(12140)]),
            Ok(Value::Int(12140))
        );
        for g in [
            Value::Int(-5),
            Value::Raw(0xffff_fffb),
            Value::Ref(0xffff_fffb),
        ] {
            assert_eq!(call("FileList", "AsInt__g", &[g]), Ok(Value::Int(-5)));
        }
        assert!(call("FileList", "AsInt__g", &[Value::Float(1.)]).is_err());
        assert!(call("FileList", "AsGUID__i", &[Value::Float(1.)]).is_err());
        assert_eq!(call("Game", "IsKioskBuild__", &[]), Ok(Value::Bool(false)));

        /// A world with two named Things, a pod and a platform.
        #[derive(Default)]
        struct Pod(Vec<String>);
        impl World for Pod {
            fn find_thing(&mut self, name: &[u8]) -> Result<Option<u32>, String> {
                Ok(match name {
                    b"made_up" => Some(4),
                    b"" => Some(9),
                    _ => None,
                })
            }
            fn pod_thing(&mut self, platform_too: bool) -> Result<Option<u32>, String> {
                Ok(platform_too.then_some(2))
            }
            fn yellowhead(&mut self, number: i32) -> Result<Option<u32>, String> {
                Ok((number == 0).then_some(11))
            }
            fn load_resource(&mut self, kind: ResourceKind, guid: u32) -> Result<u32, String> {
                self.0.push(format!("load {kind:?} {guid}"));
                Ok(guid)
            }
            fn set_render_mesh_mesh(&mut self, t: u32, mesh: Option<u32>) -> Result<(), String> {
                self.0.push(format!("mesh {t} {mesh:?}"));
                Ok(())
            }
            fn planets_background(&mut self) -> Result<u32, String> {
                Ok(DEFAULT_PLANETS_BACKGROUND)
            }
            fn state_query(&mut self, native: &str) -> Result<Value, String> {
                self.0.push(format!("query {native}"));
                Ok(Value::Bool(false))
            }
            fn engine_call(
                &mut self,
                name: &'static str,
                thing: Option<u32>,
                args: &[Value],
            ) -> Result<(), String> {
                self.0.push(format!("{name} {thing:?} {args:?}"));
                Ok(())
            }
            fn request_host_use_pod(&mut self, in_use: bool) -> Result<(), String> {
                self.0.push(format!("request {in_use}"));
                Ok(())
            }
        }
        let mut w = Pod::default();
        let mut go =
            |class: &str, m: &str, args: &[Value]| dispatch(&mut w, class, m, args).unwrap();
        let mut find = |s: &str| go("Thing", "FindThing__Q7StringA", &[Value::Str(s.into())]);
        assert_eq!(find("made_up"), Ok(Value::Thing(Some(4))));
        // The bytes are handed on as they are: another case is another name, and an
        // empty string is a name like any other.
        assert_eq!(find("Made_Up"), Ok(Value::Thing(None)));
        assert_eq!(find(""), Ok(Value::Thing(Some(9))));
        assert!(go("Thing", "FindThing__Q7StringA", &[Value::Int(1)]).is_err());
        assert_eq!(
            go("Thing", "GetPodThing__b", &[Value::Bool(true)]),
            Ok(Value::Thing(Some(2)))
        );
        assert_eq!(
            go("Thing", "GetPodThing__b", &[Value::Bool(false)]),
            Ok(Value::Thing(None))
        );
        assert!(go("Thing", "GetPodThing__b", &[Value::Int(1)]).is_err());
        let avatar = "GetAvatarFromPlayerID__i";
        assert_eq!(
            go("Game", avatar, &[Value::Int(0)]),
            Ok(Value::Thing(Some(11)))
        );
        assert_eq!(go("Game", avatar, &[Value::Int(1)]), Ok(Value::Thing(None)));
        assert_eq!(
            go("Resource", "LoadMeshByFilename__g", &[Value::Int(12140)]),
            Ok(Value::Ref(12140))
        );
        // A handle, a zero handle and a null object.
        for mesh in [Value::Ref(12140), Value::Ref(0), Value::Object(None)] {
            let args = [Value::Thing(Some(2)), mesh];
            assert_eq!(go("Thing", set, &args), Ok(Value::Void));
        }
        assert_eq!(
            go("PodComputer", "GetPlanetsBackgroundGUID__", &[]),
            Ok(Value::Int(0x1d054))
        );
        assert_eq!(go("Game", "IsStartMenuOpen__", &[]), Ok(Value::Bool(false)));
        assert_eq!(
            go("Pod", photo, &[Value::Thing(Some(11))]),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            go("PodComputer", "SetFadePodDecorations__f", &fade),
            Ok(Value::Void)
        );
        assert_eq!(
            go("PodComputer", "RequestHostUsePod__b", &use_pod),
            Ok(Value::Void)
        );
        assert!(go("PodComputer", "SetFadePodDecorations__f", &use_pod).is_err());
        assert!(go("PodComputer", "RequestHostUsePod__b", &fade).is_err());
        assert_eq!(
            w.0,
            [
                "load Mesh 12140",
                "mesh 2 Some(12140)",
                "mesh 2 None",
                "mesh 2 None",
                "query Game.IsStartMenuOpen__",
                "query Pod.IsTakingPhoto__Q5Thing",
                "SetFadePodDecorations None [Float(0.5)]",
                "request true",
            ]
        );
    }

    #[test]
    fn d273_pod_computer_natives() {
        // The stored colour's bytes are A, R, G, B from the top; the vector is
        // (R, G, B, A) / 255.
        assert_eq!(colour_vector(0xff00_0000), [0., 0., 0., 1.]);
        assert_eq!(colour_vector(0x00ff_0000), [1., 0., 0., 0.]);
        assert_eq!(colour_vector(0x0000_ff00), [0., 1., 0., 0.]);
        assert_eq!(colour_vector(0x0000_00ff), [0., 0., 1., 0.]);
        assert_eq!(colour_vector(0x3366_99cc)[0], 0x66 as f32 / 255.);
        assert_eq!(
            PlayerRecord::default().colour,
            PLACEHOLDER_PLAYER_COLOUR,
            "a record starts with the placeholder"
        );

        // Action 8: bit 0x80 newly down, its context in both masks.
        let pad = PadState {
            mask_a: 1,
            mask_b: 1,
            now: 0x80,
            ..Default::default()
        };
        assert_eq!(pad.is_just_clicked(ACTION_ENGAGE_POD), Some(true));
        let held = PadState { old: 0x80, ..pad };
        assert_eq!(
            held.is_just_clicked(8),
            Some(false),
            "down the frame before"
        );
        let other = PadState { now: 0x40, ..pad };
        assert_eq!(other.is_just_clicked(8), Some(false), "another button");
        // The context mask is 0x1001: either bit in a mode mask will do, and each mask
        // needs one.
        let pod_mode = PadState {
            mask_a: 0x1000,
            mask_b: 0x1000,
            ..pad
        };
        assert_eq!(pod_mode.is_just_clicked(8), Some(true));
        for (mask_a, mask_b) in [(0, 1), (1, 0), (2, 2), (1, 0x800)] {
            let p = PadState {
                mask_a,
                mask_b,
                ..pad
            };
            assert_eq!(p.is_just_clicked(8), Some(false), "{mask_a:#x} {mask_b:#x}");
            assert_eq!(p.shows_prompt(8), Some(mask_a & 0x1001 != 0));
        }
        // No other action was read: no answer, no made-up button.
        for action in [0, 7, 9, 98, 99, -1] {
            assert_eq!(pad.is_just_clicked(action), None, "{action}");
            assert_eq!(pad.shows_prompt(action), None, "{action}");
        }
        // A press that was used up stays out of the word until the button is released.
        let mut start = PadState::default();
        start.set_buttons(START_BUTTON | 0x80);
        start.use_up(START_BUTTON);
        assert_eq!((start.now, start.held_off), (0x80, START_BUTTON));
        start.set_buttons(START_BUTTON);
        assert_eq!(start.now, 0, "still down: still held off");
        start.set_buttons(0);
        start.set_buttons(START_BUTTON);
        assert_eq!((start.now, start.held_off), (START_BUTTON, 0));

        // A world without the state says so; a null Thing needs none.
        let null = [Value::Thing(None)];
        let some = [Value::Thing(Some(1))];
        let zone = "GetIsInPadZone__Q5Thing";
        let colour = "GetPlayerColour__Q5Thing";
        assert_eq!(call("Native", zone, &null), Ok(Value::Bool(false)));
        assert!(call("Native", zone, &some).is_err());
        assert_eq!(call("Native", colour, &null), Ok(Value::Vec4([0.; 4])));
        assert!(call("Native", colour, &some).is_err());
        assert!(call("Native", zone, &[Value::Int(1)]).is_err());
        let click = "IsJustClickedByID__iiQ6String";
        let engage = |s: Value| [Value::Int(0), Value::Int(8), s];
        assert!(call("Input", click, &engage(Value::Str("KEY".into()))).is_err());
        assert!(call("Game", "CurrentLevelIsOurPod__", &[]).is_err());
        assert!(call("Video", "PressedStartButton__", &[]).is_err());
        let texture = "SetPodTransitionTexture__g";
        assert!(call("PodComputer", texture, &[Value::Int(132243)]).is_err());

        /// Players 0, 1 and 2 with a pad each (player 1 has none), Thing 5 with a
        /// PYellowHead, Thing 6 in the pad zone.
        struct Players(Vec<PlayerRecord>, Vec<String>);
        impl World for Players {
            fn num_players(&mut self) -> Result<i32, String> {
                Ok(self.0.len() as i32)
            }
            fn player_number(&mut self, index: i32) -> Result<i32, String> {
                Ok(self.0.get(index as usize).map_or(-1, |r| r.number))
            }
            fn player(&mut self, number: i32) -> Result<Option<&mut PlayerRecord>, String> {
                Ok(self.0.iter_mut().find(|r| r.number == number))
            }
            fn in_pad_zone(&mut self, thing: u32) -> Result<bool, String> {
                Ok(thing == 6)
            }
            fn player_colour(&mut self, thing: u32) -> Result<Option<u32>, String> {
                Ok((thing == 5).then_some(0x8040_2010))
            }
            fn action_just_clicked(
                &mut self,
                player: i32,
                action: i32,
                prompt: Option<&str>,
            ) -> Result<bool, String> {
                self.1.push(format!("click {player} {action} {prompt:?}"));
                Ok(action == 8)
            }
            fn state_query(&mut self, native: &str) -> Result<Value, String> {
                self.1.push(format!("query {native}"));
                Ok(Value::Bool(true))
            }
            fn engine_call(
                &mut self,
                name: &'static str,
                thing: Option<u32>,
                args: &[Value],
            ) -> Result<(), String> {
                self.1.push(format!("{name} {thing:?} {args:?}"));
                Ok(())
            }
        }
        let with_pad = |number, now| PlayerRecord {
            number,
            pad: Some(PadState {
                mask_a: 1,
                mask_b: 1,
                now,
                ..Default::default()
            }),
            ..Default::default()
        };
        let no_pad = PlayerRecord {
            number: 1,
            ..Default::default()
        };
        let records = vec![with_pad(0, 0x80), no_pad, with_pad(2, START_BUTTON | 1)];
        let mut w = Players(records, Vec::new());
        let mut go =
            |class: &str, m: &str, args: &[Value]| dispatch(&mut w, class, m, args).unwrap();
        let thing = |t| [Value::Thing(Some(t))];
        assert_eq!(go("Native", zone, &thing(6)), Ok(Value::Bool(true)));
        assert_eq!(go("Native", zone, &thing(5)), Ok(Value::Bool(false)));
        // (R, G, B, A) of 0x80402010.
        let v = [0x40, 0x20, 0x10, 0x80].map(|b| b as f32 / 255.);
        assert_eq!(go("Native", colour, &thing(5)), Ok(Value::Vec4(v)));
        // A Thing without a PYellowHead: the zero vector.
        assert_eq!(go("Native", colour, &thing(6)), Ok(Value::Vec4([0.; 4])));
        // The string is handed on and may be null; the other arguments are checked.
        let key = Value::Str("KEY".into());
        assert_eq!(go("Input", click, &engage(key)), Ok(Value::Bool(true)));
        let args = [Value::Int(2), Value::Int(9), Value::Object(None)];
        assert_eq!(go("Input", click, &args), Ok(Value::Bool(false)));
        assert!(go("Input", click, &engage(Value::Int(1))).is_err());
        assert_eq!(
            go("Game", "CurrentLevelIsOurPod__", &[]),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            go("PodComputer", texture, &[Value::Int(132243)]),
            Ok(Value::Void)
        );
        assert!(go("PodComputer", texture, &[Value::Bool(true)]).is_err());
        // Start: the first pad with the bit, in record order, has the press used up.
        let start = "PressedStartButton__";
        assert_eq!(go("Video", start, &[]), Ok(Value::Bool(true)));
        assert_eq!(go("Video", start, &[]), Ok(Value::Bool(false)), "used up");
        assert_eq!(
            w.1,
            [
                "click 0 8 Some(\"KEY\")",
                "click 2 9 None",
                "query Game.CurrentLevelIsOurPod__",
                "SetPodTransitionTexture None [Int(132243)]",
            ]
        );
        let pad = w.0[2].pad.unwrap();
        assert_eq!((pad.now, pad.held_off), (1, START_BUTTON));
        assert_eq!(
            w.0[0].pad.unwrap().now,
            0x80,
            "the other pads are not touched"
        );
        // Only the first seven records are looked at.
        let mut many: Vec<_> = (0..7).map(|n| with_pad(n, 0)).collect();
        many.push(with_pad(7, START_BUTTON));
        let mut w = Players(many, Vec::new());
        let pressed = dispatch(&mut w, "Video", start, &[]).unwrap();
        assert_eq!(pressed, Ok(Value::Bool(false)));
        assert_eq!(w.0[7].pad.unwrap().now, START_BUTTON);
        // The table: 249 natives before these six; D274 added two.
        assert_eq!(bound().count(), 257);
    }

    #[test]
    fn d274_planet_math_natives() {
        use std::f32::consts::{FRAC_PI_2, PI, TAU};
        let f = Value::Float;
        // A matrix with a size per axis and a place: columns (2,0,0,0), (0,3,0,0),
        // (0,0,4,0), (10,20,30,1).
        let m = Value::Mat44([
            2., 0., 0., 0., 0., 3., 0., 0., 0., 0., 4., 0., 10., 20., 30., 1.,
        ]);
        let transform = |v: [f32; 4]| call("Math", "transform__mr", &[m.clone(), Value::Vec4(v)]);
        // w 0: a direction, the place stays out. w 1: a point.
        assert_eq!(
            transform([1., 1., 1., 0.]),
            Ok(Value::Vec4([2., 3., 4., 0.]))
        );
        assert_eq!(
            transform([1., 1., 1., 1.]),
            Ok(Value::Vec4([12., 23., 34., 1.]))
        );
        assert_eq!(
            transform([0., -2., 0., 2.]),
            Ok(Value::Vec4([20., 34., 60., 2.]))
        );
        // A turned matrix: a quarter turn about z takes +x to +y.
        let turn = call("Math", "rotateZ__f", &[f(FRAC_PI_2)]).unwrap();
        let Ok(Value::Vec4(v)) = call(
            "Math",
            "transform__mr",
            &[turn, Value::Vec4([1., 0., 0., 0.])],
        ) else {
            panic!()
        };
        assert!(v[0].abs() < 1e-6 && (v[1] - 1.).abs() < 1e-6 && v[2] == 0. && v[3] == 0.);
        assert!(call("Math", "transform__mr", &[m.clone(), f(1.)]).is_err());
        assert!(call("Math", "transform__mr", &[Value::Vec4([0.; 4]), m]).is_err());

        let wrap = |a: f32| {
            let Ok(Value::Float(x)) = call("Math", "ModAngleToWithinPI__f", &[f(a)]) else {
                panic!()
            };
            x
        };
        assert_eq!(wrap(0.), 0.);
        assert!((wrap(1.) - 1.).abs() < 1e-6 && (wrap(-1.) + 1.).abs() < 1e-6);
        // Just under half a turn stays; just over comes back from the other side.
        assert!((wrap(PI - 0.01) - (PI - 0.01)).abs() < 1e-5);
        assert!((wrap(PI + 0.01) - (0.01 - PI)).abs() < 1e-5);
        assert!((wrap(-PI - 0.01) - (PI - 0.01)).abs() < 1e-5);
        // Half a turn itself is the lower end.
        assert!((wrap(PI) + PI).abs() < 1e-5);
        // Whole turns go.
        assert!(wrap(3. * TAU).abs() < 1e-4);
        assert!((wrap(3. * TAU + 0.5) - 0.5).abs() < 1e-4);
        assert!((wrap(-2. * TAU - 0.5) + 0.5).abs() < 1e-4);
        assert!(call("Math", "ModAngleToWithinPI__f", &[Value::Int(1)]).is_err());
        assert_eq!(mod_angle_to_within_pi(0.25), wrap(0.25));
        // The table: 255 natives before these two.
        assert_eq!(bound().count(), 257);
    }

    #[test]
    fn d271_video_voice_over_and_flag_natives() {
        let video = "StartVideo__Q5Thinggb";
        let start = [Value::Thing(None), Value::Int(77), Value::Bool(false)];
        let line = [
            Value::Str("made_up_line".into()),
            Value::Bool(true),
            Value::Bool(true),
        ];
        let mix = [Value::Str("Made Up Mix".into()), Value::Float(0.2)];
        // A world without the state says so.
        assert!(call("Video", video, &start).is_err());
        assert!(call("Video", "StopVideo__i", &[Value::Int(3)]).is_err());
        assert!(call("Audio", "RequestVoiceOver__Q7StringAbb", &line).is_err());
        assert!(call("Audio", "IsVoiceOverPlaying__", &[]).is_err());
        assert!(call("Audio", "StopVoiceOver__", &[]).is_err());
        assert!(call("Audio", "SetMixOverlay__Q7StringAf", &mix).is_err());
        assert!(call("Audio", "RemoveMixOverlay__Q7StringAf", &mix).is_err());
        let mask = [Value::Int(3)];
        assert!(call("Profile", "SetLBP2GameProgressionFlag__i", &mask).is_err());
        assert!(call("Profile", "IsLBP2GameProgressionFlagSet__i", &mask).is_err());
        assert!(call("Profile", "HasPlayedLBP1AndUnlockedPod__", &[]).is_err());
        assert!(call("Profile", "HasPlayedLBP1AndUnlockedMoon__", &[]).is_err());
        // No world is asked: no video has a handle.
        for handle in [-1, 0, 7] {
            let args = [Value::Int(handle), Value::Bool(false)];
            assert_eq!(
                call("Video", "UpdateVideo__ib", &args),
                Ok(Value::Bool(false))
            );
        }
        assert!(call("Video", "UpdateVideo__ib", &[Value::Int(0), Value::Int(0)]).is_err());

        /// A world with a flag word and a list of the calls it was handed.
        #[derive(Default)]
        struct Media(u32, Vec<String>);
        impl World for Media {
            fn progression_flag(&mut self, mask: u32) -> Result<bool, String> {
                Ok(self.0 & mask == mask)
            }
            fn set_progression_flags(&mut self, mask: u32) -> Result<(), String> {
                self.0 |= mask;
                Ok(())
            }
            fn state_query(&mut self, native: &str) -> Result<Value, String> {
                self.1.push(format!("query {native}"));
                Ok(Value::Bool(false))
            }
            fn engine_call(
                &mut self,
                name: &'static str,
                thing: Option<u32>,
                args: &[Value],
            ) -> Result<(), String> {
                self.1.push(format!("{name} {thing:?} {}", args.len()));
                Ok(())
            }
        }
        let mut w = Media::default();
        let mut go =
            |class: &str, m: &str, args: &[Value]| dispatch(&mut w, class, m, args).unwrap();
        // The answers are the game's for "nothing started".
        assert_eq!(go("Video", video, &start), Ok(Value::Int(NO_VIDEO)));
        let on_thing = [Value::Thing(Some(4)), Value::Raw(77), Value::Bool(true)];
        assert_eq!(go("Video", video, &on_thing), Ok(Value::Int(-1)));
        let no_thing = [Value::Int(1), Value::Int(77), Value::Bool(true)];
        assert!(go("Video", video, &no_thing).is_err());
        assert!(go("Video", video, &start[..2]).is_err());
        assert_eq!(
            go("Video", "StopVideo__i", &[Value::Int(-1)]),
            Ok(Value::Void)
        );
        let request = "RequestVoiceOver__Q7StringAbb";
        assert_eq!(go("Audio", request, &line), Ok(Value::Bool(false)));
        let null_line = [Value::Object(None), Value::Bool(false), Value::Bool(false)];
        assert_eq!(go("Audio", request, &null_line), Ok(Value::Bool(false)));
        assert!(go("Audio", request, &mix).is_err());
        assert_eq!(
            go("Audio", "IsVoiceOverPlaying__", &[]),
            Ok(Value::Bool(false))
        );
        assert_eq!(go("Audio", "StopVoiceOver__", &[]), Ok(Value::Bool(true)));
        for name in ["SetMixOverlay__Q7StringAf", "RemoveMixOverlay__Q7StringAf"] {
            assert_eq!(go("Audio", name, &mix), Ok(Value::Bool(false)), "{name}");
            assert!(go("Audio", name, &line).is_err(), "{name}");
        }
        // The flag natives take a bit mask.
        let set = "SetLBP2GameProgressionFlag__i";
        let is_set = "IsLBP2GameProgressionFlagSet__i";
        assert_eq!(
            go("Profile", is_set, &[Value::Int(2)]),
            Ok(Value::Bool(false))
        );
        assert_eq!(go("Profile", set, &mask), Ok(Value::Void));
        for (m, on) in [(1, true), (2, true), (3, true), (4, false), (6, false)] {
            let answer = go("Profile", is_set, &[Value::Int(m)]);
            assert_eq!(answer, Ok(Value::Bool(on)), "mask {m}");
        }
        assert!(go("Profile", set, &[Value::Bool(true)]).is_err());
        for name in [
            "HasPlayedLBP1AndUnlockedPod__",
            "HasPlayedLBP1AndUnlockedMoon__",
        ] {
            assert_eq!(go("Profile", name, &[]), Ok(Value::Bool(false)), "{name}");
        }
        assert_eq!(
            w.1,
            [
                "StartVideo None 3",
                "StartVideo Some(4) 3",
                "StopVideo None 1",
                "RequestVoiceOver None 3",
                "RequestVoiceOver None 3",
                "query Audio.IsVoiceOverPlaying__",
                "StopVoiceOver None 0",
                "SetMixOverlay None 2",
                "RemoveMixOverlay None 2",
                "query Profile.HasPlayedLBP1AndUnlockedPod__",
                "query Profile.HasPlayedLBP1AndUnlockedMoon__",
            ]
        );
        assert_eq!(w.0, 3);
        // The table: 238 natives before these eleven; D273 added six, D274 two.
        assert_eq!(bound().count(), 257);
    }

    #[test]
    fn math() {
        let Value::Mat44(m) =
            call("Math", "translate__r", &[Value::Vec4([1., 2., 3., 0.])]).unwrap()
        else {
            panic!()
        };
        assert_eq!(&m[12..16], &[1., 2., 3., 1.]);
        let Value::Mat44(r) = call(
            "Math",
            "rotateZ__f",
            &[Value::Float(std::f32::consts::FRAC_PI_2)],
        )
        .unwrap() else {
            panic!()
        };
        // Column 0 (the image of +x) is +y.
        assert!((r[0]).abs() < 1e-6 && (r[1] - 1.).abs() < 1e-6);
        assert_eq!(
            call("Math", "length3__r", &[Value::Vec4([3., 4., 0., 9.])]),
            Ok(Value::Float(5.))
        );
        assert_eq!(
            call(
                "Math",
                "clamp__fff",
                &[Value::Float(5.), Value::Float(0.), Value::Float(1.)]
            ),
            Ok(Value::Float(1.))
        );
        assert!(call("Math", "normalize3__r", &[Value::Vec4([0.; 4])]).is_err());
        assert_eq!(
            call(
                "String",
                "Concat__Q6StringQ6String",
                &[Value::Str("a".into()), Value::Str("b".into())]
            ),
            Ok(Value::Str("ab".into()))
        );
    }

    #[test]
    fn part6_math_and_lams_keys() {
        let (f, v) = (Value::Float, |x: f32, y: f32| Value::Vec4([x, y, 0., 0.]));
        assert_eq!(
            call("Math", "lerp__frr", &[f(0.25), v(0., 4.), v(4., 8.)]),
            Ok(v(1., 5.))
        );
        assert_eq!(
            call("Math", "SmoothStep__fff", &[f(0.), f(1.), f(0.5)]),
            Ok(f(0.5))
        );
        assert_eq!(
            call("Math", "SmoothStep__fff", &[f(0.), f(1.), f(2.)]),
            Ok(f(1.))
        );
        // Far target: speed capped at max_vel 2, change at max_acc 0.5 from rest.
        assert_eq!(
            call(
                "Math",
                "Smooth3D__rrrff",
                &[v(0., 0.), v(0., 0.), v(100., 0.), f(0.5), f(2.)]
            ),
            Ok(v(0.5, 0.))
        );
        // Near target, already moving at the right speed: lands on it.
        assert_eq!(
            call(
                "Math",
                "Smooth__fffff",
                &[f(1.), f(1.), f(2.), f(10.), f(100.)]
            ),
            Ok(Value::Vec4([2., 1., 0., 0.]))
        );
        // Toolkit makeLamsKeyID vector (docs/u1-translation.md), as s32.
        let key = crate::natives::lookup("LAMS", "MakeLamsKeyID__Q6String").unwrap();
        let s = [Value::Str("A".into())];
        let got = key(
            &mut Tree,
            &Call {
                receiver: None,
                params: &s,
            },
        );
        assert_eq!(
            got,
            Ok(Value::Int(lbp_resource::translation::lams_key("A") as i32))
        );
    }

    #[test]
    fn stage1_strings_and_math() {
        let (s, i, f) = (|t: &str| Value::Str(t.into()), Value::Int, Value::Float);
        let null = || Value::Object(None);
        // Wide strings count UTF-16 units.
        assert_eq!(
            call("String", "Length__Q6String", &[s("a\u{1F600}")]),
            Ok(i(3))
        );
        assert_eq!(call("String", "Length__Q6String", &[null()]), Ok(i(0)));
        assert_eq!(call("StringA", "Length__Q7StringA", &[s("abc")]), Ok(i(3)));
        for (class, m) in [
            ("String", "IsNullOrEmpty__Q6String"),
            ("StringA", "IsNullOrEmpty__Q7StringA"),
        ] {
            assert_eq!(call(class, m, &[null()]), Ok(Value::Bool(true)));
            assert_eq!(call(class, m, &[s("")]), Ok(Value::Bool(true)));
            assert_eq!(call(class, m, &[s("x")]), Ok(Value::Bool(false)));
        }
        for (class, m) in [
            ("String", "Find__Q6StringQ6String"),
            ("StringA", "Find__Q7StringAQ7StringA"),
        ] {
            assert_eq!(call(class, m, &[s("hello"), s("ll")]), Ok(i(2)));
            assert_eq!(call(class, m, &[s("hello"), s("z")]), Ok(i(-1)));
            assert_eq!(call(class, m, &[s("hello"), null()]), Ok(i(-1)));
            assert_eq!(call(class, m, &[null(), s("z")]), Ok(i(-1)));
        }
        for (class, m) in [
            ("String", "SubStr__Q6Stringii"),
            ("StringA", "SubStr__Q7StringAii"),
        ] {
            assert_eq!(call(class, m, &[s("hello"), i(1), i(3)]), Ok(s("ell")));
            assert_eq!(call(class, m, &[s("hello"), i(3), i(99)]), Ok(s("lo")));
            assert_eq!(call(class, m, &[s("hello"), i(5), i(1)]), Ok(s("")));
            assert_eq!(call(class, m, &[null(), i(0), i(1)]), Ok(s("")));
            assert!(call(class, m, &[s("hello"), i(-1), i(1)]).is_err());
        }
        assert_eq!(
            call("String", "ToUpper__Q6String", &[s("a\u{e9}1\u{df}")]),
            Ok(s("A\u{c9}1\u{df}"))
        );
        assert_eq!(
            call("LAMS", "MakeLamsKeyID__Q6String", &[null()]),
            call("LAMS", "MakeLamsKeyID__Q6String", &[s("")])
        );
        assert_eq!(
            call("String", "StringFromStringA__Q7StringA", &[s("x")]),
            Ok(s("x"))
        );
        let Value::Mat44(m) = call("Math", "identity__", &[]).unwrap() else {
            panic!()
        };
        assert_eq!(
            (m[0], m[5], m[10], m[15], m[1], m[12]),
            (1., 1., 1., 1., 0., 0.)
        );
        assert_eq!(call("Math", "ceil__f", &[f(1.25)]), Ok(f(2.)));
        assert_eq!(call("Math", "exp__f", &[f(0.)]), Ok(f(1.)));
        assert_eq!(call("Math", "pow__ff", &[f(2.), f(10.)]), Ok(f(1024.)));
        assert_eq!(call("Math", "sign__f", &[f(0.)]), Ok(f(1.)));
        assert_eq!(call("Math", "sign__f", &[f(-0.5)]), Ok(f(-1.)));
        assert_eq!(
            call("Math", "lerp__fff", &[f(0.25), f(4.), f(8.)]),
            Ok(f(5.))
        );
        assert_eq!(
            call("Math", "Delt__fff", &[f(1.), f(5.), f(1.5)]),
            Ok(f(2.5))
        );
        assert_eq!(
            call("Math", "Delt__fff", &[f(1.), f(-5.), f(1.5)]),
            Ok(f(-0.5))
        );
        assert_eq!(
            call("Math", "Delt__fff", &[f(1.), f(2.), f(1.5)]),
            Ok(f(2.))
        );
        assert_eq!(
            call("Math", "Delt__fff", &[f(1.), f(0.5), f(1.5)]),
            Ok(f(0.5))
        );
        assert_eq!(call("Math", "BiasCosine__f", &[f(0.)]), Ok(f(0.)));
        assert_eq!(call("Math", "BiasCosine__f", &[f(1.)]), Ok(f(1.)));
        let Ok(Value::Float(half)) = call("Math", "BiasCosine__f", &[f(0.5)]) else {
            panic!()
        };
        assert!((half - 0.5).abs() < 1e-6);
    }

    #[test]
    fn unbound_natives_fault_or_default() {
        let mut strict = Bindings::new(Tree);
        assert!(strict.call_native("Gooey", "EndFrame__", &[]).is_err());
        let mut lenient = Bindings::defaulting(Tree);
        assert_eq!(
            lenient.call_native("Gooey", "EndFrame__", &[]),
            Ok(Value::Void)
        );
        // Counted as unbound either way; a bound native's error is still an error.
        assert_eq!(strict.unbound.get("Gooey.EndFrame__"), Some(&1));
        assert_eq!(lenient.unbound.get("Gooey.EndFrame__"), Some(&1));
        assert_eq!(lenient.calls.get("Gooey.EndFrame__"), Some(&1));
        assert!(lenient
            .call_native("LAMS", "Translate__i", &[Value::Int(3)])
            .is_err());
        assert!(!lenient.unbound.contains_key("LAMS.Translate__i"));
    }

    #[test]
    fn receiver_split_and_world() {
        // Member form: receiver plus the port.
        assert_eq!(
            call(
                "Thing",
                "GetSwitchActivationAnalogue__i",
                &[Value::Thing(Some(7)), Value::Int(1)]
            ),
            Ok(Value::Float(0.5))
        );
        // Stage 3: what each switch-input native reads from a wired or empty port.
        let port = |m: &str, p: i32| call("Thing", m, &[Value::Thing(Some(7)), Value::Int(p)]);
        for (m, wired, zero_ternary, empty) in [
            (
                "IsSwitchTriggered__i",
                Value::Bool(true),
                Value::Bool(true),
                Value::Bool(false),
            ),
            (
                "GetSwitchActivationBinary__i",
                Value::Bool(true),
                Value::Bool(false),
                Value::Bool(false),
            ),
            (
                "GetSwitchActivationTernary__i",
                Value::Float(-1.),
                Value::Float(0.),
                Value::Float(0.),
            ),
            (
                "GetSwitchActivationPlayer__i",
                Value::Int(-1),
                Value::Int(3),
                Value::Int(-1),
            ),
        ] {
            assert_eq!(port(m, 1), Ok(wired), "{m}");
            assert_eq!(port(m, 2), Ok(zero_ternary), "{m}");
            assert_eq!(port(m, 3), Ok(empty.clone()), "{m}");
            assert_eq!(port(m, -1), Ok(empty), "{m}");
        }
        assert_eq!(
            port("GetSwitchActivationAnalogue__i", 9),
            Ok(Value::Float(0.))
        );
        let input = |t, p| call("Thing", "GetInput__Q5Thingi", &[t, Value::Int(p)]);
        assert_eq!(input(Value::Thing(Some(7)), 0), Ok(Value::Thing(Some(8))));
        assert_eq!(input(Value::Thing(Some(7)), 1), Ok(Value::Thing(None)));
        assert_eq!(input(Value::Thing(None), 0), Ok(Value::Thing(None)));
        for (class, m, args) in [
            ("Thing", "GetOutput__Q5Thingii", vec![Value::Int(0); 2]),
            ("Native", "GetInThing__Q5Thingi", vec![Value::Int(0)]),
        ] {
            let mut all = vec![Value::Thing(None)];
            all.extend(args);
            assert_eq!(call(class, m, &all), Ok(Value::Thing(None)), "{m}");
        }
        assert_eq!(
            call("BaseWorld", "IsScoreBoard__Q5Thing", &[Value::Thing(None)]),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            call("Thing", "GetThingUID__", &[Value::Thing(Some(7))]),
            Ok(Value::Int(107))
        );
        assert!(call("Thing", "GetThingUID__", &[]).is_err());
        // Stage 4: the position natives check the Thing for null themselves, so a null
        // Thing never reaches the world (which here has no transforms at all).
        let m = Value::Mat44(IDENTITY);
        for name in [
            "GetWorldPosition__",
            "GetLocalPosition__",
            "GetWorldPositionForSensor__",
        ] {
            assert_eq!(call("Thing", name, &[Value::Thing(None)]), Ok(m.clone()));
            assert!(call("Thing", name, &[Value::Thing(Some(7))]).is_err());
        }
        for name in ["SetLocalPosition__m", "SetWorldPosition__m"] {
            let args = [Value::Thing(None), m.clone()];
            assert_eq!(call("Thing", name, &args), Ok(Value::Void), "{name}");
            let args = [Value::Thing(None), Value::Vec4([0.; 4])];
            assert!(call("Thing", name, &args).is_err(), "{name}");
        }
        let null = [Value::Thing(None)];
        assert_eq!(
            call("Thing", "GetMicroChipProxyFromThing__Q5Thing", &null),
            Ok(Value::Thing(None))
        );
        assert_eq!(
            call("Game", "IsOnSequencer__Q5Thing", &null),
            Ok(Value::Bool(false))
        );
        // IsInactive does not: a null Thing is an error.
        assert!(call("Thing", "IsInactive__", &null).is_err());
        assert_eq!(
            call("Thing", "GetParent__Q5Thing", &[Value::Thing(Some(0))]),
            Ok(Value::Thing(None))
        );
        assert!(call("Thing", "GetThingUID__", &[Value::Thing(None)]).is_err());
        // Arity outside n or n + 1.
        assert!(call("Math", "floor__f", &[]).is_err());
        // A world that lacks a feature says so.
        assert!(call("LAMS", "Translate__i", &[Value::Int(3)])
            .unwrap_err()
            .contains("translated text"));
        assert!(dispatch(&mut Tree, "Gooey", "EndFrame__", &[]).is_none());
    }

    #[test]
    fn test_natives_report_checks() {
        #[derive(Default)]
        struct Log(Vec<(bool, String, i32)>);
        impl World for Log {
            fn test_check(&mut self, passed: bool, message: &str, line: i32) {
                self.0.push((passed, message.into(), line));
            }
        }
        let mut log = Log::default();
        let msg = || Value::Str("m".into());
        let mut run =
            |m: &str, args: Vec<Value>| dispatch(&mut log, "Test", m, &args).expect("bound");
        let v = |a: [f32; 4]| Value::Vec4(a);
        run(
            "CheckEqual__iiQ7StringAi",
            vec![Value::Int(2), Value::Int(2), msg(), Value::Int(9)],
        )
        .unwrap();
        run(
            "CheckEqual__ppQ7StringAi",
            vec![
                v([1., 2., 3., 4.]),
                v([1., 2., 0., 0.]),
                msg(),
                Value::Int(9),
            ],
        )
        .unwrap();
        run(
            "CheckEqual__rrQ7StringAi",
            vec![
                v([1., 2., 3., 4.]),
                v([1., 2., 0., 0.]),
                msg(),
                Value::Int(9),
            ],
        )
        .unwrap();
        run(
            "CheckSimilar__fffQ7StringAi",
            vec![
                Value::Float(1.),
                Value::Float(1.05),
                Value::Float(0.1),
                msg(),
                Value::Int(3),
            ],
        )
        .unwrap();
        // Mixed types are a fault, not a failed check.
        assert!(run(
            "CheckEqual__iiQ7StringAi",
            vec![Value::Int(1), Value::Float(1.), msg(), Value::Int(1)]
        )
        .is_err());
        let passed: Vec<_> = log.0.iter().map(|(p, _, l)| (*p, *l)).collect();
        assert_eq!(passed, [(true, 9), (true, 9), (false, 9), (true, 3)]);
    }
}
