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
use crate::game_font::GameFont;
use crate::gooey_ui::{demo_capture, demo_capture_path, DemoCapture};
use crate::menu_layer::{CoronaPrim, MenuCanvas, MenuImage, MenuPrim, MenuShape, SHADOW_WEIGHTS};
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
                title: "craftworld-rs - Pod menu".into(),
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
    /// D280: only without `--level-scripts`. Under the scripts the shell level stands in
    /// its own level frame (`level_play::shell_frame_root`) and no shell eye is used.
    fn layer(&self, t: f32, s: Vec3, rot: Quat) -> Transform {
        let eye = self.planet_eye(t);
        let turn = self.planet_turn(t) * rot.inverse();
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

/// D319 (docs/level-scripts.md part 70): the main menu the planets level's scripts built,
/// as `level_play` hands it over after each tick: the items laid out in the root frame's
/// units ([`lbp_runtime::layout`]), the matrix the pod computer's draw call gave the root
/// frame (root frame units to the planets level's world, level units), the fade of the
/// frame's transition and the colour the Gooey took from the player.
#[derive(Resource, Default, Clone)]
pub struct ScriptMenu(pub Option<ScriptMenuFrame>);

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptMenuFrame {
    pub placed: Vec<lbp_runtime::PlacedItem>,
    pub matrix: Mat4,
    /// 0 when the frame was first built, 1 once its transition time has passed.
    pub fade: f32,
    /// The player's primary colour (`Gooey.SetColoursFromPlayer`), RGBA 0..1.
    pub colour: [f32; 4],
    /// D323: frames since the frame was first built.
    pub age: f32,
    /// D325: frames since the focus last moved to another button, or [`Self::age`] when it
    /// has not moved since the frame was first built: the focused pill's spring and slab go
    /// by it. INFERRED (not captured): a newly focused pill springs from 1 as the first does.
    pub focus_age: f32,
}

impl ScriptMenuFrame {
    /// The drawn menu of a level world, if its scripts have drawn one; text measured in
    /// the game's fonts where they are loaded (part 72).
    pub fn of(
        world: &lbp_runtime::LevelWorld,
        fonts: Option<&lbp_runtime::GooeyFonts>,
    ) -> Option<Self> {
        let menu = world.gooey.drawn_menu()?;
        // D324 FOUND (RenderDoc 7295 / 7927 / 8348, ticks 606 / 609 / 611, PS 3483): the
        // frame comes in over its transition time (`SetFrameTransitionTime(15, 15)`) as one
        // layer at alpha 101 / 152 / 186 of 255, (age + 1) / 15 at ages 5 / 8 / 10 within a
        // level (the frame's first draw counts one); 1 from age 14 on.
        let time = menu.frame.transition.map_or(0., |t| t[0]);
        let age = menu
            .first_built
            .map_or(f32::MAX, |b| (world.pworld.frame - b) as f32);
        let fade = if time > 0. {
            ((age + 1.) / time).clamp(0., 1.)
        } else {
            1.
        };
        let player = world.gooey.colours_player;
        let colour = world
            .player_records
            .iter()
            .find(|r| Some(r.number) == player)
            .or(world.player_records.first())
            .map_or(lbp_script_vm::natives::DEFAULT_PLAYER_COLOUR, |r| r.colour);
        let age = if age == f32::MAX { 0. } else { age };
        let focus_age = world
            .gooey
            .focus_since
            .map_or(age, |s| (world.pworld.frame - s) as f32)
            .clamp(0., age);
        Some(Self {
            placed: lbp_runtime::layout_with(menu.frame, menu.focus, fonts),
            matrix: Mat4::from_cols_array(&menu.matrix),
            fade,
            colour: lbp_script_vm::natives::colour_vector(colour),
            age,
            focus_age,
        })
    }
}

/// D323 (part 74): the planets' coronas: FOUND (RenderDoc 6082 to 9601, ticks 600 to 619,
/// closed and open Pod, with and without the menu) two additive draws after the copy back,
/// one square each for the Earth and the Moon, facing the eye (its normal points at the
/// eye to 1e-5), [`CORONA_HALF`] x 2 corona units a side, [`CORONA_PULL`] units behind the
/// planet's place, colour [`CORONA_COLOUR`] times the texture, depth tested. A corona unit
/// is a size of the planet's own: MEASURED in the game's world (level units under the world
/// Thing), the same in every capture.
#[derive(Resource, Default, Clone, PartialEq)]
pub struct PodCoronas(pub Vec<Corona>);

/// One planet's corona: the Thing, its place in the level's world (level units, under the
/// world Thing) and its corona unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corona {
    pub thing: u32,
    pub place: Vec3,
    pub size: f32,
}

/// D323 FOUND: the coronas' vertex colour (0.5, 0.7, 1, 0.5); the square's half side and
/// its pull behind the planet, in corona units (the quad's corners +-1.6, z -0.5).
pub const CORONA_COLOUR: [f32; 4] = [0.5, 0.7, 1., 0.5];
pub const CORONA_HALF: f32 = 1.6;
pub const CORONA_PULL: f32 = 0.5;
/// D323 MEASURED (the square's matrix against the Earth draw's, six captures, within
/// 0.1 %): the corona unit by the planet's script class, in the world's units. The Earth's
/// sphere fits r = 3239 there (part 67), so 1.12 r; what the engine derives it from was
/// not read.
pub const CORONA_SIZES: [(&str, f32); 2] = [("Earth", 3632.6), ("OrbitingMoon", 1733.6)];

impl PodCoronas {
    /// The planets of a level world that have a corona, at their places now: each Thing's
    /// world matrix, moved with the world Thing (the level hangs under it, part 23).
    pub fn of(world: &lbp_runtime::LevelWorld) -> Self {
        let moved = world
            .world_thing_moved()
            .map_or(Mat4::IDENTITY, |m| Mat4::from_cols_array(&m));
        let mut out = Vec::new();
        for (thing, state) in world.things.iter().enumerate() {
            // A planet of the Pod's planets level: a `PlanetThing` of a class in the table.
            if !state.script_classes.iter().any(|c| c == "PlanetThing") {
                continue;
            }
            let Some(size) = CORONA_SIZES.iter().find_map(|(class, size)| {
                state
                    .script_classes
                    .iter()
                    .any(|c| c == class)
                    .then_some(*size)
            }) else {
                continue;
            };
            let thing = thing as u32;
            let Some(m) = world
                .thing_display(thing)
                .and_then(|d| d.world)
                .or(state.world_matrix)
            else {
                continue;
            };
            let place = moved.transform_point3(Vec3::new(m[12], m[13], m[14]));
            out.push(Corona { thing, place, size });
        }
        Self(out)
    }
}

/// A decoded picture (RGBA, top mip).
#[derive(Clone)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// D321 (part 72): what the scripted Pod's menu draws from the user's game data: the
/// Gooey's fonts, the card's picture and the card's pins. Anything missing is drawn as
/// before (the OS sans serif, flat colours).
#[derive(Resource, Default)]
pub struct MenuAssets {
    pub fonts: std::collections::BTreeMap<lbp_runtime::GooeyFont, std::sync::Arc<GameFont>>,
    /// The layout's measure of the loaded fonts.
    pub gooey_fonts: lbp_runtime::GooeyFonts,
    pub picture: Option<Picture>,
    pub pins: Vec<Picture>,
    /// D323 (part 74): the planets' corona ([`CORONA_TEXTURE`]) and the card's atlas
    /// ([`CARD_ATLAS`]).
    pub corona: Option<Picture>,
    pub card_atlas: Option<Picture>,
}

