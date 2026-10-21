# Logic, Part 89

See docs/logic.md for Parts 1-49.

## Part 89: a piston with no k still slides (D1188)

D1183 held every piston without a k (any pattern but 2, or a pattern-2 piston wired in
mode 1 or 4) at its rule, since nothing closes its error. Part 88 showed what the game
does instead.

FOUND (01.33, Part 88): row 0x960988 is built for every non-stiff piston; its edges
1.5 × (rule - current) (0x960f98) and cut(rule step, rule - current) (0x960fe0) and the
strength cap on the step edge do not depend on k. Only the cap k × max(|err|, 10)
(0x960ec0) on the 1.5 × err edge does. Its limits are base..base + range (0xc7330,
0xc7350), with base the saved length.

INFERRED: so a piston with a strength and k 0 is pushed by its rule's change each frame
and kept inside the rule's range, but is not pulled back to the rule when something
moves it. The prismatic joint here is not pinned to the rule.

Change: `slides` is now strength > 0 alone; `wired` no longer matters. The D1183 test
becomes `d1188_a_piston_with_a_strength_slides_with_or_without_k`. A piston with
strength 0 is still held at its rule.

Checks: lbp-sim tests pass. The Rookie Test route and parity are PENDING: the PC was
locked and the viewer lost its graphics device at about tick 810 on every run, including
a control run on main without this change.

Open: whether the D1182 route fall changes with this; rerun the route once the screen is
unlocked.
