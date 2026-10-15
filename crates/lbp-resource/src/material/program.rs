//! D309: the form of the glow alpha a material's compiled fragment program writes.
//!
//! Container (`CONFIRMED_FROM_LOCAL_DATA`, the 7,980 BCUS98245 01.33 gmat blobs, D307): each of
//! the ten programs in the shader blob is a `CGB` record; the big-endian u16 at 0x30 is the
//! size of its parameter block, which starts at 0x30, and the instruction list follows at the
//! next 16-byte boundary as big-endian u16 pairs. Each pair names two 8-byte entries of a table
//! shared by all ten programs (the blob's bytes after the last program's end offset); the two
//! entries are one 16-byte instruction or inline constant.
//!
//! Instruction words: the NV40 fragment-program encoding as publicly documented by Mesa's nvfx
//! driver (four 32-bit words, each stored with its 16-bit halves swapped; opcode in word 0 bits
//! 24..=29, end flag bit 0, destination index bits 1..=6, half precision bit 7, write mask bits
//! 9..=12, no-destination bit 30; sources in words 1..=3 with type bits 0..=1, index 2..=7,
//! half bit 8, swizzle 9..=16, negate bit 17; condition bits 18..=20 of word 1, 7 = always;
//! flow control when word 2 bit 31 is set). An instruction with a constant source is followed
//! by its constant. Independently written from that description; no game code.
//!
//! Rule (`CONFIRMED_FROM_LOCAL_DATA`, D307/D309 census over the same corpus): the program's
//! last write of the output alpha (`H0.w`) in variant [`GLOW_ALPHA_VARIANT`] decides the form.
//! The glow's sum `s = 1 + 4 |g|^2` is an ADD of a DP3 result and a constant.
//! - `MUL (RCP s + MAD(-RCP s, f, f)), constant`: `tintcol.w (a + f (1 - a))`, the fog's share
//!   `f` lifts the alpha towards 1 ([`GlowAlphaForm::Fogged`], 524 programs).
//! - `DIV x, s` or `RCP s`: the fog leaves the alpha ([`GlowAlphaForm::Unfogged`], 22 + 1).
//! - Anything else, a program that could not be decoded, one with loops or calls, or a
//!   conditional write (a condition code, or inside an IF block) in the traced chain:
//!   [`GlowAlphaForm::Unknown`].
use serde::Serialize;

/// The program variant whose alpha write is read (the level-lighting variant, D307).
pub const GLOW_ALPHA_VARIANT: usize = 8;
/// Decoded instructions per program, at most.
pub const MAX_INSTRUCTIONS: usize = 4096;

/// How a material's compiled program writes a glowing surface's alpha under fog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum GlowAlphaForm {
    /// `tintcol.w x (a + f (1 - a))`, `a = 1 / (1 + 4 |g|^2)`: the fog lifts the alpha.
    Fogged,
    /// `tintcol.w / (1 + 4 |g|^2)` or the reciprocal alone: the fog leaves the alpha.
    Unfogged,
    /// Not one of the two forms, or not decoded.
    #[default]
    Unknown,
}

const OP_MUL: u8 = 0x02;
const OP_ADD: u8 = 0x03;
const OP_MAD: u8 = 0x04;
const OP_DP3: u8 = 0x05;
const OP_RCP: u8 = 0x1a;
const OP_DIV: u8 = 0x3a;
/// Operations reading no source (NOP, KIL, FENCT, FENCB).
const NO_SOURCE: [u8; 4] = [0x00, 0x12, 0x3d, 0x3e];
/// Operations reading one source.
const ONE_SOURCE: [u8; 20] = [
    0x01, 0x10, 0x11, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x1a, 0x1b, 0x1c, 0x1d, 0x22, 0x23, 0x24,
    0x25, 0x27, 0x28, 0x39,
];
/// Operations reading three sources (MAD, LRP, TXD).
const THREE_SOURCES: [u8; 3] = [0x04, 0x1f, 0x19];
/// Scalar operations: they read the first swizzle component of their source.
const SCALAR: [u8; 6] = [0x1a, 0x1b, 0x1c, 0x1d, 0x22, 0x23];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operand {
    Temp { half: bool, index: u8 },
    Input,
    Constant,
    Other,
}
#[derive(Clone, Copy, Debug)]
struct Source {
    operand: Operand,
    swizzle: [u8; 4],
    negate: bool,
}
#[derive(Clone, Copy, Debug)]
struct Destination {
    half: bool,
    index: u8,
    mask: u8,
}
#[derive(Clone, Copy, Debug)]
struct Instruction {
    opcode: u8,
    destination: Option<Destination>,
    sources: [Source; 3],
    conditional: bool,
}