/// The default picture of a player's card: `gamedata/texture_library/pod_computer/
/// profile_default.tex`. FOUND (01.33): its GUID is in the engine's GUID table at
/// 0xc2726c, next to the level picture's default (`level_default.tex`, 000029ba).
/// MEASURED (part 72): the game frame's card picture is this texture (mean difference 8.0
/// levels over the face against 41 for a flat colour), turned with the card. INFERRED: it
/// stands for the player's own picture (the PSN avatar) when there is none, offline.
pub const PROFILE_DEFAULT_PICTURE: u32 = 0x28d8;

/// D323 (part 74): `gamedata/mesh_library/coder_art/earth_corona.tex` (00006993), a ring of
/// light on black. FOUND (RenderDoc 6082 to 9601): the texture of the two additive draws
/// after the copy back, its top mip byte for byte; its GUID is in the engine's static
/// resource list (0xc2658c, set up at 0x12668).
pub const CORONA_TEXTURE: u32 = 0x6993;
/// D323: `gamedata/texture_library/ui/player_badge_overlays.tex` (00009c83), the card's
/// atlas: the Polaroid frame (top left cell), its dark back with a soft drop shadow (bottom
/// left). FOUND (9601 / 8980): the card's first and third draws, top mip byte for byte.
pub const CARD_ATLAS: u32 = 0x9c83;

/// Loads [`MenuAssets`] for the scripted Pod: the fonts as loose files under the game
/// directories, the card picture and the pins (`--pod-card-pins HEX,HEX,HEX`, pin
/// texture GUIDs) through the catalog. Returns a line for the log.
pub fn load_menu_assets(
    game: &str,
    source: &lbp_catalog::attachment::AttachedSource,
    catalog: &lbp_catalog::Catalog,
    control: &lbp_catalog::LoadControl,
    pins: &[u32],
) -> (MenuAssets, String) {
    let (fonts, missing) = crate::game_font::load_fonts(game, catalog);
    let gooey_fonts = lbp_runtime::GooeyFonts {
        faces: fonts.iter().map(|(k, f)| (*k, f.advances())).collect(),
    };
    let picture = |guid: u32| {
        lbp_preview::story::decode_icon(
            source,
            catalog,
            lbp_preview::story::ResourceKey::Guid(guid),
            control,
        )
        .map(|i| Picture {
            width: i.width,
            height: i.height,
            rgba: i.rgba,
        })
        .map_err(|e| format!("{guid:08x}: {e}"))
    };
    let mut notes = missing;
    let card = picture(PROFILE_DEFAULT_PICTURE)
        .map_err(|e| notes.push(e))
        .ok();
    let pins: Vec<Picture> = pins
        .iter()
        .filter_map(|g| picture(*g).map_err(|e| notes.push(e)).ok())
        .collect();
    let corona = picture(CORONA_TEXTURE).map_err(|e| notes.push(e)).ok();
    let card_atlas = picture(CARD_ATLAS).map_err(|e| notes.push(e)).ok();
    let line = format!(
        "Pod menu assets: fonts {:?}; card picture {}; pins {}; corona {}; card atlas {}{}",
        fonts.keys().collect::<Vec<_>>(),
        if card.is_some() {
            "profile_default"
        } else {
            "none"
        },
        pins.len(),
        corona.is_some(),
        card_atlas.is_some(),
        if notes.is_empty() {
            String::new()
        } else {
            format!("; not loaded: {}", notes.join("; "))
        }
    );
    (
        MenuAssets {
            fonts: fonts
                .into_iter()
                .map(|(k, f)| (k, std::sync::Arc::new(f)))
                .collect(),
            gooey_fonts,
            picture: card,
            pins,
            corona,
            card_atlas,
        },
        line,
    )
}

/// `--pod-card-pins HEX,HEX,HEX`: the pin textures the card shows (the player's profile,
/// which the viewer does not read, chooses them in the game).
pub fn card_pins_option(all: &[String]) -> Result<Vec<u32>, String> {
    let Some(i) = all.iter().position(|a| a == "--pod-card-pins") else {
        return Ok(Vec::new());
    };
    let list = all
        .get(i + 1)
        .ok_or("--pod-card-pins requires HEX,HEX,HEX")?;
    list.split(',')
        .filter(|g| !g.is_empty())
        .take(3)
        .map(|g| u32::from_str_radix(g.trim(), 16).map_err(|_| format!("--pod-card-pins: {g}")))
        .collect()
}

fn image_of(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Component)]
pub(crate) struct ScriptMenuRoot;

/// One placed item's node ([`ScriptMenuFrame::placed`] index).
#[derive(Component)]
pub(crate) struct ScriptMenuNode(usize);

/// A part of the card (its picture, an icon) with its colour, faded with the menu.
#[derive(Component)]
pub(crate) struct ScriptMenuPart([f32; 3]);

/// The label of a button node.
#[derive(Component)]
pub(crate) struct ScriptMenuLabel(usize);

/// A label drawn in the game's font: the item, its text and font, root-frame units a font
/// pixel, and the image pixels a font pixel it was last rasterized at.
#[derive(Component)]
pub(crate) struct GameLabel {
    item: usize,
    text: String,
    font: std::sync::Arc<GameFont>,
    units_per_font_pixel: f32,
    built: f32,
    /// D322: the last raster as the menu layer's image.
    layer_image: Option<MenuImage>,
}

/// A picture of the card (the player's picture, a pin), faded with the menu.
#[derive(Component)]
pub(crate) struct ScriptMenuImage;

/// The focused pill's colour: MEASURED on the game frame's Story pill (31 to 51, 204 to
/// 211, 200 to 204), the third colour of the default player colour (entry 0 of the script
/// class `PlayerColour`: 29, 220, 205). INFERRED: the focus takes that colour.
const FOCUS_COLOUR: [f32; 3] = [29. / 255., 220. / 255., 205. / 255.];
/// D323 FOUND (frames 8348 / 8980 / 9601, the draws after the four pills): the focused pill
/// is drawn as a pill in (0.2157, 0.5686, 0.8431) at alpha 0.8 (PS 3451), then the same
/// shape eight times in [`FOCUS_COLOUR`] at alpha 0.25 (PS 3455, its edge from a curve
/// test), each a step [`focus_layer_step`] further back along the menu's z, the last at
/// the pill itself: a slab seen from the front. Over the scene its middle is 0.9 cyan,
/// 0.08 blue.
const FOCUS_BASE: [f32; 4] = [0.2157, 0.5686, 0.8431, 0.8];
const FOCUS_LAYERS: usize = 8;
const FOCUS_LAYER_ALPHA: f32 = 0.25;
/// D323 MEASURED: the step between the focused pill's layers, in the menu's units (x and y
/// along the pill's own axes, z back), by the frame's age in frames: 62.8 / 44.5 / 17.7
/// back at ticks 611 / 616 / 619 (ages 10, 15, 18). INFERRED: linear between them and held
/// outside; the rule that shrinks it was not read.
const FOCUS_LAYER_STEPS: [(f32, [f32; 3]); 3] = [
    (10., [4.843, 3.216, 62.826]),
    (15., [2.234, 1.408, 44.524]),
    (18., [0.878, 0.167, 17.672]),
];

