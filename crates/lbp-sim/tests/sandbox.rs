//! R2 sandbox on authored synthetic shapes (no game bytes): scene rules, then avian2d runs
//! at 30 Hz. docs/sim-physics.md.
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_level::parts::{
    Body, Joint, JointParams, PhysicsTweak, Pos, Shape, ShapeColor, SwitchKey, TagSynchroniser,
};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_sim::scene::{
    build_scene, build_scene_with, build_spawn_scene_with, Endpoint, JointKind, MaterialRule,
    PhysicsScene, DENSITY,
};
use lbp_sim::{spawn_scene, to_level, SimPlugin, ThingIndex, TICK_HZ};
use std::time::Duration;

/// Column-major matrix: rotation `angle` about z, scale (sx, sy), translation.
fn matrix(x: f32, y: f32, z: f32, angle: f32, sx: f32, sy: f32) -> [f32; 16] {
    let (s, c) = angle.sin_cos();
    let mut m = [0.; 16];
    m[0] = c * sx;
    m[1] = s * sx;
    m[4] = -s * sy;
    m[5] = c * sy;
    m[10] = 1.;
    m[12] = x;
    m[13] = y;
    m[14] = z;
    m[15] = 1.;
    m
}

/// Axis-aligned w×h rectangle centred on the origin, counter-clockwise.
fn rect(w: f32, h: f32, flags: i16) -> Shape {
    let (x, y) = (w / 2., h / 2.);
    Shape {
        vertex_count: 4,
        requires_z: false,
        vertices: vec![[-x, -y, 0.], [x, -y, 0.], [x, y, 0.], [-x, y, 0.]],
        loops: vec![4],
        material: None,
        old_material: None,
        thickness: 90.,
        mass_depth: 1.,
        color: ShapeColor::Packed(0xffff_ffff),
        brightness: 0.,
        bevel_size: 10.,
        com: None,
        behavior: 0,
        flags,
        contacts: 0,
    }
}

fn thing(index: usize, world: [f32; 16], shape: Option<Shape>, body: bool) -> ThingRecord {
    let mut t = ThingRecord {
        index,
        ..Default::default()
    };
    t.data.pos = Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world,
    });
    t.data.shape = shape;
    t.data.body = body.then_some(Body {
        velocity: [0.; 3],
        angular_velocity: 0.,
        frozen: 0,
    });
    t
}

#[test]
fn scene_rules_bodies_static_children_flags_and_frames() {
    let mut things = vec![
        // 0: static floor, 2000 wide, top at y = 0.
        thing(
            0,
            matrix(0., -50., 0., 0., 1., 1.),
            Some(rect(2000., 100., 7)),
            false,
        ),
        // 1: body with its own 100×100 box at (0, 200), rotated 90°.
        thing(
            1,
            matrix(0., 200., 0., std::f32::consts::FRAC_PI_2, 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        // 2: a piece glued under Thing 1 (no BODY), mirrored in x.
        thing(
            2,
            matrix(100., 200., 0., 0., -1., 1.),
            Some(rect(100., 50., 7)),
            false,
        ),
        // 3: not collidable in game.
        thing(
            3,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(10., 10., 2)),
            false,
        ),
        // 4: a BODY Thing without a shape and no POS: its shape child is rejected.
        ThingRecord {
            index: 4,
            ..Default::default()
        },
        thing(
            5,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(10., 10., 7)),
            false,
        ),
    ];
    things[2].parent = Some(1);
    things[4].data.body = things[1].data.body;
    things[5].parent = Some(4);
    let s = build_scene(&things);
    let r = &s.report;
    assert_eq!(
        (r.shapes, r.dynamic, r.static_, r.not_collidable),
        (5, 2, 1, 1)
    );
    assert_eq!(r.rejected.get("body without POS"), Some(&1));
    assert_eq!(s.bodies.len(), 1);
    assert_eq!(s.bodies[0].thing, 1);
    let piece = s.colliders.iter().find(|c| c.thing == 2).unwrap();
    assert_eq!(piece.body, Some(0));
    assert_eq!(piece.frame, s.bodies[0].frame);
    // Every triangle is counter-clockwise, the mirrored piece included, and the pieces
    // keep their area.
    for c in &s.colliders {
        let mut area = 0.;
        for [a, b, t] in &c.triangles {
            let tri = 0.5 * ((b[0] - a[0]) * (t[1] - a[1]) - (b[1] - a[1]) * (t[0] - a[0]));
            assert!(tri > 0., "thing {}", c.thing);
            area += tri;
        }
        let want = match c.thing {
            0 => 200_000.,
            1 => 10_000.,
            _ => 5_000.,
        };
        assert!(
            (area - want).abs() < 1e-2 * want,
            "thing {}: {area}",
            c.thing
        );
        assert_eq!(c.density, DENSITY);
    }
    // The glued piece sits at world x 50..150 = body-local y -150..-50 after the 90° turn.
    let ys: Vec<f32> = piece.triangles.iter().flatten().map(|p| p[1]).collect();
    let (lo, hi) = ys
        .iter()
        .fold((f32::MAX, f32::MIN), |(l, h), &y| (l.min(y), h.max(y)));
    assert!(
        (lo + 150.).abs() < 1e-3 && (hi + 50.).abs() < 1e-3,
        "{lo} {hi}"
    );
}

fn app(scene: PhysicsScene) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, SimPlugin::default()))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1. / TICK_HZ,
        )))
        .add_systems(Startup, move |mut commands: Commands| {
            spawn_scene(&mut commands, &scene);
        });
    app.finish();
    app
}

/// Body centres by Thing index after `ticks` game ticks.
fn run(things: &[ThingRecord], ticks: usize) -> Vec<(usize, [f32; 2], f32)> {
    let mut app = app(build_scene(things));
    for _ in 0..ticks {
        app.update();
    }
    let mut q = app
        .world_mut()
        .query_filtered::<(&ThingIndex, &Position, &Rotation), With<RigidBody>>();
    let mut out: Vec<_> = q
        .iter(app.world())
        .map(|(t, p, r)| (t.0, to_level(p.0), r.as_radians()))
        .collect();
    out.sort_by_key(|o| o.0);
    out
}

fn floor() -> ThingRecord {
    thing(
        0,
        matrix(0., -50., 0., 0., 1., 1.),
        Some(rect(4000., 100., 7)),
        false,
    )
}

#[test]
fn box_falls_onto_the_floor_and_rests() {
    let things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
    ];
    let out = run(&things, 120);
    let (_, p, angle) = out[1];
    assert!((p[1] - 50.).abs() < 5., "rests on top of the floor: {p:?}");
    // The bounce (restitution 0.3) resolves one corner first and slides the box a little:
    // about 6 units here, none with restitution 0.
    assert!(p[0].abs() < 10. && angle.abs() < 0.05, "{p:?} {angle}");
    let floor = out[0].1;
    assert!(
        floor[0].abs() < 1e-3 && (floor[1] + 50.).abs() < 1e-3,
        "the floor does not move"
    );
}

#[test]
fn d1100_an_anti_gravity_tweaker_holds_its_body_up() {
    // Two boxes; box 2 carries an anti-gravity tweaker (Thing 3) as saved in the Rookie Test.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(-300., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(
            2,
            matrix(300., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(3, matrix(300., 300., 0., 0., 1., 1.), None, false),
    ];
    things[3].parent = Some(2);
    things[3].data.physics_tweak = Some(PhysicsTweak {
        gravity: 1.,
        dampening: [1., 1., 0.],
        last_known_activation: 1.,
        ..Default::default()
    });
    let scene = build_scene(&things);
    assert!(scene.bodies[0].anti_gravity.is_none());
    assert_eq!(
        scene.bodies[1].anti_gravity.as_ref().map(|a| a.cancelled),
        Some(1.)
    );
    let out = run(&things, 60);
    assert!(
        (out[1].1[1] - 50.).abs() < 5.,
        "the plain box falls: {:?}",
        out[1]
    );
    assert!(
        (out[2].1[1] - 300.).abs() < 1.,
        "the tweaked box stays: {:?}",
        out[2]
    );
}

#[test]
fn shapes_in_separate_slabs_pass_each_other() {
    // The box is one thick layer behind the floor (slabs 0±90 and 300±90 do not overlap).
    let things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 300., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
    ];
    let out = run(&things, 30);
    assert!(out[1].1[1] < -500., "fell through: {:?}", out[1].1);
}

