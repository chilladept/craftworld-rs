# Level scripts, part 21: script names, the pod Thing, the avatar (2026-10-13, D270)

Stage 6 of `docs/rscript-plan.md` (the Pod), first PR. "The Pod" is two levels:
`gamedata/levels/pod/pod.bin` (the shell: the pod box, the pod computer, a checkpoint;
world script `Pod`, a trigger script `Trigger_Pod`) and
`gamedata/lbp2/pod/levels/pod_planets.bin` (the planets view: world script
`PlanetsWorld`, `Earth`, `OrbitingMoon`). The game has both loaded at once; the runtime
plays each alone. The world script of pod.bin was the one Thing outside TriggerAirStone
that still stopped on a fault. This part binds what its create function and its
per-tick update ask for, so the fault goes because the script gets the Thing it asks
for.

### Why the Pod stopped (FOUND in the script listing, MEASURED in the D269 runs)

`Pod.OnCreate__` asks `Thing.FindThing` for the Pod's checkpoint by name and then loops
for the life of the level: its own update, then a yield. So the update runs inside the
create call. Each update ends with `SetPodTransition__b`, which takes
`Thing.GetPodThing(true)` and, without a null test, sets that Thing's shape colour and
reads its PShape. Both natives were unbound and answered null. The colour setter does
nothing for a null Thing (part 19); the read of the null Thing's PShape was the fault
("null thing" at `Pod.SetPodTransition__b+45`), in the create call, so the Thing never
ran again. The game never answers null there: the level has a Thing with the pod mesh.

