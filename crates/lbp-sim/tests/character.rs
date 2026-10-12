//! Sackboy's kinematic capsule on authored synthetic geometry (no game bytes).
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_sim::character::{spawn_sackboy, SackboyBody, SackboyPlugin, FLOOR_REST, RADIUS};
use lbp_sim::sackboy::Input;
use lbp_sim::{to_level, SimPlugin, ZSlab, TICK_HZ, UNIT_SCALE};
use std::time::Duration;

/// Standing height of the capsule centre above a floor.
const STAND: f32 = FLOOR_REST;

/// A static box (world units: centre, size) in the slab z ± 90.
fn block(commands: &mut Commands, centre: [f32; 2], size: [f32; 2], z: f32) {
    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(size[0] * UNIT_SCALE, size[1] * UNIT_SCALE),
        ZSlab {
            z,
            half_thickness: 90.,
        },
        Transform::from_xyz(centre[0] * UNIT_SCALE, centre[1] * UNIT_SCALE, 0.),
    ));
}

/// World: floor top at y = 0 in lane 0 (x -2000..2000), a second floor in lane -200 for
/// x < 0 only, and a wall in lane 0 at x 1000.
fn world(extra: impl Fn(&mut Commands) + Send + Sync + 'static) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        SimPlugin::default(),
        SackboyPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1. / TICK_HZ,
    )))
    .add_systems(Startup, move |mut commands: Commands| {
        block(&mut commands, [0., -50.], [4000., 100.], 0.);
        block(&mut commands, [-1000., -50.], [2000., 100.], -200.);
        block(&mut commands, [1000., 200.], [100., 400.], 0.);
        spawn_sackboy(&mut commands, [-500., STAND], 0.);
        extra(&mut commands);
    });
    app.finish();
    app
}

fn tick(app: &mut App, input: Input) -> ([f32; 2], f32, bool) {
    {
        let world = app.world_mut();
        let mut q = world.query::<&mut SackboyBody>();
        for mut b in q.iter_mut(world) {
            b.input = input;
        }
    }
    app.update();
    let world = app.world_mut();
    let mut q = world.query::<(&SackboyBody, &Position, &ZSlab)>();
    let (b, p, s) = q.single(world).expect("one Sackboy");
    (to_level(p.0), s.z, b.grounded)
}

#[test]
fn stands_walks_and_stops_at_a_wall() {
    let mut app = world(|_| {});
    let (p, _, grounded) = (0..5)
        .map(|_| tick(&mut app, Input::default()))
        .last()
        .unwrap();
    assert!(
        (p[1] - STAND).abs() < 1. && grounded,
        "stands on the floor: {p:?}"
    );
    let right = Input {
        x: 1.,
        ..Default::default()
    };
    let a = tick(&mut app, right).0;
    let mut last = a;
    for _ in 0..12 {
        last = tick(&mut app, right).0;
    }
    let b = tick(&mut app, right).0;
    assert!(
        (b[0] - last[0] - 30.).abs() < 0.01,
        "top speed 30/tick: {last:?} {b:?}"
    );
    assert!((b[1] - STAND).abs() < 1., "stays on the floor: {b:?}");
    for _ in 0..60 {
        last = tick(&mut app, right).0;
    }
    // The wall's left face is at x 950.
    assert!(
        (last[0] + RADIUS - 950.).abs() < 2.,
        "stopped by the wall: {last:?}"
    );
}

#[test]
fn full_jump_peaks_260_above_standing() {
    let mut app = world(|_| {});
    for _ in 0..5 {
        tick(&mut app, Input::default());
    }
    let mut top = f32::MIN;
    for t in 0..40 {
        let (p, ..) = tick(
            &mut app,
            Input {
                jump: t < 20,
                ..Default::default()
            },
        );
        top = top.max(p[1]);
    }
    assert!((top - STAND - 260.1).abs() < 2., "apex {}", top - STAND);
    let (p, _, grounded) = tick(&mut app, Input::default());
    assert!((p[1] - STAND).abs() < 1. && grounded, "landed: {p:?}");
}

#[test]
fn grounded_is_reported_on_the_touchdown_tick() {
    // D247: the tick whose move is stopped by the floor reports contact (the animation's
    // landing starts there); the controller's own landing (dip, lanes) is the tick after.
    let mut app = world(|_| {});
    for _ in 0..5 {
        tick(&mut app, Input::default());
    }
    let mut seen = Vec::new();
    for t in 0..40 {
        let (p, _, grounded) = tick(
            &mut app,
            Input {
                jump: t < 6,
                ..Default::default()
            },
        );
        let world = app.world_mut();
        let mut q = world.query::<&SackboyBody>();
        let dip = q.single(world).expect("one Sackboy").controller.dip_tick();
        seen.push((p[1], grounded, dip));
    }
    let down = (1..seen.len())
        .find(|&t| seen[t].1 && !seen[t - 1].1)
        .expect("lands");
    assert!(
        seen[down - 1].0 > seen[down].0 + 20.,
        "still falling before"
    );
    assert!((seen[down].0 - STAND).abs() < 1., "on the floor: {seen:?}");
    assert_eq!(seen[down].2, None);
    assert_eq!(seen[down + 1].2, Some(0));
    assert_eq!(seen[down + 1].0, seen[down].0);
}

#[test]
fn lane_change_needs_floor_in_the_target_lane() {
    let mut app = world(|_| {});
    for _ in 0..5 {
        tick(&mut app, Input::default());
    }
    // At x -500 lane -200 has floor: W moves back.
    tick(
        &mut app,
        Input {
            back: true,
            ..Default::default()
        },
    );
    let (_, z, _) = (0..10)
        .map(|_| tick(&mut app, Input::default()))
        .last()
        .unwrap();
    assert_eq!(z, -200.);
    // In lane -200 the wall (lane 0) is not in the way; walk right past x 0 where the
    // lane -200 floor ends and fall.
    let right = Input {
        x: 1.,
        ..Default::default()
    };
    let (p, ..) = (0..60).map(|_| tick(&mut app, right)).last().unwrap();
    assert!(p[1] < -100., "fell off the end of the back floor: {p:?}");
}
