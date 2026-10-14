//! P1b: `--pod-menu CLASS`, the Pod main menu as the game shows it with the Puter open:
//! the script's buttons as pill labels down the right side of a globe, the last one on the
//! moon, and the profile Polaroid between them (placement in `lbp_ui_script::pod`, from the
//! RPCS3 reference). The scene is a level camera looking into -Z with the globe and moon
//! placed so they project to the measured frame positions.
//! Fidelity: the globe and moon are PLACEHOLDER spheres (flat colours) until the PC finds
//! the pod meshes; the Polaroid is a blank card; the workshop background is a flat colour;
//! pill style (white, cyan highlight with a pulse) follows the captures by eye. The window
//! says so unless `--qa-hide-ui`.
//! Keys: Up/Down move the highlight, Enter prints the chosen label (the live menu, running
//! the script per frame, is a later slice).
//! Usage: `--pod-menu CLASS --snapshot SNAPSHOT --game-dir DIR [--ui-script-function NAME]
//! [--qa-hide-ui] [--qa-capture local-analysis/NEW.png]`.
use crate::gooey_ui::{demo_capture, demo_capture_path, DemoCapture};
use crate::ui_script::{run_script, DEFAULT_FUNCTION};
use bevy::prelude::*;
use lbp_ui_script::pod::{layout, PodMenu, ASPECT, GLOBE, MOON};

/// Vertical field of view (UNKNOWN in the game; only the projected positions matter here).
const FOV: f32 = std::f32::consts::FRAC_PI_4;
/// Distance of the spheres from the camera, in display units.
const DEPTH: f32 = 10.;
/// Globe spin, radians per second (slow, by eye from the captures).
const SPIN: f32 = 0.15;

#[derive(Resource)]
struct Menu {
    menu: PodMenu,
    focused: usize,
}

#[derive(Component)]
struct Globe;

#[derive(Component)]
struct PillNode(usize);

#[derive(Component)]
struct OverlayRoot;

/// The point at frame fractions `f` on the plane `DEPTH` in front of the camera.
pub fn frame_to_world(f: [f32; 2]) -> Vec3 {
    let half = (FOV / 2.).tan() * DEPTH;
    Vec3::new(
        (f[0] - 0.5) * 2. * half * ASPECT,
        (0.5 - f[1]) * 2. * half,
        -DEPTH,
    )
}

/// A radius given as a fraction of the frame height, in display units at `DEPTH`.
pub fn height_to_world(r: f32) -> f32 {
    r * 2. * (FOV / 2.).tan() * DEPTH
}

pub fn run_pod_menu() -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        all.iter()
            .position(|a| a == flag)
            .and_then(|i| all.get(i + 1))
            .cloned()
    };
    let class = value("--pod-menu").ok_or("--pod-menu requires a CLASS")?;
    let snapshot = value("--snapshot").ok_or("--pod-menu requires --snapshot")?;
    let game = value("--game-dir").ok_or("--pod-menu requires --game-dir")?;
    let function = value("--ui-script-function").unwrap_or_else(|| DEFAULT_FUNCTION.into());
    let hide = all.iter().any(|a| a == "--qa-hide-ui");
    let (report, _) = run_script(snapshot.as_ref(), game.as_ref(), &class, &function)?;
    let menu = layout(&report.tree);
    println!(
        "Pod menu: {} pills ({}), polaroid {}",
        menu.pills.len(),
        menu.pills
            .iter()
            .map(|p| format!(
                "{} {:?} at {:.2}/{:.2}",
                p.label, p.on, p.centre[0], p.centre[1]
            ))
            .collect::<Vec<_>>()
            .join(", "),
        menu.polaroid.is_some()
    );
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.42, 0.33, 0.24)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LBP2 Asset Explorer - Pod menu".into(),
                resolution: (1280, 720).into(),
                visible: false,
                focused: crate::activate_on_show(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Menu { menu, focused: 0 })
        .insert_resource(DemoCapture(demo_capture_path()))
        .add_systems(
            Startup,
            move |c: Commands,
                  m: ResMut<Assets<Mesh>>,
                  s: ResMut<Assets<StandardMaterial>>,
                  menu: Res<Menu>| setup(c, m, s, menu, hide),
        )
        .add_systems(
            Update,
            (
                crate::place_on_second_monitor,
                spin,
                keys,
                paint,
                demo_capture,
            )
                .chain(),
        )
        .run();
    Ok(())
}

fn sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    (centre, radius): ([f32; 2], f32),
    colour: Color,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(height_to_world(radius)).mesh().uv(48, 24))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: colour,
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_translation(frame_to_world(centre)),
        ))
        .id()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    menu: Res<Menu>,
    hide: bool,
) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV,
            ..default()
        }),
        Transform::IDENTITY,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 6000.,
            ..default()
        },
        Transform::from_xyz(-4., 6., 4.).looking_at(Vec3::new(0., 0., -DEPTH), Vec3::Y),
    ));
    let globe = sphere(
        &mut commands,
        &mut meshes,
        &mut materials,
        GLOBE,
        Color::srgb(0.20, 0.42, 0.62),
    );
    commands.entity(globe).insert(Globe);
    sphere(
        &mut commands,
        &mut meshes,
        &mut materials,
        MOON,
        Color::srgb(0.62, 0.62, 0.60),
    );
    let note = (!hide).then_some(
        "Placeholder globe, moon, Polaroid and background; pill positions from the \
         RPCS3 reference. Up/Down move, Enter prints the choice.",
    );
    spawn_overlay(&mut commands, &menu.menu, |f| f, note);
}

