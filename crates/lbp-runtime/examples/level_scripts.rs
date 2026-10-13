//! Rt PC check (docs/level-scripts.md): which scripts levels use and how they run.
//!
//! level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH NEW-JSON [--entry=NAME]... [--ticks=N]
//! level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH NEW-JSON --play=TICKS [--flags=N,..]
//!     [--puter-in-use=TICK]
//! level_scripts SNAPSHOT GAME-DIRS --all NEW-JSON [--play=TICKS] [--strict]
//!
//! Loads every magic-confirmed script with its MAP GUID (the U12 loader), reads the level(s)
//! and reports, per level: scripted Things (PScript with a resource GUID), how many resolve
//! to a loaded class, unresolved GUIDs, and whether the stored field names match the class
//! layout (class fields and superclass fields). Over all classes in use it counts function
//! names that have code, which is how the create/update entry names get settled. With
//! `--entry`, one level runs that function on every scripted Thing and resumes yielded
//! scripts for `--ticks` ticks (default 30), reporting outcomes, faults, unbound natives
//! and `WRITE` lines. `--play` runs the `LevelScripts` schedule instead: `OnCreate__`, then
//! TICKS ticks of resume + `Update__`; `--puter-in-use=TICK` makes
//! `GetPodComputerInUse__` true from that tick on (0 = from `OnCreate__`) and reports the
//! camera as it was just before the switch too. `--all --play=TICKS` plays every level on a
//! fresh VM and adds `play_all`: native calls bound and defaulted, each unbound native with
//! its calls and levels, and the remaining faults by kind. A play run goes past unbound
//! natives (they return their type's default and are counted); `--strict` makes them fault
//! instead. Names and counts only; no bytecode. Output: a NEW JSON file under
//! ignored `local-analysis/`. Counting unit: Thing with a PScript part (instances) or
//! distinct script class (function names).
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_runtime::{instances, LevelScripts, LevelWorld, Outcome, Runner, StepReport};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Vm;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const BUDGET_PER_CALL: u64 = 1_000_000;
const MAX_FAULTS: usize = 30;

fn main() {
    if let Err(e) = run() {
        eprintln!("level_scripts: {e}");
        std::process::exit(1);
    }
}

#[derive(Default)]
struct Totals {
    levels: usize,
    levels_not_read: usize,
    things: usize,
    scripted: usize,
    with_fields: usize,
    resolved: usize,
    unresolved: BTreeMap<u32, usize>,
    by_class: BTreeMap<String, usize>,
    fields_matched: usize,
    fields_missing_in_class: usize,
    unconverted: usize,
    objects_built: usize,
    classes: BTreeSet<lbp_script_vm::ClassId>,
}

fn add_level(vm: &Vm, world: &LevelWorld, t: &mut Totals) -> serde_json::Value {
    t.levels += 1;
    t.things += world.things.len();
    t.unconverted += world.unconverted;
    let found = instances(vm, world);
    let mut level_classes = BTreeMap::<String, usize>::new();
    let mut unresolved = BTreeSet::new();
    for inst in &found {
        t.scripted += 1;
        let state = &world.things[inst.thing as usize];
        t.with_fields += usize::from(state.members.is_some());
        match inst.class {
            Some(c) => {
                t.resolved += 1;
                t.classes.insert(c);
                let name = vm.class_name(c).to_owned();
                *level_classes.entry(name.clone()).or_default() += 1;
                *t.by_class.entry(name).or_default() += 1;
                if let Some(m) = &state.members {
                    let layout: BTreeSet<String> = vm.field_names(c).into_iter().collect();
                    for f in m.keys() {
                        if layout.contains(f) {
                            t.fields_matched += 1;
                        } else {
                            t.fields_missing_in_class += 1;
                        }
                    }
                }
            }
            None => {
                *t.unresolved.entry(inst.guid).or_default() += 1;
                unresolved.insert(format!("{:x}", inst.guid));
            }
        }
    }
    serde_json::json!({
        "things": world.things.len(),
        "scripted": found.len(),
        "classes": level_classes,
        "unresolved_guids": unresolved,
    })
}

/// What one `--play` run is given.
struct PlayOptions {
    ticks: u64,
    flags: BTreeSet<i32>,
    in_use_from: Option<u64>,
    strict: bool,
}

/// One level played: the report and what `--all` adds up.
struct Played {
    json: serde_json::Value,
    instances: usize,
    stopped: usize,
    fault_kinds: BTreeMap<String, usize>,
    thread_faults: Vec<String>,
    engine_faults: usize,
    calls: BTreeMap<String, u64>,
    unbound: BTreeMap<String, u64>,
}

