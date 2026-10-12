//! `--play-sackboy` (docs/sim-physics.md, Part 4): with a level, the level's shapes as an
//! avian world (lbp-sim, material rules by MAP path) and Sackboy as the kinematic capsule
//! driven by the RPCS3-measured movement rules at 30 Hz. Arrow keys or the left stick walk
//! and change lanes (Up = lane behind, Down = lane in front), Space or the South button
//! jumps; the camera follows him as the game's default camera does (Moon level measurement).
//! He is drawn as his own body mesh ([`SACKBOY_MESH`], or the `--select-guid` mesh) standing
//! on the capsule (feet at its bottom), turned by the facing angle and posed from the
//! weighted clip layers `lbp_sim::sackboy_anim::SackboyAnim` gives each tick (EBOOT rules,
//! D232, docs/d231-sackboy-animations.md), blended per bone from a clip bank sharing one
//! rig. With `--play-animation` that one clip cycles at the walk speed instead (facing
//! still from the picker); without a mesh, a placeholder capsule. The level's own dynamic objects are simulated but not redrawn.
//!
//! `--play-sackboy-at X,Y,Z` sets the spawn (level units, his position: 69.9 above the feet); otherwise the
//! first CHECKPOINT's POS plus [`SPAWN_OFFSET`]. `--play-sackboy-input STEPS` replaces the pad with a script of
//! `keys:ticks` steps (keys from `l r j b f`, `-` for none; e.g. `r:40,rj:12,-:30`) and prints
//! Sackboy's position every tick, for comparing paths against RPCS3.
//!
//! Moods (D233, docs/d233-sackboy-moods-headlook.md): I / K / L / J are the mood d-pad up
//! (happy) / down (sad) / right (angry) / left (scared); in a script the capitals `U D R L`
//! hold that direction (one press per step: `U:1,-:40,U:1`).
use bevy::prelude::*;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::level_scene;
use lbp_preview::skinned::{sample_pose, BlendPose, BoundAnimation, ScaleRule, SkinnedClip};
use lbp_sim::avian2d::prelude::*;
use lbp_sim::character::{
    spawn_sackboy, SackboyBody, SackboyPlugin, CENTRE_ABOVE, FLOOR_REST, HALF_SEGMENT, HEIGHT,
    RADIUS,
};
use lbp_sim::sackboy::{Input, Mood, WALK_SPEED};
use lbp_sim::sackboy_anim::{AnimFrame, Blend, ClipId, SackboyAnim};
use lbp_sim::scene::{material_rule_for_path, MaterialRule};
use lbp_sim::{
    build_scene_with, build_spawn_scene_with, spawn_scene, to_level, PhysicsScene, SimPlugin,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Level units to display units, set when the level is drawn (`LevelPlan::normalize`).
#[derive(Resource, Clone, Copy)]
pub(crate) struct LevelDisplay(pub Mat4);

#[derive(Resource)]
struct Play {
    scene: Option<PhysicsScene>,
    spawn: [f32; 3],
    script: Option<Vec<(Input, usize)>>,
    tick: usize,
}

#[derive(Component)]
pub(crate) struct Marker;

/// The Sackboy mesh standing on the capsule: `upright` turns the raw Z-up model to the
/// display (feet at the origin, level-unit scale); `facing` is the turn about the display Y
/// axis, eased between the picker's last two ticks (`yaw`); `phase` the `--play-animation`
/// clip's frame. With the clip bank (`banked`), stand is `Playing`'s own clip and `bank`
/// holds the others by [`ClipId::index`] (None for stand or a clip that failed to load);
/// `anim` is the picker's output for the latest tick, `picked`/`posed` count the ticks
/// picked and blended.
#[derive(Component)]
pub(crate) struct Figure {
    upright: Mat4,
    facing: f32,
    phase: f32,
    banked: bool,
    bank: Vec<Option<BoundAnimation>>,
    picker: SackboyAnim,
    anim: Option<AnimFrame>,
    yaw: [f32; 2],
    picked: u64,
    posed: u64,
}

/// Sackboy's body mesh, the play figure when `--select-guid` is not given: `0000043f`
/// `gamedata/mesh_library/player_character/sack_boy.mol` (FOUND, MAP and the EBOOT GUID
/// block). The default "naked" costume pieces add no geometry (INFERRED; plan section 1).
pub(crate) const SACKBOY_MESH: &str = "0000043f";

/// Reads the mesh once and binds every [`ClipId::ALL`] clip to its rig; the first
/// (stand) is the playing clip, the rest come back in order. Runs on the load worker.
pub(crate) fn load_bank(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    mesh_key: &lbp_catalog::attachment::LocationKey,
    control: &LoadControl,
) -> Result<(SkinnedClip, Vec<Result<BoundAnimation, String>>), String> {
    let guids = ClipId::ALL.map(ClipId::guid);
    let (mut clip, mut bank) =
        lbp_preview::skinned::load_clip_bank(source, catalog, mesh_key, &guids, control)?;
    // D246: Sackboy's Biped bones do not pass their scale on (the stand clip breathes by
    // scaling the spine; inherited, that stretched the whole upper body).
    clip.rig.scale_rule = ScaleRule::Biped;
    for bound in bank.iter_mut().flatten() {
        bound.rig.scale_rule = ScaleRule::Biped;
    }
    Ok((clip, bank))
}

/// True when the viewer was started with `--play-sackboy`.
pub(crate) fn enabled() -> bool {
    std::env::args().any(|a| a == "--play-sackboy")
}

/// The pose for one tick's layers: from bind, each lerp layer moves the bones it drives
/// toward its clip's (sub-frame) pose by its weight, each additive layer adds its frame
/// minus its reference frame. Stand is the playing clip; a clip missing from the bank is
/// skipped. A layer with the game's mirror flag is mirrored left/right through the
/// skeleton's mirror table (moving left: walk, jump_walk, stop/turn).
fn blend_frame(
    stand: &SkinnedClip,
    bank: &[Option<BoundAnimation>],
    frame: &AnimFrame,
) -> BlendPose {
    let mut pose = BlendPose::bind(&stand.rig);
    for l in &frame.layers {
        let (rig, animation) = if l.clip == ClipId::Stand {
            (&stand.rig, &stand.animation)
        } else {
            match bank.get(l.clip.index()) {
                Some(Some(b)) => (&b.rig, &b.animation),
                _ => continue,
            }
        };
        let Some(p) = sample_pose(animation, l.frame) else {
            continue;
        };
        match l.blend {
            Blend::Lerp => {
                if l.mirror {
                    pose.lerp_mirrored(rig, &p, l.weight);
                } else {
                    pose.lerp(rig, &p, l.weight);
                }
            }
            Blend::Additive { reference } => {
                if let Some(r) = sample_pose(animation, reference) {
                    pose.add(rig, &p, &r, l.weight);
                }
            }
        }
    }
    // The clips' root rotation is the facing channel the game overwrites (stand 0, walk
    // and idle-chain clips +90 degrees): kept, it doubled the picker's yaw (walking right
    // showed his back) and snapped 90 degrees wherever stand met another clip.
    pose.hold_root_rotation(&stand.rig);
    pose
}

/// Spawns the Sackboy mesh (`--select-guid`) as the play figure and, with a clip, its rig
/// and playback (frames chosen by [`show_sackboy`]). Model units are level units (as for
/// the Pod figure), so `k` is the level display scale.
pub(crate) fn spawn_figure(
    v: &mut crate::Viewer,
    (commands, meshes, materials, images): (
        &mut Commands,
        &mut Assets<Mesh>,
        &mut Assets<crate::ViewerMaterial>,
        &mut Assets<Image>,
    ),
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    k: f32,
    preview: &lbp_preview::GroupedMesh,
    (clip, bank): (
        Option<crate::ClipResult>,
        Option<Vec<Result<BoundAnimation, String>>>,
    ),
) -> Result<String, String> {
    let [lo, hi] =
        crate::adapter_bounds(&preview.geometry.positions).ok_or("Unsupported: empty figure")?;
    // Raw Z-up model: feet are the lowest z, centred in x and y (depth).
    let origin = Mat4::from_translation(Vec3::new(
        -(lo[0] + hi[0]) / 2.,
        -(lo[1] + hi[1]) / 2.,
        -lo[2],
    ));
    let upright =
        Mat4::from_scale(Vec3::splat(k)) * Mat4::from_quat(crate::model_up(true)) * origin;
    // Clip bank: stand is the playing clip; the others are blended in by ClipId index.
    let banked = bank.is_some() && matches!(clip, Some(Ok(_)));
    let mut slots: Vec<Option<BoundAnimation>> = ClipId::ALL.iter().map(|_| None).collect();
    let mut bank_line = String::new();
    if let (true, Some(bank)) = (banked, bank) {
        let mut missing = Vec::new();
        let mut odd = Vec::new();
        for (id, bound) in ClipId::ALL[1..].iter().zip(bank) {
            match bound {
                Ok(b) => {
                    if b.report.frames != id.frames() {
                        odd.push(format!("{} {} frames", id.name(), b.report.frames));
                    }
                    slots[id.index()] = Some(b);
                }
                Err(e) => missing.push(format!("{} {e}", id.name())),
            }
        }
        bank_line = format!(
            "; clip bank {}/{} bound to one rig, blended by the EBOOT layer rules",
            ClipId::ALL.len() - missing.len(),
            ClipId::ALL.len()
        );
        if !missing.is_empty() {
            bank_line += &format!("; not loaded: {}", missing.join(", "));
        }
        if !odd.is_empty() {
            bank_line += &format!("; frame counts differ from the census: {}", odd.join(", "));
        }
    }
    let figure = commands
        .spawn((
            Transform::from_matrix(upright),
            Visibility::default(),
            Figure {
                upright,
                // Standing faces the camera (FOUND, rules note section 2).
                facing: 0.,
                phase: 0.,
                banked,
                bank: slots,
                picker: SackboyAnim::new(),
                anim: None,
                yaw: [0.; 2],
                picked: 0,
                posed: 0,
            },
        ))
        .id();
    let mut evidence = crate::browser::PreviewEvidence::default();
    let mut details = Vec::new();
    let first = v.scene.len();
    for p in crate::prepare(preview, false, &mut evidence, &mut details)? {
        let (mesh, material, image) = crate::upload(p, meshes, materials, images);
        let entity = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                ChildOf(figure),
            ))
            .id();
        v.scene.push((entity, mesh, material, image));
    }
    let mut line = format!(
        "Play figure: model height {:.0} level units (capsule {:.0})",
        hi[2] - lo[2],
        HEIGHT
    );
    let scene: Vec<_> = v.scene[first..]
        .iter()
        .map(|s| (s.0, s.1.clone()))
        .collect();
    match clip {
        None => line += "; no clip",
        Some(Err(e)) => line += &format!("; clip not played: {e}"),
        Some(Ok(clip)) => {
            // Raw model units: the identity display frame.
            let frame = ([0.; 3], 1.);
            let gpu = if v.cpu_skinning {
                None
            } else {
                let rig = crate::gpu_rig(
                    commands,
                    meshes,
                    bindposes,
                    &clip.rig,
                    (&clip.skin.joints, &clip.skin.weights),
                    (frame, Quat::IDENTITY),
                    &scene,
                )?;
                commands.entity(rig.root).insert(ChildOf(figure));
                Some(crate::GpuRig {
                    morphs: !clip.skin.morphs.is_empty() && clip.report.animation_morphs > 0,
                    ..rig
                })
            };
            line += &format!(
                "; {} clip {} frames at {} fps{bank_line}",
                if banked { "stand" } else { "--play-animation" },
                clip.report.frames,
                clip.report.fps
            );
            v.animation_frame = Some(0);
            v.playing = Some(crate::Playing {
                clip,
                frame,
                started: -1.,
                shown: None,
                gpu,
                meshes: scene.into_iter().map(|s| s.1).collect(),
                alt: None,
                blended: None,
                layered: banked,
            });
        }
    }
    Ok(line)
}

