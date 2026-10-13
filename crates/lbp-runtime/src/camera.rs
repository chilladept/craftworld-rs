//! The script camera: the override a script lays over the follow camera, and the screen
//! the HUD natives measure in (docs/level-scripts.md, part 20).
//!
//! FOUND in the 01.33 executable, by address:
//! - The level's camera keeps one override: a position and a direction (two vectors), a
//!   blend, a blend target and a blend rate. `Cam.SetOverrideCamera__rr` (0x3760e4)
//!   stores the two vectors as passed; `Cam.SetOverrideCameraActivation__bf` (0x374cec)
//!   sets the target to 1.0 or 0.0 and the rate. Nothing else writes the target or the
//!   rate but the camera's constructor (zero): the engine never ends an override.
//! - The camera's apply function (0x6eb650) steps the blend toward the target by the rate
//!   ([`step`], helper 0x38eb40), eases it ([`ease`]), mixes the eye linearly and the view
//!   direction on the arc ([`slerp`], helper 0x3918ec) from the follow camera to the
//!   override, and aims at a point 1000 units along the direction.
//! - While the blend is above 0.0001 the camera takes a plain lens: 60 degrees of
//!   vertical field of view, near 10, far 40000 (0x378470, 0x38367c).
//! - The render camera holds a camera matrix (camera to world: column W the eye, column Z
//!   pointing backward, as its look-at builder 0x2e7990 writes it) and a view-projection
//!   matrix; `Cam.GetCameraMatrix__` (0x7347e0) copies the first out, and
//!   `HUD.GetScreenSpacePosition__r` (0x2e7bdc, 0x2e74a0) projects with the second onto a
//!   screen whose size is two globals holding 1280 and 720 ([`screen_position`]).
//!
//! INFERRED: the up vector under an override is world +Y (the look-at build after the
//! blend was not read); the step runs once per tick, after the scripts.
use crate::transform::{self, IDENTITY};
use crate::LevelWorld;
use lbp_script_vm::natives::CameraFromToUp;

/// The HUD's screen in pixels (`HUD.GetXRes__`, `HUD.GetYRes__`). FOUND: the two globals
/// the natives return (0xc25eac, 0xc25eb0) hold 1280 and 720 in the executable's data,
/// and the screen-space projection reads the same two. INFERRED: constant in play.
pub const SCREEN: [i32; 2] = [1280, 720];

/// The blend above which the override takes the camera (FOUND, the constant the lens
/// test compares with).
pub const OVERRIDE_THRESHOLD: f32 = 0.0001;

/// The lens of the camera while an override is in: vertical field of view in degrees,
/// near and far plane in level units (FOUND: 1.0472 rad, 10, 40000).
pub const OVERRIDE_FOV_DEG: f32 = 60.;
pub const OVERRIDE_NEAR: f32 = 10.;
pub const OVERRIDE_FAR: f32 = 40_000.;

/// How far along the view direction the camera aims (FOUND: the focus point is the eye
/// plus the direction times this).
pub const OVERRIDE_FOCUS: f32 = 1000.;

/// Above this dot product [`slerp`] mixes linearly (FOUND).
pub const SLERP_LINEAR_ABOVE: f32 = 0.999;

/// What a zero w is replaced by in the screen projection (FOUND, read at 0xd9c820).
pub const ZERO_W: f32 = 0.0001;

/// The follow camera's eye against the Sackboy's position, looking down -z, vertical
/// field of view 60 degrees: one idle pose MEASURED in the game's memory in an emulator
/// (the viewer's follow camera uses the same pose). The real follow camera is not
/// modelled.
pub const FOLLOW_EYE: [f32; 3] = [296., 168., 1218.];

/// The blend one camera update later: `target` when it is nearer than `rate`, else
/// `blend` moved by `rate` toward it. FOUND (0x38eb40). A blend already on its target
/// stays there whatever the rate (INFERRED for a rate of zero or less, which no script
/// passes).
pub fn step(blend: f32, target: f32, rate: f32) -> f32 {
    if blend == target || (target - blend).abs() < rate {
        target
    } else if blend < target {
        blend + rate
    } else {
        blend - rate
    }
}

