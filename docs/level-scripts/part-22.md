# Level scripts, part 22: video, voice-over and progression flags (2026-10-13, D271)

Stage 6 of `docs/rscript-plan.md` (the Pod), second PR. The planets level of the Pod
(`gamedata/lbp2/pod/levels/pod_planets.bin`) plays an intro on a fresh profile: a script
thread that zooms the camera in, waits for a video and a voice-over line to end, and
then marks the intro as seen on the profile. The thread never ended here. This part
binds the natives it asks, so that it ends for the reason it ends in the game when no
video starts and no line plays.

### What a recorded request means here

Nothing in this part plays a video or a sound. `Video.StartVideo` and
`Audio.RequestVoiceOver` keep what the script asked for (the video's GUID, the line's
name, the flags, the frame) in `LevelWorld::media`, so that a later video or audio
system can take the request from there. The script is told what the game tells it when
nothing started: -1 for the video handle, false for the line. From then on "is a line
playing" is false and no video has a handle. The scripts go on as the game's scripts do
after a video that could not start; a player of the viewer sees and hears nothing of
the intro but its camera move.

### Why the intro never ended (FOUND in the script listing, MEASURED in the D270 runs)

The planets level's world script starts the intro thread on tick 1 while a progression
flag is clear. The thread asks the pod computer's script to start a video, and that
script keeps `Video.StartVideo`'s answer as its video handle. It asks for a voice-over
line. It moves a zoom value from 0 to 1 over 451 ticks, one yield a tick; the world
script reads the value for the camera. Then it yields while `Audio.IsVoiceOverPlaying()`
is true or the pod computer's handle is not -1. Last it sets progression flags, and the
world script, which tests one of them, goes back to its idle state.

`Video.StartVideo` was unbound and its default answer is 0. The script takes any handle
but -1 as "a video is playing", and nothing set the handle back, so the wait never
ended: the thread was still there after 600 ticks (149 defaulted `IsVoiceOverPlaying`
calls: ticks 452 to 600). Behind it was a second trap: `SetLBP2GameProgressionFlag` was
unbound too, so an intro that ended would have been started again on the next tick,
for ever.

### What the game does (FOUND, 01.33 executable)

| Native | Declared | Handler -> core | What it does |
|---|---|---|---|
| `Video.StartVideo__Q5Thinggb` | static `s32 (Thing, guid, bool)` | 0x642340 -> 0x75672c -> 0x67c620 | Tries to start the video (0x67c45c). Started: the video's handle. Not started: -1 |
| `Video.UpdateVideo__ib` | static `bool (s32, bool)` | 0x5fb5cc -> 0x7566ec -> 0x67c22c | Looks for the video with that handle (0x679438). None: false. Else it steps the video, and returns false once it has ended |
| `Video.StopVideo__i` | static `void (s32)` | 0x5f5434 -> 0x67a3d8 | Stops the video with that handle, if there is one |
| `Audio.RequestVoiceOver__Q7StringAbb` | static `bool (StringA, bool, bool)` | 0x619430 -> 0x7324fc -> 0x2df200 -> 0x2de6b8 | False, and nothing starts: while voice-over is switched off (one global); for a line the profile has heard when the second bool is false; while a line plays that the first bool does not let it interrupt. Else the line's sound is started |
| `Audio.IsVoiceOverPlaying__` | static `bool ()` | 0x5f8fdc -> 0x2de218 | False unless voice-over is on, the kept sound handle is valid and its sound is playing |
| `Audio.StopVoiceOver__` | static `bool ()` | 0x5f9018 -> 0x2de498 | Stops the line; true, always |
| `Audio.SetMixOverlay__Q7StringAf` | static `bool (StringA, f32)` | 0x6194f4 -> 0x73255c -> 0x2cdb7c | False while the audio system object (one global pointer) is null; else the audio system's answer (not read) |
| `Audio.RemoveMixOverlay__Q7StringAf` | static `bool (StringA, f32)` | 0x619494 -> 0x73252c -> 0x2cdaa0 | The same test, then the audio system's answer |
| `Profile.SetLBP2GameProgressionFlag__i` | static `void (s32)` | 0x5f56cc -> 0x73fe44 -> 0x163a4c -> 0x15d22c | For each local profile: `word |= argument` on one 32-bit word of the profile; when that changed the word, the profile is marked changed |
| `Profile.IsLBP2GameProgressionFlagSet__i` | static `bool (s32)` | 0x5f6860 -> 0x73fa20 -> 0x161eac -> 0x15d2a8 | True when every bit of the argument is set in that word. Also true whatever the word while a debug byte is set, and for the arguments 1 and 2 while a build-type word has one value (0x3ac38) |
| `Profile.HasPlayedLBP1AndUnlockedPod__`, `...UnlockedMoon__` | static `bool ()` | 0x5f6958 -> 0x740084, 0x5f68e0 -> 0x7439dc | False without a loaded profile record in one of two states; else one byte the record answers with |