/// The walk clip's next frame position: advances at the clip rate scaled by walking speed
/// (full rate at [`WALK_SPEED`]) while grounded and moving; otherwise back to frame 0.
/// INFERRED pacing (RPCS3 stride timing to check).
fn walk_phase(phase: f32, dt: f32, fps: f32, vx: f32, grounded: bool) -> f32 {
    let speed = (vx.abs() / WALK_SPEED).min(1.);
    if grounded && speed > 0.05 {
        phase + dt * fps * speed
    } else {
        0.
    }
}

/// Sackboy's start relative to the first CHECKPOINT POS. RPCS3, Rookie Test (PC, d-run
/// 2026-10-09): checkpoint (-13335, 6971, -250), Sackboy appears at (-13335.7, 6763, 0) and
/// lands at y 5974. One level only: INFERRED for other levels.
pub(crate) const SPAWN_OFFSET: [f32; 3] = [0., -208., 250.];

/// Sackboy's x relative to the first CHECKPOINT POS when he spawns there; he appears at the
/// checkpoint's height and falls to the floor below. RPCS3, Moon level play-mode spawn (PC,
/// 2026-10-09, local-analysis/results/2026-10-09-moon-movement.md, "Addendum: play-mode
/// checkpoint spawn"): checkpoint (-18904.74, 1798.11, -470), first sample at +(-0.36,
/// +0.07), settles at +(-1.11, -73.82) after 23 ticks. The Pod (shell 000027b9,
/// local-analysis/results/2026-10-09-pod-spawn-memory.md): checkpoint (-3.05, 540.76, -50),
/// Sackboy at (-5.37, -477.97) in lane 0, 1018 below it, so only x and the drop carry over,
/// not the Moon's y. INFERRED for other levels.
pub(crate) const DATA_SPAWN_X: f32 = -1.11;

