//! D334: the level's water, a translucent body up to the surface height the water level
//! gives, in the colour the water tint picks (`docs/level-globals.md`).
//!
//! The game's water colours are the first nine texels of
//! `gamedata/texture_library/reflection_maps/water_colours_32bit_linear.tex` (GUID 0x19bf3,
//! a 16x1 linear texture; 01.33 0x357340 and 0x3544ac, FOUND): each texel's RGB / 255,
//! with no sRGB decode.

use bevy::prelude::*;

/// GUID of `water_colours_32bit_linear.tex`.
pub const WATER_COLOURS: u32 = 0x19bf3;
/// The palette has this many colours; the texture's other texels are not used.
pub const PALETTE_LEN: usize = 9;
/// How much the water body hides what is behind it. PLACEHOLDER for the viewer: the game's
/// underwater pass (murkiness x 0.7 into shader parameter 0x4da2db17) is not modelled.
pub const WATER_ALPHA: f32 = 0.35;

/// The palette of the decoded texture: the first `min(width, 9)` texels of its top mip.
pub fn palette(texture: &lbp_texture::Texture) -> Vec<[f32; 3]> {
    let n = (texture.header.width as usize).min(PALETTE_LEN);
    texture
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .take(n)
        .map(|p| [p[0], p[1], p[2]].map(|c| c as f32 / 255.))
        .collect()
}

/// The colour a water tint picks (01.33 0xd5c50, FOUND): below 0 the first colour, above 1
/// the last, else colour 8 T blended towards the next one (the last wraps to the first).
pub fn tint(palette: &[[f32; 3]], t: f32) -> Option<[f32; 3]> {
    let n = palette.len();
    if n == 0 {
        return None;
    }
    let last = (PALETTE_LEN - 1) as f32;
    let x = if t < 0. {
        0.
    } else if t > 1. {
        last
    } else {
        last * t
    };
    let i = (x.trunc() as usize) % n;
    let (a, b) = (palette[i], palette[(i + 1) % n]);
    let f = x - x.floor();
    Some(std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f))
}

/// The water body entity, for play mode to move and recolour.
#[derive(Component)]
pub struct WaterBody {
    /// Bottom of the body and its horizontal centre, level units.
    pub floor: f32,
    pub centre: Vec2,
    pub size: Vec2,
    /// The display transform of level units (`plan.normalize`).
    pub normalize: Mat4,
    pub palette: Vec<[f32; 3]>,
    /// The (surface Y, tint) shown.
    pub shown: Option<(f32, f32)>,
}

impl WaterBody {
    /// The transform of a unit cube that fills the level's span up to `surface`.
    pub fn transform(&self, surface: f32) -> Transform {
        let height = (surface - self.floor).max(1.);
        let local = Mat4::from_translation(Vec3::new(
            self.centre.x,
            self.floor + height * 0.5,
            self.centre.y,
        )) * Mat4::from_scale(Vec3::new(self.size.x, height, self.size.y));
        Transform::from_matrix(self.normalize * local)
    }

    /// The material colour for a tint (grey when the palette was not read).
    pub fn colour(&self, t: f32) -> Color {
        let [r, g, b] = tint(&self.palette, t).unwrap_or([0.5; 3]);
        Color::LinearRgba(LinearRgba::new(r, g, b, WATER_ALPHA))
    }
}

/// The water body's material: unlit, blended, both faces.
pub fn material(colour: Color) -> crate::dual::ViewerMaterial {
    crate::dual::material(StandardMaterial {
        base_color: colour,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d334_tint_sweeps_the_palette_and_clamps() {
        let p: Vec<[f32; 3]> = (0..9).map(|i| [i as f32; 3]).collect();
        assert_eq!(tint(&p, -1.), Some([0.; 3]));
        assert_eq!(tint(&p, 0.), Some([0.; 3]));
        assert_eq!(tint(&p, 1.), Some([8.; 3]));
        assert_eq!(tint(&p, 2.), Some([8.; 3]));
        // 8 x 0.3125 = 2.5: halfway from colour 2 to colour 3.
        assert_eq!(tint(&p, 0.3125), Some([2.5; 3]));
        assert_eq!(tint(&[], 0.5), None);
    }
}