/// The pills and Polaroid as UI nodes at frame fractions (`remap` maps the 16:9 reference
/// fractions to the window), plus an optional fidelity note.
fn spawn_overlay(
    commands: &mut Commands,
    menu: &PodMenu,
    remap: impl Fn([f32; 2]) -> [f32; 2],
    note: Option<&str>,
) {
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            OverlayRoot,
        ))
        .id();
    let centred = |f: [f32; 2]| Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(f[0] * 100.),
        top: Val::Percent(f[1] * 100.),
        ..default()
    };
    for (i, pill) in menu.pills.iter().enumerate() {
        let node = commands
            .spawn((
                Node {
                    padding: UiRect::axes(Val::Px(18.), Val::Px(6.)),
                    border_radius: BorderRadius::all(Val::Px(999.)),
                    ..centred(remap(pill.centre))
                },
                UiTransform::from_translation(Val2::percent(-50., -50.)),
                BackgroundColor(Color::WHITE),
                PillNode(i),
                children![(
                    Text::new(pill.label.clone()),
                    TextFont::from_font_size(22.),
                    TextColor(Color::srgb(0.12, 0.12, 0.14)),
                )],
            ))
            .id();
        commands.entity(root).add_child(node);
    }
    if let Some(f) = menu.polaroid {
        let card = commands
            .spawn((
                Node {
                    width: Val::Vh(10.),
                    height: Val::Vh(12.),
                    padding: UiRect::new(Val::Vh(0.8), Val::Vh(0.8), Val::Vh(0.8), Val::Vh(2.5)),
                    ..centred(remap(f))
                },
                UiTransform::from_translation(Val2::percent(-50., -50.)),
                BackgroundColor(Color::srgb(0.96, 0.95, 0.90)),
                children![(
                    Node {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.35, 0.35, 0.38)),
                )],
            ))
            .id();
        commands.entity(root).add_child(card);
    }
    if let Some(note) = note {
        let note = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(12.),
                    bottom: Val::Px(10.),
                    ..default()
                },
                Text::new(note),
                TextFont::from_font_size(14.),
                TextColor(Color::srgba(1., 1., 1., 0.8)),
            ))
            .id();
        commands.entity(root).add_child(note);
    }
}

fn spin(time: Res<Time>, mut globes: Query<&mut Transform, With<Globe>>) {
    for mut t in &mut globes {
        t.rotate_y(SPIN * time.delta_secs());
    }
}

/// Keyboard arrows/Enter or gamepad D-pad/Cross; with the Pod shell only while open.
fn keys(
    input: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    puter: Option<Res<PuterState>>,
    mut menu: ResMut<Menu>,
) {
    let n = menu.menu.pills.len();
    if n == 0 || puter.is_some_and(|p| p.t < 0.99) {
        return;
    }
    let pad = |b| pads.iter().any(|g| g.just_pressed(b));
    if input.just_pressed(KeyCode::ArrowDown) || pad(GamepadButton::DPadDown) {
        menu.focused = (menu.focused + 1) % n;
    }
    if input.just_pressed(KeyCode::ArrowUp) || pad(GamepadButton::DPadUp) {
        menu.focused = (menu.focused + n - 1) % n;
    }
    if input.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) || pad(GamepadButton::South) {
        println!("Pod menu: chose {}", menu.menu.pills[menu.focused].label);
    }
}