fn be16(bytes: &[u8], at: usize) -> Option<usize> {
    let pair = bytes.get(at..at.checked_add(2)?)?;
    Some(usize::from(u16::from_be_bytes([pair[0], pair[1]])))
}
fn source(word: u32) -> Source {
    Source {
        operand: match word & 3 {
            0 => Operand::Temp {
                half: word >> 8 & 1 != 0,
                index: (word >> 2 & 0x3f) as u8,
            },
            1 => Operand::Input,
            2 => Operand::Constant,
            _ => Operand::Other,
        },
        swizzle: [0, 1, 2, 3].map(|i| (word >> (9 + 2 * i) & 3) as u8),
        negate: word >> 17 & 1 != 0,
    }
}
fn arity(opcode: u8) -> usize {
    if NO_SOURCE.contains(&opcode) {
        0
    } else if ONE_SOURCE.contains(&opcode) {
        1
    } else if THREE_SOURCES.contains(&opcode) {
        3
    } else {
        2
    }
}

/// Decodes program `variant` of a shader blob. `None` when it is not a `CGB` record, an index
/// leaves the table, the list ends without an end flag, it uses flow control other than IF, or
/// it is over
/// [`MAX_INSTRUCTIONS`].
fn decode(blob: &[u8], end_offsets: &[u32], variant: usize) -> Option<Vec<Instruction>> {
    let end = usize::try_from(*end_offsets.get(variant)?).ok()?;
    let start = match variant {
        0 => 0,
        v => usize::try_from(*end_offsets.get(v - 1)?).ok()?,
    };
    let table_start = usize::try_from(*end_offsets.last()?).ok()?;
    let chunk = blob.get(start..end)?;
    let table = blob.get(table_start..)?;
    if !chunk.starts_with(b"CGB") {
        return None;
    }
    let entries = table.len() / 8;
    let parameters = be16(chunk, 0x30)?;
    let mut at = 0x30usize.checked_add(parameters)?.checked_add(15)? & !15;
    let mut next = || -> Option<[u32; 4]> {
        let a = be16(chunk, at)?;
        let c = be16(chunk, at.checked_add(2)?)?;
        at = at.checked_add(4)?;
        if a >= entries || c >= entries {
            return None;
        }
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&table[a * 8..a * 8 + 8]);
        bytes[8..].copy_from_slice(&table[c * 8..c * 8 + 8]);
        Some([0, 1, 2, 3].map(|i| {
            let w = u32::from_be_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
            w.rotate_left(16)
        }))
    };
    let mut out = vec![];
    // 16-byte slots read so far (instructions and constants), and the slot an IF block ends at.
    let mut slot = 0usize;
    let mut block_end = 0usize;
    while out.len() < MAX_INSTRUCTIONS {
        let w = next()?;
        slot += 1;
        let opcode = (w[0] >> 24 & 0x3f) as u8;
        if w[2] >> 31 & 1 != 0 {
            // Flow control. Only IF (op 2, no sources) is followed: its end (word 3, in 32-bit
            // words of the program; four to a slot) closes a block whose writes count as
            // conditional. CONFIRMED_FROM_LOCAL_DATA: the four variant-8 programs with flow
            // control in the corpus use IF alone, ending at a slot inside the program. The
            // others (loops, calls) make the straight-line trace below unsound.
            let end = usize::try_from(w[3] & 0x7fff_ffff).ok()? / 4;
            if opcode != 2 || w[0] & 1 != 0 || end < slot {
                return None;
            }
            block_end = block_end.max(end);
            continue;
        }
        let in_block = slot <= block_end;
        let count = arity(opcode);
        let all = [source(w[1]), source(w[2]), source(w[3])];
        let destination = (count != 0 && w[0] >> 30 & 1 == 0).then_some(Destination {
            half: w[0] >> 7 & 1 != 0,
            index: (w[0] >> 1 & 0x3f) as u8,
            mask: (w[0] >> 9 & 0xf) as u8,
        });
        if all[..count].iter().any(|s| s.operand == Operand::Constant) {
            // The inline constant; a constant missing after the last instruction is allowed.
            if w[0] & 1 == 0 {
                next()?;
            } else {
                let _ = next();
            }
            slot += 1;
        }
        out.push(Instruction {
            opcode,
            destination,
            sources: all,
            conditional: in_block || w[1] >> 18 & 7 != 7,
        });
        if w[0] & 1 != 0 {
            return Some(out);
        }
    }
    None
}

