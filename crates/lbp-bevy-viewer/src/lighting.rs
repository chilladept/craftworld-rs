//! D121 opt-in preview lighting (docs/d121-preview-lighting.md). A directional key light
//! plus ambient (Lambert on the generated diagnostic normals), multiplied onto every
//! material mode's finished colour in `dual.rs`. Not game lighting: LBP's renderer,
//! PWorld lights, rim light, exposure and backdrop lighting are not reproduced.
use bevy::prelude::*;
use lbp_preview::level_scene::LevelSettings;

pub const LABEL: &str = "preview lighting (not game lighting)";
pub const LEVEL_LABEL: &str =
    "preview lighting from level settings (approximate, not game shading)";

/// Linear fog in display units (Bevy `FogFalloff::Linear`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub color: Vec3,
    pub start: f32,
    pub end: f32,
}

impl Fog {
    /// D274: the fog with a script's fog bias (level units) added to its start and its
    /// end; `display_scale` is display units per level unit. The game's fog starts at
    /// the level's fog near plus the bias and keeps its span ([`GameFog`]); MEASURED in
    /// the Pod: 5810 + 20000 = 25810, the start of the closed Pod's fog. A bias that is
    /// not a number changes nothing.
    pub fn biased(self, bias: f32, display_scale: f32) -> Self {
        let by = bias * display_scale;
        if !by.is_finite() {
            return self;
        }
        Self {
            start: self.start + by,
            end: self.end + by,
            ..self
        }
    }
}

/// P2x: the game's output-space fog from a level's settings (level units). The game's
/// vertex shader makes `f = (depth - (FogNear + bias)) / (FogFar - FogNear)` (PC Ghidra
/// read, oracles/results/2026-10-09-shading-ghidra.md); the colour is FogColor divided by
/// its own w when that is non-zero. The bias is script-set (the Pod's SetDOFAndFog), 0 here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameFog {
    pub color: Vec3,
    pub near: f32,
    pub span: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewLight {
    /// Unit direction toward the light (display space = world space up to scale).
    pub toward_light: Vec3,
    pub key: Vec3,
    pub ambient: Vec3,
    pub fog: Option<Fog>,
    /// P2x: the level's own game fog (None without usable settings).
    pub game_fog: Option<GameFog>,
    /// P2x: the Pod scenes use the live game's values (GAME_FOG_*, Puter-driven start)
    /// because pod_planets.bin's stored settings differ from the live ones.
    pub pod_fog: bool,
    /// D336: the light keeps its own settings' fog even in the Pod (the Story's backdrop
    /// lighting: its fog is not the Pod's live constants).
    pub own_fog: bool,
    pub from_level: bool,
    /// P2l: level values before the preview fit (key = sun colour x multiplier).
    pub raw_key: Vec3,
    pub raw_ambient: Vec3,
    /// P2l: rim colours (rimColor below, rimColor2 above), zero without level settings.
    pub rim: [Vec3; 2],
    /// P2p: level exposure (1 without level settings or when unusable).
    pub exposure: f32,
    /// P2l: LBP-shaped mode (`lbp`); off = D121 Lambert preview.
    pub lbp: bool,
    /// P3g: the sun term is multiplied by the dynamic sun-shadow map (LBP mode, level
    /// meshes only: the Pod draws with fragment variant 8, which has no shadow).
    pub shadows: bool,
    /// P2q: the scalar (.w) the game sends with the sun, ambient and rim colours; it
    /// weights the gloss term (brdf.cg `light.w`). Zero outside LBP mode.
    pub scales: Vec4,
    /// P3f: the stored .w of the sun, ambient, rim and rim2 colours (1 when unusable). The
    /// game's .w is stored w x the same factor as the rgb (Moon oracle, #110); it only
    /// weights the gloss term, the rgb is not multiplied by it.
    pub stored_w: [f32; 4],
    /// P3k: baked AO tweak (scale, bias) from the level settings; the backdrop lightmap
    /// gives `ao = map * scale + bias` (oracles 2026-10-10-baked-ao). (1, 0) when unset.
    pub baked_ao: [f32; 2],
    /// P2q: level sun multiplier (already folded into `raw_key`); 1 without settings.
    pub multiplier: f32,
    /// P2t: display units per level unit (scene normalization), for the game fog.
    pub display_scale: f32,
    /// P2t: Puter opening progress (0 closed, 1 open) the game fog start follows.
    pub pod_t: f32,
    /// D314: the level's sun (`sun_position` x `sun_position_scale`) in the light frame's
    /// display units, `(sun - centre) x scale`; None without level settings.
    pub sun_display: Option<Vec3>,
    /// D314: the sun as a point in display world space, set where the game lights from
    /// it unturned (`main::apply_lighting`); None keeps the direction `toward_light`.
    pub sun_point: Option<Vec3>,
    /// Values used and every preview choice, for the panel and QA log.
    pub notes: Vec<String>,
}

