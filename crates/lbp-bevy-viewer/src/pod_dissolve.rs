//! D278 (docs/level-scripts.md, part 29): the numbers of the Pod tent's top-down
//! dissolve, and the arithmetic that turns them into what the shader takes
//! (`dual::Dissolve`). Everything here is in the shell level's own units, y up.
//!
//! What the game does, read from the pod's three materials and two captured frames
//! (the evidence stays local; part 29 has the account). While the pod's change-over
//! value `v` is between 0 and 1 (the alpha of the pod Thing's shape colour is `1 - v`):
//!
//! - a tent pixel at height `y` is cut when `y - camera y` is above
//!   [`LINE_AT_ZERO`] `-` [`LINE_FALL`] `* v`, plus up to [`GREEN_RISE`] by the green
//!   of the surface's own colour (a ragged edge);
//! - in the [`BAND`] units under the line the colour goes from the lit colour toward
//!   the flat player colour, by the edge value and the red of the transition mask
//!   sampled at UV times [`MASK_TILES`]; at an edge value above [`FLAT_ABOVE`] it is all
//!   flat colour. The mask moves no edge.
//!
//! FOUND: the rule and its constants. MEASURED at the two end states only: the camera
//! is the planets level's script camera (its eye). INFERRED: that the shell level's
//! frame and that camera's frame are one frame in the game, and so that the camera y
//! between the end states is the script camera's too.
//!
//! D284 (part 35): the second mask of the two card materials, `pod_mask.tex`
//! ([`POD_MASK`]): where its red is over one half the line stands [`ISLAND_RISE`] band
//! higher, and at `v` = 1 (the platform's card edge, the only material of the platform
//! with the rule) only those parts are drawn ([`edge`]). Both masks are sampled at the
//! mesh's raw UV0 (FOUND: the captured draws have an identity UV transform); which
//! input of a draw bucket holds raw UV0 is [`uv0_source`].
//!
//! Left out: the glass material's constant in place of its colour's green, the fog over
//! the flat colour, the player's own colour as the flat colour (part 30).
use crate::dual::Dissolve;
use bevy::prelude::*;

/// GUID of `transition_mask.tex`, the mask the pod's script names for every backdrop
/// but six (FOUND; the other six masks are not loaded).
pub const TRANSITION_MASK: u32 = 132_243;
/// D284: GUID of `pod_mask.tex`, texture slot 6 of the two card materials (FOUND).
pub const POD_MASK: u32 = 0x0001_fbc7;
/// D284: the card materials that sample [`POD_MASK`]: `pod_card` and `pod_card_edge`
/// (FOUND; `window_glassy` has no slot 6).
pub const CARD: u32 = 0x0000_2f6a;
pub const CARD_EDGE: u32 = 0x0000_2fda;
/// D284: `pod_platform.mol`, the mesh the script puts on the pod Thing at `v` = 1
/// (FOUND, part 27). Of its two materials only [`CARD_EDGE`] has the rule.
pub const PLATFORM_MESH: u32 = 0x0002_3959;
/// D284: how many bands higher the line stands where `pod_mask` is white, and how many
/// bands the `v` = 1 term adds where it is black (FOUND: one each).
pub const ISLAND_RISE: f32 = 1.;
/// The line's height over the camera at `v` = 0, level units (FOUND).
pub const LINE_AT_ZERO: f32 = 1150.;
/// How far the line falls over `v` 0..1, level units (FOUND).
pub const LINE_FALL: f32 = 1550.;
/// How much higher the line stands where the surface colour's green is 1 (FOUND for the
/// green of the material's diffuse product; the viewer's surface colour before lighting
/// stands in for it).
pub const GREEN_RISE: f32 = 100.;
/// Height of the tinted band under the line, level units (FOUND).
pub const BAND: f32 = 400.;
/// The edge value (1 on the line, 0 one band below) above which the band is all flat
/// colour (FOUND, a half float in the material).
pub const FLAT_ABOVE: f32 = 0.990_23;
/// Mask tiles per unit of UV (FOUND).
pub const MASK_TILES: f32 = 40.;
/// Size of one mask tile, level units, where a draw bucket has no usable UV and the
/// mask is laid over the position instead. PLACEHOLDER: the tent's height over
/// [`MASK_TILES`]; the game always samples by UV.
pub const POSITION_TILE: f32 = 70.;

