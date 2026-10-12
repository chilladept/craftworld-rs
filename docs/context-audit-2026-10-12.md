# Context audit, 2026-10-12

Goal: threads hand off too often and burn plan usage. Measured from the 20 most recent
transcripts in this folder (Oct 10-12, ~2,860 assistant turns) with `scripts/ctx_audit.py`
plus controlled `claude -p` runs (haiku, token totals from the API usage fields).

## Headline

1. **Every session starts at ~92k tokens before the first action** (92,081 in the median
   session; range 91.7-92.3k). The "hand off at ~100k" rule is therefore already tripped at
   turn 1; in practice sessions run to 130-380k, so the rule is not what ends them.
2. **That baseline is 49% of all tokens the project reads from cache.** Cost is turns x context:
   a 364-turn session read 92M cached tokens, of which 33M was the baseline alone.
3. Work growth is many small results, not a few giant ones: ~870 tokens per turn, largest
   single tool result 14k tokens. The biggest sinks are file dumps, repeated reads and
   one-off python probes.

## Where the 92k baseline comes from (measured)

| Piece | Tokens | How measured | Controllable here? |
|---|---|---|---|
| Harness core (6 tools: Bash/Read/Edit/Write/Grep/Glob) + system prompt | ~6.8k | `--tools` | no |
| Remaining built-in tool schemas + skill list + deferred-tool names | ~27k | default minus core | partly (deny list) |
| of which Artifact + Workflow | ~13.4k | `--disallowedTools` | Workflow yes; Artifact is a policy call |
| of which PowerShell / Agent / Skill / ScheduleWakeup | 3.7k / 2.6k / 3.4k / 1.7k | `--tools +X` | ScheduleWakeup yes |
| CLAUDE.md + AGENTS.md + memory index | ~5-6k | file sizes (21 KB) | trimmed little; not the problem |
| Remote Control session extras (app MCP schemas, project prompt, skills) | ~47k | RC first turn (92k) minus `claude -p` here (45k) | MCP schemas yes, via deny |

Docs are not the sink: STATUS-CURRENT.md is 4.5 KB, CLAUDE.md+AGENTS.md ~5k tokens.
STATUS.md (290 KB) and MODLOG.md are already read by section only; zero whole-file reads found.

## Where growth goes (19 sessions, 2.5M tokens of context growth)

| Class | k tokens | calls | tok/call | Fix |
|---|---|---|---|---|
| `cat`/`sed -n`/`head` file dumps | 829 | 455 | 1.8k | read ranges after grep; subagent for >3 files |
| `python` probe scripts | 276 | 223 | 1.2k | one script file per question, `--summary`, print counts |
| Read tool | 253 | 202 | 1.3k | same; split 150-170 KB `main.rs`, `level_scene.rs`, `linked.rs` over time |
| Artifact reads | 184 | 12 | 15k | 4 re-reads of one 50 KB page; keep a local copy, never re-read |
| git status/log/worktree/fetch | 153 | 95 | 1.6k | `-sb`, `--short`, `-5`; worktree list is 60+ lines, use `git worktree list \| wc -l` or grep |
| rpcs3 tools / pose_fit | 144 / 96 | 87 / 59 | 1.6k | already summary-ish; add `--quiet`/one-line verdict |
| hearthbot wake/reply traffic | ~175 | ~200 | 0.7-1k | each wake turn re-reads the full context; see proposals |

Notes: attribution splits each turn's growth across the tool results before it, so it also
includes the assistant's own output; treat as ranking, not exact. cargo/clippy output is NOT
a sink (710 tokens/call): the `| tail` rule in CLAUDE.md works. 202 images were read (~1.5k each).
Re-reading CLAUDE.md/AGENTS.md (already auto-loaded) cost ~43k tokens over the 19 sessions.

## Done in the PR (repo side)

- `.claude/settings.json`: deny list for tools this project never used in 43 sessions
  (Workflow, ScheduleWakeup, ReportFindings, SearchPlugins, Suggest*, mcp Browser/visualize/Docs
  servers, unused hearthbot tools). Measured with `claude -p`: -4.5k from the built-ins alone;
  MCP server schemas come on top (a dummy server's 40k dropped to 0 with a server-level deny,
  per-tool deny also works). Verify: next thread's first-turn context should drop below ~92k.
- `CLAUDE.md`: 9 lines on baseline economics and "do not re-read the auto-loaded files".
- `scripts/ctx_audit.py`: reproduces these tables in ~40 lines of output.

## Proposals (outside the repo; coordinator / owner)

1. Rebase the rollover rule. With a 92k floor, "about 100k" is meaningless. Use "baseline + 80k"
   (about 170k) or a turn count (~150 turns). The best sessions did 14-31 turns for one PR;
   the 300+ turn sessions are what cost 70-90M cached tokens.
2. Thread-per-PR is fine, but each new thread pays 92k x its turns. Prefer fewer, bigger
   subagent jobs inside one thread over starting many threads, and prefer local `claude -p`
   workers for implementation (their baseline is 45k, not 92k).
3. Avoid FYI relays to threads: 336 `no_reply_needed` turns in history, each a full
   context re-read. Only message a thread when it needs to act.
4. Decide whether threads need the Artifact tool (~10k tokens per turn, 54 uses in history).
   If publishing can live in the coordinator or one dedicated thread, add `"Artifact"`,
   `"ArtifactComments"`, `"ArtifactData"` to the deny list.
5. User-level `~/.claude/settings.json`: the savvy-progress plugin adds MCP tools (names
   only, small); drop it if unused. The failing `plugin:github` MCP (HTTP 400) is harmless to
   context but noisy; fix its token or disable.
6. Project prompt: the "Claude in Projects" block in every thread is ~8k tokens; a shorter
   thread-only variant would save that on every turn.
7. Long-run: `STATUS.md` (290 KB) is append-only history; archive everything older than the
   last ~10 entries to `docs/archive/` so grep hits stay small.

## Parked: leaving GitHub / CI billing (2026-10-12, owner on hold for a restructure)

- 2026-10-12 13:00 CT-ish: Actions jobs refused ("recent account payments have failed or your
  spending limit needs to be increased"); the private repo's Actions minutes are metered. PR 143
  ran green earlier the same day. PR 145 (deny Artifact tools) is unmerged for this reason.
- Exit cost: repo is plain git, one remote, so mirroring/moving is one push. The only
  GitHub-bound file is `.github/workflows/ci.yml` (87 lines: fmt, test, clippy, check, python
  tests). Workflow-level ties: PR/`gh` flow, merge-on-green, PR attribution and PR panel,
  cloud agents that open PRs.
- Cheapest first step: a local pre-merge check script on the PC, GitHub kept as a push mirror;
  or make the repo public (Actions free). Alternatives: Codeberg/GitLab/Gitea.
