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
//! - `Cam.SetCameraFromToUp__rrrffff` (0x2e9798) does not go through the level's camera:
//!   it builds the camera matrix from an eye, a point to look at and an up vector with
//!   the same look-at builder, writes it and its inverse into the render camera, stores
//!   its four floats there as near plane, far plane, one value whose use was not read and
//!   the field of view (the field `Cam.GetFov__` returns, radians), and rebuilds the
//!   projection. No blend ([`ScriptCamera`], docs/level-scripts.md, part 23).
//!
//! INFERRED: the up vector under an override is world +Y (the look-at build after the
//! blend was not read); the step runs once per tick, after the scripts; a camera written
//! with `SetCameraFromToUp` stands for the frame it was written in and no longer, and
//! wins over an override in that frame ([`LevelWorld::display_camera`]).
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
        self.matrix_with_up([0., 1., 0.])
    }

    /// The camera matrix of the pose with `up` as the up vector: right is forward x up
    /// at length 1, the matrix's up is right x forward. INFERRED: the usual look-at
    /// rule; FOUND is the layout (right, up, backward, eye). An up vector without a
    /// direction, or along the view, is replaced by world +Y, and that by +X as the
    /// right (INFERRED guards: no script passes such a vector).
    pub fn matrix_with_up(&self, up: [f32; 3]) -> [f32; 16] {
        let f = self.direction;
        let right = normalized(cross(f, up))
            .or_else(|| normalized(cross(f, [0., 1., 0.])))
            .unwrap_or([1., 0., 0.]);
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

/// The camera one `Cam.SetCameraFromToUp__rrrffff` call writes, in level units.
///
/// FOUND (0x2e9798): the eye is the first vector, the camera looks at the second, the
/// third is the up vector; the four floats are stored in the render camera as its near
/// plane, its far plane, a value that goes to the projection builder (what it does there
/// was not read) and its field of view, the field `Cam.GetFov__` returns. INFERRED: the
/// field of view is the vertical one (the one script that calls this passes 1.0, and
/// 57.3 degrees of vertical field of view were measured in the running game).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptCamera {
    /// The eye, and the direction to the point looked at, at length 1.
    pub pose: CameraPose,
    /// The up vector as passed.
    pub up: [f32; 3],
    pub near: f32,
    pub far: f32,
    /// The third float, as passed.
    pub lens_value: f32,
    /// The fourth float, as passed: the vertical field of view in radians.
    pub fov_y: f32,
    /// `PWorld.Frame` when the script made the call.
    pub frame: i32,
}

impl ScriptCamera {
    /// The camera of a call made on `frame`. `None` when the eye and the point looked at
    /// are the same point or one of them is not a number: such a call gives no direction
    /// (INFERRED guard; what the game's look-at builder makes of it was not read).
    pub fn of(call: &CameraFromToUp, frame: i32) -> Option<Self> {
        let v3 = |v: [f32; 4]| [v[0], v[1], v[2]];
        let (eye, to) = (v3(call.from), v3(call.to));
        if !eye.iter().all(|x| x.is_finite()) {
            return None;
        }
        let direction = normalized(std::array::from_fn(|i| to[i] - eye[i]))?;
        let [near, far, lens_value, fov_y] = call.params;
        Some(ScriptCamera {
            pose: CameraPose { eye, direction },
            up: v3(call.up),
            near,
            far,
            lens_value,
            fov_y,
            frame,
        })
    }

    /// The camera matrix the call writes ([`CameraPose::matrix_with_up`]).
    pub fn matrix(&self) -> [f32; 16] {
        self.pose.matrix_with_up(self.up)
    }

    /// The vertical field of view in degrees; `None` for a value a lens cannot have
    /// (not above 0, not below 180 degrees, not a number).
    pub fn fov_y_deg(&self) -> Option<f32> {
        (self.fov_y > 0. && self.fov_y < std::f32::consts::PI).then(|| self.fov_y.to_degrees())
    }
}

/// Which script camera a display draws with ([`LevelWorld::display_camera`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraSource {
    /// The camera `Cam.SetCameraFromToUp__rrrffff` wrote this frame.
    Script,
    /// The follow camera under the override of `Cam.SetOverrideCamera__rr`.
    Override,
}

/// The camera a display draws a frame with when a script has a say in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayCamera {
    pub source: CameraSource,
    pub pose: CameraPose,
    /// The up vector to build the camera with.
    pub up: [f32; 3],
    /// Vertical field of view in degrees; `None`: the display keeps its own (a script
    /// camera whose field of view no lens can have).
    pub fov_y_deg: Option<f32>,
    /// Near and far plane, level units.
    pub near: f32,
    pub far: f32,
}