#[test]
fn stack_of_boxes_stays_put() {
    let mut things = vec![floor()];
    for i in 0..4 {
        let y = 50. + 100. * i as f32;
        things.push(thing(
            i + 1,
            matrix(0., y, 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ));
    }
    let out = run(&things, 300);
    for (i, p, angle) in &out[1..] {
        let want = 50. + 100. * (*i as f32 - 1.);
        assert!(
            (p[1] - want).abs() < 5. && p[0].abs() < 5. && angle.abs() < 0.05,
            "box {i}: {p:?} {angle}"
        );
    }
}

/// A JOINT Thing from `a` (contact `ca` in a's frame) to `b` (contact `cb`).
fn joint(
    index: usize,
    kind: i32,
    a: usize,
    ca: [f32; 3],
    b: usize,
    cb: [f32; 3],
    length: f32,
) -> ThingRecord {
    joint_with(index, kind, a, ca, b, cb, length, JointParams::default())
}

#[allow(clippy::too_many_arguments)]
fn joint_with(
    index: usize,
    kind: i32,
    a: usize,
    ca: [f32; 3],
    b: usize,
    cb: [f32; 3],
    length: f32,
    params: JointParams,
) -> ThingRecord {
    let mut t = ThingRecord {
        index,
        ..Default::default()
    };
    t.data.joint = Some(Joint {
        a: Some(a),
        b: Some(b),
        a_contact: ca,
        b_contact: cb,
        length,
        angle: 0.,
        settings: None,
        bone_idx: vec![],
        bone_lengths: [0.; 4],
        joint_type: kind,
        render_scale: 1.,
        hide_in_play_mode: false,
        params,
    });
    t
}

/// Static ceiling block at (0, 1000).
fn ceiling() -> ThingRecord {
    thing(
        0,
        matrix(0., 1000., 0., 0., 1., 1.),
        Some(rect(200., 50., 7)),
        false,
    )
}

#[test]
fn joint_scene_rules() {
    let mut things = vec![
        ceiling(),
        thing(
            1,
            matrix(300., 1000., 0., std::f32::consts::FRAC_PI_2, 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        thing(
            2,
            matrix(0., 500., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        thing(
            3,
            matrix(0., 500., 0., 0., 1., 1.),
            Some(rect(10., 10., 7)),
            false,
        ),
        // 4: rod from the ceiling centre to body 1, contact 10 units along body 1's local x.
        joint(4, 6, 0, [0.; 3], 1, [10., 0., 0.], 0.),
        // 5: bolt between two bodies.
        joint(5, 7, 1, [0.; 3], 2, [0.; 3], 0.),
        // 6: both ends on body 2 (Thing 3 is glued under it).
        joint(6, 7, 2, [0.; 3], 3, [0.; 3], 0.),
        // 7: legacy.
        joint(7, 0, 1, [0.; 3], 2, [0.; 3], 0.),
    ];
    things[3].parent = Some(2);
    let s = build_scene(&things);
    assert_eq!(s.report.joints, 2);
    assert_eq!(
        s.report.rejected_joints.get("both ends on one body"),
        Some(&1)
    );
    assert_eq!(
        s.report.rejected_joints.get("legacy or unknown type"),
        Some(&1)
    );
    let rod = &s.joints[0];
    assert_eq!(
        (rod.kind, rod.a, rod.b),
        (JointKind::Rod, Endpoint::Static(0), Endpoint::Body(0))
    );
    // Body 1 is turned 90°: its local x points along world +y.
    let close = |p: [f32; 2], q: [f32; 2]| (p[0] - q[0]).abs() < 1e-3 && (p[1] - q[1]).abs() < 1e-3;
    assert!(close(rod.world_b, [300., 1010.]), "{:?}", rod.world_b);
    assert!(close(rod.local_b, [10., 0.]), "{:?}", rod.local_b);
    assert!((rod.length - (300f32.powi(2) + 10f32.powi(2)).sqrt()).abs() < 1e-2);
    assert_eq!(s.joints[1].kind, JointKind::Bolt);
}

fn distance(p: [f32; 2], q: [f32; 2]) -> f32 {
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
}

#[test]
fn rod_pendulum_keeps_its_length() {
    let things = vec![
        ceiling(),
        thing(
            1,
            matrix(300., 1000., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint(2, 6, 0, [0.; 3], 1, [0.; 3], 0.),
    ];
    let out = run(&things, 20);
    let p = out[1].1;
    assert!((distance(p, [0., 1000.]) - 300.).abs() < 5., "{p:?}");
    assert!(p[1] < 900., "swung down: {p:?}");
}

#[test]
fn string_stops_a_falling_box_at_its_length() {
    let things = vec![
        ceiling(),
        thing(
            1,
            matrix(0., 900., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint(2, 5, 0, [0.; 3], 1, [0.; 3], 300.),
    ];
    let out = run(&things, 90);
    let p = out[1].1;
    assert!((p[1] - 700.).abs() < 10. && p[0].abs() < 5., "{p:?}");
}

#[test]
fn motor_turns_a_bolted_wheel_in_place() {
    // D1108: pattern 1 turns at speed (rad per frame) × 30 ticks a second: 0.1 is 3 rad/s.
    let params = JointParams {
        animation_pattern: 1,
        strength: 1.,
        motion: [0., 0., 0., 0.1, 0., 0., 0., 0., 0., 0.],
        ..Default::default()
    };
    let things = vec![
        thing(
            0,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(20., 20., 7)),
            false,
        ),
        thing(
            1,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        joint_with(2, 9, 0, [0.; 3], 1, [0.; 3], 0., params),
    ];
    let mut app = app(build_scene(&things));
    for _ in 0..30 {
        app.update();
    }
    let mut q = app
        .world_mut()
        .query::<(&ThingIndex, &Position, &AngularVelocity, Option<&RigidBody>)>();
    let (_, p, w, _) = q.iter(app.world()).find(|(t, ..)| t.0 == 1).expect("wheel");
    assert!(distance(to_level(p.0), [0., 0.]) < 2., "{p:?}");
    assert!(
        (w.0 - 3.).abs() < 0.5,
        "turning at the motor speed: {}",
        w.0
    );
}

#[test]
fn piston_slides_to_its_limit_and_holds_its_line() {
    let params = JointParams {
        tweak_target_min_length: 100.,
        tweak_target_max_length: 200.,
        ..Default::default()
    };
    let things = vec![
        ceiling(),
        thing(
            1,
            matrix(0., 850., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint_with(2, 4, 0, [0.; 3], 1, [0.; 3], 150., params),
    ];
    let out = run(&things, 60);
    let p = out[1].1;
    assert!((p[1] - 800.).abs() < 5. && p[0].abs() < 2., "{p:?}");
    assert!(out[1].2.abs() < 0.01, "does not turn: {}", out[1].2);
}

#[test]
fn dark_matter_and_world_bodies_are_static() {
    const DARK: u32 = 0xd4;
    let mut dark = rect(200., 100., 7);
    dark.material = Some(Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(DARK),
        sha1: None,
    });
    // 0: dark matter block in mid air; 1: box glued to it (its own BODY merges into 0);
    // 2: a loose box; 3: a box owned by the WORLD Thing.
    let mut glued = thing(
        1,
        matrix(150., 500., 0., 0., 1., 1.),
        Some(rect(100., 100., 7)),
        true,
    );
    glued.parent = Some(0);
    let mut world = thing(
        3,
        matrix(-600., 500., 0., 0., 1., 1.),
        Some(rect(100., 100., 7)),
        true,
    );
    world.parts = vec!["WORLD"];
    let things = vec![
        thing(0, matrix(0., 500., 0., 0., 1., 1.), Some(dark), true),
        glued,
        thing(
            2,
            matrix(600., 500., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        world,
        // 4: a bolt between the two fixed bodies (avian panics on a joint with no dynamic
        // end, so it must be rejected).
        joint_with(4, 7, 0, [0.; 3], 3, [0.; 3], 0., JointParams::default()),
    ];
    let scene = build_scene_with(&things, &|d| {
        if d.guid == Some(DARK) {
            MaterialRule::Fixed
        } else {
            MaterialRule::Normal
        }
    });
    let r = &scene.report;
    assert_eq!(
        (r.dynamic, r.static_, r.fixed_bodies, r.nested_bodies),
        (1, 3, 2, 1)
    );
    assert_eq!(
        scene.report.rejected_joints.get("both ends static"),
        Some(&1)
    );
    assert_eq!(build_scene(&things).report.fixed_bodies, 1, "WORLD only");
    let mut app = app(scene);
    for _ in 0..30 {
        app.update();
    }
    let mut q = app
        .world_mut()
        .query_filtered::<(&ThingIndex, &Position), With<RigidBody>>();
    let mut out: Vec<_> = q
        .iter(app.world())
        .map(|(t, p)| (t.0, to_level(p.0)))
        .collect();
    out.sort_by_key(|o| o.0);
    let near = |p: [f32; 2], q: [f32; 2]| (p[0] - q[0]).abs() < 0.01 && (p[1] - q[1]).abs() < 0.01;
    assert!(
        out[0].0 == 0 && near(out[0].1, [0., 500.]),
        "dark matter (and the glued box) stay: {out:?}"
    );
    assert!(
        out[1].0 == 2 && out[1].1[1] < 400.,
        "loose box falls: {out:?}"
    );
    assert!(
        out[2].0 == 3 && near(out[2].1, [-600., 500.]),
        "WORLD body stays: {out:?}"
    );
}

#[test]
fn frozen_bodies_sleep_until_touched_and_pickups_hang() {
    let asleep = |index, y| {
        let mut t = thing(
            index,
            matrix(0., y, 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        );
        t.data.body.as_mut().unwrap().frozen = 1;
        t
    };
    let mut pickup = asleep(3, 300.);
    pickup.data.pos.as_mut().unwrap().world = matrix(600., 300., 0., 0., 1., 1.);
    pickup.parts = vec!["GAMEPLAY_DATA"];
    // 1: asleep in mid air; 2: awake, dropped onto it from above; 3: a score bubble.
    let things = vec![floor(), asleep(1, 300.), asleep(2, 700.), pickup];
    let mut things = things;
    things[2].data.body.as_mut().unwrap().frozen = 0;
    let before = run(&things[..2], 60);
    assert!(
        (before[1].1[1] - 300.).abs() < 0.01,
        "asleep stays: {before:?}"
    );
    let after = run(&things, 120);
    assert!(after[1].1[1] < 200., "woken by the falling box: {after:?}");
    assert!(
        (after[3].1[1] - 300.).abs() < 0.01 && (after[3].1[0] - 600.).abs() < 0.01,
        "pick-up hangs: {after:?}"
    );
}

#[test]
fn no_collide_material_is_not_a_collider() {
    const HOLO: u32 = 0x40;
    let mut holo = rect(100., 100., 7);
    holo.material = Some(Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(HOLO),
        sha1: None,
    });
    let things = vec![
        floor(),
        thing(1, matrix(0., 300., 0., 0., 1., 1.), Some(holo), true),
    ];
    let s = build_scene_with(&things, &|d| {
        if d.guid == Some(HOLO) {
            MaterialRule::NoCollide
        } else {
            MaterialRule::Normal
        }
    });
    assert_eq!(
        (s.report.not_collidable, s.colliders.len(), s.bodies.len()),
        (1, 1, 0)
    );
}

#[test]
fn crossing_outline_is_filled_and_holds_a_box() {
    // A floor whose second loop crosses its bottom edge: refused by the strict outline,
    // kept by the even-odd fill, and the box still lands on its top.
    let mut shape = rect(4000., 100., 7);
    let notch = [[1400., -70.], [1600., -70.], [1600., -30.], [1400., -30.]];
    shape.vertices.extend(notch.map(|p| [p[0], p[1], 0.]));
    shape.vertex_count = 8;
    shape.loops.push(4);
    let mut floor = floor();
    floor.data.shape = Some(shape);
    let things = vec![
        floor,
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
    ];
    let report = build_scene(&things).report;
    assert_eq!(report.filled.get("loops_intersect"), Some(&1));
    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let p = run(&things, 120)[1].1;
    assert!((p[1] - 50.).abs() < 5., "rests on top of the floor: {p:?}");
}

#[test]
fn string_from_a_thing_without_a_collider_holds_a_box() {
    // The top end is a Thing with a POS but no shape (as a hologram or decoration): the
    // joint anchors it in the world instead of being dropped.
    let things = vec![
        thing(0, matrix(0., 1000., 0., 0., 1., 1.), None, false),
        thing(
            1,
            matrix(0., 900., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint(2, 5, 0, [0.; 3], 1, [0.; 3], 300.),
    ];
    let scene = build_scene(&things);
    assert_eq!(
        (scene.report.joints, scene.report.anchored_joint_ends),
        (1, 1)
    );
    let out = run(&things, 90);
    let p = out[1].1;
    assert!((p[1] - 700.).abs() < 10. && p[0].abs() < 5., "{p:?}");
}

#[test]
fn tilted_shape_with_vertex_z_collides_in_its_projected_outline() {
    // The Pod body's layout (shell 000027b9 Thing 2): turned so local z is world up and
    // local y is world -z; the outline lives in local x/z. Here a 400 x 200 floor block
    // whose top is 100 above its POS (y -400), so the top is at y -300.
    let mut shape = rect(400., 10., 1);
    shape.requires_z = true;
    shape.vertices = vec![
        [-200., 0., -100.],
        [200., 0., -100.],
        [200., 0., 100.],
        [-200., 0., 100.],
    ];
    let mut m = [0.; 16];
    m[0] = 1.;
    m[6] = -1.;
    m[9] = 1.;
    m[12] = 0.;
    m[13] = -400.;
    m[14] = 0.;
    m[15] = 1.;
    let things = [thing(0, m, Some(shape), false)];
    let scene = build_spawn_scene_with(&things, &|_| MaterialRule::Normal);
    assert_eq!(scene.colliders.len(), 1);
    let top = scene.floor_below([-5., 500.], 0.).unwrap();
    assert!((top + 300.).abs() < 1e-3, "top {top}");
    // Below the block's bottom (y -500) there is nothing.
    assert_eq!(scene.floor_below([-5., -501.], 0.), None);
    // The simulated scene keeps the in-plane outline: no floor at the projected top.
    let plain = build_scene(&things);
    assert_ne!(
        plain
            .floor_below([-5., 500.], 0.)
            .map(|t| (t + 300.).abs() < 1e-3),
        Some(true)
    );
}

/// D1101: a tweak switched off gives its body gravity back; analog activations add up.
#[test]
fn d1101_a_switched_off_anti_gravity_tweaker_lets_its_body_fall() {
    let tweak = |thing, gravity| lbp_sim::scene::AntiGravityTweak {
        thing,
        gravity,
        behavior: 0,
        saved: 1.,
    };
    let a = lbp_sim::scene::AntiGravity {
        cancelled: 1.5,
        dampening: [0.; 3],
        tweaks: vec![tweak(3, 1.), tweak(4, 0.5)],
    };
    assert_eq!(a.gravity_scale(|t| t.saved), 0.);
    assert_eq!(a.gravity_scale(|_| 0.), 1.);
    assert_eq!(
        a.gravity_scale(|t| if t.thing == 3 { 0.25 } else { 1. }),
        0.25
    );
}

#[test]
fn d1102_an_unwired_piston_moves_out_and_back_on_its_wave() {
    use lbp_sim::scene::{piston_wave, SceneJoint};
    // time 120, pause 20: hold 10, rise 60, hold 10, fall 60.
    let w = |x| piston_wave(x, 120., 20.);
    assert_eq!(
        (w(0.), w(10.), w(40.), w(75.), w(110.), w(140.)),
        (0., 0., 0.5, 1., 0.5, 0.)
    );
    assert_eq!(piston_wave(5., 0.5, 0.), 0.);
    let mut params = JointParams::default();
    params.motion[..5].copy_from_slice(&[315., 120., 0., 0.02, 20.]);
    let mut j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 840.,
        min_length: 840.,
        max_length: 840.,
        params,
    };
    // Eased: half way along the rise is half the range; the top is base + range.
    assert_eq!((j.free_length(0.), j.free_length(70.)), (840., 1155.));
    assert!((j.free_length(40.) - 997.5).abs() < 1e-3);
    j.params.animation_pattern = 2;
    assert_eq!(
        (j.free_length(0.), j.free_length(60.), j.free_length(130.)),
        (1155., 997.5, 840.)
    );
    j.params.animation_pattern = 7;
    assert_eq!(j.free_length(40.), 840.);
}

#[test]
fn d1103_a_switched_piston_clock_freezes_while_off_and_never_jumps() {
    use lbp_sim::scene::PistonClock;
    let mut c = PistonClock::default();
    assert_eq!(c.frame(30.), 30.);
    // Off at 30: the frame holds; on again at 50 it carries on from there.
    c.set_scale(30., 0.);
    assert_eq!((c.frame(30.), c.frame(50.)), (30., 30.));
    c.set_scale(50., 1.);
    assert_eq!((c.frame(50.), c.frame(60.)), (30., 40.));
    // Running backwards from 60 returns along the same frames.
    c.set_scale(60., -1.);
    assert_eq!((c.frame(60.), c.frame(70.)), (40., 30.));
}

#[test]
fn d1104_a_direction_piston_ramps_out_while_on_and_back_while_off() {
    use lbp_sim::scene::{piston_scale, Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 1,
        ..Default::default()
    };
    params.motion[..2].copy_from_slice(&[300., 60.]);
    let j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 500.,
        min_length: 500.,
        max_length: 500.,
        params,
    };
    // Rate by mode: on 1, a negative signal -1, off -1 (mode 1); mode 0 holds while off.
    assert_eq!(
        (
            piston_scale(1, false, 1, 0., false),
            piston_scale(1, false, -1, 0., false)
        ),
        (1., -1.)
    );
    assert_eq!(piston_scale(1, false, 0, 0., false), -1.);
    // D1170: a signed source (a three-way switch at 0) stops the piston instead.
    assert_eq!(piston_scale(1, false, 0, 0., true), 0.);
    assert_eq!(
        (
            piston_scale(0, false, 0, 0., false),
            piston_scale(1, true, 1, 0., false)
        ),
        (0., 0.)
    );
    assert_eq!(piston_scale(3, false, 1, 0.25, false), 0.25);
    // The ramp: base + 2 × frame × range / time; D1105: the length is held between base
    // and base + range while the clock runs on.
    let mut c = PistonClock::default();
    let at = |c: &mut PistonClock, now: f32, s: f32| {
        c.set_scale(now, s);
        j.length(c.frame(now), true)
    };
    // Off from the start: held at the base while the clock runs below 0.
    assert_eq!((at(&mut c, 0., -1.), at(&mut c, 10., -1.)), (500., 500.));
    // On at 10 (frame -10): still at the base at 20, half way at 35, out from 50 on.
    assert_eq!((at(&mut c, 10., 1.), at(&mut c, 20., 1.)), (500., 500.));
    assert_eq!(at(&mut c, 35., 1.), 650.);
    assert_eq!((at(&mut c, 50., 1.), at(&mut c, 90., 1.)), (800., 800.));
    // Off at 90 (frame 70): out until frame 30 (tick 130), at the base by tick 160.
    assert_eq!((at(&mut c, 90., -1.), at(&mut c, 130., -1.)), (800., 800.));
    assert_eq!((at(&mut c, 145., -1.), at(&mut c, 160., -1.)), (650., 500.));
    // Unwired, the same joint follows its free-running wave instead.
    assert_eq!(j.length(15., false), j.free_length(15.));
}

/// D1106: a speed piston (mode 4) servos its length to the analog signal's fraction of the
/// way from base to base + range, at full rate until within a tenth (0xc9e80).
#[test]
fn d1106_a_speed_piston_settles_at_its_signal_fraction() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 4,
        ..Default::default()
    };
    params.motion[..2].copy_from_slice(&[300., 60.]);
    let mut j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 500.,
        min_length: 500.,
        max_length: 500.,
        params,
    };
    // Rate: full out below the target, full back above it, 10 × the error near it.
    assert_eq!(
        (j.servo_scale(0.5, 500.), j.servo_scale(0., 800.)),
        (1., -1.)
    );
    assert!((j.servo_scale(0.5, 620.) - 1.).abs() < 1e-5);
    assert!((j.servo_scale(-1., 530.) + 1.).abs() < 1e-5);
    // Driven on the clock, a signal of 0.5 settles half way out (650).
    let mut c = PistonClock::default();
    c.set_scale(0., 0.);
    let mut l = 500.;
    for now in 0..400 {
        let now = now as f32;
        c.set_scale(now, j.servo_scale(0.5, l));
        l = j.length(c.frame(now), true);
    }
    assert!((l - 650.).abs() < 1., "{l}");
    // A negative range flips the target: signal 0 holds the piston at its base.
    j.params.motion[0] = -300.;
    assert_eq!(j.servo_scale(0., 500.), 0.);
}

#[test]
fn d1107_a_winch_follows_the_piston_rule() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 1,
        ..Default::default()
    };
    params.motion[..2].copy_from_slice(&[-200., 60.]);
    let mut d = lbp_sim::PistonDrive {
        joint: SceneJoint {
            thing: 0,
            kind: JointKind::Chain,
            a: Endpoint::Static(0),
            b: Endpoint::Static(1),
            world_a: [0.; 2],
            world_b: [0.; 2],
            local_a: [0.; 2],
            local_b: [0.; 2],
            length: 400.,
            min_length: 400.,
            max_length: 400.,
            params,
        },
        clock: PistonClock::default(),
        sync: None,
    };
    let u = lbp_sim::UNIT_SCALE;
    // Switched on (direction mode), it winds in over half its time and stops at 200.
    assert!((d.step(15., Some((1, 1., false))) / u - 300.).abs() < 1e-3);
    assert!((d.step(60., Some((1, 1., false))) / u - 200.).abs() < 1e-3);
    // Switched off, the clock (run on to 200 meanwhile) counts back down and the rope
    // pays out to its base length.
    assert!((d.step(200., Some((0, 0., false))) / u - 200.).abs() < 1e-3);
    assert!((d.step(400., Some((0, 0., false))) / u - 400.).abs() < 1e-3);
}

#[test]
fn d1108_a_motor_turns_at_its_speed_on_its_clock() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 0,
        animation_pattern: 1,
        strength: 1.,
        ..Default::default()
    };
    params.motion[3] = 0.05;
    let mut d = lbp_sim::PistonDrive {
        joint: SceneJoint {
            thing: 0,
            kind: JointKind::Motor,
            a: Endpoint::Static(0),
            b: Endpoint::Static(1),
            world_a: [0.; 2],
            world_b: [0.; 2],
            local_a: [0.; 2],
            local_b: [0.; 2],
            length: 0.,
            min_length: 0.,
            max_length: 0.,
            params,
        },
        clock: PistonClock::default(),
        sync: None,
    };
    let hz = lbp_sim::TICK_HZ as f32;
    // Free running: 0.05 rad per frame, one frame a tick.
    assert!((d.turn(10., None) - 0.05 * hz).abs() < 1e-3);
    // Switched (mode 0, on/off): it turns while on and stops when off.
    assert!((d.turn(20., Some((1, 1., false))) - 0.05 * hz).abs() < 1e-3);
    assert!(d.turn(30., Some((0, 0., false))).abs() < 1e-6);
}

#[test]
fn d1171_a_motor_off_its_rate_path_never_runs_past_its_target() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    // Pattern 0 and free running, as the Rookie Test's motor saves its timing: range π,
    // time 180, pause 30.
    let mut params = JointParams {
        mode: 1,
        animation_pattern: 0,
        strength: 1.,
        ..Default::default()
    };
    params.motion[0] = std::f32::consts::PI;
    params.motion[1] = 180.;
    params.motion[4] = 30.;
    let joint = SceneJoint {
        thing: 0,
        kind: JointKind::Motor,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 0.,
        min_length: 0.,
        max_length: 0.,
        params,
    };
    let drive = || lbp_sim::PistonDrive {
        joint: joint.clone(),
        clock: PistonClock::default(),
        sync: None,
    };
    let a = |f: f32| joint.motor_angle(f, false);
    let (mut d, mut free) = (drive(), drive());
    // The first run syncs the angle rule to the measure (here 0.5 rad).
    let w = free.turn(40., None);
    assert!(w > 0.);
    assert!((d.turn_to(40., None, 0.5) - w).abs() < 1e-4);
    // On track one tick later, it turns at the rule's rate.
    let w = free.turn(41., None);
    let on = 0.5 + a(41.) - a(40.);
    assert!((d.turn_to(41., None, on) - w).abs() < 1e-4);
    // Halfway to the next frame's angle already, it turns only the rest of the way.
    let half = 0.5 + a(42.) - a(40.) + 0.5 * (a(43.) - a(42.));
    let w = free.turn(42., None);
    assert!((d.turn_to(42., None, half) - 0.5 * w).abs() < 1e-3);
    // Past it, it stops rather than turning back.
    free.turn(43., None);
    assert_eq!(d.turn_to(43., None, 0.5 + a(45.) - a(40.)), 0.);
    // Pattern 1, or wired in mode 1 or 4, is driven at its rate whatever the measure.
    let mut wired = drive();
    let w = drive().turn(40., Some((1, 1., false)));
    assert!((wired.turn_to(40., Some((1, 1., false)), 3.) - w).abs() < 1e-6);
    assert!(wired.sync.is_none());
}

#[test]
fn d1172_a_wired_speed_motor_in_mode_2_stops_after_one_turn() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 2,
        animation_pattern: 1,
        strength: 1.,
        ..Default::default()
    };
    params.motion[3] = -0.1;
    let joint = SceneJoint {
        thing: 0,
        kind: JointKind::Motor,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 0.,
        min_length: 0.,
        max_length: 0.,
        params,
    };
    let mut d = lbp_sim::PistonDrive {
        joint: joint.clone(),
        clock: PistonClock::default(),
        sync: None,
    };
    let hz = lbp_sim::TICK_HZ as f32;
    let on = Some((1, 1., false));
    // 0.1 rad a frame: it turns until frame 62 (6.2 rad), and stops at 63 (past 2π).
    assert!((d.turn(62., on) + 0.1 * hz).abs() < 1e-3);
    assert_eq!(d.turn(63., on), 0.);
    // Free running, or wired in another mode, it keeps turning.
    let mut free = lbp_sim::PistonDrive {
        joint,
        clock: PistonClock::default(),
        sync: None,
    };
    assert!((free.turn(100., None) + 0.1 * hz).abs() < 1e-3);
    d.joint.params.mode = 1;
    assert!((d.turn(100., on) + 0.1 * hz).abs() < 1e-3);
}

#[test]
fn d1109_gyroscopes_right_their_body_and_rotators_turn_at_their_speed() {
    // A box carrying a gyroscope (Thing 2) as saved in the Rookie Test: range π/15 per
    // tick, strength 1.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 1,
        vel_range: 0.2094394,
        accel_strength: 1.,
        ..Default::default()
    });
    let scene = build_scene(&things);
    let gyro = scene.bodies[0].spins[0];
    // Tilted by 1 rad and still: it turns back, no faster than its range per tick.
    let (mut angle, mut w) = (1f32, 0f32);
    for _ in 0..120 {
        w = gyro.turn(angle, w, 1.);
        assert!(w.abs() <= 0.2094394 + 1e-6, "speed {w}");
        angle += w;
    }
    assert!(angle.abs() < 0.05, "upright again: {angle}");
    // Switched off, it leaves the body alone.
    assert_eq!(gyro.turn(1., 0.3, 0.), 0.3);
    // A rotator (configuration 9) speeds up by accel × range per tick to its speed.
    let rot = lbp_sim::scene::SpinTweak {
        thing: 2,
        configuration: 9,
        behavior: 0,
        range: 0.20943,
        accel: 0.125,
        decel: 0.125,
        speed: 0.03490659,
        ..Default::default()
    };
    let w = rot.turn(0., 0., 1.);
    assert!((w - 0.20943 * 0.125).abs() < 1e-6, "{w}");
    assert!((rot.turn(0., w, 1.) - 0.03490659).abs() < 1e-6);
}