impl Default for PreviewLight {
    fn default() -> Self {
        Self::generic()
    }
}

/// Fully lit factor never exceeds 1 per channel (preview choice: no tonemapping here,
/// so brighter factors would clip textures to white).
fn fit(key: Vec3, ambient: Vec3) -> (Vec3, Vec3, f32) {
    let peak = (key + ambient).max_element();
    let scale = if peak > 1. { 1. / peak } else { 1. };
    (key * scale, ambient * scale, scale)
}

fn finite_rgb(v: [f32; 4]) -> Option<Vec3> {
    let c = Vec3::new(v[0], v[1], v[2]);
    (c.is_finite() && c.min_element() >= 0.).then_some(c)
}

impl PreviewLight {
    /// Generic key light from upper left, in front (LBP side-on view looks along -Z).
    pub fn generic() -> Self {
        Self {
            toward_light: Vec3::new(-0.45, 0.75, 0.5).normalize(),
            key: Vec3::splat(0.65),
            ambient: Vec3::splat(0.35),
            fog: None,
            game_fog: None,
            pod_fog: false,
            own_fog: false,
            from_level: false,
            raw_key: Vec3::splat(0.65),
            raw_ambient: Vec3::splat(0.35),
            rim: [Vec3::ZERO; 2],
            exposure: 1.,
            lbp: false,
            shadows: false,
            scales: Vec4::ZERO,
            stored_w: [1.; 4],
            baked_ao: [1., 0.],
            multiplier: 1.,
            display_scale: 1.,
            pod_t: 0.,
            sun_display: None,
            sun_point: None,
            notes: vec!["generic key (upper left, front) 0.65 + ambient 0.35".into()],
        }
    }

    /// Level settings mapping. `centre` is the displayed bounds centre in level units and
    /// `scale` the display units per level unit (scene normalization). Any unusable value
    /// falls back to the generic light, with the reason noted.
    pub fn from_settings(s: &LevelSettings, centre: Vec3, scale: f32) -> Self {
        match Self::try_settings(s, centre, scale) {
            Ok(light) => light,
            Err(e) => {
                let mut light = Self::generic();
                light
                    .notes
                    .push(format!("level settings not used ({e}); generic light"));
                light
            }
        }
    }