/// The weight of the override at a blend: `0.5 - 0.5 cos(pi blend)`. FOUND (the
/// constants at 0xdb0940).
pub fn ease(blend: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::PI * blend).cos()
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn mix(a: [f32; 3], b: [f32; 3], fa: f32, fb: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] * fa + b[i] * fb)
}

/// `v` at length 1, or `None` for a vector without a direction.
pub fn normalized(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = dot(v, v).sqrt();
    (len.is_finite() && len > 1e-12).then(|| v.map(|x| x / len))
}

/// From direction `a` to direction `b` on the arc between them, `w` in 0..1.
///
/// FOUND (0x3918ec): with a dot product above [`SLERP_LINEAR_ABOVE`] a plain linear mix,
/// below it the sine form `(sin((1 - w) t) a + sin(w t) b) / sin t`, `t` the angle
/// between them. An opposite pair takes a fallback axis in the game; which one was not
/// read, so here `a` turns about world +Y, or about +X when it points along Y
/// (INFERRED). The result is not brought to length 1.
pub fn slerp(a: [f32; 3], b: [f32; 3], w: f32) -> [f32; 3] {
    let d = dot(a, b);
    if d > SLERP_LINEAR_ABOVE {
        return mix(a, b, 1. - w, w);
    }
    let angle = d.clamp(-1., 1.).acos();
    let sin = angle.sin();
    if sin.abs() < 1e-4 {
        let side = normalized(cross([0., 1., 0.], a))
            .or_else(|| normalized(cross([1., 0., 0.], a)))
            .unwrap_or([1., 0., 0.]);
        let turn = w * std::f32::consts::PI;
        return mix(a, side, turn.cos(), turn.sin() * dot(a, a).sqrt());
    }
    mix(
        a,
        b,
        ((1. - w) * angle).sin() / sin,
        (w * angle).sin() / sin,
    )
}

/// Where a camera is and which way it looks, in level units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    pub eye: [f32; 3],
    /// Length 1.
    pub direction: [f32; 3],
}

impl CameraPose {
    /// The camera matrix of the pose (camera to world): columns right, up, backward and
    /// the eye, as the game's look-at builder writes them (FOUND, 0x2e7990), with world
    /// +Y as the up vector (INFERRED). A camera that looks straight up or down takes +X
    /// as its right.
    pub fn matrix(&self) -> [f32; 16] {
        let f = self.direction;
        let right = normalized(cross(f, [0., 1., 0.])).unwrap_or([1., 0., 0.]);
        let up = cross(right, f);
        let mut m = IDENTITY;
        m[0..3].copy_from_slice(&right);
        m[4..7].copy_from_slice(&up);
        m[8..11].copy_from_slice(&f.map(|x| -x));
        m[12..15].copy_from_slice(&self.eye);
        m
    }

    /// The point the game aims the camera at ([`OVERRIDE_FOCUS`]).
    pub fn focus(&self) -> [f32; 3] {
        mix(self.eye, self.direction, 1., OVERRIDE_FOCUS)
    }
}

/// The camera at override weight `w` (an [`ease`]d blend): the eye mixed linearly, the
/// direction by [`slerp`]. FOUND. Both directions are taken at length 1 and so is the
/// result (INFERRED: the game stores the override's direction as passed and its look-at
/// brings the result to length 1; the scoreboard script passes a direction of length 1).
/// An override without a direction keeps the follow camera's (INFERRED guard: a script
/// that activates before it sets one), and a position that is not a number leaves the
/// follow camera as it is.
pub fn blend_pose(follow: CameraPose, eye: [f32; 3], direction: [f32; 3], w: f32) -> CameraPose {
    let turned = normalized(direction)
        .and_then(|to| normalized(slerp(follow.direction, to, w)))
        .unwrap_or(follow.direction);
    let mixed = CameraPose {
        eye: std::array::from_fn(|i| follow.eye[i] + (eye[i] - follow.eye[i]) * w),
        direction: turned,
    };
    if mixed.eye.iter().all(|x| x.is_finite()) {
        mixed
    } else {
        follow
    }
}

