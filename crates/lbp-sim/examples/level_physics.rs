//! R2 PC check (docs/sim-physics.md): does a level stand still in the sandbox?
//!
//! level_physics SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON [--ticks=N]
//!
//! Reads the level(s), builds the physics scene (bodies, static and dynamic shapes,
//! rejected shapes by reason) and runs it headless for N game ticks at 30 Hz (default 90,
//! three seconds). Reports, per level and in total, how far bodies moved from where the
//! level stored them: bodies moved more than 1 and more than 100 world units, the largest
//! move, and bodies whose position became non-finite. Fixed materials: physical materials
//! whose MAP path names dark matter (`dark_matter`/`darkmatter`, case-insensitive) make
//! their body static; the report lists the paths matched and the most used shape materials
//! so the rule can be checked. Counting unit: SHAPE Thing (scene)
//! and BODY Thing with a collider (motion). Counts only; no game bytes. Output: a NEW JSON
//! file under ignored `local-analysis/`.
use avian2d::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_sim::{
    build_scene_with, scene::MaterialRule, spawn_scene, to_level, PhysicsScene, SimPlugin,
    ThingIndex, ZSlab, TICK_HZ,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

fn main() {
    // The level reader recurses natively: run on a thread with the level-loader stack.
    let worker = std::thread::Builder::new()
        .stack_size(lbp_preview::level_scene::LEVEL_STACK_BYTES)
        .spawn(run)
        .expect("spawn the level_physics worker");
    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("level_physics: {e}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("level_physics: worker panicked");
            std::process::exit(1);
        }
    }
}

#[derive(Default)]
struct Motion {
    bodies: usize,
    moved_1: usize,
    moved_100: usize,
    non_finite: usize,
    max_move: f32,
    /// Bodies that moved more than 100: Thing and displacement.
    moved: Vec<(usize, [f32; 2])>,
    /// First tick (1-based) each body was more than 1 unit from its start.
    first_moved: BTreeMap<usize, usize>,
    /// Bodies seen asleep, and the first tick each was awake again after that.
    asleep: std::collections::BTreeSet<usize>,
    /// What a downward ray from each dynamic body meets after tick 1 (same slab, own
    /// colliders skipped).
    below: BTreeMap<usize, &'static str>,
    woke: BTreeMap<usize, usize>,
}

/// Ray this far below each dynamic body's origin (world units).
const BELOW_RAY: f32 = 1000.;

fn below_bodies(
    spatial: &SpatialQuery,
    bodies: &Query<(Entity, &ThingIndex, &RigidBody, &Position)>,
    colliders: &Query<(Entity, &ColliderOf, &ZSlab, &ThingIndex)>,
    sideways: &std::collections::BTreeSet<usize>,
) -> BTreeMap<usize, &'static str> {
    let mut out = BTreeMap::new();
    for (e, t, rb, pos) in bodies {
        if *rb != RigidBody::Dynamic {
            continue;
        }
        let own: Vec<ZSlab> = colliders
            .iter()
            .filter(|c| c.1.body == e)
            .map(|c| *c.2)
            .collect();
        let mut best: Option<(f32, Entity)> = None;
        spatial.ray_hits_callback(
            pos.0,
            Dir2::NEG_Y,
            BELOW_RAY * lbp_sim::UNIT_SCALE,
            true,
            &SpatialQueryFilter::default(),
            |h| {
                if let Ok((_, of, slab, _)) = colliders.get(h.entity) {
                    if of.body != e
                        && own.iter().any(|s| s.overlaps(slab))
                        && best.is_none_or(|b| h.distance < b.0)
                    {
                        best = Some((h.distance, h.entity));
                    }
                }
                true
            },
        );
        let what = match best {
            None => "nothing within 1000",
            Some((_, c)) => {
                let (_, of, _, ct) = colliders.get(c).expect("hit is a collider");
                match bodies.get(of.body).map(|b| *b.2) {
                    Ok(RigidBody::Dynamic) => "dynamic body",
                    _ if sideways.contains(&ct.0) => "static, sideways shape",
                    _ => "static",
                }
            }
        };
        out.insert(t.0, what);
    }
    out
}