So the argument of the two flag natives is a bit mask, not a flag number. The intro
thread sets the mask 3 (and more bits when `HasPlayedLBP1AndUnlockedMoon` holds); the
world scripts of both Pod levels test the mask 2. The old "flag 2" of parts 6 and 7 is
bit 1 of the word.

What a script waits on is never the Video class itself: it is the pod computer's own
handle member, which only the pod computer's script function `UpdateVideo__` sets back
to -1, from `Video.UpdateVideo`'s false.

### `PodComputer.UpdateVideo__`: who calls it is FOUND, when is not

The scoping notes proposed to call this script function on the engine's PodComputer
object once a tick. Read again for this PR:

- FOUND, who: the engine holds the function's name and calls it through a wrapper
  (0x624b70) from 0x50373c, which takes the pod computer's script object from a holder.
  0x50373c is called, without a condition, at the end of 0x404598.
- NOT FOUND, when: 0x404598 has no direct caller. It is reached through one slot of a
  table of functions (the only reference to it is data), and which engine code calls
  that slot, and how often, was not traced. The function builds a view, calls another
  slot of its own table and a draw routine before it reaches the call, which fits a
  per-frame draw of the Pod, but that is INFERRED.

So the call is not added. Nothing depends on it here: every handle is -1 from the
start, and with a handle of -1 the script function only sets -1 again.
`Video.UpdateVideo__ib` is bound all the same (false: no video has a handle), so that
a later schedule entry finds it.

### INFERRED

- `RequestVoiceOver` answers true when a line starts (the end of its core was not
  read). Not used: here it always answers false, and the callers the runs reach drop
  the answer.
- The two bools of `RequestVoiceOver` are "interrupt the line that plays" and "play
  even when the profile has heard it". They are kept as passed.
- `HasPlayedLBP1AndUnlockedPod` and `...Moon` are false: a profile without save data
  of the first game, as the fresh profile of the progression flags. FOUND is only that
  the answer is false without a loaded profile record.
- The debug byte and the build test of `IsLBP2GameProgressionFlagSet` do not hold in a
  played retail game.
- The name "the Pod intro was seen" for the mask 2. FOUND is that the intro sets it and
  that the world scripts of both Pod levels test it.

### The runtime

- `LevelWorld::progression_flags` is one `u32` (it was a set of numbers).
  `World::progression_flag(mask)` is "every bit set"; new `World::set_progression_flags`.
  `level_scripts --flags=` and the viewer's `--level-scripts-flags` take bit masks,
  decimal or hexadecimal after `0x`, separated by commas and ORed
  (`lbp_runtime::parse_progression_flags`). `--flags=2` means what it meant: the mask 2.
- New `lbp-runtime/src/media.rs`: `LevelWorld::media` (`MediaState`): the video
  requests (`VideoRequest`: Thing, GUID, bool, frame) and the voice-over requests
  (`VoiceOverRequest`: name, the two bools, frame), each list at most 256 with a count
  of the rest; the stop calls; the mix overlay calls by name.
