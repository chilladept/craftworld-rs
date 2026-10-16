# Level scripts, part 25: Earth and Moon by their own scripts, the fog bias, the video update (2026-10-13, D274)

First PR of the "Pod picture" follow-up to stage 6. The planets level of the Pod
(`gamedata/lbp2/pod/levels/pod_planets.bin`) has two planets, the Earth and the Moon, each
with a script. Their per-frame function was not called (part 6: its arguments were not
established), so they stayed where the level file has them, which is not where the game
shows them. Under `--preview-lighting` the viewer's fog washed the played level out
(part 23). And the place of the pod computer's per-frame video call was not known
(part 22). All three were read in the executable for this PR. Two natives are new
(255 -> 257).

### What the game does (FOUND, 01.33 executable and script listings)

The planets. Thing 1 of the level is "earth2", class `Earth`, and Thing 4 is "moon2",
class `OrbitingMoon`; both derive from `PlanetThing`, and each adds one function, a
`GetPosition__` that returns a constant place.

- The engine's planets object has a per-frame function (0x4381d8). Near its end it goes
  through its list of planets, and for each one calls the script function
  `Update__rb(v4, bool) -> bool` (0x42115c) with a vector and a bool. After the last
  planet it calls the function that ends in the world script's `UpdatePlanetsView`
  (part 6). So in one frame every planet runs before the world script's entry.
- The bool is 1 when a flag of the planets object is not 0, else a byte of the planet.
  A true result of the call sets that byte: "arrived".
- The vector (0x421708) is a point of the planet. When an entry is selected on the
  planet it is that entry's place on it, else a vector the planet object keeps itself.
  Every path that picks another entry clears "arrived".
- The script function, in our words: it takes the lengths of the three axes of the
  matrix the planet had when it was created (times a member that is 1.0 and that no
  script writes), and the place from the virtual `GetPosition__`. The vector goes
  through the size matrix; from the result it takes a target spin (`-atan2(x, z)`,
  moved by whole turns to within half a turn of the spin it has) and a target pitch
  (`atan2(y, ..)`, kept within a quarter turn). While the bool is false and it has run
  before, spin and pitch ease toward the target (`Math.Smooth2D`); else they are the
  target at once. The Thing's matrix is then a turn by its spin offset, the pitch and
  the spin, times the size matrix, at the place. It returns whether spin and pitch are
  both within 0.01 of the target.
