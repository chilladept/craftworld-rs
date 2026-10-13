//! L5+ part readers. Each reader follows the field order of the named Toolkit
//! class at d6a2ef80b5a09999f08f23a803b516096055e5c9 (`structs/things/parts/P*.java`
//! and the component structs they call, serialize methods), CONFIRMED_FROM_COMMUNITY_CODE,
//! specialised to the LBP2 band 0x341 <= head <= 0x3f8 with subVersion 0 and no
//! Vita/DOUBLE11 branch fields. Only gates that can differ inside that band are written
//! as conditions; any part not listed in `SUPPORTED` stops the walk with
//! `Stop::UnsupportedPart` because parts carry no length prefix.
//!
//! Field lists (v = head & 0xffff; "ref" = reference, "res" = resource with flags word,
//! "desc" = resource without flags word):
//! - BODY (PBody): posVel v3, angVel f32, frozen i32, editingPlayer ref Thing. Kept as
//!   `PartData::body` (velocities and `frozen`).
//! - POS (PPos): thingOfWhichIAmABone ref Thing, animHash i32, worldPosition m44
//!   (localPosition is only serialized below 0x341).
//! - RENDER_MESH (PRenderMesh): mesh res, boneThings thingarray, anim res, animPos,
//!   animSpeed f32, animLoop bool, loopStart, loopEnd f32, editorColor i32, castShadows i8,
//!   RTTEnable bool, visibilityFlags i8, poppetRenderScale f32, parentDistanceFront/Side
//!   f32 only below 0x34d.
//! - SHAPE (PShape + Polygon + ContactCache/Contact): i32 vertex count, bool requiresZ, v3
//!   or v2 per vertex, loops intvector; material res, oldMaterial res, thickness, massDepth
//!   f32, color (v4 up to 0x389, else i32), brightness, bevelSize f32, COM m44 (>= 0x38e),
//!   behavior i32, colorOff (v4 below 0x38a, else i32), brightnessOff f32, lethalType
//!   (s32 up to 0x345, else u16), soundEnumOverride i32, playerNumberColor (i32 up to
//!   0x367, else i8), flags (i8 up to 0x345, else i16), contacts array of {ref PShape,
//!   i8}, contactsSorted bool, cacheDirtyButRecomputed bool, stickiness/grabbability/
//!   grabFilter i8 (>= 0x3bd), colorOpacity/colorOffOpacity i8 (>= 0x3c1), canCollect bool
//!   (>= 0x3e2).
//! - GENERATED_MESH (PGeneratedMesh): gfxMaterial res, bevel res, uvOffset v4, planGUID
//!   guid, play-mode visibility bool, textureAnimationSpeed/Off f32.
//! - GROUP (PGroup): creator NetworkPlayerID (36 raw bytes at >= 0x234), plan desc,
//!   emitter ref Thing, lifetime i32, aliveFrames i32, flags u8.
//! - REF (PRef): plan desc, oldLifetime i32, oldAliveFrames i32.
//! - EFFECTOR (PEffector): posVel v3, angVel, viscosity, density f32, gravity v3, pushBack
//!   bool, swimmable bool, viscosityCheap, modScale f32.
//! - SCRIPT_NAME (PScriptName): s32 length then raw bytes.
//! - SPRITE_LIGHT (PSpriteLight): color v4, colorOff v4, multiplier, multiplierOff,
//!   glowRadius, farDist, sourceSize f32, falloffTexture res, lookAt ref Thing, spotlight
//!   bool, (enableFogShadows, enableFog bool below 0x337), fogAmount, onSpeed, offSpeed,
//!   flickerProb, flickerAmount f32, behavior i32.
//! - TRIGGER (PTrigger): triggerType i8, inThings thingarray, radiusMultiplier f32,
//!   allZLayers bool, hysteresisMultiplier f32, enabled bool, zOffset f32.
//! - STICKERS (PStickers): decals array(Decal), i32 count of costume decal arrays each
//!   array(Decal), paintControl array of 4 bytes (v <= 0x3ba), eyetoyData array(EyetoyData).
//!   Decal: texture res, u, v, xvecu, xvecv, yvecu, yvecv f32, color i16, type i8,
//!   metadataIndex i16, numMetadata i16 (v <= 0x3ba), placedBy i16, playModeFrame i32,
//!   scorchMark bool, plan desc (>= 0x34c; a guid below). EyetoyData: frame res, alphaMask
//!   res, colorCorrection m44, ColorCorrection 6 f32, outline res (> 0x39f).
//! - DECORATIONS (PDecorations): array(Decoration): renderMesh ref PRenderMesh, offset m44,
//!   parentBone i32, parentTriVert i32, baryU, baryV, angle f32, onCostumePiece s32,
//!   earthDecoration s32, scale f32, placedBy i16, reversed bool, playModeFrame i32,
//!   planGUID guid.
//! - LEVEL_SETTINGS (PLevelSettings extends LevelSettings): LevelSettings fields (sun
//!   position v3, scale f32, sun/ambient colour v4, multiplier, exposure f32, fog colour v4,
//!   fogNear, fogFar f32, rim colour v4, rim colour 2 v4, then 6 f32 baked/DOF values, dofFar
//!   f32, 3 zEffect f32; two extra f32 only for 0x324 < v < 0x331, below this band), then
//!   presets arraylist(LevelSettings), backdropAmbience str, backdropMesh res.
//! - CAMERA_TWEAK (PCameraTweak): (pitchAngle v3, targetBox v4 below 0x37e), triggerBox v4,
//!   (zoomDistance f32 below 0x37e), positionFactor f32, photoBoothTimerLength i32,
//!   cameraType s32, disableZoomMode, requireAll, motionControllerZone bool, behavior i32,
//!   (u8 below 0x37e), cutSceneTransitionType i8, cutSceneHoldTime i32, cutSceneSkippable
//!   bool, cutSceneTimeSinceUsed i8, cutSceneTransitionTime i32, (bool below 0x35a),
//!   cutSceneColour i32 (> 0x359), cutSceneMovieMode bool, DOF, fog, FOV, shake f32,
//!   fadeAudio, oldStyleCameraZone bool, trackPlayer bool (> 0x369), sendsSignalOnCancelled
//!   bool (> 0x395), wasActiveLastFrame bool (> 0x396), nodes array(CameraNode: v4, v3, f32,
//!   bool) (> 0x37d).
//! - JOINT (PJoint): a, b ref Thing, aContact, bContact v3, length, angle, offsetTime f32,
//!   invertAngle bool, settings res, boneIdx intarray, boneLengths v4, type i32, strength
//!   f32, stiff bool, slideDir v3, animationPattern i32, ten f32 (range, time, phase, speed,
//!   pause, a/b angle offsets, modStartFrames, modDeltaFrames, modScale), renderScale f32,
//!   jointSoundEnum i32, tweakTargetMax/MinLength f32, currentlyEditing bool,
//!   hideInPlayMode bool, behaviour i32. Drive fields kept as `Joint::params`.
//! - CHECKPOINT (PCheckpoint): activeFlags bool (subVersion <= 0x101), activationFrame i32,
//!   spawnsLeft s32, maxSpawnsLeft s32, instanceInfiniteSpawns bool, spawningList
//!   thingarray, spawningDelay i32, lifeMultiplier i32, teamFilter i32.
//! - PHYSICS_TWEAK (PPhysicsTweak): tweakGravity, tweakBuoyancy f32, tweakDampening,
//!   input, middleVel v3, velRange, accelStrength, decelStrength f32, localSpace bool,
//!   configuration i32, (ref Thing below 0x327), hideInPlayMode bool, colorIndex i32, name
//!   wstr, teamFilter u8, behavior i32, allowInOut, allowUpDown bool, minRange, maxRange
//!   f32, followKey bool, angleRange f32 (> 0x381), flee bool, maximumMass f32, canPush bool,
//!   zBehavior s32, lastKnownActivation f32, waitingToMove bool (> 0x36a); RecordingPlayer
//!   when configuration == 0xd and v > 0x3b8 (recording res, playHead f32, (u8 below
//!   0x3c4), type i8, dir i8, prevDesiredPosition v3, prevDesiredPosSet bool,
//!   startOrientation m44, speed f32, pathIsAbsolute bool (> 0x3c4)); attractor block when
//!   configuration == 0xe and v > 0x3e3 (f32, i32, f32, f32, i32, f32, f32, 4 u8).
//! - SWITCH_KEY (PSwitchKey): colorIndex s32, name wstr, hideInPlayMode bool (v < 0x3ed)
//!   or a u8 flag byte (v >= 0x3ed).
//! - GAMEPLAY_DATA (PGameplayData): fluffCost s32, eggLink ref EggLink, keyLink ref SlotID,
//!   gameplayType i32. EggLink: plan res, shareable bool, hasPainting bool (> 0x3e0),
//!   painting desc when set. SlotID: slotType i32, slotNumber u32.
//! - SWITCH (PSwitch + SwitchOutput/SwitchSignal/SwitchTarget; inverted, the outputs'
//!   signals and targets, type, manualActivation and behavior are kept as
//!   `PartData::switch`, rscript-plan stage 3): inverted bool, radius
//!   f32, minRadius f32 (>= 0x382), colorIndex s32, name wstr, crappyOldLbp1Switch bool
//!   (>= 0x38f), behaviorOld s32, outputs array of ref SwitchOutput {SwitchSignal
//!   (activation f32, ternary s32, player i32), targets array {ref Thing, port i32},
//!   userDefinedName wstr}, stickerPlan desc (v < 0x398), hideInPlayMode bool, type s32,
//!   referenceThing ref Thing, manualActivation SwitchSignal, stickerPlan desc when
//!   v >= 0x398 and type == STICKER (5), platformVisualFactor f32 (v < 0x368),
//!   activationHoldTime s32, requireAll bool, angleRange f32, includeTouching s32,
//!   stickerPlan desc when v >= 0x398, type == MICROCHIP (36) and includeTouching == 1,
//!   bulletsRequired s32, bulletsDetected s32, bulletPlayerNumber i32 (v < 0x398),
//!   bulletRefreshTime i32, resetWhenFull bool, logicType i32 (v < 0x398), updateFrame
//!   i32 (v < 0x369), inputList thingarray, includeRigidConnectors bool, timerCount f32,
//!   teamFilter i32, behavior i32, six random* i32, (randomPhaseOn u8, randomPhaseTime
//!   s32 below 0x3ad), retardedOldJoint bool, keySensorMode i32, userDefinedColour i32 +
//!   wiresVisible bool (> 0x34c), bulletTypes i8 (> 0x34f), detectUnspawnedPlayers bool
//!   (> 0x390), playSwitchAudio bool
//!   (> 0x3a4), playerMode i8 (> 0x3ec), DATA_SAMPLER (65) block when v > 0x3ee
//!   (labelIndex i32, NetworkOnlineID 20 bytes, labelName wstr, floatarray, bytearray).
//! - SCRIPT (PScript -> ScriptInstance): script res, bool hasLayout, ref InstanceLayout
//!   {arraylist FieldLayoutDetails, instanceSize i32}, reflectDivergent bool, then one
//!   value per layout field in order, skipping fields with the DIVERGENT modifier (bit
//!   12) unless reflectDivergent: BOOL bool, CHAR i16, S32 i32, F32 f32, V4 v4, M44 m44,
//!   OBJECT_REF ScriptObjectUID, SAFE_PTR ref Thing; other machine types stop.
//!   FieldLayoutDetails: name str, then for v >= 0x3d9 modifiers i16, machineType u8,
//!   fishType u8, dimensionCount i8, arrayBaseMachineType u8; below 0x3d9 modifiers i32,
//!   machineType i32, fishType i32, dimensionCount i8, arrayBaseMachineType i32; then
//!   instanceOffset i32. ScriptObjectUID: type i32 (0 null), tag i32 in the shared
//!   reference table (0 null, seen = back-reference), then by type: ARRAY_BOOL i32 count +
//!   bools, ARRAY_S32 intvector, ARRAY_F32 floatarray, ARRAY_VECTOR4 vectorarray,
//!   ARRAY_SAFE_PTR thingarray, ARRAY_OBJECT_REF array of ScriptObjectUID, RESOURCE (i32
//!   type, res when nonzero), INSTANCE ScriptInstance, STRINGW wstr, STRINGA str,
//!   AUDIOHANDLE nothing; other types stop.
//!   Kept as `PartData::script` (Rt, docs/level-scripts.md): resource, field names, machine
//!   types and values; object references keep their kind, a level-wide object number
//!   (first-occurrence order, shared by back-references) and, on first occurrence, the
//!   contents (`ScriptObject`).
//! - MATERIAL_TWEAK (PMaterialTweak): hideInPlayMode bool, restitution f32, frictionScale
//!   f32, grabbability i8, grabFilter i8 (> 0x3bc), stickiness i8, noAutoDestruct bool,
//!   disablePhysicsAudio bool, hasShadow bool (> 0x3ec); (two u8 below 0x343).
//! - MATERIAL_OVERRIDE (PMaterialOverride, v >= 0x360 only): array(RegionOverride: region
//!   i32, materialPlan desc, material res, uvScale v3), mesh res. Below 0x360 stops.
//! - SWITCH_INPUT (PSwitchInput): dataSource ref Thing, updateType i32, lethalType i32,
//!   includeRigidConnectors bool, lethalActivationFrame i32, lethalInverted bool,
//!   hideInPlayMode bool, disableLethalAudio bool (> 0x38f), ref NpcBehavior (a new
//!   non-null body stops: not implemented), sackbotObjectColorIndex s32, behavior i32,
//!   effectDestroy i32, playerMode i8 (> 0x3ec).
//! - MICROCHIP (PMicrochip + CompactComponent): circuitBoardThing ref Thing,
//!   hideInPlayMode bool, wiresVisible bool, lastTouched s32, offset v4, then (>= 0x34d)
//!   name wstr, components array {ref Thing, x, y, angle, scaleX, scaleY f32, flipped
//!   bool}, circuitBoardSizeX/Y f32.
//! - ENEMY (PEnemy): partType s32, radius f32, snapVertex i32, centerOffset v3, animThing
//!   ref Thing, animSpeed f32, sourcePlayerNumber i32, newWalkConstraintMass bool,
//!   smokeColor i32, smokeBrightness f32 (>= 0x39a).
//! - EMITTER (PEmitter + EmittedObjectSource): posVel v3 (v < 0x368), angVel f32,
//!   frequency, phase, lifetime i32, recycleEmittedObjects bool, plan res, maxEmitted i32,
//!   maxEmittedAtOnce i32, speedScaleStartFrame, speedScaleDeltaFrames f32, speedScale
//!   SwitchSignal, worldRotationForEditorEmitters f32 (>= 0x38e), emitScale, linearVel
//!   f32, currentEmitted i32, emitFlip bool, parentRelativeOffset v4, parentRelative
//!   Rotation, worldZ, zOffset, emitFrontZ, emitBackZ f32, hideInPlayMode bool, behavior
//!   i32, effectCreate/Destroy (i8 pair >= 0x38d, else i32 pair), ignoreParentsVelocity
//!   bool, ref EmittedObjectSource {thingarray, plan res}, editorEmitter bool (>= 0x361),
//!   isLimboFlippedForGunEmitter bool (>= 0x38d), zOffset f32 (>= 0x3ae).
//! - AUDIO_WORLD (PAudioWorld): soundName str, initialVolume, initialPitch,
//!   initialParam1, maxFalloff, impactTolerance f32, playMode i32, paramAffectVol/Pitch/
//!   Param bool, isLocal bool, hideInPlayMode bool, behavior i32, soundNames guid and
//!   meshColor i32 (>= 0x380).
//! - SEQUENCER (PSequencer): tempo, swing, echoFeedback, echoTime, echoMix f32,
//!   reverbSettings i32 (> 0x370), loop bool (> 0x36e), startPoint f32, numChannels i32,
//!   six volume f32, playHead f32 + isPlaying bool (> 0x369), musicSequencer bool
//!   (> 0x36c), behavior i32 (> 0x371), triggerPlayer i32 + previewThing ref Thing
//!   (> 0x3a0).
//! - WORLD (PWorld): things arraylist(ref Thing), maxVel, maxAVel f32, frame s32, simFrame
//!   s32, frameLevelStarted i32 (>= 0x377), thingUIDCounter i32, then the tail read by
//!   `world_tail`: selections array of ref EditorSelection {name str, thingarray},
//!   backdrop and backdropNew ref Thing, backdropTimer f32, lbp2NightDaySwapped s32
//!   (>= 0x3a3), isPaused bool, lighting/colourCorrection/fog/fogTint/darkness f32,
//!   complete/collect/ace rewards arrays of EggLink, areRewardsShareable bool,
//!   scoreboardLevelLinkSlot SlotID (>= 0x35e), complete/collect/ace unlocks arrays of
//!   KeyLink (SlotID), deathCount i32 (< 0x36e), maxNumPlayers s32, dissolvingThings and
//!   oldDissolvingThings thingarray, isTutorialLevel bool (< 0x345), everSpawned bool,
//!   spawnFailureCount s32, target/from GlobalSettings (5 f32 each), blend factor f32,
//!   hasLevelLightingBeenSetup bool, globalSettingsThingUID s32, CameraSettings 4 f32
//!   (>= 0x370), water level/wave/gravity triples (9 f32), floatarray, two intarrays,
//!   waterTint f32 + v4 + f32, murkiness and bits triples (6 f32), drain-sound bools
//!   (>= 0x34e), currWavePos f32, gameMode i32, nextSackbotPlayerNumber s32,
//!   CutsceneCameraManager (state i32, ref Thing, s32, s32, f32, eight legacy fields below
//!   0x36e, bool from 0x3a0), GlobalAudioSettings (reverb s32, f32 below 0x347, 4 f32,
//!   2 f32 from 0x342, 4 f32 from 0x347), backdropPlan and backdropNewPlan desc, subLevel bool
//!   (>= 0x352), scoreLocked bool (>= 0x38a), debugTimeInLevel s32 (>= 0x38b),
//!   useEvenNewerCheckpointCode bool (>= 0x3ac), moveCursors array of MoveCursor
//!   (>= 0x3bd), min/max players u8 + moveRecommended bool (>= 0x3d0),
//!   fixInvalidInOutMoverContacts bool (>= 0x3dd), continueMusic bool (>= 0x3f1).
use crate::serial::{Descriptor, Reader, Result, Tag};
use crate::thing::{thing_array, thing_ref, Things};
use serde::Serialize;