/// Runs the `LevelScripts` schedule on one level whose objects are bound to `vm`.
fn play_level(vm: &mut Vm, mut world: LevelWorld, opts: &PlayOptions) -> Played {
    world.add_class_defaults(vm);
    world.progression_flags = opts.flags.clone();
    world.pod_computer_in_use = opts.in_use_from == Some(0);
    let mut host = Bindings::new(world);
    host.default_unbound = !opts.strict;
    let engine_faults = lbp_runtime::add_engine_objects(vm, &mut host, BUDGET_PER_CALL);
    let mut level = LevelScripts::new(vm, &host.world);
    let created = level.create(vm, &mut host, BUDGET_PER_CALL);
    let mut sum = StepReport::default();
    let mut fault_kinds = BTreeMap::<String, usize>::new();
    let mut first_faults = Vec::new();
    let mut ticks_with_faults = 0usize;
    let mut note_faults = |r: &StepReport| {
        for (thing, f) in &r.faults {
            let kind = f.replace(&format!("thing {thing} "), "thing # ");
            *fault_kinds.entry(kind).or_default() += 1;
            if first_faults.len() < MAX_FAULTS {
                first_faults.push(format!("thing {thing}: {f}"));
            }
        }
    };
    note_faults(&created);
    let camera_json = |c: &lbp_runtime::CameraState| {
        serde_json::json!({
            "from_to_up": c.from_to_up.map(|c| serde_json::json!({
                "from": c.from, "to": c.to, "up": c.up, "params": c.params,
            })),
            "from_to_up_calls": c.from_to_up_calls,
        })
    };
    let mut camera_before_in_use = None;
    for tick in 0..opts.ticks {
        if opts.in_use_from.is_some_and(|t| t > 0 && t == tick) {
            camera_before_in_use = Some(camera_json(&host.world.camera));
            host.world.pod_computer_in_use = true;
        }
        let t = level.tick(vm, &mut host, BUDGET_PER_CALL);
        ticks_with_faults += usize::from(!t.faults.is_empty());
        note_faults(&t);
        sum.done += t.done;
        sum.yielded += t.yielded;
        sum.resumed += t.resumed;
        sum.no_function += t.no_function;
        sum.no_class += t.no_class;
        sum.faults.extend(t.faults);
        sum.threads_started += t.threads_started;
        sum.threads_done += t.threads_done;
        sum.thread_faults.extend(t.thread_faults);
    }
    let report = |r: &StepReport| {
        serde_json::json!({
            "done": r.done, "yielded": r.yielded, "resumed": r.resumed,
            "no_function": r.no_function, "no_class": r.no_class, "faults": r.faults.len(),
            "threads_started": r.threads_started, "threads_done": r.threads_done,
            "thread_faults": r.thread_faults.iter().take(MAX_FAULTS).collect::<Vec<_>>(),
        })
    };
    let mut unbound: Vec<_> = host.unbound.iter().collect();
    unbound.sort_by(|a, b| b.1.cmp(a.1));
    unbound.truncate(40);
    let json = serde_json::json!({
        "instances": level.instances.len(),
        "ticks": level.ticks,
        "tick_hz": lbp_runtime::TICK_HZ,
        "create": report(&created),
        "ticks_total": report(&sum),
        "ticks_with_faults": ticks_with_faults,
        "stopped_by_fault": level.stopped.len(),
        "threads_running_at_end": level.threads(),
        "pending_at_end": level.pending(),
        "faults_by_kind": fault_kinds,
        "first_faults": first_faults,
        "unbound_natives": unbound
            .iter()
            .map(|(k, n)| serde_json::json!({"native": k, "calls": n}))
            .collect::<Vec<_>>(),
        "written": host.world.written.iter().take(40).collect::<Vec<_>>(),
        "engine_object_faults": engine_faults,
        "gooeys_created": host.world.gooeys_created,
        "puter_in_use_from": opts.in_use_from,
        "camera_before_in_use": camera_before_in_use,
        "camera": {
            "from_to_up": host.world.camera.from_to_up.map(|c| serde_json::json!({
                "from": c.from, "to": c.to, "up": c.up, "params": c.params,
            })),
            "from_to_up_calls": host.world.camera.from_to_up_calls,
            "dof_and_fog": host.world.camera.dof_and_fog,
            "motion_blur_clears": host.world.camera.motion_blur_clears,
        },
        "frame_entries": level.instances.iter().filter_map(|i| {
            let e = lbp_runtime::frame_entry(vm, i.class?)?;
            Some(format!("thing {}: {}.{}", i.thing, e.class, e.function))
        }).collect::<Vec<_>>(),
    });
    Played {
        json,
        instances: level.instances.len(),
        stopped: level.stopped.len(),
        fault_kinds,
        thread_faults: sum
            .thread_faults
            .iter()
            .chain(&created.thread_faults)
            .cloned()
            .collect(),
        engine_faults: engine_faults.len(),
        calls: host.calls,
        unbound: host.unbound,
    }
}