fn paint(
    time: Res<Time>,
    menu: Res<Menu>,
    puter: Option<Res<PuterState>>,
    mut root: Query<&mut Visibility, With<OverlayRoot>>,
    mut pills: Query<(&PillNode, &mut BackgroundColor, &mut UiTransform)>,
) {
    // With the Pod shell, the menu shows once the Puter is open.
    let shown = puter.is_none_or(|p| p.t >= 0.99);
    for mut v in &mut root {
        *v = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let pulse = (time.elapsed_secs() * 4.).sin() * 0.5 + 0.5;
    for (pill, mut background, mut transform) in &mut pills {
        let focused = pill.0 == menu.focused;
        *background = BackgroundColor(if focused {
            Color::srgb(0.30 + 0.10 * pulse, 0.85, 0.95)
        } else {
            Color::WHITE
        });
        transform.scale = Vec2::splat(if focused { 1.08 + 0.03 * pulse } else { 1. });
    }
}

/// P2: the level camera that shows the globe where the RPCS3 reference has it (centre
/// GLOBE.0 of a 16:9 frame, radius GLOBE.1 of the height), looking -Z with no tilt, for a
/// vertical field of view `fov`. `centre` and `radius` are the globe's in display units;
/// returns the camera position (the orbit target is straight ahead of it at the globe's
/// depth). Only the projected centre and radius are matched; the game's camera itself is
/// script-driven and UNKNOWN (PlanetsWorld.UpdatePodCamera).
pub fn pod_camera(centre: Vec3, radius: f32, fov: f32) -> Vec3 {
    eye_for(GLOBE.0, centre, radius, GLOBE.1, fov)
}

/// A level camera (looking -Z) that shows a thing centred at `centre` at frame fraction
/// `at` (16:9 reference) with `size` (display units) spanning `size_h` of the frame height.
pub fn eye_for(at: [f32; 2], centre: Vec3, size: f32, size_h: f32, fov: f32) -> Vec3 {
    let t = (fov / 2.).tan();
    let distance = size / (2. * size_h * t);
    // Offset from the frame centre in heights (x scaled from width to height).
    let dx = (at[0] - 0.5) * ASPECT;
    let dy = 0.5 - at[1];
    Vec3::new(
        centre.x - dx * 2. * distance * t,
        centre.y - dy * 2. * distance * t,
        centre.z + distance,
    )
}

/// P2b closed-Puter (default) view of the globe: centre 0.47/0.48 (RPCS3, about 0.02).
/// The radius is INFERRED: a third of the open view's, from the globe-moon spacing (0.09
/// of the width closed, 0.27 open).
pub const GLOBE_CLOSED: ([f32; 2], f32) = ([0.47, 0.48], 0.075);
/// The DualShock in the shell view: centre and width as a fraction of the 16:9 width,
/// Puter closed and open (RPCS3 previews, about 0.02).
pub const PAD_CLOSED: ([f32; 2], f32) = ([0.50, 0.70], 0.15);
pub const PAD_OPEN: ([f32; 2], f32) = ([0.50, 0.86], 0.17);
/// Opening the Puter takes about a second (RPCS3).
pub const DOLLY_SECONDS: f32 = 1.0;
/// The game's frame aspect (RPCS3 16:9), for expected positions.
pub const ASPECT_16_9: f32 = ASPECT;

/// P2k: one PlanetsView (PlanetsWorld script): pitch, yaw (radians) and offset (level
/// units). CONFIRMED_FROM_LOCAL_DATA: view 0 from the D180/D174 VM trace, view 1 (main
/// menu) matches the RPCS3 World matrix read (docs/p2k-pod-game-camera.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetsView {
    pub pitch: f32,
    pub yaw: f32,
    pub offset: Vec3,
}
/// Puter not in use (view 0) and the main menu (view 1).
pub const VIEW_CLOSED: PlanetsView = PlanetsView {
    pitch: 0.15,
    yaw: -0.07,
    offset: Vec3::new(0., 0., -15000.),
};
pub const VIEW_MENU: PlanetsView = PlanetsView {
    pitch: 0.35,
    yaw: -0.07,
    offset: Vec3::new(-700., 400., -5000.),
};
/// The game's camera presets (`from`, `to`; level units, up +Y), not in use then in use
/// (D169 script trace; RPCS3 view rotation confirms the in-use pitch).
pub const CAMERA_PRESETS: [[Vec3; 2]; 2] = [
    [Vec3::new(0., -250., 1130.), Vec3::new(0., 100., -1500.)],
    [Vec3::new(0., -300., 900.), Vec3::new(0., 500., -1500.)],
];
/// Vertical field of view from RPCS3's projection block (16:9, horizontal scale 1.0296):
/// 2 atan(1 / 1.8305) = 57.3 degrees. CONFIRMED_FROM_LOCAL_DATA (in-use menu only).
pub const GAME_FOV: f32 = 0.999_994;
/// Earth and Moon placement in the main menu, in World (Thing 0) space: translation and
/// uniform scale (RPCS3 world matrices / World; their stored POS differ).
pub const EARTH_MENU: (Vec3, f32) = (Vec3::new(-1250., 0., 0.), 1.3505);
pub const MOON_MENU: (Vec3, f32) = (Vec3::new(2000., 0., 0.), 1.5444);
/// pod_planets Thing 4, the Moon (Thing 1 is the Earth, `--pod-camera 1`).
pub const MOON_THING: usize = 4;

/// P2k: World (pod_planets Thing 0) matrix for view `v` with the camera at `eye` (level
/// units, column vectors): scale 2 about the eye, then pitch, offset and yaw:
/// `T(eye) S(2) T(-eye) Rx(pitch) T(offset) Ry(yaw)`.
pub fn planets_world(v: PlanetsView, eye: Vec3) -> Mat4 {
    Mat4::from_translation(eye)
        * Mat4::from_scale(Vec3::splat(2.))
        * Mat4::from_translation(-eye)
        * Mat4::from_rotation_x(v.pitch)
        * Mat4::from_translation(v.offset)
        * Mat4::from_rotation_y(v.yaw)
}

/// P2k: the view and camera eye at opening progress `t` (eased like the dolly; the game
/// blends the view and the camera separately, so mid-dolly frames are approximate).
pub fn planets_world_at(t: f32) -> Mat4 {
    let e = PodRig::ease_pub(t);
    let lerp = |a: f32, b: f32| a + (b - a) * e;
    let v = PlanetsView {
        pitch: lerp(VIEW_CLOSED.pitch, VIEW_MENU.pitch),
        yaw: lerp(VIEW_CLOSED.yaw, VIEW_MENU.yaw),
        offset: VIEW_CLOSED.offset.lerp(VIEW_MENU.offset, e),
    };
    planets_world(v, CAMERA_PRESETS[0][0].lerp(CAMERA_PRESETS[1][0], e))
}
/// Sackboy standing in the closed view: head top and feet as fractions of the 16:9
/// height, centred at 0.50 of the width (RPCS3, about 0.02).
pub const SACKBOY_CLOSED: [f32; 2] = [0.56, 0.82];
/// P2g: the closed tent (pod.mol) draws its window 0.21-0.64 h where RPCS3 has
/// 0.28-0.69 h (PC run of D165), about 0.06 h too high, while the DualShock is within
/// 0.01. The tent camera is tilted up by this much of the frame height (a correction
/// fitted to that measurement; a tilted game camera is INFERRED as the cause).
/// P2h: with the tilt (D168) the window measured 0.32-0.68 w x 0.28-0.72 h, about 0.03
/// too large on each side; `TENT_PULL` shrinks it about the frame centre first, so the
/// tilt drops to the remaining 0.045 h.
pub const TENT_TILT_CLOSED: f32 = 0.045;
/// P2h: closed window size, RPCS3 over ours at D168 (0.32 x 0.41 over 0.36 x 0.44, about
/// 0.91). The closed tent eye backs off along its view so the far wall (the window)
/// projects this much smaller (correction fitted to that measurement).
pub const TENT_PULL: f32 = 0.91;
/// Sackboy holding the controller, seen from behind, Puter open: centre (RPCS3, 0.02).
pub const SACKBOY_OPEN: [f32; 2] = [0.50, 0.90];