/// A level point on the screen, as `HUD.GetScreenSpacePosition__r` gives it: (x, y) in
/// pixels of `screen` from the top left corner, y down; then the depth after the divide
/// and 0.
///
/// FOUND (0x2e7bdc, 0x2e74a0): `clip = view_proj x (x, y, z, 1)`, whatever the point's
/// own w; divided by `clip.w`, or by [`ZERO_W`] when that is exactly 0;
/// `x = (ndc.x + 1) / 2 x width`, `y = (1 - ndc.y) / 2 x height`. There is no test for a
/// point behind the camera: its w is negative and it comes out mirrored through the
/// centre. INFERRED: the third component is the divided depth (no script reads it).
pub fn screen_position(view_proj: &[f32; 16], point: [f32; 4], screen: [i32; 2]) -> [f32; 4] {
    let p = [point[0], point[1], point[2], 1.];
    let clip: [f32; 4] =
        std::array::from_fn(|row| (0..4).map(|c| view_proj[c * 4 + row] * p[c]).sum());
    let w = if clip[3] == 0. { ZERO_W } else { clip[3] };
    let ndc = [clip[0] / w, clip[1] / w, clip[2] / w];
    [
        (ndc[0] + 1.) * 0.5 * screen[0] as f32,
        (1. - ndc[1]) * 0.5 * screen[1] as f32,
        ndc[2],
        0.,
    ]
}

/// A view-projection matrix for a camera matrix and a lens, for a caller that has no
/// renderer of its own: x to the right, y up, w the distance in front of the camera,
/// depth -1 at `near` and 1 at `far`. `None` for a camera matrix without an inverse.
/// The game's own projection matrix was not read; its x, y and w follow from the lens.
pub fn view_projection(
    camera: &[f32; 16],
    fov_y_deg: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> Option<[f32; 16]> {
    let view = transform::inverse(camera)?;
    let f = 1. / (fov_y_deg.to_radians() * 0.5).tan();
    let mut p = [0.; 16];
    p[0] = f / aspect;
    p[5] = f;
    p[10] = (far + near) / (near - far);
    p[11] = -1.;
    p[14] = 2. * far * near / (near - far);
    Some(transform::mul(&p, &view))
}

/// The override a script lays over the follow camera (`Cam.SetOverrideCamera__rr`,
/// `Cam.SetOverrideCameraActivation__bf`). One per level camera: the last caller wins.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OverrideCamera {
    /// As passed, level units.
    pub position: [f32; 4],
    /// As passed; the game does not bring it to length 1.
    pub direction: [f32; 4],
    /// 1.0 while a script has the override on, else 0.0.
    pub target: f32,
    /// How far the blend moves per tick.
    pub rate: f32,
    /// 0 = the follow camera, 1 = the override; stepped by [`LevelWorld::step_camera`].
    pub blend: f32,
    pub set_calls: u64,
    pub activation_calls: u64,
    /// Activation calls that turned the override on.
    pub activations_on: u64,
    /// Ticks after which the blend was above [`OVERRIDE_THRESHOLD`].
    pub ticks_in: u64,
    /// The highest blend reached.
    pub blend_max: f32,
}

impl OverrideCamera {
    /// The override's share of the camera now ([`ease`] of the blend).
    pub fn weight(&self) -> f32 {
        ease(self.blend)
    }

    /// Does the override have a share of the camera (and the camera its plain lens)?
    pub fn is_in(&self) -> bool {
        self.blend > OVERRIDE_THRESHOLD
    }
}

