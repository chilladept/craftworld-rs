//! R2 sandbox on authored synthetic shapes (no game bytes): scene rules, then avian2d runs
//! at 30 Hz. docs/sim-physics.md.
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_level::parts::{Body, Joint, JointParams, Pos, Shape, ShapeColor};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_sim::scene::{
    build_scene, build_scene_with, Endpoint, JointKind, MaterialRule, PhysicsScene, DENSITY,
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
    let params = JointParams {
        motion: [0., 0., 0., 3., 0., 0., 0., 0., 0., 0.],
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
fn floor_turned_on_its_side_still_holds_a_box() {
    // Rotated 90 degrees about x (as the d198 DLC samples): local y runs along world z and
    // the extrusion (local z, half depth 50) along world -y. The outline is a line in xy,
    // so the collider is the slab's side view: 4000 wide, 100 tall, z -100..100.
    let mut world = matrix(0., -50., 0., 0., 1., 1.);
    world[4..7].copy_from_slice(&[0., 0., 1.]);
    world[8..11].copy_from_slice(&[0., -1., 0.]);
    let mut shape = rect(4000., 200., 7);
    shape.thickness = 50.;
    let things = vec![
        thing(0, world, Some(shape), false),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
    ];
    let scene = build_scene(&things);
    assert_eq!(scene.report.filled.get("sideways prism"), Some(&1));
    assert!(
        scene.report.rejected.is_empty(),
        "{:?}",
        scene.report.rejected
    );
    let floor = &scene.colliders[0];
    assert!(floor.z.abs() < 1e-3 && (floor.half_thickness - 100.).abs() < 1e-3);
    let p = run(&things, 120)[1].1;
    assert!((p[1] - 50.).abs() < 5., "rests on top of the floor: {p:?}");
}