- Natives 238 -> 249. Three are state queries (`STATE_QUERIES` 26 -> 29:
  `IsVoiceOverPlaying`, the two `HasPlayedLBP1...`), all false. `StopVideo` is recorded
  only (`RECORDED_ONLY` 23 -> 24). Five ask for something to play and return an answer:
  they are the new list `natives::RECORDED_REQUESTS`, with the answer each gives
  (`StartVideo` -1, `RequestVoiceOver` false, `StopVoiceOver` true, `SetMixOverlay`
  and `RemoveMixOverlay` false). `UpdateVideo` is false for every handle.
  `SetLBP2GameProgressionFlag` is a real binding.
- `level_scripts`: a `media` block per level and in `play_all` (requests and their
  levels, threads started and done), `progression_flags` (the word at the start and at
  the end, and the ticks that changed it), `thread_ticks` (the ticks on which a script
  thread started or ended) and `recorded_request_natives` in `play_all`.
- The viewer: only the parse of `--level-scripts-flags`, which the changed type forced.

### Left unbound, and why

- `Video.PressedStartButton__` (0x5f61a4 -> 0x756520): FOUND that it looks for a button
  bit over the pads and uses the press up. Which of the pad's words holds the bit, and
  so how it maps to `PadState`, was not read. Not reached by the intro.
- `PodComputer.UpdateVideo__` as an engine call each tick: see above.
- From part 21, unchanged: `PodComputer.SetPodTransitionTexture__g`,
  `Game.CurrentLevelIsOurPod__`, `Native.GetPlayerColour__Q5Thing`,
  `Native.GetIsInPadZone__Q5Thing`, `Input.IsJustClickedByID__iiQ6String`.

### PC run

`level_scripts --all --play=60`, 378 levels, 13,547 scripted Things, plain / with
`--visit-triggers` / with `--visit-triggers --pulse-switches`, inputs as in part 11.
Outputs under ignored `local-analysis/`: `d271-play-all-{novisit,visit,visit-pulse}.json`
(D270: `d270-play-all-*`).

| 60 ticks per level | D270 | D271 |
|---|---|---|
| Things stopped by a fault | 3 / 3 / 3 | 3 / 3 / 3 |
| Fault kinds, levels with a fault | 1, 1 | 1, 1 |
| Natives in the table | 238 | 249 |
| Unbound natives called | 44 / 78 / 78 | 41 / 73 / 73 |
| Unbound calls | 61,483 / 865,266 / 872,042 | 61,480 / 862,986 / 868,663 |
| Native calls | 573,134 / 5,365,247 / 5,424,893 | the same |
| Recorded-only calls | 12,328 / 91,027 / 96,943 | the same |
| Calls answered "nothing started" (`RECORDED_REQUESTS`) | - | 2 / 2,262 / 3,357 |
| Video requests, levels | - | 1, 1 in all three |
| Voice-over requests, levels | - | 1, 1 in all three |
| Mix overlays removed, levels | - | 0 / 2,260 in 114 / 3,355 in 143 |
| Levels where a script set a progression flag | - | 0 / 3 / 3 |

- Gone from the unbound list (calls / levels): `Video.StartVideo` 1 / 1,
  `Audio.RequestVoiceOver` 1 / 1, `Profile.HasPlayedLBP1AndUnlockedPod` 1 / 1 in all
  three runs; with the visit `Audio.RemoveMixOverlay` 2,260 / 114 and 3,355 / 143 (the
  speech bubble, when it closes) and `Profile.SetLBP2GameProgressionFlag` 17 / 3 and
  21 / 3 (notes that unlock something). Nothing is new on the list.
- The one video request and the one voice-over request are the intro's, in
  pod_planets. At 60 ticks the intro thread is in its zoom, as before.
- The three Things that stop are the TriggerAirStone instances of part 21.
- Messages, global settings, display, the camera, the pod block, trigger events and
  switch reactions are equal to D270.

The two Pod levels alone (`d271-planets-{60,600,1200,600-inuse,600-flags2}.json`,
`d271-pod-{60,600,600-flags3,60-visit-pulse,60-visit-pulse-flags3}.json`):