/// `--all --play` sums. Pairs are (count, levels).
#[derive(Default)]
struct PlayTotals {
    levels: usize,
    instances: usize,
    stopped: usize,
    levels_with_faults: usize,
    engine_faults: usize,
    calls: BTreeMap<String, (u64, usize)>,
    unbound: BTreeMap<String, (u64, usize)>,
    faults: BTreeMap<String, (u64, usize)>,
    thread_faults: BTreeMap<String, (u64, usize)>,
}

impl PlayTotals {
    fn add(&mut self, p: &Played) {
        self.levels += 1;
        self.instances += p.instances;
        self.stopped += p.stopped;
        self.engine_faults += p.engine_faults;
        self.levels_with_faults +=
            usize::from(!p.fault_kinds.is_empty() || !p.thread_faults.is_empty());
        let bump = |m: &mut BTreeMap<String, (u64, usize)>, k: &str, n: u64| {
            let e = m.entry(k.to_owned()).or_default();
            e.0 += n;
            e.1 += 1;
        };
        for (k, n) in &p.calls {
            bump(&mut self.calls, k, *n);
        }
        for (k, n) in &p.unbound {
            bump(&mut self.unbound, k, *n);
        }
        for (k, n) in &p.fault_kinds {
            bump(&mut self.faults, k, *n as u64);
        }
        let mut threads = BTreeMap::<&str, u64>::new();
        for f in &p.thread_faults {
            *threads.entry(f).or_default() += 1;
        }
        for (k, n) in threads {
            bump(&mut self.thread_faults, k, n);
        }
    }

    fn json(&self, opts: &PlayOptions) -> serde_json::Value {
        let ranked = |m: &BTreeMap<String, (u64, usize)>, key: &str, max: usize| {
            let mut v: Vec<_> = m.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            v.iter()
                .take(max)
                .map(|(k, (n, levels))| serde_json::json!({key: k, "count": n, "levels": levels}))
                .collect::<Vec<_>>()
        };
        let total = |m: &BTreeMap<String, (u64, usize)>| m.values().map(|v| v.0).sum::<u64>();
        let (calls, unbound) = (total(&self.calls), total(&self.unbound));
        serde_json::json!({
            "unit": "native call; scripted Thing; fault (a Thing stops at its first fault)",
            "ticks_per_level": opts.ticks,
            "unbound_natives_fault": opts.strict,
            "levels_played": self.levels,
            "levels_with_faults": self.levels_with_faults,
            "instances": self.instances,
            "stopped_by_fault": self.stopped,
            "engine_object_faults": self.engine_faults,
            "native_calls": calls,
            "native_calls_bound": calls - unbound,
            "native_calls_unbound": unbound,
            "natives_called": self.calls.len(),
            "natives_called_unbound": self.unbound.len(),
            "faults": total(&self.faults),
            "fault_kinds": self.faults.len(),
            "thread_faults": total(&self.thread_faults),
            "unbound_natives": ranked(&self.unbound, "native", usize::MAX),
            "faults_by_kind": ranked(&self.faults, "fault", 200),
            "thread_faults_by_kind": ranked(&self.thread_faults, "fault", 100),
        })
    }
}

