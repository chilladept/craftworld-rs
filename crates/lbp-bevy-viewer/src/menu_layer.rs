//! D322 (docs/level-scripts.md part 73): the scripted Pod's main menu drawn the way the game
//! draws it. FOUND (RenderDoc replays of frames 8980 and 9601, every draw after the copy back):
//! the menu's pills, text, card and pins are drawn into the scene target after the
//! anti-aliasing job's copy back and before the final pass, each fragment premultiplied
//! (colour x alpha) and blended One, OneMinusSrcAlpha on colour and One, One on alpha, in the
//! target's encoded 8-bit values; the final pass then mixes back by that alpha.
//!
//! [`MenuCanvas`] rasterizes the menu's shapes ([`MenuPrim`]) on the CPU into a layer with
//! both quantities the two blends need: the premultiplied colour composed over (with the
//! layer's transmittance) and the sum of the alphas. The final pass composes the layer
//! (`glow_post`, `MENU_LAYER`).
use bevy::math::Vec2;
use std::sync::Arc;

/// The outline of a primitive: a rectangle, or a capsule (a rectangle with fully rounded
/// ends, the pills; a circle when square).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuShape {
    Rect,
    Capsule,
}

/// A texture drawn over a primitive's rectangle: premultiplied RGBA texels in 0..1
/// (encoded values), row-major from the top.
#[derive(Clone, Debug)]
pub struct MenuImage {
    pub width: usize,
    pub height: usize,
    pub texels: Arc<Vec<[f32; 4]>>,
}

impl PartialEq for MenuImage {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.texels, &other.texels)
    }
}

impl MenuImage {
    /// From straight-alpha 8-bit RGBA (a decoded picture).
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Self {
        let texels = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                let a = f32::from(p[3]) / 255.;
                [
                    f32::from(p[0]) / 255. * a,
                    f32::from(p[1]) / 255. * a,
                    f32::from(p[2]) / 255. * a,
                    a,
                ]
            })
            .collect();
        Self {
            width: width as usize,
            height: height as usize,
            texels: Arc::new(texels),
        }
    }
    /// White with an 8-bit coverage (a text raster).
    pub fn from_coverage(width: u32, height: u32, alpha: &[u8]) -> Self {
        let texels = alpha
            .iter()
            .map(|&c| {
                let a = f32::from(c) / 255.;
                [a, a, a, a]
            })
            .collect();
        Self {
            width: width as usize,
            height: height as usize,
            texels: Arc::new(texels),
        }
    }
    /// Bilinear at texel coordinates (texel centres at +0.5), clamped to the edge.
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let (w, h) = (self.width as isize, self.height as isize);
        if w == 0 || h == 0 {
            return [0.; 4];
        }
        let (x, y) = (u - 0.5, v - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |i: isize, j: isize| {
            let i = i.clamp(0, w - 1) as usize;
            let j = j.clamp(0, h - 1) as usize;
            self.texels[j * self.width + i]
        };
        let (i, j) = (x0 as isize, y0 as isize);
        let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1));
        let mut out = [0.; 4];
        for k in 0..4 {
            let top = a[k] * (1. - fx) + b[k] * fx;
            let bottom = c[k] * (1. - fx) + d[k] * fx;
            out[k] = top * (1. - fy) + bottom * fy;
        }
        out
    }
}

/// One draw of the menu, in pixels of the layer (from the top left): a shape of `size`
/// centred at `centre`, turned by `angle` radians (clockwise on the picture, y down), filled
/// with `colour` (encoded RGB and an alpha), times `image` over its rectangle if any.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuPrim {
    pub centre: Vec2,
    pub size: Vec2,
    pub angle: f32,
    pub shape: MenuShape,
    pub colour: [f32; 4],
    pub image: Option<MenuImage>,
}