/// Lane centres (world z): front 0, middle -200, back -400 (Moon level, same results file).
const LANES: [f32; 3] = [0., -200., -400.];

/// The lane Sackboy spawns in for a checkpoint at depth `z`: the nearest lane centre. Moon:
/// checkpoint z -470 -> back lane -400. INFERRED (one checkpoint; Rookie Test's z 0 from a
/// -250 checkpoint does not fit this rule and was not taken in play mode).
pub(crate) fn spawn_lane(z: f32) -> f32 {
    LANES
        .into_iter()
        .min_by(|a, b| (a - z).abs().total_cmp(&(b - z).abs()))
        .unwrap_or(0.)
}

/// The default game camera's eye relative to Sackboy's position (level units), looking
/// straight down -z (no pitch), vertical FOV [`FOLLOW_FOV_DEG`]. RPCS3 memory read in the
/// owner's blank Moon level (PC run d208, idle, one pose): CONFIRMED_FROM_LOCAL_DATA
/// (emulated) for that pose; whether the x lead follows his facing or lags while walking is
/// not measured (INFERRED: held constant).
const FOLLOW_EYE: [f32; 3] = [296., 168., 1218.];
const FOLLOW_FOV_DEG: f32 = 60.;

fn value(all: &[String], flag: &str) -> Option<String> {
    all.iter()
        .position(|a| a == flag)
        .and_then(|i| all.get(i + 1))
        .cloned()
}