fn run() -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let entries: Vec<&str> = all
        .iter()
        .filter_map(|a| a.strip_prefix("--entry="))
        .collect();
    let ticks: usize = all
        .iter()
        .find_map(|a| a.strip_prefix("--ticks="))
        .map_or(Ok(30), str::parse)
        .map_err(|e| format!("--ticks: {e}"))?;
    let play: Option<u64> = all
        .iter()
        .find_map(|a| a.strip_prefix("--play="))
        .map(str::parse)
        .transpose()
        .map_err(|e| format!("--play: {e}"))?;
    // `--flags=2,5`: LBP2 game progression flags set on the profile (default none).
    let flags: std::collections::BTreeSet<i32> = all
        .iter()
        .find_map(|a| a.strip_prefix("--flags="))
        .map(|f| f.split(',').map(str::parse).collect::<Result<_, _>>())
        .transpose()
        .map_err(|e| format!("--flags: {e}"))?
        .unwrap_or_default();
    // `--puter-in-use=TICK`: a player is at the pod computer from that tick on.
    let in_use_from: Option<u64> = all
        .iter()
        .find_map(|a| a.strip_prefix("--puter-in-use="))
        .map(str::parse)
        .transpose()
        .map_err(|e| format!("--puter-in-use: {e}"))?;
    let args: Vec<&String> = all
        .iter()
        .filter(|a| {
            !a.starts_with("--entry=")
                && !a.starts_with("--ticks=")
                && !a.starts_with("--play=")
                && !a.starts_with("--flags=")
                && !a.starts_with("--puter-in-use=")
                && *a != "--strict"
        })
        .collect();
    if args.len() != 5 {
        return Err(
            "usage: level_scripts SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON \
                    [--entry=NAME]... [--ticks=N] [--play=TICKS] [--flags=N,..] \
                    [--puter-in-use=TICK] [--strict]"
                .into(),
        );
    }
    let output = Path::new(args[4]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let every = args[3] == "--all";
    if every && !entries.is_empty() {
        return Err("--entry runs one level, not --all".into());
    }
    let play_opts = play.map(|ticks| PlayOptions {
        ticks,
        flags: flags.clone(),
        in_use_from,
        strict: all.iter().any(|a| a == "--strict"),
    });
    if play.is_some() && !entries.is_empty() {
        return Err("use --play or --entry, not both".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(args[2]), &catalog, &control)?;
    let (scripts, scripts_not_read) = lbp_ui_script::load::load(&source, &catalog, &control)?;
    let mut vm = Vm::default();
    let scripts_loaded = scripts.len();
    let with_guid = scripts.iter().filter(|(g, _)| g.is_some()).count();
    for (guid, s) in &scripts {
        vm.load(s.clone(), *guid);
    }

    let mut keys = Vec::new();
    if every {
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
        let (key, label) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(args[3]))?;
        keys.push((key, label));
    }

    let mut totals = Totals::default();
    let mut levels = Vec::new();
    let mut last_world = None;
    let mut play_totals = PlayTotals::default();
    for (key, label) in &keys {
        let read =
            lbp_preview::level_scene::read_level_bytes(&source, key, &control).and_then(|b| {
                lbp_level::read(&b, lbp_level::Limits::default(), &|| false)
                    .map_err(|e| e.to_string())
            });
        let report = match read {
            Ok(r) => r,
            Err(_) => {
                totals.levels_not_read += 1;
                continue;
            }
        };
        let mut world = LevelWorld::from_things(&report.things);
        // `--all --play`: each level on its own VM (same load order, so the same class
        // ids), so no heap object or static outlives its level.
        let mut fresh = match (&play_opts, every) {
            (Some(_), true) => {
                let mut v = Vm::default();
                for (guid, s) in &scripts {
                    v.load(s.clone(), *guid);
                }
                Some(v)
            }
            _ => None,
        };
        let level_vm = fresh.as_mut().unwrap_or(&mut vm);
        totals.objects_built += world.bind_objects(level_vm);
        let mut row = add_level(level_vm, &world, &mut totals);
        row["level"] = serde_json::json!(label);
        row["stopped"] = serde_json::json!(report.stop.is_some());
        if let (Some(opts), true) = (&play_opts, every) {
            let p = play_level(level_vm, world, opts);
            play_totals.add(&p);
            row["play"] = serde_json::json!({
                "instances": p.instances,
                "stopped_by_fault": p.stopped,
                "unbound_native_calls": p.unbound.values().sum::<u64>(),
            });
            levels.push(row);
            continue;
        }
        levels.push(row);
        last_world = Some(world);
    }

    // Function names with code over the classes in use.
    let mut names = BTreeMap::<String, usize>::new();
    for &c in &totals.classes {
        for n in vm.function_names(c) {
            *names.entry(n.to_owned()).or_default() += 1;
        }
    }
    let mut ranked: Vec<_> = names.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(60);

    // Optional run on the single level.
    let mut runs = serde_json::Map::new();
    if let (Some(opts), Some(world), false) = (&play_opts, last_world.take(), every) {
        runs.insert("play".into(), play_level(&mut vm, world, opts).json);
    }
    if let (false, Some(mut world)) = (entries.is_empty(), last_world) {
        world.add_class_defaults(&vm);
        world.progression_flags = flags.clone();
        let mut host = Bindings::new(world);
        let found = instances(&vm, &host.world);
        let mut runner = Runner::default();
        for entry in &entries {
            let mut outcomes = BTreeMap::<String, usize>::new();
            let mut faults = Vec::new();
            // Faults by site and message, Thing numbers removed.
            let mut fault_kinds = BTreeMap::<String, usize>::new();
            let mut note = |o: &Outcome, thing: u32, outcomes: &mut BTreeMap<String, usize>| {
                let key = match o {
                    Outcome::Fault(_) => "fault",
                    Outcome::Done => "done",
                    Outcome::Yielded => "yielded",
                    Outcome::NoClass => "no_class",
                    Outcome::NoFunction => "no_function",
                };
                *outcomes.entry(key.into()).or_default() += 1;
                if let Outcome::Fault(f) = o {
                    let kind = f.replace(&format!("thing {thing} "), "thing # ");
                    *fault_kinds.entry(kind).or_default() += 1;
                    if faults.len() < MAX_FAULTS {
                        faults.push(format!("thing {thing}: {f}"));
                    }
                }
            };
            for inst in &found {
                let mut budget = BUDGET_PER_CALL;
                let o = runner.start(&mut vm, &mut host, *inst, entry, &mut budget);
                note(&o, inst.thing, &mut outcomes);
            }
            let mut resumed = BTreeMap::<String, usize>::new();
            for _ in 0..ticks {
                if runner.pending() == 0 {
                    break;
                }
                let mut budget = BUDGET_PER_CALL.saturating_mul(runner.pending() as u64);
                for (thing, o) in runner.tick(&mut vm, &mut host, &mut budget) {
                    note(&o, thing, &mut resumed);
                }
            }
            runs.insert(
                (*entry).to_owned(),
                serde_json::json!({
                    "started": outcomes,
                    "resumed": resumed,
                    "still_pending": runner.pending(),
                    "first_faults": faults,
                    "faults_by_kind": fault_kinds,
                }),
            );
        }
        let mut unbound: Vec<_> = host.unbound.iter().collect();
        unbound.sort_by(|a, b| b.1.cmp(a.1));
        unbound.truncate(40);
        runs.insert(
            "unbound_natives".into(),
            serde_json::json!(unbound
                .iter()
                .map(|(k, n)| serde_json::json!({"native": k, "calls": n}))
                .collect::<Vec<_>>()),
        );
        runs.insert(
            "written".into(),
            serde_json::json!(host.world.written.iter().take(40).collect::<Vec<_>>()),
        );
    }

    let mut by_class: Vec<_> = totals.by_class.iter().collect();
    by_class.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let unresolved: BTreeMap<String, usize> = totals
        .unresolved
        .iter()
        .map(|(g, n)| (format!("{g:x}"), *n))
        .collect();
    let doc = serde_json::json!({
        "schema_version": 1,
        "unit": "Thing with a PScript resource GUID; function names per distinct class",
        "scripts_loaded": scripts_loaded,
        "scripts_with_guid": with_guid,
        "scripts_not_read": scripts_not_read,
        "levels": totals.levels,
        "levels_not_read": totals.levels_not_read,
        "things": totals.things,
        "scripted_things": totals.scripted,
        "with_fields": totals.with_fields,
        "resolved": totals.resolved,
        "unresolved_by_guid": unresolved,
        "classes_in_use": totals.classes.len(),
        "instances_by_class": by_class,
        "fields_matched": totals.fields_matched,
        "fields_missing_in_class": totals.fields_missing_in_class,
        "unconverted_values": totals.unconverted,
        "objects_built": totals.objects_built,
        "function_names": ranked,
        "per_level": levels,
        "runs": runs,
        "play_all": play_opts.as_ref().filter(|_| every).map(|o| play_totals.json(o)),
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "scripts {scripts_loaded} (with GUID {with_guid}, not read {scripts_not_read}); \
         levels {} (not read {}); scripted things {} resolved {} (classes {}), unresolved \
         GUIDs {}; fields matched {} missing in class {}; unconverted values {}; objects built {}",
        totals.levels,
        totals.levels_not_read,
        totals.scripted,
        totals.resolved,
        totals.classes.len(),
        totals.unresolved.len(),
        totals.fields_matched,
        totals.fields_missing_in_class,
        totals.unconverted,
        totals.objects_built
    );
    if let (Some(o), true) = (&play_opts, every) {
        let mut j = play_totals.json(o);
        for k in ["unbound_natives", "faults_by_kind", "thread_faults_by_kind"] {
            j.as_object_mut().map(|m| m.remove(k));
        }
        println!("--all --play={}: {j}", o.ticks);
    }
    for (entry, r) in &runs {
        if entry != "unbound_natives" && entry != "written" {
            let flag = if entry == "play" { "--play" } else { "--entry" };
            println!("{flag}={entry}: {r}");
        }
    }
    Ok(())
}