#[test]
fn d1144_aim_tweaks_turn_up_along_velocity_stick_and_target() {
    use lbp_sim::scene::SpinTweak;
    let tweak = |configuration| SpinTweak {
        configuration,
        range: 0.2094394,
        accel: 1.,
        min_range: 10.,
        max_range: 500.,
        angle_range: 180.,
        ..Default::default()
    };
    // Configuration 3 aims along the velocity: moving right, an upright body turns
    // clockwise (negative), moving left anticlockwise, upright when moving up.
    let along = tweak(3);
    let right = along.aim([5., 0.], None);
    assert!(along.turn_along(0., 0., 1., right) < 0.);
    assert!(along.turn_along(0., 0., 1., along.aim([-5., 0.], None)) > 0.);
    assert_eq!(along.target_along(0., 0., along.aim([0., 5.], None)).0, 0.);
    // Slower than the dead zone (x² + y² under 0.1) it wants no turn and brakes.
    let slow = along.aim([0.3, 0.], None);
    assert_eq!(along.target_along(0., 0., slow).0, 0.);
    let w = along.turn_along(0., 0.5, 1., slow);
    assert!((w - (0.5 - 0.2094394)).abs() < 1e-6, "{w}");
    // Configuration 4 sees no stick here, so it brakes too.
    let stick = tweak(4);
    assert_eq!(stick.aim([5., 0.], Some([100., 0.])), [0., 0.]);
    // Configuration 5 aims at its target, and turns nothing towards none.
    let seek = tweak(5);
    assert!(seek.turn_along(0., 0., 1., seek.aim([0., 0.], Some([100., 0.]))) < 0.);
    assert_eq!(seek.target_along(0., 0., seek.aim([0., 0.], None)).0, 0.);
    // Its search is the follower's: inside the distance band only.
    assert!(seek.sees([100., 0.], [0., 1.]));
    assert!(!seek.sees([5., 0.], [0., 1.]));
    assert!(!seek.sees([600., 0.], [0., 1.]));
    assert!(!along.sees([100., 0.], [0., 1.]));
    // The gyroscope still aims at world up.
    assert_eq!(tweak(1).aim([5., 0.], None), [0., 1.]);
    // Built from a level: configurations 3, 4 and 5 now ride their body.
    for c in [3, 4, 5] {
        let mut things = vec![
            floor(),
            thing(
                1,
                matrix(0., 300., 0., 0., 1., 1.),
                Some(rect(100., 100., 7)),
                true,
            ),
            thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
        ];
        things[2].parent = Some(1);
        things[2].data.physics_tweak = Some(PhysicsTweak {
            configuration: c,
            vel_range: 0.2094394,
            accel_strength: 1.,
            ..Default::default()
        });
        assert_eq!(build_scene(&things).bodies[0].spins[0].configuration, c);
    }
}

