//! D331 (docs/level-globals.md): the level's colour correction setting in the final pass.
//!
//! FOUND (01.33): the setting is a position over eight preset records (0x316a7c: record
//! floor(v) blended with the next by the fraction, 7 from 7 up; a negative value blends
//! records 0 and 1 by `v - floor(v)`). The world keeps one record for the blend's start and
//! one for its target and blends the two by the lighting group's factor (0xda43c, 0xd978c).
//! A record holds two colour transforms (a tint and saturation, hue, contrast, brightness),
//! a picked colour and the vignette's top and bottom colours. The final pass (0x323354) turns
//! each transform into a 4-row colour matrix (0xe6e04) and its shader (PS 14857) uses them as
//!
//! `g = lerp((c, 1) x M0, (c, 1) x M1, saturate(pick.w x |c - pick.rgb|))`, then
//! `g x lerp(top + v x (bottom - top), 1, smoothstep(0, 1, saturate(1.2 - r)))`
//!
//! on `c = scene x 1.2998 + glow` (encoded values; M0 near the picked colour, M1 away from
//! it). Record 0 is the identity with a 0.2 grey vignette: part 45's frame. FOUND: which
//! shader constant holds each value (the live constants of maingrapple, read from RPCS3).
//!
//! MEASURED, not FOUND: the vignette's top colour is at the top of the frame (v = 0), from the
//! game frames' top and bottom bands. Taken as clear: the bss flag at 0xe31e34 that scales
//! saturation by 0.8.
use bevy::{
    prelude::*,
    render::extract_resource::{ExtractResource, ExtractResourcePlugin},
};
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};

/// One colour transform of a record as stored: tint (rgb, weight w), then saturation, hue
/// (radians), contrast, brightness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub tint: [f32; 4],
    pub shcb: [f32; 4],
}

/// One colour-correction record (0x70 bytes, 28 floats in this order).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub matrix: [Transform; 2],
    pub pick: [f32; 4],
    pub top: [f32; 4],
    pub bottom: [f32; 4],
}

const IDENTITY: Transform = Transform {
    tint: [0., 0., 0., 1.],
    shcb: [1., 0., 1., 0.],
};
const GREY: [f32; 4] = [0.2, 0.2, 0.2, 1.];

impl Record {
    pub const IDENTITY: Self = Self {
        matrix: [IDENTITY; 2],
        pick: [0.; 4],
        top: GREY,
        bottom: GREY,
    };

    pub fn from_floats(f: &[f32; 28]) -> Self {
        let v = |i: usize| [f[i], f[i + 1], f[i + 2], f[i + 3]];
        let t = |i: usize| Transform {
            tint: v(i),
            shcb: v(i + 4),
        };
        Self {
            matrix: [t(0), t(8)],
            pick: v(16),
            top: v(20),
            bottom: v(24),
        }
    }

    fn floats(&self) -> [f32; 28] {
        let mut f = [0.; 28];
        let parts = [
            self.matrix[0].tint,
            self.matrix[0].shcb,
            self.matrix[1].tint,
            self.matrix[1].shcb,
            self.pick,
            self.top,
            self.bottom,
        ];
        for (k, p) in parts.iter().enumerate() {
            f[k * 4..k * 4 + 4].copy_from_slice(p);
        }
        f
    }

    /// Every float `a + (b - a) x t`.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let (a, b) = (self.floats(), other.floats());
        Self::from_floats(&std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t))
    }

    /// FOUND (0x316a7c): the record at setting `v`.
    pub fn at(table: &[Record; 8], v: f32) -> Self {
        let f = v.floor();
        let (lo, hi) = if f < 0. {
            (0, 1)
        } else if f >= 7. {
            (7, 7)
        } else {
            (f as usize, f as usize + 1)
        };
        table[lo].lerp(&table[hi], v - f)
    }
}