### What the game does (FOUND, 01.33 executable and script listings)

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Thing.FindThing__Q7StringA` | static `Thing (StringA)` | 0x61b67c -> 0x753880 -> 0xd3360 | Walks a list of the PWorld of the script's world. An entry has a Thing and a byte string; the C library's string compare tests it against the argument: the whole string, every byte, case sensitive. The first equal entry gives its Thing; none, or no world: null. One call site in 426 scripts (`Pod.OnCreate__`) |
| `Thing.GetPodThing__b` | static `Thing (bool)` | 0x6011d8 -> 0x7553b8 -> 0xd32dc | Walks a list of the PWorld of the level the game is in. An entry has a Thing and a resource, and the resource's GUID is compared with two constants. The first entry with 0x2f6c (the pod mesh) gives its Thing. An entry with 0x23959 (the platform mesh) gives its Thing when the bool is true and ends the search with null when it is false. None: null. One script call site (`Pod.SetPodTransition__b`, with true) |
| `Game.GetAvatarFromPlayerID__i` | static `Thing (s32)` | 0x600ed0 -> 0x5e79b8 -> 0x18ec14 | The core of `GetYellowheadFromPlayerNumber` (part 13): the Thing of the first entry of a PWorld list whose number is the argument; null with none |
| `Resource.LoadMeshByFilename__g` | static `Resource (guid)` | 0x617df4 -> 0x50aa08 | Asks the resource system for the mesh with that GUID and hands the script a resource handle, as `LoadTextureByFilename__g` does for a texture |
| `Thing.SetRenderMeshMesh__Q5ThingQ8Resource` | static `void (Thing, Resource)` | 0x642834 -> 0x755580 | Nothing for a null Thing or one without a PRenderMesh. Else the PRenderMesh's mesh becomes the resource when the resource's type is a mesh, and none when it is not |
| `FileList.AsGUID__i`, `FileList.AsInt__g` | static `guid (s32)`, `s32 (guid)` | 0x6016a8, 0x616c44 | The value unchanged, both ways |
| `PodComputer.GetPlanetsBackgroundGUID__` | static `guid ()` | 0x601644 -> 0x503358 -> 0x3fcbb4 | One field of the planets level's PWorld; 0x1d054 while that level is not loaded |
| `Game.IsKioskBuild__` | static `bool ()` | 0x5f8588 -> 0x3aa5c | The constant false in this executable |
| `Game.IsStartMenuOpen__` | static `bool ()` | 0x5f8644 -> 0x3ef1a4 | One word of a UI object is not 0 |
| `Pod.IsTakingPhoto__Q5Thing` | static private `bool (Thing)` | 0x644f3c -> 0x502f40 | False for a null Thing or one without a PYellowHead; else an answer from that player's Popit |
| `PodComputer.SetFadePodDecorations__f` | static `void (f32)` | 0x5fb36c -> 0x403ca4 | Takes `GetPodThing(true)` and works over a list of records on one of its parts with the float. The arithmetic was not read |
| `PodComputer.RequestHostUsePod__b` | static `void (bool)` | 0x5fbd04 -> 0x3fa9cc | Only in the player's own Pod: stores the request (use or stop) and the time and sends a message to the host. It does not set "in use" itself |

How the Pod's script uses them (FOUND in the listing). `OnCreate__`: the checkpoint by
name; when it is not null, its `Checkpoint.Active` is set. Each update: the party
leader's avatar; when there is one, a chain of tests (pod computer not in use, no
video, a progression flag, the avatar in the pod computer's zone, no photo being taken,
no start menu, a button press) that ends in `RequestHostUsePod(true)`; then
`SetPodTransition`: a value moves toward 1 while the pod computer is in use and toward
0 while it is not, by 0.05 a tick (`Math.Delt`, 20 ticks); the pod Thing's shape colour
is the avatar's player colour (zero without an avatar) with alpha 1 minus the value;
the value goes to `SetFadePodDecorations`; the pod Thing's mesh is the pod's while the
value is below 1 and the platform's at 1 (`AsGUID`, `LoadMeshByFilename`,
`SetRenderMeshMesh`); a texture picked by the planets' backdrop
(`AsInt(GetPlanetsBackgroundGUID())`) goes to `SetPodTransitionTexture`.

INFERRED:

- The list `FindThing` walks is the level's PScriptName parts, and the list
  `GetPodThing` walks is the level's PRenderMesh parts, both in the order of the Things.
  The entries fit those parts (a Thing and a name; a Thing and the mesh field
  `SetRenderMeshMesh` writes), and pod.bin has one candidate for each call, so its
  answers do not depend on the order. One name is on two Things in 18 levels; no script
  asks for it.
- A null string finds nothing (the native hands the core a null pointer; what the
  compare does with it was not followed).
- A null resource clears a mesh, as a resource that is not a mesh does.
- `Game.IsStartMenuOpen__` and `Pod.IsTakingPhoto__` are false: nothing here opens the
  start menu and no player has a Popit.
- `GetAvatarFromPlayerID`: as for `GetYellowheadFromPlayerNumber`, the list is the
  PYellowHead parts and the number is the player number.

### The level reader

`lbp-level` kept only the length of a PScriptName. It now keeps the name:
`PartData::script_name` is a `ScriptName { stored, name }`, the stored byte count and the
bytes up to the first NUL (all of them when there is none; MEASURED on pod.bin: the
length, then the name, no NUL after it). The name is kept as bytes because the game
compares bytes; a report writes it as text. The bytes read are the same as before:
Toolkit parity stays 236 / 236 (4,180 / 4,180 scene Things).

### The runtime

- `ThingState::script_name` (the name's bytes) and `ThingState::mesh_guid` (the mesh the
  PRenderMesh shows: the level file's until a script sets another).
- New `lbp-runtime/src/pod.rs`: `LevelWorld::find_thing(name)`, `pod_thing(platform_too)`,
  `script_names()`, `planets_background()`, and `LevelWorld::pod` (`PodState`): the mesh
  handles `LoadMeshByFilename` handed out, call and hit counters, the last fade value,
  the `RequestHostUsePod` requests (`HostUsePodRequest`: the bool and the frame), the
  planets' backdrop (none: the runtime plays one level, so the native answers 0x1d054).
- `pod_thing` reads the mesh the PRenderMesh shows now, so it follows
  `SetRenderMeshMesh`: once the script has swapped in the platform, `GetPodThing(true)`
  still finds the Thing and `GetPodThing(false)` does not.
- A resource handle is the GUID (part 2); a handle is a mesh when `LoadMeshByFilename`
  handed it out.
- A display learns of a new mesh: `DisplayChange::mesh`, `ThingDisplay::mesh`,
  `DisplaySummary::mesh_swapped`, a fourth `marks` counter. Nothing draws by it yet.
- Natives 225 -> 238: the eleven of the table above that are not recorded only, and
  `SetFadePodDecorations__f` and `RequestHostUsePod__b` recorded only (`RECORDED_ONLY`
  21 -> 23; the world keeps the float and the requests). `IsStartMenuOpen__` and
  `IsTakingPhoto__Q5Thing` are state queries (`STATE_QUERIES` 24 -> 26); `IsTakingPhoto`
  answers false for a null Thing without asking the world.
- `level_scripts`: a `pod` block per level and in `play_all` (Things with a script name
  and their levels, levels with one name on two Things, `FindThing` and `GetPodThing`
  calls and hits, mesh sets and changes, fade calls); a level played alone also lists
  its named Things with the part members scripts touched, the pod Thing, its mesh and
  shape colour.

### Left unbound, and why

- `PodComputer.SetPodTransitionTexture__g` (0x617428 -> 0x4015e8): the core asks for a
  resource by the GUID; what it does with it was not read (the decompiler dropped the
  rest of the function as unreachable). Its effect is not established, so it is counted
  as unbound: one call a tick in pod.bin.
- `Game.CurrentLevelIsOurPod__` (0x5f8b38 -> 0x5e8024): one byte of the game object.
  What sets the byte was not read, so the answer for a level played alone is not
  established. No measuring run calls it.
- `Native.GetPlayerColour__Q5Thing` (0x64a4dc -> 0x6d88b0 -> 0x139308): from the
  Thing's PYellowHead; the colour function was not read. Called in pod.bin once there is
  an avatar (3 and 5 calls in the runs with a visiting player); the default is a zero
  colour, which is also what the script uses without an avatar.
- `Native.GetIsInPadZone__Q5Thing`, `Input.IsJustClickedByID__iiQ6String`: the last test
  of the first and the button table of the second were not read. Not reached: the
  progression flag before them in the chain is not set on a fresh profile.
- Video, voice-over and the progression flags (the intro of pod_planets): the next PR.

### PC run

`level_scripts --all --play=60`, 378 levels, 13,547 scripted Things, plain / with
`--visit-triggers` / with `--visit-triggers --pulse-switches`, inputs as in part 11.
Outputs under ignored `local-analysis/`: `d270-play-all-{novisit,visit,visit-pulse}.json`
(D269: `d269-play-all-*`).

| 60 ticks per level | D269 | D270 |
|---|---|---|
| Things stopped by a fault | 4 / 4 / 4 | 3 / 3 / 3 |
| Fault kinds | 2 / 2 / 2 | 1 / 1 / 1 |
| Levels with a fault | 2 / 2 / 2 | 1 / 1 / 1 |
| Natives in the table | 225 | 238 |
| Unbound natives called | 48 / 81 / 81 | 44 / 78 / 78 |
| Unbound calls | 62,000 / 865,777 / 872,549 | 61,483 / 865,266 / 872,042 |
| Native calls | 571,776 / 5,363,790 / 5,423,370 | 573,134 / 5,365,247 / 5,424,893 |
| Recorded-only calls | 12,267 / 90,963 / 96,877 | 12,328 / 91,027 / 96,943 |

- The three Things that remain are the TriggerAirStone instances of one level:
  `Thing.GetCOM__` has no centre of mass to give (`world does not provide centres of
  mass`, in `OnCreate__`). That needs the physics body of a Thing, which the level
  world does not have.
- Gone from the unbound list, in all three runs (calls / levels): `FileList.AsInt__g`
  289 / 28, `Resource.FindGuidByPath__Q7StringA` 285 / 27,
  `Game.GetAvatarFromPlayerID__i` 2 / 1, `Thing.FindThing__Q7StringA` 1 / 1,
  `Thing.GetPodThing__b` 1 / 1. `FindGuidByPath` is not bound: the speech-bubble script
  looks its dialogue file up by path only while `AsInt` of the stored GUID is 0, which
  it was for every bubble while `AsInt` was unbound. With the real number no bubble
  in these runs makes the lookup.
- New on the list: `PodComputer.SetPodTransitionTexture__g` 61 / 64 / 66 calls in 1
  level, and with a visiting player `Native.GetPlayerColour__Q5Thing` 3 / 5 calls in 1
  level. Both are the Pod's update, which now runs. Nothing else on the list changed.
- Script names: 173 Things in 66 levels have a PScriptName; 141 names are not empty
  (7 distinct names in 56 levels, from the scoping census); 18 levels have one name on
  two Things. `FindThing`: 1 call in 1 level, 1 found (the Pod's checkpoint).
  `GetPodThing`: 61 / 64 / 66 calls, all found; 1 level of 378 has a pod Thing.
- Display: one more colour change marked (215, was 214) and one more shape colour
  unlike the level file (103 Things in 102 levels, was 102 in 101): the pod's shape,
  which the script makes opaque black while there is no avatar. Mesh changes 0: the
  script sets the pod's own mesh again each tick.
- Messages, global settings, the camera, trigger events and switch reactions are equal
  to D269.

The two Pod levels alone (`d270-pod-{60,600,60-visit-pulse}.json`,
`d270-planets-{60,600}.json`):

| Level, ticks | Before | Now |
|---|---|---|
| pod.bin, 60 and 600 | create: 0 done, 1 fault; 1 Thing stopped on tick 0; "19" written; nothing runs after | create: 0 done, 1 yielded, 0 faults; the Pod's script is resumed on every tick (60 of 60, 600 of 600) and never stopped; `FindThing` 1 call, 1 found (Thing 4, whose `Checkpoint.Active` is now true); `GetPodThing` 61 / 601 calls, all Thing 2; the pod's shape colour (0, 0, 0, 1); its mesh set to the pod mesh each tick, 0 changes; fade 0.0; unbound: `SetPodTransitionTexture` 61 / 601 |
| pod.bin, 60, visit and pulse | the same fault | 0 faults; `GetPodThing` 66 calls, all found; the trigger's Enter and Leave run once each; unbound: `SetPodTransitionTexture` 66, `GetPlayerColour` 5 |
| pod_planets.bin, 60 and 600 | no fault; the intro thread starts on tick 1 and is still running at the end | unchanged: the thread runs its 451 ticks of camera zoom and then waits on the video handle and the voice-over (4 unbound natives, 152 calls at 600 ticks: `Video.StartVideo` 1, `Audio.RequestVoiceOver` 1, `Profile.HasPlayedLBP1AndUnlockedPod` 1, `Audio.IsVoiceOverPlaying` 149). That is the next PR |

So pod.bin now gets as far as the game's own idle state: the checkpoint is active, the
pod is drawn with its mesh and an opaque black shape colour (no avatar), the transition
stays at 0, and the script waits for a player to use the pod computer, which needs the
progression flag the intro sets in the other level.

### Not done

- The Pod's change-over is not drawn: the mesh swap reaches the display list, but the
  viewer does not follow it, and the fade and the transition texture are not applied.
- One session for both Pod levels (the flag the intro sets in pod_planets opens the
  engage test in pod.bin): a later PR.
- The viewer: nothing changed in this PR.

### Verification

`cargo test`: `lbp-level` (new `script_name_keeps_the_name`: a name without a NUL and
with a byte outside ASCII, an empty name, a lone NUL, a NUL inside, the name as text in
a report; the existing shared-shape test now reads the name), `lbp-script-vm` (new
`d270_pod_natives_check_their_arguments_and_ask_the_world`: the thirteen natives in a
world without the state, null Things and a null string, the identities, wrong argument
types, the calls handed to a world; the table count 238), `lbp-runtime` 58 unit and 43
world tests. New world tests, all on made-up levels and names:
`d270_things_are_found_by_their_script_name` (two Things with one name: the first wins;
another case, a prefix and a longer string find nothing; the empty string; a null
string; a level without a name), `d270_the_pod_thing_follows_the_mesh_and_the_avatar_is_the_sackboy`
(the pod mesh alone, neither mesh, the platform before the pod for true and for false;
`SetRenderMeshMesh` then `GetPodThing`; a texture handle, a zero handle and a null
object clear the mesh; a null Thing and a Thing without a PRenderMesh; the display list
and summary; the avatar with and without a Sackboy; the backdrop, the fade and the
requests) and `d270_a_script_swaps_the_pod_mesh_when_its_transition_ends` (a made-up
script shaped like the change-over: 19 ticks of `Math.Delt` with the pod mesh, the
platform mesh on tick 20 when the value lands on 1, and the pod mesh back on the first
tick below 1). fmt, the CI clippy line, the CI test set and the Python tests pass.
Because the runtime's display types changed: `cargo check` of both viewers and `cargo
clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass; the Bevy viewer's tests
pass but the one that fails on main (`sackboy_play::tests::input_script_steps`). The
viewer was not started. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (reader and runtime feature).