#[test]
fn d1146_recorded_moves_play_by_activation_and_restart_or_pause_when_off() {
    use lbp_level::parts::RecordingPlayer;
    use lbp_sim::scene::PlaybackTweak;
    let tweak = |behavior, dir| PlaybackTweak {
        behavior,
        player: RecordingPlayer {
            speed: 1.,
            dir,
            ..Default::default()
        },
        ..Default::default()
    };
    // On, the head moves by activation × speed from a fresh start.
    let mut digital = tweak(0, 0);
    digital.pos = 7.;
    assert!(digital.tick(1.));
    assert_eq!(digital.pos, 1.);
    assert!(digital.tick(1.));
    assert_eq!(digital.pos, 2.);
    // Off, a digital input restarts next time; an analog one pauses where it was.
    assert!(!digital.tick(0.));
    digital.tick(1.);
    assert_eq!(digital.pos, 1.);
    let mut analog = tweak(1, 0);
    analog.tick(0.5);
    analog.tick(0.5);
    assert!(!analog.tick(0.0005));
    analog.tick(0.5);
    assert_eq!(analog.pos, 1.5);
    // Forward holds the last frame, backward runs from it, back and forth turns.
    let mut p = tweak(0, 0);
    p.pos = 12.;
    assert_eq!(p.frame(5), 4.);
    p.player.dir = 1;
    p.pos = 1.;
    assert_eq!(p.frame(5), 3.);
    p.player.dir = 2;
    p.pos = 6.;
    assert_eq!(p.frame(5), 2.);
    // Frames are offsets from the start pose, linear between neighbours.
    let frames = [[0., 0., 0.], [10., 0., 0.], [10., 20., 0.]];
    assert_eq!(PlaybackTweak::sample(&frames, 0.5), [5., 0., 0.]);
    assert_eq!(PlaybackTweak::sample(&frames, 1.25), [10., 5., 0.]);
    assert_eq!(PlaybackTweak::sample(&frames, 9.), [10., 20., 0.]);
    assert_eq!(PlaybackTweak::sample(&[], 1.), [0.; 3]);
    // Built from a level: configuration 13 rides its body with its saved player.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 13,
        behavior: 1,
        recording: Some(RecordingPlayer {
            speed: 0.5,
            dir: 2,
            ..Default::default()
        }),
        ..Default::default()
    });
    let scene = build_scene(&things);
    let b = &scene.bodies[0];
    assert_eq!(b.playbacks[0].player.dir, 2);
    assert_eq!(b.playbacks[0].behavior, 1);
    assert!(b.layers.is_empty() && b.spins.is_empty() && b.moves.is_empty());
}

#[test]
fn d1145_layer_switches_step_a_layer_or_go_to_a_set_one_when_there_is_room() {
    use lbp_sim::scene::LayerTweak;
    let tweak = |z_behavior| LayerTweak {
        z_behavior,
        ..Default::default()
    };
    let room = |_: f32| true;
    // Back (0) and front (1) step one layer on the input's rising edge, once.
    let mut back = tweak(0);
    assert_eq!(back.tick(1., 1, 0., room), -200.);
    assert_eq!(back.tick(1., 1, -200., room), 0.);
    let mut front = tweak(1);
    assert_eq!(front.tick(1., 1, -200., room), 200.);
    // Turning off moves nothing back.
    assert_eq!(front.tick(0., 1, 0., room), 0.);
    // Out of the depth band (-511 to 110) the step is dropped.
    let mut front = tweak(1);
    assert_eq!(front.tick(1., 1, 0., room), 0.);
    assert!(!front.waiting);
    // Set layers 2..4 go to layers 1..3 from the back: z -479 is layer 1, -279 layer 2.
    assert_eq!(tweak(2).delta(-479., 1), 0);
    assert_eq!(tweak(3).delta(-479., 1), 1);
    assert_eq!(tweak(4).delta(-479., 1), 2);
    assert_eq!(tweak(2).delta(-79., 1), -2);
    let mut deep = tweak(4);
    assert_eq!(deep.tick(1., 1, -479., room), 400.);
    // Blocked, a set-layer move waits again and tries one layer less.
    let mut deep = tweak(4);
    assert_eq!(deep.tick(1., 1, -479., |dz| dz < 300.), 200.);
    let mut stuck = tweak(4);
    assert_eq!(stuck.tick(1., 1, -479., |_| false), 0.);
    assert!(stuck.waiting);
    assert_eq!(stuck.tick(1., 1, -479., room), 400.);
    // A zero count clears the wait and moves nothing.
    let mut none = tweak(1);
    assert_eq!(none.tick(1., 0, -200., room), 0.);
    assert!(!none.waiting);
    // Built from a level: configuration 12 rides its body as a layer switch.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 12,
        z_behavior: 3,
        waiting_to_move: true,
        ..Default::default()
    });
    let scene = build_scene(&things);
    assert_eq!(scene.bodies[0].layers[0].z_behavior, 3);
    assert!(scene.bodies[0].layers[0].waiting);
    assert!(scene.bodies[0].spins.is_empty() && scene.bodies[0].moves.is_empty());
}

#[test]
fn d1110_movers_reach_their_velocity_and_followers_stop_short_of_their_target() {
    // A box carrying a mover (Thing 2) as saved in the Rookie Test: 35 up per tick.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 8,
        middle_vel: [0., 35., 0.],
        vel_range: 28.,
        accel_strength: 0.125,
        decel_strength: 0.125,
        ..Default::default()
    });
    let scene = build_scene(&things);
    let mover = scene.bodies[0].moves[0];
    let up = [0., 1.];
    // From a sideways drift it gains accel Ã— range (3.5) a tick up and loses the drift.
    let v = mover.push([2., 0.], up, None, 1.);
    assert!(v[0].abs() < 1e-6 && (v[1] - 3.5).abs() < 1e-5, "{v:?}");
    let mut v = [0f32, 0.];
    for _ in 0..12 {
        v = mover.push(v, up, None, 1.);
    }
    assert!((v[1] - 35.).abs() < 1e-4, "{v:?}");
    assert_eq!(mover.push([1., 2.], up, None, 0.), [1., 2.]);
    // A follower (configuration 11) with the census's numbers walks to 100 short of its
    // target, never faster than its range, and ignores a target past max_range.
    let follower = lbp_sim::scene::MoveTweak {
        configuration: 11,
        min_range: 100.,
        max_range: 2100.,
        angle_range: 360.,
        up_down: true,
        vel: [0., 0.],
        ..mover
    };
    let target = [1000f32, 0.];
    let (mut x, mut v) = (0f32, [0f32, 0.]);
    for _ in 0..200 {
        let d = [target[0] - x, 0.];
        let d = follower.sees(d, up).then_some(d);
        v = follower.push(v, up, d, 1.);
        assert!(v[0].abs() <= 28. + 1e-4, "speed {v:?}");
        x += v[0];
    }
    assert!((895. ..920.).contains(&x), "stopped at {x}");
    assert!(!follower.sees([3000., 0.], up) && !follower.sees([50., 0.], up));
}

#[test]
fn d1111_keyed_followers_find_tags_with_their_colour_and_name() {
    // A keyed follower box (Things 1, 2), a red "A" tag on a second box turned a quarter
    // (Things 3, 4), a red "A" tag on the follower's own box (5), a loose red "A" tag (6),
    // a blue "A" tag (7) and a red "A" tag with no position of its own under Thing 4 (8).
    let tag = |colour: i32| SwitchKey {
        colour_index: colour,
        name: "A".into(),
    };
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
        thing(
            3,
            matrix(800., 300., 0., quarter, 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(4, matrix(800., 340., 0., 0., 1., 1.), None, false),
        thing(5, matrix(10., 300., 0., 0., 1., 1.), None, false),
        thing(6, matrix(-500., 300., 0., 0., 1., 1.), None, false),
        thing(7, matrix(300., 300., 0., 0., 1., 1.), None, false),
        thing(8, matrix(0., 0., 0., 0., 1., 1.), None, false),
    ];
    things[8].data.pos = None;
    for (t, p) in [(2, 1), (4, 3), (5, 1), (8, 4)] {
        things[t].parent = Some(p);
    }
    for (t, c) in [(4, 1), (5, 1), (6, 1), (7, 2), (8, 1)] {
        things[t].data.switch_key = Some(tag(c));
    }
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 11,
        color_index: 1,
        name: "A".into(),
        follow_key: true,
        vel_range: 28.,
        accel_strength: 0.125,
        max_range: 2100.,
        angle_range: 360.,
        ..Default::default()
    });
    let scene = build_scene(&things);
    let key = scene.bodies[0].moves[0].key.expect("keyed");
    let red: Vec<_> = scene.tags.iter().filter(|g| g.key == key).collect();
    assert_eq!(
        red.iter().map(|g| g.thing).collect::<Vec<_>>(),
        [4, 5, 6, 8]
    );
    assert_eq!(
        (red[0].body, red[1].body, red[2].body),
        (Some(1), Some(0), None)
    );
    // The tag on the turned box sits 40 along that box's x, back at (800, 340) in the world.
    let at = red[0].at;
    assert!((at[0] - 40.).abs() < 1e-3 && at[1].abs() < 1e-3, "{at:?}");
    let world = scene.bodies[1].frame.world(at);
    assert!((world[0] - 800.).abs() < 1e-3 && (world[1] - 340.).abs() < 1e-3);
    // A tag with no position takes its parent's: the same spot on the same box.
    assert_eq!(red[3].body, Some(1));
    assert!((red[3].at[0] - 40.).abs() < 1e-3 && red[3].at[1].abs() < 1e-3);
    // Spots: a body's tags move with it; loose tags keep their world point.
    let mut w = World::new();
    let (a, b) = (w.spawn_empty().id(), w.spawn_empty().id());
    let on_body = lbp_sim::TagMarks(vec![(4, key, at, 0.)]);
    let loose = lbp_sim::TagMarks(vec![(6, key, [-500., 300.], 0.)]);
    let (p, r) = (
        Position::new(Vec2::new(10., 3.)),
        Rotation::radians(quarter),
    );
    let spots = lbp_sim::tag_spots([(a, &on_body, Some((&p, &r))), (b, &loose, None)]);
    let s0 = spots[0].at;
    assert!(
        (s0[0] - 1000.).abs() < 1e-2 && (s0[1] - 340.).abs() < 1e-2,
        "{s0:?}"
    );
    assert_eq!(
        (spots[0].body, spots[1].body, spots[1].at),
        (Some(a), None, [-500., 300.])
    );
    // D1155: a tag whose signal's ternary is 0 cannot be found; the others stay.
    let lit = lbp_sim::lit_tags(spots, |t| i32::from(t != 4));
    assert_eq!(lit.iter().map(|g| g.thing).collect::<Vec<_>>(), [6]);
}