/// FOUND (0xe6e04): the transform as the 4-row matrix the shader multiplies `(c, 1)` by:
/// rows 0..2 are the outputs' weights of input r, g, b, row 3 the offset. The steps, applied
/// to a colour in this order: hue (a rotation by h about the grey axis, only when h != 0),
/// saturation (luminance weights 0.2, 0.7, 0.1), contrast about 0.5 after the brightness
/// add, the tint `c x (w + (1 - w) t) + w t`.
pub fn matrix(t: &Transform) -> [[f32; 4]; 4] {
    let [s, h, contrast, brightness] = t.shcb;
    let [tr, tg, tb, w] = t.tint;
    let tint = [tr, tg, tb];
    let (sin, cos) = h.sin_cos();
    let k = 1. / 3f32.sqrt();
    let apply = |c: [f32; 3]| -> [f32; 3] {
        let c = if h != 0. {
            let g = (1. - cos) / 3. * (c[0] + c[1] + c[2]);
            // cos h c + sin h (u x c) + (1 - cos h)(u . c) u, u = (1, 1, 1) / sqrt 3.
            let x = [c[2] - c[1], c[0] - c[2], c[1] - c[0]];
            std::array::from_fn(|i| cos * c[i] + sin * k * x[i] + g)
        } else {
            c
        };
        let l = 0.2 * c[0] + 0.7 * c[1] + 0.1 * c[2];
        std::array::from_fn(|i| {
            let v = s * c[i] + (1. - s) * l;
            let v = contrast * (v + brightness - 0.5) + 0.5;
            v * (w + (1. - w) * tint[i]) + w * tint[i]
        })
    };
    let o = apply([0.; 3]);
    let mut m = [
        [0., 0., 0., 0.],
        [0., 0., 0., 0.],
        [0., 0., 0., 0.],
        [o[0], o[1], o[2], 1.],
    ];
    for (i, row) in m.iter_mut().take(3).enumerate() {
        let mut e = [0.; 3];
        e[i] = 1.;
        let v = apply(e);
        *row = [v[0] - o[0], v[1] - o[1], v[2] - o[2], 0.];
    }
    m
}

/// The final pass's colour-correction constants, as the shader's uniform reads them.
#[derive(Resource, Clone, Copy, Debug, PartialEq, ExtractResource)]
pub struct ColourGrade {
    pub record: Record,
}

impl Default for ColourGrade {
    fn default() -> Self {
        Self {
            record: Record::IDENTITY,
        }
    }
}

/// Bytes of [`ColourGrade::uniform`].
pub const UNIFORM_SIZE: u64 = 44 * 4;

