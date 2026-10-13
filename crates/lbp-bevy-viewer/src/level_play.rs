//! `--level-scripts` (docs/level-scripts.md, part 6): with a level, run the level's own
//! scripts at the game's 30 Hz tick in Bevy's `FixedUpdate`, through the same
//! `LevelScripts` schedule as the `level_scripts` example: `OnCreate__` on the first
//! tick, then each tick resume suspended scripts and run `Update__`. A small window shows
//! the counts. Nothing the scripts change is drawn yet; this is the clock and the counts.
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::level_scene;
use lbp_runtime::{LevelScripts, LevelWorld, StepReport, TICK_HZ};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Vm;
use std::path::Path;

/// Instruction budget per script call (as the `level_scripts` example).
const BUDGET_PER_CALL: u64 = 1_000_000;
/// Fault lines kept for the window.
const MAX_FAULT_LINES: usize = 8;

struct Play {
    vm: Vm,
    host: Bindings<LevelWorld>,
    scripts: LevelScripts,
    created: Option<StepReport>,
    /// Sums over update ticks.
    totals: StepReport,
    ticks_with_faults: usize,
    faults: Vec<String>,
    header: String,
    hide_ui: bool,
}

fn value(all: &[String], flag: &str) -> Option<String> {
    all.iter()
        .position(|a| a == flag)
        .and_then(|i| all.get(i + 1))
        .cloned()
}

/// Loads scripts and the level and adds the 30 Hz schedule when `--level-scripts` is
/// given; otherwise does nothing.
pub fn install(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    if !all.iter().any(|a| a == "--level-scripts") {
        return Ok(());
    }
    let snapshot = value(&all, "--snapshot").ok_or("--level-scripts requires --snapshot")?;
    let game = value(&all, "--game-dir").ok_or("--level-scripts requires --game-dir")?;
    let guid = value(&all, "--level-guid")
        .map(|g| u32::from_str_radix(&g, 16).map_err(|_| "--level-guid requires hex digits"))
        .transpose()?;
    let path = value(&all, "--level-path");
    if guid.is_none() && path.is_none() {
        return Err("--level-scripts requires --level-guid or --level-path".into());
    }
    let hide_ui = all.iter().any(|a| a == "--qa-hide-ui");
    // LBP2 game progression flags set on the profile (as `level_scripts --flags`).
    let flags = value(&all, "--level-scripts-flags")
        .map(|v| {
            v.split(',')
                .map(|f| f.trim().parse::<i32>())
                .collect::<Result<std::collections::BTreeSet<_>, _>>()
                .map_err(|e| format!("--level-scripts-flags: {e}"))
        })
        .transpose()?
        .unwrap_or_default();
    // The level reader recurses deeply: same worker stack as the level scene.
    let (mut vm, mut world, header) = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<(Vm, LevelWorld, String), String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(Path::new(&snapshot), &control)?;
            let source = AttachedSource::attach(Path::new(&game), &catalog, &control)?;
            let (scripts, not_read) = lbp_ui_script::load::load(&source, &catalog, &control)?;
            let mut vm = Vm::default();
            let loaded = scripts.len();
            for (g, s) in scripts {
                vm.load(s, g);
            }
            let (key, identity) = level_scene::resolve_level(&catalog, guid, path.as_deref())?;
            let bytes = level_scene::read_level_bytes(&source, &key, &control)?;
            let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
                .map_err(|e| e.to_string())?;
            let world = LevelWorld::from_things(&report.things);
            let header =
                format!("Level scripts: {identity}; scripts loaded {loaded} (not read {not_read})");
            Ok((vm, world, header))
        })
        .map_err(|e| format!("level scripts thread: {e}"))?
        .join()
        .map_err(|_| "level scripts thread panicked".to_string())??;
    let built = world.bind_objects(&mut vm);
    world.add_class_defaults(&vm);
    world.progression_flags = flags;
    // Unbound natives return their type's default and are counted (rscript-plan stage 1).
    let mut host = Bindings::defaulting(world);
    let mut faults = lbp_runtime::add_engine_objects(&mut vm, &mut host, BUDGET_PER_CALL);
    faults.truncate(MAX_FAULT_LINES);
    let scripts = LevelScripts::new(&vm, &host.world);
    let header = format!(
        "{header}; scripted Things {}; objects built {built}, unconverted {}",
        scripts.instances.len(),
        host.world.unconverted
    );
    println!("{header}");
    app.insert_non_send(Play {
        vm,
        host,
        scripts,
        created: None,
        totals: StepReport::default(),
        ticks_with_faults: 0,
        faults,
        header,
        hide_ui,
    })
    .insert_resource(Time::<Fixed>::from_hz(f64::from(TICK_HZ)))
    .add_systems(FixedUpdate, tick)
    .add_systems(EguiPrimaryContextPass, window);
    Ok(())
}