fn first_component(mask: u8) -> usize {
    (mask.trailing_zeros() as usize).min(3)
}
/// The last unconditional write before `before` of `component` of the temp.
fn defining(
    code: &[Instruction],
    before: usize,
    half: bool,
    index: u8,
    component: u8,
) -> Option<usize> {
    let j = (0..before).rev().find(|&j| {
        code[j]
            .destination
            .is_some_and(|d| d.half == half && d.index == index && d.mask >> component & 1 != 0)
    })?;
    (!code[j].conditional).then_some(j)
}
/// The instruction that wrote source `k` of `code[at]` for its destination `component`, and
/// whether the source is negated.
fn source_definition(
    code: &[Instruction],
    at: usize,
    k: usize,
    component: usize,
) -> Option<(usize, bool)> {
    let i = &code[at];
    let s = i.sources[k];
    let Operand::Temp { half, index } = s.operand else {
        return None;
    };
    let read = if SCALAR.contains(&i.opcode) || (i.opcode == OP_DIV && k == 1) {
        s.swizzle[0]
    } else {
        s.swizzle[component]
    };
    defining(code, at, half, index, read).map(|j| (j, s.negate))
}
fn written_component(i: &Instruction) -> usize {
    i.destination.map_or(0, |d| first_component(d.mask))
}
/// `1 + 4 |g|^2`: an ADD of a DP3 result and a constant.
fn glow_sum(code: &[Instruction], at: Option<usize>) -> bool {
    let Some(at) = at else {
        return false;
    };
    let i = &code[at];
    let c = written_component(i);
    i.opcode == OP_ADD
        && (0..2).any(|k| {
            i.sources[1 - k].operand == Operand::Constant
                && source_definition(code, at, k, c).is_some_and(|(d, _)| code[d].opcode == OP_DP3)
        })
}
fn fogged_mix(code: &[Instruction], at: usize) -> bool {
    let i = &code[at];
    (0..2).any(|k| {
        if i.sources[1 - k].operand != Operand::Constant {
            return false;
        }
        let Some((sum, _)) = source_definition(code, at, k, 3) else {
            return false;
        };
        if code[sum].opcode != OP_ADD {
            return false;
        }
        let c = written_component(&code[sum]);
        let d0 = source_definition(code, sum, 0, c).map(|d| d.0);
        let d1 = source_definition(code, sum, 1, c).map(|d| d.0);
        [(d0, d1), (d1, d0)].into_iter().any(|pair| {
            let (Some(r), Some(m)) = pair else {
                return false;
            };
            code[r].opcode == OP_RCP
                && code[m].opcode == OP_MAD
                && glow_sum(code, source_definition(code, r, 0, 0).map(|d| d.0))
                && source_definition(code, m, 0, written_component(&code[m])) == Some((r, true))
        })
    })
}
fn classify(code: &[Instruction]) -> GlowAlphaForm {
    let Some(last) = code.iter().rposition(|i| {
        i.destination
            .is_some_and(|d| d.half && d.index == 0 && d.mask & 8 != 0)
    }) else {
        return GlowAlphaForm::Unknown;
    };
    if code[last].conditional {
        return GlowAlphaForm::Unknown;
    }
    let unfogged = match code[last].opcode {
        OP_RCP => glow_sum(code, source_definition(code, last, 0, 3).map(|d| d.0)),
        OP_DIV => glow_sum(code, source_definition(code, last, 1, 3).map(|d| d.0)),
        OP_MUL if fogged_mix(code, last) => return GlowAlphaForm::Fogged,
        _ => false,
    };
    if unfogged {
        GlowAlphaForm::Unfogged
    } else {
        GlowAlphaForm::Unknown
    }
}