impl ColourGrade {
    /// The uniform: M0's four rows, M1's, the picked colour, top, bottom (44 floats).
    pub fn uniform(&self) -> Vec<u8> {
        let r = &self.record;
        let mut f: Vec<f32> = Vec::with_capacity(44);
        for t in &r.matrix {
            f.extend(matrix(t).iter().flatten());
        }
        f.extend(r.pick.iter().chain(&r.top).chain(&r.bottom));
        f.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
}

/// The WGSL the final entries call: the struct, its binding and `graded`.
pub const WGSL: &str = r#"
struct ColourGrade {
    m0: array<vec4<f32>, 4>,
    m1: array<vec4<f32>, 4>,
    pick: vec4<f32>,
    top: vec4<f32>,
    bottom: vec4<f32>,
}
@group(0) @binding(5) var<uniform> grade: ColourGrade;
fn grade_matrix(m: array<vec4<f32>, 4>, c: vec3<f32>) -> vec3<f32> {
    return (c.r * m[0] + c.g * m[1] + c.b * m[2] + m[3]).rgb;
}
// D331: the two matrices blended by the distance to the picked colour, then the vignette
// towards its top / bottom colour.
fn graded(c: vec3<f32>, position: vec2<f32>) -> vec3<f32> {
    let t = saturate(grade.pick.w * length(c - grade.pick.rgb));
    let g = mix(grade_matrix(grade.m0, c), grade_matrix(grade.m1, c), t);
    let v = position.y / f32(textureDimensions(source_texture).y);
    let edge = mix(grade.top.rgb, grade.bottom.rgb, v);
    return g * mix(edge, vec3<f32>(1.0), vignette(position));
}
"#;

/// FOUND (01.33, loader 0x5e0814, parser 0x5df7e8): the presets' file (GUID 31250): a
/// revision word, a 0 word, then eight records of 28 big-endian floats. The Pod's table
/// (`colcorrectpresetspod.raw`, GUID 127928) holds the same records in 01.33.
pub const PRESETS_PATH: &str = "gamedata/data/colcorrectpresets.raw";
const PRESETS_BYTES: usize = 8 + 8 * 28 * 4;

/// The eight records from the presets file's bytes.
pub fn parse_presets(bytes: &[u8]) -> Result<[Record; 8], String> {
    if bytes.len() < PRESETS_BYTES {
        return Err(format!(
            "{PRESETS_PATH}: {} bytes, under {PRESETS_BYTES}",
            bytes.len()
        ));
    }
    let float = |i: usize| {
        let o = 8 + i * 4;
        f32::from_be_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
    };
    Ok(std::array::from_fn(|r| {
        Record::from_floats(&std::array::from_fn(|k| float(r * 28 + k)))
    }))
}

/// Reads the presets file from the game's archives.
pub fn load_presets(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> Result<[Record; 8], String> {
    let row = catalog
        .snapshot
        .resources
        .iter()
        .find(|r| r.map_row.path.eq_ignore_ascii_case(PRESETS_PATH))
        .ok_or_else(|| format!("{PRESETS_PATH}: no MAP row"))?;
    let key = lbp_preview::resolve(catalog, None, Some(&row.map_row.sha1))?;
    let bytes = lbp_preview::level_scene::read_resource(source, &key, control, PRESETS_BYTES)?;
    parse_presets(&bytes)
}

/// The setting's record from the presets as the world holds it: the start and target
/// records blended by the lighting group's factor.
pub fn record_of(table: &[Record; 8], from: f32, to: f32, blend: f32) -> Record {
    Record::at(table, from).lerp(&Record::at(table, to), blend)
}

/// The presets as read for the level played (none when the file could not be read).
#[derive(Resource, Default)]
pub struct Presets(pub Option<[Record; 8]>);

pub struct ColourGradePlugin;
impl Plugin for ColourGradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ColourGrade>()
            .add_plugins(ExtractResourcePlugin::<ColourGrade>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) {
        for (ra, rb) in a.iter().zip(&b) {
            for (x, y) in ra.iter().zip(rb) {
                assert!((x - y).abs() < 2e-5, "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn d331_identity_record_is_identity() {
        let id = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        close(matrix(&IDENTITY), id);
    }

    /// The matrices 0xe6e04 itself gives (an emulation of its instructions, 2026-10-16) for
    /// three transforms of the presets.
    #[test]
    fn d331_matrices_match_the_game() {
        let t = |tint, shcb| Transform { tint, shcb };
        close(
            matrix(&t(
                [0.49865, 0.25974, 0.096407, 0.68088],
                [0.13402, 0., 1.6701, -0.041237],
            )),
            [
                [0.430993, 0.220924, 0.205847, 0.],
                [0.850418, 0.944184, 0.720464, 0.],
                [0.121488, 0.110462, 0.262209, 0.],
                [0.000224, -0.131649, -0.221806, 1.],
            ],
        );
        close(
            matrix(&t([0., 0., 0., 1.], [0., 0., 2.2062, 0.12371])),
            [
                [0.44124, 0.44124, 0.44124, 0.],
                [1.54434, 1.54434, 1.54434, 0.],
                [0.22062, 0.22062, 0.22062, 0.],
                [-0.330171, -0.330171, -0.330171, 1.],
            ],
        );
        close(
            matrix(&t(
                [0.21121, 0.40089, 0.49041, 1.],
                [0.25773, 0.45343, 2.2887, -0.10309],
            )),
            [
                [1.170575, 0.789497, 0.491132, 0.],
                [0.95396, 1.633403, 1.252325, 0.],
                [0.164165, -0.1342, 0.545243, 0.],
                [-0.669082, -0.479402, -0.389882, 1.],
            ],
        );
    }

    #[test]
    fn d331_setting_picks_and_blends_records() {
        let mut table = [Record::IDENTITY; 8];
        for (i, r) in table.iter_mut().enumerate() {
            r.pick = [i as f32; 4];
        }
        assert_eq!(Record::at(&table, 2.25).pick[0], 2.25);
        assert_eq!(Record::at(&table, 7.0).pick[0], 7.);
        assert_eq!(Record::at(&table, 9.5).pick[0], 7.);
        // Below 0: records 0 and 1 by v - floor(v).
        assert_eq!(Record::at(&table, -0.75).pick[0], 0.25);
        assert_eq!(record_of(&table, 1., 3., 0.5).pick[0], 2.);
    }

    #[test]
    fn d331_presets_file_layout() {
        let mut bytes = vec![0, 0, 3, 0x7a, 0, 0, 0, 0];
        for r in 0..8 {
            for k in 0..28 {
                bytes.extend((r as f32 * 100. + k as f32).to_be_bytes());
            }
        }
        let t = parse_presets(&bytes).unwrap();
        assert_eq!(t[0].matrix[0].tint, [0., 1., 2., 3.]);
        assert_eq!(t[2].matrix[1].shcb, [212., 213., 214., 215.]);
        assert_eq!(t[7].bottom, [724., 725., 726., 727.]);
        assert!(parse_presets(&bytes[..900]).is_err());
    }

    #[test]
    fn d331_uniform_layout() {
        let g = ColourGrade::default();
        let u = g.uniform();
        assert_eq!(u.len() as u64, UNIFORM_SIZE);
        let f: Vec<f32> = u
            .chunks(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(&f[0..4], &[1., 0., 0., 0.]);
        assert_eq!(&f[28..32], &[0., 0., 0., 1.]);
        assert_eq!(&f[32..36], &[0.; 4]);
        assert_eq!(&f[36..40], &GREY);
    }

    /// The constants the game wrote into PS 14857 in Cosmos `main_03_maingrapple` (setting
    /// 5.77), read from RPCS3 guest memory on 2026-10-16: M0 rows fc19, fc17, fc23, fc25,
    /// M1 rows fc20, fc14, fc21, fc22, pick fc16/fc24, top fc2/fc12, bottom fc8.
    #[test]
    fn d331_record_matches_the_live_constants() {
        let r5 = [
            0.27112, 0.038254, 0.0042362, 1., 0.97938, 0., 1.9794, 0.12371, 0.49656, 0.32288,
            0.20506, 1., 0.69072, 0., 1.8763, 0., 0.022281, 0.022281, 0.022281, 1.1783, 0.89272,
            -0.034991, -0.063226, 1., 0.126, -0.042453, -0.026925, 1.,
        ];
        let r6 = [
            0.38307, -0.043775, 0.11366, 1., 1., 0., 1.7732, 0., 0.15958, 0.41416, 0.21335, 1.,
            0.29897, 0., 1.6701, -0.041237, -0.075631, -0.075631, 0.031297, 1.0749, 0.49821,
            0.43492, 0.1821, 1., 0.21169, -0.042894, 0.1133, 1.,
        ];
        let mut table = [Record::IDENTITY; 8];
        table[5] = Record::from_floats(&r5);
        table[6] = Record::from_floats(&r6);
        let r = Record::at(&table, 5.77);
        let near = |a: &[f32], b: &[f32]| {
            for (x, y) in a.iter().zip(b) {
                assert!((x - y).abs() < 1e-3, "{a:?} vs {b:?}");
            }
        };
        let (m0, m1) = (matrix(&r.matrix[0]), matrix(&r.matrix[1]));
        near(&m0[0], &[1.8137, 0.0017268, 0.0017268]);
        near(&m0[1], &[0.0060437, 1.818, 0.0060437]);
        near(&m0[2], &[0.00086339, 0.00086339, 1.8128]);
        near(&m0[3], &[-0.0011806, -0.38341, -0.27001]);
        near(&m1[0], &[0.8781, 0.20986, 0.20986]);
        near(&m1[1], &[0.7345, 1.4027, 0.7345]);
        near(&m1[2], &[0.10493, 0.10493, 0.77317]);
        near(&m1[3], &[-0.17621, -0.02013, -0.20186]);
        near(&r.pick, &[-0.053111, -0.053111, 0.029223, 1.0987]);
        near(&r.top, &[0.58895, 0.32684, 0.12567, 1.]);
        near(&r.bottom, &[0.19198, -0.042793, 0.081048, 1.]);
    }
}
