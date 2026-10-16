# Level scripts, part 56: where the game sets "in use" within the frame (2026-10-15, D305)

Research only, no code change. Part 52 left one tick between the game's planets zoom and the
VM's at the change-over (at the dissolve's line, tick 614.2, `ViewTransition` about 0.71 in
the VM and about 0.75 in the game) and named one candidate: the handler of network message
0x30, which sets "in use", running between the main level's (shell's) world update and the
pod computer's step, so that the pod computer would see "in use" one frame before the shell.
The handler's dispatcher is found here, and the candidate is ruled out.

### What was found (01.33 executable)

- FOUND: 0x28a804 fills a table of 12-byte entries in the network object at +0x1820 + id *
  0xc (the handler's descriptor at +0x10, two words at +8 and +0xc); 0x401d40 puts message
  0x30's handler there (0xa09c38, which reads the message's byte and calls 0x3fa868).
- FOUND: the dispatcher is 0x28b304 itself. Part 52 read it as calling only message 0x79's
  handler, but 0x79 is only the value written to the message header (and to a global "current
  message") before the packet is read and after the handler returns; the listing loads the id
  from the read header and calls the handler at +0x1820 + id * 0xc + 0x10 for any id. 0x28b304
  is called only by 0x28b5a4 (each of the 8 connections, the three receive channels), and
  0x28b5a4 only by the network pump 0x28baa4.
- FOUND: the pump runs in the main loop (0x19348) at the start of the frame (0x197d0, through the
  network update 0x25ef24) and at the start of each simulation step (0x19d18), both times with all
  three channels (flags 7), and each step's world update (0x1a077c) comes after it (0x19edc); the
  step loop closes at 0x19f60, and the frame function (0x18558, the pod computer's step) is called
  after it (0x1a24c). Between the last world update and the frame function the main loop calls no
  pump. The other callers of the network update (0x15314, 0x14e444, 0x281014, 0x25f5b4,
  0x89eecc, 0x1acb4) are loading and shutdown waits, not this frame.
- FOUND: sending does not call a handler. 0x3fa9cc (`PodComputer.RequestHostUsePod__b`, native
  0x5fbd04 -> 0x50332c -> 0x89752c) checks 0x3fa974 (our Pod, owned by the local user: game
  object +0x3b5 and 0x16c788), stores the request and the time, and sends the one-byte message
  0x30 on channel 3 to the host's connection (0x28a834) through 0x28b74c -> 0x240cd4, which only
  queues the packet (0x231520). The handler 0x3fa868 sets "in use" through 0x18ed30 and clears
  the stored request and time. The other callers of 0x18ed30 are the level start (0x1ab508, true
  in one branch), the leave path (0x1a7580, false) and the pod computer's two destructors
  (0x3ffb58, 0x3fff5c, false).
- So: "in use" changes only at a pump, which comes before a step's world update. In the frame it
  changes, the shell's update and the pod computer's step (later in the frame) read the same new
  byte. The pod computer cannot see "in use" before the shell. The request is made in pod.bin's
  script (part 24), that is in the shell's world update, after that frame's pump; the earliest
  pump that can deliver it is the next frame's (INFERRED for our own Pod, where the host is the
  local user: the loopback's delay is not read). That is the VM's model: the answer is taken at
  the start of the next round, by both worlds (`LevelWorld::begin_session_tick`).
- FOUND, beside the question: the pod computer's step itself calls `RequestHostUsePod(true)`
  (0x406920) when "in use" is off and a queued request or 0x2628c0 holds (INFERRED: another way
  in, e.g. from the start menu; not in this session's path). 0x4064bc reads byte +0x408 once more:
  its fourth argument to the planets object's slot 0x1c is 1 when "in use" is on, a global
  object's +0x58 is 0 and its +0x44 and +0x50 are at most a constant, and the pod computer's list
  at +0x9f4 is empty.
- The eleven functions 0x40663c calls after `PerFrameUpdate__b` (0x3fa654, 0x4064bc, 0x3fe570,
  0x3fbe3c, 0x3f9b80, 0x3f9aa4, 0x3fc3c0, 0x3fdf6c, 0x402560, 0x400308, 0x3fda70): only 0x4064bc
  reaches the planets view; the others step the pod computer's own state machines (the
  controller's state at +0xa48, the menus' results at +0xa1c, a fade list at +0x9f0, loading at
  +0x870) and none steps the blend or the change-over (listing). 0x402560 resets the pod
  computer's state through 0x402478 when "in use" is off.

### What the runtime does

Unchanged. `LevelWorld::begin_session_tick` takes the host's answer for every world at the start
of the round, before the shell ticks, and the pod computer's step comes later in the planets
level's tick: both see "in use" in the same round, as in the game.

### Measured

Nothing new: no code change, so part 52's table stands (601: 0.05; 614: 0.70; 616: 0.80; 620 and
621: 1.0 in the VM). RPCS3 was not running, so the loopback's delay was not read in memory.

### Left out

- The tick is still open. Not "in use" earlier for the pod computer (this part), not the extra
  zoom, `GetTransitionSpeed` or SmoothStep's argument (part 52). Candidates left (INFERRED,
  unread): the frame the game's change-over is drawn from (the render, 0x31e820, after the frame
  function, against the shell's value of that frame), and the main loop running two steps for
  one frame near the push (the step loop's count is not read at that frame).
- The loopback's delay for our own Pod (0x28a834's connection kinds, the queue of 0x231520).
- The fourth argument of the planets object's slot 0x1c (above) and where 0x4381d8 passes it.

### Verification

Listings and decompiles only (local, not committed); names recorded in the Ghidra project
(`lbp_NetRegisterMessageHandler` 0x28a804, `lbp_NetReceiveAndDispatchOne` 0x28b304,
`lbp_NetDispatchAllConnections` 0x28b5a4, `lbp_NetSendMessage` 0x28b74c,
`lbp_RequestHostUsePod` 0x3fa9cc, `lbp_IsOwnPodOfLocalUser` 0x3fa974,
`lbp_OnMsg30_HostUsePod` 0x3fa868, `lbp_SetPodComputerInUse` 0x18ed30,
`lbp_PodComputerNetInit` 0x401d40) with plate comments on 0x28b304 and 0x3fa9cc. No code change;
`cargo fmt --all --check` passes.

Counts: +0 newly viewable / +0 newly Complete (research).
