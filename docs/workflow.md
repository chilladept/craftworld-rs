# LBP2 multi-agent workflow

Rewritten 2026-10-07 (America/Chicago) after the project audit to match how batches
actually run. AGENTS.md stays authoritative. `docs/cloud-sessions.md` covers the cloud
credit and Linux setup.

## 0. Who pays

| Runs as | Paid by |
|---|---|
| Cloud session (claude.ai/code, `claude --cloud`) | $250 cloud credit (expires 2026-11-05) |
| Project threads, Remote Control on the PC, PC subagents, local `claude -p` | Plan limits |

Since 2026-10-07 14:50 CT the owner runs new batches on the PC under plan limits. Do
not launch new cloud sessions and do not give a finished cloud session new work unless
the owner asks again.

Running N agents costs about N times one agent when their tasks are independent. It
costs more when they touch the same files (conflicts, re-reads). Parallelism buys
wall-clock time, not savings.

## 1. Roles

| Role | Where | Owns |
|---|---|---|
| **Dispatcher** | The project's "Multi-agent workflow plan" thread | Picks the next tasks from `docs/backlog.md`, assigns D-numbers, briefs the Integrator. One turn per batch. |
| **Integrator** | One Remote Control session on the PC | Spawns Implementer subagents, oracle/parity runs, viewer QA, counts, merges, `main`, STATUS.md, MODLOG.md, STATUS-CURRENT.md, `docs/backlog.md`, `docs/batch-report.md` |
| **Implementer** | Integrator subagent in its own git worktree | One task: branch, code, synthetic tests, its own milestone doc |

Only one PC session works in the checkout at a time. Older Remote Control sessions in
other project threads must stay parked; don't wake them for repo work.

## 2. D-numbers

Only the Dispatcher assigns them: max(latest STATUS milestone, highest remote branch
D-number) + 1, sequentially, written into the batch prompt. No GitHub Issues queue.
Never start a task whose scope matches an existing branch or milestone doc (D74
duplicated D71 this way).

## 3. Parallelism

- 2 Implementers per batch, 3 if the last batch merged without conflicts.
- Never run two tasks that edit the same file at once. Each task prompt has a
  `Touches:` line; only non-overlapping tasks share a batch.
- Implementers write full detail only in `docs/d<NN>-<slug>.md`. Edits to
  `docs/formats.md` / `docs/architecture.md` stay inside the task's own section.
  STATUS/MODLOG/STATUS-CURRENT/backlog/batch-report are Integrator-only.

## 4. Task prompt

```markdown
**D<NN>** · <one-line goal>
Touches: <files>, docs/d<NN>-<slug>.md
Read only: STATUS-CURRENT.md, <doc sections by heading>. No other milestone docs.
Evidence: <Toolkit path @ d6a2ef80, method/section; evidence label from docs/formats.md>
Done when: <tests that must exist and pass>
Verify: <exact game-data command; expected count change>
Out of scope: <what not to touch>
```

## 5. Implementer steps

1. Worktree off `origin/main` (never the main folder): branch `cloud/D<NN>-<slug>`.
2. Implement; synthetic fixtures only; checked arithmetic on untrusted lengths.
3. Checks, output capped (CLAUDE.md): fmt, `-p <crate>` test and Clippy, then the
   task's Verify command on game data.
4. Keep each file's existing line endings (see §8). Commit, push the branch, report
   counts in ≤8 lines with a STATUS draft (≤15 lines) and MODLOG draft (≤3 lines).

## 6. Integrator steps (per batch)

1. `git fetch origin`, then list every candidate: `git branch -r --no-merged origin/main`.
   This includes `cloud/*`, `claude/*` (cloud CLI names) and any other prefix.
2. In D-number order: fmt / test / Clippy (capped), both viewer builds, the task's
   Verify command. Pass: merge into main. Fail: ≤10 lines in `docs/batch-report.md`,
   skip.
3. Visual check for Complete counts (AGENTS.md): for each cohort mesh that newly has
   all diffuse textures routed, write a `--qa-capture` PNG under `local-analysis/`
   and list the paths in the batch report so the owner can mark pass/fail. Count a
   mesh Complete only after the owner's pass.
4. One batch STATUS entry (≤40 lines), MODLOG (≤6), overwrite STATUS-CURRENT.md,
   refresh `docs/backlog.md`. Commit, **push main before closing anything**: GitHub
   then marks each open PR "Merged" by itself. Don't close PRs by hand.
5. Delete merged branches: `git push origin --delete <branch>...`.
6. Report ≤8 lines to the Dispatcher. The Dispatcher waits for this report instead of
   scheduling timed check-ins.

## 7. Toolchain parity

CI pins Rust 1.99.0 (the PC's `stable-x86_64-pc-windows-gnu` as of 2026-10-07). When
the PC toolchain updates, bump `toolchain:` in `.github/workflows/ci.yml` in the same
commit. Cloud containers ship an older Rust (1.97 on 2026-10-09), so a cloud session runs
`scripts/precheck.sh` before pushing: it installs the CI toolchain if missing and runs
fmt, Clippy, tests, the viewer check and the Python tests with capped output (D79 and
#67 failed CI on lints the cloud's default Clippy doesn't have).

## 8. Line endings

Until the repo is normalized, 56 tracked files are CRLF and the rest LF, and there is
no `.gitattributes`. Edits from Linux must keep a file's existing endings or the diff
rewrites the whole file (D78 turned a 517-line change into 5,313 lines).

One-time fix, by the Integrator, right after a batch merges and while no task branch
is open (otherwise every open branch conflicts):

1. `.gitattributes` exists already (it marks the viewer help list `merge=union`).
2. Add to `.gitattributes`:
   ```
   * text=auto eol=lf
   *.ps1 text eol=crlf
   *.cmd text eol=crlf
   ```
3. `git add --renormalize .` then `git status --short` (expect only line-ending
   changes; `git diff --cached --ignore-cr-at-eol --stat` must be empty apart from
   `.gitignore` and `.gitattributes`).
4. One commit, "Normalize line endings", nothing else in it; push main. Branches
   created afterwards need nothing special.

After this, delete this section.

## 9. Cloud threads, PC results and relays

Since 2026-10-08 cloud research threads in the project open PRs on `claude/*` branches
and the PC verifies them. Each hop through a long-running session (coordinator, PC
thread) replays its whole context, ~300-370k tokens per model call on 2026-10-09, so
the rules below keep hops few and messages small.

1. **Results are written once.** The PC posts a PR's verification result as one
   comment on that PR (`gh pr comment <n> --body-file <file>`): verdict, commands,
   counts, RPCS3 deltas, follow-ups. The thread that opened the PR is subscribed to
   it and gets the comment directly; nothing needs relaying.
2. **Results with no PR** (measurements, RPCS3 runs, research questions) go to
   `/mnt/project-files/lbp2/results/<YYYY-MM-DD>-<topic>.md` when the writer can
   reach the project folder, otherwise into a PR comment on the requesting thread's
   latest PR.
3. **Relays carry pointers.** A relay is at most 3 lines: what, verdict, link. Never
   paste a log, table or list of numbers into a relay; it gets copied 2-3 times.
4. **Requests are batched.** A cloud thread sends the PC one request per PR (or per
   batch of PRs), listing the exact commands, not one message per question.
5. **Stacked PRs:** open the second PR against `main` after the first merges, or say
   "merge after #N" in the PR body so the PC merges in that order.