/// D324 MEASURED (RenderDoc 7295 / 7927 / 8348 / 8980 / 9601, ticks 606 / 609 / 611 / 616 /
/// 619, ages 5 / 8 / 10 / 15 / 18): the focused pill's matrix scale over the other pills' is
/// 1.468 / 1.571 / 1.574 / 1.511 / 1.491, a damped spring from 1 at the frame's first draw
/// to 1.5: `1.5 - 0.5 e^(-d age) cos(f age)` with d 0.1869, f 0.2830 a frame fits every
/// capture within 0.003 (INFERRED: the rule; Gooey's own code was not read).
const FOCUS_SPRING_REST: f32 = 1.5;
const FOCUS_SPRING_DECAY: f32 = 0.1869;
const FOCUS_SPRING_FREQ: f32 = 0.2830;

/// The focused pill's scale over an unfocused one's at a frame age ([`FOCUS_SPRING_REST`]).
fn focus_spring(age: f32) -> f32 {
    let age = age.max(0.);
    FOCUS_SPRING_REST
        - (FOCUS_SPRING_REST - 1.)
            * (-FOCUS_SPRING_DECAY * age).exp()
            * (FOCUS_SPRING_FREQ * age).cos()
}

/// The focused pill's size at a frame age as a share of its size at rest (the layout's
/// [`lbp_runtime::FOCUS_SCALE`], measured on the game's frame at rest).
fn focus_grow(age: f32) -> f32 {
    focus_spring(age) / FOCUS_SPRING_REST
}

/// D324 FOUND: the eight layers of the focused pill (PS 3455) are not drawn at ages 5 and 8
/// and are at 10, 15 and 18. INFERRED: they are drawn once the spring has passed its peak,
/// at age (pi - atan(d / f)) / f, 9.04.
fn focus_slab_shown(age: f32) -> bool {
    let peak = (std::f32::consts::PI - (FOCUS_SPRING_DECAY / FOCUS_SPRING_FREQ).atan())
        / FOCUS_SPRING_FREQ;
    age >= peak
}

/// D324 FOUND (PS 3483, the last draw of frames 7295 / 7927 / 8348, absent from 8980 on):
/// while the frame comes in the menu is drawn into a target of its own, then laid over the
/// scene as one: the target read at the pixel and at seven taps on a ring (the shader's
/// unit directions), the eight averaged and multiplied by the frame's alpha, blended One,
/// OneMinusSrcAlpha. The ring's radius is 16 texels times the vertex colour's red, which is
/// 0.618 of its blue, and the blue is 1 - the alpha (95 / 153, 63 / 102, 42 / 68 of 255):
/// 9.9 (1 - alpha) pixels of the 1280-wide picture.
const TRANSITION_BLUR_PX: f32 = 16. * 0.618;

/// The step between the focused pill's layers at a frame age ([`FOCUS_LAYER_STEPS`]).
fn focus_layer_step(age: f32) -> Vec3 {
    let steps = FOCUS_LAYER_STEPS;
    let (first, last) = (steps[0], steps[steps.len() - 1]);
    if age <= first.0 {
        return Vec3::from(first.1);
    }
    for pair in steps.windows(2) {
        let ((a0, s0), (a1, s1)) = (pair[0], pair[1]);
        if age <= a1 {
            let t = (age - a0) / (a1 - a0);
            return Vec3::from(s0).lerp(Vec3::from(s1), t);
        }
    }
    Vec3::from(last.1)
}

/// D323 FOUND (PS 3441 before each pin, frames 8348 / 8980 / 9601): a pin's drop shadow is
/// its texture's alpha read at nine taps 0.0125 of the texture apart, black, at alpha 0.7
/// times the taps' weighted sum (0.7796 at most); MEASURED: the texture lands 1.0392 times
/// the pin's size about a centre 0.0100 / 0.0112 of the pin's side right of and below the
/// pin's.
const PIN_SHADOW_TAP: f32 = 0.0125;
const PIN_SHADOW_ALPHA: f32 = 0.7;
const PIN_SHADOW_SCALE: f32 = 1.0392;
const PIN_SHADOW_OFFSET: [f32; 2] = [0.0100, 0.0112];
/// D323 FOUND (frames 8980 / 9601, the card's three draws): the card is a square
/// [`CARD_QUAD`] menu units a side drawn three times: the atlas's dark back (bottom left
/// cell; its soft edge is the card's drop shadow), the player's picture at
/// [`CARD_QUAD_PICTURE`] (left, top, side as shares of the square), the Polaroid frame (top
/// left cell). The frame's opaque part spans the cell's x 23 to 233 and y 0 to 250 of 256:
/// the square is placed with that box's centre on the card's.
const CARD_QUAD: f32 = 280.8;
const CARD_QUAD_PICTURE: [f32; 3] = [0.1250, 0.0390, 0.7493];
const CARD_FRAME_BOX_DOWN: f32 = 0.5 - 125. / 256.;

/// D323: the coronas as the menu layer draws them, made each frame by [`paint_coronas`].
#[derive(Resource, Default)]
pub struct CoronaPrims(pub Vec<CoronaPrim>);