/// Two layers as the game appears to draw them: the Pod shell (tent, DualShock) over the
/// planets view through its window, each seen by its own camera. The viewer has one
/// camera, so the shell is placed each frame by `X = P + scale (X_shell - S)` (P, S the
/// planets and shell eyes): scaling about the eye keeps its projection, and `scale` puts
/// the whole shell in front of the globe. Eyes are display-space, closed then open.
/// P2f: a shell eye may also turn (`shell_rot`, identity = looking -Z), for the game's own
/// camera (`with_game_camera`); the root then also applies the inverse turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PodRig {
    pub planet: [Vec3; 2],
    pub globe_z: f32,
    pub shell: [Vec3; 2],
    pub shell_rot: [Quat; 2],
    /// P2g: the tent (pod.mol) has its own eye: closed as the shell's but tilted by
    /// `TENT_TILT_CLOSED`; open past the shell's far side, so the tent leaves the frame
    /// during the dolly (RPCS3: no tent once open).
    pub tent: [Vec3; 2],
    pub tent_rot: [Quat; 2],
    pub scale: f32,
    pub fov: f32,
    /// P2k: the planets camera turn, closed then open (identity = looking -Z; the fitted
    /// eyes). `with_planets_camera` sets it from the game's presets.
    pub planet_rot: [Quat; 2],
    /// P2k: distance along the view from the closed planets eye to the globe (orbit target).
    pub focus: f32,
}

impl PodRig {
    /// `globe`: the globe's display centre and radius. `pad`: the DualShock's display
    /// centre and width in the shell's display space; `shell_near`: the shell's smallest
    /// z (its far side from the camera).
    pub fn new(globe: (Vec3, f32), pad: (Vec3, f32), shell_near: f32, fov: f32) -> Self {
        let (g, r) = globe;
        let planet = [
            eye_for(GLOBE_CLOSED.0, g, r, GLOBE_CLOSED.1, fov),
            pod_camera(g, r, fov),
        ];
        let (c, w) = pad;
        let shell = [
            eye_for(PAD_CLOSED.0, c, w, PAD_CLOSED.1 * ASPECT, fov),
            eye_for(PAD_OPEN.0, c, w, PAD_OPEN.1 * ASPECT, fov),
        ];
        // The shell's far side at half the closed eye's distance to the globe.
        let depth = (shell[0].z - shell_near).max(1e-6);
        let scale = 0.5 * (planet[0].z - g.z) / depth;
        let tilt = (TENT_TILT_CLOSED * 2. * (fov / 2.).tan()).atan();
        Self {
            planet,
            globe_z: g.z,
            shell,
            shell_rot: [Quat::IDENTITY; 2],
            tent: [
                shell[0] + Vec3::Z * (depth * (1. / TENT_PULL - 1.)),
                Vec3::new(shell[1].x, shell[1].y, shell_near - depth * 0.05),
            ],
            tent_rot: [Quat::from_rotation_x(tilt), Quat::IDENTITY],
            scale,
            fov,
            planet_rot: [Quat::IDENTITY; 2],
            focus: planet[0].z - g.z,
        }
    }

    /// P2k: the planets eyes from the game's camera presets (`from`/`to` pairs, closed then
    /// open, display space, up +Y) instead of the globe fit. `globe` is where the globe
    /// shows when closed (display, after the World transform); `scale` is reset so the
    /// shell's far side stays at half the closed view distance to it.
    pub fn with_planets_camera(mut self, presets: [[Vec3; 2]; 2], globe: Vec3) -> Self {
        for (i, [from, to]) in presets.into_iter().enumerate() {
            self.planet[i] = from;
            self.planet_rot[i] = Transform::from_translation(from)
                .looking_at(to, Vec3::Y)
                .rotation;
        }
        let forward = self.planet_rot[0] * Vec3::NEG_Z;
        let old = self.focus;
        self.focus = (globe - self.planet[0]).dot(forward).max(1e-6);
        self.scale *= self.focus / old;
        self
    }

    /// P2k: the planets camera turn at `t`.
    pub fn planet_turn(&self, t: f32) -> Quat {
        self.planet_rot[0].slerp(self.planet_rot[1], Self::ease(t))
    }

    /// P2f: the closed shell eye from the game's camera preset (`from` looking at `to`, up
    /// +Y; shell display space). `bounds` (the shell's display bounds) reset `scale` so
    /// the shell's far side stays at half the closed planets distance.
    pub fn with_game_camera(mut self, from: Vec3, to: Vec3, [lo, hi]: [Vec3; 2]) -> Self {
        let rot = Transform::from_translation(from)
            .looking_at(to, Vec3::Y)
            .rotation;
        let forward = rot * Vec3::NEG_Z;
        let depth = (0..8)
            .map(|c| {
                let p = Vec3::new(
                    if c & 1 == 0 { lo.x } else { hi.x },
                    if c & 2 == 0 { lo.y } else { hi.y },
                    if c & 4 == 0 { lo.z } else { hi.z },
                );
                (p - from).dot(forward)
            })
            .fold(1e-6, f32::max);
        self.shell[0] = from;
        self.shell_rot[0] = rot;
        // The preset replaces the fitted tilt too.
        self.tent[0] = from;
        self.tent_rot[0] = rot;
        self.scale = 0.5 * self.focus / depth;
        self
    }

    pub fn ease_pub(t: f32) -> f32 {
        Self::ease(t)
    }