/// The render camera as the caller last drew with it; scripts read it and never write
/// it. Both matrices in the VM's m44 order, level units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraView {
    /// Camera to world: column W the eye, column Z pointing backward, axes of length 1.
    pub camera: [f32; 16],
    /// Level point to clip space: x to the right, y up, w positive in front.
    pub view_proj: [f32; 16],
}

/// Camera state: what scripts set through `Cam.*` natives, for a display to read, and
/// the camera the display drew with, for the scripts to read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CameraState {
    /// The last `Cam.SetCameraFromToUp__rrrffff`.
    pub from_to_up: Option<CameraFromToUp>,
    pub from_to_up_calls: u64,
    /// The last `Cam.SetDOFAndFog__fff` (as passed).
    pub dof_and_fog: Option<[f32; 3]>,
    pub motion_blur_clears: u64,
    /// The camera a display drew the last frame with. `None` when nothing draws (tests,
    /// the measuring runs): see [`LevelWorld::camera_matrix`] and
    /// [`LevelWorld::screen_position`] for what the scripts are told then.
    pub view: Option<CameraView>,
    pub override_camera: OverrideCamera,
    /// `Cam.GetCameraMatrix__` calls.
    pub matrix_queries: u64,
    /// `HUD.GetScreenSpacePosition__r` calls, and those answered without a view.
    pub screen_queries: u64,
    pub screen_queries_without_view: u64,
    /// Screen queries whose answer was more than [`OFF_SCREEN_MARGIN`] pixels outside
    /// the screen (the test of the speech-bubble script; a count for reports).
    pub screen_queries_off_screen: u64,
}

/// The margin of the one script that asks for a screen position: a speech bubble counts
/// as off screen when its point is more than this many pixels outside (FOUND, script
/// listing). Used for a count only.
pub const OFF_SCREEN_MARGIN: f32 = 100.;

impl LevelWorld {
    /// The follow camera a caller without one is given: [`FOLLOW_EYE`] from the first
    /// player's Sackboy that has a position, looking down -z. `None` without a player.
    pub fn default_follow_pose(&self) -> Option<CameraPose> {
        let at = self.players.iter().find_map(|p| {
            let m = self.things.get(p.thing as usize)?.world_matrix?;
            Some([m[12], m[13], m[14]])
        })?;
        Some(CameraPose {
            eye: std::array::from_fn(|i| at[i] + FOLLOW_EYE[i]),
            direction: [0., 0., -1.],
        })
    }

    /// The camera a display should draw with while a script's override is in, given the
    /// follow camera it would draw with otherwise (eye and view direction, level units);
    /// `None` while the blend is at or below [`OVERRIDE_THRESHOLD`], when the follow
    /// camera stands. The lens then is [`OVERRIDE_FOV_DEG`], [`OVERRIDE_NEAR`],
    /// [`OVERRIDE_FAR`].
    pub fn camera_pose(&self, follow_eye: [f32; 3], follow_dir: [f32; 3]) -> Option<CameraPose> {
        let o = &self.camera.override_camera;
        if !o.is_in() {
            return None;
        }
        let follow = CameraPose {
            eye: follow_eye,
            direction: normalized(follow_dir)?,
        };
        let v3 = |v: [f32; 4]| [v[0], v[1], v[2]];
        Some(blend_pose(
            follow,
            v3(o.position),
            v3(o.direction),
            o.weight(),
        ))
    }

    /// One camera update: the override's blend takes a [`step`]. Called at the end of
    /// each tick ([`crate::LevelScripts::tick_level`]); nothing in a level without a
    /// PWorld, which has no level camera.
    ///
    /// FOUND: the step is the first thing the camera's apply function does (0x6eb960).
    /// INFERRED: once per tick, after the scripts (which of the two camera update
    /// functions that call it runs in a level frame, and where, was not read).
    pub fn step_camera(&mut self) {
        if self.world.is_none() {
            return;
        }
        let o = &mut self.camera.override_camera;
        o.blend = step(o.blend, o.target, o.rate);
        o.blend_max = o.blend_max.max(o.blend);
        o.ticks_in += u64::from(o.is_in());
    }