    fn try_settings(s: &LevelSettings, centre: Vec3, scale: f32) -> Result<Self, String> {
        let sun = Vec3::from(s.sun_position) * s.sun_position_scale;
        let toward = (sun - centre)
            .try_normalize()
            .filter(|_| sun.is_finite() && centre.is_finite())
            .ok_or("sun position unusable")?;
        let multiplier = Some(s.sun_multiplier)
            .filter(|m| m.is_finite() && *m >= 0.)
            .ok_or("sun multiplier unusable")?;
        let key = finite_rgb(s.sun_color).ok_or("sun colour unusable")? * multiplier;
        let ambient = finite_rgb(s.ambient_color).ok_or("ambient colour unusable")?;
        let rim = [
            finite_rgb(s.rim_color).unwrap_or(Vec3::ZERO),
            finite_rgb(s.rim_color2).unwrap_or(Vec3::ZERO),
        ];
        let stored_w = [
            s.sun_color[3],
            s.ambient_color[3],
            s.rim_color[3],
            s.rim_color2[3],
        ]
        .map(|w| if w.is_finite() && w > 0. { w } else { 1. });
        let (raw_key, raw_ambient) = (key, ambient);
        let (key, ambient, fitted) = fit(key, ambient);
        let fog = finite_rgb(s.fog_color)
            .filter(|_| {
                s.fog_near.is_finite()
                    && s.fog_far.is_finite()
                    && s.fog_far > s.fog_near
                    && scale.is_finite()
                    && scale > 0.
            })
            .map(|color| Fog {
                color,
                start: s.fog_near * scale,
                end: s.fog_far * scale,
            });
        let mut notes = vec![
            format!(
                "sun position ({:.4}, {:.4}, {:.4}) x {:.1}; toward light ({:.3}, {:.3}, {:.3})",
                s.sun_position[0],
                s.sun_position[1],
                s.sun_position[2],
                s.sun_position_scale,
                toward.x,
                toward.y,
                toward.z
            ),
            format!(
                "sun colour ({:.3}, {:.3}, {:.3}) x multiplier {:.3}; ambient ({:.3}, {:.3}, {:.3})",
                s.sun_color[0],
                s.sun_color[1],
                s.sun_color[2],
                multiplier,
                s.ambient_color[0],
                s.ambient_color[1],
                s.ambient_color[2]
            ),
            format!("preview fit: key and ambient x {fitted:.3} (lit peak 1; exposure {:.3}, rim colours not applied)", s.exposure),
        ];
        notes.push(match fog {
            Some(f) => format!(
                "fog ({:.3}, {:.3}, {:.3}) linear {:.1}..{:.1} level units (Bevy distance fog)",
                f.color.x, f.color.y, f.color.z, s.fog_near, s.fog_far
            ),
            None => "fog not used (unusable colour or range)".into(),
        });
        let game_fog = (s.fog_near.is_finite() && s.fog_far.is_finite() && s.fog_far > s.fog_near)
            .then(|| {
                // The game divides the stored colour by its own w when w is non-zero.
                let w = s.fog_color[3];
                let divisor = if w.is_finite() && w > 0. { w } else { 1. };
                finite_rgb(s.fog_color).map(|c| GameFog {
                    color: c / divisor,
                    near: s.fog_near,
                    span: s.fog_far - s.fog_near,
                })
            })
            .flatten();
        Ok(Self {
            toward_light: toward,
            key,
            ambient,
            fog,
            game_fog,
            pod_fog: false,
            own_fog: false,
            from_level: true,
            raw_key,
            raw_ambient,
            rim,
            stored_w,
            baked_ao: if s.baked_ao_scale.is_finite()
                && s.baked_ao_scale > 0.
                && s.baked_ao_bias.is_finite()
            {
                [s.baked_ao_scale, s.baked_ao_bias]
            } else {
                [1., 0.]
            },
            exposure: Some(s.exposure)
                .filter(|e| e.is_finite() && *e > 0.)
                .unwrap_or(1.),
            lbp: false,
            shadows: false,
            scales: Vec4::ZERO,
            multiplier,
            display_scale: if scale.is_finite() && scale > 0. {
                scale
            } else {
                1.
            },
            pod_t: 0.,
            sun_display: (scale.is_finite() && scale > 0.).then(|| (sun - centre) * scale),
            sun_point: None,
            notes,
        })
    }