const SUPPORTED: &[&str] = &[
    "BODY",
    "JOINT",
    "WORLD",
    "RENDER_MESH",
    "POS",
    "TRIGGER",
    "GENERATED_MESH",
    "LEVEL_SETTINGS",
    "SPRITE_LIGHT",
    "SCRIPT_NAME",
    "CHECKPOINT",
    "STICKERS",
    "DECORATIONS",
    "SHAPE",
    "EFFECTOR",
    "REF",
    "METADATA",
    "CAMERA_TWEAK",
    "SWITCH_KEY",
    "GAMEPLAY_DATA",
    "GROUP",
    "PHYSICS_TWEAK",
    "SWITCH",
    "SCRIPT",
    "MATERIAL_TWEAK",
    "MATERIAL_OVERRIDE",
    "SWITCH_INPUT",
    "MICROCHIP",
    "ENEMY",
    "EMITTER",
    "AUDIO_WORLD",
    "SEQUENCER",
    "CREATURE",
    "CONTROLINATOR",
    "COSTUME",
    "TAG_SYNCHRONIZER",
    "NPC",
    "WORMHOLE",
    "INSTRUMENT",
];

pub fn supported(name: &str) -> bool {
    SUPPORTED.contains(&name)
}

/// D119: parts audited for plan Things at heads 0x335..0x340 (Toolkit serialize gates
/// in that range: PPos localPosition m44 below 0x341; PGroup pre-0x341 bool fields;
/// PSpriteLight two fog bools below 0x337 (already in its reader); PBody, PRenderMesh,
/// LevelSettings/PLevelSettings, PMetadata and PScriptName have no gate between 0x335
/// and 0x341). Every other part stops below 0x341.
const PRE_BAND_SUPPORTED: &[&str] = &[
    "BODY",
    "POS",
    "RENDER_MESH",
    "GROUP",
    "LEVEL_SETTINGS",
    "METADATA",
    "SCRIPT_NAME",
    "SPRITE_LIGHT",
];

/// `supported` for the band, the audited short list below 0x341.
pub fn supported_at(name: &str, version: u32) -> bool {
    if version < 0x341 {
        PRE_BAND_SUPPORTED.contains(&name)
    } else {
        supported(name)
    }
}

pub fn supported_parts() -> &'static [&'static str] {
    SUPPORTED
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PartData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<Pos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_mesh: Option<RenderMesh>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_mesh: Option<GeneratedMesh>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Group>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_ref: Option<Descriptor>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub decorations: Vec<Decoration>,
    /// D152: STICKERS decals (main array plus costume arrays), counted.
    pub sticker_decals: usize,
    /// D160: main STICKERS decal array with every serialized field (costume arrays are
    /// counted in `sticker_decals` only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stickers: Vec<Decal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world: Option<World>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joint: Option<Joint>,
    /// PScriptName (raw byte count, name length up to the first NUL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_name: Option<(usize, usize)>,
    /// PLevelSettings: own LevelSettings fields (D121; presets are read through) and the
    /// backdrop mesh tail (D119).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_settings: Option<LevelSettings>,
    /// P2l: the stored preset LevelSettings in order (at most `MAX_KEPT_PRESETS` kept; the
    /// rest are read through). Field order as the own settings (Toolkit PLevelSettings).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub level_settings_presets: Vec<LevelSettings>,
    /// PScript: script resource and instance field values (Rt). A part shared by
    /// back-reference is not repeated here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptPart>,
    /// PTrigger as stored (rscript-plan stage 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Trigger>,
    /// PSwitch as stored, the fields the script world uses (rscript-plan stage 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub switch: Option<Switch>,
}