fn parse_script(text: &str) -> Result<Vec<(Input, usize)>, String> {
    text.split(',')
        .map(|step| {
            let (keys, ticks) = step
                .split_once(':')
                .ok_or_else(|| format!("--play-sackboy-input: {step:?} needs keys:ticks"))?;
            let ticks = ticks
                .trim()
                .parse::<usize>()
                .map_err(|e| format!("--play-sackboy-input: {step:?}: {e}"))?;
            let mut input = Input::default();
            for c in keys.trim().chars() {
                match c {
                    'l' => input.x = -1.,
                    'r' => input.x = 1.,
                    'j' => input.jump = true,
                    'b' => input.back = true,
                    'f' => input.front = true,
                    'U' => input.mood = Some(Mood::Happy),
                    'D' => input.mood = Some(Mood::Sad),
                    'R' => input.mood = Some(Mood::Angry),
                    'L' => input.mood = Some(Mood::Scared),
                    '-' => {}
                    _ => return Err(format!("--play-sackboy-input: unknown key {c:?}")),
                }
            }
            Ok((input, ticks))
        })
        .collect()
}

/// Non-normal material rules by descriptor GUID, from the MAP paths.
fn material_rules(catalog: &lbp_catalog::Catalog) -> BTreeMap<u32, MaterialRule> {
    let mut rules = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
            let rule = material_rule_for_path(&r.map_row.path);
            if rule != MaterialRule::Normal {
                rules.entry(g).or_insert(rule);
            }
        }
    }
    rules
}