fn simulate(scene: &PhysicsScene, ticks: usize) -> Motion {
    let start: BTreeMap<usize, [f32; 2]> = scene
        .bodies
        .iter()
        .filter(|b| {
            scene
                .colliders
                .iter()
                .any(|c| c.body.is_some_and(|i| scene.bodies[i].thing == b.thing))
        })
        .map(|b| (b.thing, b.frame.position))
        .collect();
    let owned = scene.clone();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, SimPlugin::default()))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1. / TICK_HZ,
        )))
        .add_systems(Startup, move |mut commands: Commands| {
            spawn_scene(&mut commands, &owned);
        });
    app.finish();
    let mut m = Motion::default();
    let sideways: std::collections::BTreeSet<usize> = scene
        .colliders
        .iter()
        .filter(|c| c.sideways)
        .map(|c| c.thing)
        .collect();
    for tick in 1..=ticks {
        app.update();
        if tick == 1 {
            let side = sideways.clone();
            m.below = app
                .world_mut()
                .run_system_once(move |spatial: SpatialQuery,
                      bodies: Query<(Entity, &ThingIndex, &RigidBody, &Position)>,
                      colliders: Query<(Entity, &ColliderOf, &ZSlab, &ThingIndex)>| {
                    below_bodies(&spatial, &bodies, &colliders, &side)
                })
                .unwrap_or_default();
        }
        let mut q = app
            .world_mut()
            .query_filtered::<(&ThingIndex, &Position, Has<Sleeping>), With<RigidBody>>();
        for (t, p, sleeping) in q.iter(app.world()) {
            if sleeping {
                m.asleep.insert(t.0);
            } else if m.asleep.contains(&t.0) {
                m.woke.entry(t.0).or_insert(tick);
            }
            if m.first_moved.contains_key(&t.0) {
                continue;
            }
            if let Some(s) = start.get(&t.0) {
                let p = to_level(p.0);
                if (p[0] - s[0]).powi(2) + (p[1] - s[1]).powi(2) > 1. {
                    m.first_moved.insert(t.0, tick);
                }
            }
        }
    }
    let mut q = app
        .world_mut()
        .query_filtered::<(&ThingIndex, &Position), With<RigidBody>>();
    for (t, p) in q.iter(app.world()) {
        let Some(s) = start.get(&t.0) else {
            continue;
        };
        m.bodies += 1;
        let p = to_level(p.0);
        let d = ((p[0] - s[0]).powi(2) + (p[1] - s[1]).powi(2)).sqrt();
        if !d.is_finite() {
            m.non_finite += 1;
            continue;
        }
        m.moved_1 += usize::from(d > 1.);
        m.moved_100 += usize::from(d > 100.);
        if d > 100. {
            m.moved.push((t.0, [p[0] - s[0], p[1] - s[1]]));
        }
        m.max_move = m.max_move.max(d);
    }
    m
}