/// SwitchSignal as stored: activation f32, ternary s32, player i32 (-1 = no player).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct SwitchSignal {
    pub activation: f32,
    pub ternary: i32,
    pub player: i32,
}

/// One SwitchOutput of a PSwitch: its signal when the level was saved and the inputs it
/// is wired to, each a (Thing, port) pair. A target's Thing can be null.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SwitchOutput {
    pub signal: SwitchSignal,
    pub targets: Vec<(Option<usize>, i32)>,
}

/// PSwitch as stored, the fields kept (the rest is read through; the module comment has
/// the full serialized order). `outputs` keeps the stored order; an entry is `None` for a
/// null reference or a back-reference to an output already read.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Switch {
    pub inverted: bool,
    pub outputs: Vec<Option<SwitchOutput>>,
    pub switch_type: i32,
    pub manual_activation: SwitchSignal,
    pub behavior: i32,
}

/// PTrigger as stored, in serialized order. `in_things` are the Things inside the
/// trigger when the level was saved.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Trigger {
    pub trigger_type: i8,
    pub in_things: Vec<Option<usize>>,
    pub radius_multiplier: f32,
    pub all_z_layers: bool,
    pub hysteresis_multiplier: f32,
    pub enabled: bool,
    pub z_offset: f32,
}

/// PScript / ScriptInstance as stored: the script resource and, when the instance has a
/// layout, one entry per serialized layout field in layout order.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ScriptPart {
    pub script: Option<Descriptor>,
    pub fields: Vec<ScriptField>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScriptField {
    pub name: String,
    /// Toolkit MachineType byte (1 bool .. 0xb object reference).
    pub machine_type: u8,
    pub value: ScriptValue,
}

/// One InstanceLayout field: name, machine type, DIVERGENT modifier.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutField {
    pub name: String,
    pub machine_type: u8,
    pub divergent: bool,
}

/// Stored value of a script instance field.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ScriptValue {
    Bool(bool),
    Char(i16),
    Int(i32),
    Float(f32),
    V4([f32; 4]),
    M44([f32; 16]),
    /// Thing index in the level's Thing list; None = null.
    Thing(Option<usize>),
    /// ScriptObjectUID: `kind` 0 = null. `id` numbers the level's script objects in
    /// first-occurrence order (the shared reference table), so a back-reference carries
    /// the same `id` as the first occurrence; only the first occurrence has a `body`.
    Object {
        kind: u32,
        id: Option<usize>,
        body: Option<Box<ScriptObject>>,
    },
    /// DIVERGENT field the instance does not serialize.
    NotStored,
}

/// Contents of a script object (ScriptObjectUID body), by object type.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ScriptObject {
    /// ARRAY_BOOL (1).
    Bools(Vec<bool>),
    /// ARRAY_S32 (3).
    Ints(Vec<i32>),
    /// ARRAY_F32 (4).
    Floats(Vec<f32>),
    /// ARRAY_VECTOR4 (5).
    Vectors(Vec<[f32; 4]>),
    /// ARRAY_SAFE_PTR (10): Thing indices, None = null.
    Things(Vec<Option<usize>>),
    /// ARRAY_OBJECT_REF (11): each element an `Object` (or null) value.
    Objects(Vec<ScriptValue>),
    /// RESOURCE (12): resource type and descriptor (None when the type is 0).
    Resource {
        kind: u32,
        resource: Option<Descriptor>,
    },
    /// INSTANCE (13): a nested script instance.
    Instance(ScriptPart),
    /// STRINGW (14) / STRINGA (16).
    Text(String),
    /// AUDIOHANDLE (15): nothing stored.
    AudioHandle,
}

/// D121: LevelSettings lighting/fog fields as stored (in-band LBP2 layout, see module
/// doc). Toolkit LevelSettings.java @ d6a2ef80 names; meaning in the game renderer is
/// UNKNOWN (preview mapping only, docs/d121-preview-lighting.md).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct LevelSettings {
    pub sun_position: [f32; 3],
    pub sun_position_scale: f32,
    pub sun_color: [f32; 4],
    pub ambient_color: [f32; 4],
    pub sun_multiplier: f32,
    pub exposure: f32,
    pub fog_color: [f32; 4],
    pub fog_near: f32,
    pub fog_far: f32,
    pub rim_color: [f32; 4],
    pub rim_color2: [f32; 4],
    /// P3k: PLevelSettings bakedAOBias / bakedAOScale (file order after rimColor2: shadow
    /// amount, shadow blur, AO bias, AO scale, dynamic AO, DOF near). The game's baked AO is
    /// `lightmap * scale + bias` (oracles 2026-10-10-baked-ao). `Default` is 0/0; readers treat
    /// a non-positive scale as unset (game default bias 0, scale 1).
    pub baked_ao_bias: f32,
    pub baked_ao_scale: f32,
    /// Number of stored preset LevelSettings (not projected).
    pub presets: usize,
    /// D119: `backdropMesh` resource (STATIC_MESH, head >= 0x2f3), after the presets and
    /// the ambience name.
    pub backdrop_mesh: Option<Descriptor>,
}

/// P2l: presets kept per PLevelSettings (levels seen store one).
pub const MAX_KEPT_PRESETS: usize = 16;

impl LevelSettings {
    /// P2l: the stored presets of the holder `find` picks.
    pub fn find_presets(things: &[crate::thing::ThingRecord]) -> &[LevelSettings] {
        match Self::find(things) {
            Some((at, _)) => things
                .iter()
                .find(|t| t.index == at)
                .map_or(&[], |t| t.data.level_settings_presets.as_slice()),
            None => &[],
        }
    }
    /// P3m: linear blend `self * (1 - t) + other * t` of every lighting field (the game
    /// blends the stored presets by PWorld `lighting`; live reads of Bravery Test and
    /// tutorial_23 fit this to 4 digits, local-analysis 2026-10-10-live-level-lighting).
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let f = |a: f32, b: f32| a + (b - a) * t;
        let v3 = |a: [f32; 3], b: [f32; 3]| [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])];
        let v4 =
            |a: [f32; 4], b: [f32; 4]| [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])];
        Self {
            sun_position: v3(self.sun_position, other.sun_position),
            sun_position_scale: f(self.sun_position_scale, other.sun_position_scale),
            sun_color: v4(self.sun_color, other.sun_color),
            ambient_color: v4(self.ambient_color, other.ambient_color),
            sun_multiplier: f(self.sun_multiplier, other.sun_multiplier),
            exposure: f(self.exposure, other.exposure),
            fog_color: v4(self.fog_color, other.fog_color),
            fog_near: f(self.fog_near, other.fog_near),
            fog_far: f(self.fog_far, other.fog_far),
            rim_color: v4(self.rim_color, other.rim_color),
            rim_color2: v4(self.rim_color2, other.rim_color2),
            baked_ao_bias: f(self.baked_ao_bias, other.baked_ao_bias),
            baked_ao_scale: f(self.baked_ao_scale, other.baked_ao_scale),
            presets: self.presets,
            backdrop_mesh: self.backdrop_mesh,
        }
    }

    /// P3m: the lighting the game applies to a level. `presets` are spread evenly over
    /// PWorld `lighting` 0..1 (position `lighting x (n - 1)`, blended between the two
    /// neighbours); PWorld `darkness` scales the exposure and the fog colour by
    /// `1 - darkness`. Without presets `main` is used (darkness still applies).
    /// Returns the settings and the preset position (`None` for the main block).
    /// CONFIRMED_FROM_LOCAL_DATA on live reads of tutorial_23 (lighting 1.0 = preset 2,
    /// darkness 0) and Bravery Test (lighting 0.6, darkness 0.19: 80% preset 1 + 20% preset 2,
    /// exposure 1.0417 x 0.81 = 0.8437, fog x 0.81); spacing for 4 or more presets is
    /// INFERRED from that rule, and so is the fog/exposure split of `darkness`.
    pub fn at_lighting(
        main: &Self,
        presets: &[Self],
        lighting: f32,
        darkness: f32,
    ) -> (Self, Option<f32>) {
        let (mut s, at) = match presets {
            [] => (*main, None),
            [only] => (*only, Some(0.)),
            _ => {
                let lighting = if lighting.is_finite() {
                    lighting.clamp(0., 1.)
                } else {
                    0.
                };
                let at = lighting * (presets.len() - 1) as f32;
                let i = (at.floor() as usize).min(presets.len() - 2);
                (presets[i].lerp(&presets[i + 1], at - i as f32), Some(at))
            }
        };
        if darkness.is_finite() && darkness > 0. {
            let k = (1. - darkness).clamp(0., 1.);
            s.exposure *= k;
            for c in &mut s.fog_color[..3] {
                *c *= k;
            }
        }
        (s, at)
    }
    /// The world Thing's (PWorld holder) settings, else the first Thing carrying the
    /// part, with that Thing's index. Several holders are not merged.
    pub fn find(things: &[crate::thing::ThingRecord]) -> Option<(usize, &LevelSettings)> {
        let held = |t: &&crate::thing::ThingRecord| t.data.level_settings.is_some();
        let t = things
            .iter()
            .filter(held)
            .find(|t| t.data.world.is_some())
            .or_else(|| things.iter().find(held))?;
        t.data.level_settings.as_ref().map(|s| (t.index, s))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Pos {
    pub bone_of: Option<usize>,
    pub anim_hash: i32,
    /// Serialized element order (translation in 12..14, see serial.rs).
    pub world: [f32; 16],
}

/// PBody as stored: linear velocity, angular velocity (z only) and the `frozen` int
/// (meaning UNVERIFIED, docs/runtime-evidence/physics.md A1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Body {
    pub velocity: [f32; 3],
    pub angular_velocity: f32,
    pub frozen: i32,
}

/// PJoint placement summary (D120): endpoint Things, contact points in each endpoint's
/// local frame, rest length and angle, settings, bone indices/lengths, type, render scale.
#[derive(Debug, Clone, Serialize)]
pub struct Joint {
    pub a: Option<usize>,
    pub b: Option<usize>,
    pub a_contact: [f32; 3],
    pub b_contact: [f32; 3],
    pub length: f32,
    pub angle: f32,
    pub settings: Option<Descriptor>,
    pub bone_idx: Vec<i32>,
    pub bone_lengths: [f32; 4],
    pub joint_type: i32,
    pub render_scale: f32,
    pub hide_in_play_mode: bool,
    /// Strength, motion and piston settings (lbp-sim joints).
    pub params: JointParams,
}