- So the turn the level file stores for a planet is dropped, and its place is a
  constant of its class: (-1250, 0, 0) for the Earth, (2000, 0, 0) for the Moon.

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Math.transform__mr` | static `v4 (m44, v4)` | 0x62ae30 -> 0x5ee288 | Column 0 times x plus column 1 times y plus column 2 times z plus column 3 times w |
| `Math.ModAngleToWithinPI__f` | static `f32 (f32)` | 0x5fcf60 -> 0x5eebb0 | With `t = (a + pi) / (2 pi)`: `(t - r(t)) * 2 pi - pi`, `r` a rounding call through a stub |

The fog. The three arguments of `Cam.SetDOFAndFog__fff` in the world script are the
current view's depth-of-field pair and its fog bias, each times 2: (80000, 100000,
20000) on every tick of the idle view. MEASURED before (P2p, P2t): the closed Pod's fog
starts at depth 25810 and the level's own fog near is 5810. So the start is the fog near
plus the script's bias, and the span stays.

The video update. `PodComputer.UpdateVideo__` (part 22) is the last step of the pod
computer's draw function (0x404598, a slot of that object's table). Its one caller is
the function that writes a frame's draw commands (0x31e820, at 0x31fa38): after the
scene, when a byte of the render state is set and a pod computer exists. The byte is set
when the render state is made. So the call comes once a rendered frame, after that
frame's update, and is not tied to the Things' updates.

MEASURED (level and mesh data, this PR's scoping): the planets' stored axis lengths are
0.9610 (Earth) and 2.1121 (Moon), and the inverse bind of bone 0 of their meshes scales
by 1.4052 and 0.7312. The products, 1.3504 and 1.5444, are the sizes read from the
running game before (P2k: 1.3505 and 1.5444, at (-1250, 0, 0) and (2000, 0, 0)). So the
menu places of P2k are the script's own output.

INFERRED:

- The rounding call of `ModAngleToWithinPI` rounds down.
- The argument registers of `Update__rb` (+16 the vector, +32 the bool), from its
  argument rows. The flag of the planets object is 0.
- That a virtual call on a Thing goes to the class of the script the Thing carries. The
  VM's code for it was not read. It is what the placement above needs: with the call
  going to `PlanetThing`, both planets land on (0, 0, 0).
- A tick is a rendered frame, and the video call is the last script call of a tick.

Not established: the planet's own vector while nothing is selected (a field written in a
large per-frame function that was not read); who else clears "arrived"; the fog bias and
the depth-of-field pair of the views other than the idle one; the frames in which the
render state's byte is clear; what the first script call of the pod computer's draw
function is.

### What the runtime does

`lbp-script-vm`:

- `Math.transform__mr` and `Math.ModAngleToWithinPI__f` (`mod_angle_to_within_pi`: from
  -pi, included, up to pi).
- The virtual call on a Thing (`CALLVsp`) now dispatches on the Thing's own script:
  `Host::thing_script` gives the GUID of the script a Thing carries (`World::thing_script`,
  the level world's PScript GUID). The call goes to that class when it is, or derives
  from, the class the caller names; a null Thing, a Thing without a known script and a
  script of another family are called as named, as before. It was a plain call until now.

`lbp-runtime` (`lib.rs`):

- Two frame entries, classes `Earth` and `OrbitingMoon`, function `Update__rb`
  (`PLANET_UPDATE`). The function is found on the base class: a frame entry is looked up
  by the instance's class name, and the code through the superclass chain.
- `LevelScripts::planets`: per planet Thing a `PlanetFrame` with the point, "arrived" and
  the calls made. The bool passed is "arrived"; a true result sets it and a false one
  does not take it back. `LevelScripts::set_planet_point` gives a planet another point
  and clears "arrived" when the point is new. The state is the schedule's, not the level
  world's: the entries are called through the plain `Host`.
- `PLACEHOLDER_PLANET_POINT`, (0, 0, 1): the point while the host sets none. It is a
  placeholder, not a game value. It asks for no turn (spin 0, pitch 0), so it decides
  only which side of a planet faces the camera.
- Order: the planets' entries run just before the entry of the level's first
  `PlanetsWorld` instance, in Thing order among themselves, whatever their Thing
  numbers; in a level without that class they run in Thing order. Where the planets
  function stands among other Things' updates was not read.
- `LevelScripts::update_video`: `UpdateVideo__()` on the engine's PodComputer script
  object, once a tick, after the global-settings pass and before the camera's step.
  Nothing without the object or the function. A call that faults or yields is reported
  once (`StepReport::thread_faults`) and not made again. Part 22 did not add the call and
  guessed "before the Things"; it is new here, not moved. It changes nothing today: no
  video has a handle, and without a handle the script function does nothing.
- `StepReport`: `planet_updates`, `planets_arrived`, `video_updates`.

`level_scripts`: `play_all.pod` has four new counts; a level played alone lists each
planet after ticks 1, 226, 452 and the last (`planets`: point, arrived, calls, place,
axis lengths, the script's spin, pitch and spin offset).

### What the viewer does

`lbp-bevy-viewer --level-scripts` (`level_play.rs`, `lighting.rs`, `main.rs`).

- With `--pod-camera N` and no shell layer the planets level hangs under the root that
  follows its world Thing (part 23). The Thing map of part 19 is now on for that scene
  too, so each planet's model is moved inside the root by the matrix that takes its
  Thing from its stored place to the place its script gave it. A moved Thing's children
  are on the runtime's dirty list with it (the rule of part 13).
- The models stay placed by bone 0's inverse bind pose, so the drawn size is the
  script's axis length times the bind pose's scale.
- `follow_fog`: the third value of the scripts' last `SetDOFAndFog` goes to
  `LightingState::fog_bias` when it changes. Under `--preview-lighting` the viewer's
  distance fog then starts and ends that much further out (`Fog::biased`: both moved by
  the bias times the display scale, the span and the colour kept). One line says so, and
  the lighting lines name the moved range. The default lighting of level mode fogs in
  its shader with the measured Pod constants and is not touched.
- Without `--level-scripts` nothing changes: the bias is 0 and no map is built for the
  Pod. With the shell layer or `--pod-game-camera` the planets keep the viewer's own
  placement, as before.

Not done:

- Sticker decals of the planets. The scoping notes proposed to map them with the
  Thing. The scene has none: the three Things with stickers here have no shape, and
  the viewer projects stickers on shapes only (0 of 0 drawn). Nothing to map.
- The depth-of-field pair, and the bias in the default lighting's shader fog.
- A mapped Thing that is itself a child of the world Thing would be moved twice under
  the root. The planets are not children of it.

### Measured (PC, release build)

The planets level alone, 600 ticks, a fresh profile (`d274-planets-600.json`):

| After tick | Earth: place, axis length | Moon: place, axis length |
|---|---|---|
| level file | (-2295.7, 1132.2, 40.2), 0.9610 | (1332.1, 529.3, -51.6), 2.1121 |
| 1, 226, 452, 600 | (-1250, 0, 0), 0.9610 | (2000, 0, 0), 2.1121 |
| expected (the game, P2k, less the bind scale) | (-1250, 0, 0), 0.961 | (2000, 0, 0), 2.112 |

- Both planets answer "arrived" on their first call (spin 0, pitch 0, spin offset 0 on
  every sampled tick): the placeholder point asks for no turn.
- 1,200 planet calls and 600 video calls in 600 ticks. Entries without a function: 0
  (2 a tick before). No fault, no unbound native. The Earth's two child Things move
  with it. The camera poses and the world Thing equal part 23's.

`level_scripts --all --play=60`, 378 levels, 13,547 scripted Things, plain / with
`--visit-triggers` / with `--visit-triggers --pulse-switches`, inputs as in part 11.
Outputs under ignored `local-analysis/`: `d274-play-all-{novisit,visit,visit-pulse}.json`
(D273: `d273-play-all-*`).

| 60 ticks per level | D273 | D274 |
|---|---|---|
| Things stopped by a fault | 3 / 3 / 3 | 3 / 3 / 3 |
| Natives in the table | 255 | 257 |
| Natives called | 196 / 260 / 262 | 200 / 264 / 266 |
| Unbound natives called | 40 / 71 / 71 | 41 / 72 / 72 |
| Unbound calls | 61,419 / 862,919 / 868,592 | 61,420 / 862,920 / 868,593 |
| Native calls | 573,134 / 5,365,247 / 5,424,893 | 574,456 / 5,366,569 / 5,426,259 |
| Recorded-only calls | 12,389 / 91,091 / 97,009 | the same |
| Planet calls, levels | - | 120 / 120 / 124, 1 level |
| Pod computer video calls | - | 22,680 / 104,826 / 105,582 |
| Things moved far from their stored place, with a mesh | 1, 0 | 5, 2 |

- Two levels differ. The planets level: the planets' calls and their natives, five
  Things moved (the two planets, the Earth's two child Things; the world Thing was moved
  before). And one cutscene level (`cutscene_outro_theme6`): one call to a native that
  was not reached before, `Gooey.StartFrameNamed`, unbound. That is the virtual call on
  a Thing: a call there now reaches a subclass's function. The scoping notes expected
  the planets level only.
- Bound native calls go up by 1,321 / 1,321 / 1,365, which fits eleven a planet call and
  one more in the cutscene level (the per-level split of bound calls is not in the
  report).
- The video call runs once a tick in every level (the tool loads the PodComputer class
  for all of them) and calls no native.
- Everything else in the `play_all` block is equal.

Viewer, release build, 1280 x 720, no focus taken (started hidden, `--qa-capture`), the
planets level with `--pod-camera 1 --level-scripts --window-16-9 --preview-lighting
--qa-hide-ui` and `--level-scripts-hold-at` 30, 200, 440 and 600. The frames stay in
ignored `local-analysis/rscript/stage6/shots/` (`d274-*`).

- Printed: `Thing map (level scripts): 2 Things with a model of their own, 8 draw
  entities`; `Things at tick 30 (new scene): listed 2, ... placed 2 (8 entities)`; `Fog
  bias at tick 30: 20000.0 level units ...`; `fog moved out by the level scripts' fog
  bias 20000.0: linear 25804.6..78453.6 level units` (stored: 5804.6..58453.6). The
  camera and world Thing lines equal part 23's.
- Looked at, tick 600 (the camera at rest): the room is clear, with a light haze, and no
  longer washed out. The Earth is upright, a little left of the middle, its centre near
  (0.46, 0.48) of the frame; the Moon is right of it near 0.56 of the width, at the
  same height. Those are the places P2k read from the closed Pod in the game (the
  Earth near (0.46, 0.48), the Moon near 0.56 of the width). The lamp hangs between
  and above them; the table is at the right.
- Tick 200: the ceiling fills the upper half; the Earth and the Moon are left of the
  middle and lower, the same pair at the same spacing. Tick 30: the ceiling fills the
  picture and the Earth is cut by the lower edge.
- Tick 600 with the default lighting of level mode: the same framing, the planets at
  the same places. That frame is not equal to part 23's, as expected.
- Without `--level-scripts` the capture is equal byte for byte to part 23's.

Not seen: the motion (stills only); the frame at tick 440 was captured and not looked
at. Not compared: any frame against the game, side by side. The side of the Earth that
faces the camera comes from the placeholder point and is not claimed to be the game's;
the Moon's face was not compared either.

### Not modelled

- The game's point for a planet (its entries, the idle vector), so the planets do not
  turn, and the views of the pod computer's menus with their spin offsets.
- The shell layer under the script's camera; the pod's mesh swap in the scene; the two
  Pod levels in one viewer session.
- Depth of field and motion blur; the fog bias of other views (it comes from the
  script, so nothing is made up when it changes).
- A video system: the video call has nothing to update.

### Verification

`cargo test -p lbp-script-vm`: new `d274_planet_math_natives` (`transform` on a matrix
with a size and a place, w 0 and w 1, a turned matrix, argument checks;
`ModAngleToWithinPI` at 0, either side of half a turn, at half a turn, whole turns,
negative) and `virtual_calls_on_a_thing_dispatch_on_the_script_it_carries` (own class,
derived class, a class of another family, a script that is not loaded, an unknown
Thing, a null Thing; the plain call). `cargo test -p lbp-runtime`: 61 unit and 52 world
tests. New world tests: `d274_a_planet_gets_the_hosts_point_and_keeps_arrived` (a
derived class without a function of its own is called through its base; the
placeholder point; the planet before the world script; false until the script answers
true, true after, not taken back; a new point clears it; a level without the world
script), `d274_a_virtual_call_on_a_thing_goes_to_its_own_class` and
`d274_the_pod_computer_updates_its_video_after_the_things` (once a tick after the
Things, not in the Things' part alone, a fault reported once, no object, no function).
`lbp-bevy-viewer`: `a_planet_its_script_placed_is_drawn_under_the_world_thing_and_the_fog_bias`
and the fog bias in `p2l_lbp_shape_uses_raw_values_hemisphere_and_grazing_rim` (start
and end moved alike, span and colour kept, no bias no change). None needs game data.
fmt, the CI clippy line, the CI test set and the Python tests pass; `cargo check` of
both viewers and `cargo clippy -p lbp-bevy-viewer --all-targets -- -D warnings` pass; the
viewer's tests pass but the one that fails on main
(`sackboy_play::tests::input_script_steps`). Seen in the viewer window: stills at ticks
30, 200 and 600, and 600 with the default lighting. Nothing compared against the game in
RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime and viewer feature).