    pub fn label(&self) -> &'static str {
        if self.from_level {
            LEVEL_LABEL
        } else {
            LABEL
        }
    }

    /// P2l `--lbp-lighting`: the same values in an LBP-shaped model (community brdf.cg
    /// reconstruction, docs/p2l-lbp-lighting.md): hemisphere ambient, sun, two rim
    /// colours, unfitted. `turn` rotates the light from level into world space (the Pod
    /// planets root). The uniform mapping (exposure, tint, rim constants) is INFERRED.
    pub fn lbp_shaped(&self, turn: Quat) -> Self {
        let mut light = self.clone();
        let e = self.exposure;
        light.lbp = true;
        light.shadows = true;
        // P2p: the scales the game sends with each colour (D-RSX capture of the Pod).
        light.key = self.raw_key * GAME_SUN_SCALE * e;
        light.ambient = self.raw_ambient * GAME_AMBIENT_SCALE * e;
        light.rim = self.rim.map(|r| r * GAME_RIM_SCALE * e);
        let w = self.stored_w;
        light.scales = Vec4::new(
            GAME_SUN_SCALE * self.multiplier * e * w[0],
            GAME_AMBIENT_SCALE * e * w[1],
            GAME_RIM_SCALE * e * w[2],
            GAME_RIM_SCALE * e * w[3],
        );
        light.toward_light = (turn * self.toward_light).normalize_or_zero();
        light.notes.push(format!(
            "LBP-shaped: ambient x (N.y 0.15 + 0.5), sun, rim ({:.3}, {:.3}, {:.3}) below to ({:.3}, {:.3}, {:.3}) above x (N.V + 0.9)^2; game scales sun {GAME_SUN_SCALE} x multiplier, ambient {GAME_AMBIENT_SCALE}, rim {GAME_RIM_SCALE}, all x exposure {e:.3} (RSX capture fit)",
            self.rim[0].x, self.rim[0].y, self.rim[0].z, self.rim[1].x, self.rim[1].y, self.rim[1].z
        ));
        light
    }

    /// P2t: fog uniform for the shader: start and slope in display units, CC, on (w).
    /// Start is interpolated between the closed and open values by `pod_t` (INFERRED).
    pub fn lbp_fog(&self) -> Vec4 {
        if !self.lbp {
            return Vec4::ZERO;
        }
        // P2x: a level's own FogNear/FogFar; the Pod keeps the live-game constants.
        if let (false, Some(f)) = (self.pod_fog, self.game_fog) {
            return Vec4::new(
                f.near * self.display_scale,
                1. / (f.span * self.display_scale),
                GAME_FOG_CC,
                1.,
            );
        }
        let t = self.pod_t.clamp(0., 1.);
        let start = GAME_FOG_START[0] + (GAME_FOG_START[1] - GAME_FOG_START[0]) * t;
        Vec4::new(
            start * self.display_scale,
            GAME_FOG_SLOPE / self.display_scale,
            GAME_FOG_CC,
            1.,
        )
    }

    /// P2x: fog colour in output space (the level's FogColor, or the Pod's live value).
    pub fn lbp_fog_color(&self) -> Vec3 {
        match (self.pod_fog, self.game_fog) {
            (false, Some(f)) => f.color,
            _ => Vec3::from(GAME_FOG_COLOR),
        }
    }

    /// CPU mirror of the shader's `preview_light` for an already oriented unit normal.
    #[allow(dead_code)] // shader mirror, exercised by tests
    pub fn factor(&self, normal: Vec3) -> Vec3 {
        self.ambient + self.key * normal.dot(self.toward_light).max(0.)
    }

    /// CPU mirror of the shader's LBP-shaped light for an oriented unit normal and the
    /// unit direction from the surface to the viewer.
    #[allow(dead_code)] // shader mirror, exercised by tests
    pub fn lbp_factor(&self, normal: Vec3, to_view: Vec3) -> Vec3 {
        self.lbp_factor_baked(normal, to_view, 1., 1.)
    }

    /// P3k mirror (fpdec v0/v3 arithmetic): ambient x ao, sun x baked x ao, rim x baked, where
    /// `ao` = bakedAOScale + bakedAOBias and `baked` is the lightmap texel (shadowmap.x).
    #[allow(dead_code)] // shader mirror, exercised by tests
    pub fn lbp_factor_baked(&self, normal: Vec3, to_view: Vec3, ao: f32, baked: f32) -> Vec3 {
        let up = (normal.y + 1.) / 2.;
        // V from the eye: 0 facing the viewer, 0.81 at grazing. P2m tried V toward the
        // eye; D203 measured the room 2.8x too bright with it, so it was reverted (P2n).
        let rim = (normal.dot(-to_view) + 0.9).clamp(0., 1.).powi(2);
        self.ambient * (normal.y * 0.15 + 0.5) * ao
            + self.key * (normal.dot(self.toward_light).max(0.) * baked * ao)
            + self.rim[0].lerp(self.rim[1], up) * rim * baked
    }
}

/// CPU mirror of the shader's double-sided orientation: the smooth normal is turned into
/// the hemisphere of the face normal that points at the viewer.
#[allow(dead_code)] // shader mirror, exercised by tests
pub fn facing_normal(smooth: Vec3, face: Vec3, to_view: Vec3) -> Vec3 {
    let n = smooth.normalize_or_zero();
    let Some(mut f) = face.try_normalize() else {
        return n;
    };
    if f.dot(to_view) < 0. {
        f = -f;
    }
    if n.dot(f) < 0. {
        -n
    } else {
        n
    }
}