impl DisplayCamera {
    /// The camera matrix (camera to world, level units).
    pub fn matrix(&self) -> [f32; 16] {
        self.pose.matrix_with_up(self.up)
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
    /// `PWorld.Frame` at the last such call ([`LevelWorld::script_camera`]).
    pub from_to_up_frame: Option<i32>,
    /// Ticks at whose end the camera of such a call stood, and those of them on which
    /// the override was in as well (counts for reports).
    pub script_camera_ticks: u64,
    pub script_camera_ticks_with_override: u64,
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
    /// PWorld, which has no level camera. Also counts the tick when a script wrote the
    /// camera on it ([`Self::script_camera`]).
    ///
    /// FOUND: the step is the first thing the camera's apply function does (0x6eb960).
    /// INFERRED: once per tick, after the scripts (which of the two camera update
    /// functions that call it runs in a level frame, and where, was not read).
    pub fn step_camera(&mut self) {
        // Written on this tick (its frame count has not gone up yet).
        let script = self.camera.from_to_up_frame == Some(self.pworld.frame)
            && self.script_camera().is_some();
        self.camera.script_camera_ticks += u64::from(script);
        if self.world.is_none() {
            return;
        }
        let o = &mut self.camera.override_camera;
        o.blend = step(o.blend, o.target, o.rate);
        o.blend_max = o.blend_max.max(o.blend);
        o.ticks_in += u64::from(o.is_in());
        let both = script && o.is_in();
        self.camera.script_camera_ticks_with_override += u64::from(both);
    }

    /// The camera a script wrote with `Cam.SetCameraFromToUp__rrrffff`, while it stands:
    /// from the call until the end of the tick after it. A script that wants the camera
    /// calls every tick, so the camera stands as long as it does; one tick without a
    /// call and the display has its own camera back. `None` also for a call without a
    /// direction ([`ScriptCamera::of`]).
    ///
    /// FOUND: the call writes the render camera at once and nothing blends it.
    /// INFERRED: how long it stands. The engine code that writes the render camera in a
    /// frame without such a call was not read; a level camera would write it on its
    /// next update. The tick is counted by `PWorld.Frame`, which goes up at the end of
    /// [`crate::LevelScripts::tick_level`].
    pub fn script_camera(&self) -> Option<ScriptCamera> {
        let c = &self.camera;
        let frame = c.from_to_up_frame?;
        // 0: made on the tick in progress, or on the last one before its frame count
        // went up. 1: made on the tick that just ended.
        let age = self.pworld.frame.wrapping_sub(frame);
        if !(0..=1).contains(&age) {
            return None;
        }
        ScriptCamera::of(c.from_to_up.as_ref()?, frame)
    }

    /// The camera a display should draw the frame with when a script has a say in it,
    /// given the follow camera it would draw with otherwise (eye and view direction,
    /// level units). `None`: the display's own camera stands.
    ///
    /// 1. A camera written with `SetCameraFromToUp` this frame ([`Self::script_camera`]):
    ///    that camera outright, with its up vector and its lens. FOUND: no blend.
    /// 2. Else, while the override is in ([`Self::camera_pose`]): the follow camera
    ///    mixed with the override, world +Y up, the override's lens.
    ///
    /// INFERRED: the order of the two. The first writes the render camera itself; the
    /// second is the level camera's own update, and which of the two writes last in a
    /// frame that has both was not read. No level of the measuring runs has both
    /// ([`CameraState::script_camera_ticks_with_override`]).
    pub fn display_camera(
        &self,
        follow_eye: [f32; 3],
        follow_dir: [f32; 3],
    ) -> Option<DisplayCamera> {
        if let Some(script) = self.script_camera() {
            return Some(DisplayCamera {
                source: CameraSource::Script,
                pose: script.pose,
                up: script.up,
                fov_y_deg: script.fov_y_deg(),
                near: script.near,
                far: script.far,
            });
        }
        let pose = self.camera_pose(follow_eye, follow_dir)?;
        Some(DisplayCamera {
            source: CameraSource::Override,
            pose,
            up: [0., 1., 0.],
            fov_y_deg: Some(OVERRIDE_FOV_DEG),
            near: OVERRIDE_NEAR,
            far: OVERRIDE_FAR,
        })
    }

    /// `Cam.GetCameraMatrix__`: the render camera's matrix. The view a display wrote
    /// ([`CameraState::view`]) when there is one. Without one: the matrix a script wrote
    /// with `SetCameraFromToUp` while that camera stands (FOUND: the call writes the
    /// matrix this native copies out); else the default follow camera
    /// ([`Self::default_follow_pose`]) under the override, and the identity when there
    /// is no player either (INFERRED stand-ins; they only decide where a scoreboard's
    /// zoom starts from).
    pub fn camera_matrix(&self) -> [f32; 16] {
        if let Some(view) = &self.camera.view {
            return view.camera;
        }
        if let Some(script) = self.script_camera() {
            return script.matrix();
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

    fn call(from: [f32; 3], to: [f32; 3], up: [f32; 3], params: [f32; 4]) -> CameraFromToUp {
        CameraFromToUp {
            from: [from[0], from[1], from[2], 1.],
            to: [to[0], to[1], to[2], 1.],
            up: [up[0], up[1], up[2], 0.],
            params,
        }
    }

    #[test]
    fn a_script_camera_is_an_eye_a_direction_an_up_vector_and_a_lens() {
        // Made-up numbers: an eye, a point 5 away along (0, 3, -4), near, far, the
        // value that is kept only, and a field of view of one radian.
        let lens = [10., 100_000., 0.25, 1.];
        let c = ScriptCamera::of(
            &call([100., 20., 500.], [100., 320., 100.], [0., 1., 0.], lens),
            7,
        )
        .unwrap();
        assert_eq!(c.pose.eye, [100., 20., 500.]);
        assert!(close3(c.pose.direction, [0., 0.6, -0.8]));
        assert_eq!(
            (c.near, c.far, c.lens_value, c.fov_y, c.frame),
            (10., 100_000., 0.25, 1., 7)
        );
        assert!(close(c.fov_y_deg().unwrap(), 57.295_78));
        // The matrix: right, up, backward, eye; axes of length 1 at right angles.
        let m = c.matrix();
        let col = |n: usize| [m[n * 4], m[n * 4 + 1], m[n * 4 + 2]];
        assert!(close3(col(0), [1., 0., 0.]));
        assert!(close3(col(1), [0., 0.8, 0.6]));
        assert!(close3(col(2), [0., -0.6, 0.8]));
        assert_eq!(col(3), [100., 20., 500.]);
        assert_eq!([m[3], m[7], m[11], m[15]], [0., 0., 0., 1.]);
        // With world +Y as up it is the matrix of the pose alone.
        assert_eq!(m, c.pose.matrix());
        // The up vector as given: +X as up rolls the camera a quarter turn.
        let rolled = ScriptCamera::of(&call([0.; 3], [0., 0., -1.], [1., 0., 0.], lens), 0)
            .unwrap()
            .matrix();
        assert!(close3([rolled[0], rolled[1], rolled[2]], [0., -1., 0.]));
        assert!(close3([rolled[4], rolled[5], rolled[6]], [1., 0., 0.]));
        // An up vector that need not be of length 1 or at a right angle to the view.
        let leaning = ScriptCamera::of(&call([0.; 3], [0., 0., -1.], [0., 3., -3.], lens), 0)
            .unwrap()
            .matrix();
        assert!(close3([leaning[4], leaning[5], leaning[6]], [0., 1., 0.]));
        // No up vector, or one along the view: world +Y, and +X as the right when the
        // camera looks along Y.
        let straight = call([0.; 3], [0., 0., -1.], [0.; 3], lens);
        assert_eq!(ScriptCamera::of(&straight, 0).unwrap().matrix(), IDENTITY);
        let along = call([0.; 3], [0., 0., -1.], [0., 0., 5.], lens);
        assert_eq!(ScriptCamera::of(&along, 0).unwrap().matrix(), IDENTITY);
        let down = ScriptCamera::of(&call([0.; 3], [0., -9., 0.], [0., 1., 0.], lens), 0)
            .unwrap()
            .matrix();
        assert!(close3([down[0], down[1], down[2]], [1., 0., 0.]));
        // No direction: the eye is the point looked at, or is not a number.
        assert_eq!(
            ScriptCamera::of(&call([1., 2., 3.], [1., 2., 3.], [0., 1., 0.], lens), 0),
            None
        );
        assert_eq!(
            ScriptCamera::of(
                &call([f32::NAN, 0., 0.], [1., 2., 3.], [0., 1., 0.], lens),
                0
            ),
            None
        );
        // A field of view no lens can have gives no field of view, and still a pose.
        for fov in [0., -1., std::f32::consts::PI, 4., f32::NAN] {
            let c = ScriptCamera::of(
                &call([0.; 3], [0., 0., -1.], [0., 1., 0.], [1., 2., 0., fov]),
                0,
            );
            assert_eq!(c.unwrap().fov_y_deg(), None, "{fov}");
        }
    }

    #[test]
    fn a_script_camera_stands_for_its_tick_and_goes_before_the_override() {
        use lbp_script_vm::natives::World;
        let mut w = LevelWorld {
            world: Some(0),
            ..Default::default()
        };
        w.pworld.frame = 500;
        let follow = ([0., 0., 1000.], [0., 0., -1.]);
        let tick = |w: &mut LevelWorld| {
            w.step_camera();
            w.advance_frame();
        };
        // Nothing set: the display's own camera stands.
        assert_eq!(w.script_camera(), None);
        assert_eq!(w.display_camera(follow.0, follow.1), None);
        // An override, fully in after one tick at the rate 1.
        w.set_override_camera([50., 0., 0., 1.], [1., 0., 0., 0.])
            .unwrap();
        w.set_override_camera_activation(true, 1.).unwrap();
        tick(&mut w);
        let over = w.display_camera(follow.0, follow.1).unwrap();
        assert_eq!(over.source, CameraSource::Override);
        assert_eq!(over.pose.eye, [50., 0., 0.]);
        assert_eq!(
            (over.fov_y_deg, over.near, over.far, over.up),
            (
                Some(OVERRIDE_FOV_DEG),
                OVERRIDE_NEAR,
                OVERRIDE_FAR,
                [0., 1., 0.]
            )
        );
        // A script writes the camera on this tick: it is the camera at once, with its
        // own up vector and lens, whatever the override says.
        let lens = [10., 100_000., 0., 1.];
        w.set_camera(call([0., 10., 20.], [0., 10., -80.], [1., 0., 0.], lens))
            .unwrap();
        let script = w.script_camera().unwrap();
        assert_eq!((script.frame, script.pose.eye), (501, [0., 10., 20.]));
        let shown = w.display_camera(follow.0, follow.1).unwrap();
        assert_eq!(shown.source, CameraSource::Script);
        assert_eq!(shown.pose, script.pose);
        assert_eq!(shown.up, [1., 0., 0.]);
        assert!(close(shown.fov_y_deg.unwrap(), 57.295_78));
        assert_eq!((shown.near, shown.far), (10., 100_000.));
        assert_eq!(shown.matrix(), script.matrix());
        // `GetCameraMatrix` without a view answers with the matrix the call wrote.
        assert_eq!(w.camera_matrix(), script.matrix());
        // After the tick it still stands: this is what a display draws.
        tick(&mut w);
        assert_eq!(
            w.display_camera(follow.0, follow.1).map(|c| c.source),
            Some(CameraSource::Script)
        );
        assert_eq!(
            (
                w.camera.script_camera_ticks,
                w.camera.script_camera_ticks_with_override
            ),
            (1, 1)
        );
        // A tick without a call: the override has the camera again, and the last call's
        // values are still there to read.
        tick(&mut w);
        assert_eq!(w.script_camera(), None);
        assert_eq!(
            w.display_camera(follow.0, follow.1).map(|c| c.source),
            Some(CameraSource::Override)
        );
        assert!(w.camera.from_to_up.is_some());
        assert_eq!(w.camera.script_camera_ticks, 1);
        assert_ne!(w.camera_matrix(), script.matrix());
        // The override out as well: nothing.
        w.set_override_camera_activation(false, 1.).unwrap();
        tick(&mut w);
        assert_eq!(w.display_camera(follow.0, follow.1), None);
        // A call every tick keeps the camera, and each tick is counted once.
        for n in 0..3 {
            let to = [0., 10., -80. - n as f32];
            w.set_camera(call([0., 10., 20.], to, [0., 1., 0.], lens))
                .unwrap();
            tick(&mut w);
            assert!(w.script_camera().is_some());
        }
        assert_eq!(
            (
                w.camera.script_camera_ticks,
                w.camera.script_camera_ticks_with_override,
                w.camera.from_to_up_calls
            ),
            (4, 1, 4)
        );
        // A call without a direction is no camera; a field of view no lens can have
        // leaves the display its own.
        w.set_camera(call([1.; 3], [1.; 3], [0., 1., 0.], lens))
            .unwrap();
        assert_eq!(w.display_camera(follow.0, follow.1), None);
        w.set_camera(call(
            [0.; 3],
            [0., 0., -1.],
            [0., 1., 0.],
            [10., 20., 0., 0.],
        ))
        .unwrap();
        let odd = w.display_camera(follow.0, follow.1).unwrap();
        assert_eq!((odd.source, odd.fov_y_deg), (CameraSource::Script, None));
        // A level without a PWorld has no level camera, and still the render camera.
        let mut bare = LevelWorld::default();
        bare.set_camera(call([0.; 3], [0., 0., -1.], [0., 1., 0.], lens))
            .unwrap();
        bare.step_camera();
        assert!(bare.script_camera().is_some());
        assert_eq!(bare.camera.script_camera_ticks, 1);
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
