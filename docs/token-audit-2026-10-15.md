# Token audit, 2026-10-15

Follow-up to `docs/context-audit-2026-10-12.md`. Question: where do the PC threads
(RScript parts 13-22 and their siblings) spend context, and what would cut it.
Recommendations only; nothing here changes settings.

Data: the 24 most recent transcripts of the PC thread folder (Oct 13-15, 929 model
calls, 159M context tokens read in total), `scripts/ctx_audit.py`, three small analysis
scripts, and nine `claude -p --model haiku` runs that measure a fresh session's
first-call context under different settings. "Context per call" is input + cache read +
cache write from the API usage fields, i.e. what each model call re-reads.

## Headline

1. **Every PC thread now starts at 99.4k tokens** (24/24 threads: 99.1-99.7k), up from 92k
   on 10-12. At ~39 calls per thread that floor is **58% of all tokens read** (92M of 159M).
2. **The 10-12 deny list never reached the PC threads.** It lives in
   `E:\Projects\LBP2 Rust Rewrite\.claude\settings.json`, but Remote Control threads run in
   `C:\Users\<user>\Documents\Projects\LBP2 Rust Rewrite`, a near-empty folder outside the
   repo. Same reason the repo `CLAUDE.md`/`AGENTS.md` (output caps, read-by-section rules)
   are never auto-loaded there.
3. **The rollover rule is overshot almost half the time.** 45% of calls ran above 175k
   context and those calls are **57% of all tokens**. Peaks were 209-281k. Threads cannot
   see their own context size, so "plan at 150k" is guessed.
4. **Each rollover costs 20-35k of orientation in the first 10 calls** (measured on 7
   threads; one reached 190k by call 20), mostly section reads of a 603 KB doc and probe
   dumps.

Cost per call is roughly the whole context, so fixes that lower the floor or stop a
thread earlier pay on every later call.

## Where the 99.4k floor comes from (measured)

| Piece | Tokens | How measured |
|---|---|---|
| CLI part in the thread folder (system prompt, built-in tools, skill list, memory index) | 39-41k | `claude -p` in the C: folder, 3 runs |
| same, with the repo's existing deny list | 23-25k | `--settings <repo settings.json>`, 3 runs |
| same, deny list + Skill and other unused tools (list below) | 17.8k | `--settings deny-ext.json` |
| CLI floor with only Bash/Read/Edit/Write/Grep/Glob | 12.4k | `--tools` |
| Remote Control extras: project prompt, hearthbot/ccd/Browser/Docs/visualize schemas, brief | ~58-60k | 99.4k thread first call minus the 40k CLI part |

Denying in the thread folder saves **~22k measured** on the CLI side. The app-side MCP
servers already in the repo deny list (Claude_Browser, visualize, Claude Docs) would come
off the Remote Control extras on top; estimated 5-8k from their schema text, not measured
(`claude -p` does not load them).

Tool use in the 40 latest threads: Bash 543, PowerShell 488, update_status 262, Read 181,
reply 116, no_reply_needed 96, Write 94, Agent 84, send_message 74, Edit 63, ccd_pr 41,
ToolSearch 23, Grep 20, SendMessage 15, react 15, SendUserFile 9, WebSearch 6, Skill 1,
Glob 1. Never used: Artifact*, Workflow, ScheduleWakeup, AskUserQuestion, plan mode,
Claude_Browser, visualize, Claude Docs, computer-use, scheduled-tasks, mcp-registry.

## Where the work tokens go

Context growth per single tool result (24 threads; median tokens added):

| Class | Results | Median | Total |
|---|---|---|---|
| shell other (builds, gh, scripts) | 305 | 851 | 430k |
| shell file dumps (cat/sed -n/Get-Content) | 211 | 1,231 | 358k |
| git status/log/diff/worktree | 68 | 1,296 | 155k |
| wakes ending in no_reply_needed | 66 | 679 | 113k |
| Agent results | 35 | 2,116 | 77k |
| Read | 44 | 666 | 76k |
| images | 29 | 1,310 | 36k |

No single sink: only 15 results added more than 8k (GitHub search loops, probe JSON
dumps, a `fetch_thread`, level-scripts.md ranges). `cargo` output stays capped.

Calls that did no work (model call whose only tools were bookkeeping):

| Kind | Calls | Share of tokens |
|---|---|---|
| no_reply_needed alone (subagent hand-backs, task notifications, FYI relays) | 69 | 5% |
| update_status alone | 50 | 4% |
| no_reply_needed + update_status | 24 | 2% |
| ToolSearch alone | 15 | 1% |

Of the no-reply wakes, 30 were subagent hand-back messages (1-7k characters each) that
arrived after the same result had come in as a task notification, 14 were task
notifications, 8 coordinator relays.

## Orientation per rollover

Context at call 1 / 10 / 20 (k): part 21 99/124/150, part 20 99/122/140, part 19
99/134/167, part 18 100/119/129, part 17 99/123/141, part 16 99/135/156, part 13
93/115/133, part 22 100/140/190. What the first calls read:

- `docs/level-scripts.md` is **603 KB, 8,839 lines**; one section read costs 7-9k and
  every new thread reads 2-3 of them.
- `docs/rscript-plan.md` (30 KB, ~8.5k) is named in briefs as a start read.
- `STATUS-CURRENT.md` is **120 KB, 1,440 lines** against its own 60-line rule; STATUS.md
  stopped getting entries around D282, so the "Before:" sections in STATUS-CURRENT became
  the history log. Threads read 3-16k of it at start.