/// D323 (part 74): the planets' coronas for the menu layer, from [`PodCoronas`], the
/// camera and the planets' draw entities (their bounds give the spheres that hide them).
/// Only with the final pass (the layer's route).
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_coronas(
    coronas: Res<PodCoronas>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    cameras: Query<(&Transform, &Projection), With<Camera3d>>,
    windows: Query<&Window>,
    assets: Option<Res<MenuAssets>>,
    things: Option<Res<crate::thing_view::LevelThings>>,
    parts: Query<(
        &GlobalTransform,
        &bevy::camera::primitives::Aabb,
        &InheritedVisibility,
    )>,
    mut out: ResMut<CoronaPrims>,
    mut image: Local<Option<MenuImage>>,
) {
    out.0.clear();
    if !crate::glow_post::final_pass_on() || coronas.0.is_empty() {
        return;
    }
    let (Some(display), Ok((transform, projection)), Ok(window), Some(picture)) = (
        display,
        cameras.single(),
        windows.single(),
        assets.as_ref().and_then(|a| a.corona.as_ref()),
    ) else {
        return;
    };
    let texture = image
        .get_or_insert_with(|| MenuImage::from_rgba(picture.width, picture.height, &picture.rgba))
        .clone();
    let (w, h) = (
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    let lens = projection.get_clip_from_view();
    let (p00, p11) = (lens.x_axis.x, lens.y_axis.y);
    let turn = Mat3::from_quat(transform.rotation);
    let pixel_to_ray = turn
        * Mat3::from_cols(
            Vec3::new(2. / (w * p00), 0., 0.),
            Vec3::new(0., -2. / (h * p11), 0.),
            Vec3::new(-1. / p00, 1. / p11, -1.),
        );
    let eye = transform.translation;
    let clip_from_display = lens * transform.to_matrix().inverse();
    let unit = display.0.x_axis.truncate().length();
    // Each planet's sphere: the largest of its draw entities' bounds (the globe; the
    // per-bone parts of the Earth are pieces of it), as a sphere around the box's centre.
    let spheres: Vec<Vec4> = coronas
        .0
        .iter()
        .filter_map(|c| {
            let entities = things.as_ref()?.entities.get(&c.thing)?;
            entities
                .iter()
                .filter_map(|e| parts.get(*e).ok())
                .filter(|(_, _, shown)| shown.get())
                .map(|(g, aabb, _)| {
                    let m = g.to_matrix();
                    let k = (m.x_axis.truncate().length()
                        + m.y_axis.truncate().length()
                        + m.z_axis.truncate().length())
                        / 3.;
                    let half = Vec3::from(aabb.half_extents);
                    let r = (half.x + half.y + half.z) / 3. * k;
                    (m.transform_point3(Vec3::from(aabb.center)) - eye).extend(r)
                })
                .max_by(|a, b| a.w.total_cmp(&b.w))
        })
        .collect();
    let up = turn * Vec3::Y;
    for c in &coronas.0 {
        let place = display.0.transform_point3(c.place);
        let size = c.size * unit;
        let to_eye = (eye - place).normalize_or_zero();
        let across = up.cross(to_eye).normalize_or_zero();
        if to_eye == Vec3::ZERO || across == Vec3::ZERO {
            continue;
        }
        let along = to_eye.cross(across);
        let centre = place - to_eye * (CORONA_PULL * size) - eye;
        let axes = [across * size, along * size];
        // The pixels of the square's corners (all of the layer when one is behind the eye).
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        let mut behind = false;
        for (sx, sy) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
            let p = eye + centre + (axes[0] * sx + axes[1] * sy) * CORONA_HALF;
            let clip = clip_from_display * p.extend(1.);
            if clip.w <= 0. {
                behind = true;
                break;
            }
            let px = Vec2::new(
                (clip.x / clip.w + 1.) / 2. * w,
                (1. - clip.y / clip.w) / 2. * h,
            );
            lo = lo.min(px);
            hi = hi.max(px);
        }
        let bounds = if behind {
            [0, 0, w as usize, h as usize]
        } else {
            [
                lo.x.floor().clamp(0., w) as usize,
                lo.y.floor().clamp(0., h) as usize,
                hi.x.ceil().clamp(0., w) as usize,
                hi.y.ceil().clamp(0., h) as usize,
            ]
        };
        out.0.push(CoronaPrim {
            pixel_to_ray,
            centre,
            axes,
            half: CORONA_HALF,
            bounds,
            spheres: spheres.clone(),
            colour: CORONA_COLOUR,
            image: texture.clone(),
        });
    }
}

/// MEASURED (game frame, alpha channel = the menu's coverage): an unfocused pill's body
/// covers 82 / 255 of the picture, its text about 0.62.
const PILL_ALPHA: f32 = 82. / 255.;
const LABEL_ALPHA: f32 = 0.62;
/// D322 FOUND (frames 8980 / 9601, the text draws after the copy back): an unfocused pill's
/// text is white at alpha 102 / 255 over the pill (the target's alpha 82 + 102 = 184 under
/// the letters); the focused pill's text is opaque white. Used when the menu is drawn as
/// the game draws it (the final pass on); [`LABEL_ALPHA`] stays the overlay's.
const GAME_LABEL_ALPHA: f32 = 102. / 255.;
/// Text size against the pill's height (INFERRED by eye from the game frame's letters).
const LABEL_SIZE: f32 = 0.56;
/// A game-font label is rasterized again when its size has changed by this share.
const LABEL_RASTER_STEP: f32 = 0.02;
/// The card: the white frame MEASURED on the game frame (216, 221, 226, opaque), turned
/// clockwise by `CARD_TURN` (MEASURED: its edges 2.9 to 3.8 degrees, the picture's best
/// match 2.5; what turns it is not found). Without the picture, a flat colour (the mean of
/// the game's picture of the player); without pins (`--pod-card-pins`), three round
/// PLACEHOLDER icons in the game's colours.
const CARD_FRAME: [f32; 4] = [216. / 255., 221. / 255., 226. / 255., 1.];
const CARD_TURN: f32 = 0.05;
/// MEASURED on the game frame (part 72), in shares of the card: the picture's left, top,
/// width and height (108 px of a 118 x 145 px card, square) ...
const CARD_PICTURE_BOX: [f32; 4] = [4.2, 4.5, 91.5, 74.5];
/// ... and the pins: their centres across and down, their width (41 px).
const CARD_PIN_CENTRES: [f32; 3] = [18.5, 49.3, 80.];
const CARD_PIN_DOWN: f32 = 86.;
const CARD_PIN_WIDTH: f32 = 35.;
const CARD_PICTURE: [f32; 3] = [132. / 255., 128. / 255., 105. / 255.];
const CARD_ICONS: [[f32; 3]; 3] = [
    [161. / 255., 179. / 255., 132. / 255.],
    [253. / 255., 253. / 255., 253. / 255.],
    [253. / 255., 120. / 255., 1. / 255.],
];

fn rgba(c: [f32; 3], a: f32) -> Color {
    Color::srgba(c[0], c[1], c[2], a)
}