/// Where a level puts Sackboy's feet, from its data alone: the first CHECKPOINT's POS
/// moved by [`DATA_SPAWN_X`] into the [`spawn_lane`] lane, dropped from the checkpoint's
/// height to the highest collider top below in that slab (the Pod figure, in place of the
/// screenshot fit). Pod: feet at y -547, i.e. the measured position row -478 less the 69
/// from feet to Sackboy's position (`lbp_sim::character`). Level units; and a line for the status. Runs on the
/// level worker (deep reader stack).
pub(crate) fn level_spawn_feet(
    source: &AttachedSource,
    catalog: &lbp_catalog::Catalog,
    key: &lbp_catalog::attachment::LocationKey,
    control: &LoadControl,
) -> Result<([f32; 3], String), String> {
    let bytes = level_scene::read_level_bytes(source, key, control)?;
    let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let (thing, m) = *level_scene::checkpoints(&report.things)
        .first()
        .ok_or("no CHECKPOINT")?;
    let rules = material_rules(catalog);
    let scene = build_spawn_scene_with(&report.things, &|d| {
        d.guid
            .and_then(|g| rules.get(&g).copied())
            .unwrap_or_default()
    });
    let [x, y, z] = [m[12] + DATA_SPAWN_X, m[13], spawn_lane(m[14])];
    let floor = scene
        .floor_below([x, y], z)
        .ok_or_else(|| format!("no floor below ({x:.0}, {y:.0}) in slab z {z:.0}"))?;
    Ok((
        [x, floor, z],
        format!(
            "data spawn: checkpoint Thing {thing} ({:.0}, {:.0}, {:.0}) x {DATA_SPAWN_X:+.2}, dropped in lane z {z:.0} to floor y {floor:.0}",
            m[12], m[13], m[14]
        ),
    ))
}

/// Reads the level and adds physics and Sackboy when `--play-sackboy` is given.
pub fn install(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    if !all.iter().any(|a| a == "--play-sackboy") {
        return Ok(());
    }
    let snapshot = value(&all, "--snapshot").ok_or("--play-sackboy requires --snapshot")?;
    let game = value(&all, "--game-dir").ok_or("--play-sackboy requires --game-dir")?;
    let guid = value(&all, "--level-guid")
        .map(|g| u32::from_str_radix(&g, 16).map_err(|_| "--level-guid requires hex digits"))
        .transpose()?;
    let path = value(&all, "--level-path");
    if guid.is_none() && path.is_none() {
        return Err("--play-sackboy requires --level-guid or --level-path".into());
    }
    let at = value(&all, "--play-sackboy-at")
        .map(|v| {
            let p: Vec<f32> = v
                .split(',')
                .map(|x| x.trim().parse::<f32>())
                .collect::<Result<_, _>>()
                .map_err(|e| format!("--play-sackboy-at: {e}"))?;
            <[f32; 3]>::try_from(p).map_err(|_| "--play-sackboy-at needs X,Y,Z".to_string())
        })
        .transpose()?;
    let script = value(&all, "--play-sackboy-input")
        .map(|s| parse_script(&s))
        .transpose()?;
    // The level reader recurses deeply: same worker stack as the level scene.
    let (scene, checkpoints, header, popit) = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<_, String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(Path::new(&snapshot), &control)?;
            let source = AttachedSource::attach(Path::new(&game), &catalog, &control)?;
            let rules = material_rules(&catalog);
            let (key, identity) = level_scene::resolve_level(&catalog, guid, path.as_deref())?;
            let bytes = level_scene::read_level_bytes(&source, &key, &control)?;
            let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
                .map_err(|e| e.to_string())?;
            let scene = build_scene_with(&report.things, &|d| {
                d.guid
                    .and_then(|g| rules.get(&g).copied())
                    .unwrap_or_default()
            });
            let r = &scene.report;
            let header = format!(
                "Play Sackboy: {identity}; shapes {}: dynamic {}, static {}, not collidable {}, rejected {}; joints {}",
                r.shapes,
                r.dynamic,
                r.static_,
                r.not_collidable,
                r.rejected.values().sum::<usize>(),
                r.joints
            );
            let popit = crate::popit_play::load_assets(&source, &catalog, &control)?;
            Ok((
                scene,
                level_scene::checkpoints(&report.things),
                header,
                popit,
            ))
        })
        .map_err(|e| format!("play sackboy thread: {e}"))?
        .join()
        .map_err(|_| "play sackboy thread panicked".to_string())??;
    let spawn = match (at, checkpoints.first()) {
        (Some(p), _) => p,
        (None, Some((_, m))) => {
            let mut p = [
                m[12] + SPAWN_OFFSET[0],
                m[13] + SPAWN_OFFSET[1],
                m[14] + SPAWN_OFFSET[2],
            ];
            // The Rookie Test offset drops him 208 below the checkpoint; where the floor
            // under the checkpoint is higher than that (blank level: top 1655, his feet at
            // 1520), he would start inside it (no lane changes, drawn sunk into the floor):
            // stand him on it instead.
            if let Some(top) = scene.floor_below([p[0], m[13]], p[2]) {
                p[1] = p[1].max(top + FLOOR_REST);
            }
            p
        }
        (None, None) => return Err("--play-sackboy: no CHECKPOINT; give --play-sackboy-at".into()),
    };
    let points: Vec<String> = checkpoints
        .iter()
        .map(|(t, m)| format!("Thing {t} ({:.0}, {:.0}, {:.0})", m[12], m[13], m[14]))
        .collect();
    println!(
        "{header}; checkpoints [{}]; spawn ({:.0}, {:.0}, {:.0})",
        points.join(", "),
        spawn[0],
        spawn[1],
        spawn[2]
    );
    crate::popit_play::install(app, popit);
    app.add_plugins((SimPlugin::default(), SackboyPlugin))
        .insert_resource(Play {
            scene: Some(scene),
            spawn,
            script,
            tick: 0,
        })
        .add_systems(Update, (spawn_world, read_pad, show_sackboy).chain())
        .add_systems(FixedFirst, script_input)
        .add_systems(FixedLast, (tick_anim, log_scripted).chain());
    if let Some(spec) = value(&all, "--qa-record") {
        let parts: Vec<&str> = spec.split(',').collect();
        let num = |i: usize, d: f32| -> Result<f32, String> {
            parts.get(i).map_or(Ok(d), |s| {
                s.trim()
                    .parse::<f32>()
                    .map_err(|e| format!("--qa-record: {e}"))
            })
        };
        let dir = std::path::PathBuf::from(parts[0]);
        if !dir.starts_with("local-analysis") || dir.exists() {
            return Err("--qa-record DIR must be a new directory under local-analysis".into());
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        app.insert_resource(Record {
            dir,
            start: num(1, 1300.)? as usize,
            end: num(2, 1400.)? as usize,
            speed: num(3, 0.25)?,
            last: 0,
        })
        .add_systems(Update, record.after(show_sackboy));
    }
    Ok(())
}