- `git worktree list` prints ~100 worktrees (~4k each time).
- Briefs say "read MEMORY.md", which is already auto-loaded in the thread folder; one
  thread also cat'ed five memory files (+5k).
- Probe-result dumps done in the main thread (part 22: three calls of +8.9k, +10.2k, +7.7k).

## Ranked fixes

Savings are per the 24-thread sample (159M tokens). They overlap, so do not add them up;
together they should cut roughly a third to a half for the same work.

| # | Fix | Est. saving | Who |
|---|---|---|---|
| 1 | Make threads able to see their context and stop on time (below) | 15-20% | agent PR (script) + coordinator brief |
| 2 | Deny unused tools in the thread folder (`C:\...\Documents\Projects\LBP2 Rust Rewrite\.claude\settings.json`) | 13-18% (22k measured + 5-8k est. per call) | user (settings) |
| 3 | Cheaper rollovers: handoff lists the exact reads, no MEMORY.md re-read, exploration in an Explore subagent | ~8% (15k less orientation x ~39 later calls) | coordinator brief + handoff template |
| 4 | Fewer no-work calls: send update_status in the same message as the next tool call; ask subagents for a 10-line final report; batch ToolSearch | 5-8% | thread-folder CLAUDE.md (below) |
| 5 | Split `docs/level-scripts.md` into one file per part with an index | 2-4% | agent PR |
| 6 | Move STATUS-CURRENT's "Before:" sections to the end of STATUS.md once, then keep the 60-line rule | 1-2% | agent PR, best done by the next milestone thread to avoid conflicts |
| 7 | Prune merged worktrees (~100 listed) and their `target-d*` dirs | ~1%, plus disk | user yes (deletes folders) |
| 8 | Repo CLAUDE.md still says "rollover at about 100k" | correctness | this PR |

### 1. Stop on time

The threshold itself is about right. With a 99k floor, ~25k orientation (~1.2M tokens of
calls per thread start) and ~1.1k growth per call, the cheapest thread length is about
45-50 calls, i.e. a peak near 170-175k. The loss is overshoot: 422 of 929 calls ran above
175k at a mean of ~218k. Had those calls run in fresh threads at ~140k average, the sample
would have read ~33M fewer tokens, minus ~6M for the extra rollovers: ~17%.

Threads have no way to read their context size today. Proposal: add
`scripts/ctx_audit.py --session <id-prefix>` printing one line (calls, current context).
A thread's session id is its scratchpad folder name, so the brief can say "check
`ctx_audit.py --session <scratchpad id>` after each PR; at 150k write the handoff".
Until then, `python -I scripts/ctx_audit.py --last 5` works (find the row by the
scratchpad id prefix; its `peak` is the current context) at ~1.5k tokens per check.

Scope rule that matches the data: one D-number (one PR) per thread when the thread
starts above ~130k after orientation; two when it starts lean.

### 2. Deny list where the threads run

Copy the repo deny list into the thread folder's `.claude\settings.json` and add the
tools no thread used in 40 sessions:

```json
"Skill", "AskUserQuestion", "EnterPlanMode", "ExitPlanMode", "NotebookEdit",
"CronCreate", "CronDelete", "CronList", "RemoteTrigger", "EnterWorktree", "ExitWorktree",
"mcp__claude-in-chrome", "mcp__computer-use", "mcp__scheduled-tasks", "mcp__mcp-registry",
"mcp__savvy-progress", "mcp__ccd_sidebar", "mcp__ccd_window", "mcp__ccd_view",
"mcp__ccd_settings", "mcp__ccd_session_mgmt"
```

Trade-offs: threads lose Artifact publishing (the coordinator keeps it) and skills
(1 use in 40 threads). The first call after the change should read ~70-77k instead of 99k;
if it does not, the app-side servers ignore the deny and only the 22k CLI part applies.
Alternative with the same effect plus the repo rules auto-loaded: start PC threads in
`E:\Projects\LBP2 Rust Rewrite` itself (measured 32.5k CLI part there, incl. CLAUDE.md),
at the cost of threads sitting in the main checkout.

### 3/4. A CLAUDE.md for the thread folder

The repo rules never load in the thread folder. A ~1 KB `CLAUDE.md` there would carry
the token rules to every thread (adds ~300 tokens to the floor). Draft:

```markdown
# PC thread folder
Work is in E:\Projects\LBP2 Rust Rewrite. Its CLAUDE.md is not auto-loaded here; read
only its "Command output" and "Standing rules" sections when needed.
- MEMORY.md is already loaded; do not re-read it or the memory files.
- Start from the handoff's "Read first" list only. Grep headings, then read ranges.
- Send update_status in the same message as the next tool call, never alone.
- Exploration (more than ~3 files or probe outputs) goes to an Explore subagent that
  returns a 10-line conclusion. Ask every subagent for a final report under 15 lines.
- Cap output: `| tail -n 30`, `git worktree list | wc -l`, `git log --oneline -5`.
- Check context after each PR; plan the handoff at 150k, stop at 170k.
```

Handoff template addition ("Read first", at most 5 lines): the exact file ranges the next
thread needs, plus a "Facts" block of the numbers it would otherwise re-derive.

## Notes

- Calls are what cost: a 100-call thread at 170k average reads 17M tokens. Long waits on
  workers and CI do not cost by themselves, but each wake re-reads the whole context.
- Re-measure: `python -I scripts/ctx_audit.py --last 25`; floor check: first-call context
  of the next thread.