/// Builds the menu's nodes for `menu` under a new root.
fn spawn_script_menu(
    commands: &mut Commands,
    menu: &ScriptMenuFrame,
    assets: Option<&MenuAssets>,
    images: &mut Assets<Image>,
) {
    let font_of = |style: i32| {
        let (font, size) = lbp_runtime::text_style(style);
        Some((
            assets?.fonts.get(&font)?.clone(),
            lbp_runtime::units_per_font_pixel(size),
        ))
    };
    // The card's pictures, made once per build.
    let card_picture = assets
        .and_then(|a| a.picture.as_ref())
        .map(|p| images.add(image_of(p.width, p.height, p.rgba.clone())));
    let card_pins: Vec<Handle<Image>> = assets
        .map(|a| {
            a.pins
                .iter()
                .map(|p| images.add(image_of(p.width, p.height, p.rgba.clone())))
                .collect()
        })
        .unwrap_or_default();
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            ScriptMenuRoot,
        ))
        .id();
    for (i, item) in menu.placed.iter().enumerate() {
        let node = match &item.kind {
            lbp_runtime::PlacedKind::Button {
                label,
                style,
                font_scale,
                ..
            }
            | lbp_runtime::PlacedKind::Text {
                text: label,
                style,
                font_scale,
            } if font_of(*style).is_some() => {
                let (font, units) = font_of(*style).expect("checked");
                commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(Val::Px(9999.)),
                            ..default()
                        },
                        UiTransform::IDENTITY,
                        BackgroundColor(Color::NONE),
                        ScriptMenuNode(i),
                        children![(
                            Node {
                                flex_shrink: 0.,
                                ..default()
                            },
                            ImageNode::default(),
                            GameLabel {
                                item: i,
                                text: label.clone(),
                                font,
                                units_per_font_pixel: units * font_scale,
                                built: 0.,
                                layer_image: None,
                            },
                        )],
                    ))
                    .id()
            }
            lbp_runtime::PlacedKind::Button { label, .. } => commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(9999.)),
                        ..default()
                    },
                    UiTransform::IDENTITY,
                    BackgroundColor(Color::NONE),
                    ScriptMenuNode(i),
                    children![(
                        Text::new(label.clone()),
                        TextFont {
                            font: bevy::text::FontSource::SansSerif,
                            weight: bevy::text::FontWeight::BOLD,
                            ..TextFont::from_font_size(20.)
                        },
                        TextColor(Color::WHITE),
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        ScriptMenuLabel(i),
                    )],
                ))
                .id(),
            lbp_runtime::PlacedKind::PlayerIcon => {
                // Every part is placed in percent of the card itself (absolute), so the
                // card keeps the size the projection gives it; the pins are square, their
                // height a share of the card's width (`aspect_ratio`).
                let card = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        UiTransform::IDENTITY,
                        BackgroundColor(Color::NONE),
                        ScriptMenuNode(i),
                    ))
                    .id();
                let [left, top, width, height] = CARD_PICTURE_BOX;
                let picture_node = Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(left),
                    top: Val::Percent(top),
                    width: Val::Percent(width),
                    height: Val::Percent(height),
                    ..default()
                };
                let picture = match &card_picture {
                    Some(h) => commands
                        .spawn((picture_node, ImageNode::new(h.clone()), ScriptMenuImage))
                        .id(),
                    None => commands
                        .spawn((
                            picture_node,
                            BackgroundColor(rgba(CARD_PICTURE, 1.)),
                            ScriptMenuPart(CARD_PICTURE),
                        ))
                        .id(),
                };
                commands.entity(card).add_child(picture);
                for (n, x) in CARD_PIN_CENTRES.into_iter().enumerate() {
                    let pin_node = Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(x - CARD_PIN_WIDTH / 2.),
                        top: Val::Percent(CARD_PIN_DOWN),
                        width: Val::Percent(CARD_PIN_WIDTH),
                        aspect_ratio: Some(1.),
                        margin: UiRect::top(Val::Percent(-CARD_PIN_WIDTH / 2.)),
                        border_radius: BorderRadius::all(Val::Px(9999.)),
                        ..default()
                    };
                    let pin = match card_pins.get(n) {
                        Some(h) => commands
                            .spawn((
                                Node {
                                    border_radius: BorderRadius::ZERO,
                                    ..pin_node
                                },
                                ImageNode::new(h.clone()),
                                ScriptMenuImage,
                            ))
                            .id(),
                        None => commands
                            .spawn((
                                pin_node,
                                BackgroundColor(rgba(CARD_ICONS[n], 1.)),
                                ScriptMenuPart(CARD_ICONS[n]),
                            ))
                            .id(),
                    };
                    commands.entity(card).add_child(pin);
                }
                card
            }
            lbp_runtime::PlacedKind::Text { text, .. } => commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    UiTransform::IDENTITY,
                    BackgroundColor(Color::NONE),
                    ScriptMenuNode(i),
                    children![(
                        Text::new(text.clone()),
                        TextFont::from_font_size(20.),
                        TextColor(Color::WHITE),
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        ScriptMenuLabel(i),
                    )],
                ))
                .id(),
        };
        commands.entity(root).add_child(node);
    }
}

/// Where a point of the menu's root frame (its units) shows, in logical pixels from the
/// top left, through `world_from_menu`, the level's display matrix and the camera.
fn menu_to_screen(clip_from_world: Mat4, world_from_menu: Mat4, p: Vec2, size: Vec2) -> Vec3 {
    menu_to_screen_3d(clip_from_world, world_from_menu, p.extend(0.), size)
}

/// [`menu_to_screen`] for a point off the menu's plane (z along the menu's own z).
fn menu_to_screen_3d(clip_from_world: Mat4, world_from_menu: Mat4, p: Vec3, size: Vec2) -> Vec3 {
    let world = world_from_menu.transform_point3(p);
    let clip = clip_from_world * world.extend(1.);
    let ndc = clip.truncate() / clip.w;
    Vec3::new(
        (ndc.x + 1.) / 2. * size.x,
        (1. - ndc.y) / 2. * size.y,
        clip.w,
    )
}

/// D322: the menu layer's state: the canvas, what was drawn on it last and the card's
/// pictures as layer images (made once).
#[derive(Default)]
pub(crate) struct LayerState {
    canvas: Option<MenuCanvas>,
    last: Vec<MenuPrim>,
    /// D323: the coronas drawn last.
    last_coronas: Vec<CoronaPrim>,
    /// D324: the transition drawn last (alpha, ring radius in pixels) and the menu's own
    /// target while the frame comes in.
    last_transition: (f32, f32),
    transition_canvas: Option<MenuCanvas>,
    pictures: Option<LayerPictures>,
}

/// The menu's pictures as layer images: the player's picture, the pins and (D323) their
/// shadows, the card atlas's back and frame cells.
#[derive(Default, Clone)]
struct LayerPictures {
    picture: Option<MenuImage>,
    pins: Vec<MenuImage>,
    pin_shadows: Vec<MenuImage>,
    card_back: Option<MenuImage>,
    card_frame: Option<MenuImage>,
}

impl LayerPictures {
    fn of(assets: Option<&MenuAssets>) -> Self {
        let image = |p: &Picture| MenuImage::from_rgba(p.width, p.height, &p.rgba);
        let pins: Vec<MenuImage> = assets
            .map(|a| a.pins.iter().map(image).collect())
            .unwrap_or_default();
        let pin_shadows = pins
            .iter()
            .map(|p| p.shadow(PIN_SHADOW_TAP * p.width as f32))
            .collect();
        let atlas = assets.and_then(|a| a.card_atlas.as_ref()).map(image);
        let cell = |row: usize| {
            atlas.as_ref().map(|a| {
                let (w, h) = (a.width / 2, a.height / 2);
                a.crop(0, row * h, w, h)
            })
        };
        Self {
            picture: assets.and_then(|a| a.picture.as_ref()).map(image),
            pins,
            pin_shadows,
            card_back: cell(1),
            card_frame: cell(0),
        }
    }
}

fn layer_texture(
    width: u32,
    height: u32,
    format: bevy::render::render_resource::TextureFormat,
    data: Vec<u8>,
) -> Image {
    Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        format,
        bevy::asset::RenderAssetUsages::default(),
    )
}

/// D322: draws `prims` into the layer when they or the window size changed, uploads it
/// and turns it on for the final pass.
#[allow(clippy::too_many_arguments)]
fn update_layer(
    commands: &mut Commands,
    layer: Option<Mut<crate::glow_post::MenuLayer>>,
    state: &mut LayerState,
    images: &mut Assets<Image>,
    (width, height): (usize, usize),
    prims: Vec<MenuPrim>,
    coronas: &[CoronaPrim],
    transition: (f32, f32),
) {
    use bevy::render::render_resource::TextureFormat;
    let resized = state
        .canvas
        .as_ref()
        .is_none_or(|c| (c.width, c.height) != (width, height));
    if !resized
        && prims == state.last
        && coronas == state.last_coronas.as_slice()
        && transition == state.last_transition
        && layer.as_ref().is_some_and(|l| l.on)
    {
        return;
    }
    if resized {
        state.canvas = Some(MenuCanvas::new(width, height));
    }
    let canvas = state.canvas.as_mut().expect("made above");
    canvas.clear();
    // D323: the coronas first (drawn before the menu, after the copy back).
    for c in coronas {
        canvas.draw_corona(c);
    }
    let (alpha, radius) = transition;
    if alpha < 1. && !prims.is_empty() {
        // D324: while the frame comes in, the menu in its own target, then laid over as one.
        let menu = state
            .transition_canvas
            .get_or_insert_with(|| MenuCanvas::new(width, height));
        if (menu.width, menu.height) != (width, height) {
            *menu = MenuCanvas::new(width, height);
        }
        menu.clear();
        for p in &prims {
            menu.draw(p);
        }
        canvas.compose_transition(menu, alpha, radius);
    } else {
        state.transition_canvas = None;
        for p in &prims {
            canvas.draw(p);
        }
    }
    state.last = prims;
    state.last_coronas = coronas.to_vec();
    state.last_transition = transition;
    let (colour, alpha) = canvas.bytes();
    let (w, h) = (width as u32, height as u32);
    match layer {
        Some(mut l) if !resized => {
            if let Some(mut image) = images.get_mut(&l.colour) {
                image.data = Some(colour);
            }
            if let Some(mut image) = images.get_mut(&l.alpha) {
                image.data = Some(alpha);
            }
            l.on = true;
        }
        _ => {
            commands.insert_resource(crate::glow_post::MenuLayer {
                colour: images.add(layer_texture(w, h, TextureFormat::Rgba8Unorm, colour)),
                alpha: images.add(layer_texture(w, h, TextureFormat::R8Unorm, alpha)),
                on: true,
            });
        }
    }
}