| Level, run | D270 | D271 |
|---|---|---|
| pod_planets, 60 ticks | thread started on tick 1, running at the end; 3 unbound natives, 3 calls | the same thread state; 0 unbound natives; 1 video request (GUID 0x21fde, a null Thing, false) and 1 voice-over request kept, both made on tick 1; the camera at the same point of the zoom |
| pod_planets, 600 ticks | thread still running; 4 unbound natives, 152 calls; flags never set | the thread ends on tick 452: 451 ticks of zoom, then one test of the wait, which is over at once; the flag word goes 0 -> 3 on tick 452; no thread is started again (1 started, 1 done); 0 unbound natives; 0 faults |
| pod_planets, 1,200 ticks | - | as 600: 1 thread started, 1 done, flags 3, 0 unbound natives |
| pod_planets, 600 ticks, pod computer in use from tick 100 | at the end the camera is the not-in-use preset: the running intro holds it (scoping run) | the intro ends on tick 452; at the end the camera is the in-use preset, from (0, -300, 900) to (0, 500, -1500) |
| pod_planets, 600 ticks, `--flags=2` | no thread | no thread, no request, flags stay 2 |
| pod.bin, 60 and 600 ticks | no fault, resumed every tick; unbound `SetPodTransitionTexture` 61 / 601 | the same |
| pod.bin, 600 ticks, `--flags=3` | - | the same: without an avatar the update does not reach the flag test |
| pod.bin, 60 ticks, visit and pulse, `--flags=3` | - | with the intro's flags and a player the update goes one test further and asks `Native.GetIsInPadZone__Q5Thing` (5 calls, unbound, so false) |

After tick 452 the planets level is in its idle state: the world script's per-frame
entry runs each tick, the camera is the "pod computer not in use" preset, from
(0, -250, 1130) to (0, 100, -1500) (or the in-use preset), and the intro is not played
again because the mask 2 is set. Earth and Moon still have no per-tick call
(`Update__rb`, part 6). The flags stay in the one level world: pod.bin, played alone,
starts with a fresh profile unless `--flags` says otherwise.

### Not done

- No video and no voice-over plays, and nothing shows that one was asked for. The
  requests are there for the system that will.
- A stand-in video of some length (so that the camera shift of a playing video is
  seen): not built; a length would be a guess.
- The per-tick `PodComputer.UpdateVideo__` call and `Video.PressedStartButton__`.
- One session for both Pod levels, so that the flags the intro sets in pod_planets
  open the engage test in pod.bin.
- The viewer's Pod camera (PR 3) and entering the pod computer (PR 4).
- A profile: the flag word lives in the level world and is not saved.

### Verification

`cargo test`: `lbp-script-vm` (new `d271_video_voice_over_and_flag_natives`: the eleven
natives in a world without the state, their answers, wrong argument types, the calls
handed to a world, the mask rule; the table count 249), `lbp-runtime` 59 unit and 45
world tests. New, all on made-up levels and names: `progression_flag_masks_are_ored`
(the option's parse), `d271_flags_are_bit_masks_and_media_requests_are_kept_and_not_played`
(set 3 and test 1, 2, 3, 4; a mask of two bits with one set; the top bit; requests kept
with Thing, GUID, name, flags and frame; the answers; a null line; the lists' bound;
none of it in the kept engine requests or the messages) and
`d271_a_thread_that_waits_on_video_and_voice_over_ends_and_sets_its_flags` (a made-up
thread shaped like the intro: a video request, a line request, five yields, the wait
on "a line plays or the handle is not -1", then the mask 3; it ends on the tick after
its last yield, the flags are 3, both requests are kept, no native is unbound). fmt,
the CI clippy line, the CI test set and the Python tests pass. Because a runtime type
the viewer uses changed: `cargo check` of both viewers and `cargo clippy -p
lbp-bevy-viewer --all-targets -- -D warnings` pass; the Bevy viewer's tests pass but
the one that fails on main (`sackboy_play::tests::input_script_steps`). The viewer was
not started. Nothing compared against the game in RPCS3.

Counts: +0 newly viewable / +0 newly Complete (runtime feature).