    /// `Cam.GetCameraMatrix__`: the render camera's matrix. The view a display wrote
    /// ([`CameraState::view`]) when there is one. Without one: the default follow camera
    /// ([`Self::default_follow_pose`]) under the override, and the identity when there
    /// is no player either (INFERRED stand-ins; they only decide where a scoreboard's
    /// zoom starts from).
    pub fn camera_matrix(&self) -> [f32; 16] {
        if let Some(view) = &self.camera.view {
            return view.camera;
        }
        match self.default_follow_pose() {
            Some(follow) => self
                .camera_pose(follow.eye, follow.direction)
                .unwrap_or(follow)
                .matrix(),
            None => IDENTITY,
        }
    }

    /// `HUD.GetScreenSpacePosition__r` ([`screen_position`] on [`SCREEN`]) with the view
    /// a display wrote. Without a view the answer is the middle of the screen: INFERRED
    /// stand-in, so that a run without a display sees every point as on screen, as it
    /// did while the native was unbound (its default (0, 0) is on screen too).
    pub fn screen_position(&mut self, point: [f32; 4]) -> [f32; 4] {
        let c = &mut self.camera;
        c.screen_queries += 1;
        let Some(view) = c.view else {
            c.screen_queries_without_view += 1;
            return [SCREEN[0] as f32 / 2., SCREEN[1] as f32 / 2., 0., 0.];
        };
        let at = screen_position(&view.view_proj, point, SCREEN);
        let (m, [w, h]) = (OFF_SCREEN_MARGIN, SCREEN.map(|n| n as f32));
        // As the script tests it; a position that is not a number is not "off".
        c.screen_queries_off_screen +=
            u64::from(at[0] < -m || at[0] > w + m || at[1] < -m || at[1] > h + m);
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn close3(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| close(a[i], b[i]))
    }

    #[test]
    fn the_blend_steps_linearly_and_lands_on_its_target() {
        // The scoreboard's rate: 20 steps each way, the last one onto the target itself.
        let mut blend = 0.;
        let mut steps = 0;
        while blend != 1. {
            let next = step(blend, 1., 0.05);
            assert!(next > blend && next <= 1.);
            blend = next;
            steps += 1;
            assert!(steps <= 21);
        }
        assert!((20..=21).contains(&steps), "{steps}");
        let up = steps;
        steps = 0;
        while blend != 0. {
            blend = step(blend, 0., 0.05);
            steps += 1;
            assert!(steps <= 21);
        }
        assert!((20..=21).contains(&steps), "{steps}");
        assert_eq!((up, steps), (STEPS_UP, STEPS_DOWN));
        // One step moves by the rate.
        assert_eq!(step(0.5, 1., 0.125), 0.625);
        assert_eq!(step(0.5, 0., 0.125), 0.375);
        // A rate larger than the gap lands on the target; a gap of exactly the rate is
        // not "nearer than the rate" and is stepped over.
        assert_eq!(step(0.9, 1., 0.5), 1.);
        assert_eq!(step(0.25, 0., 0.5), 0.);
        assert_eq!(step(0.5, 1., 0.5), 1.);
        // No rate, no motion; on the target it stays.
        assert_eq!(step(0.3, 1., 0.), 0.3);
        assert_eq!(step(1., 1., 0.), 1.);
        assert_eq!(step(0., 0., 0.05), 0.);
    }

    /// Steps the f32 arithmetic takes with the rate 0.05 (MEASURED here).
    const STEPS_UP: i32 = 20;
    const STEPS_DOWN: i32 = 20;

    #[test]
    fn the_ease_is_half_a_cosine() {
        assert!(close(ease(0.), 0.) && close(ease(0.5), 0.5) && close(ease(1.), 1.));
        assert!(close(ease(0.25), 0.146_446_6) && close(ease(0.75), 0.853_553_4));
        let mut last = ease(0.);
        for n in 1..=100 {
            let w = ease(n as f32 / 100.);
            assert!(w >= last, "{n}");
            last = w;
        }
    }