/// P2p: per-colour scales in the game's Pod shader constants (.w of suncol, ambcol and
/// both rim colours), fitted to the PC's RenderDoc capture of RPCS3 with pod_planets
/// preset 0 (multiplier 1.0206, exposure 1.2165): sun 0.372 = 0.3 x 1.0206 x 1.2165,
/// ambient 0.547 = 0.45 x 1.2165, rim 0.730 = 0.6 x 1.2165 (each within 0.001).
/// CONFIRMED_FROM_LOCAL_DATA for that frame; the split into constant x exposure (x
/// multiplier) is INFERRED from one level, as is the shader applying .w to .rgb.
pub const GAME_SUN_SCALE: f32 = 0.3;
/// P2t: the game's output-space fog in the Pod (PC RenderDoc frames 5247 and 14790):
/// v = mix(v, (luma(v) x CC + 1) x COLOR, saturate((depth - start) x SLOPE)), depth in level
/// units, start 25810 closed and 5810 open. CONFIRMED_FROM_LOCAL_DATA for those frames;
/// the colour's source and why start moves with the Puter are unknown.
pub const GAME_FOG_COLOR: [f32; 3] = [0.645597, 0.409244, 0.2281];
pub const GAME_FOG_CC: f32 = 0.240216;
pub const GAME_FOG_SLOPE: f32 = 1. / 52648.;
pub const GAME_FOG_START: [f32; 2] = [25810., 5810.];
pub const GAME_AMBIENT_SCALE: f32 = 0.45;
pub const GAME_RIM_SCALE: f32 = 0.6;

/// Viewer-side toggle state.
#[derive(Default)]
pub struct LightingState {
    pub enabled: bool,
    /// P2l `--lbp-lighting`.
    pub lbp: bool,
    /// P2l: the Pod planets root rotation the LBP light was last turned by.
    pub turn: Quat,
    /// P2l: scene entities from this index on (the Pod shell layer and figure) are lit by
    /// their own level's settings, turned by the shell root.
    pub shell: Option<(usize, PreviewLight)>,
    /// P2l: the shell root rotation last applied.
    pub shell_turn: Quat,
    /// P2t: Puter progress the fog was last applied for.
    pub pod_t: f32,
    pub light: PreviewLight,
    /// Set when the toggle or scene changes; `apply` runs once per change.
    pub dirty: bool,
    /// Lighting was applied last time (so turning it off restores materials).
    pub applied: bool,
    pub lines: Vec<String>,
    /// D267: what the level's light was built from, when it follows the PWorld lighting
    /// (P3m), so a played level can be lit again when a script changes that.
    pub level: Option<LevelRelight>,
    /// D274: the fog bias the played level's scripts set (the third value of
    /// `Cam.SetDOFAndFog`, level units), added to the preview fog ([`Fog::biased`]).
    /// 0 without `--level-scripts` and until a script sets one.
    pub fog_bias: f32,
}

/// D267: a level's light source for the PWorld lighting rule (P3m,
/// `LevelSettings::at_lighting`): the main block and the presets as stored, and the frame
/// `from_settings` was given (bounds centre in level units, display units per level unit).
pub struct LevelRelight {
    pub main: LevelSettings,
    pub presets: Vec<LevelSettings>,
    pub centre: Vec3,
    pub scale: f32,
    /// PWorld (lighting, darkness, fog tint, fog) the shown light was built from at load.
    pub stored: [f32; 4],
    /// D332: the fog tint's palette for this level (empty when not read: no tint).
    pub fog_palette: Vec<[f32; 4]>,
    /// D332: the level is the Pod's (its palette, tint up to 1).
    pub pod: bool,
}

impl LevelRelight {
    /// The level's light at that PWorld lighting, darkness, fog tint and fog.
    pub fn light(&self, lighting: f32, darkness: f32, fog_tint: f32, fog: f32) -> PreviewLight {
        let (settings, _) =
            LevelSettings::at_lighting(&self.main, &self.presets, lighting, darkness);
        let settings = settings
            .with_fog_tint(fog_tint, &self.fog_palette, self.pod)
            .with_fog(fog);
        PreviewLight::from_settings(&settings, self.centre, self.scale)
    }
}