    fn ease(t: f32) -> f32 {
        let t = t.clamp(0., 1.);
        t * t * (3. - 2. * t)
    }

    fn at(pair: [Vec3; 2], t: f32) -> Vec3 {
        pair[0].lerp(pair[1], Self::ease(t))
    }

    fn turn(&self, t: f32) -> Quat {
        self.shell_rot[0].slerp(self.shell_rot[1], Self::ease(t))
    }

    /// `X -> P + scale R^-1 (X - S)` for a layer whose eye is `s` turned by `rot`.
    /// P2k: with a turned planets camera `Rp`, `X -> P + scale Rp R^-1 (X - S)`.
    fn layer(&self, t: f32, s: Vec3, rot: Quat) -> Transform {
        self.layer_at(self.planet_eye(t), self.planet_turn(t), s, rot)
    }

    /// D275: [`Self::layer`] for a planets camera at `eye` turned by `turn` that is not
    /// the rig's own (the camera the planets level's script writes): a layer point seen
    /// from `s` turned by `rot` shows at the same place of the frame from `eye`.
    pub fn layer_at(&self, eye: Vec3, turn: Quat, s: Vec3, rot: Quat) -> Transform {
        let turn = turn * rot.inverse();
        Transform {
            translation: eye - turn * (s * self.scale),
            rotation: turn,
            scale: Vec3::splat(self.scale),
        }
    }

    /// P2g: the tent's eye and turn at `t`.
    fn tent_pose(&self, t: f32) -> (Vec3, Quat) {
        let rot = self.tent_rot[0].slerp(self.tent_rot[1], Self::ease(t));
        (Self::at(self.tent, t), rot)
    }

    /// P2g: the tent root transform at `t`.
    pub fn tent_root(&self, t: f32) -> Transform {
        let (s, rot) = self.tent_pose(t);
        self.layer(t, s, rot)
    }

    /// D275: the tent root at `t` under a planets camera at `eye` turned by `turn`.
    pub fn tent_root_at(&self, eye: Vec3, turn: Quat, t: f32) -> Transform {
        let (s, rot) = self.tent_pose(t);
        self.layer_at(eye, turn, s, rot)
    }

    /// D275: the shell root at `t` under a planets camera at `eye` turned by `turn`.
    pub fn shell_root_at(&self, eye: Vec3, turn: Quat, t: f32) -> Transform {
        self.layer_at(eye, turn, Self::at(self.shell, t), self.turn(t))
    }

    /// The planets eye at opening progress `t` (0 closed, 1 open, eased).
    pub fn planet_eye(&self, t: f32) -> Vec3 {
        Self::at(self.planet, t)
    }

    /// Where a shell display-space point appears at `t`, as 16:9 frame fractions.
    pub fn shell_frac(&self, t: f32, q: Vec3) -> [f32; 2] {
        let local = self.turn(t).inverse() * (q - Self::at(self.shell, t));
        let c = Mat4::perspective_rh(self.fov, ASPECT, 0.001, 1000.).project_point3(local);
        [(c.x + 1.) / 2., (1. - c.y) / 2.]
    }

    /// Feet position (shell display space) for an upright figure `h` display units tall
    /// that the closed shell eye sees centred in x with head top and feet at
    /// `SACKBOY_CLOSED`. Solves depth from the projected height, then height from the feet.
    pub fn fit_figure(&self, h: f32) -> Vec3 {
        let tan = (self.fov / 2.).tan();
        let depth = self.figure_depth(h);
        let local = Vec3::new(0., (0.5 - SACKBOY_CLOSED[1]) * 2. * tan * depth, -depth);
        self.shell[0] + self.shell_rot[0] * local
    }

    /// Distance in front of the closed eye at which `fit_figure` stands the figure.
    pub fn figure_depth(&self, h: f32) -> f32 {
        let [top, feet] = SACKBOY_CLOSED;
        h / ((feet - top) * 2. * (self.fov / 2.).tan())
    }

    /// P2f: feet for the figure holding the controller: as far in front of the open eye as
    /// `figure_depth`, its centre at `SACKBOY_OPEN` (the hold spot is not measured, only
    /// where it shows; same distance is INFERRED).
    pub fn fit_figure_open(&self, h: f32) -> Vec3 {
        let tan = (self.fov / 2.).tan();
        let depth = self.figure_depth(h);
        let [x, y] = SACKBOY_OPEN;
        let local = Vec3::new(
            (x - 0.5) * 2. * tan * ASPECT * depth,
            (0.5 - y) * 2. * tan * depth,
            -depth,
        );
        self.shell[1] + self.shell_rot[1] * local - Vec3::Y * (h / 2.)
    }

    /// The shell root transform at `t`: `X -> P + scale R^-1 (X - S)`.
    pub fn shell_root(&self, t: f32) -> Transform {
        self.layer(t, Self::at(self.shell, t), self.turn(t))
    }
}

/// Opening progress of the Puter: 0 closed (tent view), 1 open (menu on the globe).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct PuterState {
    pub t: f32,
    pub open: bool,
    /// P2h `--pod-t`: stay at `t` until the next open/close key (mid-dolly captures).
    pub hold: bool,
    /// D279: under `--level-scripts` the keys do not set `open`. The key that went down
    /// last waits here until the scripts' next 30 Hz step takes it
    /// (`level_play::tick`), and `open` follows the script world's "pod computer in use".
    pub key: Option<PuterKey>,
}

/// D279: the two key groups of the Pod shell layer, as `pod_dolly` reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuterKey {
    /// Space or gamepad Square: opens a closed Puter, closes an open one.
    Toggle,
    /// Esc, Backspace or gamepad Circle: closes.
    Back,
}