fn run() -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let ticks: usize = all
        .iter()
        .find_map(|a| a.strip_prefix("--ticks="))
        .map_or(Ok(90), str::parse)
        .map_err(|e| format!("--ticks: {e}"))?;
    let args: Vec<&String> = all.iter().filter(|a| !a.starts_with("--ticks=")).collect();
    if args.len() != 5 {
        return Err(
            "usage: level_physics SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON [--ticks=N]".into(),
        );
    }
    let output = Path::new(args[4]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(args[2]), &catalog, &control)?;
    let mut paths = BTreeMap::<u32, String>::new();
    for r in &catalog.snapshot.resources {
        if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
            paths.entry(g).or_insert_with(|| r.map_row.path.clone());
        }
    }
    // Material rules by MAP path (INFERRED until RMaterial is read): dark matter and
    // infinite-mass materials are fixed, hologram does not collide.
    let rule_of_path = |p: &str| {
        let p = p.to_ascii_lowercase().replace(' ', "_");
        if !p.ends_with(".mat") {
            MaterialRule::Normal
        } else if p.contains("dark_matter")
            || p.contains("darkmatter")
            || p.contains("infinite_mass")
        {
            MaterialRule::Fixed
        } else if p.contains("hologram") {
            MaterialRule::NoCollide
        } else {
            MaterialRule::Normal
        }
    };
    let rules: BTreeMap<u32, MaterialRule> = paths
        .iter()
        .map(|(g, p)| (*g, rule_of_path(p)))
        .filter(|(_, r)| *r != MaterialRule::Normal)
        .collect();
    let fixed_guids: std::collections::BTreeSet<u32> = rules
        .iter()
        .filter(|(_, r)| **r == MaterialRule::Fixed)
        .map(|(g, _)| *g)
        .collect();
    let no_collide_guids: Vec<u32> = rules
        .iter()
        .filter(|(_, r)| **r == MaterialRule::NoCollide)
        .map(|(g, _)| *g)
        .collect();
    let fixed = |d: &lbp_level::serial::Descriptor| {
        d.guid
            .and_then(|g| rules.get(&g).copied())
            .unwrap_or_default()
    };
    let material_name = |d: &lbp_level::serial::Descriptor| match d.guid {
        Some(g) => paths.get(&g).cloned().unwrap_or_else(|| format!("g{g:x}")),
        None => "by hash".to_string(),
    };
    let mut materials = BTreeMap::<String, usize>::new();
    let (mut fixed_bodies, mut nested_bodies) = (0, 0);
    let mut keys = Vec::new();
    if args[3] == "--all" {
        let mut hashes = BTreeMap::new();
        for r in &catalog.snapshot.resources {
            if r.display_type() == "level" {
                hashes
                    .entry(r.map_row.sha1.to_ascii_lowercase())
                    .or_insert_with(|| r.map_row.path.clone());
            }
        }
        for (h, path) in hashes {
            if let Ok(key) = lbp_preview::resolve(&catalog, None, Some(&h)) {
                keys.push((key, path));
            }
        }
    } else {
        keys.push(lbp_preview::level_scene::resolve_level(
            &catalog,
            None,
            Some(args[3]),
        )?);
    }
    let mut levels = Vec::new();
    let mut total = Motion::default();
    let (mut shapes, mut dynamic, mut static_, mut not_collidable, mut not_read) = (0, 0, 0, 0, 0);
    let mut rejected = BTreeMap::<&str, usize>::new();
    let mut filled = BTreeMap::<&str, usize>::new();
    let (mut joints, mut rejected_joints) = (0, BTreeMap::<&str, usize>::new());
    let mut panicked = Vec::new();
    // Why bodies move: the same keys over every dynamic body and over those that moved >100.
    let mut why = BTreeMap::<&str, (BTreeMap<String, usize>, BTreeMap<String, usize>)>::new();
    let count = keys.len();
    for (n, (key, label)) in keys.into_iter().enumerate() {
        // Progress on stderr so a crash names the level it was in.
        eprintln!("level {}/{count}: {label}", n + 1);
        let read =
            lbp_preview::level_scene::read_level_bytes(&source, &key, &control).and_then(|b| {
                lbp_level::read(&b, lbp_level::Limits::default(), &|| false)
                    .map_err(|e| e.to_string())
            });
        let Ok(report) = read else {
            not_read += 1;
            continue;
        };
        for t in &report.things {
            if let Some(m) = t.data.shape.as_ref().and_then(|s| s.material.as_ref()) {
                *materials.entry(material_name(m)).or_default() += 1;
            }
        }
        let scene = build_scene_with(&report.things, &fixed);
        let r = &scene.report;
        fixed_bodies += r.fixed_bodies;
        nested_bodies += r.nested_bodies;
        shapes += r.shapes;
        dynamic += r.dynamic;
        static_ += r.static_;
        not_collidable += r.not_collidable;
        for (k, n) in &r.rejected {
            *rejected.entry(k).or_default() += n;
        }
        for (k, n) in &r.filled {
            *filled.entry(k).or_default() += n;
        }
        joints += r.joints;
        for (k, n) in &r.rejected_joints {
            *rejected_joints.entry(k).or_default() += n;
        }
        let run =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| simulate(&scene, ticks)));
        let Ok(m) = run else {
            panicked.push(label.clone());
            continue;
        };
        let moved: BTreeMap<usize, [f32; 2]> = m.moved.iter().copied().collect();
        for (bi, b) in scene.bodies.iter().enumerate() {
            if b.fixed {
                continue;
            }
            let t = &report.things[b.thing];
            let shapes: Vec<_> = scene
                .colliders
                .iter()
                .filter(|c| c.body == Some(bi))
                .collect();
            if shapes.is_empty() {
                continue;
            }
            let material = report.things[shapes[0].thing]
                .data
                .shape
                .as_ref()
                .and_then(|s| s.material.as_ref())
                .map_or("none".to_string(), material_name);
            let jointed = scene
                .joints
                .iter()
                .any(|j| [j.a, j.b].contains(&lbp_sim::scene::Endpoint::Body(bi)));
            let first = match m.first_moved.get(&b.thing) {
                None => "never",
                Some(1) => "1",
                Some(2..=5) => "2-5",
                Some(6..=30) => "6-30",
                Some(_) => "31+",
            };
            let bucket = |t: usize| match t {
                0..=5 => "woke 1-5",
                6..=30 => "woke 6-30",
                _ => "woke 31+",
            };
            let sleep = match (m.asleep.contains(&b.thing), m.woke.get(&b.thing)) {
                (false, _) => "never asleep",
                (true, None) => "asleep to the end",
                (true, Some(&t)) => bucket(t),
            };
            let keys = [
                (
                    "below_at_start",
                    m.below
                        .get(&b.thing)
                        .copied()
                        .unwrap_or("not a body")
                        .to_string(),
                ),
                ("first_moved_tick", first.to_string()),
                ("sleep", sleep.to_string()),
                ("frozen", b.frozen.to_string()),
                ("material", material),
                ("parts", t.parts.join("+")),
                ("jointed", jointed.to_string()),
                ("shapes", shapes.len().min(5).to_string()),
                ("has_parent", t.parent.is_some().to_string()),
                (
                    "direction",
                    match moved.get(&b.thing) {
                        Some(d) if d[1] < -100. => "down",
                        Some(_) => "other",
                        None => "stayed",
                    }
                    .to_string(),
                ),
            ];
            for (k, v) in keys {
                let e = why.entry(k).or_default();
                *e.0.entry(v.clone()).or_default() += 1;
                if moved.contains_key(&b.thing) {
                    *e.1.entry(v).or_default() += 1;
                }
            }
        }
        total.bodies += m.bodies;
        total.moved_1 += m.moved_1;
        total.moved_100 += m.moved_100;
        total.non_finite += m.non_finite;
        total.max_move = total.max_move.max(m.max_move);
        levels.push(serde_json::json!({
            "level": label,
            "stopped": report.stop.is_some(),
            "shapes": r.shapes, "dynamic": r.dynamic, "static": r.static_,
            "not_collidable": r.not_collidable, "rejected": r.rejected, "filled": r.filled,
            "flat_samples": r.flat_samples,
            "joints": r.joints, "rejected_joints": r.rejected_joints,
            "fixed_bodies": r.fixed_bodies, "nested_bodies": r.nested_bodies,
            "bodies": m.bodies, "moved_over_1": m.moved_1, "moved_over_100": m.moved_100,
            "non_finite": m.non_finite, "max_move": m.max_move,
        }));
    }
    let mut top: Vec<(&String, &usize)> = materials.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    top.truncate(40);
    let fixed_paths: Vec<&String> = fixed_guids.iter().filter_map(|g| paths.get(g)).collect();
    let mut most_moved: Vec<(&str, u64)> = levels
        .iter()
        .filter_map(|l| Some((l["level"].as_str()?, l["moved_over_100"].as_u64()?)))
        .filter(|l| l.1 > 0)
        .collect();
    most_moved.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    most_moved.truncate(15);
    let doc = serde_json::json!({
        "levels_most_moved_top15": most_moved,
        "schema_version": 1,
        "unit": "SHAPE Thing (scene); BODY Thing with a collider (motion), world units",
        "ticks": ticks,
        "tick_hz": TICK_HZ,
        "levels": levels.len(),
        "levels_not_read": not_read,
        "shapes": shapes, "dynamic": dynamic, "static": static_,
        "not_collidable": not_collidable, "rejected": rejected, "filled": filled,
        "joints": joints, "rejected_joints": rejected_joints,
        "fixed_bodies": fixed_bodies, "nested_bodies": nested_bodies,
        "fixed_material_paths": fixed_paths,
        "no_collide_material_paths": no_collide_guids.iter().filter_map(|g| paths.get(g)).collect::<Vec<_>>(),
        "levels_panicked": panicked,
        "why_moved": why.iter().map(|(k, (all, moved))| {
            let top = |m: &BTreeMap<String, usize>| {
                let mut v: Vec<_> = m.iter().collect();
                v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                v.truncate(20);
                v.into_iter().map(|(k, n)| serde_json::json!([k, n])).collect::<Vec<_>>()
            };
            (k.to_string(), serde_json::json!({"dynamic_bodies": top(all), "moved_over_100": top(moved)}))
        }).collect::<serde_json::Map<_, _>>(),
        "shape_materials_top40": top,
        "bodies": total.bodies, "moved_over_1": total.moved_1,
        "moved_over_100": total.moved_100, "non_finite": total.non_finite,
        "max_move": total.max_move,
        "per_level": levels,
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "levels {} (not read {not_read}, panicked {}); shapes {shapes}: dynamic {dynamic}, static {static_}, \
         not collidable {not_collidable}, rejected {}, filled {}; joints {joints} (rejected {}); \
         fixed bodies {fixed_bodies} ({} fixed materials), nested bodies {nested_bodies}; \
         after {ticks} ticks: bodies {}, moved >1 {}, >100 {}, non-finite {}, max move {:.1}",
        levels.len(),
        panicked.len(),
        rejected.values().sum::<usize>(),
        filled.values().sum::<usize>(),
        rejected_joints.values().sum::<usize>(),
        fixed_paths.len(),
        total.bodies,
        total.moved_1,
        total.moved_100,
        total.non_finite,
        total.max_move
    );
    Ok(())
}