/// `--qa-record DIR,START,END[,SPEED]` (QA only): one screenshot per scripted tick in
/// START..=END into DIR/tick_NNNN.png, game time slowed to SPEED meanwhile so every tick
/// is drawn, then exit.
#[derive(Resource)]
struct Record {
    dir: std::path::PathBuf,
    start: usize,
    end: usize,
    speed: f32,
    last: usize,
}

fn record(
    mut commands: Commands,
    play: Res<Play>,
    mut rec: ResMut<Record>,
    mut time: ResMut<Time<Virtual>>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let t = play.tick;
    if t + 30 >= rec.start && t <= rec.end {
        time.set_relative_speed(rec.speed);
    }
    if t > rec.end + 10 {
        exit.write(AppExit::Success);
        return;
    }
    if t >= rec.start && t <= rec.end && t != rec.last {
        rec.last = t;
        let path = rec.dir.join(format!("tick_{t:04}.png"));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

fn spawn_world(mut commands: Commands, mut play: ResMut<Play>) {
    let Some(scene) = play.scene.take() else {
        return;
    };
    spawn_scene(&mut commands, &scene);
    let [x, y, z] = play.spawn;
    spawn_sackboy(&mut commands, [x, y], z);
}

fn read_pad(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    play: Res<Play>,
    mut sackboy: Query<&mut SackboyBody>,
) {
    if play.script.is_some() {
        return;
    }
    let pad = pads.iter().next();
    let held = |k: KeyCode, b: GamepadButton| keys.pressed(k) || pad.is_some_and(|p| p.pressed(b));
    let mut x = pad.map_or(0., |p| p.left_stick().x);
    if x.abs() < 0.3 {
        x = 0.;
    }
    if held(KeyCode::ArrowLeft, GamepadButton::DPadLeft) {
        x = -1.;
    }
    if held(KeyCode::ArrowRight, GamepadButton::DPadRight) {
        x = 1.;
    }
    // Moods (D233): I/K/L/J as the mood d-pad up/down/right/left. The pad's own d-pad
    // already walks and changes lanes here, so it is not the mood pad.
    let mood = [
        (KeyCode::KeyI, Mood::Happy),
        (KeyCode::KeyK, Mood::Sad),
        (KeyCode::KeyL, Mood::Angry),
        (KeyCode::KeyJ, Mood::Scared),
    ]
    .into_iter()
    .find_map(|(k, m)| keys.pressed(k).then_some(m));
    let input = Input {
        x,
        jump: held(KeyCode::Space, GamepadButton::South),
        back: held(KeyCode::ArrowUp, GamepadButton::DPadUp),
        front: held(KeyCode::ArrowDown, GamepadButton::DPadDown),
        mood,
    };
    for mut body in &mut sackboy {
        body.input = input;
    }
}

/// Scripted input for this tick (none once the script ends).
fn script_input(play: Res<Play>, mut sackboy: Query<&mut SackboyBody>) {
    let Some(script) = &play.script else {
        return;
    };
    let mut left = play.tick;
    let input = script.iter().find_map(|(input, ticks)| {
        if left < *ticks {
            Some(*input)
        } else {
            left -= ticks;
            None
        }
    });
    for mut body in &mut sackboy {
        body.input = input.unwrap_or_default();
    }
}

/// The picker's layers and facing for this tick, after the controller step.
fn tick_anim(sackboy: Query<&SackboyBody>, mut figure: Query<&mut Figure>) {
    let (Ok(body), Ok(mut f)) = (sackboy.single(), figure.single_mut()) else {
        return;
    };
    let a = f.picker.tick(&body.controller, body.grounded);
    f.yaw = [f.yaw[1], a.yaw];
    f.anim = Some(a);
    f.picked += 1;
}

/// The layers with a visible share, for the scripted log: `clip@frame*share` (additive
/// layers show their weight with a `+`).
fn layer_summary(a: &AnimFrame) -> String {
    let share = a.effective_weights();
    a.layers
        .iter()
        .zip(share)
        .filter_map(|(l, w)| match l.blend {
            Blend::Lerp if w > 0.01 => Some(format!("{}@{:.1}*{w:.2}", l.clip.name(), l.frame)),
            // A look layer on its neutral frame adds nothing: left out.
            Blend::Additive { reference } if l.frame == reference => None,
            Blend::Additive { .. } => {
                Some(format!("+{}@{:.1}*{:.2}", l.clip.name(), l.frame, l.weight))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The mood for the scripted log: `happy 2 emote` (level, emote state), `happy 1 held`
/// once the emote holds, `happy off to_neutral` while an ended mood plays out, `-` for none.
fn mood_summary(picker: &SackboyAnim) -> String {
    let level = picker.mood().map(|(m, l)| format!("{} {l}", m.name()));
    match (level, picker.emote()) {
        (None, None) => "-".into(),
        (None, Some((m, state, _))) => format!("{} off {}", m.name(), state.name()),
        (Some(level), Some((_, state, _))) if !picker.emote_held() => {
            format!("{level} {}", state.name())
        }
        (Some(level), _) => format!("{level} held"),
    }
}

/// QA (`LBP_DUMP_VERTS=FILE`): appends each shown pose's skinned vertices (mesh space,
/// little-endian f32 xyz) so `pose_fit` can compare them with the game's buffer (D234).
fn dump_pose(positions: &[[f32; 3]]) {
    use std::io::Write;
    let Ok(path) = std::env::var("LBP_DUMP_VERTS") else {
        return;
    };
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let mut buf = Vec::with_capacity(positions.len() * 12);
    for p in positions {
        for c in p {
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }
    let _ = f.write_all(&buf);
}

/// With a script, Sackboy's position after each tick (1-based), and the picked clip.
fn log_scripted(
    mut play: ResMut<Play>,
    sackboy: Query<(&SackboyBody, &Position, &lbp_sim::ZSlab)>,
    figure: Query<&Figure>,
) {
    if play.script.is_none() {
        return;
    }
    let Ok((body, pos, slab)) = sackboy.single() else {
        return;
    };
    play.tick += 1;
    let p = to_level(pos.0);
    let clip = figure
        .single()
        .ok()
        .and_then(|f| f.anim.as_ref().map(|a| (f, a)))
        .map(|(f, a)| {
            format!(
                " mood {} yaw {:.1} layers {}",
                mood_summary(&f.picker),
                a.yaw.to_degrees(),
                layer_summary(a)
            )
        })
        .unwrap_or_default();
    println!(
        "sackboy tick {} x {:.1} y {:.1} z {:.1} vx {:.2} vy {:.2} grounded {}{clip}",
        play.tick, p[0], p[1], slab.z, body.controller.vx, body.controller.vy, body.grounded
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn show_sackboy(
    mut commands: Commands,
    display: Option<Res<LevelDisplay>>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    mut v: ResMut<crate::Viewer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sackboy: Query<(&SackboyBody, &Position, &lbp_sim::ZSlab)>,
    mut marker: Query<&mut Transform, (With<Marker>, Without<Figure>)>,
    mut figure: Query<(&mut Transform, &mut Figure)>,
    mut orbit: ResMut<crate::Orbit>,
    mut cameras: Query<&mut Projection, With<Camera3d>>,
    mut framed: Local<bool>,
) {
    let (Some(display), Ok((body, pos, slab))) = (display, sackboy.single()) else {
        return;
    };
    let p = to_level(pos.0);
    let at = display.0.transform_point3(Vec3::new(p[0], p[1], slab.z));
    let s = display.0.x_axis.length();
    if let Ok((mut t, mut f)) = figure.single_mut() {
        let f = &mut *f;
        let vx = body.controller.vx;
        // The picker eases facing per tick (FOUND 0.4); between ticks the shown angle
        // moves from the previous tick's to the latest by the fixed-step overstep.
        let alpha = fixed.overstep_fraction().clamp(0., 1.);
        f.facing = f.yaw[0] + (f.yaw[1] - f.yaw[0]) * alpha;
        let feet = display.0.transform_point3(Vec3::new(
            p[0],
            p[1] - FLOOR_REST + body.controller.landing_dip(),
            slab.z,
        ));
        *t = Transform::from_matrix(
            Mat4::from_translation(feet) * Mat4::from_rotation_y(f.facing) * f.upright,
        );
        if f.banked {
            // Clip bank: one blended pose per picked tick (stand stays the playing clip).
            if f.posed != f.picked {
                if let (Some(a), Some(playing)) = (&f.anim, &mut v.playing) {
                    let pose = blend_frame(&playing.clip, &f.bank, a);
                    dump_pose(&playing.clip.blended_positions(&pose));
                    playing.blended = Some(pose);
                }
                f.posed = f.picked;
            }
        } else if let Some(fps) = v
            .playing
            .as_ref()
            .map(|p| f32::from(p.clip.report.fps.max(1)))
        {
            f.phase = walk_phase(f.phase, time.delta_secs(), fps, vx, body.grounded);
            v.animation_frame = Some(f.phase as i64);
        }
    } else if let Ok(mut t) = marker.single_mut() {
        t.translation = at + Vec3::Y * CENTRE_ABOVE * s;
    } else {
        commands.spawn((
            Marker,
            Mesh3d(meshes.add(Capsule3d::new(RADIUS * s, 2. * HALF_SEGMENT * s))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.55, 0.3),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(at + Vec3::Y * CENTRE_ABOVE * s),
        ));
    }
    let [ex, ey, ez] = FOLLOW_EYE;
    if !*framed {
        *framed = true;
        FRAMINGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // The game's default camera: straight on, at its distance and FOV (orbit and zoom
        // stay free afterwards).
        orbit.yaw = 0.;
        orbit.pitch = 0.;
        orbit.distance = ez * s;
        for mut projection in &mut cameras {
            if let Projection::Perspective(p) = projection.as_mut() {
                p.fov = FOLLOW_FOV_DEG.to_radians();
            }
        }
    }
    orbit.target = display
        .0
        .transform_point3(Vec3::new(p[0] + ex, p[1] + ey, slab.z));
}

/// Counts camera framings, so overlays that nudge `orbit.distance` re-apply after one.
pub(crate) static FRAMINGS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_script_steps() {
        let s = parse_script("r:40, rj:12,-:30,bl:1").unwrap();
        assert_eq!(s.len(), 4);
        assert_eq!((s[0].0.x, s[0].1), (1., 40));
        assert!(s[1].0.jump && s[1].0.x == 1.);
        assert_eq!(s[2].0, Input::default());
        assert!(s[3].0.back && s[3].0.x == -1.);
        assert!(parse_script("x:3").is_err());
        assert!(parse_script("r").is_err());
    }

    #[test]
    fn walk_cycle_paces_with_speed() {
        assert_eq!(walk_phase(4., 0.5, 30., WALK_SPEED, true), 19.);
        assert_eq!(walk_phase(4., 0.5, 30., -WALK_SPEED / 2., true), 11.5);
        assert_eq!(walk_phase(4., 0.5, 30., WALK_SPEED, false), 0.);
        assert_eq!(walk_phase(4., 0.5, 30., 0., true), 0.);
    }

    #[test]
    fn spawn_lane_is_the_nearest_lane() {
        // Moon level: checkpoint z -470 spawns in the back lane.
        assert_eq!(spawn_lane(-470.), -400.);
        assert_eq!(spawn_lane(-50.), 0.);
        assert_eq!(spawn_lane(-180.), -200.);
        assert_eq!(spawn_lane(300.), 0.);
    }
}