/// The glow alpha's form in program [`GLOW_ALPHA_VARIANT`] of a shader blob (the blob and its
/// ten end offsets as the material prefix stores them).
pub fn glow_alpha_form(blob: &[u8], end_offsets: &[u32]) -> GlowAlphaForm {
    decode(blob, end_offsets, GLOW_ALPHA_VARIANT).map_or(GlowAlphaForm::Unknown, |c| classify(&c))
}

#[cfg(test)]
mod tests {
    use super::{glow_alpha_form, GlowAlphaForm};

    // Synthetic programs built from the documented field layout; no game bytes.
    const ALWAYS: u32 = 7 << 18;
    fn swz(s: &str) -> u32 {
        s.bytes()
            .enumerate()
            .map(|(i, c)| {
                u32::from(b"xyzw".iter().position(|&x| x == c).unwrap() as u8) << (9 + 2 * i)
            })
            .sum()
    }
    fn temp(half: bool, index: u32, s: &str, negate: bool) -> u32 {
        index << 2 | u32::from(half) << 8 | swz(s) | u32::from(negate) << 17
    }
    fn h(index: u32, s: &str) -> u32 {
        temp(true, index, s, false)
    }
    fn r(index: u32, s: &str) -> u32 {
        temp(false, index, s, false)
    }
    fn constant(s: &str) -> u32 {
        2 | swz(s)
    }
    fn mask(m: &str) -> u32 {
        m.bytes()
            .map(|c| 1 << b"xyzw".iter().position(|&x| x == c).unwrap())
            .sum::<u32>()
    }
    struct Op {
        words: [u32; 4],
        constant: bool,
    }
    fn op(opcode: u32, half: bool, index: u32, m: &str, sources: [u32; 3]) -> Op {
        Op {
            words: [
                opcode << 24 | mask(m) << 9 | u32::from(half) << 7 | index << 1,
                sources[0] | ALWAYS,
                sources[1],
                sources[2],
            ],
            constant: sources.iter().any(|s| s & 3 == 2),
        }
    }
    /// Variant 8 holds `ops` (the last one ends the program); the other variants are empty.
    fn blob(ops: Vec<Op>) -> (Vec<u8>, Vec<u32>) {
        let mut program = b"CGB\0".to_vec();
        program.resize(0x30, 0);
        program.extend([0, 0x10]); // parameter block size: instructions at 0x40
        program.resize(0x40, 0);
        let mut table: Vec<u8> = vec![];
        let n = ops.len();
        let mut push = |program: &mut Vec<u8>, words: [u32; 4]| {
            let bytes: Vec<u8> = words
                .iter()
                .flat_map(|w| w.rotate_left(16).to_be_bytes())
                .collect();
            let entry = (table.len() / 8) as u16;
            table.extend(&bytes);
            program.extend(entry.to_be_bytes());
            program.extend((entry + 1).to_be_bytes());
        };
        for (k, mut o) in ops.into_iter().enumerate() {
            if k + 1 == n {
                o.words[0] |= 1;
            }
            push(&mut program, o.words);
            if o.constant {
                push(&mut program, [0x3f80_0000, 0, 0, 0]);
            }
        }
        let end = program.len() as u32;
        let mut ends = vec![0; 8];
        ends.extend([end, end]);
        program.extend(table);
        (program, ends)
    }
    const MUL: u32 = 0x02;
    const ADD: u32 = 0x03;
    const MAD: u32 = 0x04;
    const DP3: u32 = 0x05;
    const MOV: u32 = 0x01;
    const RCP: u32 = 0x1a;
    const DIV: u32 = 0x3a;
    /// `s = 1 + 4 |g|^2` into `R0.w` (or `H3.w`).
    fn sum(into_h3: bool) -> Vec<Op> {
        vec![
            op(DP3, false, 0, "z", [constant("xyzw"), constant("xyzw"), 0]),
            if into_h3 {
                op(ADD, true, 3, "w", [r(0, "zzzz"), constant("yyyy"), 0])
            } else {
                op(ADD, false, 0, "w", [r(0, "zzzz"), constant("yyyy"), 0])
            },
        ]
    }
    fn fogged(negate: bool) -> Vec<Op> {
        let mut ops = sum(false);
        ops.extend([
            op(RCP, false, 0, "z", [r(0, "wwww"), 0, 0]),
            op(MOV, false, 1, "w", [1 | swz("zzzz"), 0, 0]),
            op(
                MAD,
                false,
                2,
                "w",
                [temp(false, 0, "zzzz", negate), r(1, "wwww"), r(1, "wwww")],
            ),
            op(ADD, true, 1, "x", [r(0, "zzzz"), r(2, "wwww"), 0]),
            op(MUL, true, 0, "w", [h(1, "xxxx"), constant("xyzw"), 0]),
        ]);
        ops
    }