/// The change-over value a shape colour's alpha stands for, when the dissolve is on:
/// strictly between 0 and 1. At alpha 1 (`v` = 0) the game's rule is off, and at alpha 0
/// (`v` = 1) the script has swapped the tent for the platform.
pub fn value(alpha: f32) -> Option<f32> {
    let v = 1. - alpha;
    (v > 0. && v < 1.).then_some(v)
}

/// D284: the value of the platform's card edge, drawn with the rule's `v` = 1 term while
/// the shape colour's alpha is 0 (FOUND: the alpha-0 draw of the captured frame); none
/// otherwise.
pub fn value_at_one(alpha: f32) -> Option<f32> {
    (1. - alpha >= 1.).then_some(1.)
}

/// D284: the edge value of the game's rule, the shader's `edge` (1 on the line, 0 one
/// band under it; above 1 the fragment is cut). `above` is the height over the line
/// ([`line_y`]), `g` the green of the surface colour, `m6` 1 where `pod_mask` is white
/// and `at_one` 1 at `v` = 1:
/// `(above - 100 g) / 400 + 1 - m6 + at_one (1 - m6)`. The CPU mirror the tests hold the
/// shader's arithmetic to.
#[cfg(test)]
pub fn edge(above: f32, g: f32, m6: f32, at_one: f32) -> f32 {
    (above - GREEN_RISE * g) / BAND + 1. - ISLAND_RISE * m6 + at_one * ISLAND_RISE * (1. - m6)
}

/// D284: which input of a draw bucket holds the mesh's raw UV0, for the masks: 1 =
/// `in.uv` (the primary coordinates are raw UV0), 2 = `in.uv_b` (the second sample's
/// coordinates are), 0 = neither (the mask is laid over the position,
/// [`POSITION_TILE`]). Each argument is the raw channel of that coordinate set and
/// whether the set is that channel unchanged; none when the bucket has no such set.
pub fn uv0_source(primary: Option<(usize, bool)>, second: Option<(usize, bool)>) -> u8 {
    match (primary, second) {
        (Some((0, true)), _) => 1,
        (_, Some((0, true))) => 2,
        _ => 0,
    }
}

/// The line's height in the level for a camera at height `camera_y`.
pub fn line_y(camera_y: f32, v: f32) -> f32 {
    camera_y + LINE_AT_ZERO - LINE_FALL * v
}

/// A plane of the level, `dot(xyz, p) + w` for a level point `p`, as the same function
/// of a Bevy world point when `level_to_world` takes level points to world points. None
/// when the matrix has no inverse.
pub fn world_plane(level_plane: Vec4, level_to_world: Mat4) -> Option<Vec4> {
    let back = level_to_world.inverse();
    let plane = back.transpose() * level_plane;
    (back.is_finite() && plane.is_finite()).then_some(plane)
}

/// The constants as the shader takes them (`dual::Dissolve::consts`).
pub fn consts() -> Vec4 {
    Vec4::new(GREEN_RISE, BAND, FLAT_ABOVE, MASK_TILES)
}

