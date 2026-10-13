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
//! Left out: the second mask of the two card materials (islands where the line stands
//! one band higher), the glass material's constant in place of its colour's green, the
//! fog over the flat colour.
use crate::dual::Dissolve;
use bevy::prelude::*;

/// GUID of `transition_mask.tex`, the mask the pod's script names for every backdrop
/// but six (FOUND; the other six masks are not loaded).
pub const TRANSITION_MASK: u32 = 132_243;
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
pub fn uniforms(
    v: f32,
    colour: [f32; 3],
    camera_y: f32,
    level_to_world: Mat4,
    mask: Option<Handle<Image>>,
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
        let d = uniforms(0.5, [1., 0.5, 0.], -275., to_world, None).unwrap();
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
        assert!(uniforms(0.5, [0.; 3], 0., Mat4::ZERO, None).is_none());
        assert!(world_plane(Vec4::Y, Mat4::IDENTITY) == Some(Vec4::Y));
    }
}