/// The key of a frame in which `toggle` and `back` went down: closing wins, as the
/// keys' order without scripts has it.
pub fn puter_key(toggle: bool, back: bool) -> Option<PuterKey> {
    match (toggle, back) {
        (_, true) => Some(PuterKey::Back),
        (true, false) => Some(PuterKey::Toggle),
        (false, false) => None,
    }
}

/// Maps a 16:9 reference frame fraction into a window of `aspect`, keeping offsets from
/// the centre in heights (a 16:10 window shows a little more above and below).
pub fn reference_to_window(f: [f32; 2], aspect: f32) -> [f32; 2] {
    [0.5 + (f[0] - 0.5) * ASPECT / aspect, f[1]]
}

/// `--pod-menu CLASS` with a level (`--level-guid`/`--level-path`): the pills over the
/// level view in the main viewer instead of the standalone placeholder scene. Pair with
/// `--pod-camera N` (the globe's Thing) so the globe sits where the pills expect it.
pub fn install_overlay(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        all.iter()
            .position(|a| a == flag)
            .and_then(|i| all.get(i + 1))
            .cloned()
    };
    let Some(class) = value("--pod-menu") else {
        return Ok(());
    };
    let snapshot = value("--snapshot").ok_or("--pod-menu requires --snapshot")?;
    let game = value("--game-dir").ok_or("--pod-menu requires --game-dir")?;
    let function = value("--ui-script-function").unwrap_or_else(|| DEFAULT_FUNCTION.into());
    let (report, _) = run_script(snapshot.as_ref(), game.as_ref(), &class, &function)?;
    let menu = layout(&report.tree);
    println!("Pod menu overlay: {} pills", menu.pills.len());
    let aspect = if all.iter().any(|a| a == "--window-16-9") {
        ASPECT
    } else {
        1280. / 800.
    };
    app.insert_resource(Menu { menu, focused: 0 })
        .add_systems(Startup, move |mut commands: Commands, menu: Res<Menu>| {
            spawn_overlay(
                &mut commands,
                &menu.menu,
                move |f| reference_to_window(f, aspect),
                None,
            )
        })
        .add_systems(Update, (keys, paint).chain());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p2k_planets_world_matches_the_game_and_frames_the_globe_and_moon() {
        // RPCS3 in-use main menu World matrix (translation at 12..14).
        let game = Mat4::from_cols_array(&[
            1.995, -0.04797, 0.1314, 0., 0., 1.879, 0.6858, 0., -0.1399, -0.6841, 1.874, 0.,
            -1400., 4480., -10020., 1.,
        ]);
        let ours = planets_world(VIEW_MENU, CAMERA_PRESETS[1][0]);
        assert!(ours.abs_diff_eq(game, 1.5), "{ours:?}");
        assert_eq!(planets_world_at(1.), ours);
        // Through the game camera: Earth and Moon centres where RPCS3 shows them (open,
        // 16:9 frame fractions) and the Earth where the closed reference has it.
        let frac = |t: f32, local: Vec3| {
            let [from, to] = CAMERA_PRESETS[usize::from(t > 0.5)];
            let view = Mat4::look_at_rh(from, to, Vec3::Y);
            let c = Mat4::perspective_rh(GAME_FOV, ASPECT, 10., 100000.)
                .project_point3(view.transform_point3(planets_world_at(t).transform_point3(local)));
            [(c.x + 1.) / 2., (1. - c.y) / 2.]
        };
        let near =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 0.03 && (a[1] - b[1]).abs() < 0.03;
        assert!(
            near(frac(1., EARTH_MENU.0), [0.35, 0.40]),
            "{:?}",
            frac(1., EARTH_MENU.0)
        );
        assert!(
            near(frac(1., MOON_MENU.0), [0.605, 0.417]),
            "{:?}",
            frac(1., MOON_MENU.0)
        );
        assert!(
            near(frac(0., EARTH_MENU.0), GLOBE_CLOSED.0),
            "{:?}",
            frac(0., EARTH_MENU.0)
        );
    }

    #[test]
    fn p2k_turned_planets_camera_keeps_the_shell_projection() {
        let fov = GAME_FOV;
        let globe = (Vec3::new(-0.3, 0.2, 0.0), 0.25);
        let pad = (Vec3::new(0.01, -0.2, -0.1), 0.12);
        let presets = [
            [Vec3::new(0., -0.1, 1.), Vec3::new(0., 0.2, -1.)],
            [Vec3::new(0., -0.12, 0.8), Vec3::new(0., 0.5, -1.)],
        ];
        let plain = PodRig::new(globe, pad, -0.4, fov);
        let rig = plain.with_planets_camera(presets, globe.0);
        let projection = Mat4::perspective_rh(fov, ASPECT, 0.001, 1000.);
        for t in [0., 0.5, 1.] {
            let q = Vec3::new(0.05, -0.15, -0.2);
            // Seen from the planets camera, the placed shell point projects where the
            // shell eye sees it.
            let placed = rig.shell_root(t).transform_point(q);
            let local = rig.planet_turn(t).inverse() * (placed - rig.planet_eye(t));
            let c = projection.project_point3(local);
            let got = [(c.x + 1.) / 2., (1. - c.y) / 2.];
            let want = rig.shell_frac(t, q);
            assert!((got[0] - want[0]).abs() < 1e-4 && (got[1] - want[1]).abs() < 1e-4);
        }
        // Half the closed view distance to the globe, as before.
        let far = rig.shell_root(0.).transform_point(Vec3::new(0., 0., -0.4));
        let forward = rig.planet_rot[0] * Vec3::NEG_Z;
        assert!(((far - rig.planet[0]).dot(forward) / rig.focus - 0.5).abs() < 0.05);
    }

    #[test]
    fn d275_layer_at_keeps_the_shell_projection_under_any_planets_camera() {
        let fov = GAME_FOV;
        let globe = (Vec3::new(-0.3, 0.2, 0.0), 0.25);
        let pad = (Vec3::new(0.01, -0.2, -0.1), 0.12);
        let presets = [
            [Vec3::new(0., -0.1, 1.), Vec3::new(0., 0.2, -1.)],
            [Vec3::new(0., -0.12, 0.8), Vec3::new(0., 0.5, -1.)],
        ];
        let bounds = [Vec3::new(-0.5, -0.5, -0.4), Vec3::new(0.5, 0.5, 0.2)];
        let rig = PodRig::new(globe, pad, -0.4, fov)
            .with_planets_camera(presets, globe.0)
            .with_game_camera(Vec3::new(0., -0.1, 0.45), Vec3::new(0., 0.03, -0.6), bounds);
        // At the rig's own planets poses the two rules are one, bit for bit.
        for t in [0., 0.3, 0.5, 1.] {
            let (eye, turn) = (rig.planet_eye(t), rig.planet_turn(t));
            assert_eq!(rig.shell_root_at(eye, turn, t), rig.shell_root(t));
            assert_eq!(rig.tent_root_at(eye, turn, t), rig.tent_root(t));
        }
        // Planets cameras that are not the rig's: far out and looking steeply up (as the
        // intro's outer camera), turned and rolled behind the scene, and far back.
        let cameras = [
            (Vec3::new(-0.4, -1.7, 1.8), Quat::from_rotation_x(0.8)),
            (
                Vec3::new(3., 2., -5.),
                Quat::from_euler(EulerRot::YXZ, 2.4, -0.3, 0.5),
            ),
            (Vec3::new(0., 0., 40.), Quat::IDENTITY),
        ];
        let projection = Mat4::perspective_rh(fov, ASPECT, 0.001, 1000.);
        let frac = |eye: Vec3, turn: Quat, placed: Vec3| {
            let local = turn.inverse() * (placed - eye);
            let c = projection.project_point3(local);
            ([(c.x + 1.) / 2., (1. - c.y) / 2.], -local.z)
        };
        let near =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4;
        for (eye, turn) in cameras {
            for t in [0., 0.5, 1.] {
                for q in [Vec3::new(0.05, -0.15, -0.2), Vec3::new(-0.3, 0.2, -0.4)] {
                    let placed = rig.shell_root_at(eye, turn, t).transform_point(q);
                    let (got, depth) = frac(eye, turn, placed);
                    let want = rig.shell_frac(t, q);
                    assert!(near(got, want), "{eye:?} {t} {got:?} {want:?}");
                    // In front of the camera by the shell eye's own depth times the scale.
                    let own = -(rig.turn(t).inverse() * (q - PodRig::at(rig.shell, t))).z;
                    assert!(own > 0. && (depth - own * rig.scale).abs() < 1e-3 * depth.max(1.));
                }
            }
            // The tent shows where it shows under the rig's own closed camera.
            let wall = Vec3::new(0.1, 0.1, -0.4);
            let own = rig.tent_root(0.).transform_point(wall);
            let (want, _) = frac(rig.planet_eye(0.), rig.planet_turn(0.), own);
            let placed = rig.tent_root_at(eye, turn, 0.).transform_point(wall);
            assert!(near(frac(eye, turn, placed).0, want));
        }
    }

    #[test]
    fn p2b_rig_keeps_each_layer_where_its_reference_puts_it() {
        let fov = std::f32::consts::FRAC_PI_4;
        let globe = (Vec3::new(-0.3, 0.2, 0.0), 0.25);
        let pad = (Vec3::new(0.01, -0.2, -0.1), 0.12);
        let rig = PodRig::new(globe, pad, -0.4, fov);
        let projection = Mat4::perspective_rh(fov, ASPECT, 0.001, 1000.);
        let frac = |eye: Vec3, p: Vec3| {
            let c = projection.project_point3(p - eye);
            [(c.x + 1.) / 2., (1. - c.y) / 2.]
        };
        for (t, globe_at, pad_at) in [
            (0., GLOBE_CLOSED.0, PAD_CLOSED.0),
            (1., GLOBE.0, PAD_OPEN.0),
        ] {
            let eye = rig.planet_eye(t);
            let g = frac(eye, globe.0);
            assert!((g[0] - globe_at[0]).abs() < 1e-3 && (g[1] - globe_at[1]).abs() < 1e-3);
            // The shell, placed by its root transform, projects as from its own eye.
            let placed = rig.shell_root(t).transform_point(pad.0);
            let p = frac(eye, placed);
            assert!(
                (p[0] - pad_at[0]).abs() < 1e-3 && (p[1] - pad_at[1]).abs() < 1e-3,
                "{t} {p:?}"
            );
            // ... and in front of the globe.
            assert!(placed.z > globe.0.z);
        }
        let closed = rig.shell_frac(0., pad.0);
        assert!((closed[0] - PAD_CLOSED.0[0]).abs() < 1e-3);
        assert!((closed[1] - PAD_CLOSED.0[1]).abs() < 1e-3);
        let feet = rig.fit_figure(0.05);
        let [x, low] = rig.shell_frac(0., feet);
        let top = rig.shell_frac(0., feet + Vec3::Y * 0.05)[1];
        assert!((x - 0.5).abs() < 1e-4 && (low - SACKBOY_CLOSED[1]).abs() < 1e-4);
        assert!((top - SACKBOY_CLOSED[0]).abs() < 1e-4, "{top}");
        // P2g/P2h: on the far wall the tent shows TENT_PULL smaller about the centre and
        // TENT_TILT_CLOSED lower than the shell closed, and is wholly behind the eye open.
        let p = rig.planet_eye(0.);
        let frac = |q: Vec3| {
            let c = projection.project_point3(q - p);
            [(c.x + 1.) / 2., (1. - c.y) / 2.]
        };
        let wall = Vec3::new(rig.shell[0].x + 0.05, rig.shell[0].y + 0.04, -0.4);
        let shell_at = frac(rig.shell_root(0.).transform_point(wall));
        let tent_at = frac(rig.tent_root(0.).transform_point(wall));
        let want = 0.5 + (shell_at[1] - 0.5) * TENT_PULL + TENT_TILT_CLOSED;
        assert!((tent_at[1] - want).abs() < 0.01, "{tent_at:?} {want}");
        let want_x = 0.5 + (shell_at[0] - 0.5) * TENT_PULL;
        assert!((tent_at[0] - want_x).abs() < 0.01, "{tent_at:?} {want_x}");
        let open_eye = rig.planet_eye(1.);
        for z in [-0.4, 0.0, 0.3] {
            let q = rig.tent_root(1.).transform_point(Vec3::new(0.2, 0.1, z));
            assert!(q.z > open_eye.z, "{z} {q:?} {open_eye:?}");
        }
        let far = rig.shell_root(0.).transform_point(Vec3::new(0., 0., -0.4));
        assert!(far.z > globe.0.z);
    }

    #[test]
    fn p2f_game_camera_turns_the_shell_and_keeps_projection() {
        let fov = std::f32::consts::FRAC_PI_4;
        let globe = (Vec3::new(-0.3, 0.2, 0.0), 0.25);
        let pad = (Vec3::new(0.01, -0.2, -0.1), 0.12);
        let bounds = [Vec3::new(-0.5, -0.5, -0.4), Vec3::new(0.5, 0.5, 0.2)];
        let (from, to) = (Vec3::new(0., -0.1, 0.45), Vec3::new(0., 0.03, -0.6));
        let rig = PodRig::new(globe, pad, -0.4, fov).with_game_camera(from, to, bounds);
        let projection = Mat4::perspective_rh(fov, ASPECT, 0.001, 1000.);
        let q = Vec3::new(0.05, -0.15, -0.05);
        // The game camera's own view of q ...
        let view = Mat4::look_at_rh(from, to, Vec3::Y);
        let c = projection.project_point3(view.transform_point3(q));
        let want = [(c.x + 1.) / 2., (1. - c.y) / 2.];
        let at = rig.shell_frac(0., q);
        assert!((at[0] - want[0]).abs() < 1e-4 && (at[1] - want[1]).abs() < 1e-4);
        // ... is where the placed shell shows from the planets eye.
        let placed = rig.shell_root(0.).transform_point(q);
        let c = projection.project_point3(placed - rig.planet_eye(0.));
        let got = [(c.x + 1.) / 2., (1. - c.y) / 2.];
        assert!((got[0] - want[0]).abs() < 1e-4 && (got[1] - want[1]).abs() < 1e-4);
        assert!(rig.shell_root(0.).transform_point(bounds[0]).z > globe.0.z);
        // The open hold spot projects its centre to SACKBOY_OPEN.
        let h = 0.05;
        let centre = rig.fit_figure_open(h) + Vec3::Y * (h / 2.);
        let f = rig.shell_frac(1., centre);
        assert!((f[0] - SACKBOY_OPEN[0]).abs() < 1e-4 && (f[1] - SACKBOY_OPEN[1]).abs() < 1e-4);
    }

    #[test]
    fn p2_pod_camera_projects_the_globe_to_its_reference_place() {
        let fov = std::f32::consts::FRAC_PI_4;
        let (centre, radius) = (Vec3::new(-0.3, 0.2, 0.1), 0.25);
        let eye = pod_camera(centre, radius, fov);
        let view = Mat4::from_translation(-eye);
        let projection = Mat4::perspective_rh(fov, ASPECT, 0.01, 100.);
        let clip = projection.project_point3(view.transform_point3(centre));
        let at = [(clip.x + 1.) / 2., (1. - clip.y) / 2.];
        assert!((at[0] - GLOBE.0[0]).abs() < 1e-4 && (at[1] - GLOBE.0[1]).abs() < 1e-4);
        let edge = projection.project_point3(view.transform_point3(centre + Vec3::Y * radius));
        assert!((((edge.y - clip.y) / 2.) - GLOBE.1).abs() < 1e-4);
        assert_eq!(reference_to_window([0.5, 0.3], 1.6), [0.5, 0.3]);
        // A narrower window keeps offsets in heights, so they grow as width fractions.
        assert!(reference_to_window([0.35, 0.4], 1.6)[0] < 0.35);
    }

    #[test]
    fn p1b_frame_points_project_back_to_their_fractions() {
        let projection = Mat4::perspective_rh(FOV, ASPECT, 0.1, 100.);
        for f in [GLOBE.0, MOON.0, [0.5, 0.5], [0.1, 0.9]] {
            let clip = projection.project_point3(frame_to_world(f));
            let back = [(clip.x + 1.) / 2., (1. - clip.y) / 2.];
            assert!((back[0] - f[0]).abs() < 1e-4 && (back[1] - f[1]).abs() < 1e-4);
        }
        // A radius of the full height spans the visible height at DEPTH.
        let top = projection.project_point3(Vec3::new(0., height_to_world(0.5), -DEPTH));
        assert!((top.y - 1.).abs() < 1e-4);
    }
}