/// D319: draws the scripts' main menu over the planets each frame, after the script camera
/// is placed: every item at the place the menu's matrix, the level's display matrix and the
/// camera give its centre, sized and turned by the same projection (the menu follows the
/// planets world through the zoom), the pill colours of the game frame, faded in by the
/// frame's transition. D321: text in the game's font and the card with the player's
/// picture and pins where [`MenuAssets`] has them (else the OS sans serif in bold and flat
/// PLACEHOLDER colours). D322: with the final pass the menu is drawn into the layer the
/// pass composes (`menu_layer.rs`), as the game draws it (part 73).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn paint_script_menu(
    mut commands: Commands,
    menu: Option<Res<ScriptMenu>>,
    display: Option<Res<crate::sackboy_play::LevelDisplay>>,
    cameras: Query<(&Transform, &Projection), With<Camera3d>>,
    windows: Query<&Window>,
    mut roots: Query<(Entity, &mut Visibility), With<ScriptMenuRoot>>,
    mut nodes: Query<
        (
            &ScriptMenuNode,
            &mut Node,
            &mut UiTransform,
            &mut BackgroundColor,
        ),
        Without<ScriptMenuRoot>,
    >,
    mut labels: Query<(&ScriptMenuLabel, &mut TextFont, &mut TextColor)>,
    mut parts: Query<(&ScriptMenuPart, &mut BackgroundColor), Without<ScriptMenuNode>>,
    mut game_labels: Query<
        (&mut GameLabel, &mut Node, &mut ImageNode),
        (Without<ScriptMenuNode>, Without<ScriptMenuImage>),
    >,
    mut pictures: Query<&mut ImageNode, (With<ScriptMenuImage>, Without<GameLabel>)>,
    (assets, mut images): (Option<Res<MenuAssets>>, ResMut<Assets<Image>>),
    mut built: Local<Option<usize>>,
    (mut layer, mut layer_state): (
        Option<ResMut<crate::glow_post::MenuLayer>>,
        Local<LayerState>,
    ),
    coronas: Res<CoronaPrims>,
) {
    let frame = menu.as_ref().and_then(|m| m.0.as_ref());
    let (Some(frame), Some(display), Ok((transform, projection)), Ok(window)) =
        (frame, display, cameras.single(), windows.single())
    else {
        for (_, mut v) in &mut roots {
            *v = Visibility::Hidden;
        }
        // D323: the planets' coronas show without the menu too (the closed Pod, part 74).
        if let (false, Ok(window)) = (coronas.0.is_empty(), windows.single()) {
            let size = (
                window.physical_width() as usize,
                window.physical_height() as usize,
            );
            update_layer(
                &mut commands,
                layer.map(Mut::from),
                &mut layer_state,
                &mut images,
                size,
                Vec::new(),
                &coronas.0,
                (1., 0.),
            );
        } else if let Some(l) = layer.as_mut().filter(|l| l.on) {
            l.on = false;
        }
        return;
    };
    if *built != Some(frame.placed.len()) {
        for (e, _) in &roots {
            commands.entity(e).despawn();
        }
        spawn_script_menu(&mut commands, frame, assets.as_deref(), &mut images);
        *built = Some(frame.placed.len());
        return;
    }
    for (_, mut v) in &mut roots {
        *v = Visibility::Inherited;
    }
    let size = Vec2::new(window.width(), window.height());
    let clip_from_world =
        projection.get_clip_from_view() * transform.to_matrix().inverse() * display.0;
    let world_from_menu = frame.matrix;
    // D322: with the final pass the menu is drawn as the game draws it, into a layer the pass
    // composes (premultiplied over in encoded values, alpha added); the UI nodes then only
    // place things (and draw the OS-font labels when the game's font is missing).
    let layer_route = crate::glow_post::final_pass_on();
    // D324: on the layer every draw keeps its own alpha and the frame comes in as one
    // ([`TRANSITION_BLUR_PX`]); the UI nodes fade each item.
    let fade = if layer_route { 1. } else { frame.fade };
    let scale = window.scale_factor();
    let mut prims: Vec<Vec<MenuPrim>> = vec![Vec::new(); frame.placed.len()];
    let mut placed_at: Vec<Option<(Vec2, f32)>> = vec![None; frame.placed.len()];
    if layer_route && layer_state.pictures.is_none() {
        layer_state.pictures = Some(LayerPictures::of(assets.as_deref()));
    }
    let pics = layer_state.pictures.clone().unwrap_or_default();
    for (ScriptMenuPart(c), mut background) in &mut parts {
        *background = if layer_route {
            BackgroundColor(Color::NONE)
        } else {
            BackgroundColor(rgba(*c, fade))
        };
    }
    for mut image in &mut pictures {
        image.color = Color::srgba(1., 1., 1., if layer_route { 0. } else { fade });
    }
    // Screen pixels a root-frame unit along each item's own x (for the game-font labels).
    let mut px_per_unit = vec![None; frame.placed.len()];
    for (ScriptMenuNode(i), mut node, mut ui, mut background) in &mut nodes {
        let Some(item) = frame.placed.get(*i) else {
            continue;
        };
        let c = Vec2::from(item.centre);
        let at = menu_to_screen(clip_from_world, world_from_menu, c, size);
        let along = menu_to_screen(clip_from_world, world_from_menu, c + Vec2::X, size);
        let down = menu_to_screen(clip_from_world, world_from_menu, c + Vec2::Y, size);
        let (dx, dy) = ((along - at).truncate(), (down - at).truncate());
        // D324: the focused pill grows in on its spring.
        let grow = match item.kind {
            lbp_runtime::PlacedKind::Button { focused: true, .. } => focus_grow(frame.age),
            _ => 1.,
        };
        let (w, h) = (
            item.size[0] * dx.length() * grow,
            item.size[1] * dy.length() * grow,
        );
        if at.z <= 0. || !w.is_finite() || !h.is_finite() {
            node.display = Display::None;
            continue;
        }
        px_per_unit[*i] = Some(dx.length());
        node.display = Display::Flex;
        node.left = Val::Px(at.x - w / 2.);
        node.top = Val::Px(at.y - h / 2.);
        node.width = Val::Px(w);
        node.height = Val::Px(h);
        let turn = if item.kind == lbp_runtime::PlacedKind::PlayerIcon {
            CARD_TURN
        } else {
            0.
        };
        let angle = item.angle + turn + dx.y.atan2(dx.x);
        ui.rotation = Rot2::radians(angle);
        // The fill in encoded values (the game's numbers) and its alpha.
        let fill = match &item.kind {
            // D323: on the layer the focused pill starts as the blue base under its layers.
            lbp_runtime::PlacedKind::Button { focused: true, .. } if layer_route => Some([
                FOCUS_BASE[0],
                FOCUS_BASE[1],
                FOCUS_BASE[2],
                FOCUS_BASE[3] * fade,
            ]),
            lbp_runtime::PlacedKind::Button { focused: true, .. } => {
                Some([FOCUS_COLOUR[0], FOCUS_COLOUR[1], FOCUS_COLOUR[2], fade])
            }
            lbp_runtime::PlacedKind::Button { .. } => {
                let c = frame.colour;
                Some([c[0], c[1], c[2], PILL_ALPHA * fade])
            }
            lbp_runtime::PlacedKind::PlayerIcon => Some([
                CARD_FRAME[0],
                CARD_FRAME[1],
                CARD_FRAME[2],
                CARD_FRAME[3] * fade,
            ]),
            lbp_runtime::PlacedKind::Text { .. } => None,
        };
        *background = match fill {
            Some(f) if !layer_route => BackgroundColor(Color::srgba(f[0], f[1], f[2], f[3])),
            _ => BackgroundColor(Color::NONE),
        };
        if layer_route {
            let centre = Vec2::new(at.x, at.y);
            placed_at[*i] = Some((centre, angle));
            let card = item.kind == lbp_runtime::PlacedKind::PlayerIcon;
            let (w, h) = (w * scale, h * scale);
            let shape = if card {
                MenuShape::Rect
            } else {
                MenuShape::Capsule
            };
            let list = &mut prims[*i];
            let card_atlas = match (&pics.card_back, &pics.card_frame) {
                (Some(back), Some(front)) if card => Some((back.clone(), front.clone())),
                _ => None,
            };
            if let Some(colour) = fill.filter(|_| card_atlas.is_none()) {
                list.push(MenuPrim {
                    centre: centre * scale,
                    size: Vec2::new(w, h),
                    angle,
                    shape,
                    colour,
                    image: None,
                });
            }
            if matches!(
                item.kind,
                lbp_runtime::PlacedKind::Button { focused: true, .. }
            ) && focus_slab_shown(frame.focus_age)
            {
                // D323: the focused pill's eight layers, the deepest first, each a step
                // back along the menu's z (the projection moves and shrinks it) and a little
                // along the pill's own axes; the last at the pill.
                let step = focus_layer_step(frame.focus_age);
                let ex = Vec2::from_angle(angle);
                let up = Vec2::new(ex.y, -ex.x);
                let unit = dx.length();
                for k in (0..FOCUS_LAYERS).rev() {
                    let k = k as f32;
                    let p = c.extend(step.z * k);
                    let at_k = menu_to_screen_3d(clip_from_world, world_from_menu, p, size);
                    let along_k =
                        menu_to_screen_3d(clip_from_world, world_from_menu, p + Vec3::X, size);
                    let shrink = (along_k - at_k).truncate().length() / unit;
                    if at_k.z <= 0. || !shrink.is_finite() {
                        continue;
                    }
                    let drift = (ex * step.x + up * step.y) * k * unit;
                    list.push(MenuPrim {
                        centre: (at_k.truncate() + drift) * scale,
                        size: Vec2::new(w, h) * shrink,
                        angle,
                        shape: MenuShape::Capsule,
                        colour: [
                            FOCUS_COLOUR[0],
                            FOCUS_COLOUR[1],
                            FOCUS_COLOUR[2],
                            FOCUS_LAYER_ALPHA * fade,
                        ],
                        image: None,
                    });
                }
            }
            if card {
                // The card's parts at the places the card's nodes give them (shares of the
                // card; the pins square, a share of its width).
                let rot = Vec2::from_angle(angle);
                let at_local = |x: f32, y: f32| centre * scale + rot.rotate(Vec2::new(x, y));
                let (colour, image) = match pics.picture.clone() {
                    Some(img) => ([1., 1., 1., fade], Some(img)),
                    None => (rgba_of(CARD_PICTURE, fade), None),
                };
                if let Some((back, front)) = card_atlas {
                    // D323: the game's three draws over the card's square: the dark back
                    // (its soft edge the card's shadow), the picture, the frame.
                    let q = CARD_QUAD * w / item.size[0];
                    let quad = at_local(0., CARD_FRAME_BOX_DOWN * q);
                    let [left, top, side] = CARD_QUAD_PICTURE;
                    let white = [1., 1., 1., fade];
                    let square = |centre: Vec2, side: f32, colour, image| MenuPrim {
                        centre,
                        size: Vec2::splat(side),
                        angle,
                        shape: MenuShape::Rect,
                        colour,
                        image,
                    };
                    list.push(square(quad, q, white, Some(back)));
                    let off = Vec2::new(left + side / 2. - 0.5, top + side / 2. - 0.5) * q;
                    list.push(square(quad + rot.rotate(off), side * q, colour, image));
                    list.push(square(quad, q, white, Some(front)));
                } else {
                    let [left, top, pw, ph] = CARD_PICTURE_BOX;
                    list.push(MenuPrim {
                        centre: at_local(
                            -w / 2. + (left + pw / 2.) / 100. * w,
                            -h / 2. + (top + ph / 2.) / 100. * h,
                        ),
                        size: Vec2::new(pw / 100. * w, ph / 100. * h),
                        angle,
                        shape: MenuShape::Rect,
                        colour,
                        image,
                    });
                }
                let weights = SHADOW_WEIGHTS[0] + 4. * SHADOW_WEIGHTS[1] + 4. * SHADOW_WEIGHTS[2];
                for (n, x) in CARD_PIN_CENTRES.into_iter().enumerate() {
                    let side = CARD_PIN_WIDTH / 100. * w;
                    let pin_centre =
                        at_local(-w / 2. + x / 100. * w, -h / 2. + CARD_PIN_DOWN / 100. * h);
                    // D323: the pin's drop shadow first (PS 3441).
                    if let (Some(_), Some(shadow)) = (pics.pins.get(n), pics.pin_shadows.get(n)) {
                        list.push(MenuPrim {
                            centre: pin_centre + rot.rotate(Vec2::from(PIN_SHADOW_OFFSET) * side),
                            size: Vec2::splat(side * PIN_SHADOW_SCALE),
                            angle,
                            shape: MenuShape::Rect,
                            colour: [0., 0., 0., PIN_SHADOW_ALPHA * weights * fade],
                            image: Some(shadow.clone()),
                        });
                    }
                    let (colour, image, shape) = match pics.pins.get(n) {
                        Some(img) => ([1., 1., 1., fade], Some(img.clone()), MenuShape::Rect),
                        None => (rgba_of(CARD_ICONS[n], fade), None, MenuShape::Capsule),
                    };
                    list.push(MenuPrim {
                        centre: pin_centre,
                        size: Vec2::splat(side),
                        angle,
                        shape,
                        colour,
                        image,
                    });
                }
            }
        }
        for (ScriptMenuLabel(j), mut font, mut colour) in &mut labels {
            if j != i {
                continue;
            }
            let px = (h * LABEL_SIZE * 2.).round() / 2.;
            if !matches!(font.font_size, bevy::text::FontSize::Px(s) if s == px) {
                font.font_size = bevy::text::FontSize::Px(px.max(1.));
            }
            let focused = matches!(
                item.kind,
                lbp_runtime::PlacedKind::Button { focused: true, .. }
            );
            let a = if focused { 1. } else { LABEL_ALPHA };
            *colour = TextColor(Color::srgba(1., 1., 1., a * frame.fade));
        }
    }
    // Game-font labels: rasterized from the font's distance field at the size the
    // projection gives (the style's units a font pixel, the focused pill's scale), again
    // when that size changes; centred in the pill by their advance and cap height.
    for (mut label, mut node, mut image) in &mut game_labels {
        let Some(item) = frame.placed.get(label.item) else {
            continue;
        };
        let Some(px_scale) = px_per_unit.get(label.item).copied().flatten() else {
            continue;
        };
        let focused = matches!(
            item.kind,
            lbp_runtime::PlacedKind::Button { focused: true, .. }
        );
        let k = px_scale
            * label.units_per_font_pixel
            * if focused {
                lbp_runtime::FOCUS_SCALE * focus_grow(frame.focus_age)
            } else {
                1.
            }
            * if layer_route { scale } else { 1. };
        if (k / label.built - 1.).abs() > LABEL_RASTER_STEP || label.built == 0. {
            label.built = k;
            match label.font.raster(&label.text, k) {
                Some(r) if layer_route => {
                    label.layer_image = Some(MenuImage::from_coverage(r.width, r.height, &r.alpha));
                }
                Some(r) => {
                    node.width = Val::Px(r.width as f32);
                    node.height = Val::Px(r.height as f32);
                    image.image = images.add(image_of(r.width, r.height, r.rgba()));
                }
                None => {
                    label.layer_image = None;
                    node.display = Display::None;
                }
            }
        }
        if layer_route {
            image.color = Color::NONE;
            if let (Some(img), Some((centre, angle))) = (
                label.layer_image.clone(),
                placed_at.get(label.item).copied().flatten(),
            ) {
                let a = if focused { 1. } else { GAME_LABEL_ALPHA };
                prims[label.item].push(MenuPrim {
                    centre: centre * scale,
                    size: Vec2::new(img.width as f32, img.height as f32),
                    angle,
                    shape: MenuShape::Rect,
                    colour: [1., 1., 1., a * fade],
                    image: Some(img),
                });
            }
            continue;
        }
        let a = if focused { 1. } else { LABEL_ALPHA };
        image.color = Color::srgba(1., 1., 1., a * fade);
    }
    if layer_route {
        let size = (
            window.physical_width() as usize,
            window.physical_height() as usize,
        );
        update_layer(
            &mut commands,
            layer.map(Mut::from),
            &mut layer_state,
            &mut images,
            size,
            prims.into_iter().flatten().collect(),
            &coronas.0,
            (
                frame.fade,
                TRANSITION_BLUR_PX * (1. - frame.fade) * size.0 as f32 / 1280.,
            ),
        );
    }
}