/// Summary lines (panel and QA log). `unlit` of `total` draw entities kept unlit.
pub fn summary(light: &PreviewLight, unlit: usize, total: usize) -> Vec<String> {
    let mut lines = vec![format!("Lighting: {}", light.label())];
    lines.extend(light.notes.iter().map(|n| format!("  {n}")));
    lines.push(format!(
        "  {unlit} of {total} draw entities with untrusted generated normals ({})",
        if light.lbp {
            "LBP mode: lit flat by their face normals"
        } else {
            "unlit"
        }
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> LevelSettings {
        LevelSettings {
            sun_position: [0., 1., 0.],
            sun_position_scale: 1000.,
            sun_color: [1., 0.5, 0.25, 1.],
            ambient_color: [0.5, 0.5, 0.5, 1.],
            sun_multiplier: 2.,
            exposure: 1.,
            fog_color: [0.1, 0.2, 0.3, 1.],
            fog_near: 100.,
            fog_far: 300.,
            rim_color: [0.; 4],
            rim_color2: [0.; 4],
            baked_ao_bias: 0.,
            baked_ao_scale: 1.,
            presets: 0,
            backdrop_mesh: None,
        }
    }

    #[test]
    fn baked_ao_tweak_follows_settings_and_the_mirror_splits_ao_from_the_texel() {
        let mut st = settings();
        st.baked_ao_scale = 0.5;
        st.baked_ao_bias = 0.25;
        let light = PreviewLight::from_settings(&st, Vec3::ZERO, 1.);
        assert_eq!(light.baked_ao, [0.5, 0.25]);
        // Unset (default 0/0) and unusable values fall back to the identity tweak.
        st.baked_ao_scale = 0.;
        assert_eq!(
            PreviewLight::from_settings(&st, Vec3::ZERO, 1.).baked_ao,
            [1., 0.]
        );
        let mut lit =
            PreviewLight::from_settings(&settings(), Vec3::ZERO, 1.).lbp_shaped(Quat::IDENTITY);
        lit.rim = [Vec3::splat(0.5); 2];
        let (n, v) = (Vec3::Y, Vec3::Z);
        let full = lit.lbp_factor(n, v);
        assert_eq!(full, lit.lbp_factor_baked(n, v, 1., 1.));
        // ao scales ambient and sun only; the texel scales sun and rim only.
        let no_ao = lit.lbp_factor_baked(n, v, 0., 1.);
        assert!(no_ao.x > 0. && no_ao.x < full.x, "rim survives");
        let dark = lit.lbp_factor_baked(n, v, 1., 0.);
        assert!(dark.x > 0. && dark.x < full.x, "ambient survives");
    }

    #[test]
    fn lambert_factor_is_ambient_plus_clamped_key() {
        let l = PreviewLight::generic();
        assert!((l.factor(l.toward_light) - Vec3::ONE).abs().max_element() < 1e-6);
        assert_eq!(l.factor(-l.toward_light), Vec3::splat(0.35));
        assert!(l.toward_light.is_normalized());
    }

    #[test]
    fn back_faces_and_mirrored_winding_light_like_front_faces() {
        let to_view = Vec3::Z;
        // Front face, smooth normal toward the viewer: kept.
        assert_eq!(facing_normal(Vec3::Z, Vec3::Z, to_view), Vec3::Z);
        // Back face seen from behind (face normal away from the viewer): flipped.
        assert_eq!(facing_normal(-Vec3::Z, -Vec3::Z, to_view), Vec3::Z);
        // Mirrored instance: winding reversed, face normal sign arbitrary; still faces view.
        assert_eq!(facing_normal(-Vec3::Z, Vec3::Z, to_view), Vec3::Z);
        // Grazing smooth normal on a front face keeps its tangential part.
        let n = facing_normal(Vec3::new(1., 0., 0.1), Vec3::Z, to_view);
        assert!(n.z > 0. && n.x > 0.9);
        // No face normal: smooth normal unchanged.
        assert_eq!(facing_normal(Vec3::X, Vec3::ZERO, to_view), Vec3::X);
    }

    #[test]
    fn level_settings_map_sun_ambient_fog_and_fit_to_unit_peak() {
        let l = PreviewLight::from_settings(&settings(), Vec3::ZERO, 0.01);
        assert!(l.from_level);
        assert_eq!(l.label(), LEVEL_LABEL);
        assert!((l.toward_light - Vec3::Y).length() < 1e-6);
        // key (2, 1, 0.5) + ambient 0.5: red peak 2.5 fitted to 1.
        assert!((l.key - Vec3::new(0.8, 0.4, 0.2)).length() < 1e-6);
        assert!((l.ambient - Vec3::splat(0.2)).length() < 1e-6);
        assert!(l.factor(Vec3::Y).max_element() <= 1. + 1e-6);
        let fog = l.fog.unwrap();
        assert!((fog.start - 1.).abs() < 1e-6 && (fog.end - 3.).abs() < 1e-6);
        // Sun direction is taken from the scene centre.
        let l = PreviewLight::from_settings(&settings(), Vec3::new(1000., 0., 0.), 0.01);
        assert!((l.toward_light - Vec3::new(-1., 1., 0.).normalize()).length() < 1e-6);
    }

    /// D314: the sun's point in the light frame's display units, `(sun - centre) x scale`;
    /// the shaped LBP light keeps it (no point set until `apply_lighting` asks for one).
    #[test]
    fn d314_sun_display_is_the_level_sun_in_display_units() {
        let l = PreviewLight::from_settings(&settings(), Vec3::new(1000., 0., 0.), 0.01);
        let p = l.sun_display.unwrap();
        assert!((p - Vec3::new(-10., 10., 0.)).length() < 1e-5, "{p}");
        let shaped = l.lbp_shaped(Quat::from_rotation_x(0.3));
        assert_eq!(shaped.sun_display, Some(p));
        assert_eq!(shaped.sun_point, None);
        assert_eq!(PreviewLight::generic().sun_display, None);
    }

    #[test]
    fn unusable_settings_fall_back_to_generic_and_say_why() {
        let mut s = settings();
        s.sun_position = [0.; 3];
        let l = PreviewLight::from_settings(&s, Vec3::ZERO, 1.);
        assert!(!l.from_level);
        assert_eq!(l.toward_light, PreviewLight::generic().toward_light);
        assert!(l.notes.last().unwrap().contains("sun position unusable"));
        let mut s = settings();
        s.ambient_color[1] = f32::NAN;
        assert!(!PreviewLight::from_settings(&s, Vec3::ZERO, 1.).from_level);
        // Bad fog range keeps the light but drops fog.
        let mut s = settings();
        s.fog_far = s.fog_near;
        let l = PreviewLight::from_settings(&s, Vec3::ZERO, 1.);
        assert!(l.from_level && l.fog.is_none());
    }

    #[test]
    fn p2l_lbp_shape_uses_raw_values_hemisphere_and_grazing_rim() {
        let mut s = settings();
        s.rim_color = [0.2, 0.1, 0.0, 1.];
        s.rim_color2 = [0.0, 0.1, 0.4, 1.];
        s.exposure = 2.;
        let light = PreviewLight::from_settings(&s, Vec3::ZERO, 1.).lbp_shaped(Quat::IDENTITY);
        assert!(light.lbp);
        // Unfitted, game scales: sun colour x multiplier 2 x 0.3 x exposure 2.
        let key = Vec3::new(2., 1., 0.5) * 0.6;
        assert!((light.key - key).length() < 1e-5);
        // P2q: the scalars the game sends in .w (sun x multiplier, ambient, rim).
        assert!((light.scales - Vec4::new(1.2, 0.9, 1.2, 1.2)).length() < 1e-5);
        // P3f: a stored rim .w of 1.5 weights that rim's gloss only.
        let mut s2 = s;
        s2.rim_color[3] = 1.5;
        let l2 = PreviewLight::from_settings(&s2, Vec3::ZERO, 1.).lbp_shaped(Quat::IDENTITY);
        assert!((l2.scales.z - 1.8).abs() < 1e-5 && (l2.scales.w - 1.2).abs() < 1e-5);
        assert!((l2.rim[0] - light.rim[0]).length() < 1e-6);
        // P2t: fog start follows the Puter progress, in display units.
        let mut fog = light.clone();
        fog.display_scale = 2.;
        fog.pod_t = 1.;
        fog.pod_fog = true;
        let f = fog.lbp_fog();
        assert!((f.x - 5810. * 2.).abs() < 1e-2 && (f.y - 1. / 52648. / 2.).abs() < 1e-9);
        assert_eq!(PreviewLight::generic().lbp_fog(), Vec4::ZERO);
        // D274: a script's fog bias moves the preview fog's start and end by the same
        // amount, in display units; the span and the colour stay. No bias: no change.
        let preview = PreviewLight::from_settings(&s, Vec3::ZERO, 0.5);
        let stored = preview.fog.expect("the settings have a usable fog");
        assert_eq!(stored.biased(0., preview.display_scale), stored);
        let moved = stored.biased(20000., preview.display_scale);
        assert_eq!(
            (moved.start - stored.start, moved.end - stored.end),
            (10000., 10000.)
        );
        assert_eq!(moved.end - moved.start, stored.end - stored.start);
        assert_eq!(moved.color, stored.color);
        assert_eq!(stored.biased(f32::NAN, 1.), stored);
        assert_eq!(stored.biased(1., f32::INFINITY), stored);
        // Ambient 0.5 x 0.45 x 2; rims x 0.6 x 2.
        let ambient = 0.45;
        let rim_high = Vec3::new(0.0, 0.1, 0.4) * 1.2;
        let up = Vec3::Y;
        // Facing the viewer straight on: no rim; sun overhead lights fully.
        let f = light.lbp_factor(up, up);
        assert!((f - (Vec3::splat(ambient * 0.65) + key)).length() < 1e-5);
        // Grazing view: rim (0.9^2) with the upper colour for an upward normal.
        let g = light.lbp_factor(up, Vec3::X);
        assert!((g - f - rim_high * 0.81).length() < 1e-5);
        // Downward normal: lower ambient (0.35) and no sun.
        let d = light.lbp_factor(-up, -up);
        assert!((d - Vec3::splat(ambient * 0.35)).length() < 1e-5);
        // The turn moves the light with the scene.
        let t = PreviewLight::from_settings(&s, Vec3::ZERO, 1.)
            .lbp_shaped(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        assert!((t.toward_light - Vec3::NEG_X).length() < 1e-5);
    }

    #[test]
    fn shadows_follow_lbp_mode_only() {
        let plain = PreviewLight::from_settings(&settings(), Vec3::ZERO, 1.);
        assert!(!plain.shadows);
        assert!(plain.lbp_shaped(Quat::IDENTITY).shadows);
    }

    #[test]
    fn summary_labels_never_claim_game_lighting() {
        for light in [
            PreviewLight::generic(),
            PreviewLight::from_settings(&settings(), Vec3::ZERO, 1.),
        ] {
            let lines = summary(&light, 2, 5);
            assert!(lines[0].contains("not game"));
            assert!(lines.last().unwrap().contains("2 of 5"));
        }
    }

    #[test]
    fn level_fog_uses_its_own_near_far_and_colour_unless_pod() {
        let mut s = LevelSettings {
            sun_position: [0., 1., 0.],
            sun_position_scale: 1000.,
            sun_color: [1.; 4],
            ambient_color: [0.5; 4],
            sun_multiplier: 1.,
            exposure: 1.,
            fog_color: [0.4, 0.2, 0.1, 0.5],
            fog_near: 1000.,
            fog_far: 5000.,
            rim_color: [0.; 4],
            rim_color2: [0.; 4],
            baked_ao_bias: 0.,
            baked_ao_scale: 1.,
            ..LevelSettings::default()
        };
        let base = PreviewLight::from_settings(&s, Vec3::ZERO, 0.01);
        let lit = PreviewLight {
            display_scale: 0.01,
            ..base.lbp_shaped(Quat::IDENTITY)
        };
        let f = lit.lbp_fog();
        assert!((f.x - 10.).abs() < 1e-4, "{f:?}");
        assert!((f.y - 1. / 40.).abs() < 1e-6, "{f:?}");
        assert_eq!(f.w, 1.);
        // Stored colour is divided by its own w (0.5).
        assert!((lit.lbp_fog_color() - Vec3::new(0.8, 0.4, 0.2)).length() < 1e-6);
        let pod = PreviewLight {
            pod_fog: true,
            own_fog: false,
            ..lit.clone()
        };
        assert_eq!(pod.lbp_fog_color(), Vec3::from(GAME_FOG_COLOR));
        assert!((pod.lbp_fog().x - 25810. * 0.01).abs() < 1e-3);
        // An unusable range leaves the Pod constants.
        s.fog_far = s.fog_near;
        let bad = PreviewLight::from_settings(&s, Vec3::ZERO, 0.01);
        assert!(bad.game_fog.is_none());
    }
}
