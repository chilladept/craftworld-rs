# Logic, Part 90

See docs/logic.md for Parts 1-49.

## Part 90: a piston's angle rows give way past 800 × the reduced mass (D1189)

Part 87 found that a non-stiff piston's angle rows are hard, frictionless contact rows from
the filler 0x95ba80, and left the impulse cap at row+0x4c half decoded. D1189 decoded the
rest of it.

FOUND (01.33):
- The cap is min(m, 800 / (a + b)) (0x95bf58). 800 is 200 (TOC -0x20b0) × 4 (TOC
  -0x2160), multiplied at 0x95bf00.
- a and b are the floats at +0x28 of the two solver bodies the row joins (+0x10 and +0x14
  of the filler's second argument, each +0x20 then +8, 0x95be74..0x95beb4). The builder
  0x1c58d0 tests the same float for 0 to skip a row for a fixed body (0x1c59bc), so it is
  the body's inverse mass, and 800 / (a + b) is 800 × the reduced mass.
- When a + b gives no number (NaN from the reciprocal step), the plain reciprocal
  estimate is kept (vsel at 0x95bf50).
- m is the smaller of the +0x14 floats of the two records at +8 and +0xc of the same
  argument (fsel at 0x95bbcc, stored at 0x95bbd0 and splatted at 0x95bc04).

INFERRED:
- The two +8/+0xc records are the bodies' materials, so m is a per-material impulse
  limit. Which RMaterial field sits at +0x14 was not checked.
- A piston's angle rows therefore hold any push up to 800 × the reduced mass per solver
  step, then give way, so a hard enough hit can twist a non-stiff piston off its line
  for a step.

Change: none to behaviour. avian's prismatic joint has no per-step impulse limit, and
the solver units of the cap were not mapped to ours. Route and parity are not affected,
since no code changed.