    #[test]
    fn d309_fog_mix_of_the_glow_reciprocal_is_fogged() {
        let (b, e) = blob(fogged(true));
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Fogged);
        // Without the MAD's negation it is not `a + f (1 - a)`.
        let (b, e) = blob(fogged(false));
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
    }

    #[test]
    fn d309_division_or_bare_reciprocal_is_unfogged() {
        let mut ops = sum(true);
        ops.push(op(DIV, true, 0, "w", [constant("xyzw"), h(3, "wwww"), 0]));
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unfogged);
        let mut ops = sum(true);
        ops.push(op(RCP, true, 0, "w", [h(3, "wwww"), 0, 0]));
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unfogged);
        // A reciprocal of something that is not the glow's sum.
        let ops = vec![
            op(MOV, true, 3, "w", [constant("xxxx"), 0, 0]),
            op(RCP, true, 0, "w", [h(3, "wwww"), 0, 0]),
        ];
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
    }

    #[test]
    fn d309_other_programs_and_bad_blobs_are_unknown() {
        let (b, e) = blob(vec![op(MOV, true, 0, "xyzw", [constant("xyzw"), 0, 0])]);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
        // Only the full-precision R0.w is written: no output alpha write.
        let mut ops = sum(true);
        ops.push(op(RCP, false, 0, "w", [h(3, "wwww"), 0, 0]));
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
        // Not a CGB record, an index past the table, no end flag, empty input.
        let (mut b, e) = blob(fogged(true));
        b[0] = b'X';
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
        let (mut b, e) = blob(fogged(true));
        b[0x40] = 0xff;
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
        let (b, e) = blob(fogged(true));
        // The last instruction and its constant cut off.
        let cut = e[8] as usize - 8;
        let truncated: Vec<u8> = [&b[..cut], &b[e[8] as usize..]].concat();
        let ends: Vec<u32> = e.iter().map(|&x| if x == 0 { 0 } else { x - 8 }).collect();
        assert_eq!(glow_alpha_form(&truncated, &ends), GlowAlphaForm::Unknown);
        assert_eq!(glow_alpha_form(&[], &[]), GlowAlphaForm::Unknown);
        assert_eq!(glow_alpha_form(&b, &e[..8]), GlowAlphaForm::Unknown);
    }

    #[test]
    fn d309_conditional_or_flow_control_is_unknown() {
        let mut ops = sum(true);
        ops.push(op(DIV, true, 0, "w", [constant("xyzw"), h(3, "wwww"), 0]));
        ops[1].words[1] &= !(7 << 18); // the sum written only if a condition holds
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
        // Flow control other than IF (here a flagged DP3).
        let mut ops = fogged(true);
        ops[0].words[2] |= 1 << 31;
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
    }

    /// IF (op 2, flow bit) whose block ends at slot `end` (word 3 counts 32-bit words).
    fn if_block(end: u32) -> Op {
        Op {
            words: [2 << 24, ALWAYS, (1 << 31) | (end * 4), end * 4],
            constant: false,
        }
    }

    #[test]
    fn d309_writes_inside_an_if_block_are_conditional() {
        // An empty block before the chain leaves it unconditional.
        let mut ops = vec![if_block(1)];
        ops.extend(fogged(true));
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Fogged);
        // A block holding the DP3 (slots 1 and 2, its constant) breaks the glow's sum.
        let mut ops = vec![if_block(3)];
        ops.extend(fogged(true));
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
        // A block ending before itself is refused.
        let mut ops = vec![op(MOV, true, 2, "x", [constant("xxxx"), 0, 0]), if_block(1)];
        ops.extend(fogged(true));
        let (b, e) = blob(ops);
        assert_eq!(glow_alpha_form(&b, &e), GlowAlphaForm::Unknown);
    }
}