/// What the shader takes for change-over value `v`: the line as a plane in world
/// space whose value is the height above the line in level units (so the band and the
/// ragged edge keep their size under any scale of the level's root), the flat colour,
/// and the mask's position coordinates. `colour` is the pod's shape colour as stored
/// (INFERRED: a display value, so it is decoded to linear here and shows as it is).
/// D284: `island` is `pod_mask` for the card materials (none for the glass); at `v` = 1
/// the rule's last term is on.
pub fn uniforms(
    v: f32,
    colour: [f32; 3],
    camera_y: f32,
    level_to_world: Mat4,
    mask: Option<Handle<Image>>,
    island: Option<Handle<Image>>,
) -> Option<Dissolve> {
    let plane = |p: Vec4| world_plane(p, level_to_world);
    let tile = 1. / POSITION_TILE;
    let [r, g, b] = colour.map(|c| c.clamp(0., 1.));
    Some(Dissolve {
        plane: plane(Vec4::new(0., 1., 0., -line_y(camera_y, v)))?,
        colour: Color::srgb(r, g, b).to_linear().to_vec3(),
        consts: consts(),
        // Across x and z together, so that a wall along either axis gets a pattern.
        mask_u: plane(Vec4::new(tile, 0., tile, 0.))?,
        mask_v: plane(Vec4::new(0., tile, 0., 0.))?,
        mask,
        island,
        at_one: v >= 1.,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The line of the game's rule: off at the ends, 450 under the tent's top on the
    /// first tick, at the platform's edge on the last tick before the mesh swap.
    #[test]
    fn d278_the_line_falls_with_the_value() {
        assert_eq!(value(1.), None);
        assert_eq!(value(0.), None);
        assert_eq!(value(1.5), None);
        assert!(value(f32::NAN).is_none());
        assert!((value(0.95).unwrap() - 0.05).abs() < 1e-6);
        // The camera at rest in the closed Pod is at y -250 (MEASURED).
        assert!((line_y(-250., 0.05) - 822.5).abs() < 1e-3);
        assert!((line_y(-250., 0.5) - 125.).abs() < 1e-3);
        assert!((line_y(-250., 0.95) + 572.5).abs() < 1e-3);
        // The line moves with the camera.
        assert!((line_y(-300., 0.5) - 75.).abs() < 1e-3);
        assert_eq!(consts(), Vec4::new(100., 400., 0.990_23, 40.));
    }

    /// The world-space plane gives, for the world image of a level point, the level
    /// plane's value at that point: under a root that is turned, scaled and moved, and
    /// under the level's display matrix. A matrix without an inverse gives none.
    #[test]
    fn d278_the_world_plane_measures_in_level_units() {
        let normalize = Mat4::from_scale(Vec3::splat(0.001))
            * Mat4::from_translation(Vec3::new(-20., 150., -170.));
        let root = Transform {
            translation: Vec3::new(0.3, -0.4, 0.2),
            rotation: Quat::from_euler(EulerRot::YXZ, 0.4, -0.3, 0.1),
            scale: Vec3::splat(6.08),
        };
        let to_world = root.to_matrix() * normalize;
        let line = line_y(-275., 0.5);
        let d = uniforms(0.5, [1., 0.5, 0.], -275., to_world, None, None).unwrap();
        for p in [
            Vec3::new(0., line, 0.),
            Vec3::new(700., line + 400., -300.),
            Vec3::new(-1200., -1525., 168.9),
            Vec3::new(19.1, 1282., 900.),
        ] {
            let w = to_world.transform_point3(p).extend(1.);
            let above = d.plane.dot(w);
            assert!((above - (p.y - line)).abs() < 0.05, "{p}: {above}");
            assert!((d.mask_u.dot(w) - (p.x + p.z) / POSITION_TILE).abs() < 1e-2);
            assert!((d.mask_v.dot(w) - p.y / POSITION_TILE).abs() < 1e-2);
        }
        // The flat colour is decoded: it shows as the stored value after the output's
        // encoding. Black stays black.
        assert!((d.colour.x - 1.).abs() < 1e-6 && (d.colour.y - 0.214).abs() < 1e-3);
        assert_eq!(d.colour.z, 0.);
        assert_eq!(d.consts, consts());
        assert!(uniforms(0.5, [0.; 3], 0., Mat4::ZERO, None, None).is_none());
        assert!(world_plane(Vec4::Y, Mat4::IDENTITY) == Some(Vec4::Y));
        assert!(!d.at_one && d.island.is_none());
    }

    /// D284: the edge value against the game's rule `E = (e / 400 - m6) + (v >= 1)(1 -
    /// m6)` with `e = TC4.y + 1550 v - 750 - 100 g` (TC4.y = level y - camera y, part 29):
    /// for v under 1 and at v = 1, pod_mask white and black. At v = 1 a fragment is cut
    /// above camera y - 800 + 100 g + 800 m6 (pod_dissolve-notes.md, the v = 1 table).
    #[test]
    fn d284_edge_matches_the_shader_rule() {
        let camera_y = -299.48;
        let game = |y: f32, v: f32, g: f32, m6: f32| {
            let e = (y - camera_y) + 1550. * v - 750. - 100. * g;
            let a = 1. - v;
            let lt = f32::from(u8::from(a < 1.));
            let at_one = f32::from(u8::from(v >= 1.));
            (e * 0.0025 - m6) * lt + at_one * (1. - m6)
        };
        for v in [0.05, 0.5, 0.95, 1.] {
            let at_one = f32::from(u8::from(v >= 1.));
            for g in [0., 0.4, 1.] {
                for m6 in [0., 1.] {
                    for y in [-1500., -560., -250., 0., 700., 1300.] {
                        let above = y - line_y(camera_y, v);
                        let ours = edge(above, g, m6, at_one);
                        let theirs = game(y, v, g, m6);
                        assert!(
                            (ours - theirs).abs() < 1e-3,
                            "{v} {g} {m6} {y}: {ours} {theirs}"
                        );
                    }
                }
            }
        }
        // At v = 1 the cut is at camera y - 800 + 100 g + 800 m6.
        for (g, m6) in [(0., 0.), (0.5, 0.), (0., 1.), (1., 1.)] {
            let cut = camera_y - 800. + 100. * g + 800. * m6;
            let above = |y: f32| y - line_y(camera_y, 1.);
            assert!(edge(above(cut - 1.), g, m6, 1.) <= 1.);
            assert!(edge(above(cut + 1.), g, m6, 1.) > 1.);
        }
        // The platform's edge (level y camera - 260.7 .. - 239.7, MEASURED) at v = 1: cut
        // where pod_mask is black, drawn and tinted where it is white.
        for y in [camera_y - 260.7, camera_y - 239.7] {
            let above = y - line_y(camera_y, 1.);
            assert!(edge(above, 0.5, 0., 1.) > 1.);
            let kept = edge(above, 0.5, 1., 1.);
            assert!(kept > 0.2 && kept < 0.45, "{kept}");
        }
        // Under 1 the island stands one band higher; without it nothing changes from D278.
        let above = 100.;
        assert!((edge(above, 0.2, 0., 0.) - ((above - 20.) / 400. + 1.)).abs() < 1e-6);
        assert!((edge(above, 0.2, 0., 0.) - edge(above, 0.2, 1., 0.) - 1.).abs() < 1e-6);
        assert_eq!(value_at_one(0.), Some(1.));
        assert_eq!(value_at_one(0.05), None);
        assert_eq!(value_at_one(1.), None);
        assert_eq!(value(0.), None);
        let d = uniforms(
            1.,
            [1.; 3],
            camera_y,
            Mat4::IDENTITY,
            None,
            Some(Handle::default()),
        );
        let d = d.unwrap();
        assert!(d.at_one && d.island.is_some());
        assert!((d.plane.dot(Vec4::new(0., camera_y - 400., 0., 1.))).abs() < 1e-3);
    }

    /// D284: the input that holds raw UV0 per draw bucket, by the route of its material:
    /// `pod_card` (primary UV0, unchanged) takes `in.uv`; `pod_card_edge` (primary UV1,
    /// second sample UV0 unchanged) takes `in.uv_b`; a solid bucket, or one whose UV0 set
    /// is transformed, takes the position.
    #[test]
    fn d284_the_uv_source_follows_the_route() {
        assert_eq!(uv0_source(Some((0, true)), Some((1, false))), 1);
        assert_eq!(uv0_source(Some((1, true)), Some((0, true))), 2);
        assert_eq!(uv0_source(Some((0, true)), Some((0, true))), 1);
        assert_eq!(uv0_source(None, None), 0);
        assert_eq!(uv0_source(Some((0, false)), None), 0);
        assert_eq!(uv0_source(Some((1, true)), Some((0, false))), 0);
        assert_eq!(uv0_source(Some((2, true)), None), 0);
        assert_eq!([CARD, CARD_EDGE, PLATFORM_MESH], [0x2f6a, 0x2fda, 145_753]);
        assert_eq!(POD_MASK, 129_991);
    }
}
