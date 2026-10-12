# PC lanes (parallel work on the owner's PC)

Owner direction 2026-10-08 (Chicago): run work in parallel and organize the windows on the
monitors. This page records the measured headroom, the lanes and the window layout. Local
tools (Python helpers, RPCS3 copies) live outside the repository.

## Hardware (measured 2026-10-08)

| Item | Value |
|---|---|
| CPU | Intel i7-12700KF, 12 cores / 20 threads |
| RAM | 31.8 GB (about 10-11 GB free with one RPCS3 + agents) |
| GPU | NVIDIA RTX 3080, 10 GB VRAM (about 4.5 GB used with one RPCS3) |
| Disk | C: 476 GB, about 44 GB free after the lane-2 RPCS3 copy (7.4 GB) |
| Monitors | DISPLAY1 primary 2560x1440 at (0,0); DISPLAY2 1920x1080 at (2560,0) |
| Load | one RPCS3 (Vulkan) about 37% CPU; two RPCS3 + builds reach about 94% |

## Lanes

| Lane | What | Limits |
|---|---|---|
| A | RPCS3 #1: the owner's install `%USERPROFILE%\Emulators\rpcs3` (Vulkan, IPC 28012). Pod/menus and level measurements. | Owner's config and saves untouched. |
| B | RPCS3 #2: separate copy `%USERPROFILE%\Emulators\rpcs3-lane2` (own config, dev_hdd0 copy, shader cache; IPC 28013). OpenGL + RenderDoc compatibility for RSX/RenderDoc captures. | Runs only while lane C is not building, to keep CPU under about 85%. |
| C | Builds and viewer captures in worktrees with their own `CARGO_TARGET_DIR` (target-pr51, target-pr60). | `cargo -j 6` while an emulator runs. |
| D | Headless research/census as background agents in their own worktrees (target-lane-*). | `-j 4..6`, at most two at a time. |

Both RPCS3 instances read the same read-only ISO (`%USERPROFILE%\Emulators\lbp2-iso`). The
lane-2 copy was made with robocopy (no logs, caches, captures or screenshots); its save data
is a copy, so the owner's save is never written by lane B. Its config changes are in its own
`config/config.yml` (original kept as `config.yml.lane2-orig`).

Memory reads and key input pick one instance by process id (`RPCS3_PID` for the scratch
helpers; `RPCS3_DIR` for its screenshot folder). Guest memory base is 0x300000000 in both.

## Window layout (DISPLAY2 only; the primary monitor stays free)

| Slot | Position (x, y, w, h) | Window |
|---|---|---|
| top-left | 2560, 0, 960, 572 | RPCS3 #1 game window |
| top-right | 3520, 0, 960, 572 | RPCS3 #2 game window |
| bottom-left | 2560, 540, 960, 540 | viewer QA window |
| bottom-right | 3520, 540, 960, 540 | second viewer / RenderDoc |

The viewer already places its window on the non-primary monitor and logs it before showing.
RPCS3 windows are moved with SetWindowPos and the resulting rectangles are read back.

## Launch

```powershell
# lane B (lane A is the owner's normal RPCS3)
Start-Process $env:USERPROFILE\Emulators\rpcs3-lane2\rpcs3.exe -WorkingDirectory $env:USERPROFILE\Emulators\rpcs3-lane2 -ArgumentList "`"$env:USERPROFILE\Emulators\lbp2-iso\LittleBigPlanet 2 (USA) (En,Fr,Es,Pt).iso`""
```

Pod comparison captures add `--window-16-9 --preview-lighting` (rule from PR #71).