    #[test]
    fn directions_mix_on_the_arc() {
        let (x, z) = ([1., 0., 0.], [0., 0., -1.]);
        // A right angle: half way is the half angle, at length 1.
        let half = slerp(z, x, 0.5);
        let r = std::f32::consts::FRAC_1_SQRT_2;
        assert!(close3(half, [r, 0., -r]), "{half:?}");
        assert!(close3(slerp(z, x, 0.), z) && close3(slerp(z, x, 1.), x));
        let third = slerp(z, x, 1. / 3.);
        assert!(close(dot(third, third), 1.));
        assert!(close(dot(third, z), (std::f32::consts::FRAC_PI_6).cos()));
        // Nearly parallel (dot above 0.999): the linear branch, which is the plain mix.
        let near = normalized([0.02, 0., -1.]).unwrap();
        assert!(dot(z, near) > SLERP_LINEAR_ABOVE);
        assert_eq!(slerp(z, near, 0.25), mix(z, near, 0.75, 0.25));
        // Just below the threshold the sine form runs, and agrees to within the bend.
        let wider = normalized([0.05, 0., -1.]).unwrap();
        assert!(dot(z, wider) < SLERP_LINEAR_ABOVE);
        let s = slerp(z, wider, 0.5);
        assert!(close(dot(s, s), 1.));
        // An opposite pair turns about +Y and still ends on the target.
        let back = [0., 0., 1.];
        let mid = slerp(z, back, 0.5);
        assert!(close(dot(mid, mid), 1.) && close(mid[1], 0.) && close(mid[2], 0.));
        assert!(close3(slerp(z, back, 1.), back));
        // Along the axis itself: about +X.
        let (up, down) = ([0., 1., 0.], [0., -1., 0.]);
        let mid = slerp(up, down, 0.5);
        assert!(close(dot(mid, mid), 1.) && close(mid[1], 0.));
    }

    #[test]
    fn the_pose_goes_from_the_follow_camera_to_the_override() {
        let follow = CameraPose {
            eye: [100., 50., 1200.],
            direction: [0., 0., -1.],
        };
        let (eye, dir) = ([300., 250., 200.], [1., 0., 0.]);
        assert_eq!(blend_pose(follow, eye, dir, 0.), follow);
        let full = blend_pose(follow, eye, dir, 1.);
        assert_eq!(full.eye, eye);
        assert!(close3(full.direction, dir));
        let half = blend_pose(follow, eye, dir, 0.5);
        assert_eq!(half.eye, [200., 150., 700.]);
        let r = std::f32::consts::FRAC_1_SQRT_2;
        assert!(close3(half.direction, [r, 0., -r]));
        // A direction that is not of length 1 still gives a pose of length 1.
        let long = blend_pose(follow, eye, [3., 0., 0.], 0.3);
        assert!(close(dot(long.direction, long.direction), 1.));
        // No direction set yet: the follow camera's at the end, never a zero vector.
        let none = blend_pose(follow, eye, [0.; 3], 1.);
        assert_eq!(none.direction, follow.direction);
        assert_eq!(none.eye, eye);
        // A position that is not a number leaves the follow camera.
        assert_eq!(blend_pose(follow, [f32::NAN, 0., 0.], dir, 0.5), follow);
        // The focus point is 1000 along the direction.
        assert_eq!(follow.focus(), [100., 50., 200.]);
    }

