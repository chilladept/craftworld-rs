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
use std::f32::consts::FRAC_1_SQRT_2;
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
    /// D323: the texels of the rectangle at (`x`, `y`) of `width` x `height` (an atlas
    /// cell), clamped to the image.
    pub fn crop(&self, x: usize, y: usize, width: usize, height: usize) -> Self {
        let width = width.min(self.width.saturating_sub(x));
        let height = height.min(self.height.saturating_sub(y));
        let texels = (y..y + height)
            .flat_map(|j| {
                let row = j * self.width;
                self.texels[row + x..row + x + width].iter().copied()
            })
            .collect();
        Self {
            width,
            height,
            texels: Arc::new(texels),
        }
    }

    /// D323 FOUND (frames 8980 / 9601, PS 3441, the drop shadow under each pin): the
    /// image's alpha read at nine taps `step` texels apart (a 3 x 3 grid, weights 0.1592 at
    /// the centre, 0.0965 on the sides, 0.0586 at the corners), as a coverage over the
    /// weights' sum.
    pub fn shadow(&self, step: f32) -> Self {
        let mut texels = Vec::with_capacity(self.width * self.height);
        let total = SHADOW_WEIGHTS[0] + 4. * SHADOW_WEIGHTS[1] + 4. * SHADOW_WEIGHTS[2];
        for j in 0..self.height {
            for i in 0..self.width {
                let (u, v) = (i as f32 + 0.5, j as f32 + 0.5);
                let mut a = 0.;
                for dj in -1i32..=1 {
                    for di in -1i32..=1 {
                        let w = SHADOW_WEIGHTS[(di.abs() + dj.abs()) as usize];
                        a += w * self.sample(u + di as f32 * step, v + dj as f32 * step)[3];
                    }
                }
                let a = a / total;
                texels.push([a, a, a, a]);
            }
        }
        Self {
            width: self.width,
            height: self.height,
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

/// The pin shadow's tap weights: centre, side, corner (PS 3441's constants).
pub const SHADOW_WEIGHTS: [f32; 3] = [0.1592, 0.0965, 0.0586];

/// D324: the seven ring taps of the transition's draw (PS 3483's constants 2 to 8, unit
/// directions; the eighth tap is the pixel itself). y is down the picture here as in the
/// texture's rows.
pub const TRANSITION_TAPS: [[f32; 2]; 7] = [
    [0.993712, 0.111964],
    [0.532032, 0.846724],
    [-0.330279, 0.943883],
    [-0.943883, 0.330279],
    [-0.846724, -0.532032],
    [-0.111964, -0.993712],
    [FRAC_1_SQRT_2, -FRAC_1_SQRT_2],
];

/// D323: a planet's corona (part 74): a square of `half` x 2 corona units a side facing the
/// eye, `pull` corona units behind the planet's place, drawn with the texture `image`
/// times `colour` and added to the picture (SrcAlpha, One: the shader's premultiplied
/// colour times its alpha again), the alpha untouched. Where a planet's sphere is nearer
/// than the square along a pixel's ray it is hidden (the game's depth test). Positions are
/// relative to the eye, in any one unit.
#[derive(Clone, Debug, PartialEq)]
pub struct CoronaPrim {
    /// A pixel (x, y, 1) of the layer to the direction of its ray from the eye.
    pub pixel_to_ray: bevy::math::Mat3,
    /// The square's centre, and its two axes in the plane (each one corona unit long).
    pub centre: bevy::math::Vec3,
    pub axes: [bevy::math::Vec3; 2],
    /// Half the square's side, in corona units.
    pub half: f32,
    /// The pixels to look at: x0, y0, x1, y1 (end exclusive).
    pub bounds: [usize; 4],
    /// Spheres that hide it: centre and radius.
    pub spheres: Vec<bevy::math::Vec4>,
    pub colour: [f32; 4],
    pub image: MenuImage,
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

    /// D323: adds a planet's corona (PS 3429 with SrcAlpha, One): per pixel the texel `t`
    /// times the colour `c`, premultiplied by its alpha `c.a t.a`, then times that alpha
    /// again by the blend, added to the colour; the alpha and its sum are not changed.
    /// Draw it before the menu's shapes, which then cover it as they cover the scene.
    pub fn draw_corona(&mut self, p: &CoronaPrim) {
        let n = p.axes[0].cross(p.axes[1]);
        let (l0, l1) = (p.axes[0].length_squared(), p.axes[1].length_squared());
        if !(n.length_squared() > 0. && l0 > 0. && l1 > 0. && p.half > 0.) {
            return;
        }
        let d_plane = n.dot(p.centre);
        let [r, g, b, alpha] = p.colour;
        let [x0, y0, x1, y1] = p.bounds;
        let (img_w, img_h) = (p.image.width as f32, p.image.height as f32);
        for y in y0..y1.min(self.height) {
            for x in x0..x1.min(self.width) {
                let ray =
                    p.pixel_to_ray * bevy::math::Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 1.);
                let denom = n.dot(ray);
                if denom.abs() < 1e-12 {
                    continue;
                }
                let t = d_plane / denom;
                if t <= 0. {
                    continue;
                }
                let rel = ray * t - p.centre;
                let (u, v) = (rel.dot(p.axes[0]) / l0, rel.dot(p.axes[1]) / l1);
                if u.abs() >= p.half || v.abs() >= p.half {
                    continue;
                }
                let len = ray.length();
                let (dir, at) = (ray / len, t * len);
                let hidden = p.spheres.iter().any(|s| {
                    let c = s.truncate();
                    let b = dir.dot(c);
                    let disc = b * b - (c.length_squared() - s.w * s.w);
                    disc >= 0. && {
                        let near = b - disc.sqrt();
                        near > 0. && near < at
                    }
                });
                if hidden {
                    continue;
                }
                let tex = p.image.sample(
                    (u / p.half + 1.) / 2. * img_w,
                    (1. - v / p.half) / 2. * img_h,
                );
                // t.rgb (straight) x c.rgb x (c.a t.a)^2, from premultiplied texels.
                let k = alpha * alpha * tex[3];
                let dst = &mut self.over[y * self.width + x];
                dst[0] += r * tex[0] * k;
                dst[1] += g * tex[1] * k;
                dst[2] += b * tex[2] * k;
            }
        }
    }

    /// D324 (PS 3483): lays `menu` (the menu drawn in a target of its own) over the layer as
    /// one draw while the frame comes in: per pixel the menu's colour and its alpha sum
    /// (saturated, the target's 8 bits) read at the pixel and at the seven
    /// [`TRANSITION_TAPS`] `radius` pixels away, the eight averaged, times `alpha`, then
    /// composed over (One, OneMinusSrcAlpha) with the alpha added.
    pub fn compose_transition(&mut self, menu: &MenuCanvas, alpha: f32, radius: f32) {
        let (w, h) = (self.width.min(menu.width), self.height.min(menu.height));
        // Only where the ring reaches something the menu drew.
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        for y in 0..h {
            for x in 0..w {
                if menu.sum[y * menu.width + x] > 0. {
                    (x0, y0) = (x0.min(x), y0.min(y));
                    (x1, y1) = (x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        if x0 >= x1 {
            return;
        }
        let pad = radius.max(0.).ceil() as usize + 1;
        let (x0, y0) = (x0.saturating_sub(pad), y0.saturating_sub(pad));
        let (x1, y1) = ((x1 + pad).min(w), (y1 + pad).min(h));
        let texel = |x: isize, y: isize| -> [f32; 4] {
            if x < 0 || y < 0 || x >= menu.width as isize || y >= menu.height as isize {
                return [0.; 4];
            }
            let i = y as usize * menu.width + x as usize;
            let o = menu.over[i];
            [o[0], o[1], o[2], menu.sum[i].clamp(0., 1.)]
        };
        // Bilinear between texel centres (the game's sampler), outside the target clear.
        let sample = |u: f32, v: f32| -> [f32; 4] {
            let (fx, fy) = (u - 0.5, v - 0.5);
            let (i, j) = (fx.floor(), fy.floor());
            let (tx, ty) = (fx - i, fy - j);
            let (i, j) = (i as isize, j as isize);
            let (a, b, c, d) = (
                texel(i, j),
                texel(i + 1, j),
                texel(i, j + 1),
                texel(i + 1, j + 1),
            );
            let mut out = [0.; 4];
            for k in 0..4 {
                let top = a[k] * (1. - tx) + b[k] * tx;
                let bottom = c[k] * (1. - tx) + d[k] * tx;
                out[k] = top * (1. - ty) + bottom * ty;
            }
            out
        };
        for y in y0..y1 {
            for x in x0..x1 {
                let (u, v) = (x as f32 + 0.5, y as f32 + 0.5);
                let mut acc = sample(u, v);
                for [dx, dy] in TRANSITION_TAPS {
                    let t = sample(u + dx * radius, v + dy * radius);
                    for k in 0..4 {
                        acc[k] += t[k];
                    }
                }
                let src = acc.map(|c| c / 8. * alpha);
                if src[3] <= 0. && src[0] <= 0. && src[1] <= 0. && src[2] <= 0. {
                    continue;
                }
                let i = y * self.width + x;
                let dst = &mut self.over[i];
                for ch in 0..3 {
                    dst[ch] = src[ch] + (1. - src[3]) * dst[ch];
                }
                dst[3] = src[3] + (1. - src[3]) * dst[3];
                self.sum[i] += src[3];
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

    /// A corona 20 pixels across, centred in a 32 x 32 layer, the eye at the origin looking
    /// down -z (a pixel's ray: (x - 16, 16 - y, -16)), the square 10 units away.
    fn corona(spheres: Vec<bevy::math::Vec4>) -> CoronaPrim {
        use bevy::math::{Mat3, Vec3};
        CoronaPrim {
            pixel_to_ray: Mat3::from_cols(
                Vec3::new(1., 0., 0.),
                Vec3::new(0., -1., 0.),
                Vec3::new(-16., 16., -16.),
            ),
            centre: Vec3::new(0., 0., -10.),
            axes: [Vec3::new(1., 0., 0.), Vec3::new(0., 1., 0.)],
            half: 6.25,
            bounds: [0, 0, 32, 32],
            spheres,
            colour: [0.5, 0.7, 1., 0.5],
            image: MenuImage::from_rgba(2, 2, &[255; 16]),
        }
    }

    /// D323: the corona adds colour x texel x (alpha x texel alpha)^2 (premultiplied, then
    /// SrcAlpha, One) to the layer's colour inside its square, leaves the alphas alone, and
    /// is hidden where a sphere is nearer than the square along the ray.
    #[test]
    fn d323_corona_adds_and_hides_behind_spheres() {
        let mut canvas = MenuCanvas::new(32, 32);
        canvas.draw_corona(&corona(Vec::new()));
        let at = |c: &MenuCanvas, x: usize, y: usize| c.over[y * 32 + x];
        let centre = at(&canvas, 16, 16);
        for (got, want) in centre.iter().zip([0.125, 0.175, 0.25, 0.]) {
            assert!((got - want).abs() < 1e-6, "{centre:?}");
        }
        // The square is 12.5 units across at depth 10: 20 pixels at 16 pixels a unit
        // of depth; outside it nothing is added.
        assert_eq!(at(&canvas, 2, 16), [0.; 4]);
        assert!(at(&canvas, 7, 16)[2] > 0.);
        assert!(canvas.sum.iter().all(|&a| a == 0.));
        // A sphere of radius 2 at depth 9 hides the middle; one behind the square does not.
        let mut hidden = MenuCanvas::new(32, 32);
        hidden.draw_corona(&corona(vec![bevy::math::Vec4::new(0., 0., -9., 2.)]));
        assert_eq!(at(&hidden, 16, 16), [0.; 4]);
        assert!(at(&hidden, 7, 16)[2] > 0.);
        let mut behind = MenuCanvas::new(32, 32);
        behind.draw_corona(&corona(vec![bevy::math::Vec4::new(0., 0., -14., 2.)]));
        assert_eq!(at(&behind, 16, 16), centre);
        // A menu shape drawn after covers it as it covers the scene.
        canvas.draw(&MenuPrim {
            centre: bevy::math::Vec2::new(16., 16.),
            size: bevy::math::Vec2::new(4., 4.),
            angle: 0.,
            shape: MenuShape::Rect,
            colour: [0., 0., 0., 0.5],
            image: None,
        });
        assert!((at(&canvas, 16, 16)[2] - 0.125).abs() < 1e-6);
    }

    /// D323: a pin's shadow is its alpha over the nine taps' weights (1 inside, 0 far
    /// outside); an atlas cell is cropped texel for texel.
    /// D324: the transition's draw: with no ring and full alpha the menu lands as drawn
    /// (alpha saturated); at half alpha it is half; a one-pixel dot spreads over the ring.
    #[test]
    fn d324_transition_composes_the_menu_as_one() {
        let pill = [150. / 255., 221. / 255., 60. / 255., 82. / 255.];
        let mut menu = MenuCanvas::new(16, 8);
        menu.draw(&prim(pill));
        let i = 4 * 16 + 8;
        let mut whole = MenuCanvas::new(16, 8);
        whole.compose_transition(&menu, 1., 0.);
        for k in 0..4 {
            assert!((whole.over[i][k] - menu.over[i][k]).abs() < 1e-6);
        }
        assert!((whole.sum[i] - 82. / 255.).abs() < 1e-6);
        let mut half = MenuCanvas::new(16, 8);
        half.compose_transition(&menu, 0.5, 0.);
        assert!((half.over[i][0] - menu.over[i][0] / 2.).abs() < 1e-6);
        assert!((half.sum[i] - 41. / 255.).abs() < 1e-6);
        // A dot at (20, 20) seen from the ring's taps 4 pixels out.
        let mut dot = MenuCanvas::new(41, 41);
        let k = 20 * 41 + 20;
        dot.over[k] = [1., 1., 1., 1.];
        dot.sum[k] = 1.;
        let mut out = MenuCanvas::new(41, 41);
        out.compose_transition(&dot, 1., 4.);
        assert!((out.sum[k] - 1. / 8.).abs() < 1e-6);
        // The pixel whose first tap lands on the dot.
        let [dx, dy] = TRANSITION_TAPS[0];
        let (x, y) = (20.5 - dx * 4., 20.5 - dy * 4.);
        let near = out.sum[y as usize * 41 + x as usize];
        assert!(near > 0. && near < 1. / 8., "{near}");
        assert_eq!(out.sum[0], 0.);
    }

    #[test]
    fn d323_shadow_and_atlas_cell() {
        let mut rgba = vec![0u8; 16 * 16 * 4];
        for j in 4..12 {
            for i in 4..12 {
                rgba[(j * 16 + i) * 4 + 3] = 255;
            }
        }
        let img = MenuImage::from_rgba(16, 16, &rgba);
        let shadow = img.shadow(1.);
        assert!((shadow.texels[8 * 16 + 8][3] - 1.).abs() < 1e-6);
        assert_eq!(shadow.texels[0][3], 0.);
        let edge = shadow.texels[8 * 16 + 4][3];
        assert!(edge > 0.4 && edge < 1., "{edge}");
        let cell = img.crop(4, 4, 8, 8);
        assert_eq!((cell.width, cell.height), (8, 8));
        assert!(cell.texels.iter().all(|t| t[3] == 1.));
    }
}