/// PJoint fields that drive the joint, as stored (units UNVERIFIED).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct JointParams {
    pub offset_time: f32,
    pub invert_angle: bool,
    pub strength: f32,
    pub stiff: bool,
    pub slide_dir: [f32; 3],
    pub animation_pattern: i32,
    /// range, time, phase, speed, pause, aAngleOffset, bAngleOffset, modStartFrames,
    /// modDeltaFrames, modScale.
    pub motion: [f32; 10],
    pub tweak_target_max_length: f32,
    pub tweak_target_min_length: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderMesh {
    pub mesh: Option<Descriptor>,
    pub bone_things: Vec<Option<usize>>,
    pub anim: Option<Descriptor>,
    pub anim_pos: f32,
    pub anim_speed: f32,
    pub anim_loop: bool,
    pub editor_color: u32,
    pub cast_shadows: i8,
    pub visibility_flags: i8,
    pub poppet_render_scale: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shape {
    pub vertex_count: usize,
    pub requires_z: bool,
    pub vertices: Vec<[f32; 3]>,
    pub loops: Vec<u32>,
    pub material: Option<Descriptor>,
    pub old_material: Option<Descriptor>,
    pub thickness: f32,
    pub mass_depth: f32,
    /// Raw i32 colour (head > 0x389) or the v4 for older heads.
    pub color: ShapeColor,
    pub brightness: f32,
    pub bevel_size: f32,
    pub com: Option<[f32; 16]>,
    pub behavior: u32,
    pub flags: i16,
    pub contacts: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ShapeColor {
    Packed(u32),
    Vector([f32; 4]),
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedMesh {
    pub gfx_material: Option<Descriptor>,
    pub bevel: Option<Descriptor>,
    pub uv_offset: [f32; 4],
    pub plan_guid: Option<u32>,
    pub play_mode_visible: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Group {
    pub plan: Option<Descriptor>,
    pub emitter: Option<usize>,
    pub flags: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct Decoration {
    /// Render mesh of the decoration when its reference body was read here (None for a
    /// null or back-reference).
    pub render_mesh: Option<RenderMesh>,
    /// Serialized element order (translation in 12..14), as POS.
    pub offset: [f32; 16],
    pub parent_bone: i32,
    pub parent_tri_vert: i32,
    pub bary_u: f32,
    pub bary_v: f32,
    pub angle: f32,
    pub on_costume_piece: i32,
    pub earth_decoration: i32,
    pub scale: f32,
    pub placed_by: i16,
    pub reversed: bool,
    pub play_mode_frame: i32,
    pub plan_guid: Option<u32>,
}

/// D160: one STICKERS decal. `u`, `v` is the decal centre and `xvec` / `yvec` its half
/// axes, all in the owner's surface (UV) space; see docs/d160-globe-material-and-stickers.md.
#[derive(Debug, Clone, Serialize)]
pub struct Decal {
    pub texture: Option<Descriptor>,
    pub u: f32,
    pub v: f32,
    pub xvec: [f32; 2],
    pub yvec: [f32; 2],
    /// Packed 16-bit colour (layout unverified).
    pub color: i16,
    pub kind: i8,
    pub metadata_index: i16,
    pub num_metadata: Option<i16>,
    pub placed_by: i16,
    pub play_mode_frame: i32,
    pub scorch_mark: bool,
    pub plan: Option<Descriptor>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct World {
    pub things: Vec<Option<usize>>,
    pub thing_uid_counter: Option<i32>,
    /// D119: PWorld `backdrop` / `backdropNew` Thing references (Thing indices) and the
    /// `backdropPlan` / `backdropNewPlan` plan descriptors (head >= 0x321).
    pub backdrop: Option<usize>,
    pub backdrop_new: Option<usize>,
    pub backdrop_plan: Option<Descriptor>,
    pub backdrop_new_plan: Option<Descriptor>,
    /// P3e: PWorld `lighting`, `colourCorrection`, `fog`, `fogTint`, `darkness` f32 (the
    /// current values; meaning of each UNKNOWN, read for the preset-selection question).
    pub lighting: [f32; 5],
    /// P3e: the `target` and `from` GlobalSettings (5 f32 each) and the blend factor f32
    /// stored after them (field names and order from community layouts, UNVERIFIED).
    pub gs_target: [f32; 5],
    pub gs_from: [f32; 5],
    pub gs_blend: f32,
}

/// `reference(P*)` for a present part of Thing `me`.
pub fn part_ref(
    r: &mut Reader,
    things: &mut Things,
    me: usize,
    name: &'static str,
) -> Result<&'static str> {
    let idx = r.other_count(name);
    let tag = r.tag(name, |r| Ok(r.next_other(name)))?;
    match tag {
        Tag::Null => Ok("null"),
        Tag::Seen(o) => {
            if let crate::serial::Obj::Other(_, i) = o {
                let d = &mut things.list[me].data;
                match name {
                    "SHAPE" => d.shape = things.shapes.get(&i).cloned(),
                    "RENDER_MESH" => d.render_mesh = things.render_meshes.get(&i).cloned(),
                    _ => {}
                }
            }
            Ok("shared")
        }
        Tag::New(_) => {
            r.enter()?;
            part_body(r, things, me, name, idx)?;
            r.leave();
            Ok("parsed")
        }
    }
}

fn part_body(
    r: &mut Reader,
    things: &mut Things,
    me: usize,
    name: &'static str,
    idx: usize,
) -> Result<()> {
    let v = r.rev.version();
    match name {
        "BODY" => {
            let velocity = r.v3()?;
            let angular_velocity = r.f32()?;
            let frozen = r.i32()?;
            thing_ref(r, things)?;
            things.list[me].data.body = Some(Body {
                velocity,
                angular_velocity,
                frozen,
            });
        }
        "POS" => {
            let bone_of = thing_ref(r, things)?;
            let anim_hash = r.i32()?;
            if v < 0x341 {
                r.m44()?; // localPosition (removed at THING_FLAGS 0x341)
            }
            let world = r.m44()?;
            things.list[me].data.pos = Some(Pos {
                bone_of,
                anim_hash,
                world,
            });
        }
        "RENDER_MESH" => {
            let m = render_mesh(r, things)?;
            things.render_meshes.insert(idx, m.clone());
            things.list[me].data.render_mesh = Some(m);
        }
        "SHAPE" => {
            let s = shape(r, things)?;
            things.shapes.insert(idx, s.clone());
            things.list[me].data.shape = Some(s);
        }
        "GENERATED_MESH" => {
            let g = GeneratedMesh {
                gfx_material: r.resource(false)?,
                bevel: r.resource(false)?,
                uv_offset: r.v4()?,
                plan_guid: r.guid()?,
                play_mode_visible: r.bool()?,
            };
            r.skip_f32(2)?;
            things.list[me].data.generated_mesh = Some(g);
        }
        "GROUP" => {
            // Below 0x341 (plans only): copyright bool before the creator, editable bool
            // after the plan, then pickupAllMembers and mainSelectableObject bools
            // (>= 0x30f) instead of the u8 flags. Pre-band flags are reported as 0.
            let pre = v < 0x341;
            if pre {
                r.bool()?;
            }
            r.bytes(36)?;
            let plan = r.resource(true)?;
            if pre {
                r.bool()?;
            }
            let emitter = thing_ref(r, things)?;
            r.i32()?;
            r.i32()?;
            let flags = if pre {
                r.bool()?;
                r.bool()?;
                0
            } else {
                r.u8()?
            };
            things.list[me].data.group = Some(Group {
                plan,
                emitter,
                flags,
            });
        }
        "METADATA" => {
            // D119 (plans): PMetadata at head > 0x2ba: titleKey, descriptionKey, location,
            // category u32, primaryIndex i32 (>= 0x195), fluffCost i32, type flags i32,
            // subType i32, creationDate u32, icon res, reference(PhotoMetadata),
            // referencable bool (>= 0x15f), allowEmit bool (>= 0x205).
            for _ in 0..4 {
                r.u32()?;
            }
            for _ in 0..4 {
                r.i32()?;
            }
            r.u32()?;
            r.resource(false)?;
            if let Tag::New(_) = r.tag("PhotoMetadata", |r| Ok(r.next_other("PhotoMetadata")))? {
                r.enter()?;
                photo_metadata(r)?;
                r.leave();
            }
            r.bool()?;
            r.bool()?;
        }
        "REF" => {
            let plan = r.resource(true)?;
            r.i32()?;
            r.i32()?;
            things.list[me].data.plan_ref = plan;
        }
        "EFFECTOR" => {
            r.v3()?;
            r.skip_f32(3)?;
            r.v3()?;
            r.bool()?;
            r.bool()?;
            r.skip_f32(2)?;
        }
        "SCRIPT_NAME" => {
            let n = r.s32()?;
            let n = usize::try_from(n).or_else(|_| r.invalid("negative script name length"))?;
            let b = r.bytes(n)?;
            let name_len = b.iter().position(|&c| c == 0).unwrap_or(n);
            things.list[me].data.script_name = Some((n, name_len));
        }
        "SPRITE_LIGHT" => {
            r.v4()?;
            r.v4()?;
            r.skip_f32(5)?;
            r.resource(false)?;
            thing_ref(r, things)?;
            r.bool()?;
            if v < 0x337 {
                r.bool()?;
                r.bool()?;
            }
            r.skip_f32(5)?;
            r.i32()?;
        }
        "TRIGGER" => {
            let trigger = Trigger {
                trigger_type: r.i8()?,
                in_things: thing_array(r, things)?,
                radius_multiplier: r.f32()?,
                all_z_layers: r.bool()?,
                hysteresis_multiplier: r.f32()?,
                enabled: r.bool()?,
                z_offset: r.f32()?,
            };
            things.list[me].data.trigger = Some(trigger);
        }
        "STICKERS" => {
            let (main, total) = stickers(r)?;
            things.list[me].data.sticker_decals = total;
            things.list[me].data.stickers = main;
        }
        "DECORATIONS" => {
            let n = r.count(4)?;
            let mut out = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                out.push(decoration(r, things)?);
            }
            things.list[me].data.decorations = out;
        }
        "LEVEL_SETTINGS" => {
            let mut own = level_settings(r)?;
            let n = r.count(4)?;
            let mut kept = Vec::with_capacity(n.min(MAX_KEPT_PRESETS));
            for _ in 0..n {
                let preset = level_settings(r)?;
                if kept.len() < MAX_KEPT_PRESETS {
                    kept.push(preset);
                }
            }
            own.presets = n;
            things.list[me].data.level_settings_presets = kept;
            r.str8()?;
            own.backdrop_mesh = r.resource(false)?;
            things.list[me].data.level_settings = Some(own);
        }
        "CAMERA_TWEAK" => camera_tweak(r)?,
        "JOINT" => {
            let a = thing_ref(r, things)?;
            let b = thing_ref(r, things)?;
            let a_contact = r.v3()?;
            let b_contact = r.v3()?;
            let length = r.f32()?;
            let angle = r.f32()?;
            let offset_time = r.f32()?;
            let invert_angle = r.bool()?;
            let settings = r.resource(false)?;
            let bone_idx = r.int_array()?;
            let bone_lengths = r.v4()?;
            let joint_type = r.i32()?;
            let strength = r.f32()?;
            let stiff = r.bool()?;
            let slide_dir = r.v3()?;
            let animation_pattern = r.i32()?;
            let mut motion = [0.; 10];
            for m in &mut motion {
                *m = r.f32()?;
            }
            let render_scale = r.f32()?;
            r.i32()?;
            let tweak_target_max_length = r.f32()?;
            let tweak_target_min_length = r.f32()?;
            r.bool()?;
            let hide_in_play_mode = r.bool()?;
            r.i32()?;
            things.list[me].data.joint = Some(Joint {
                a,
                b,
                a_contact,
                b_contact,
                length,
                angle,
                settings,
                bone_idx: bone_idx.into_iter().map(|v| v as i32).collect(),
                bone_lengths,
                joint_type,
                render_scale,
                hide_in_play_mode,
                params: JointParams {
                    offset_time,
                    invert_angle,
                    strength,
                    stiff,
                    slide_dir,
                    animation_pattern,
                    motion,
                    tweak_target_max_length,
                    tweak_target_min_length,
                },
            });
        }
        "CHECKPOINT" => {
            r.bool()?;
            r.i32()?;
            r.s32()?;
            r.s32()?;
            r.bool()?;
            thing_array(r, things)?;
            r.i32()?;
            r.i32()?;
            r.i32()?;
        }
        "PHYSICS_TWEAK" => physics_tweak(r, things)?,
        "SWITCH_KEY" => {
            r.s32()?;
            r.wstr()?;
            r.u8()?;
        }
        "GAMEPLAY_DATA" => {
            r.s32()?;
            let tag = r.tag("EggLink", |r| Ok(r.next_other("EggLink")))?;
            if let Tag::New(_) = tag {
                r.resource(false)?;
                r.bool()?;
                if v > 0x3e0 && r.bool()? {
                    r.resource(true)?;
                }
            }
            let tag = r.tag("SlotID", |r| Ok(r.next_other("SlotID")))?;
            if let Tag::New(_) = tag {
                r.u32()?;
                r.u32()?;
            }
            r.i32()?;
        }
        "MICROCHIP" => {
            thing_ref(r, things)?;
            r.bool()?;
            r.bool()?;
            r.s32()?;
            r.v4()?;
            if v >= 0x34d {
                r.wstr()?;
                let n = r.count(1)?;
                for _ in 0..n {
                    thing_ref(r, things)?;
                    r.skip_f32(5)?;
                    r.bool()?;
                }
                r.skip_f32(2)?;
            }
        }
        "ENEMY" => {
            r.s32()?;
            r.f32()?;
            r.i32()?;
            r.v3()?;
            thing_ref(r, things)?;
            r.f32()?;
            r.i32()?;
            r.bool()?;
            r.i32()?;
            if v >= 0x39a {
                r.f32()?;
            }
        }
        "EMITTER" => emitter(r, things)?,
        "AUDIO_WORLD" => {
            r.str8()?;
            r.skip_f32(5)?;
            r.i32()?;
            r.bytes(5)?;
            r.i32()?;
            if v >= 0x380 {
                r.guid()?;
                r.i32()?;
            }
        }
        "SEQUENCER" => {
            r.skip_f32(5)?;
            if v > 0x370 {
                r.i32()?;
            }
            if v > 0x36e {
                r.bool()?;
            }
            r.f32()?;
            r.i32()?;
            r.skip_f32(6)?;
            if v > 0x369 {
                r.f32()?;
                r.bool()?;
            }
            if v > 0x36c {
                r.bool()?;
            }
            if v > 0x371 {
                r.i32()?;
            }
            if v > 0x3a0 {
                r.i32()?;
                thing_ref(r, things)?;
            }
        }
        "SWITCH" => {
            let switch = switch(r, things)?;
            things.list[me].data.switch = Some(switch);
        }
        "SCRIPT" => {
            let part = script_instance(r, things)?;
            things.list[me].data.script = Some(part);
        }
        "MATERIAL_TWEAK" => {
            r.bool()?;
            r.skip_f32(2)?;
            if v < 0x343 {
                r.bytes(2)?;
            }
            if v > 0x342 {
                r.i8()?;
            }
            if v > 0x3bc {
                r.i8()?;
            }
            if v > 0x342 {
                r.i8()?;
            }
            r.bool()?;
            if v > 0x356 {
                r.bool()?;
            }
            if v > 0x3ec {
                r.bool()?;
            }
        }
        "NPC" => {
            r.bool()?;
            r.v3()?;
            r.v3()?;
            r.v3()?;
            r.s32()?;
            r.i32()?;
            let n = r.count(1)?;
            r.bytes(n)?;
            r.int_vector()?;
            r.i32()?;
            r.i32()?;
            r.int_vector()?;
            npc_behavior_ref(r, things)?;
            r.i32()?;
            thing_ref(r, things)?;
            thing_ref(r, things)?;
            if v < 0x36e {
                for _ in 0..3 {
                    r.i32()?;
                }
            }
            r.v3()?;
            if v < 0x36e {
                r.v3()?;
            }
            r.s32()?;
            r.i32()?;
            r.wstr()?;
            if v > 0x353 {
                if v == 0x354 || v == 0x355 {
                    r.skip_f32(2)?;
                } else {
                    r.i32()?;
                    r.i32()?;
                }
                r.i32()?;
            }
            if v > 0x391 {
                r.i8()?;
            }
        }
        "WORMHOLE" => {
            r.s32()?;
            r.s32()?;
            r.bytes(4)?;
            r.i32()?;
            r.i32()?;
        }
        "INSTRUMENT" => {
            r.resource(false)?;
            if v >= 0x35b {
                r.wstr()?;
            }
            for _ in 0..4 {
                r.i32()?;
            }
            r.skip_f32(4)?;
            if v >= 0x389 {
                r.bytes(8)?;
            }
            let n = r.count(4)?;
            r.bytes(n * 4)?;
            if v >= 0x379 {
                r.resource(false)?;
            }
        }
        "CREATURE" => creature(r, things)?,
        "CONTROLINATOR" => {
            for _ in 0..4 {
                thing_ref(r, things)?;
            }
            r.i32()?;
            r.f32()?;
            r.bool()?;
            r.i32()?;
            r.bytes(3)?;
            thing_ref(r, things)?;
            if v > 0x3ec {
                r.i8()?;
            }
        }
        "COSTUME" => {
            r.resource(false)?;
            r.resource(false)?;
            r.resource(true)?;
            r.int_vector()?;
            primitives(r)?;
            let pieces = if v >= 0x2c5 { 2 } else { 1 };
            for _ in 0..pieces {
                let n = r.count(1)?;
                for _ in 0..n {
                    r.resource(false)?;
                    r.i32()?;
                    let m = r.count(1)?;
                    for _ in 0..m {
                        r.i32()?;
                    }
                    primitives(r)?;
                    r.resource(true)?;
                }
            }
        }
        "TAG_SYNCHRONIZER" => {
            r.s32()?;
            r.s32()?;
            r.wstr()?;
            r.skip_f32(4)?;
            r.bool()?;
            r.bool()?;
        }
        "MATERIAL_OVERRIDE" if v < 0x360 => {
            primitives(r)?;
            let n = r.count(1)?;
            for _ in 0..n {
                r.resource(true)?;
            }
            let n = r.count(4)?;
            r.bytes(n.checked_mul(12).ok_or(crate::serial::Stop::Budget {
                what: "uv scales",
                offset: r.pos(),
            })?)?;
            r.resource(false)?;
        }
        "MATERIAL_OVERRIDE" => {
            let n = r.count(4)?;
            for _ in 0..n {
                r.i32()?;
                r.resource(true)?;
                r.resource(false)?;
                r.v3()?;
            }
            r.resource(false)?;
        }
        "SWITCH_INPUT" => {
            thing_ref(r, things)?;
            r.i32()?;
            r.i32()?;
            r.bool()?;
            r.i32()?;
            r.bool()?;
            r.bool()?;
            if v > 0x38f {
                r.bool()?;
            }
            npc_behavior_ref(r, things)?;
            r.s32()?;
            r.i32()?;
            r.i32()?;
            if v > 0x3ec {
                r.i8()?;
            }
        }
        "WORLD" => {
            let list = thing_array(r, things)?;
            things.world_array_done = Some(list.len());
            r.skip_f32(2)?;
            r.s32()?;
            r.s32()?;
            if v >= 0x377 {
                r.i32()?;
            }
            let uid = r.i32()?;
            things.list[me].data.world = Some(World {
                things: list,
                thing_uid_counter: Some(uid),
                ..Default::default()
            });
            let tail = world_tail(r, things)?;
            if let Some(w) = things.list[me].data.world.as_mut() {
                w.backdrop = tail.backdrop;
                w.backdrop_new = tail.backdrop_new;
                w.backdrop_plan = tail.backdrop_plan;
                w.backdrop_new_plan = tail.backdrop_new_plan;
                w.lighting = tail.lighting;
                w.gs_target = tail.gs_target;
                w.gs_from = tail.gs_from;
                w.gs_blend = tail.gs_blend;
            }
        }
        _ => return r.unsupported(format!("part reader {name}")),
    }
    Ok(())
}

/// RLevel tutorialInventory element (InventoryItem + InventoryItemDetails). Band rules:
/// plan desc; details (head > 0x37c: dateAdded s64, SlotID, highlightSound guid, colour
/// i32, type i32, subType i32, titleKey u32, descriptionKey u32, ref CreationHistory
/// {i32 count, 20-byte fixed strings}, icon desc, ref UserCreatedDetails {wstr, wstr},
/// ref InventoryItemPhotoData, ref EyetoyData, location/category/primary i16, ref
/// NetworkPlayerID (36 bytes), toolType i8, flags i8; head <= 0x37c: fixed BE32
/// highlightSound, slotType, slotNumber, location, category, primary, lastUsed, numUses,
/// pad, fixed BE64 dateAdded, fixed BE32 fluffCost, colour, type, subType, toolType,
/// NetworkPlayerID, allowEmit/shareable/copyright bool, flags i8, titleKey u32,
/// descriptionKey u32, UserCreatedDetails struct, CreationHistory struct (i32 count +
/// wstr each), icon desc, ref photo data, ref EyetoyData, u8 (> 0x358)); then fixed BE32
/// UID, (two fixed BE32 tutorial values below 0x36c), fixed BE32 flags, fixed BE32
/// userCategoryIndex (>= 0x34a). Toolkit: structs/profile/InventoryItem.java,
/// structs/inventory/InventoryItemDetails.java, CreationHistory, UserCreatedDetails,
/// InventoryItemPhotoData, PhotoMetadata, PhotoUser, EyetoyData.
pub(crate) fn inventory_item(r: &mut Reader) -> Result<()> {
    let v = r.rev.version();
    r.resource(true)?;
    if v > 0x37c {
        r.u64()?;
        r.u32()?;
        r.u32()?;
        r.guid()?;
        for _ in 0..5 {
            r.u32()?;
        }
        if let Tag::New(_) = r.tag("CreationHistory", |r| Ok(r.next_other("CreationHistory")))? {
            let n = r.count(20)?;
            r.bytes(n * 20)?;
        }
        r.resource(true)?;
        if let Tag::New(_) = r.tag("UserCreatedDetails", |r| {
            Ok(r.next_other("UserCreatedDetails"))
        })? {
            r.wstr()?;
            r.wstr()?;
        }
        photo_data_ref(r)?;
        eyetoy_ref(r)?;
        r.bytes(6)?;
        if let Tag::New(_) = r.tag("NetworkPlayerID", |r| Ok(r.next_other("NetworkPlayerID")))? {
            r.bytes(36)?;
        }
        r.bytes(2)?;
    } else {
        for _ in 0..8 {
            r.fixed_u32()?;
        }
        r.fixed_u32()?;
        r.fixed_u64()?;
        for _ in 0..5 {
            r.fixed_u32()?;
        }
        r.bytes(36)?;
        r.bytes(3)?;
        r.i8()?;
        r.u32()?;
        r.u32()?;
        r.wstr()?;
        r.wstr()?;
        let n = r.count(1)?;
        for _ in 0..n {
            r.wstr()?;
        }
        r.resource(true)?;
        photo_data_ref(r)?;
        eyetoy_ref(r)?;
        if v > 0x358 {
            r.u8()?;
        }
    }
    r.fixed_u32()?;
    if v < 0x36c {
        r.fixed_u32()?;
        r.fixed_u32()?;
    }
    r.fixed_u32()?;
    if v >= 0x34a {
        r.fixed_u32()?;
    }
    Ok(())
}

fn photo_data_ref(r: &mut Reader) -> Result<()> {
    if let Tag::New(_) = r.tag("InventoryItemPhotoData", |r| {
        Ok(r.next_other("InventoryItemPhotoData"))
    })? {
        r.resource(true)?;
        r.resource(true)?;
        photo_metadata(r)?;
        if r.rev.version() > 0x3c7 {
            r.resource(true)?;
        }
    }
    Ok(())
}

/// PhotoMetadata struct: photo desc, SlotID (2 x u32), levelName wstr, levelHash sha1,
/// timestamp s64 (same bytes as u64), users array(PhotoUser: 20-byte PSID, wstr, v4).
fn photo_metadata(r: &mut Reader) -> Result<()> {
    r.resource(true)?;
    r.u32()?;
    r.u32()?;
    r.wstr()?;
    r.sha1()?;
    r.u64()?;
    let n = r.count(1)?;
    for _ in 0..n {
        r.bytes(20)?;
        r.wstr()?;
        r.v4()?;
    }
    Ok(())
}

fn eyetoy_ref(r: &mut Reader) -> Result<()> {
    if let Tag::New(_) = r.tag("EyetoyData", |r| Ok(r.next_other("EyetoyData")))? {
        r.resource(false)?;
        r.resource(false)?;
        r.m44()?;
        r.skip_f32(6)?;
        if r.rev.version() > 0x39f {
            r.resource(false)?;
        }
    }
    Ok(())
}

/// EggLink struct: plan res, shareable bool, hasPainting bool (> 0x3e0), painting desc.
fn egg_link(r: &mut Reader) -> Result<()> {
    r.resource(false)?;
    r.bool()?;
    if r.rev.version() > 0x3e0 && r.bool()? {
        r.resource(true)?;
    }
    Ok(())
}

/// PWorld fields after thingUIDCounter (see module notes, WORLD tail).
fn world_tail(r: &mut Reader, things: &mut Things) -> Result<World> {
    let mut out = World::default();
    let v = r.rev.version();
    let n = r.count(1)?;
    for _ in 0..n {
        let tag = r.tag("EditorSelection", |r| Ok(r.next_other("EditorSelection")))?;
        if let Tag::New(_) = tag {
            r.enter()?;
            r.str8()?;
            thing_array(r, things)?;
            r.leave();
        }
    }
    out.backdrop = thing_ref(r, things)?;
    out.backdrop_new = thing_ref(r, things)?;
    r.f32()?;
    if v >= 0x3a3 {
        r.s32()?;
    }
    r.bool()?;
    for x in &mut out.lighting {
        *x = r.f32()?;
    }
    for _ in 0..3 {
        let n = r.count(2)?;
        for _ in 0..n {
            egg_link(r)?;
        }
    }
    r.bool()?;
    if v >= 0x35e {
        r.u32()?;
        r.u32()?;
    }
    for _ in 0..3 {
        let n = r.count(2)?;
        for _ in 0..n {
            r.u32()?;
            r.u32()?;
        }
    }
    if v < 0x36e {
        r.i32()?;
    }
    r.s32()?;
    thing_array(r, things)?;
    thing_array(r, things)?;
    if v < 0x345 {
        r.bool()?;
    }
    r.bool()?;
    r.s32()?;
    for x in out.gs_target.iter_mut().chain(out.gs_from.iter_mut()) {
        *x = r.f32()?;
    }
    out.gs_blend = r.f32()?;
    r.bool()?;
    r.s32()?;
    if v >= 0x370 {
        r.skip_f32(4)?;
    }
    r.skip_f32(9)?;
    let n = r.count(4)?;
    r.bytes(n * 4)?;
    r.int_array()?;
    r.int_array()?;
    r.f32()?;
    r.v4()?;
    r.f32()?;
    r.skip_f32(6)?;
    if v >= 0x34e {
        r.bool()?;
        r.bool()?;
    }
    r.f32()?;
    r.i32()?;
    r.s32()?;
    // CutsceneCameraManager
    r.i32()?;
    thing_ref(r, things)?;
    r.s32()?;
    r.s32()?;
    r.f32()?;
    if v < 0x36e {
        r.bool()?;
        r.v3()?;
        r.f32()?;
        r.v3()?;
        r.bool()?;
        r.v3()?;
        r.f32()?;
        r.v3()?;
    }
    if v >= 0x3a0 {
        r.bool()?;
    }
    // GlobalAudioSettings
    r.s32()?;
    if v < 0x347 {
        r.f32()?;
    }
    r.skip_f32(4)?;
    if v >= 0x342 {
        r.skip_f32(2)?;
    }
    if v >= 0x347 {
        r.skip_f32(4)?;
    }
    out.backdrop_plan = r.resource(true)?;
    out.backdrop_new_plan = r.resource(true)?;
    if v >= 0x352 {
        r.bool()?;
    }
    if v >= 0x38a {
        r.bool()?;
    }
    if v >= 0x38b {
        r.s32()?;
    }
    if v >= 0x3ac {
        r.bool()?;
    }
    if v >= 0x3bd {
        let n = r.count(1)?;
        for _ in 0..n {
            if v > 0x3c1 {
                r.i32()?;
            }
            thing_ref(r, things)?;
            thing_ref(r, things)?;
            r.v4()?;
            r.f32()?;
            if v < 0x3c2 {
                r.v4()?;
                r.v4()?;
            }
            r.v4()?;
            if v > 0x3c1 {
                r.v4()?;
                r.v4()?;
                r.v4()?;
                r.v4()?;
                r.skip_f32(2)?;
            }
            r.bool()?;
            r.bool()?;
            if v < 0x3c2 {
                r.v4()?;
            }
            r.f32()?;
        }
    }
    if v >= 0x3d0 {
        r.u8()?;
        r.u8()?;
        r.bool()?;
    }
    if v >= 0x3dd {
        r.bool()?;
    }
    if v >= 0x3f1 {
        r.bool()?;
    }
    Ok(out)
}

/// reference(NpcBehavior) with its inline body (and the ActingData/InputRecording it
/// references).
fn npc_behavior_ref(r: &mut Reader, things: &mut Things) -> Result<()> {
    let v = r.rev.version();
    let tag = r.tag("NpcBehavior", |r| Ok(r.next_other("NpcBehavior")))?;
    if !matches!(tag, Tag::New(_)) {
        return Ok(());
    }
    r.enter()?;
    thing_ref(r, things)?;
    thing_ref(r, things)?;
    r.s32()?;
    r.i32()?;
    r.f32()?;
    r.s32()?;
    r.wstr()?;
    r.s32()?;
    r.wstr()?;
    r.s32()?;
    let tag = r.tag("ActingData", |r| Ok(r.next_other("ActingData")))?;
    if let Tag::New(_) = tag {
        r.enter()?;
        r.i32()?;
        thing_ref(r, things)?;
        // InputRecording
        let n = r.count(1)?;
        r.bytes(n)?;
        r.int_vector()?;
        r.int_vector()?;
        r.m44()?;
        let n = r.count(2)?;
        for _ in 0..n {
            r.m44()?;
        }
        r.v3()?;
        let n = r.count(12)?;
        r.bytes(n * 12)?;
        if v > 0x3c4 {
            r.bool()?;
        }
        thing_ref(r, things)?;
        r.i32()?;
        r.s32()?;
        r.resource(false)?;
        r.leave();
    }
    r.f32()?;
    r.i32()?;
    r.v3()?;
    r.v3()?;
    r.bool()?;
    r.v3()?;
    for _ in 0..8 {
        r.s32()?;
    }
    r.s32()?;
    if v > 0x371 {
        r.bytes(2)?;
    }
    if v > 0x375 {
        r.bool()?;
    }
    r.leave();
    Ok(())
}

/// array(Primitive): material res, textureAlternatives res, minVert, maxVert,
/// firstIndex, numIndices, region i32.
fn primitives(r: &mut Reader) -> Result<()> {
    let n = r.count(7)?;
    for _ in 0..n {
        r.resource(false)?;
        r.resource(false)?;
        for _ in 0..5 {
            r.i32()?;
        }
    }
    Ok(())
}

fn creature(r: &mut Reader, things: &mut Things) -> Result<()> {
    let v = r.rev.version();
    r.resource(false)?;
    r.s32()?;
    r.f32()?;
    r.v3()?;
    thing_ref(r, things)?;
    thing_ref(r, things)?;
    r.i32()?;
    r.i32()?;
    r.skip_f32(3)?;
    r.i32()?;
    r.s32()?;
    r.s32()?;
    r.v3()?;
    r.i32()?;
    r.f32()?;
    r.i32()?;
    if v >= 0x354 {
        r.int_vector()?;
        r.int_vector()?;
    }
    r.bool()?;
    thing_array(r, things)?;
    thing_array(r, things)?;
    thing_ref(r, things)?;
    thing_ref(r, things)?;
    r.i32()?;
    r.i32()?;
    r.bool()?;
    for _ in 0..3 {
        thing_array(r, things)?;
    }
    r.bool()?;
    r.m44()?;
    r.f32()?;
    r.v3()?;
    r.v3()?;
    r.f32()?;
    r.v3()?;
    thing_ref(r, things)?;
    r.i32()?;
    r.i32()?;
    r.i32()?;
    thing_ref(r, things)?;
    if v >= 0x3a2 {
        thing_ref(r, things)?;
    }
    r.i32()?;
    r.i32()?;
    r.f32()?;
    r.bool()?;
    r.i32()?;
    r.f32()?;
    r.v3()?;
    r.f32()?;
    thing_ref(r, things)?;
    thing_ref(r, things)?;
    r.i32()?;
    r.i32()?;
    r.skip_f32(2)?;
    r.bool()?;
    r.bool()?;
    r.resource(false)?;
    thing_ref(r, things)?;
    thing_ref(r, things)?;
    r.f32()?;
    thing_ref(r, things)?;
    r.s32()?;
    r.v3()?;
    let n = r.count(12)?;
    r.bytes(n * 12)?;
    r.s32()?;
    r.bool()?;
    thing_ref(r, things)?;
    r.i32()?;
    r.i32()?;
    r.bool()?;
    if v >= 0x3f0 {
        r.i8()?;
    }
    if v >= 0x35a {
        r.i32()?;
    }
    if v >= 0x373 {
        r.i32()?;
    }
    if v >= 0x3c0 {
        let tag = r.tag("WhipSim", |r| Ok(r.next_other("WhipSim")))?;
        if let Tag::New(_) = tag {
            r.enter()?;
            thing_ref(r, things)?;
            r.m44()?;
            r.v3()?;
            r.v3()?;
            r.i32()?;
            r.i8()?;
            thing_ref(r, things)?;
            r.v3()?;
            r.v3()?;
            r.skip_f32(2)?;
            r.bool()?;
            r.f32()?;
            r.leave();
        }
    }
    Ok(())
}

fn emitter(r: &mut Reader, things: &mut Things) -> Result<()> {
    let v = r.rev.version();
    if v < 0x368 {
        r.v3()?;
    }
    r.f32()?;
    r.i32()?;
    r.i32()?;
    r.i32()?;
    r.bool()?;
    r.resource(false)?;
    r.i32()?;
    r.i32()?;
    r.skip_f32(2)?;
    switch_signal(r)?;
    if v >= 0x38e {
        r.f32()?;
    }
    r.skip_f32(2)?;
    r.i32()?;
    r.bool()?;
    r.v4()?;
    r.skip_f32(5)?;
    r.bool()?;
    r.i32()?;
    if v >= 0x38d {
        r.i8()?;
        r.i8()?;
    } else {
        r.i32()?;
        r.i32()?;
    }
    r.bool()?;
    let tag = r.tag("EmittedObjectSource", |r| {
        Ok(r.next_other("EmittedObjectSource"))
    })?;
    if let Tag::New(_) = tag {
        r.enter()?;
        thing_array(r, things)?;
        r.resource(false)?;
        r.leave();
    }
    if v >= 0x361 {
        r.bool()?;
    }
    if v >= 0x38d {
        r.bool()?;
    }
    if v >= 0x3ae {
        r.f32()?;
    }
    Ok(())
}

fn switch_signal(r: &mut Reader) -> Result<SwitchSignal> {
    Ok(SwitchSignal {
        activation: r.f32()?,
        ternary: r.s32()?,
        player: r.i32()?,
    })
}

fn switch(r: &mut Reader, things: &mut Things) -> Result<Switch> {
    let v = r.rev.version();
    let inverted = r.bool()?;
    r.f32()?;
    if v >= 0x382 {
        r.f32()?;
    }
    r.s32()?;
    r.wstr()?;
    if v >= 0x38f {
        r.bool()?;
    }
    r.s32()?;
    let count = r.count(1)?;
    let mut outputs = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let tag = r.tag("SwitchOutput", |r| Ok(r.next_other("SwitchOutput")))?;
        let mut output = None;
        if let Tag::New(_) = tag {
            r.enter()?;
            let signal = switch_signal(r)?;
            let n = r.count(1)?;
            let mut targets = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                let thing = thing_ref(r, things)?;
                targets.push((thing, r.i32()?));
            }
            if v >= 0x34d {
                r.wstr()?;
            }
            r.leave();
            output = Some(SwitchOutput { signal, targets });
        }
        outputs.push(output);
    }
    if v < 0x398 {
        r.resource(true)?;
    }
    r.bool()?;
    let kind = r.s32()?;
    thing_ref(r, things)?;
    let manual_activation = switch_signal(r)?;
    if v >= 0x398 && kind == 5 {
        r.resource(true)?;
    }
    if v < 0x368 {
        r.f32()?;
    }
    r.s32()?;
    r.bool()?;
    r.f32()?;
    let include_touching = r.s32()?;
    if v >= 0x398 && kind == 36 && include_touching == 1 {
        r.resource(true)?;
    }
    r.s32()?;
    r.s32()?;
    if v < 0x398 {
        r.i32()?;
    }
    r.i32()?;
    r.bool()?;
    if v < 0x398 {
        r.i32()?;
    }
    if v < 0x369 {
        r.i32()?;
    }
    thing_array(r, things)?;
    r.bool()?;
    r.f32()?;
    r.i32()?;
    let behavior = r.i32()?;
    for _ in 0..6 {
        r.i32()?;
    }
    if v < 0x3ad {
        r.u8()?;
        r.s32()?;
    }
    r.bool()?;
    r.i32()?;
    if v > 0x34c {
        r.i32()?;
        r.bool()?;
    }
    if v > 0x34f {
        r.i8()?;
    }
    if v > 0x390 {
        r.bool()?;
    }
    if v > 0x3a4 {
        r.bool()?;
    }
    if v > 0x3ec {
        r.i8()?;
    }
    if v > 0x3ee && kind == 65 {
        r.i32()?;
        r.bytes(20)?;
        r.wstr()?;
        let n = r.count(4)?;
        r.bytes(n * 4)?;
        let n = r.count(1)?;
        r.bytes(n)?;
    }
    Ok(Switch {
        inverted,
        outputs,
        switch_type: kind,
        manual_activation,
        behavior,
    })
}

const DIVERGENT: u32 = 1 << 12;

fn instance_layout(r: &mut Reader, things: &mut Things) -> Result<()> {
    let v = r.rev.version();
    let n = r.count(4)?;
    let mut fields = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        let name = r.string8()?;
        let (modifiers, machine) = if v >= 0x3d9 {
            let m = u32::from(r.u16()?);
            let t = r.u8()?;
            r.u8()?;
            r.i8()?;
            r.u8()?;
            (m, u32::from(t))
        } else {
            let m = r.u32()?;
            let t = r.u32()?;
            r.u32()?;
            r.i8()?;
            r.u32()?;
            (m, t)
        };
        r.i32()?;
        let machine = u8::try_from(machine).unwrap_or(u8::MAX);
        fields.push(LayoutField {
            name,
            machine_type: machine,
            divergent: modifiers & DIVERGENT != 0,
        });
    }
    r.i32()?;
    things.layouts.push(fields);
    Ok(())
}

fn script_instance(r: &mut Reader, things: &mut Things) -> Result<ScriptPart> {
    let script = r.resource(false)?;
    let mut part = ScriptPart {
        script,
        fields: Vec::new(),
    };
    if !r.bool()? {
        return Ok(part);
    }
    let tag = r.tag("InstanceLayout", |r| Ok(r.next_other("InstanceLayout")))?;
    let layout = match tag {
        Tag::Null => return r.invalid("script instance without a layout"),
        Tag::Seen(crate::serial::Obj::Other(_, i)) => i,
        Tag::Seen(_) => return r.invalid("layout tag is not a layout"),
        Tag::New(_) => {
            let id = things.layouts.len();
            r.enter()?;
            instance_layout(r, things)?;
            r.leave();
            id
        }
    };
    let divergent = r.bool()?;
    let Some(fields) = things.layouts.get(layout).cloned() else {
        return r.invalid("instance layout still being read");
    };
    part.fields.reserve(fields.len());
    for f in fields {
        let value = if f.divergent && !divergent {
            ScriptValue::NotStored
        } else {
            match f.machine_type {
                1 => ScriptValue::Bool(r.bool()?),
                2 => ScriptValue::Char(r.i16()?),
                3 => ScriptValue::Int(r.i32()?),
                4 => ScriptValue::Float(r.f32()?),
                5 => ScriptValue::V4(r.v4()?),
                6 => ScriptValue::M44(r.m44()?),
                0xa => ScriptValue::Thing(thing_ref(r, things)?),
                0xb => script_object(r, things)?,
                other => return r.unsupported(format!("script field machine type {other}")),
            }
        };
        part.fields.push(ScriptField {
            name: f.name,
            machine_type: f.machine_type,
            value,
        });
    }
    Ok(part)
}

fn script_object(r: &mut Reader, things: &mut Things) -> Result<ScriptValue> {
    let kind = r.u32()?;
    if kind == 0 {
        return Ok(ScriptValue::Object {
            kind,
            id: None,
            body: None,
        });
    }
    let mut new_id = None;
    let tag = r.tag("ScriptObject", |r| {
        let o = r.next_other("ScriptObject");
        if let crate::serial::Obj::Other(_, i) = o {
            new_id = Some(i);
        }
        Ok(o)
    })?;
    let id = match tag {
        Tag::Null => None,
        Tag::Seen(crate::serial::Obj::Other(_, i)) => Some(i),
        Tag::Seen(_) => return r.invalid("script object tag is not a script object"),
        Tag::New(_) => new_id,
    };
    if !matches!(tag, Tag::New(_)) {
        return Ok(ScriptValue::Object {
            kind,
            id,
            body: None,
        });
    }
    r.enter()?;
    let body = match kind {
        1 => {
            let n = r.count(1)?;
            ScriptObject::Bools(r.bytes(n)?.iter().map(|&b| b != 0).collect())
        }
        3 => ScriptObject::Ints(r.int_vector()?.into_iter().map(|v| v as i32).collect()),
        4 => {
            let n = r.count(4)?;
            let mut v = Vec::with_capacity(n);
            for _ in 0..n {
                v.push(r.f32()?);
            }
            ScriptObject::Floats(v)
        }
        5 => {
            let n = r.count(16)?;
            let mut v = Vec::with_capacity(n);
            for _ in 0..n {
                v.push(r.v4()?);
            }
            ScriptObject::Vectors(v)
        }
        10 => ScriptObject::Things(thing_array(r, things)?),
        11 => {
            let n = r.count(1)?;
            let mut v = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                v.push(script_object(r, things)?);
            }
            ScriptObject::Objects(v)
        }
        12 => {
            let kind = r.u32()?;
            let resource = if kind != 0 { r.resource(false)? } else { None };
            ScriptObject::Resource { kind, resource }
        }
        13 => ScriptObject::Instance(script_instance(r, things)?),
        14 => ScriptObject::Text(r.wstring()?),
        15 => ScriptObject::AudioHandle,
        16 => ScriptObject::Text(r.string8()?),
        other => return r.unsupported(format!("script object type {other}")),
    };
    r.leave();
    Ok(ScriptValue::Object {
        kind,
        id,
        body: Some(Box::new(body)),
    })
}