fn note(play: &mut Play, r: &StepReport) {
    for (thing, f) in &r.faults {
        if play.faults.len() < MAX_FAULT_LINES {
            play.faults.push(format!("thing {thing}: {f}"));
        }
    }
}

fn tick(mut play: NonSendMut<Play>) {
    let p = &mut *play;
    if p.created.is_none() {
        let r = p.scripts.create(&mut p.vm, &mut p.host, BUDGET_PER_CALL);
        note(p, &r);
        p.created = Some(r);
        return;
    }
    let r = p.scripts.tick(&mut p.vm, &mut p.host, BUDGET_PER_CALL);
    p.ticks_with_faults += usize::from(!r.faults.is_empty());
    note(p, &r);
    let t = &mut p.totals;
    t.done += r.done;
    t.yielded += r.yielded;
    t.resumed += r.resumed;
    t.no_function += r.no_function;
    t.no_class += r.no_class;
    t.faults.extend(r.faults);
}

fn line(r: &StepReport) -> String {
    format!(
        "done {}, yielded {}, resumed {}, no function {}, no class {}, faults {}",
        r.done,
        r.yielded,
        r.resumed,
        r.no_function,
        r.no_class,
        r.faults.len()
    )
}

fn window(mut contexts: EguiContexts, play: NonSend<Play>) -> Result {
    if play.hide_ui {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    egui::Window::new("Level scripts")
        .default_pos([16., 480.])
        .show(ctx, |ui| {
            ui.small(&play.header);
            ui.label(format!(
                "Tick {} at {TICK_HZ} Hz; suspended {}; stopped by a fault {}",
                play.scripts.ticks,
                play.scripts.pending(),
                play.scripts.stopped.len()
            ));
            if let Some(c) = &play.created {
                ui.label(format!("Create: {}", line(c)));
            }
            ui.label(format!("Ticks: {}", line(&play.totals)));
            ui.label(format!("Ticks with faults: {}", play.ticks_with_faults));
            let mut unbound: Vec<_> = play.host.unbound.iter().collect();
            unbound.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            if !unbound.is_empty() {
                let calls: u64 = play.host.calls.values().sum();
                let defaulted: u64 = unbound.iter().map(|u| *u.1).sum();
                ui.small(format!(
                    "Native calls: bound {}, defaulted {defaulted} ({} unbound natives):",
                    calls - defaulted,
                    unbound.len()
                ));
                for (name, n) in unbound.iter().take(6) {
                    ui.small(format!("  {name}: {n}"));
                }
            }
            let cam = &play.host.world.camera;
            if let Some(c) = &cam.from_to_up {
                ui.small(format!(
                    "Camera ({} calls): from {:?} to {:?} up {:?}",
                    cam.from_to_up_calls, c.from, c.to, c.up
                ));
            }
            for f in &play.faults {
                ui.small(f);
            }
            for w in play.host.world.written.iter().rev().take(4) {
                ui.small(format!("WRITE {w}"));
            }
        });
    Ok(())
}
