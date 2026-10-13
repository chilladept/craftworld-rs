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
#[derive(Debug, Clone, PartialEq, Default)]
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
}

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
}

impl PadState {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Texture,
    Plan,
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
    /// Tint of a Thing's render mesh in the editor colour (`SetRenderMeshEditorColourTint`,
    /// called next to `SetShapeEditorColour` with a colour vector).
    fn set_render_mesh_tint(&mut self, _thing: u32, _colour: [f32; 4]) -> Result<(), String> {
        Err(not_available("render mesh tints"))
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
    /// [`RECORDED_ONLY`].
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
    /// `Profile.IsLBP2GameProgressionFlagSet__i` (PlanetsWorld asks for flag 2).
    fn progression_flag(&mut self, _flag: i32) -> Result<bool, String> {
        Err(not_available("the profile"))
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

/// A [`RECORDED_ONLY`] native: hands the call to [`World::engine_call`].
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
    ("Thing", "SetShapeEditorColour__Q5Thingr", |w, c| {
        w.set_shape_editor_colour(subject(c)?, vec4(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    (
        "Thing",
        "SetRenderMeshEditorColourTint__Q5Thingr",
        |w, c| {
            w.set_render_mesh_tint(subject(c)?, vec4(param(c, 1)?)?)?;
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
    ("Profile", "IsLBP2GameProgressionFlagSet__i", |w, c| {
        Ok(Value::Bool(w.progression_flag(int(param(c, 0)?)?)?))
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
    // FOUND (0x642cd4 -> 0x754aac). Declared static `void HideRenderMesh(Thing)`: nothing
    // for a null Thing; else a visibility call on its PRenderMesh. Display state.
    ("Thing", "HideRenderMesh__Q5Thing", |w, c| {
        recorded(w, "HideRenderMesh", thing(param(c, 0)?)?, &[])
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
    // `void SetRenderMeshEditorColour(Thing, v4)`: nothing for a null Thing; else the
    // colour of its PRenderMesh. Display state.
    ("Thing", "SetRenderMeshEditorColour__Q5Thingr", |w, c| {
        let t = thing(param(c, 0)?)?;
        vec4(param(c, 1)?)?;
        recorded(w, "SetRenderMeshEditorColour", t, &c.params[1..])
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
    // ---- D263: recorded only ([`RECORDED_ONLY`]) ----
    // The messaging natives go to one of the game's five message managers, which does
    // nothing when the Thing already has a message of that type; the world keeps each
    // request with its arguments.
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
pub const STATE_QUERIES: [(&str, &str); 24] = [
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
];

/// Bound natives that return nothing and whose whole effect here is a record in the world
/// ([`World::engine_call`], or a counter for the audio ones): the engine would draw, play
/// or send something that nothing models yet. Reports count calls to them apart from the
/// other bound natives.
pub const RECORDED_ONLY: [(&str, &str); 29] = [
    ("Audio", "PlaySound__Q5ThingQ7StringA"),
    ("Audio", "SetInteractiveMusic__ggffiQ5Thing"),
    ("Audio", "SetInteractiveMusicStereoPairVolume__giff"),
    ("Audio", "StopInteractiveMusic__"),
    ("World", "SetPieChartMultiplier__i"),
    ("Thing", "UpdateScale__"),
    ("Thing", "HideRenderMesh__Q5Thing"),
    ("Thing", "SetRenderMeshEditorTexture__Q5ThingQ8Resource"),
    ("Thing", "SetRenderMeshEditorColour__Q5Thingr"),
    ("Thing", "RemoveFromAnimatingDivergentList__"),
    ("Trigger_Speech_Bubble", "NotifyNpcsOfMouthEnd__"),
    ("NoJoinMarker", "SetGameNoJoin__"),
    ("Game", "GotoScoreboard__b"),
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessage__Q6StringQ6StringQ5Thingbib",
    ),
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageNoPrompt__Q6StringQ6StringQ5Thingbib",
    ),
    (
        "GameMessagingManager",
        "Messaging_NewTriggerMessageSubtitles__Q6StringQ5Thingbb",
    ),
    (
        "GameMessagingManager",
        "Messaging_NewLevelLinkPrompt__Q5Thing",
    ),
    (
        "GameMessagingManager",
        "Messaging_NewWormHolePrompt__Q5Thing",
    ),
    ("GameMessagingManager", "Messaging_NewGameEvent__iiQ5Thingb"),
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
        assert!(call("Thing", "HideRenderMesh__Q5Thing", &[Value::Thing(None)]).is_err());
    }

    #[test]
    fn d263_pad_rules_constants_and_a_world_without_the_state() {
        // The pad rule: every bit of the mask, the mode in the masks.
        let pad = PadState {
            mask_a: 1,
            mask_b: 1,
            now: 0b0110,
            old: 0b0100,
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