fn render_mesh(r: &mut Reader, things: &mut Things) -> Result<RenderMesh> {
    let v = r.rev.version();
    let mesh = r.resource(false)?;
    let bone_things = thing_array(r, things)?;
    let anim = r.resource(false)?;
    let anim_pos = r.f32()?;
    let anim_speed = r.f32()?;
    let anim_loop = r.bool()?;
    r.skip_f32(2)?;
    let editor_color = r.u32()?;
    let cast_shadows = r.i8()?;
    r.bool()?;
    let visibility_flags = r.i8()?;
    let poppet_render_scale = r.f32()?;
    if v < 0x34d {
        r.skip_f32(2)?;
    }
    Ok(RenderMesh {
        mesh,
        bone_things,
        anim,
        anim_pos,
        anim_speed,
        anim_loop,
        editor_color,
        cast_shadows,
        visibility_flags,
        poppet_render_scale,
    })
}

fn decoration(r: &mut Reader, things: &mut Things) -> Result<Decoration> {
    let idx = r.other_count("RENDER_MESH");
    let tag = r.tag("RENDER_MESH", |r| Ok(r.next_other("RENDER_MESH")))?;
    let render_mesh = match tag {
        Tag::New(_) => {
            r.enter()?;
            let m = render_mesh(r, things)?;
            r.leave();
            things.render_meshes.insert(idx, m.clone());
            Some(m)
        }
        Tag::Seen(crate::serial::Obj::Other(_, i)) => things.render_meshes.get(&i).cloned(),
        _ => None,
    };
    let offset = r.m44()?;
    let parent_bone = r.i32()?;
    let parent_tri_vert = r.i32()?;
    let bary_u = r.f32()?;
    let bary_v = r.f32()?;
    let angle = r.f32()?;
    let on_costume_piece = r.s32()?;
    let earth_decoration = r.s32()?;
    let scale = r.f32()?;
    let placed_by = r.i16()?;
    let reversed = r.bool()?;
    let play_mode_frame = r.i32()?;
    let plan_guid = r.guid()?;
    Ok(Decoration {
        render_mesh,
        offset,
        parent_bone,
        parent_tri_vert,
        bary_u,
        bary_v,
        angle,
        on_costume_piece,
        earth_decoration,
        scale,
        placed_by,
        reversed,
        play_mode_frame,
        plan_guid,
    })
}