/// An encoded colour and an alpha as a layer colour.
fn rgba_of(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D323: the focused pill's layer step is the measured one at ages 10 / 15 / 18,
    /// linear between, held outside.
    #[test]
    fn d323_focus_layer_step_by_age() {
        assert_eq!(focus_layer_step(0.).z, 62.826);
        assert_eq!(focus_layer_step(15.).z, 44.524);
        assert_eq!(focus_layer_step(40.).z, 17.672);
        let mid = focus_layer_step(16.5);
        assert!((mid.z - (44.524 + 17.672) / 2.).abs() < 1e-3, "{mid:?}");
        // The coronas' table: the Earth's unit 1.12 of its sphere (r 3239 in the world).
        let earth = CORONA_SIZES
            .iter()
            .find(|(c, _)| *c == "Earth")
            .expect("earth")
            .1;
        assert!((earth / 3239. - 1.1215).abs() < 1e-3);
        assert_eq!(CORONA_COLOUR, [0.5, 0.7, 1., 0.5]);
    }

    /// D324: the focused pill's spring against the five captures' scales, its layers shown
    /// from the spring's peak (not at 5 and 8, at 10), the transition's alpha and ring.
    #[test]
    fn d324_focus_spring_slab_and_transition() {
        for (age, game) in [
            (5., 1.468),
            (8., 1.571),
            (10., 1.574),
            (15., 1.511),
            (18., 1.491),
        ] {
            let s = focus_spring(age);
            assert!((s - game).abs() < 0.003, "{age}: {s} against {game}");
        }
        assert_eq!(focus_spring(0.), 1.);
        assert!((focus_grow(200.) - 1.).abs() < 1e-6);
        assert!(!focus_slab_shown(5.) && !focus_slab_shown(8.));
        assert!(focus_slab_shown(10.) && focus_slab_shown(400.));
        // The game's alpha (of 255) and ring red at ages 5 / 8 / 10, from (age + 1) / 15.
        for (age, alpha, red) in [(5., 101., 95.), (8., 152., 63.), (10., 186., 42.)] {
            let a: f32 = (age + 1.) / 15.;
            assert!((a * 255. - alpha).abs() < 1.5, "{age}: {a}");
            let ring = TRANSITION_BLUR_PX * (1. - a);
            assert!((ring / 16. * 255. - red).abs() < 1.5, "{age}: {ring}");
        }
    }

    #[test]
    fn d321_card_pins_option_reads_up_to_three_guids() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(card_pins_option(&args(&["--x"])), Ok(Vec::new()));
        assert_eq!(
            card_pins_option(&args(&["--pod-card-pins", "1e755,1e75e,1e753,1"])),
            Ok(vec![0x1e755, 0x1e75e, 0x1e753])
        );
        assert!(card_pins_option(&args(&["--pod-card-pins", "1e7g5"])).is_err());
        assert!(card_pins_option(&args(&["--pod-card-pins"])).is_err());
    }

    #[test]
    fn d319_menu_points_project_through_the_menu_matrix() {
        // A menu matrix that puts root units 1:1 on the plane z = -10 in front of a camera
        // at the origin: its centre projects to the screen's centre, and a unit step along
        // x moves right.
        let world_from_menu = Mat4::from_translation(Vec3::new(-5., 3., -10.))
            * Mat4::from_scale(Vec3::new(0.01, -0.01, 1.));
        let clip = Mat4::perspective_rh(1., 16. / 9., 0.1, 100.);
        let size = Vec2::new(1280., 720.);
        let centre = menu_to_screen(clip, world_from_menu, Vec2::new(500., 300.), size);
        assert!((centre.x - 640.).abs() < 0.01 && (centre.y - 360.).abs() < 0.01);
        let right = menu_to_screen(clip, world_from_menu, Vec2::new(510., 300.), size);
        let below = menu_to_screen(clip, world_from_menu, Vec2::new(500., 310.), size);
        assert!(right.x > centre.x && (right.y - centre.y).abs() < 1e-3);
        assert!(below.y > centre.y);
    }

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
