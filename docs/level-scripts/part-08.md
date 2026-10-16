# Level scripts, part 8: script threads (2026-10-09)

Fresh-profile Pod planets stopped at `Thread.StartImpl__`: PlanetsWorld.SetState starts
the intro cutscene as a thread (PodIntroductionThread : PodThread : Thread). PC findings
(behaviour only): Thread is abstract with a private `Running` bool; `Start__` asserts
`!Running`, sets it and calls the native `StartImpl__(this)`; `ThreadMain__` asserts
`Running`, makes three virtual calls on `this` (most likely OnStart, Run, OnFinish) and
clears `Running`; `.get.Finished__` = `!Running`. No stop, sleep or is-running native:
waiting is a script loop over `System.Yield__` (PodThread `Wait__i`,
`WaitUntilStartPressed__i`, `PlayAudioAndWait__`). `Thread.Start__` is called from 15
classes (PodComputer 9, Race_End 3, Score_Board 3, PodState menus, PlanetsWorld 2, ...).

Implementation: the VM records the object passed to `Thread.StartImpl__` (null faults the
caller). `LevelScripts` runs `ThreadMain__` on each new thread at the end of the step
that started it (create or tick) until it yields, and resumes suspended threads once per
tick after the Things' suspended scripts and before the updates. Threads that start
threads run in the same step (at most `MAX_THREAD_STARTS`, 256). A faulted thread is
dropped and reported. `StepReport` gains `threads_started`, `threads_done` and
`thread_faults`; `level_scripts --play` reports them and `threads_running_at_end`.

| Rule | Label |
| --- | --- |
| A started thread runs `ThreadMain__` with the thread object as `this` | INFERRED from the listings (PC) |
| It first runs at the end of the starting step, then once per tick | INFERRED; the Pod intro timing in RPCS3 can check it |
| Threads resume after suspended Thing scripts, before updates | INFERRED |

Test: `lbp-runtime` `script_thread_runs_until_yield_then_resumes_each_tick` (a synthetic
Thread and a Thing script that starts it: writes 1 at create, 2 on the next tick; a
null thread faults the starting script). `cargo +1.99.0 clippy -p lbp-script-vm -p
lbp-runtime --all-targets -- -D warnings` clean; tests pass.

PC check: `level_scripts` on Pod planets with `--play=300` and no flags (fresh profile).
Expect `threads_started` 1 and the intro thread's next blockers in `thread_faults` (video
and voice-over natives: `StartVideo__g`, `IsPlayingVideo__`, `RequestVoiceOver__Q7StringAbb`,
`IsVoiceOverPlaying__`). Then compare with RPCS3: what the fresh-profile Pod shows first
and for how many ticks.