fn decal(r: &mut Reader) -> Result<Decal> {
    let v = r.rev.version();
    let texture = r.resource(false)?;
    let u = r.f32()?;
    let vv = r.f32()?;
    let xvec = [r.f32()?, r.f32()?];
    let yvec = [r.f32()?, r.f32()?];
    let color = r.i16()?;
    let kind = r.i8()?;
    let metadata_index = r.i16()?;
    let num_metadata = if v <= 0x3ba { Some(r.i16()?) } else { None };
    let placed_by = r.i16()?;
    let play_mode_frame = r.i32()?;
    let scorch_mark = r.bool()?;
    let plan = if v >= 0x34c {
        r.resource(true)?
    } else {
        r.guid()?.map(|g| Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(g),
            sha1: None,
        })
    };
    Ok(Decal {
        texture,
        u,
        v: vv,
        xvec,
        yvec,
        color,
        kind,
        metadata_index,
        num_metadata,
        placed_by,
        play_mode_frame,
        scorch_mark,
        plan,
    })
}

/// Returns the main decal array and the total decal count (main plus costume arrays).
fn stickers(r: &mut Reader) -> Result<(Vec<Decal>, usize)> {
    let v = r.rev.version();
    let n = r.count(1)?;
    let mut decals = n;
    let mut main = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        main.push(decal(r)?);
    }
    let costumes = r.count(1)?;
    for _ in 0..costumes {
        let n = r.count(1)?;
        decals = decals.saturating_add(n);
        for _ in 0..n {
            decal(r)?;
        }
    }
    if v <= 0x3ba {
        let n = r.count(4)?;
        r.bytes(n * 4)?;
    }
    let n = r.count(1)?;
    for _ in 0..n {
        r.resource(false)?;
        r.resource(false)?;
        r.m44()?;
        r.skip_f32(6)?;
        if v > 0x39f {
            r.resource(false)?;
        }
    }
    Ok((main, decals))
}