#[test]
fn d1154_followers_measure_from_their_parent() {
    // A box turned a quarter (Things 1, 2) holding a chip 200 above it (3), the chip's
    // board with no position of its own (4) and a follower on the board (5), and a
    // second follower straight on the box (6).
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., quarter, 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., quarter, 1., 1.), None, false),
        thing(3, matrix(0., 500., 0., 0., 1., 1.), None, false),
        thing(4, matrix(0., 0., 0., 0., 1., 1.), None, false),
        thing(5, matrix(0., 0., 0., 0., 1., 1.), None, false),
        thing(6, matrix(0., 0., 0., 0., 1., 1.), None, false),
    ];
    for t in [4, 5, 6] {
        things[t].data.pos = None;
    }
    for (t, p) in [(2, 1), (3, 1), (4, 3), (5, 4), (6, 1)] {
        things[t].parent = Some(p);
    }
    for t in [5, 6] {
        things[t].data.physics_tweak = Some(PhysicsTweak {
            configuration: 11,
            vel_range: 28.,
            accel_strength: 0.125,
            max_range: 2100.,
            angle_range: 360.,
            ..Default::default()
        });
    }
    let scene = build_scene(&things);
    let moves = &scene.bodies[0].moves;
    assert_eq!(moves.iter().map(|m| m.thing).collect::<Vec<_>>(), [5, 6]);
    // The board has no position, so the chip's is taken: 200 along the turned box's x,
    // back at (0, 500) in the world.
    let at = moves[0].at;
    assert!((at[0] - 200.).abs() < 1e-3 && at[1].abs() < 1e-3, "{at:?}");
    let world = scene.bodies[0].frame.world(at);
    assert!(
        world[0].abs() < 1e-3 && (world[1] - 500.).abs() < 1e-3,
        "{world:?}"
    );
    // On the box itself the follower measures from the box.
    let at = moves[1].at;
    assert!(at[0].abs() < 1e-3 && at[1].abs() < 1e-3, "{at:?}");
}

#[test]
fn d1156_spin_aimers_measure_from_their_parent() {
    // As d1154: a quarter-turned box (1, 2) holding a chip 200 above it (3), the chip's
    // board with no position (4) and a configuration-5 tweak on the board (5), and a
    // second one straight on the box (6).
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., quarter, 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., quarter, 1., 1.), None, false),
        thing(3, matrix(0., 500., 0., 0., 1., 1.), None, false),
        thing(4, matrix(0., 0., 0., 0., 1., 1.), None, false),
        thing(5, matrix(0., 0., 0., 0., 1., 1.), None, false),
        thing(6, matrix(0., 0., 0., 0., 1., 1.), None, false),
    ];
    for t in [4, 5, 6] {
        things[t].data.pos = None;
    }
    for (t, p) in [(2, 1), (3, 1), (4, 3), (5, 4), (6, 1)] {
        things[t].parent = Some(p);
    }
    for t in [5, 6] {
        things[t].data.physics_tweak = Some(PhysicsTweak {
            configuration: 5,
            vel_range: 0.1,
            accel_strength: 0.125,
            max_range: 2100.,
            angle_range: 360.,
            ..Default::default()
        });
    }
    let scene = build_scene(&things);
    let spins = &scene.bodies[0].spins;
    assert_eq!(spins.iter().map(|m| m.thing).collect::<Vec<_>>(), [5, 6]);
    let world = scene.bodies[0].frame.world(spins[0].at);
    assert!(
        world[0].abs() < 1e-3 && (world[1] - 500.).abs() < 1e-3,
        "{world:?}"
    );
    let at = spins[1].at;
    assert!(at[0].abs() < 1e-3 && at[1].abs() < 1e-3, "{at:?}");
}

#[test]
fn d1147_attractors_pick_the_highest_priority_own_or_shared_block() {
    use lbp_level::parts::AttractorBlock;
    use lbp_sim::scene::AttractTweak;
    let tweak = |thing, priority, beyond_group| AttractTweak {
        thing,
        block: AttractorBlock {
            priority,
            beyond_group,
            ..Default::default()
        },
    };
    let (own_low, other_high, shared_mid) =
        (tweak(1, 1., false), tweak(2, 9., false), tweak(3, 5., true));
    // Another group's tweak only counts when it is shared; the highest priority wins.
    let picked =
        AttractTweak::pick(&[(&own_low, true), (&other_high, false), (&shared_mid, false)]);
    assert_eq!(picked.map(|t| t.thing), Some(3));
    let picked = AttractTweak::pick(&[(&own_low, true), (&other_high, false)]);
    assert_eq!(picked.map(|t| t.thing), Some(1));
    assert!(AttractTweak::pick(&[(&other_high, false)]).is_none());
    // Of equal priorities the first walked stays.
    let twin = tweak(4, 1., false);
    let picked = AttractTweak::pick(&[(&own_low, true), (&twin, true)]);
    assert_eq!(picked.map(|t| t.thing), Some(1));
    // The record is active above the input threshold.
    assert!(AttractTweak::active(1.));
    assert!(!AttractTweak::active(0.));
}

#[test]
fn d1148_aim_tweaks_take_the_stick_inputs_mirror_and_lead_the_target() {
    use lbp_sim::scene::SpinTweak;
    // Configuration 4 aims along its wired stick (slots 1 and 2, x then y).
    let mut stick = SpinTweak {
        configuration: 4,
        range: 0.2094394,
        accel: 1.,
        stick: [1., 0.],
        scale: 1.,
        ..Default::default()
    };
    let right = stick.aim([0., 0.], None);
    assert_eq!(right, [1., 0.]);
    assert!(stick.turn_along(0., 0., 1., right) < 0.);
    // Mirrored, x is negated, so the same stick turns the other way.
    stick.flip = true;
    assert_eq!(stick.aim([0., 0.], None), [-1., 0.]);
    // Configuration 5's lead: the target point's velocity, vel + omega x r.
    assert_eq!(SpinTweak::lead([2., 0.], 0., [10., 0.]), [2., 0.]);
    assert_eq!(SpinTweak::lead([0., 0.], 0.5, [10., 0.]), [0., 5.]);
    assert_eq!(SpinTweak::lead([1., 1.], 0.5, [0., 4.]), [-1., 1.]);
    // Built from a level, a tweak on a mirrored body is flipped and an upright one is not.
    for (sx, flip) in [(1., false), (-1., true)] {
        let mut things = vec![
            floor(),
            thing(
                1,
                matrix(0., 300., 0., 0., sx, 1.),
                Some(rect(100., 100., 7)),
                true,
            ),
            thing(2, matrix(0., 300., 0., 0., sx, 1.), None, false),
        ];
        things[2].parent = Some(1);
        things[2].data.physics_tweak = Some(PhysicsTweak {
            configuration: 4,
            vel_range: 0.2094394,
            accel_strength: 1.,
            ..Default::default()
        });
        let spin = build_scene(&things).bodies[0].spins[0];
        assert_eq!((spin.flip, spin.stick), (flip, [0., 0.]));
    }
}

#[test]
fn d1149_turn_speed_speeds_up_eases_in_and_lands_in_a_tick() {
    use lbp_sim::scene::SpinTweak;
    // half = 0.5 × accel × range; full acceleration is 2 × half per tick.
    let half = 0.01;
    // Far off it wants full acceleration on top of its speed, either way.
    assert!((SpinTweak::speed(half, 0., 0., 1.) - 0.02).abs() < 1e-6);
    assert!((SpinTweak::speed(half, 0., 0., -1.) + 0.02).abs() < 1e-6);
    assert!((SpinTweak::speed(half, 0.05, 0., 1.) - 0.07).abs() < 1e-6);
    // Arriving in two ticks (h = 2, u = 0.75) it eases in: 2 × half × u × (2 − u).
    let e = 23. * half / 8.;
    assert!((SpinTweak::speed(half, 0., 0., e) - 0.01875).abs() < 1e-5);
    assert!((SpinTweak::speed(half, 0., 0., -e) + 0.01875).abs() < 1e-5);
    // Within a tick it closes the gap at once, plus the aim's own turn.
    assert!((SpinTweak::speed(half, 0., 0., 0.001) - 0.001).abs() < 1e-7);
    assert!((SpinTweak::speed(half, 0., 0.0005, 0.001) - 0.0015).abs() < 1e-6);
    // A gyroscope tilted by 0.3 rad comes upright without overshooting past 0.01.
    let gyro = SpinTweak {
        configuration: 1,
        range: 0.2094394,
        accel: 0.1,
        ..Default::default()
    };
    let (mut angle, mut w) = (0.3f32, 0f32);
    for _ in 0..200 {
        w = gyro.turn(angle, w, 1.);
        angle += w;
        assert!(angle > -0.01, "overshot to {angle}");
    }
    assert!(angle.abs() < 1e-3 && w.abs() < 1e-3, "{angle} {w}");
}

#[test]
fn d1150_followers_ease_in_with_the_turn_shaper() {
    // A follower (configuration 11) with accel 0.125 and range 28 keeping 100 from a
    // target 1000 away: full acceleration (3.5 a tick) from rest, never past its range,
    // then the shaper eases it in so it stops at 900 without overshooting.
    let follower = lbp_sim::scene::MoveTweak {
        configuration: 11,
        vel: [0., 0.],
        range: 28.,
        accel: 0.125,
        decel: 0.125,
        min_range: 100.,
        max_range: 2100.,
        angle_range: 360.,
        up_down: true,
        thing: 0,
        behavior: 0,
        local: false,
        flee: false,
        key: None,
        angle: 0.,
        at: [0.; 2],
        stick: [0.; 2],
        scale: 1.,
        mirror: false,
        flip: false,
        in_out: false,
    };
    let up = [0., 1.];
    let v = follower.push([0., 0.], up, Some([1000., 0.]), 1.);
    assert!((v[0] - 3.5).abs() < 1e-5 && v[1].abs() < 1e-6, "{v:?}");
    // Half activation halves both the shaper's acceleration and the limits.
    let v = follower.push([0., 0.], up, Some([1000., 0.]), 0.5);
    assert!((v[0] - 1.75).abs() < 1e-5, "{v:?}");
    let (mut x, mut v, mut top) = (0f32, [0f32, 0.], 0f32);
    for _ in 0..200 {
        v = follower.push(v, up, Some([1000. - x, 0.]), 1.);
        top = top.max(v[0]);
        x += v[0];
        assert!(x <= 900. + 1e-2, "overshot to {x}");
    }
    assert!((top - 28.).abs() < 1e-4, "top speed {top}");
    assert!(
        (x - 900.).abs() < 1e-2 && v[0].abs() < 1e-3,
        "stopped at {x}, {v:?}"
    );
    // Moving away from the target, the wanted speed is negative along the aim: it brakes.
    let v = follower.push([-10., 0.], up, Some([500., 0.]), 1.);
    assert!((v[0] - -6.5).abs() < 1e-4, "{v:?}");
}