    #[test]
    fn the_camera_matrix_has_the_eye_in_w_and_looks_along_minus_z() {
        let straight = CameraPose {
            eye: [1., 2., 3.],
            direction: [0., 0., -1.],
        };
        let mut want = IDENTITY;
        want[12..15].copy_from_slice(&[1., 2., 3.]);
        assert_eq!(straight.matrix(), want);
        // Looking along +x: right is +z, up +y, column Z (backward) -x.
        let side = CameraPose {
            eye: [0.; 3],
            direction: [1., 0., 0.],
        }
        .matrix();
        assert!(close3([side[0], side[1], side[2]], [0., 0., 1.]));
        assert!(close3([side[4], side[5], side[6]], [0., 1., 0.]));
        assert!(close3([side[8], side[9], side[10]], [-1., 0., 0.]));
        // Straight down: +X is the right, and the axes stay of length 1.
        let down = CameraPose {
            eye: [0.; 3],
            direction: [0., -1., 0.],
        }
        .matrix();
        assert!(close3([down[0], down[1], down[2]], [1., 0., 0.]));
        assert!(close3([down[8], down[9], down[10]], [0., 1., 0.]));
    }

    /// A camera at (0, 0, 1000) looking down -z, 60 degrees, 16:9.
    fn view() -> [f32; 16] {
        let camera = CameraPose {
            eye: [0., 0., 1000.],
            direction: [0., 0., -1.],
        }
        .matrix();
        view_projection(
            &camera,
            OVERRIDE_FOV_DEG,
            16. / 9.,
            OVERRIDE_NEAR,
            OVERRIDE_FAR,
        )
        .unwrap()
    }

    #[test]
    fn a_level_point_lands_on_the_screen_in_pixels_from_the_top_left() {
        let m = view();
        let at = |x: f32, y: f32, z: f32, w: f32| screen_position(&m, [x, y, z, w], SCREEN);
        // On the view axis: the middle. The point's own w does not count.
        let mid = at(0., 0., 0., 1.);
        assert!(close(mid[0], 640.) && close(mid[1], 360.), "{mid:?}");
        assert_eq!(at(0., 0., 0., 0.), at(0., 0., 0., 7.));
        assert_eq!(mid[3], 0.);
        // The frame's edge at distance 1000: half the height is 1000 tan 30.
        let h = 1000. * (30f32).to_radians().tan();
        let w = h * 16. / 9.;
        let top_right = at(w, h, 0., 1.);
        assert!((top_right[0] - 1280.).abs() < 0.01 && top_right[1].abs() < 0.01);
        let bottom_left = at(-w, -h, 0., 1.);
        assert!(bottom_left[0].abs() < 0.01 && (bottom_left[1] - 720.).abs() < 0.01);
        // y is down: a point above the axis has the smaller y.
        assert!(at(0., 100., 0., 1.)[1] < 360. && at(100., 0., 0., 1.)[0] > 640.);
        // Depth: between -1 at the near plane and 1 at the far plane.
        assert!(close(at(0., 0., 990., 1.)[2], -1.));
        assert!(mid[2] > -1. && mid[2] < 1.);
        // Behind the camera: no test, the point is mirrored through the middle.
        let behind = at(w / 2., h / 2., 2000., 1.);
        assert!((behind[0] - 320.).abs() < 0.01 && (behind[1] - 540.).abs() < 0.01);
        // In the camera's own plane w is exactly 0 and the divide is by 0.0001.
        let flat = at(1., 0., 1000., 1.);
        let f = 1. / (30f32).to_radians().tan();
        let want = (f / (16. / 9.) / ZERO_W + 1.) * 640.;
        assert!((flat[0] - want).abs() / want < 1e-4, "{flat:?}");
        assert!(flat[0].is_finite() && flat[1].is_finite());
        // The numbers are the matrix's alone: a clip matrix of the identity maps ndc.
        let ndc = |x, y| screen_position(&IDENTITY, [x, y, 0., 1.], SCREEN);
        assert_eq!(ndc(1., 1.), [1280., 0., 0., 0.]);
        assert_eq!(ndc(-1., -1.), [0., 720., 0., 0.]);
        assert_eq!(ndc(0., 0.), [640., 360., 0., 0.]);
        // A camera matrix that cannot be inverted gives no view.
        assert_eq!(view_projection(&[0.; 16], 60., 1., 1., 2.), None);
    }
}