fn level_settings(r: &mut Reader) -> Result<LevelSettings> {
    let mut s = LevelSettings {
        sun_position: r.v3()?,
        sun_position_scale: r.f32()?,
        sun_color: r.v4()?,
        ambient_color: r.v4()?,
        sun_multiplier: r.f32()?,
        exposure: r.f32()?,
        fog_color: r.v4()?,
        fog_near: r.f32()?,
        fog_far: r.f32()?,
        rim_color: r.v4()?,
        rim_color2: r.v4()?,
        baked_ao_bias: 0.,
        baked_ao_scale: 0.,
        presets: 0,
        backdrop_mesh: None,
    };
    r.skip_f32(2)?;
    s.baked_ao_bias = r.f32()?;
    s.baked_ao_scale = r.f32()?;
    r.skip_f32(2)?;
    r.f32()?;
    r.skip_f32(3)?;
    Ok(s)
}

fn camera_tweak(r: &mut Reader) -> Result<()> {
    let v = r.rev.version();
    if v < 0x37e {
        r.v3()?;
        r.v4()?;
    }
    r.v4()?;
    if v < 0x37e {
        r.f32()?;
    }
    r.f32()?;
    r.i32()?;
    r.s32()?;
    r.bool()?;
    r.bool()?;
    r.bool()?;
    r.i32()?;
    if v < 0x37e {
        r.u8()?;
    }
    r.i8()?;
    r.i32()?;
    r.bool()?;
    r.i8()?;
    r.i32()?;
    if v < 0x35a {
        r.bool()?;
    }
    if v > 0x359 {
        r.i32()?;
    }
    r.bool()?;
    r.skip_f32(4)?;
    r.bool()?;
    r.bool()?;
    if v > 0x369 {
        r.bool()?;
    }
    if v > 0x395 {
        r.bool()?;
    }
    if v > 0x396 {
        r.bool()?;
    }
    if v > 0x37d {
        let n = r.count(33)?;
        for _ in 0..n {
            r.v4()?;
            r.v3()?;
            r.f32()?;
            r.bool()?;
        }
    }
    Ok(())
}

