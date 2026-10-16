# Handoff: token fixes (2026-10-16)

From the token audit thread (`docs/token-audit-2026-10-15.md`, PR 87). Owner said
"please execute all of these".

Done:
- Thread folder (`%USERPROFILE%\Documents\Projects\LBP2 Rust Rewrite`): `.claude/settings.json`
  deny list (41 unused tools) and a 0.9 KB `CLAUDE.md` with the token rules. Measured CLI
  floor there: 18.2k (was 39-41k). Applies to new threads; the deny took effect live.
- PR 87: `scripts/ctx_audit.py --session <id prefix>` prints one line with the current
  context (id = scratchpad folder name); default transcript dir no longer hardcodes a user name.
- Coordinator told to add "Read first" + "Facts" to handoffs and drop "read MEMORY.md".

Left (owner approved all, including deletes):
1. STATUS-CURRENT.md: move every "## Before:" section to the end of STATUS.md (CRLF,
   oldest first; Python newline=''), keep header + Latest (~60 lines). Do it right after
   part 22 merges its next PR, to avoid conflicts.
2. docs/level-scripts.md (603 KB): split into docs/level-scripts/part-NN.md + index; same
   timing as 1; fix links in STATUS-CURRENT/handoffs.
3. Prune worktrees (~100): remove only ones that are clean, not used by a running thread
   (wt-d324 = part 22; check Level globals'), with HEAD merged/pushed and no ignored files
   outside target/. Then delete `target-d*` dirs of merged D-numbers not touched in 6 h.
4. Verify: next thread's first call should be ~70-77k (was 99.4k).