#[test]
fn d1151_followers_turn_their_up_axis_and_keep_local_space() {
    // A box with a follower (Thing 2) that keeps 100 from its target.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 11,
        vel_range: 28.,
        accel_strength: 0.125,
        decel_strength: 0.125,
        min_range: 100.,
        max_range: 2100.,
        angle_range: 360.,
        allow_up_down: true,
        ..Default::default()
    });
    let follower = build_scene(&things).bodies[0].moves[0];
    // With allowInOut the caller moves layers first (0x1c8250), then packs it as usual.
    things[2].data.physics_tweak.as_mut().unwrap().allow_in_out = true;
    assert_eq!(build_scene(&things).bodies[0].moves.len(), 1);
    let up = [0., 1.];
    let d = Some([1000., 0.]);
    // allowUpDown off drops the error along the up axis; the +0x220 angle turns that axis,
    // here a quarter turn onto x, so the sideways target is dropped too.
    let flat = lbp_sim::scene::MoveTweak {
        up_down: false,
        ..follower
    };
    let v = flat.push([0., 0.], up, d, 1.);
    assert!((v[0] - 3.5).abs() < 1e-5, "{v:?}");
    let turned = lbp_sim::scene::MoveTweak {
        angle: std::f32::consts::FRAC_PI_2,
        ..flat
    };
    let v = turned.push([0., 0.], up, d, 1.);
    assert!(v[0].abs() < 1e-3 && v[1].abs() < 1e-3, "{v:?}");
    // localSpace turns the wanted velocity by the up axis: upright it is unchanged, with
    // the up axis on x a push along x becomes one along -y.
    let local = lbp_sim::scene::MoveTweak {
        local: true,
        ..follower
    };
    let v = local.push([0., 0.], up, d, 1.);
    assert!((v[0] - 3.5).abs() < 1e-5 && v[1].abs() < 1e-5, "{v:?}");
    let v = local.push([0., 0.], [1., 0.], d, 1.);
    assert!(v[0].abs() < 1e-5 && (v[1] + 3.5).abs() < 1e-5, "{v:?}");
}

#[test]
fn d1158_stick_movers_map_the_stick_onto_a_circle() {
    let stick = lbp_sim::scene::MoveTweak {
        thing: 0,
        configuration: 6,
        behavior: 0,
        range: 10.,
        accel: 1.,
        decel: 1.,
        vel: [0., 0.],
        local: false,
        min_range: 0.,
        max_range: 0.,
        angle_range: 360.,
        flee: false,
        up_down: true,
        key: None,
        angle: 0.,
        at: [0.; 2],
        stick: [1., 1.],
        scale: 1.,
        mirror: false,
        flip: false,
        in_out: false,
    };
    let up = [0., 1.];
    // A full diagonal stick wants range (10) along the diagonal, not range times root 2.
    let v = stick.push([0., 0.], up, None, 1.);
    let h = 10. / 2f32.sqrt();
    assert!((v[0] - h).abs() < 1e-4 && (v[1] - h).abs() < 1e-4, "{v:?}");
    // Configuration 7 keeps the unscaled stick: at most 1 a tick.
    let seven = lbp_sim::scene::MoveTweak {
        configuration: 7,
        stick: [-0.5, 0.],
        scale: 1.,
        ..stick
    };
    let v = seven.push([0., 0.], up, None, 1.);
    assert!((v[0] + 0.5).abs() < 1e-5 && v[1].abs() < 1e-5, "{v:?}");
    // localSpace turns it by the up axis; no stick brakes with ±decel × range.
    let local = lbp_sim::scene::MoveTweak {
        local: true,
        stick: [0., 1.],
        scale: 1.,
        ..stick
    };
    let v = local.push([0., 0.], [1., 0.], None, 1.);
    assert!((v[0] - 10.).abs() < 1e-4 && v[1].abs() < 1e-4, "{v:?}");
    let idle = lbp_sim::scene::MoveTweak {
        stick: [0., 0.],
        scale: 1.,
        ..stick
    };
    assert_eq!(idle.push([0., 30.], up, None, 1.), [0., 20.]);
}

#[test]
fn d1157_tag_synchronisers_send_a_tag_offset_and_receivers_place_a_tag() {
    // A red "A" sender at (0, 300) on a box turned a quarter (Things 1, 2) and a loose
    // red "A" receiver at (500, 300) scaling the offset by (2, 1) (Thing 3).
    let quarter = std::f32::consts::FRAC_PI_2;
    let sync = |kind: i32, scale: [f32; 2]| TagSynchroniser {
        kind,
        colour_index: 1,
        name: "A".into(),
        radius: 100.,
        min_radius: 0.,
        scale,
        ..Default::default()
    };
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., quarter, 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., quarter, 3., 3.), None, false),
        thing(3, matrix(500., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.tag_synchroniser = Some(sync(0, [1., 1.]));
    things[3].data.tag_synchroniser = Some(sync(1, [2., 1.]));
    let scene = build_scene(&things);
    let [s, r] = [scene.syncs[0], scene.syncs[1]];
    assert_eq!(
        (s.thing, s.kind, s.body, r.kind, r.body),
        (2, 0, Some(0), 1, None)
    );
    assert_eq!(s.key, r.key);
    // The sender's axes are unit and in its box's frame (the box turns with it).
    let near = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3;
    assert!(
        near(s.axes[0], [1., 0.]) && near(s.axes[1], [0., 1.]),
        "{:?}",
        s.axes
    );
    assert!(near(s.at, [0., 0.]) && near(r.at, [500., 300.]));
    // Spots: the box at (0, 300) turned a quarter, so the sender's x axis points up.
    let mut w = World::new();
    let (a, b, c) = (
        w.spawn_empty().id(),
        w.spawn_empty().id(),
        w.spawn_empty().id(),
    );
    let (p, q) = (
        Position::new(Vec2::new(0., 300.) * lbp_sim::UNIT_SCALE),
        Rotation::radians(quarter),
    );
    let on_box = lbp_sim::SyncMarks(vec![s]);
    let loose = lbp_sim::SyncMarks(vec![r]);
    let spots = lbp_sim::sync_spots([(a, &on_box, Some((&p, &q))), (b, &loose, None)]);
    assert!(near(spots[0].at, [0., 300.]) && near(spots[0].axes[0], [0., 1.]));
    let tag = |at: [f32; 2], body| lbp_sim::TagSpot {
        thing: 9,
        key: s.key,
        body,
        at,
        z: 0.,
    };
    // A tag 30 up and 10 left of the sender is (30, 10) along its axes: the receiver
    // places its tag at (560, 310).
    let out = lbp_sim::synced_tags(&spots, &[tag([-10., 330.], Some(c))]);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].thing, out[0].key, out[0].body), (3, s.key, None));
    assert!(near(out[0].at, [560., 310.]), "{:?}", out[0].at);
    // No tag, a tag out of the ring, or a tag on the sender's own box: no tag placed.
    assert!(lbp_sim::synced_tags(&spots, &[]).is_empty());
    assert!(lbp_sim::synced_tags(&spots, &[tag([0., 500.], Some(c))]).is_empty());
    assert!(lbp_sim::synced_tags(&spots, &[tag([-10., 330.], Some(a))]).is_empty());
    // A radius below 0 has no ring.
    let mut far = spots.clone();
    far[0].radius = -1.;
    let out = lbp_sim::synced_tags(&far, &[tag([0., 500.], None)]);
    assert!(near(out[0].at, [900., 300.]), "{:?}", out[0].at);
}

#[test]
fn d1159_tweaks_on_a_chip_take_their_turn_on_the_board() {
    use lbp_level::parts::{ChipComponent, Microchip};
    use lbp_sim::scene::SpinTweak;
    let build = |flagged: bool| {
        let mut things = vec![
            floor(),
            thing(
                1,
                matrix(0., 300., 0., 0.3, 1., 1.),
                Some(rect(100., 100., 7)),
                true,
            ),
            thing(2, matrix(0., 300., 0., 0.3, 1., 1.), None, false),
            thing(3, matrix(0., 300., 0., 0.3, 1., 1.), None, false),
            // On the board, turned 0.4 further: a PPos, so its local matrix counts.
            thing(4, matrix(0., 300., 0., 0.7, 1., 1.), None, false),
            // No PPos: its entry in the chip's component table counts.
            thing(5, matrix(0., 300., 0., 0., 1., 1.), None, false),
        ];
        things[5].data.pos = None;
        for (k, p) in [(2, 1), (3, 2), (4, 3), (5, 3)] {
            things[k].parent = Some(p);
        }
        things[2].data.microchip = Some(Microchip {
            circuit_board: Some(3),
            components: vec![ChipComponent {
                thing: Some(5),
                values: [0., 0., 0.5, 1., 1.],
                flag: flagged,
            }],
            ..Default::default()
        });
        things[4].data.physics_tweak = Some(PhysicsTweak {
            configuration: 1,
            vel_range: 0.2,
            accel_strength: 1.,
            ..Default::default()
        });
        things[5].data.physics_tweak = Some(PhysicsTweak {
            configuration: 8,
            vel_range: 1.,
            accel_strength: 1.,
            ..Default::default()
        });
        let scene = build_scene(&things);
        (scene.bodies[0].spins[0], scene.bodies[0].moves[0].angle)
    };
    let (spin, mover): (SpinTweak, f32) = build(false);
    // +0x220 is the inverse of the board turn: -0.4 for the spin, +0.5 from the table.
    assert!(
        (spin.angle + 0.4).abs() < 1e-5 && !spin.flip,
        "{}",
        spin.angle
    );
    assert_eq!(mover, 0.5);
    // A gyroscope turned 0.4 on its chip holds its body 0.4 clockwise of upright.
    assert!(spin.target_along(-0.4, 0., [0., 1.]).0.abs() < 1e-5);
    assert!(spin.target_along(0., 0., [0., 1.]).0 < 0.);
    // A flipped entry negates the sum.
    assert_eq!(build(true).1, -0.5);
}

#[test]
fn d1160_mirrored_movers_negate_their_local_turn_and_stick_x() {
    let base = lbp_sim::scene::MoveTweak {
        thing: 0,
        configuration: 8,
        behavior: 0,
        range: 10.,
        accel: 1.,
        decel: 1.,
        vel: [0., 1.],
        local: true,
        min_range: 0.,
        max_range: 0.,
        angle_range: 360.,
        flee: false,
        up_down: true,
        key: None,
        angle: std::f32::consts::FRAC_PI_2,
        at: [0.; 2],
        stick: [0.; 2],
        scale: 1.,
        mirror: false,
        flip: false,
        in_out: false,
    };
    let up = [0., 1.];
    // localSpace turns the velocity by the chip's angle; mirrored, by minus it (0xcc6f0).
    let v = base.push([0., 0.], up, None, 1.);
    let m = lbp_sim::scene::MoveTweak {
        mirror: true,
        ..base
    }
    .push([0., 0.], up, None, 1.);
    assert!(
        (v[0] + m[0]).abs() < 1e-4 && (v[1] - m[1]).abs() < 1e-4,
        "{v:?} {m:?}"
    );
    assert!(v[0].abs() > 0.5, "{v:?}");
    // A stick mover whose mirror differs from +0x224 negates stick x (0x958038).
    let stick = lbp_sim::scene::MoveTweak {
        configuration: 6,
        local: false,
        angle: 0.,
        stick: [1., 0.],
        scale: 1.,
        ..base
    };
    let v = stick.push([0., 0.], up, None, 1.);
    let f = lbp_sim::scene::MoveTweak {
        flip: true,
        ..stick
    }
    .push([0., 0.], up, None, 1.);
    assert!(
        (v[0] - 10.).abs() < 1e-4 && (f[0] + 10.).abs() < 1e-4,
        "{v:?} {f:?}"
    );
}

#[test]
fn d1161_the_input_scale_scales_movers_and_caps_followers_and_spins() {
    let mover = lbp_sim::scene::MoveTweak {
        thing: 0,
        configuration: 8,
        behavior: 2,
        range: 10.,
        accel: 1.,
        decel: 1.,
        vel: [0., 4.],
        local: false,
        min_range: 0.,
        max_range: 0.,
        angle_range: 360.,
        flee: false,
        up_down: true,
        key: None,
        angle: 0.,
        at: [0.; 2],
        stick: [0.; 2],
        scale: 1.,
        mirror: false,
        flip: false,
        in_out: false,
    };
    let up = [0., 1.];
    assert_eq!(mover.push([0., 0.], up, None, 1.), [0., 4.]);
    // A movers' target times the signed scale (0x95816c): half, or backwards.
    let half = lbp_sim::scene::MoveTweak {
        scale: 0.5,
        ..mover
    };
    assert_eq!(half.push([0., 0.], up, None, 1.), [0., 2.]);
    let back = lbp_sim::scene::MoveTweak {
        scale: -0.5,
        ..mover
    };
    assert_eq!(back.push([0., 0.], up, None, 1.), [0., -2.]);
    // A follower's speed is capped at ±range × |scale| (0x958724).
    let follower = lbp_sim::scene::MoveTweak {
        configuration: 11,
        accel: 100.,
        scale: -0.25,
        ..mover
    };
    let v = follower.push([0., 0.], up, Some([0., 1000.]), 1.);
    assert!((v[1] - 2.5).abs() < 1e-4 && v[0].abs() < 1e-4, "{v:?}");
    // A gyroscope's wanted turn speed is capped at ±range × |scale| (0x9607ec).
    let gyro = lbp_sim::scene::SpinTweak {
        configuration: 1,
        range: 0.2,
        accel: 10.,
        scale: 0.5,
        ..Default::default()
    };
    let (s, _, _) = gyro.target(1., 0.);
    assert!((s.abs() - 0.1).abs() < 1e-6, "{s}");
}