fn physics_tweak(r: &mut Reader, things: &mut Things) -> Result<()> {
    let v = r.rev.version();
    r.skip_f32(2)?;
    r.v3()?;
    r.v3()?;
    r.v3()?;
    r.skip_f32(3)?;
    r.bool()?;
    let configuration = r.u32()?;
    if v < 0x327 {
        thing_ref(r, things)?;
    }
    r.bool()?;
    r.i32()?;
    r.wstr()?;
    r.u8()?;
    r.i32()?;
    r.bool()?;
    r.bool()?;
    r.skip_f32(2)?;
    r.bool()?;
    if v > 0x381 {
        r.f32()?;
    }
    r.bool()?;
    if v > 0x36a {
        r.f32()?;
        r.bool()?;
        r.s32()?;
        r.f32()?;
        r.bool()?;
    }
    if v > 0x3b8 && configuration == 0xd {
        r.resource(false)?;
        r.f32()?;
        if v < 0x3c4 {
            r.u8()?;
        }
        r.i8()?;
        r.i8()?;
        r.v3()?;
        r.bool()?;
        r.m44()?;
        r.f32()?;
        if v > 0x3c4 {
            r.bool()?;
        }
    }
    if configuration == 0xe && v > 0x3e3 {
        r.f32()?;
        r.i32()?;
        r.skip_f32(2)?;
        r.i32()?;
        r.skip_f32(2)?;
        r.bytes(4)?;
    }
    Ok(())
}

fn shape(r: &mut Reader, things: &mut Things) -> Result<Shape> {
    let v = r.rev.version();
    let raw = r.u32()? as usize;
    let requires_z = r.bool()?;
    let size = if requires_z { 12 } else { 8 };
    match raw.checked_mul(size) {
        Some(b) if b <= r.remaining() && raw <= r.limits.max_array => {}
        _ => return r.invalid("polygon vertex count exceeds remaining bytes"),
    }
    let mut vertices = Vec::with_capacity(raw);
    for _ in 0..raw {
        vertices.push(if requires_z {
            r.v3()?
        } else {
            let [x, y] = r.v2()?;
            [x, y, 0.0]
        });
    }
    let loops = r.int_vector()?;
    let material = r.resource(false)?;
    let old_material = r.resource(false)?;
    let thickness = r.f32()?;
    let mass_depth = r.f32()?;
    let color = if v <= 0x389 {
        ShapeColor::Vector(r.v4()?)
    } else {
        ShapeColor::Packed(r.u32()?)
    };
    let brightness = r.f32()?;
    let bevel_size = r.f32()?;
    let com = if v >= 0x38e { Some(r.m44()?) } else { None };
    let behavior = r.u32()?;
    if v < 0x38a {
        r.v4()?;
    } else {
        r.u32()?;
    }
    r.f32()?;
    if v <= 0x345 {
        r.s32()?;
    } else {
        r.u16()?;
    }
    r.i32()?;
    if v <= 0x367 {
        r.i32()?;
    } else {
        r.i8()?;
    }
    let flags = if v <= 0x345 {
        i16::from(r.i8()?)
    } else {
        r.i16()?
    };
    // ContactCache: array(Contact) of {reference(PShape), i8 flags}, then two bools.
    let contacts = r.count(2)?;
    for _ in 0..contacts {
        let idx = r.other_count("SHAPE");
        let tag = r.tag("SHAPE", |r| Ok(r.next_other("SHAPE")))?;
        if let Tag::New(_) = tag {
            // First reached through a contact: read inline here; a Thing that names the
            // same tag later shares this body.
            r.enter()?;
            let s = shape(r, things)?;
            r.leave();
            things.shapes.insert(idx, s);
        }
        r.i8()?;
    }
    r.bool()?;
    r.bool()?;
    if v >= 0x3bd {
        r.bytes(3)?;
    }
    if v >= 0x3c1 {
        r.bytes(2)?;
    }
    if v >= 0x3e2 {
        r.bool()?;
    }
    Ok(Shape {
        vertex_count: raw,
        requires_z,
        vertices,
        loops,
        material,
        old_material,
        thickness,
        mass_depth,
        color,
        brightness,
        bevel_size,
        com,
        behavior,
        flags,
        contacts,
    })
}
