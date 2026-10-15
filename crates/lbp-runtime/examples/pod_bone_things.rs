//! Read-only probe (D317): which Things the scripts of the Pod's planets level move, as
//! the viewer plays it (the shell level and the planets level in one session, the pod
//! computer in use when IN-USE rounds have run). Prints, for the listed Things, the
//! parent, the stored world matrix and, at each sample tick, the live world matrix and
//! the relative matrix inv(live A) x live B against inv(stored A) x stored B for the
//! first listed Thing A. Numbers only (no game bytes). Usage:
//! pod_bone_things SNAPSHOT GAME-DIRS PLANETS-GUID SHELL-GUID IN-USE TICKS THING,..
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_runtime::{tick_session, LevelScripts, LevelWorld, SessionLevel};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::Vm;
use std::path::Path;

const BUDGET: u64 = 1_000_000;

fn inv(m: &[f32; 16]) -> [f32; 16] {
    lbp_preview::level_backdrop::affine_inverse(m).expect("invertible")
}

fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    lbp_preview::skinned::mul(a, b)
}

fn show(m: &[f32; 16]) -> String {
    let s = |c: usize| (0..3).map(|r| m[c * 4 + r].powi(2)).sum::<f32>().sqrt();
    format!(
        "t ({:.2}, {:.2}, {:.2}) scales ({:.4}, {:.4}, {:.4}) cols [{:.4} {:.4} {:.4} | {:.4} {:.4} {:.4} | {:.4} {:.4} {:.4}]",
        m[12], m[13], m[14], s(0), s(1), s(2),
        m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]
    )
}

fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 8 {
        return Err(
            "pod_bone_things SNAPSHOT GAME-DIRS PLANETS-GUID SHELL-GUID IN-USE TICKS THING,.."
                .into(),
        );
    }
    let hex = |s: &str| u32::from_str_radix(s, 16).map_err(|e| e.to_string());
    let (planets, shell) = (hex(&a[3])?, hex(&a[4])?);
    let in_use: u64 = a[5].parse().map_err(|_| "IN-USE")?;
    let ticks: Vec<u64> = a[6].split(',').map(|t| t.parse().unwrap()).collect();
    let watch: Vec<u32> = a[7].split(',').map(|t| t.parse().unwrap()).collect();
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&a[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&a[2]), &catalog, &control)?;
    let (scripts, _) = lbp_ui_script::load::load(&source, &catalog, &control)?;
    let (mut vm, mut shell_vm) = (Vm::default(), Vm::default());
    for (g, s) in scripts {
        shell_vm.load(s.clone(), g);
        vm.load(s, g);
    }
    let read = |g: u32| -> Result<LevelWorld, String> {
        let (key, _) = lbp_preview::level_scene::resolve_level(&catalog, Some(g), None)?;
        let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
        let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
            .map_err(|e| e.to_string())?;
        Ok(LevelWorld::from_things(&report.things))
    };
    let mut world = read(planets)?;
    let mut sworld = read(shell)?;
    world.bind_objects(&mut vm);
    world.add_class_defaults(&vm);
    sworld.bind_objects(&mut shell_vm);
    sworld.add_class_defaults(&shell_vm);
    let stored: Vec<Option<[f32; 16]>> = world.things.iter().map(|t| t.world_matrix).collect();
    for &t in &watch {
        let s = &world.things[t as usize];
        println!(
            "thing {t}: parent {:?} first_child {:?} script {:?} stored {}",
            s.parent,
            s.first_child,
            s.script_classes,
            stored[t as usize].map(|m| show(&m)).unwrap_or_default()
        );
    }
    let mut host = Bindings::defaulting(world);
    lbp_runtime::add_engine_objects(&mut vm, &mut host, BUDGET);
    let mut level = LevelScripts::new(&vm, &host.world);
    let mut shost = Bindings::defaulting(sworld);
    lbp_runtime::add_engine_objects(&mut shell_vm, &mut shost, BUDGET);
    let mut slevel = LevelScripts::new(&shell_vm, &shost.world);
    host.world.our_pod = shost.world.pod_thing(true).is_some();
    host.world.hand_session_to(&mut shost.world);
    slevel.create(&mut shell_vm, &mut shost, BUDGET);
    level.create(&mut vm, &mut host, BUDGET);
    let last = *ticks.iter().max().unwrap_or(&0);
    for n in 0..=last {
        if n > 0 {
            if n - 1 == in_use {
                host.world.pod_computer_in_use = true;
                shost.world.pod_computer_in_use = true;
            }
            let mut levels = [
                SessionLevel {
                    scripts: &mut slevel,
                    vm: &mut shell_vm,
                    host: &mut shost,
                },
                SessionLevel {
                    scripts: &mut level,
                    vm: &mut vm,
                    host: &mut host,
                },
            ];
            tick_session(&mut levels, BUDGET);
        }
        if !ticks.contains(&n) {
            continue;
        }
        println!("tick {n}:");
        let moved: Vec<usize> = (0..stored.len())
            .filter(|&i| {
                let live = host.world.things[i].world_matrix;
                match (live, stored[i]) {
                    (Some(l), Some(s)) => l.iter().zip(s).any(|(x, y)| (x - y).abs() > 1e-3),
                    (l, s) => l.is_some() != s.is_some(),
                }
            })
            .collect();
        println!("  moved Things ({}): {:?}", moved.len(), moved);
        let a0 = watch[0] as usize;
        for &t in &watch {
            let t = t as usize;
            let (Some(l), Some(s)) = (host.world.things[t].world_matrix, stored[t]) else {
                continue;
            };
            println!("  thing {t}: live {}", show(&l));
            let (Some(la), Some(sa)) = (host.world.things[a0].world_matrix, stored[a0]) else {
                continue;
            };
            println!("    live rel to {a0}:   {}", show(&mul(&inv(&la), &l)));
            println!("    stored rel to {a0}: {}", show(&mul(&inv(&sa), &s)));
            println!(
                "    moved by (live x inv stored): {}",
                show(&mul(&l, &inv(&s)))
            );
        }
    }
    Ok(())
}

fn main() {
    // The level reader recurses deeply.
    let r = std::thread::Builder::new()
        .stack_size(lbp_preview::level_scene::LEVEL_STACK_BYTES)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
    if let Err(e) = r {
        eprintln!("pod_bone_things: {e}");
        std::process::exit(1);
    }
}