#[test]
fn d1163_a_follower_always_packs_its_cross_row() {
    let mover = lbp_sim::scene::MoveTweak {
        thing: 0,
        configuration: 8,
        behavior: 0,
        range: 10.,
        accel: 1.,
        decel: 0.,
        vel: [0., 4.],
        local: false,
        min_range: 0.,
        max_range: 0.,
        angle_range: 360.,
        flee: false,
        up_down: true,
        key: None,
        angle: 0.,
        at: [0.; 2],
        stick: [0.; 2],
        scale: 1.,
        mirror: false,
        flip: false,
        in_out: false,
    };
    let up = [0., 1.];
    // decel × range 0: a mover packs row 1 only (0x1c804c, 0x1c81ac).
    assert_eq!(mover.rows([3., 0.], up, None, 1.).len(), 1);
    // A follower packs row 2 anyway, ±accel × range, and holds its sideways speed.
    let follower = lbp_sim::scene::MoveTweak {
        configuration: 11,
        ..mover
    };
    let d = Some([0., 1000.]);
    assert_eq!(follower.rows([3., 0.], up, d, 1.).len(), 2);
    let v = follower.push([3., 0.], up, d, 1.);
    assert!(v[0].abs() < 1e-4 && v[1] > 0., "{v:?}");
}

#[test]
fn d1164_an_aimer_leads_a_moving_target_by_its_turn_per_tick() {
    use lbp_sim::scene::SpinTweak;
    let aimer = SpinTweak {
        configuration: 5,
        range: 0.2094394,
        accel: 0.1,
        ..Default::default()
    };
    // Lined up on a still target it wants no turn.
    assert_eq!(aimer.target_led(0., 0., [0., 10.], [0., 0.]).0, 0.);
    // A target 10 above moving 0.01 left a tick turns the aim 0.001 rad a tick
    // anticlockwise, which the aimer takes on at once.
    let (s, _, _) = aimer.target_led(0., 0., [0., 10.], [-0.01, 0.]);
    assert!((s - (0.01f32).atan2(10.)).abs() < 1e-6, "{s}");
    // Chasing a target that keeps moving, the lead leaves it nearer the aim.
    let chase = |lead: bool| {
        let (mut angle, mut w, mut at) = (0f32, 0f32, [0f32, 100.]);
        let v = [-2f32, 0.];
        for _ in 0..60 {
            let l = if lead { v } else { [0., 0.] };
            w = aimer.turn_led(angle, w, 1., at, l);
            angle += w;
            at = [at[0] + v[0], at[1] + v[1]];
        }
        ((-at[0]).atan2(at[1]) - angle).abs()
    };
    assert!(
        chase(true) < chase(false),
        "{} {}",
        chase(true),
        chase(false)
    );
    // The lead is the point velocity of the target's body.
    assert_eq!(SpinTweak::lead([1., 0.], 0.5, [0., 2.]), [0., 0.]);
}

#[test]
fn d1165_an_aimer_leads_the_player_by_his_velocity() {
    use bevy::ecs::system::RunSystemOnce;
    use lbp_sim::scene::SpinTweak;
    let aimer = SpinTweak {
        configuration: 5,
        range: 0.2094394,
        accel: 0.1,
        min_range: 0.,
        max_range: -1.,
        angle_range: 180.,
        ..Default::default()
    };
    // The aimer at the origin, lined up on Sackboy 10 above, moving `v` a tick.
    let turn = |v: [f32; 2]| {
        let mut w = World::new();
        // No physics here, so the body cannot be woken.
        w.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::ignore,
        ));
        let e = w
            .spawn((
                lbp_sim::SpinTweaks(vec![aimer]),
                Position::new(Vec2::ZERO),
                Rotation::radians(0.),
                LinearVelocity::ZERO,
                AngularVelocity(0.),
            ))
            .id();
        w.run_system_once(
            move |mut c: Commands,
                  mut q: Query<(
                Entity,
                &lbp_sim::SpinTweaks,
                &Position,
                &Rotation,
                &LinearVelocity,
                &mut AngularVelocity,
            )>| {
                let player = Some((Vec2::new(0., 10. * lbp_sim::UNIT_SCALE), v));
                lbp_sim::drive_spins(
                    &mut c,
                    &mut q,
                    player,
                    &[],
                    |_| 1.,
                    |t| t.stick,
                    |t| t.scale,
                    |_| None,
                );
            },
        )
        .unwrap();
        w.get::<AngularVelocity>(e).unwrap().0 / TICK_HZ as f32
    };
    // Standing still he needs no turn; walking left a tick he is led anticlockwise.
    assert_eq!(turn([0., 0.]), 0.);
    let s = turn([-0.01, 0.]);
    assert!((s - (0.01f32).atan2(10.)).abs() < 1e-6, "{s}");
}

#[test]
fn d1166_in_out_followers_move_to_their_targets_layer_or_as_near_as_fits() {
    use lbp_sim::scene::{layer_of, MoveTweak};
    // Layers are 200 deep, counted from the back at z -679.
    assert_eq!((layer_of(-679.), layer_of(-480.), layer_of(0.)), (0, 0, 3));
    let follower = MoveTweak {
        thing: 1,
        configuration: 11,
        behavior: 0,
        range: 1.,
        accel: 1.,
        decel: 1.,
        vel: [0., 0.],
        local: false,
        min_range: 0.,
        max_range: -1.,
        angle_range: 360.,
        flee: false,
        up_down: true,
        key: None,
        angle: 0.,
        at: [0., 0.],
        stick: [0., 0.],
        scale: 1.,
        mirror: false,
        flip: false,
        in_out: true,
    };
    // Two layers in front of it the target pulls the group 400 forward when it fits.
    assert_eq!(follower.in_out_step(-479., -79., |_| true), 400.);
    // With room for only one layer it takes that; with none it stays.
    assert_eq!(follower.in_out_step(-479., -79., |dz| dz < 300.), 200.);
    assert_eq!(follower.in_out_step(-479., -79., |_| false), 0.);
    // On the target's layer, or without allowInOut, nothing moves.
    assert_eq!(follower.in_out_step(-479., -400., |_| true), 0.);
    let plain = MoveTweak {
        in_out: false,
        ..follower
    };
    assert_eq!(plain.in_out_step(-479., -79., |_| true), 0.);
    // A step that would leave the depth band is blocked.
    assert_eq!(follower.in_out_step(79., 479., |_| true), 0.);
}

#[test]
fn d1167_offline_a_receiver_needs_a_sender_of_its_key_in_the_level() {
    let spot = |thing, kind, key, at| lbp_sim::SyncSpot {
        thing,
        kind,
        key,
        radius: -1.,
        min_radius: 0.,
        scale: [1., 1.],
        use_layers: false,
        body: None,
        at,
        axes: [[1., 0.], [0., 1.]],
        z: 0.,
    };
    let tag = lbp_sim::TagSpot {
        thing: 9,
        key: 5,
        body: None,
        at: [10., 20.],
        z: 0.,
    };
    // A receiver of key 5 beside a tag of key 5 but no sender: the online source
    // 0x2a8538 is absent offline, so it places nothing.
    let alone = [spot(1, 1, 5, [100., 0.])];
    assert!(lbp_sim::synced_tags(&alone, &[tag]).is_empty());
    // A sender of another key does not feed it either.
    let other = [spot(1, 1, 5, [100., 0.]), spot(2, 0, 6, [0., 0.])];
    assert!(lbp_sim::synced_tags(&other, &[tag]).is_empty());
    // With a sender of key 5 it places the tag at its own point plus the offset.
    let sent = [spot(1, 1, 5, [100., 0.]), spot(2, 0, 5, [0., 0.])];
    let out = lbp_sim::synced_tags(&sent, &[tag]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].at, [110., 20.]);
}

#[test]
fn d1168_a_receiver_with_use_layers_puts_its_tag_on_the_sent_tags_z() {
    let spot = |thing, kind, use_layers, z| lbp_sim::SyncSpot {
        thing,
        kind,
        key: 5,
        radius: -1.,
        min_radius: 0.,
        scale: [1., 1.],
        use_layers,
        body: None,
        at: [100. * thing as f32, 0.],
        axes: [[1., 0.], [0., 1.]],
        z,
    };
    let tag = lbp_sim::TagSpot {
        thing: 9,
        key: 5,
        body: None,
        at: [10., 20.],
        z: 279.,
    };
    // The sender at z 79 sends the tag's world z 279 (0x13187c's z lane is not a
    // difference). A receiver at z -121 without useLayers keeps its own z; with it, the
    // tag goes to 279.
    let syncs = [
        spot(0, 0, false, 79.),
        spot(1, 1, false, -121.),
        spot(2, 1, true, -121.),
    ];
    let out = lbp_sim::synced_tags(&syncs, &[tag]);
    assert_eq!(out.len(), 2);
    assert_eq!((out[0].at, out[0].z), ([110., 20.], -121.));
    assert_eq!((out[1].at, out[1].z), ([210., 20.], 279.));
}