/// The menu layer: per pixel the premultiplied colour composed over (`rgb`) with 1 - the
/// transmittance (`a`), and the sum of the draws' alphas (the game's alpha target).
pub struct MenuCanvas {
    pub width: usize,
    pub height: usize,
    pub over: Vec<[f32; 4]>,
    pub sum: Vec<f32>,
}

impl MenuCanvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            over: vec![[0.; 4]; width * height],
            sum: vec![0.; width * height],
        }
    }

    pub fn clear(&mut self) {
        self.over.fill([0.; 4]);
        self.sum.fill(0.);
    }

    /// The share of the pixel centred at local `l` that the shape covers (about one pixel
    /// of soft edge).
    fn coverage(shape: MenuShape, l: Vec2, half: Vec2) -> f32 {
        match shape {
            MenuShape::Rect => {
                (half.x + 0.5 - l.x.abs()).clamp(0., 1.) * (half.y + 0.5 - l.y.abs()).clamp(0., 1.)
            }
            MenuShape::Capsule => {
                let r = half.x.min(half.y);
                let q = Vec2::new(
                    (l.x.abs() - (half.x - r)).max(0.),
                    (l.y.abs() - (half.y - r)).max(0.),
                );
                (0.5 - (q.length() - r)).clamp(0., 1.)
            }
        }
    }

    /// Draws `p` as the game's menu draws do: premultiplied colour over (One,
    /// OneMinusSrcAlpha), the alpha added (One, One).
    pub fn draw(&mut self, p: &MenuPrim) {
        let half = p.size / 2.;
        if !(half.x > 0. && half.y > 0.) || !p.centre.is_finite() || !p.angle.is_finite() {
            return;
        }
        let (s, c) = p.angle.sin_cos();
        let ext = Vec2::new(
            half.x * c.abs() + half.y * s.abs(),
            half.x * s.abs() + half.y * c.abs(),
        ) + Vec2::splat(1.);
        let x0 = (p.centre.x - ext.x).floor().max(0.) as usize;
        let y0 = (p.centre.y - ext.y).floor().max(0.) as usize;
        let x1 = ((p.centre.x + ext.x).ceil().max(0.) as usize).min(self.width);
        let y1 = ((p.centre.y + ext.y).ceil().max(0.) as usize).min(self.height);
        let [r, g, b, alpha] = p.colour;
        for y in y0..y1 {
            for x in x0..x1 {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - p.centre;
                let l = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
                let cov = Self::coverage(p.shape, l, half);
                if cov <= 0. {
                    continue;
                }
                let t = match &p.image {
                    Some(img) => img.sample(
                        (l.x + half.x) / p.size.x * img.width as f32,
                        (l.y + half.y) / p.size.y * img.height as f32,
                    ),
                    None => [1.; 4],
                };
                let a = alpha * t[3] * cov;
                if a <= 0. {
                    continue;
                }
                let k = alpha * cov;
                let src = [r * t[0] * k, g * t[1] * k, b * t[2] * k];
                let i = y * self.width + x;
                let dst = &mut self.over[i];
                for (ch, v) in src.iter().enumerate() {
                    dst[ch] = v + (1. - a) * dst[ch];
                }
                dst[3] = a + (1. - a) * dst[3];
                self.sum[i] += a;
            }
        }
    }

    /// The layer as 8-bit textures: RGBA (colour, 1 - transmittance) and the alpha sum
    /// (saturated, as the game's 8-bit target).
    pub fn bytes(&self) -> (Vec<u8>, Vec<u8>) {
        let q = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
        let colour = self.over.iter().flat_map(|p| p.map(q)).collect();
        let alpha = self.sum.iter().map(|&a| q(a)).collect();
        (colour, alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prim(colour: [f32; 4]) -> MenuPrim {
        MenuPrim {
            centre: Vec2::new(8., 4.),
            size: Vec2::new(12., 6.),
            angle: 0.,
            shape: MenuShape::Capsule,
            colour,
            image: None,
        }
    }

    /// D322: a pill at 82/255 then white text at 102/255 over it (the game's two draws over
    /// an unfocused pill): colour composed over in encoded values, alphas summed (the game
    /// frame's 82 and 184).
    #[test]
    fn d322_pill_then_text_composes_over_and_sums_alpha() {
        let mut canvas = MenuCanvas::new(16, 8);
        let pill = [150. / 255., 221. / 255., 60. / 255., 82. / 255.];
        canvas.draw(&prim(pill));
        let i = 4 * 16 + 8;
        let (colour, alpha) = canvas.bytes();
        assert_eq!(&colour[i * 4..i * 4 + 4], &[48, 71, 19, 82]);
        assert_eq!(alpha[i], 82);
        // Outside the capsule's rounded end nothing is drawn.
        assert_eq!(alpha[16 + 2], 0);
        canvas.draw(&MenuPrim {
            colour: [1., 1., 1., 102. / 255.],
            shape: MenuShape::Rect,
            ..prim(pill)
        });
        let (colour, alpha) = canvas.bytes();
        // 102 + (1 - 0.4) x 48.2 = 130.9 (red); 1 - 0.68 x 0.6 = 0.593 (151).
        assert_eq!(&colour[i * 4..i * 4 + 4], &[131, 145, 114, 151]);
        assert_eq!(alpha[i], 184);
        // Over a scene value s the composed pixel is colour + (1 - a) s.
        let over = canvas.over[i];
        let s = 0.5;
        let composed = over[0] + (1. - over[3]) * s;
        let by_steps = 0.4 + 0.6 * (pill[0] * pill[3] + (1. - pill[3]) * s);
        assert!((composed - by_steps).abs() < 1e-5);
    }

    /// The capsule is a circle when square and has a soft edge about a pixel wide.
    #[test]
    fn d322_capsule_edge_and_turn() {
        let half = Vec2::new(10., 5.);
        assert_eq!(
            MenuCanvas::coverage(MenuShape::Capsule, Vec2::ZERO, half),
            1.
        );
        assert_eq!(
            MenuCanvas::coverage(MenuShape::Capsule, Vec2::new(0., 5.), half),
            0.5
        );
        assert_eq!(
            MenuCanvas::coverage(MenuShape::Capsule, Vec2::new(9.9, 4.9), half),
            0.
        );
        // A rectangle turned by 90 degrees covers the transposed box.
        let mut canvas = MenuCanvas::new(20, 20);
        canvas.draw(&MenuPrim {
            centre: Vec2::new(10., 10.),
            size: Vec2::new(10., 2.),
            angle: std::f32::consts::FRAC_PI_2,
            shape: MenuShape::Rect,
            colour: [1., 1., 1., 1.],
            image: None,
        });
        assert_eq!(canvas.sum[6 * 20 + 10], 1.);
        assert_eq!(canvas.sum[10 * 20 + 6], 0.);
    }

    /// An image is sampled over the rectangle at its texel centres (a raster drawn at its
    /// own size lands texel for pixel), its alpha times the colour's.
    #[test]
    fn d322_image_lands_texel_for_pixel() {
        let alpha: Vec<u8> = (0..12).map(|i| (i * 20) as u8).collect();
        let img = MenuImage::from_coverage(4, 3, &alpha);
        let mut canvas = MenuCanvas::new(8, 8);
        canvas.draw(&MenuPrim {
            centre: Vec2::new(4., 3.5),
            size: Vec2::new(4., 3.),
            angle: 0.,
            shape: MenuShape::Rect,
            colour: [1., 1., 1., 0.5],
            image: Some(img),
        });
        for j in 0..3 {
            for i in 0..4 {
                let got = canvas.sum[(2 + j) * 8 + 2 + i];
                let want = f32::from(alpha[j * 4 + i]) / 255. * 0.5;
                assert!((got - want).abs() < 1e-6, "{i} {j} {got} {want}");
            }
        }
    }
}