/// D1169: the solver row for a piston (0x962590) locks the relative angle its ends
/// had when the joint first ran (synced by 0xcab7c/0xc9af4), wrapped to [-pi, pi).
#[test]
fn d1169_a_piston_holds_the_relative_angle_its_bodies_start_at() {
    let params = JointParams {
        tweak_target_min_length: 150.,
        tweak_target_max_length: 150.,
        ..Default::default()
    };
    let things = vec![
        ceiling(),
        thing(
            1,
            matrix(0., 850., 0., 0.5, 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint_with(2, 4, 0, [0.; 3], 1, [0.; 3], 150., params),
    ];
    let out = run(&things, 120);
    let p = out[1].1;
    assert!((p[1] - 850.).abs() < 5. && p[0].abs() < 2., "{p:?}");
    assert!(
        (out[1].2 - 0.5).abs() < 0.01,
        "keeps its start angle: {}",
        out[1].2
    );
}

#[test]
fn d1173_a_motor_pushes_by_its_strength_clamped_to_one() {
    use lbp_sim::{motor_damping, MOTOR_DAMPING};
    let at = |strength| {
        motor_damping(&JointParams {
            strength,
            ..Default::default()
        })
    };
    // Strength 0 bounds the push to nothing (0x95fab8): the motor does not turn its ends.
    assert_eq!(at(0.), 0.);
    assert_eq!(at(-1.), 0.);
    // 0xca760 clamps the strength to [0, 1].
    assert!((at(0.5) - 0.5 * MOTOR_DAMPING).abs() < 1e-6);
    assert_eq!(at(1.), MOTOR_DAMPING);
    assert_eq!(at(3.), MOTOR_DAMPING);
}

#[test]
fn d1174_a_pattern_0_motor_is_limited_to_half_its_range_either_side() {
    use lbp_sim::motor_angle_limits;
    let limits = |pattern, range| {
        motor_angle_limits(
            &JointParams {
                animation_pattern: pattern,
                motion: [range, 30., 0., 0., 0., 0., 0., 0., 0., 0.],
                ..Default::default()
            },
            0.,
        )
    };
    // 0x1c9eac: pattern 0 with a range under 2π is held to the centre ± range / 2, which
    // (D1175, 0xc95ec) is the sweep: from the start to the start + range.
    assert_eq!(limits(0, 1.), Some((0., 1.)));
    assert_eq!(limits(0, -1.), Some((-1., 0.)));
    // A range of 2π or more, and pattern 1, have no limits.
    assert_eq!(limits(0, 7.), None);
    assert_eq!(limits(1, 1.), None);
}

#[test]
fn d1175_a_pattern_0_motor_joint_is_limited_to_its_sweep() {
    let limits = |pattern, range, phase| {
        let params = JointParams {
            animation_pattern: pattern,
            strength: 1.,
            motion: [range, 30., phase, 0., 0., 0., 0., 0., 0., 0.],
            ..Default::default()
        };
        let things = vec![
            thing(
                0,
                matrix(0., 0., 0., 0., 1., 1.),
                Some(rect(20., 20., 7)),
                false,
            ),
            thing(
                1,
                matrix(0., 0., 0., 0., 1., 1.),
                Some(rect(100., 100., 7)),
                true,
            ),
            joint_with(2, 9, 0, [0.; 3], 1, [0.; 3], 0., params),
        ];
        let mut app = app(build_scene(&things));
        app.update();
        let mut q = app.world_mut().query::<&RevoluteJoint>();
        let j = q.single(app.world()).expect("motor");
        j.angle_limit.map(|l| (l.min, l.max))
    };
    assert_eq!(limits(0, 1., 0.), Some((0., 1.)));
    // Started half way through its sweep (phase 7.5 of time 30), the limits sit either side.
    let (lo, hi) = limits(0, 1., 7.5).expect("limits");
    assert!(
        (lo + 0.5).abs() < 1e-3 && (hi - 0.5).abs() < 1e-3,
        "{lo} {hi}"
    );
    assert_eq!(limits(0, 7., 0.), None);
    assert_eq!(limits(1, 1., 0.), None);
}

#[test]
fn d1177_a_pattern_2_motor_joint_is_limited_to_its_sweep() {
    let limits = |range, phase| {
        let params = JointParams {
            animation_pattern: 2,
            strength: 1.,
            motion: [range, 30., phase, 0., 0., 0., 0., 0., 0., 0.],
            ..Default::default()
        };
        let things = vec![
            thing(
                0,
                matrix(0., 0., 0., 0., 1., 1.),
                Some(rect(20., 20., 7)),
                false,
            ),
            thing(
                1,
                matrix(0., 0., 0., 0., 1., 1.),
                Some(rect(100., 100., 7)),
                true,
            ),
            joint_with(2, 9, 0, [0.; 3], 1, [0.; 3], 0., params),
        ];
        let mut app = app(build_scene(&things));
        app.update();
        let mut q = app.world_mut().query::<&RevoluteJoint>();
        let j = q.single(app.world()).expect("motor");
        j.angle_limit.map(|l| (l.min, l.max))
    };
    // 0x1ca6f0: the centre to the centre + range. The ramp starts at the top of its sweep.
    let (lo, hi) = limits(1., 0.).expect("limits");
    assert!((lo + 1.).abs() < 1e-5 && hi.abs() < 1e-5, "{lo} {hi}");
    // A third of the way down (phase 10 of time 30) it sits two thirds up the sweep.
    let (lo, hi) = limits(1., 10.).expect("limits");
    assert!(
        (lo + 2. / 3.).abs() < 1e-5 && (hi - 1. / 3.).abs() < 1e-5,
        "{lo} {hi}"
    );
    // A range of 2π or more turns freely.
    assert_eq!(limits(7., 0.), None);
}

#[test]
fn d1176_a_one_shot_piston_runs_its_pattern_once() {
    use lbp_sim::scene::SceneJoint;
    let mut params = JointParams {
        mode: 2,
        ..Default::default()
    };
    // range 315, time 120, phase 30 (unused one shot), speed 0.02, pause 20.
    params.motion[..5].copy_from_slice(&[315., 120., 30., 0.02, 20.]);
    let mut j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 840.,
        min_length: 840.,
        max_length: 840.,
        params,
    };
    // Pattern 0 (0xc998c): out and back once with no phase, then the base for good.
    assert_eq!((j.length(0., true), j.length(70., true)), (840., 1155.));
    assert_eq!((j.length(140., true), j.length(210., true)), (840., 840.));
    assert!(j.length(210., false) > 840.);
    // Pattern 2 (0xc9804): down from the top once, then the base.
    j.params.animation_pattern = 2;
    assert_eq!(j.length(60., true), 997.5);
    assert_eq!(j.length(200., true), 840.);
    // Pattern 1 (0xc99a4): base + frame × speed until a turn of 2π, then the base.
    j.params.animation_pattern = 1;
    assert!((j.length(100., true) - 842.).abs() < 1e-3);
    assert_eq!(j.length(400., true), 840.);
}

#[test]
fn d1178_only_a_pattern_2_motor_has_a_k_term() {
    use lbp_sim::{motor_band, motor_k};
    let params = |pattern, mode, strength| JointParams {
        animation_pattern: pattern,
        mode,
        strength,
        ..Default::default()
    };
    // 0x1c9eac: 5 × strength for pattern 2, clamped like the strength cap.
    assert_eq!(motor_k(&params(2, 0, 0.5), false), 2.5);
    assert_eq!(motor_k(&params(2, 0, 3.), false), 5.);
    assert_eq!(motor_k(&params(0, 0, 1.), false), 0.);
    assert_eq!(motor_k(&params(1, 0, 1.), false), 0.);
    // Wired in mode 1 or 4 it is 0; in mode 2 it stays.
    assert_eq!(motor_k(&params(2, 1, 1.), true), 0.);
    assert_eq!(motor_k(&params(2, 4, 1.), true), 0.);
    assert_eq!(motor_k(&params(2, 2, 1.), true), 5.);
    assert_eq!(motor_k(&params(2, 1, 1.), false), 5.);
    // 0x95f494: the clipped delta to 1.5 × err, sorted, with a cap of k × max(|err|, 0.1).
    let near = |(a, b, c): (f32, f32, f32), (x, y, z): (f32, f32, f32)| {
        assert!(
            (a - x).abs() < 1e-6 && (b - y).abs() < 1e-6 && (c - z).abs() < 1e-6,
            "{a} {b} {c}"
        )
    };
    near(motor_band(0.2, 0.4, 5.), (0.2, 0.6, 2.));
    near(motor_band(0.2, -0.4, 5.), (-0.6, 0., 2.));
    near(motor_band(-0.1, -0.02, 5.), (-0.03, -0.02, 0.5));
}

#[test]
fn d1179_a_motor_pushes_to_its_band_and_only_pattern_2_is_braked_past_it() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    use lbp_sim::{motor_damping, MOTOR_DAMPING};
    let joint = |animation_pattern| {
        let mut params = JointParams {
            mode: 1,
            animation_pattern,
            strength: 1.,
            ..Default::default()
        };
        params.motion[0] = std::f32::consts::PI;
        params.motion[1] = 180.;
        params.motion[4] = 30.;
        SceneJoint {
            thing: 0,
            kind: JointKind::Motor,
            a: Endpoint::Static(0),
            b: Endpoint::Static(1),
            world_a: [0.; 2],
            world_b: [0.; 2],
            local_a: [0.; 2],
            local_b: [0.; 2],
            length: 0.,
            min_length: 0.,
            max_length: 0.,
            params,
        }
    };
    for pattern in [0, 2] {
        let drive = || lbp_sim::PistonDrive {
            joint: joint(pattern),
            clock: PistonClock::default(),
            sync: None,
        };
        let band = |rate| drive().band_to(40., None, 0.5, rate);
        let w = drive().turn_to(40., None, 0.5);
        assert!(w.abs() > 1e-3, "pattern {pattern} turns: {w}");
        let push = motor_damping(&joint(pattern).params);
        // At rest it is driven to the cut `delta` edge with the strength cap.
        let (v, d) = band(0.);
        assert!(
            (v - w).abs() < 1e-4 && d == push,
            "pattern {pattern}: {v} {d}"
        );
        // On its edge it is inside the band and not pushed.
        assert_eq!(band(w).1, 0.);
        // Past 1.5 × `err` only a pattern-2 motor (k 5 × strength) is braked, to that edge.
        let (v, d) = band(3. * w);
        assert!((v - 1.5 * w).abs() < 1e-3, "pattern {pattern}: {v}");
        let brake = if pattern == 2 {
            MOTOR_DAMPING * (5. * (w / 30.).abs().max(0.1)).min(1.)
        } else {
            0.
        };
        assert!((d - brake).abs() < 1e-5, "pattern {pattern}: {d} {brake}");
    }
}

#[test]
fn d1180_a_piston_row_has_the_motor_band_with_a_floor_of_10() {
    use lbp_sim::{motor_band, piston_band};
    let near = |a: (f32, f32, f32), b: (f32, f32, f32)| {
        assert!(
            (a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5 && (a.2 - b.2).abs() < 1e-4,
            "{a:?} vs {b:?}"
        )
    };
    // The same cut `delta` and 1.5 × `err` edges as the motor row.
    near(piston_band(4., 8., 5.), (4., 12., 50.));
    near(piston_band(4., -8., 5.), (-12., 0., 50.));
    // The cap's floor is 10 game units of length, not the motor's 0.1 rad.
    near(piston_band(2., 30., 5.), (2., 45., 150.));
    let (lo, hi, _) = motor_band(4., 8., 5.);
    assert_eq!((lo, hi), (4., 12.));
    assert_eq!(piston_band(4., 8., 0.).2, 0.);
}

#[test]
fn d1181_a_piston_drives_its_slider_within_its_band() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    use lbp_sim::{motor_damping, UNIT_SCALE};
    let mut params = JointParams {
        mode: 1,
        strength: 1.,
        ..Default::default()
    };
    params.motion[..2].copy_from_slice(&[-200., 60.]);
    let drive = || lbp_sim::PistonDrive {
        joint: SceneJoint {
            thing: 0,
            kind: JointKind::Piston,
            a: Endpoint::Static(0),
            b: Endpoint::Static(1),
            world_a: [0.; 2],
            world_b: [0.; 2],
            local_a: [0.; 2],
            local_b: [0.; 2],
            length: 400.,
            min_length: 400.,
            max_length: 400.,
            params,
        },
        clock: PistonClock::default(),
        sync: None,
    };
    let on = Some((1, 1., false));
    let hz = lbp_sim::TICK_HZ as f32;
    // Switched on (mode 1) it shortens 200 over 30 frames, uncut: the band is that
    // `delta` to 1.5 × `err`, with the length at its rule (300 at tick 15).
    let delta = -200. / 30.;
    let slide = |rate: f32| drive().slide_to(15., on, 300., rate * hz);
    let push = motor_damping(&params);
    // At rest it is above the band and driven to `delta` with the strength cap.
    let (v, d) = slide(0.);
    assert!(
        (v - delta * hz * UNIT_SCALE).abs() < 1e-3 && d == push,
        "{v} {d}"
    );
    // Inside the band it is not pushed.
    assert_eq!(slide(-8.).1, 0.);
    // Too fast it is held to 1.5 × `err`, with k 0 (wired in mode 1) and so no push.
    let (v, d) = slide(-15.);
    assert!(
        (v - 1.5 * delta * hz * UNIT_SCALE).abs() < 1e-2 && d == 0.,
        "{v} {d}"
    );
}

#[test]
fn d1188_a_piston_with_a_strength_slides_with_or_without_k() {
    let params = |stiff, strength, animation_pattern| JointParams {
        stiff,
        strength,
        animation_pattern,
        ..Default::default()
    };
    // Rows 0x962590 (stiff) and 0x960988 (not stiff) share the band.
    assert!(lbp_sim::slides(&params(true, 0.5, 2), false));
    assert!(lbp_sim::slides(&params(false, 0.5, 2), false));
    // The push needs a strength; k (pattern 2, not wired in mode 1) does not matter.
    assert!(!lbp_sim::slides(&params(true, 0., 2), false));
    assert!(lbp_sim::slides(&params(true, 0.5, 0), false));
    let mode1 = JointParams {
        mode: 1,
        ..params(true, 0.5, 2)
    };
    assert!(lbp_sim::slides(&mode1, true));
}

#[test]
fn d1187_a_piston_rule_starts_from_its_saved_length() {
    use lbp_sim::scene::SceneJoint;
    let mut params = JointParams::default();
    params.motion[..5].copy_from_slice(&[315., 120., 0., 0.02, 20.]);
    let j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 840.,
        min_length: 100.,
        max_length: 200.,
        params,
    };
    // The base is the saved length (+0xf0), not the tweak minimum.
    assert_eq!((j.free_length(0.), j.free_length(70.)), (840., 1155.));
    assert_eq!(
        (j.range().min, j.range().max),
        (840. * lbp_sim::UNIT_SCALE, 1155. * lbp_sim::UNIT_SCALE)
    );
}
