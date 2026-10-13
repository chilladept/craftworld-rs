//! Rt on authored synthetic data (no game bytes): a three-Thing level and a small script
//! class with a superclass. docs/level-scripts.md.
use lbp_level::parts::{
    Pos, RenderMesh, ScriptField, ScriptObject, ScriptPart, ScriptValue, Switch, SwitchOutput,
    SwitchSignal, Trigger, World as WorldPart,
};
use lbp_level::script::{FieldDef, FunctionDef, MemberRef, Script, Span, TypeRef};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{instances, InputWire, LevelScripts, LevelWorld, Outcome, Runner};
use lbp_script_vm::natives::{Bindings, World, SCORE_BOARD_SCRIPT};
use lbp_script_vm::{Host, MemberKind, Value, Vm};

const GUID: u32 = 0x2d06;
const BASE: u32 = 0x2d07;

fn i(op: u8, ty: u8, a: u16, b: u16, c: u16) -> u64 {
    u64::from(op)
        | u64::from(ty) << 8
        | u64::from(a) << 16
        | u64::from(b) << 32
        | u64::from(c) << 48
}

fn desc(guid: u32) -> Descriptor {
    Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(guid),
        sha1: None,
    }
}

fn ty(machine_type: u8, type_name: u32) -> TypeRef {
    TypeRef {
        machine_type,
        builtin_type: 0,
        dimension_count: 0,
        array_base_machine_type: 0,
        script: None,
        type_name,
    }
}

fn function(modifiers: u32, name: u32, begin: u32, end: u32) -> FunctionDef {
    FunctionDef {
        modifiers,
        type_ref: 0,
        name,
        arguments: Span { begin: 0, end: 0 },
        bytecode: Span { begin, end },
        line_numbers: Span { begin: 0, end: 0 },
        locals: Span { begin: 0, end: 0 },
        stack_size: 16,
    }
}

/// `Base`: fields `tweaking` (bool, never stored) and `count`, and a native `Mode__`.
fn base() -> Script {
    let field = |type_ref, name| FieldDef {
        modifiers: 0,
        type_ref,
        name,
    };
    Script {
        class_name: "Base".into(),
        super_class: None,
        type_refs: vec![ty(1, 0), ty(3, 0)],
        field_refs: vec![],
        function_refs: vec![],
        field_defs: vec![field(0, 1), field(1, 2)],
        function_defs: vec![function(2, 3, 0, 0)],
        bytecode: vec![],
        strings_a: ["Base", "tweaking", "count", "Mode__"]
            .map(String::from)
            .to_vec(),
        ..lamp()
    }
}

/// `Lamp : Base`. `Tick`: `count += 1; Yield(); return count;` on the Thing in register
/// 0. `Ask`: calls the inherited native `Mode__` through Lamp's own function reference.
fn lamp() -> Script {
    let code = vec![
        i(0x95, 3, 4, 0, 0),                   // r4 = thing(r0).count
        u64::from(0x03u8) | 8 << 16 | 1 << 32, // r8 = 1
        i(0x2a, 3, 4, 4, 8),                   // r4 = r4 + r8 (ADDi)
        i(0x97, 3, 4, 0, 0),                   // thing(r0).count = r4
        i(0xa4, 0, 0, 0, 0),                   // CALL System.Yield
        i(0xa5, 3, 0, 4, 0),                   // RETURN r4
        i(0xa4, 0, 0, 1, 0),                   // CALL Lamp.Mode__ (declared on Base)
        i(0xa5, 0, 0, 0, 0),                   // RETURN
    ];
    Script {
        up_to_date_script: None,
        class_name: "Lamp".into(),
        super_class: Some(desc(BASE)),
        modifiers: 0,
        type_refs: vec![ty(10, 0), ty(11, 1)],
        field_refs: vec![MemberRef {
            type_ref: 0,
            name: 2,
        }],
        function_refs: vec![
            MemberRef {
                type_ref: 1,
                name: 3,
            },
            MemberRef {
                type_ref: 0,
                name: 5,
            },
        ],
        field_defs: vec![],
        property_defs: vec![],
        function_defs: vec![function(0, 4, 0, 6), function(0, 6, 6, 8)],
        shared_arguments: vec![],
        bytecode: code,
        line_numbers: vec![],
        locals: vec![],
        strings_a: [
            "Lamp", "System", "count", "Yield__", "Tick", "Mode__", "Ask",
        ]
        .map(String::from)
        .to_vec(),
        strings_w: vec![],
        constants_s64: Some(vec![]),
        constants_f32: vec![],
        depending_guids: Some(vec![]),
    }
}

/// World Thing 0; Thing 1 (scripted, UID 7, at (1, 2, 3)) and Thing 2 under it.
fn level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..3)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    t[1].parent = Some(0);
    t[2].parent = Some(0);
    t[1].uid = Some(7);
    t[1].parts = vec!["POS", "SCRIPT"];
    let mut world = [0.; 16];
    world[..4].copy_from_slice(&[1., 0., 0., 0.]);
    world[12..].copy_from_slice(&[1., 2., 3., 1.]);
    t[1].data.pos = Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world,
    });
    t[1].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![
            ScriptField {
                name: "count".into(),
                machine_type: 3,
                value: ScriptValue::Int(41),
            },
            ScriptField {
                name: "label".into(),
                machine_type: 0xb,
                value: ScriptValue::Object {
                    kind: 16,
                    id: Some(0),
                    body: Some(Box::new(ScriptObject::Text("hi".into()))),
                },
            },
            ScriptField {
                name: "cache".into(),
                machine_type: 3,
                value: ScriptValue::NotStored,
            },
        ],
    });
    t
}

#[test]
fn thing_tree_positions_parts_and_members() {
    let mut w = LevelWorld::from_things(&level());
    assert_eq!(w.world_thing(), Ok(Some(0)));
    assert_eq!(w.first_child(0), Ok(Some(1)));
    assert_eq!(w.next_sibling(1), Ok(Some(2)));
    assert_eq!(w.next_sibling(2), Ok(None));
    assert_eq!(w.parent(2), Ok(Some(0)));
    assert_eq!(w.thing_uid(1), Ok(7));
    assert!(w.thing_uid(2).is_err());
    assert_eq!(
        w.world_transform(1).unwrap().unwrap()[12..],
        [1., 2., 3., 1.]
    );
    assert_eq!(w.world_transform(2), Ok(None));
    assert_eq!(w.has_part(1, "PScript"), Ok(true));
    assert_eq!(w.has_part(1, "PSwitch"), Ok(false));
    let me = Value::Thing(Some(1));
    let get = |w: &mut LevelWorld, f| w.get_member(MemberKind::Thing, &me, "Lamp", f);
    assert_eq!(get(&mut w, "count"), Ok(Value::Int(41)));
    assert_eq!(get(&mut w, "label"), Ok(Value::Str("hi".into())));
    assert!(get(&mut w, "cache").is_err());
    assert!(w
        .get_member(MemberKind::Native, &me, "Lamp", "count")
        .is_err());
    assert_eq!(w.unconverted, 0);
}

#[test]
fn runner_starts_a_thing_script_and_resumes_it_next_tick() {
    let mut vm = Vm::default();
    vm.load(lamp(), Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&level()));
    let found = instances(&vm, &host.world);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].thing, found[0].guid), (1, GUID));
    assert!(found[0].class.is_some());
    let mut runner = Runner::default();
    let mut budget = 1000;
    assert_eq!(
        runner.start(&mut vm, &mut host, found[0], "Missing", &mut budget),
        Outcome::NoFunction
    );
    assert_eq!(
        runner.start(&mut vm, &mut host, found[0], "Tick", &mut budget),
        Outcome::Yielded
    );
    let me = Value::Thing(Some(1));
    assert_eq!(
        host.world
            .get_member(MemberKind::Thing, &me, "Lamp", "count"),
        Ok(Value::Int(42))
    );
    assert_eq!(runner.pending(), 1);
    assert_eq!(
        runner.tick(&mut vm, &mut host, &mut budget),
        [(1, Outcome::Done)]
    );
    assert_eq!(runner.pending(), 0);
    let unloaded = lbp_runtime::Instance {
        class: None,
        ..found[0]
    };
    assert_eq!(
        runner.start(&mut vm, &mut host, unloaded, "Tick", &mut budget),
        Outcome::NoClass
    );
}

#[test]
fn class_defaults_inherited_natives_and_parts() {
    let mut vm = Vm::default();
    vm.load(base(), Some(BASE));
    vm.load(lamp(), Some(GUID));
    let mut things = level();
    things[1].parts.push("SPRITE_LIGHT");
    let mut world = LevelWorld::from_things(&things);
    let me = Value::Thing(Some(1));
    assert!(world
        .get_member(MemberKind::Thing, &me, "Lamp", "tweaking")
        .is_err());
    world.add_class_defaults(&vm);
    // A superclass field the instance never stores exists at its default; stored wins.
    let get = |w: &mut LevelWorld, f| w.get_member(MemberKind::Thing, &me, "Lamp", f);
    assert_eq!(get(&mut world, "tweaking"), Ok(Value::Bool(false)));
    assert_eq!(get(&mut world, "count"), Ok(Value::Int(41)));
    // `thing.SpriteLight`: a part handle whose ref_ptr members keep what scripts write;
    // null for a part the Thing lacks; an error for a name that is not a part.
    let light = world
        .get_member(MemberKind::Native, &me, "Thing", "SpriteLight")
        .unwrap();
    assert_eq!(light, Value::Raw(1));
    world
        .set_member(
            MemberKind::RefPtr,
            &light,
            "PSpriteLight",
            "FarDist",
            Value::Float(2.5),
        )
        .unwrap();
    assert_eq!(
        world.get_member(MemberKind::RefPtr, &light, "PSpriteLight", "FarDist"),
        Ok(Value::Float(2.5))
    );
    let other = Value::Thing(Some(2));
    assert_eq!(
        world.get_member(MemberKind::Native, &other, "Thing", "SpriteLight"),
        Ok(Value::Raw(0))
    );
    assert!(world
        .get_member(MemberKind::Native, &me, "Thing", "Colour")
        .is_err());
    assert_eq!(world.num_players(), Ok(1));
    assert_eq!(world.play_or_edit_mode(), Ok(0));
    // A sound request is counted by name; nothing plays.
    world.play_sound(Some(1), "gameplay/ping").unwrap();
    world.play_sound(None, "gameplay/ping").unwrap();
    assert_eq!(world.sounds_played["gameplay/ping"], 2);
    // A native inherited through Lamp's reference reaches the host as Base.Mode__.
    let mut host = Bindings::new(world);
    let inst = instances(&vm, &host.world)[0];
    let mut budget = 1000;
    let o = Runner::default().start(&mut vm, &mut host, inst, "Ask", &mut budget);
    assert!(matches!(o, Outcome::Fault(_)));
    assert_eq!(host.unbound.keys().collect::<Vec<_>>(), ["Base.Mode__"]);
}

#[test]
fn part4_anim_speed_tint_camera_tweak_and_player_numbers() {
    let mut things = level();
    things[1].parts.push("CAMERA_TWEAK");
    things[2].data.render_mesh = render_mesh(1);
    let mut world = LevelWorld::from_things(&things);
    assert_eq!(world.has_part(1, "PCameraTweak"), Ok(true));
    assert_eq!(world.has_part(2, "PCameraTweak"), Ok(false));
    world.set_anim_speed(1, 0.).unwrap();
    world.set_render_mesh_tint(2, [1., 0.5, 0., 1.]).unwrap();
    assert_eq!(world.things[1].anim_speed, Some(0.));
    assert_eq!(world.things[2].render_mesh_tint, Some([1., 0.5, 0., 1.]));
    assert!(world.set_anim_speed(99, 1.).is_err());
    // One record, player number 0; the game answers -1 outside the records.
    assert_eq!(world.player_number(0), Ok(0));
    assert_eq!(world.player_number(1), Ok(-1));
    assert_eq!(world.player_number(-1), Ok(-1));
}

#[test]
fn schedule_creates_then_updates_and_resumes_suspended_scripts() {
    // Lamp with `Tick` renamed to `Update__` and no `OnCreate__`.
    let mut ticker = lamp();
    ticker.strings_a[4] = lbp_runtime::UPDATE.into();
    let mut vm = Vm::default();
    vm.load(ticker, Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&level()));
    let mut level = LevelScripts::new(&vm, &host.world);
    let count = |host: &mut Bindings<LevelWorld>| {
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(1)), "Lamp", "count")
    };
    let created = level.create(&mut vm, &mut host, 1000);
    assert_eq!(created.no_function, 1);
    // Tick 1: Update runs and yields.
    let t = level.tick(&mut vm, &mut host, 1000);
    assert_eq!((t.yielded, t.resumed), (1, 0));
    assert_eq!(count(&mut host), Ok(Value::Int(42)));
    // Tick 2: the suspended Update finishes; no new Update for that Thing this tick.
    let t = level.tick(&mut vm, &mut host, 1000);
    assert_eq!((t.yielded, t.resumed, t.done), (0, 1, 0));
    assert_eq!(count(&mut host), Ok(Value::Int(42)));
    // Tick 3: Update runs again.
    let t = level.tick(&mut vm, &mut host, 1000);
    assert_eq!(t.yielded, 1);
    assert_eq!(count(&mut host), Ok(Value::Int(43)));
    assert_eq!((level.ticks, level.pending()), (3, 1));
    assert!(level.stopped.is_empty());
}

/// [`level`] with Thing 2 under the scripted Thing 1, carrying a PTrigger of radius 10
/// (1.5 times that while something is inside) at (100, 0, 0), scaled 2 in x and y.
fn trigger_level(trigger_type: i8) -> Vec<ThingRecord> {
    let mut t = level();
    t[2].parent = Some(1);
    t[2].parts = vec!["POS", "TRIGGER"];
    let mut world = [0.; 16];
    (world[0], world[5], world[10], world[15]) = (2., 2., 1., 1.);
    world[12] = 100.;
    t[2].data.pos = Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world,
    });
    t[2].data.trigger = Some(lbp_level::parts::Trigger {
        trigger_type,
        in_things: vec![],
        radius_multiplier: 10.,
        all_z_layers: false,
        hysteresis_multiplier: 1.5,
        enabled: true,
        z_offset: 0.,
    });
    t
}

#[test]
fn stage2_trigger_holds_the_player_by_shape_hysteresis_and_layer() {
    use lbp_runtime::TriggerEventKind::{Enter, Inside, Leave};
    let mut w = LevelWorld::from_things(&trigger_level(0));
    assert!(w.update_triggers().is_empty());
    let p = w.add_player([150., 0., 0.], (-1., 1.));
    assert_eq!(w.has_part(p, "PYellowHead"), Ok(true));
    let at = |w: &mut LevelWorld, x: f32, y: f32, layers| {
        w.move_player(p, [x, y, 0.], layers);
        w.update_triggers()
            .iter()
            .map(|e| {
                assert_eq!((e.trigger, e.thing), (2, p));
                e.kind
            })
            .collect::<Vec<_>>()
    };
    let here = (-1., 1.);
    // 25 away in a space scaled by 2: 12.5 in the trigger's space, outside radius 10.
    assert_eq!(at(&mut w, 125., 0., here), []);
    assert_eq!(at(&mut w, 119., 0., here), [Enter, Inside]);
    assert_eq!(w.trigger_count(2), Ok(1));
    // Inside, the radius is 15: 12.5 stays in, 15.5 is out.
    assert_eq!(at(&mut w, 125., 0., here), [Inside]);
    assert_eq!(at(&mut w, 131., 0., here), [Leave]);
    assert_eq!(w.trigger_count(2), Ok(0));
    // A circle, not a box: (9, 9) in its space is outside.
    assert_eq!(at(&mut w, 118., 18., here), []);
    // The trigger's z (0) must lie in the z span of the player's body.
    assert_eq!(at(&mut w, 100., 0., (50., 60.)), []);
    w.things[2].trigger.as_mut().unwrap().all_z_layers = true;
    assert_eq!(at(&mut w, 100., 0., (50., 60.)), [Enter, Inside]);
    w.things[2].trigger.as_mut().unwrap().enabled = false;
    assert_eq!(at(&mut w, 100., 0., (50., 60.)), [Leave]);

    let mut w = LevelWorld::from_things(&trigger_level(1));
    let p = w.add_player([118., 18., 0.], (-1., 1.));
    assert_eq!(w.update_triggers().len(), 2);
    w.move_player(p, [121., 0., 0.], (-1., 1.));
    // Still within the widened box (15).
    assert_eq!(w.update_triggers().len(), 1);
    // Type 2 holds nothing.
    let mut w = LevelWorld::from_things(&trigger_level(2));
    w.add_player([100., 0., 0.], (-1., 1.));
    assert!(w.update_triggers().is_empty());
}

#[test]
fn stage2_trigger_events_call_the_parent_script() {
    // Lamp with `Tick` renamed to the Enter entry; no Inside or Leave function.
    let mut lamp = lamp();
    lamp.strings_a[4] = "Enter__Q5Thing".into();
    let mut vm = Vm::default();
    vm.load(lamp, Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&trigger_level(0)));
    let mut level = LevelScripts::new(&vm, &host.world);
    let count = |host: &mut Bindings<LevelWorld>| {
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(1)), "Lamp", "count")
    };
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!((t.trigger_events, t.done + t.yielded), ([0, 0, 0], 0));
    let p = host.world.add_player([100., 0., 0.], (-1., 1.));
    // Enter runs on Thing 1 (the trigger's Thing 2 has no script) and yields.
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!((t.trigger_events, t.events_run), ([1, 1, 0], [1, 0, 0]));
    assert_eq!((t.yielded, t.events_without_script), (1, 0));
    assert_eq!(count(&mut host), Ok(Value::Int(42)));
    // Next tick the suspended Enter finishes; Inside has no function.
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(
        (t.trigger_events, t.events_run, t.resumed),
        ([0, 1, 0], [0, 0, 0], 1)
    );
    host.world.move_player(p, [900., 0., 0.], (-1., 1.));
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!((t.trigger_events, t.events_run), ([0, 0, 1], [0, 0, 0]));
    assert_eq!(count(&mut host), Ok(Value::Int(42)));
    assert!(level.stopped.is_empty());

    // A trigger with no script on its Thing or its parent raises events nobody takes.
    let mut things = trigger_level(0);
    things[2].parent = Some(0);
    let mut host = Bindings::new(LevelWorld::from_things(&things));
    let mut level = LevelScripts::new(&vm, &host.world);
    host.world.add_player([100., 0., 0.], (-1., 1.));
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!((t.trigger_events, t.events_without_script), ([1, 1, 0], 2));
}

/// Thing 0: a PSwitch with two outputs. Output 0 (activation 0.25, ternary -1, player 2)
/// goes to Thing 1 port 0, Thing 1 port 2 and a null Thing; output 1 (ternary 0, no
/// player) goes to Thing 2 port 1. Thing 3: a PSwitch whose only output also targets
/// Thing 1 port 0 (a collision) and a negative port. Thing 4: a PSwitch with a null output.
fn switch_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..5)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    let signal = |activation, ternary, player| SwitchSignal {
        activation,
        ternary,
        player,
    };
    t[0].data.switch = Some(Switch {
        outputs: vec![
            Some(SwitchOutput {
                signal: signal(0.25, -1, 2),
                targets: vec![(Some(1), 0), (Some(1), 2), (None, 0)],
            }),
            Some(SwitchOutput {
                signal: signal(0.75, 0, -1),
                targets: vec![(Some(2), 1)],
            }),
        ],
        ..Default::default()
    });
    t[3].data.switch = Some(Switch {
        outputs: vec![Some(SwitchOutput {
            signal: signal(1., 1, 0),
            targets: vec![(Some(1), 0), (Some(2), -1)],
        })],
        ..Default::default()
    });
    t[4].data.switch = Some(Switch {
        outputs: vec![None],
        ..Default::default()
    });
    t
}

#[test]
fn stage3_input_tables_are_built_from_the_switch_outputs() {
    let w = LevelWorld::from_things(&switch_level());
    let wire = |switch, output| Some(InputWire { switch, output });
    // As long as the highest wired port + 1; the first output on a port is kept.
    assert_eq!(w.things[1].inputs, [wire(0, 0), None, wire(0, 0)]);
    assert_eq!(w.things[2].inputs, [None, wire(0, 1)]);
    assert!(w.things[0].inputs.is_empty() && w.things[3].inputs.is_empty());
    assert_eq!((w.input_collisions, w.input_ports_dropped), (1, 1));
    assert_eq!(w.things[0].switch.as_ref().unwrap().outputs.len(), 2);
}

#[test]
fn stage3_switch_input_natives_follow_the_wired_output() {
    let mut host = Bindings::new(LevelWorld::from_things(&switch_level()));
    let mut port = |m: &str, thing: u32, p: i32| {
        host.call_native("Thing", m, &[Value::Thing(Some(thing)), Value::Int(p)])
            .unwrap()
    };
    let (yes, no) = (Value::Bool(true), Value::Bool(false));
    // Triggered is "something is wired", whatever the signal says.
    assert_eq!(port("IsSwitchTriggered__i", 1, 0), yes);
    assert_eq!(port("IsSwitchTriggered__i", 2, 1), yes);
    for (thing, p) in [(1, 1), (1, 3), (1, -1), (0, 0)] {
        assert_eq!(port("IsSwitchTriggered__i", thing, p), no);
        assert_eq!(port("GetSwitchActivationBinary__i", thing, p), no);
        assert_eq!(
            port("GetSwitchActivationAnalogue__i", thing, p),
            Value::Float(0.)
        );
        assert_eq!(
            port("GetSwitchActivationTernary__i", thing, p),
            Value::Float(0.)
        );
        assert_eq!(
            port("GetSwitchActivationPlayer__i", thing, p),
            Value::Int(-1)
        );
        assert_eq!(port("GetInput__Q5Thingi", thing, p), Value::Thing(None));
    }
    // Binary is "the ternary is not 0".
    assert_eq!(port("GetSwitchActivationBinary__i", 1, 2), yes);
    assert_eq!(port("GetSwitchActivationBinary__i", 2, 1), no);
    assert_eq!(
        port("GetSwitchActivationAnalogue__i", 1, 0),
        Value::Float(0.25)
    );
    assert_eq!(
        port("GetSwitchActivationAnalogue__i", 2, 1),
        Value::Float(0.75)
    );
    assert_eq!(
        port("GetSwitchActivationTernary__i", 1, 0),
        Value::Float(-1.)
    );
    assert_eq!(port("GetSwitchActivationPlayer__i", 1, 0), Value::Int(2));
    // A wired output without a player: -1 (the game's owner fallback is not modelled).
    assert_eq!(port("GetSwitchActivationPlayer__i", 2, 1), Value::Int(-1));
    assert_eq!(port("GetInput__Q5Thingi", 1, 2), Value::Thing(Some(0)));
    assert_eq!(port("GetInput__Q5Thingi", 2, 1), Value::Thing(Some(0)));
    // The signal is read through the wire, so a changed output is seen at once.
    host.world.things[0].switch.as_mut().unwrap().outputs[1]
        .as_mut()
        .unwrap()
        .signal
        .ternary = 1;
    assert_eq!(
        host.call_native(
            "Thing",
            "GetSwitchActivationBinary__i",
            &[Value::Thing(Some(2)), Value::Int(1)]
        ),
        Ok(yes)
    );
}

#[test]
fn stage3_outputs_in_things_and_score_boards() {
    let mut things = switch_level();
    things[2].data.trigger = Some(Trigger {
        in_things: vec![Some(4), Some(1)],
        ..Default::default()
    });
    things[1].data.script = Some(ScriptPart {
        script: Some(desc(SCORE_BOARD_SCRIPT)),
        fields: vec![],
    });
    things[2].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![],
    });
    things[3].data.script = Some(ScriptPart {
        script: Some(desc(BASE)),
        fields: vec![],
    });
    let mut host = Bindings::new(LevelWorld::from_things(&things));
    let thing = |t: u32| Value::Thing(Some(t));
    let mut call = |class: &str, m: &str, args: &[Value]| host.call_native(class, m, args).unwrap();
    let mut output = |t: Value, o: i32, i: i32| {
        call(
            "Thing",
            "GetOutput__Q5Thingii",
            &[t, Value::Int(o), Value::Int(i)],
        )
    };
    assert_eq!(output(thing(0), 0, 1), thing(1));
    assert_eq!(output(thing(0), 1, 0), thing(2));
    // A null target, a target or output index out of range, a null output, no PSwitch,
    // a null Thing.
    for (t, o, i) in [
        (thing(0), 0, 2),
        (thing(0), 0, 3),
        (thing(0), 0, -1),
        (thing(0), 2, 0),
        (thing(0), -1, 0),
        (thing(4), 0, 0),
        (thing(1), 0, 0),
        (Value::Thing(None), 0, 0),
    ] {
        assert_eq!(output(t.clone(), o, i), Value::Thing(None), "{t:?} {o} {i}");
    }
    let mut inside = |t: Value, i: i32| call("Native", "GetInThing__Q5Thingi", &[t, Value::Int(i)]);
    assert_eq!(inside(thing(2), 0), thing(4));
    assert_eq!(inside(thing(2), 1), thing(1));
    for (t, i) in [
        (thing(2), 2),
        (thing(2), -1),
        (thing(0), 0),
        (Value::Thing(None), 0),
    ] {
        assert_eq!(inside(t, i), Value::Thing(None));
    }
    // The scoreboard script itself, or a class with it on its superclass chain.
    let mut vm = Vm::default();
    vm.load(base(), Some(SCORE_BOARD_SCRIPT));
    vm.load(
        Script {
            super_class: Some(desc(SCORE_BOARD_SCRIPT)),
            ..lamp()
        },
        Some(GUID),
    );
    let board = |host: &mut Bindings<LevelWorld>, t: Value| {
        host.call_native("BaseWorld", "IsScoreBoard__Q5Thing", &[t])
    };
    assert_eq!(board(&mut host, thing(1)), Ok(Value::Bool(true)));
    assert_eq!(board(&mut host, thing(2)), Ok(Value::Bool(false)));
    host.world.add_class_defaults(&vm);
    assert_eq!(board(&mut host, thing(2)), Ok(Value::Bool(true)));
    assert_eq!(board(&mut host, thing(3)), Ok(Value::Bool(false)));
    assert_eq!(board(&mut host, thing(0)), Ok(Value::Bool(false)));
    assert_eq!(board(&mut host, Value::Thing(None)), Ok(Value::Bool(false)));
}

fn object(kind: u32, id: usize, body: Option<ScriptObject>) -> ScriptValue {
    ScriptValue::Object {
        kind,
        id: Some(id),
        body: body.map(Box::new),
    }
}

fn field(name: &str, value: ScriptValue) -> ScriptField {
    ScriptField {
        name: name.into(),
        machine_type: 0xb,
        value,
    }
}

#[test]
fn stored_objects_become_shared_heap_objects() {
    let mut vm = Vm::default();
    vm.load(base(), Some(BASE));
    let mut things = level();
    // An object array holding a back-reference to the `label` string, an instance of
    // `Base` and a second reference to that instance; a Thing array; a resource; and on
    // Thing 2 a reference to the same object array.
    let instance = ScriptPart {
        script: Some(desc(BASE)),
        fields: vec![ScriptField {
            name: "count".into(),
            machine_type: 3,
            value: ScriptValue::Int(5),
        }],
    };
    let items = vec![
        object(16, 0, None),
        object(13, 2, Some(ScriptObject::Instance(instance))),
        object(13, 2, None),
        ScriptValue::Object {
            kind: 0,
            id: None,
            body: None,
        },
    ];
    let part = things[1].data.script.as_mut().unwrap();
    part.fields.push(field(
        "items",
        object(11, 1, Some(ScriptObject::Objects(items))),
    ));
    part.fields.push(field(
        "targets",
        object(10, 3, Some(ScriptObject::Things(vec![Some(2), None]))),
    ));
    part.fields.push(field(
        "mesh",
        object(
            12,
            4,
            Some(ScriptObject::Resource {
                kind: 2,
                resource: None,
            }),
        ),
    ));
    things[2].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![field("same", object(11, 1, None))],
    });
    let mut world = LevelWorld::from_things(&things);
    assert_eq!(world.unconverted, 4);
    assert_eq!(world.bind_objects(&mut vm), 3);
    assert_eq!(world.unconverted, 1, "the resource stays null");
    let get = |w: &mut LevelWorld, t: u32, f| {
        w.get_member(MemberKind::Thing, &Value::Thing(Some(t)), "Lamp", f)
    };
    let Ok(Value::Object(Some(items))) = get(&mut world, 1, "items") else {
        panic!("items is not an object");
    };
    assert_eq!(get(&mut world, 2, "same"), Ok(Value::Object(Some(items))));
    let a = vm.array(items).unwrap().to_vec();
    assert_eq!(a[0], Value::Str("hi".into()));
    let Value::Object(Some(inst)) = a[1] else {
        panic!("instance expected");
    };
    assert_eq!(
        (&a[2], &a[3]),
        (&Value::Object(Some(inst)), &Value::Object(None))
    );
    assert_eq!(vm.field(inst, "count"), Some(&Value::Int(5)));
    assert_eq!(vm.field(inst, "tweaking"), Some(&Value::Bool(false)));
    let Ok(Value::Object(Some(targets))) = get(&mut world, 1, "targets") else {
        panic!("targets is not an object");
    };
    assert_eq!(
        vm.array(targets),
        Some(&[Value::Thing(Some(2)), Value::Thing(None)][..])
    );
    assert_eq!(get(&mut world, 1, "mesh"), Ok(Value::Object(None)));
}

#[test]
fn part6_frame_entry_gets_its_arguments_each_tick() {
    // `PlanetsWorld` whose per-tick export stores its +16 argument ("snap") in `count`.
    let mut planets = lamp();
    planets.class_name = "PlanetsWorld".into();
    planets.strings_a[0] = "PlanetsWorld".into();
    planets.strings_a[4] = "UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb".into();
    planets.bytecode = vec![
        i(0x97, 1, 16, 0, 0), // thing(r0).count = r16
        i(0xa5, 0, 0, 0, 0),  // RETURN
    ];
    let mut f = function(0, 4, 0, 2);
    f.stack_size = 32;
    planets.function_defs = vec![f];
    let mut vm = Vm::default();
    vm.load(planets, Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&level()));
    let mut level = LevelScripts::new(&vm, &host.world);
    let class = level.instances[0].class.unwrap();
    assert!(lbp_runtime::frame_entry(&vm, class).is_some());
    let snap = |host: &mut Bindings<LevelWorld>| {
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(1)), "X", "count")
    };
    let t = level.tick(&mut vm, &mut host, 1000);
    assert_eq!((t.done, t.no_function, t.faults.len()), (1, 0, 0));
    assert_eq!(snap(&mut host), Ok(Value::Bool(true)));
    level.tick(&mut vm, &mut host, 1000);
    assert_eq!(snap(&mut host), Ok(Value::Bool(false)));
}

#[test]
fn part6_camera_world_position_and_engine_state() {
    use lbp_script_vm::natives::dispatch;
    let mut w = LevelWorld::from_things(&level());
    let v = |x: f32| Value::Vec4([x, 0., 0., 1.]);
    let f = Value::Float;
    let args = [v(1.), v(2.), v(3.), f(10.), f(1e5), f(0.), f(1.)];
    dispatch(&mut w, "Cam", "SetCameraFromToUp__rrrffff", &args)
        .unwrap()
        .unwrap();
    dispatch(&mut w, "Cam", "SetDOFAndFog__fff", &[f(1.), f(2.), f(3.)])
        .unwrap()
        .unwrap();
    dispatch(&mut w, "Cam", "ClearMotionBlur__", &[])
        .unwrap()
        .unwrap();
    let cam = w.camera.from_to_up.unwrap();
    assert_eq!((cam.from[0], cam.to[0], cam.up[0]), (1., 2., 3.));
    assert_eq!(cam.params, [10., 1e5, 0., 1.]);
    assert_eq!(w.camera.dof_and_fog, Some([1., 2., 3.]));
    assert_eq!(
        (w.camera.from_to_up_calls, w.camera.motion_blur_clears),
        (1, 1)
    );
    let mut m = [0.; 16];
    m[12..].copy_from_slice(&[4., 5., 6., 1.]);
    let set = |w: &mut LevelWorld, t| {
        dispatch(w, "Thing", "SetWorldPosition__m", &[t, Value::Mat44(m)])
            .unwrap()
            .unwrap()
    };
    set(&mut w, Value::Thing(Some(1)));
    assert_eq!(w.world_transform(1), Ok(Some(m)));
    // Nothing happens for a Thing without a PPos, or for a null Thing.
    set(&mut w, Value::Thing(Some(2)));
    assert_eq!(w.world_transform(2), Ok(None));
    set(&mut w, Value::Thing(None));
    // Defaults of the PC run, and what is not there yet.
    let q = |w: &mut LevelWorld, c, n| dispatch(w, c, n, &[]).unwrap();
    assert_eq!(q(&mut w, "HUD", "GetCinemaModeCount__"), Ok(Value::Int(0)));
    assert_eq!(
        q(&mut w, "PodComputer", "GetPodComputerInUse__"),
        Ok(Value::Bool(false))
    );
    w.pod_computer_in_use = true;
    assert_eq!(
        q(&mut w, "PodComputer", "GetPodComputerInUse__"),
        Ok(Value::Bool(true))
    );
    w.pod_computer_in_use = false;
    assert_eq!(q(&mut w, "Game", "GetNumAvatars__"), Ok(Value::Int(0)));
    assert!(q(&mut w, "PodComputer", "GetPodComputer__").is_err());
    assert!(q(&mut w, "Gooey", "CreateGooey__i").is_err(), "arity");
    let flag = |w: &mut LevelWorld| {
        dispatch(
            w,
            "Profile",
            "IsLBP2GameProgressionFlagSet__i",
            &[Value::Int(2)],
        )
        .unwrap()
    };
    assert_eq!(flag(&mut w), Ok(Value::Bool(false)));
    w.progression_flags = 2;
    assert_eq!(flag(&mut w), Ok(Value::Bool(true)));
    // Rookie Test natives.
    let me = Value::Thing(Some(1));
    dispatch(&mut w, "Native", "SetAnimPos__Q5Thingf", &[me, f(0.5)])
        .unwrap()
        .unwrap();
    assert_eq!(w.things[1].anim_pos, Some(0.5));
    let (t, n) = (
        "Game",
        "SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi",
    );
    dispatch(&mut w, t, n, &[Value::Int(0), Value::Int(90)])
        .unwrap()
        .unwrap();
    assert_eq!(w.player_records[0].multiplier_timer, 90);
    // A player without a record: nothing happens.
    dispatch(&mut w, t, n, &[Value::Int(3), Value::Int(1)])
        .unwrap()
        .unwrap();
    assert_eq!(w.player_records.len(), 1);
    dispatch(&mut w, "Native", "DebugPrint__i", &[Value::Int(19)])
        .unwrap()
        .unwrap();
    assert_eq!(w.written, ["19"]);
}

#[test]
fn part6_engine_objects_and_pod_computer() {
    // A class named PodComputer with no code: the object is made and handed out.
    let mut pod = base();
    pod.class_name = "PodComputer".into();
    let mut vm = Vm::default();
    vm.load(pod, Some(BASE));
    let mut host = Bindings::new(LevelWorld::from_things(&level()));
    assert!(lbp_runtime::add_engine_objects(&mut vm, &mut host, 1000).is_empty());
    let o = host.world.pod_computer.expect("pod computer");
    assert_eq!(host.world.pod_computer(), Ok(Value::Object(Some(o))));
    assert!(host.world.gooey_pool.is_empty(), "no Gooey class loaded");
    assert!(host.world.create_gooey(0).is_err());
}

/// `Thread`: `Start__` passes `this` to the native `StartImpl__`; `ThreadMain__` writes 1,
/// yields, writes 2.
fn thread_class() -> Script {
    let int = |r: u16, v: u64| u64::from(0x03u8) | u64::from(r) << 16 | v << 32;
    Script {
        class_name: "Thread".into(),
        super_class: None,
        type_refs: vec![ty(11, 0), ty(11, 1)],
        field_refs: vec![],
        function_refs: vec![
            MemberRef {
                type_ref: 0,
                name: 5,
            },
            MemberRef {
                type_ref: 1,
                name: 2,
            },
        ],
        function_defs: vec![function(0, 3, 0, 3), function(0, 4, 3, 9)],
        bytecode: vec![
            i(0xa3, 11, 0, 0, 0), // ARG @0 = this
            i(0xa4, 0, 0, 0, 0),  // CALL Thread.StartImpl__ (native)
            i(0xa5, 0, 0, 0, 0),
            int(4, 1),
            i(0xa2, 3, 0, 4, 0), // WRITE r4
            i(0xa4, 0, 0, 1, 0), // CALL System.Yield
            int(4, 2),
            i(0xa2, 3, 0, 4, 0),
            i(0xa5, 0, 0, 0, 0),
        ],
        strings_a: [
            "Thread",
            "System",
            "Yield__",
            "Start__",
            lbp_runtime::THREAD_MAIN,
            "StartImpl__",
        ]
        .map(String::from)
        .to_vec(),
        ..lamp()
    }
}

/// `Spawner.OnCreate__`: `thing(this).t.Start()`.
fn spawner() -> Script {
    Script {
        class_name: "Spawner".into(),
        super_class: None,
        type_refs: vec![ty(10, 0), ty(11, 1)],
        field_refs: vec![MemberRef {
            type_ref: 0,
            name: 2,
        }],
        function_refs: vec![MemberRef {
            type_ref: 1,
            name: 3,
        }],
        function_defs: vec![function(0, 4, 0, 4)],
        bytecode: vec![
            i(0x95, 11, 4, 0, 0), // r4 = thing(r0).t
            i(0xa3, 11, 4, 0, 0), // ARG @0 = r4
            i(0xa4, 0, 0, 0, 0),  // CALL Thread.Start__
            i(0xa5, 0, 0, 0, 0),
        ],
        strings_a: ["Spawner", "Thread", "t", "Start__", lbp_runtime::CREATE]
            .map(String::from)
            .to_vec(),
        ..lamp()
    }
}

#[test]
fn script_thread_runs_until_yield_then_resumes_each_tick() {
    let mut vm = Vm::default();
    let thread = vm.load(thread_class(), Some(0xb25d));
    vm.load(spawner(), Some(GUID));
    let t = vm.instantiate(thread).unwrap();
    let mut world = LevelWorld::from_things(&level());
    world.things[1]
        .members
        .as_mut()
        .unwrap()
        .insert("t".into(), Value::Object(Some(t)));
    let mut host = Bindings::new(world);
    let mut level = LevelScripts::new(&vm, &host.world);
    let created = level.create(&mut vm, &mut host, 1000);
    assert_eq!((created.done, created.threads_started), (1, 1));
    assert!(
        created.thread_faults.is_empty(),
        "{:?}",
        created.thread_faults
    );
    assert_eq!(host.world.written, ["Int(1)"]);
    assert_eq!(level.threads(), 1);
    let tick = level.tick(&mut vm, &mut host, 1000);
    assert_eq!((tick.threads_done, tick.threads_started), (1, 0));
    assert_eq!(host.world.written, ["Int(1)", "Int(2)"]);
    assert_eq!(level.threads(), 0);
    // A thread object that is null faults the starting script, not the scheduler.
    host.world.things[1]
        .members
        .as_mut()
        .unwrap()
        .insert("t".into(), Value::Object(None));
    let mut again = LevelScripts::new(&vm, &host.world);
    let created = again.create(&mut vm, &mut host, 1000);
    assert_eq!((created.faults.len(), created.threads_started), (1, 0));
}

/// A class whose one function stores the argument register `reg` (machine type `ty`) in
/// the Thing's `count`.
fn reactor(name: &str, function: &str, ty: u8, reg: u16) -> Script {
    let mut s = lamp();
    s.class_name = name.into();
    s.super_class = None;
    s.strings_a[0] = name.into();
    s.strings_a[4] = function.into();
    s.bytecode = vec![
        i(0x97, ty, reg, 0, 0), // thing(r0).count = r<reg>
        i(0xa5, 0, 0, 0, 0),    // RETURN
    ];
    s.function_defs = vec![function_def_with_stack(32)];
    s
}

fn function_def_with_stack(stack_size: i32) -> FunctionDef {
    let mut f = function(0, 4, 0, 2);
    f.stack_size = stack_size;
    f
}

/// Thing 0: a PSwitch with one output (off) wired to Things 1..=6. Scripts: Thing 1 a
/// listed GUID with `__b`; Thing 2 a GUID that is not listed, with `__b`; Thing 3 the
/// `__bf` GUID; Thing 4 a listed GUID on a Thing with its own PSwitch; Thing 5 a listed
/// GUID whose class has no `__b`; Thing 6 no script.
fn reaction_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..7)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.switch = Some(Switch {
        outputs: vec![Some(SwitchOutput {
            signal: SwitchSignal {
                activation: 0.,
                ternary: 0,
                player: -1,
            },
            targets: (1..7).map(|i| (Some(i), 0)).collect(),
        })],
        ..Default::default()
    });
    t[4].data.switch = Some(Switch::default());
    for (thing, guid) in [(1, 18256), (2, 0x2d06), (3, 18420), (4, 18256), (5, 27452)] {
        t[thing].data.script = Some(ScriptPart {
            script: Some(desc(guid)),
            fields: vec![field("count", ScriptValue::Int(-1))],
        });
    }
    t
}

#[test]
fn stage3b_switch_reactions_go_to_listed_scripts_only() {
    use lbp_runtime::{REACTION_B, REACTION_BF};
    let mut vm = Vm::default();
    vm.load(reactor("Listed", REACTION_B, 1, 4), Some(18256));
    vm.load(reactor("Other", REACTION_B, 1, 4), Some(0x2d06));
    vm.load(reactor("Level", REACTION_BF, 4, 8), Some(18420));
    vm.load(reactor("Mute", "Tick", 1, 4), Some(27452));
    let mut host = Bindings::new(LevelWorld::from_things(&reaction_level()));
    let mut level = LevelScripts::new(&vm, &host.world);
    let count = |host: &mut Bindings<LevelWorld>, thing: u32| {
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(thing)), "X", "count")
            .unwrap()
    };
    let total = |m: &std::collections::BTreeMap<u32, usize>| m.values().sum::<usize>();
    // The stored signals raise nothing, on the first tick or later.
    for _ in 0..2 {
        let t = level.tick_level(&mut vm, &mut host, 1000);
        assert_eq!(t.reactions, Default::default());
    }
    assert_eq!(count(&mut host, 1), Value::Int(-1));

    host.world.switch_pulse = Some(true);
    let t = level.tick_level(&mut vm, &mut host, 1000);
    let r = &t.reactions;
    assert_eq!(
        (r.targets_visited, r.targets_on, r.without_script),
        (6, 6, 1)
    );
    assert_eq!(r.calls_b, [(18256, 1)].into());
    assert_eq!(r.calls_bf, [(18420, 1)].into());
    assert_eq!(r.not_whitelisted, [(0x2d06, 1)].into());
    assert_eq!(r.shadowed_by_switch, [(18256, 1)].into());
    assert_eq!(r.no_function, [(27452, 1)].into());
    assert!(t.faults.is_empty(), "{:?}", t.faults);
    assert_eq!(count(&mut host, 1), Value::Bool(true));
    assert_eq!(count(&mut host, 3), Value::Float(1.));
    // Not listed, or its Thing is a switch: the function exists but is not called.
    assert_eq!(count(&mut host, 2), Value::Int(-1));
    assert_eq!(count(&mut host, 4), Value::Int(-1));
    // Held on: no edge, no call.
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(t.reactions, Default::default());

    host.world.switch_pulse = Some(false);
    let t = level.tick_level(&mut vm, &mut host, 1000);
    let r = &t.reactions;
    assert_eq!((r.targets_visited, r.targets_on), (6, 0));
    assert_eq!((total(&r.calls_b), total(&r.calls_bf)), (1, 1));
    assert_eq!(count(&mut host, 1), Value::Bool(false));
    assert_eq!(count(&mut host, 3), Value::Float(0.));
    assert!(level.stopped.is_empty());

    // A Thing stopped by a fault gets no reaction.
    level.stopped.insert(1);
    host.world.switch_pulse = Some(true);
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(t.reactions.to_stopped, [(18256, 1)].into());
    assert!(t.reactions.calls_b.is_empty());
}

/// Things 0 and 1: batteries, both on, wired to ports 0 and 1 of an AND gate (Thing 2);
/// the gate feeds an inverted NOT gate (Thing 3), which feeds Thing 4, a listed script.
fn gate_chain_level() -> Vec<ThingRecord> {
    let signal = |on: bool| SwitchSignal {
        activation: f32::from(u8::from(on)),
        ternary: i32::from(on),
        player: -1,
    };
    let part = |switch_type, on, target: (usize, i32)| Switch {
        switch_type,
        outputs: vec![Some(SwitchOutput {
            signal: signal(on),
            targets: vec![(Some(target.0), target.1)],
        })],
        manual_activation: signal(on),
        bullets_required: 2,
        ..Default::default()
    };
    let mut t: Vec<ThingRecord> = (0..5)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.switch = Some(part(28, true, (2, 0)));
    t[1].data.switch = Some(part(28, true, (2, 1)));
    t[2].data.switch = Some(part(14, true, (3, 0)));
    t[3].data.switch = Some(Switch {
        inverted: true,
        ..part(23, false, (4, 0))
    });
    t[4].data.script = Some(ScriptPart {
        script: Some(desc(18256)),
        fields: vec![field("count", ScriptValue::Int(-1))],
    });
    t
}

#[test]
fn stage3b_a_gate_chain_fires_the_reaction_true_then_false() {
    use lbp_runtime::REACTION_B;
    let mut vm = Vm::default();
    vm.load(reactor("Listed", REACTION_B, 1, 4), Some(18256));
    let mut host = Bindings::new(LevelWorld::from_things(&gate_chain_level()));
    let mut level = LevelScripts::new(&vm, &host.world);
    let count = |host: &mut Bindings<LevelWorld>| {
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(4)), "X", "count")
            .unwrap()
    };
    let set_battery = |host: &mut Bindings<LevelWorld>, on: bool| {
        let manual = &mut host.world.things[1].switch.as_mut().unwrap().memory.manual;
        (manual.activation, manual.ternary) = (f32::from(u8::from(on)), i32::from(on));
    };
    // The stored state is settled: both batteries on, AND on, NOT off. Nothing fires.
    for _ in 0..2 {
        let t = level.tick_level(&mut vm, &mut host, 1000);
        assert_eq!(t.reactions, Default::default());
    }
    assert_eq!(count(&mut host), Value::Int(-1));
    // One battery off: the AND falls and the NOT rises within the same pass.
    set_battery(&mut host, false);
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert!(t.faults.is_empty(), "{:?}", t.faults);
    // Visited: the gate's port (falling), the NOT's port (falling), the script (rising).
    assert_eq!(
        (t.reactions.targets_visited, t.reactions.targets_on),
        (3, 1)
    );
    assert_eq!(t.reactions.calls_b, [(18256, 1)].into());
    assert_eq!(count(&mut host), Value::Bool(true));
    // Held: no call.
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(t.reactions, Default::default());
    // Back on: the NOT falls, the script gets false.
    set_battery(&mut host, true);
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(
        (t.reactions.targets_visited, t.reactions.targets_on),
        (3, 2)
    );
    assert_eq!(t.reactions.calls_b, [(18256, 1)].into());
    assert_eq!(count(&mut host), Value::Bool(false));
}

// ---- stage 4: world, position and transform state (docs/rscript-plan.md) ----

fn identity() -> [f32; 16] {
    lbp_runtime::transform::IDENTITY
}

fn translation(x: f32, y: f32, z: f32) -> [f32; 16] {
    let mut m = identity();
    m[12..15].copy_from_slice(&[x, y, z]);
    m
}

/// A quarter turn about z (x axis to +y), scaled by `s`, at (x, y, z).
fn turned(s: f32, x: f32, y: f32, z: f32) -> [f32; 16] {
    let mut m = translation(x, y, z);
    (m[0], m[1], m[4], m[5], m[10]) = (0., s, -s, 0., s);
    m
}

fn pos(world: [f32; 16]) -> Option<Pos> {
    Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world,
    })
}

fn close(a: &[f32; 16], b: &[f32; 16]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
}

/// The part handle `thing.<member>` gives a script.
fn part(w: &mut LevelWorld, thing: u32, member: &str) -> Value {
    w.get_member(
        MemberKind::Native,
        &Value::Thing(Some(thing)),
        "Thing",
        member,
    )
    .unwrap()
}

#[test]
fn stage4_pworld_members_are_the_stored_values_and_the_frame_counts_ticks() {
    let mut things = level();
    things[0].parts = vec!["WORLD"];
    things[0].data.world = Some(WorldPart {
        game_mode: 1,
        frame: 100,
        sim_frame: 90,
        water_level: 3.5,
        is_paused: true,
        global_settings_thing_uids: vec![7, 0],
        global_settings_thing_priority: vec![1, 0],
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&things);
    // The stored arrays, padded to the game's 8 slots.
    assert_eq!(
        w.pworld.global_settings_thing_uids,
        [7, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        w.pworld.global_settings_thing_priority,
        [1, 0, 0, 0, 0, 0, 0, 0]
    );
    assert!(w.pworld.is_paused);
    // `World.World`: the PWorld of the world Thing.
    let world = part(&mut w, 0, "World");
    assert_ne!(world, Value::Raw(0));
    let get = |w: &mut LevelWorld, f| w.get_member(MemberKind::RefPtr, &world, "PWorld", f);
    assert_eq!(get(&mut w, "GameMode"), Ok(Value::Int(1)));
    assert_eq!(get(&mut w, "Frame"), Ok(Value::Int(100)));
    assert_eq!(get(&mut w, "WaterLevel"), Ok(Value::Float(3.5)));
    // A member the runtime does not keep still says so.
    assert!(get(&mut w, "SubLevel")
        .unwrap_err()
        .contains("not read yet"));
    // A write is kept and read back; the wrong type is refused.
    let set = |w: &mut LevelWorld, f, v| w.set_member(MemberKind::RefPtr, &world, "PWorld", f, v);
    set(&mut w, "GameMode", Value::Int(5)).unwrap();
    assert_eq!(get(&mut w, "GameMode"), Ok(Value::Int(5)));
    assert!(set(&mut w, "GameMode", Value::Float(5.)).is_err());
    assert_eq!(w.pworld.game_mode, 5);

    // One frame per level tick; create and the first tick read the stored frame.
    let vm = &mut Vm::default();
    let mut host = Bindings::new(w);
    let mut level = LevelScripts::new(vm, &host.world);
    level.create(vm, &mut host, 1000);
    assert_eq!(host.world.pworld.frame, 100);
    level.tick_level(vm, &mut host, 1000);
    level.tick_level(vm, &mut host, 1000);
    assert_eq!(
        host.world
            .get_member(MemberKind::RefPtr, &world, "PWorld", "Frame"),
        Ok(Value::Int(102))
    );
    assert_eq!(host.world.pworld.sim_frame, 92);
    // A level without a PWorld reads the defaults.
    let w = LevelWorld::from_things(&[ThingRecord::default()]);
    assert_eq!(w.pworld, lbp_runtime::WorldState::default());
}

#[test]
fn stage4_ppos_and_ptrigger_members_are_the_things_own_state() {
    let mut w = LevelWorld::from_things(&trigger_level(0));
    // `thing.Pos` is the PPos (the name keeps its own P), null without the part.
    let ppos = part(&mut w, 2, "Pos");
    assert_ne!(ppos, Value::Raw(0));
    assert_eq!(part(&mut w, 0, "Pos"), Value::Raw(0));
    let stored = w.things[2].world_matrix.unwrap();
    assert_eq!(
        w.get_member(MemberKind::RefPtr, &ppos, "PPos", "WorldPosition"),
        Ok(Value::Mat44(stored))
    );
    // Thing 2's parent (Thing 1) has a flat matrix, so its local matrix is its world one.
    assert_eq!(w.things[2].local_matrix, Some(stored));

    let trigger = part(&mut w, 2, "Trigger");
    let get = |w: &mut LevelWorld, f| w.get_member(MemberKind::RefPtr, &trigger, "PTrigger", f);
    assert_eq!(get(&mut w, "RadiusMultiplier"), Ok(Value::Float(10.)));
    assert_eq!(get(&mut w, "HysteresisMultiplier"), Ok(Value::Float(1.5)));
    assert_eq!(get(&mut w, "AllZLayers"), Ok(Value::Bool(false)));

    // A player 60 away in z and outside the layer span is not inside ...
    let p = w.add_player([100., 0., 60.], (59., 61.));
    assert!(w.update_triggers().is_empty());
    // ... until a script sets AllZLayers: the write reaches the trigger test.
    let set =
        |w: &mut LevelWorld, f, v| w.set_member(MemberKind::RefPtr, &trigger, "PTrigger", f, v);
    set(&mut w, "AllZLayers", Value::Bool(true)).unwrap();
    assert!(w.things[2].trigger.as_ref().unwrap().all_z_layers);
    assert_eq!(get(&mut w, "AllZLayers"), Ok(Value::Bool(true)));
    assert_eq!(w.update_triggers().len(), 2);
    set(&mut w, "HysteresisMultiplier", Value::Float(4.)).unwrap();
    set(&mut w, "RadiusMultiplier", Value::Float(20.)).unwrap();
    let g = w.things[2].trigger.as_ref().unwrap();
    assert_eq!((g.radius, g.hysteresis), (20., 4.));
    assert!(set(&mut w, "AllZLayers", Value::Int(1)).is_err());
    assert!(set(&mut w, "RadiusMultiplier", Value::Bool(true)).is_err());

    // Writing PPos.WorldPosition moves the trigger: the player is left behind.
    let mut moved = stored;
    moved[12] = 5000.;
    w.set_member(
        MemberKind::RefPtr,
        &ppos,
        "PPos",
        "WorldPosition",
        Value::Mat44(moved),
    )
    .unwrap();
    assert_eq!(w.things[2].world_matrix, Some(moved));
    // A plain field write: the local matrix is not worked out again.
    assert_eq!(w.things[2].local_matrix, Some(stored));
    let events = w.update_triggers();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, lbp_runtime::TriggerEventKind::Leave);
    assert_eq!(events[0].thing, p);
    assert!(w
        .set_member(
            MemberKind::RefPtr,
            &ppos,
            "PPos",
            "WorldPosition",
            Value::Vec4([0.; 4])
        )
        .is_err());
    // Other members of the same parts stay in the handle's own map.
    set(&mut w, "Custom", Value::Int(3)).unwrap();
    assert_eq!(get(&mut w, "Custom"), Ok(Value::Int(3)));
}

/// World Thing 0 (no PPos). Thing 1 under it; Thing 2 under 1; Thing 3 under 2; Thing 4
/// under 2 without a PPos; Thing 5 under 4; Thing 6 under the world, no PPos.
fn transform_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..7)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    for (thing, parent, world) in [
        (1, 0, Some(turned(2., 10., 0., 0.))),
        (2, 1, Some(translation(10., 4., 0.))),
        (3, 2, Some(turned(1., 11., 4., 5.))),
        (4, 2, None),
        (5, 4, Some(translation(1., 1., 1.))),
        (6, 0, None),
    ] {
        t[thing].parent = Some(parent);
        if let Some(world) = world {
            t[thing].parts = vec!["POS"];
            t[thing].data.pos = pos(world);
        }
    }
    t
}

#[test]
fn stage4_local_matrices_are_derived_and_the_transform_natives_move_the_tree() {
    use lbp_runtime::transform::{inverse, mul};
    use lbp_script_vm::natives::dispatch;
    let things = transform_level();
    let mut w = LevelWorld::from_things(&things);
    let world = |w: &LevelWorld, t: usize| w.things[t].world_matrix.unwrap();
    let local = |w: &LevelWorld, t: usize| w.things[t].local_matrix.unwrap();
    let stored: Vec<_> = things
        .iter()
        .map(|t| t.data.pos.as_ref().map(|p| p.world))
        .collect();
    // local = inverse(parent world) x world; without a parent PPos it is the world matrix.
    assert_eq!(local(&w, 1), stored[1].unwrap());
    let inv1 = inverse(&stored[1].unwrap()).unwrap();
    assert!(close(&local(&w, 2), &mul(&inv1, &stored[2].unwrap())));
    // Thing 2 sits 4 along the parent's y axis in world terms: (2, 0, 0) in its space.
    assert!(close(
        &mul(&stored[1].unwrap(), &local(&w, 2)),
        &stored[2].unwrap()
    ));
    assert!((local(&w, 2)[12] - 2.).abs() < 1e-4 && local(&w, 2)[13].abs() < 1e-4);
    assert_eq!(local(&w, 5), stored[5].unwrap());
    assert_eq!(w.things[4].local_matrix, None);
    // Loading changes no stored world matrix.
    for (t, m) in stored.iter().enumerate() {
        assert_eq!(w.things[t].world_matrix, *m);
    }

    let call = |w: &mut LevelWorld, name: &str, args: &[Value]| {
        dispatch(w, "Thing", name, args).unwrap().unwrap()
    };
    let me = |t: u32| Value::Thing(Some(t));
    // The getters return matrices.
    assert_eq!(
        call(&mut w, "GetWorldPosition__", &[me(2)]),
        Value::Mat44(stored[2].unwrap())
    );
    assert_eq!(
        call(&mut w, "GetLocalPosition__", &[me(2)]),
        Value::Mat44(local(&w, 2))
    );
    // A null Thing or one without a PPos: the identity, and the setters do nothing.
    for t in [Value::Thing(None), me(4)] {
        for name in ["GetWorldPosition__", "GetLocalPosition__"] {
            let got = call(&mut w, name, std::slice::from_ref(&t));
            assert_eq!(got, Value::Mat44(identity()));
        }
        for name in ["SetLocalPosition__m", "SetWorldPosition__m"] {
            let m = Value::Mat44(translation(9., 9., 9.));
            assert_eq!(call(&mut w, name, &[t.clone(), m]), Value::Void);
        }
    }
    assert_eq!(w.things[4].world_matrix, None);
    assert_eq!(world(&w, 5), stored[5].unwrap());
    assert!(dispatch(
        &mut w,
        "Thing",
        "SetLocalPosition__m",
        &[me(2), Value::Int(1)]
    )
    .unwrap()
    .is_err());

    // SetLocalPosition: world = parent world x local, and the descendants follow.
    let local3 = local(&w, 3);
    let new_local = translation(0., 1., 0.);
    call(
        &mut w,
        "SetLocalPosition__m",
        &[me(2), Value::Mat44(new_local)],
    );
    assert_eq!(local(&w, 2), new_local);
    let world2 = mul(&stored[1].unwrap(), &new_local);
    assert_eq!(world(&w, 2), world2);
    // (0, 1, 0) in the parent's space is 2 along -x from (10, 0, 0), turned as the parent.
    assert!(close(&world2, &turned(2., 8., 0., 0.)));
    assert_eq!(local(&w, 3), local3);
    assert_eq!(world(&w, 3), mul(&world2, &local3));
    // Thing 5's parent has no PPos: it stays.
    assert_eq!(world(&w, 5), stored[5].unwrap());
    // Thing 1 (the parent) did not move.
    assert_eq!(world(&w, 1), stored[1].unwrap());

    // SetWorldPosition on the root of the group: its local matrix is the new world one
    // (the world Thing has no PPos), the locals below are kept and the worlds follow.
    let new_world = translation(100., 200., 300.);
    call(
        &mut w,
        "SetWorldPosition__m",
        &[me(1), Value::Mat44(new_world)],
    );
    assert_eq!((world(&w, 1), local(&w, 1)), (new_world, new_world));
    assert_eq!(local(&w, 2), new_local);
    assert!(close(&world(&w, 2), &translation(100., 201., 300.)));
    assert!(close(&world(&w, 3), &mul(&world(&w, 2), &local3)));
    // SetWorldPosition on a child: local = inverse(parent world) x world.
    call(
        &mut w,
        "SetWorldPosition__m",
        &[me(2), Value::Mat44(translation(90., 200., 300.))],
    );
    assert!(close(&local(&w, 2), &translation(-10., 0., 0.)));
    assert!(close(
        &world(&w, 3),
        &mul(&translation(90., 200., 300.), &local3)
    ));
    // A parent with a flat matrix counts as no parent.
    let mut flat = identity();
    flat[10] = 0.;
    call(&mut w, "SetWorldPosition__m", &[me(1), Value::Mat44(flat)]);
    let far = translation(1., 2., 3.);
    call(&mut w, "SetWorldPosition__m", &[me(2), Value::Mat44(far)]);
    assert_eq!((world(&w, 2), local(&w, 2)), (far, far));
}

/// `Probe.OnCreate__`: reads its own world matrix, stores the position's z in `height`,
/// then writes the matrix back with z replaced by x.
fn probe() -> Script {
    let mut s = lamp();
    s.class_name = "Probe".into();
    s.super_class = None;
    s.strings_a = [
        "Probe",
        "Thing",
        "height",
        "GetWorldPosition__",
        lbp_runtime::CREATE,
        "SetWorldPosition__m",
    ]
    .map(String::from)
    .to_vec();
    s.type_refs = vec![ty(10, 0), ty(10, 1)];
    s.field_refs = vec![MemberRef {
        type_ref: 0,
        name: 2,
    }];
    s.function_refs = vec![
        MemberRef {
            type_ref: 1,
            name: 3,
        },
        MemberRef {
            type_ref: 1,
            name: 5,
        },
    ];
    s.bytecode = vec![
        i(0xa3, 10, 0, 0, 0),     // ARG @0 = this
        i(0xa4, 6, 4, 0, 0),      // r4 = CALL Thing.GetWorldPosition__ (m44)
        i(0x78, 0, 8, 4, 0),      // r8 = GET_M44_cW r4
        i(0x5c, 0, 12, 8, 0),     // r12 = GET_V4_Z r8
        i(0x97, 4, 12, 0, 0),     // thing(r0).height = r12
        i(0x5a, 0, 13, 8, 0),     // r13 = GET_V4_X r8
        i(0x7b, 0, 13, 8, 0),     // SET_V4_Z r8 = r13
        i(0x94, 0, 8, 4, 0),      // SET_M44_cW r4 = r8
        i(0xa3, 10, 0, 0, 0),     // ARG @0 = this
        i(0xa3, 6, 4, 16, 0),     // ARG @16 = r4
        i(0xa4, 0, 0xffff, 1, 0), // CALL Thing.SetWorldPosition__m
        i(0xa5, 0, 0, 0, 0),      // RETURN
    ];
    let mut f = function(0, 4, 0, 12);
    f.stack_size = 64;
    s.function_defs = vec![f];
    s
}

#[test]
fn stage4_a_script_takes_the_position_out_of_the_world_matrix() {
    let mut things = transform_level();
    things[2].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![field("height", ScriptValue::Float(0.))],
    });
    things[2].data.pos = pos(translation(10., 4., 7.));
    let mut vm = Vm::default();
    vm.load(probe(), Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&things));
    let child_local = host.world.things[3].local_matrix.unwrap();
    let mut level = LevelScripts::new(&vm, &host.world);
    let created = level.create(&mut vm, &mut host, 1000);
    assert_eq!(
        (created.done, created.faults.len()),
        (1, 0),
        "{:?}",
        created.faults
    );
    assert!(host.unbound.is_empty());
    // `GetWorldPosition().cW.z` is the Thing's z.
    assert_eq!(
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(2)), "Probe", "height"),
        Ok(Value::Float(7.))
    );
    // The matrix written back has z = x, and the child followed.
    let m = host.world.things[2].world_matrix.unwrap();
    assert_eq!(m, translation(10., 4., 10.));
    assert!(close(
        &host.world.things[3].world_matrix.unwrap(),
        &lbp_runtime::transform::mul(&m, &child_local)
    ));
}

/// Thing 1: a microchip (PSwitch type 36) at (50, 0, 0) whose board is Thing 2; Thing 3 a
/// component on the board at (7, 0, 0). Thing 4: a sequencer whose board is Thing 5, with
/// the component Thing 6. Thing 7: a switch whose output (off) is wired to port 0 of the
/// chip. Thing 8: loose, under the world. Thing 9: a chip without a PPos on the first
/// chip's board, with its own board Thing 10 and the component Thing 11 on it.
fn chip_level() -> Vec<ThingRecord> {
    use lbp_level::parts::Microchip;
    let mut t: Vec<ThingRecord> = (0..12)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    let logic = |switch_type| {
        Some(Switch {
            switch_type,
            ..Default::default()
        })
    };
    for (thing, parent) in [
        (1, 0),
        (2, 1),
        (3, 2),
        (4, 0),
        (5, 4),
        (6, 5),
        (7, 0),
        (8, 0),
        (9, 2),
        (10, 9),
        (11, 10),
    ] {
        t[thing].parent = Some(parent);
    }
    t[9].parts = vec!["SWITCH", "MICROCHIP"];
    t[9].data.switch = logic(36);
    t[9].data.microchip = Some(Microchip {
        circuit_board: Some(10),
    });
    t[10].parts = vec!["SWITCH"];
    t[10].data.switch = logic(37);
    t[11].parts = vec!["POS"];
    t[11].data.pos = pos(translation(9., 9., 9.));
    t[1].parts = vec!["POS", "SWITCH", "MICROCHIP"];
    t[1].data.pos = pos(translation(50., 0., 0.));
    t[1].data.switch = logic(36);
    t[1].data.microchip = Some(Microchip {
        circuit_board: Some(2),
    });
    t[2].parts = vec!["SWITCH"];
    t[2].data.switch = logic(37);
    t[3].parts = vec!["POS"];
    t[3].data.pos = pos(translation(7., 0., 0.));
    t[4].parts = vec!["POS", "SEQUENCER", "MICROCHIP"];
    t[4].data.pos = pos(translation(-20., 0., 0.));
    t[4].data.microchip = Some(Microchip {
        circuit_board: Some(5),
    });
    t[5].parts = vec!["SWITCH"];
    t[5].data.switch = logic(37);
    t[6].parts = vec!["POS"];
    t[6].data.pos = pos(translation(3., 0., 0.));
    t[7].parts = vec!["SWITCH"];
    t[7].data.switch = Some(Switch {
        switch_type: 28,
        outputs: vec![Some(SwitchOutput {
            signal: SwitchSignal {
                activation: 0.,
                ternary: 0,
                player: -1,
            },
            targets: vec![(Some(1), 0)],
        })],
        ..Default::default()
    });
    t[8].parts = vec!["POS"];
    t[8].data.pos = pos(translation(1., 2., 3.));
    t
}

#[test]
fn stage4_inactive_microchip_proxy_sensor_position_and_sequencer() {
    use lbp_script_vm::natives::dispatch;
    let mut w = LevelWorld::from_things(&chip_level());
    let call = |w: &mut LevelWorld, class: &str, name: &str, args: &[Value]| {
        dispatch(w, class, name, args).unwrap()
    };
    let me = |t: u32| Value::Thing(Some(t));

    // IsInactive: the deactivated flag. The chip's port 0 is wired and off, so the chip,
    // its board and the component on it are inactive, before any switch pass ran.
    let inactive = |w: &mut LevelWorld, t: u32| call(w, "Thing", "IsInactive__", &[me(t)]);
    for t in [1, 2, 3] {
        assert_eq!(inactive(&mut w, t), Ok(Value::Bool(true)), "thing {t}");
    }
    for t in [0, 6, 7, 8] {
        assert_eq!(inactive(&mut w, t), Ok(Value::Bool(false)), "thing {t}");
    }
    // It follows the flag the switch pass keeps.
    w.set_deactivated(3, false);
    assert_eq!(inactive(&mut w, 3), Ok(Value::Bool(false)));
    assert!(inactive(&mut w, 99).is_err());
    assert!(call(&mut w, "Thing", "IsInactive__", &[Value::Thing(None)]).is_err());

    // The microchip proxy: the chip for a component on its board, else the Thing itself.
    let proxy = |w: &mut LevelWorld, t: Value| {
        call(w, "Thing", "GetMicroChipProxyFromThing__Q5Thing", &[t])
    };
    assert_eq!(proxy(&mut w, me(3)), Ok(me(1)));
    assert_eq!(proxy(&mut w, me(6)), Ok(me(4)));
    for t in [0, 1, 2, 8] {
        assert_eq!(proxy(&mut w, me(t)), Ok(me(t)), "thing {t}");
    }
    assert_eq!(proxy(&mut w, Value::Thing(None)), Ok(Value::Thing(None)));
    assert_eq!(w.microchip_of(3), Some(1));
    assert_eq!(w.microchip_of(8), None);
    // A chip inside a chip: the outermost one, which is the one with a PPos.
    assert_eq!(proxy(&mut w, me(11)), Ok(me(1)));
    assert_eq!(proxy(&mut w, me(9)), Ok(me(1)));
    assert_eq!(inactive(&mut w, 11), Ok(Value::Bool(true)));

    // The sensor position: the world matrix of the proxy.
    let sensor =
        |w: &mut LevelWorld, t: Value| call(w, "Thing", "GetWorldPositionForSensor__", &[t]);
    assert_eq!(
        sensor(&mut w, me(3)),
        Ok(Value::Mat44(translation(50., 0., 0.)))
    );
    assert_eq!(
        sensor(&mut w, me(8)),
        Ok(Value::Mat44(translation(1., 2., 3.)))
    );
    assert_eq!(
        sensor(&mut w, me(11)),
        Ok(Value::Mat44(translation(50., 0., 0.)))
    );
    // The board has no PPos and no chip of its own: the identity, as for a null Thing.
    assert_eq!(sensor(&mut w, me(2)), Ok(Value::Mat44(identity())));
    assert_eq!(
        sensor(&mut w, Value::Thing(None)),
        Ok(Value::Mat44(identity()))
    );

    // IsOnSequencer: the parent is a circuit board whose parent has a PSequencer.
    let on = |w: &mut LevelWorld, t: Value| call(w, "Game", "IsOnSequencer__Q5Thing", &[t]);
    assert_eq!(on(&mut w, me(6)), Ok(Value::Bool(true)));
    for t in [3, 5, 4, 8, 0] {
        assert_eq!(on(&mut w, me(t)), Ok(Value::Bool(false)), "thing {t}");
    }
    assert_eq!(on(&mut w, Value::Thing(None)), Ok(Value::Bool(false)));
}

/// Calls a bound native on the world; panics when the native has no binding.
fn native(w: &mut LevelWorld, class: &str, name: &str, args: &[Value]) -> Result<Value, String> {
    lbp_script_vm::natives::dispatch(w, class, name, args)
        .unwrap_or_else(|| panic!("{class}.{name} is not bound"))
}

/// A `Game` native that takes a player number first.
fn game(w: &mut LevelWorld, name: &str, args: &[Value]) -> Value {
    native(w, "Game", name, args).unwrap()
}

#[test]
fn stage4_player_records_and_player_queries() {
    let int = Value::Int;
    let mut w = LevelWorld::from_things(&level());
    // A read level: the record of local player 0, and no Sackboy yet.
    assert_eq!(w.player_records.len(), 1);
    assert_eq!(game(&mut w, "GetNumPlayers__", &[]), int(1));
    assert_eq!(game(&mut w, "GetPlayerNumber__i", &[int(0)]), int(0));
    for index in [-1, 1, 7] {
        assert_eq!(game(&mut w, "GetPlayerNumber__i", &[int(index)]), int(-1));
    }
    assert_eq!(game(&mut w, "EPlayerNumberFromInt__i", &[int(3)]), int(3));
    let exists = "DoesPlayerExist__Q18Game.EPlayerNumber";
    let local = "PlayerNumberIsLocalPlayer__Q18Game.EPlayerNumber";
    let local_id = "IsLocalPlayerID__Q18Game.EPlayerNumber";
    let head = "GetYellowheadFromPlayerNumber__Q18Game.EPlayerNumber";
    for (name, with_record) in [
        (exists, true),
        ("IsPlayerConnected__i", true),
        (local, true),
    ] {
        assert_eq!(game(&mut w, name, &[int(0)]), Value::Bool(with_record));
        assert_eq!(game(&mut w, name, &[int(1)]), Value::Bool(false), "{name}");
        assert_eq!(game(&mut w, name, &[int(-1)]), Value::Bool(false));
    }
    // Before his Sackboy is there the player has no yellowhead, so his ID is not local.
    assert_eq!(game(&mut w, head, &[int(0)]), Value::Thing(None));
    assert_eq!(game(&mut w, local_id, &[int(0)]), Value::Bool(false));
    assert_eq!(game(&mut w, "GetPartyLeadersPlayerNumber__", &[]), int(0));

    let sackboy = w.add_player([1., 2., 3.], (2., 4.));
    assert_eq!(w.player_records.len(), 1, "the record was already there");
    assert_eq!(w.players[0].number, 0);
    assert_eq!(game(&mut w, head, &[int(0)]), Value::Thing(Some(sackboy)));
    assert_eq!(game(&mut w, local_id, &[int(0)]), Value::Bool(true));
    assert_eq!(game(&mut w, head, &[int(1)]), Value::Thing(None));
    // A second Sackboy brings his record with him.
    let second = w.add_player([5., 2., 3.], (2., 4.));
    assert_eq!(game(&mut w, "GetNumPlayers__", &[]), int(2));
    assert_eq!(game(&mut w, "GetPlayerNumber__i", &[int(1)]), int(1));
    assert_eq!(game(&mut w, exists, &[int(1)]), Value::Bool(true));
    assert_eq!(game(&mut w, head, &[int(1)]), Value::Thing(Some(second)));
    let part = part(&mut w, second, "YellowHead");
    assert_eq!(
        w.get_member(MemberKind::RefPtr, &part, "PYellowHead", "PlayerNumber"),
        Ok(int(1))
    );

    // Teams: the record's field, -1 without a record; a set for nobody does nothing.
    let (set, get) = (
        "SetPlayerTeamNumber__Q18Game.EPlayerNumberi",
        "GetPlayerTeamNumber__Q18Game.EPlayerNumber",
    );
    assert_eq!(game(&mut w, get, &[int(1)]), int(0));
    assert_eq!(game(&mut w, set, &[int(1), int(1)]), Value::Void);
    assert_eq!(game(&mut w, get, &[int(1)]), int(1));
    assert_eq!(game(&mut w, get, &[int(0)]), int(0));
    assert_eq!(game(&mut w, set, &[int(3), int(1)]), Value::Void);
    assert_eq!(game(&mut w, get, &[int(3)]), int(-1));
    assert_eq!(w.player_records[1].team, 1);

    // A world that was not read from a level has no player at all.
    let mut empty = LevelWorld::default();
    assert_eq!(game(&mut empty, "GetNumPlayers__", &[]), int(0));
    assert_eq!(game(&mut empty, "GetPlayerNumber__i", &[int(0)]), int(-1));
    assert_eq!(game(&mut empty, exists, &[int(0)]), Value::Bool(false));
    assert_eq!(
        game(&mut empty, "GetPartyLeadersPlayerNumber__", &[]),
        int(-1)
    );
}

#[test]
fn stage4_score_and_multiplier_natives_keep_the_player_record() {
    let (int, float) = (Value::Int, Value::Float);
    let mut w = LevelWorld::from_things(&level());
    let p = |name: &str| format!("{name}__Q18Game.EPlayerNumber");
    let get = |w: &mut LevelWorld, name: &str, player: i32| game(w, &p(name), &[int(player)]);

    // The counter, and the multiplier it stands for: counter / 5 + 1.
    let set_counter = p("SetPlayerScoreMultiplierCounter") + "i";
    assert_eq!(get(&mut w, "GetPlayerScoreMultiplier", 0), int(1));
    game(&mut w, &set_counter, &[int(0), int(12)]);
    assert_eq!(get(&mut w, "GetPlayerScoreMultiplierCounter", 0), int(12));
    assert_eq!(get(&mut w, "GetPlayerScoreMultiplier", 0), int(3));
    // Increment: min(counter + add, cap), stored and returned.
    let increment = p("IncrementPlayerScoreMultiplierCounter") + "ii";
    assert_eq!(
        game(&mut w, &increment, &[int(0), int(2), int(20)]),
        int(14)
    );
    assert_eq!(
        game(&mut w, &increment, &[int(0), int(9), int(20)]),
        int(20)
    );
    assert_eq!(w.player_records[0].multiplier_counter, 20);

    // The timer counts down to 0 and stays there.
    let set_timer = p("SetPlayerScoreMultiplierTimer") + "i";
    game(&mut w, &set_timer, &[int(0), int(2)]);
    assert_eq!(get(&mut w, "GetPlayerScoreMultiplierTimer", 0), int(2));
    for left in [1, 0, 0] {
        assert_eq!(
            get(&mut w, "DecrementPlayerScoreMultiplierTimer", 0),
            int(left)
        );
    }
    assert_eq!(w.player_records[0].multiplier_timer, 0);

    // Visible and source are kept on the record; a null source is allowed.
    let visible = p("SetPlayerScoreMultiplierVisible") + "b";
    let source = p("SetPlayerScoreMultiplierSource") + "Q5Thing";
    game(&mut w, &visible, &[int(0), Value::Bool(true)]);
    game(&mut w, &source, &[int(0), Value::Thing(Some(1))]);
    assert!(w.player_records[0].multiplier_visible);
    assert_eq!(w.player_records[0].multiplier_source, Some(1));
    game(&mut w, &source, &[int(0), Value::Thing(None)]);
    assert_eq!(w.player_records[0].multiplier_source, None);

    // The score: negative becomes 0, the cap is 4,000,000.
    let set_score = p("SetPlayerScore") + "f";
    assert_eq!(get(&mut w, "GetPlayerScore", 0), float(0.));
    game(&mut w, &set_score, &[int(0), float(150.5)]);
    assert_eq!(get(&mut w, "GetPlayerScore", 0), float(150.5));
    game(&mut w, &set_score, &[int(0), float(-3.)]);
    assert_eq!(get(&mut w, "GetPlayerScore", 0), float(0.));
    game(&mut w, &set_score, &[int(0), float(5.0e6)]);
    assert_eq!(get(&mut w, "GetPlayerScore", 0), float(4.0e6));

    // A player without a record: the getters give 0, the setters do nothing.
    let before = w.player_records.clone();
    for name in [
        "GetPlayerScoreMultiplier",
        "GetPlayerScoreMultiplierCounter",
        "GetPlayerScoreMultiplierTimer",
        "DecrementPlayerScoreMultiplierTimer",
    ] {
        assert_eq!(get(&mut w, name, 2), int(0), "{name}");
    }
    assert_eq!(game(&mut w, &increment, &[int(2), int(1), int(9)]), int(0));
    assert_eq!(get(&mut w, "GetPlayerScore", 2), float(0.));
    game(&mut w, &set_counter, &[int(2), int(5)]);
    game(&mut w, &set_timer, &[int(2), int(5)]);
    game(&mut w, &set_score, &[int(2), float(5.)]);
    game(&mut w, &visible, &[int(2), Value::Bool(true)]);
    game(&mut w, &source, &[int(2), Value::Thing(Some(1))]);
    assert_eq!(w.player_records, before);
    // A wrong argument type is an error, not a silent default.
    assert!(native(&mut w, "Game", &set_score, &[int(0), int(1)]).is_err());
}

#[test]
fn stage4_global_settings_follow_the_priority_rule() {
    let int = Value::Int;
    let mut things = level();
    things[0].data.world = Some(WorldPart {
        global_settings_thing_uids: vec![0, 0, 40],
        global_settings_thing_priority: vec![0, 0, 1],
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&things);
    let get = |w: &mut LevelWorld, kind: i32| {
        let name = "GetGlobalSettingUID__Q24Game.EGlobalSettingsType";
        native(w, "Game", name, &[int(kind)])
    };
    let set = |w: &mut LevelWorld, kind: i32, uid: i32, priority: i32| {
        let name = "SetGlobalSettingUID__Q24Game.EGlobalSettingsTypeii";
        native(w, "Game", name, &[int(kind), int(uid), int(priority)])
    };
    // The stored values are the start, for all 8 slots.
    assert_eq!(get(&mut w, 2), Ok(int(40)));
    assert_eq!(get(&mut w, 7), Ok(int(0)));
    assert!(get(&mut w, 8).is_err());
    assert!(get(&mut w, -1).is_err());
    assert!(set(&mut w, 8, 1, 0).is_err());

    // A free slot (priority 0) goes to whoever asks, and again to the next at priority 0.
    set(&mut w, 0, 11, 0).unwrap();
    assert_eq!(get(&mut w, 0), Ok(int(11)));
    set(&mut w, 0, 12, 0).unwrap();
    assert_eq!(get(&mut w, 0), Ok(int(12)));
    // Priority 1 takes it and then holds it: nobody gets it after that, at any priority.
    set(&mut w, 0, 13, 1).unwrap();
    assert_eq!(get(&mut w, 0), Ok(int(13)));
    for (uid, priority) in [(14, 0), (15, 1), (16, 2), (0, 1)] {
        set(&mut w, 0, uid, priority).unwrap();
        assert_eq!(get(&mut w, 0), Ok(int(13)), "uid {uid} priority {priority}");
    }
    // The same for a slot the level file stores as held.
    set(&mut w, 2, 41, 1).unwrap();
    assert_eq!(get(&mut w, 2), Ok(int(40)));
    // A higher priority than 1 is not a lock; a lower request loses against it.
    set(&mut w, 1, 21, 2).unwrap();
    set(&mut w, 1, 22, 1).unwrap();
    assert_eq!(get(&mut w, 1), Ok(int(21)));
    set(&mut w, 1, 23, 3).unwrap();
    assert_eq!(get(&mut w, 1), Ok(int(23)));
    // UID 0 clears the slot and its priority, so anyone can take it again.
    set(&mut w, 1, 0, 3).unwrap();
    assert_eq!(get(&mut w, 1), Ok(int(0)));
    assert_eq!(w.pworld.global_settings_thing_priority[1], 0);
    set(&mut w, 1, 24, 0).unwrap();
    assert_eq!(get(&mut w, 1), Ok(int(24)));
    assert_eq!(
        w.pworld.global_settings_thing_uids,
        [13, 24, 40, 0, 0, 0, 0, 0]
    );
}

fn render_mesh(guid: u32) -> Option<RenderMesh> {
    Some(RenderMesh {
        mesh: Some(desc(guid)),
        bone_things: vec![],
        anim: None,
        anim_pos: 0.,
        anim_speed: 0.,
        anim_loop: false,
        editor_color: 0,
        cast_shadows: 0,
        visibility_flags: 0,
        poppet_render_scale: 1.,
    })
}

#[test]
fn stage4_slot_ids_game_state_and_the_lock_mesh() {
    use lbp_script_vm::natives::LOCK_MESH;
    // A class named SlotID with two s32 fields: the objects are made ahead.
    let mut slot = base();
    slot.class_name = "SlotID".into();
    slot.strings_a = ["SlotID", "Type", "SlotNumber", "Mode__"]
        .map(String::from)
        .to_vec();
    slot.type_refs = vec![ty(3, 0), ty(3, 0)];
    let mut vm = Vm::default();
    vm.load(slot, Some(BASE));

    // Thing 0 is the world, 1 the level link; under it 3 (another mesh), 4 below 3 (the
    // lock mesh) and 5 (the lock mesh again, later in the walk). 2 is beside the link.
    let mut things: Vec<ThingRecord> = level();
    things.extend((3..6).map(|index| ThingRecord {
        index,
        ..Default::default()
    }));
    things[3].parent = Some(1);
    things[4].parent = Some(3);
    things[5].parent = Some(1);
    things[2].data.render_mesh = render_mesh(LOCK_MESH);
    things[3].data.render_mesh = render_mesh(77);
    things[4].data.render_mesh = render_mesh(LOCK_MESH);
    things[5].data.render_mesh = render_mesh(LOCK_MESH);
    let mut host = Bindings::new(LevelWorld::from_things(&things));

    // Without the objects the natives say so instead of handing out null.
    assert!(native(&mut host.world, "Game", "GetLevelSlotID__", &[]).is_err());
    assert!(lbp_runtime::add_engine_objects(&mut vm, &mut host, 1000).is_empty());
    let w = &mut host.world;
    let level_slot = w.level_slot.expect("level slot");
    let primary = w.primary_level_slot.expect("primary slot");
    assert_ne!(level_slot, primary);
    for _ in 0..2 {
        assert_eq!(
            native(w, "Game", "GetLevelSlotID__", &[]),
            Ok(Value::Object(Some(level_slot)))
        );
    }
    assert_eq!(
        native(w, "Game", "GetPrimaryLevelSlotID__", &[]),
        Ok(Value::Object(Some(primary)))
    );
    for o in [level_slot, primary] {
        assert_eq!(vm.field(o, "Type"), Some(&Value::Int(0)));
        assert_eq!(vm.field(o, "SlotNumber"), Some(&Value::Int(0)));
    }

    // Game state in play: not paused whatever the level file stores, no tutorial, not
    // the demo build, music may start.
    w.pworld.is_paused = true;
    let flag = |w: &mut LevelWorld, name: &str| native(w, "Game", name, &[]);
    assert_eq!(flag(w, "IsVcrPaused__"), Ok(Value::Bool(false)));
    assert_eq!(flag(w, "IsPSNDemoBuild__"), Ok(Value::Bool(false)));
    assert_eq!(flag(w, "IsTutorialLevel__"), Ok(Value::Bool(false)));
    w.tutorial_level = true;
    assert_eq!(flag(w, "IsTutorialLevel__"), Ok(Value::Bool(true)));
    assert_eq!(
        native(w, "Audio", "IsSafeToTriggerMusic__i", &[Value::Int(0)]),
        Ok(Value::Bool(true))
    );

    // FindLockMesh: the first descendant with the lock mesh, a Thing before its
    // children; Things outside the subtree do not count.
    let find = |w: &mut LevelWorld, t: Option<u32>| {
        native(w, "TriggerLevelLink", "FindLockMesh__", &[Value::Thing(t)])
    };
    assert_eq!(find(w, Some(1)), Ok(Value::Thing(Some(4))));
    assert_eq!(find(w, Some(3)), Ok(Value::Thing(Some(4))));
    assert_eq!(find(w, Some(4)), Ok(Value::Thing(None)));
    assert_eq!(find(w, Some(5)), Ok(Value::Thing(None)));
    assert_eq!(find(w, Some(0)), Ok(Value::Thing(Some(4))));
    w.things[4].mesh_guid = Some(77);
    assert_eq!(find(w, Some(1)), Ok(Value::Thing(Some(5))));
    assert_eq!(find(w, Some(0)), Ok(Value::Thing(Some(5))));
    w.things[5].mesh_guid = None;
    assert_eq!(find(w, Some(1)), Ok(Value::Thing(None)));
    assert_eq!(find(w, Some(0)), Ok(Value::Thing(Some(2))));
    assert!(find(w, None).is_err());
}

#[test]
fn stage4_recorded_calls_are_counted_and_change_nothing_else() {
    use lbp_script_vm::natives::{bound, RECORDED_ONLY};
    let mut w = LevelWorld::from_things(&level());
    let me = Value::Thing(Some(1));
    let null = Value::Thing(None);
    let calls: [(&str, &str, Vec<Value>); 9] = [
        ("World", "SetPieChartMultiplier__i", vec![Value::Int(3)]),
        ("Thing", "UpdateScale__", vec![me.clone()]),
        (
            "Thing",
            "SetRenderMeshEditorTexture__Q5ThingQ8Resource",
            vec![me.clone(), Value::Ref(900)],
        ),
        (
            "Thing",
            "SetRenderMeshEditorTexture__Q5ThingQ8Resource",
            vec![null.clone(), Value::Ref(900)],
        ),
        (
            "Thing",
            "RemoveFromAnimatingDivergentList__",
            vec![me.clone()],
        ),
        (
            "Trigger_Speech_Bubble",
            "NotifyNpcsOfMouthEnd__",
            vec![me.clone()],
        ),
        ("NoJoinMarker", "SetGameNoJoin__", vec![me.clone()]),
        ("Game", "GotoScoreboard__b", vec![Value::Bool(true)]),
        ("Game", "GotoScoreboard__b", vec![Value::Bool(false)]),
    ];
    for (class, name, args) in &calls {
        assert_eq!(native(&mut w, class, name, args), Ok(Value::Void), "{name}");
        assert!(RECORDED_ONLY.contains(&(*class, *name)), "{name}");
    }
    let counted: Vec<_> = w.engine_calls.iter().map(|(k, n)| (*k, *n)).collect();
    assert_eq!(
        counted,
        [
            ("GotoScoreboard", 2),
            ("NotifyNpcsOfMouthEnd", 1),
            ("RemoveFromAnimatingDivergentList", 1),
            ("SetGameNoJoin", 1),
            ("SetPieChartMultiplier", 1),
            ("SetRenderMeshEditorTexture", 2),
            ("UpdateScale", 1),
        ]
    );
    // The texture is kept on its Thing; the scoreboard request with its flag.
    assert_eq!(w.things[1].render_mesh_texture, Some(Value::Ref(900)));
    assert_eq!(w.scoreboard_requests, [true, false]);
    assert!(w.things[2].render_mesh_texture.is_none());
    // A wrong argument is still an error.
    let bad = [Value::Int(1), Value::Ref(900)];
    let texture = "SetRenderMeshEditorTexture__Q5ThingQ8Resource";
    assert!(native(&mut w, "Thing", texture, &bad).is_err());
    // Hiding a mesh and colouring it are no longer recorded only (part 19).
    for name in [
        "HideRenderMesh__Q5Thing",
        "SetRenderMeshEditorColour__Q5Thingr",
    ] {
        assert!(!RECORDED_ONLY.contains(&("Thing", name)), "{name}");
        assert!(bound().any(|b| b == ("Thing", name)), "{name}");
    }
    // Every recorded-only native is in the table.
    for entry in RECORDED_ONLY {
        assert!(bound().any(|b| b == entry), "{entry:?}");
    }
}

#[test]
fn d263_pad_queries_follow_the_players_pad_and_the_setter_feeds_it() {
    let mut w = LevelWorld::from_things(&level());
    let head = w.add_player([0.; 3], (0., 0.));
    let ask = |w: &mut LevelWorld, name: &str, t: Option<u32>, buttons: i32| {
        let args = [Value::Thing(t), Value::Int(1), Value::Int(buttons)];
        native(w, "Pad", name, &args).unwrap()
    };
    let (just, held, released) = (
        "IsJustPressed__Q5Thingii",
        "IsPressed__Q5Thingii",
        "IsJustReleased__Q5Thingii",
    );
    let (yes, no) = (Value::Bool(true), Value::Bool(false));
    // A record without a pad: every query is false.
    assert_eq!(w.player_records[0].pad, None);
    for name in [just, held, released] {
        assert_eq!(ask(&mut w, name, Some(head), 4), no, "{name}");
    }
    // The viewer's side: buttons down this frame, on the pad of player 0.
    assert!(w.set_pad_buttons(0, 4));
    assert!(!w.set_pad_buttons(3, 4), "no record for player 3");
    assert_eq!(ask(&mut w, just, Some(head), 4), yes);
    assert_eq!(ask(&mut w, held, Some(head), 4), yes);
    assert_eq!(ask(&mut w, released, Some(head), 4), no);
    // Every bit of the mask must be down; another mode does not own the pad.
    assert_eq!(ask(&mut w, just, Some(head), 6), no);
    let other_mode = [Value::Thing(Some(head)), Value::Int(2), Value::Int(4)];
    assert_eq!(native(&mut w, "Pad", held, &other_mode), Ok(no.clone()));
    // A Thing that is nobody's Sackboy, and a null Thing, lead to no pad.
    assert_eq!(ask(&mut w, held, Some(1), 4), no);
    assert_eq!(ask(&mut w, held, None, 4), no);
    // The next frame, still down: held, no longer an edge.
    w.end_pad_frame();
    assert_eq!(ask(&mut w, just, Some(head), 4), no);
    assert_eq!(ask(&mut w, held, Some(head), 4), yes);
    // Let go: the release edge for one frame.
    w.set_pad_buttons(0, 0);
    assert_eq!(ask(&mut w, released, Some(head), 4), yes);
    assert_eq!(ask(&mut w, held, Some(head), 4), no);
    w.end_pad_frame();
    assert_eq!(ask(&mut w, released, Some(head), 4), no);
    // A second player has his own pad.
    let second = w.add_player([0.; 3], (0., 0.));
    assert_eq!(ask(&mut w, held, Some(second), 4), no);
    assert!(w.set_pad_buttons(1, 4));
    assert_eq!(ask(&mut w, just, Some(second), 4), yes);
    assert_eq!(ask(&mut w, held, Some(head), 4), no);
    // A level tick ends the pad frame.
    let mut vm = Vm::default();
    let mut host = Bindings::new(w);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    scripts.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(ask(&mut host.world, just, Some(second), 4), no);
    assert_eq!(ask(&mut host.world, held, Some(second), 4), yes);
}

/// A binary translation table: (key, text) rows, keys ascending.
fn translation_table(
    rows: &[(u32, &str)],
) -> std::sync::Arc<lbp_resource::translation::TranslationTable> {
    use lbp_resource::translation::{parse_translation_table, TranslationLimits};
    let mut out = (rows.len() as u32).to_be_bytes().to_vec();
    let mut blob = Vec::new();
    for (key, text) in rows {
        out.extend_from_slice(&key.to_be_bytes());
        out.extend_from_slice(&(blob.len() as u32).to_be_bytes());
        for unit in std::iter::once(0xfeff).chain(text.encode_utf16()) {
            blob.extend_from_slice(&unit.to_be_bytes());
        }
    }
    out.extend_from_slice(&blob);
    std::sync::Arc::new(parse_translation_table(&out, TranslationLimits::default()).unwrap())
}

#[test]
fn d263_lams_keys_are_looked_up_in_the_loaded_tables() {
    use lbp_resource::translation::lams_key;
    let mut w = LevelWorld::from_things(&level());
    let key = |tag: &str| Value::Int(lams_key(tag) as i32);
    let in_lams = |w: &mut LevelWorld, k: Value| native(w, "LAMS", "IsInLams__i", &[k]);
    let translate = |w: &mut LevelWorld, k: Value| native(w, "LAMS", "Translate__i", &[k]);
    // The key a script makes is the key the tables are searched with.
    let made = native(
        &mut w,
        "LAMS",
        "MakeLamsKeyID__Q6String",
        &[Value::Str("HELLO".into())],
    );
    assert_eq!(made, Ok(key("HELLO")));
    // No table loaded: nothing is in LAMS, and there is no text to give.
    assert_eq!(in_lams(&mut w, key("HELLO")), Ok(Value::Bool(false)));
    assert!(translate(&mut w, key("HELLO")).is_err());

    let mut rows = [(lams_key("HELLO"), "Hello!"), (lams_key("BYE"), "Bye")];
    rows.sort();
    let base = translation_table(&rows);
    let patch = translation_table(&[(lams_key("HELLO"), "Hello again")]);
    w.translations = vec![base.clone()];
    assert_eq!(in_lams(&mut w, key("HELLO")), Ok(Value::Bool(true)));
    assert_eq!(in_lams(&mut w, key("BYE")), Ok(Value::Bool(true)));
    assert_eq!(in_lams(&mut w, key("NOPE")), Ok(Value::Bool(false)));
    assert_eq!(
        translate(&mut w, key("HELLO")),
        Ok(Value::Str("Hello!".into()))
    );
    // A key in no table has no text: an error, not an empty string.
    assert!(translate(&mut w, key("NOPE")).is_err());
    // The first table that has the key answers.
    w.translations = vec![patch, base];
    assert_eq!(
        translate(&mut w, key("HELLO")),
        Ok(Value::Str("Hello again".into()))
    );
    assert_eq!(translate(&mut w, key("BYE")), Ok(Value::Str("Bye".into())));
}

#[test]
fn d263_global_settings_blend_from_start_to_target() {
    use lbp_runtime::Blend;
    let (float, flag) = (Value::Float, Value::Bool);
    let mut things = level();
    things[0].data.world = Some(WorldPart {
        water_level: 5.,
        water_wave_magnitude: 0.5,
        gravity: -2.5,
        water_tint: 0.25,
        water_murkiness: 0.75,
        water_bits: 0.125,
        gs_from: [0.; 5],
        gs_target: [1., 2., 3., 4., 5.],
        gs_blend: 0.5,
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&things);
    let get = |w: &mut LevelWorld, name: &str| {
        native(w, "Game", &format!("GetGlobalSettings{name}__"), &[])
    };
    let set = |w: &mut LevelWorld, name: &str, value: f32, blend: bool| {
        let name = format!("SetGlobalSettings{name}To__fb");
        native(w, "Game", &name, &[float(value), flag(blend)]).unwrap()
    };
    let factor = |w: &mut LevelWorld, kind: i32, t: f32| {
        let name = "SetGlobalSettingsBlendFactor__Q24Game.EGlobalSettingsTypef";
        native(w, "Game", name, &[Value::Int(kind), float(t)])
    };
    // The start: each setting at rest on its stored value.
    for (name, stored) in [
        ("WaterLevel", 5.),
        ("WaterWaveMagnitude", 0.5),
        ("WaterTint", 0.25),
        ("WaterMurkiness", 0.75),
        ("WaterBits", 0.125),
    ] {
        assert_eq!(get(&mut w, name), Ok(float(stored)), "{name}");
    }
    assert_eq!(w.pworld.gravity, Blend::at(-2.5));

    // With blend the value stays until the script raises the factor (type 0 = level).
    set(&mut w, "WaterLevel", 9., true);
    assert_eq!(get(&mut w, "WaterLevel"), Ok(float(5.)));
    factor(&mut w, 0, 0.25).unwrap();
    assert_eq!(get(&mut w, "WaterLevel"), Ok(float(6.)));
    // A new target half way: the start is the value in force, the factor 0 again.
    set(&mut w, "WaterLevel", 2., true);
    let expected = Blend {
        from: 6.,
        to: 2.,
        t: 0.,
    };
    assert_eq!(w.pworld.water, expected);
    factor(&mut w, 0, 1.).unwrap();
    assert_eq!(get(&mut w, "WaterLevel"), Ok(float(2.)));
    // Without blend the setting jumps.
    set(&mut w, "WaterLevel", 7., false);
    assert_eq!(w.pworld.water, Blend::at(7.));
    // A level that is not above 0.0001 reads as no water, 0.0.
    set(&mut w, "WaterLevel", 0.0001, false);
    assert_eq!(get(&mut w, "WaterLevel"), Ok(float(0.)));
    set(&mut w, "WaterLevel", 0.001, false);
    assert_eq!(get(&mut w, "WaterLevel"), Ok(float(0.001)));
    set(&mut w, "WaterLevel", -3., false);
    assert_eq!(get(&mut w, "WaterLevel"), Ok(float(0.)));

    // Each type has its own factor: 1 gravity, 2 wave, 4 murkiness, 6 bits.
    set(&mut w, "Gravity", 1.5, true);
    set(&mut w, "WaterWaveMagnitude", 1.5, true);
    set(&mut w, "WaterMurkiness", 0.25, true);
    set(&mut w, "WaterBits", 1.125, true);
    for kind in [1, 2, 4, 6] {
        factor(&mut w, kind, 0.5).unwrap();
    }
    assert_eq!(w.pworld.gravity.value(), -0.5);
    assert_eq!(get(&mut w, "WaterWaveMagnitude"), Ok(float(1.)));
    assert_eq!(get(&mut w, "WaterMurkiness"), Ok(float(0.5)));
    assert_eq!(get(&mut w, "WaterBits"), Ok(float(0.625)));
    // The tint is one value; its factor (type 3) is kept and does not change it.
    set(&mut w, "WaterTint", 0.5, true);
    factor(&mut w, 3, 0.25).unwrap();
    assert_eq!(get(&mut w, "WaterTint"), Ok(float(0.5)));
    assert_eq!(w.pworld.water_tint_blend, 0.25);
    factor(&mut w, 7, 0.75).unwrap();
    assert_eq!(w.pworld.audio_blend, 0.75);
    assert!(factor(&mut w, 8, 0.).is_err());
    assert!(factor(&mut w, -1, 0.).is_err());

    // Lighting (type 5). Stored: from 0, target 1..5, factor 0.5.
    assert_eq!(w.pworld.lighting(), [0.5, 1., 1.5, 2., 2.5]);
    let lighting = |w: &mut LevelWorld, values: [f32; 5], blend: bool| {
        let mut args: Vec<Value> = values.iter().map(|v| float(*v)).collect();
        args.push(flag(blend));
        native(w, "Game", "SetGlobalSettingsTo__fffffb", &args).unwrap()
    };
    // Arguments: Lighting, Darkness, Fog, FogTint, ColourCorrection; kept in stored
    // order: lighting, colour correction, fog, fog tint, darkness.
    lighting(&mut w, [10., 50., 30., 40., 20.], true);
    assert_eq!(w.pworld.lighting_from, [0.5, 1., 1.5, 2., 2.5]);
    assert_eq!(w.pworld.lighting_to, [10., 20., 30., 40., 50.]);
    assert_eq!(w.pworld.lighting(), [0.5, 1., 1.5, 2., 2.5]);
    factor(&mut w, 5, 1.).unwrap();
    assert_eq!(w.pworld.lighting(), [10., 20., 30., 40., 50.]);
    lighting(&mut w, [1., 5., 3., 4., 2.], false);
    assert_eq!(w.pworld.lighting_from, [1., 2., 3., 4., 5.]);
    assert_eq!(
        (w.pworld.lighting(), w.pworld.lighting_blend),
        ([1., 2., 3., 4., 5.], 1.)
    );

    let drain = "SetGlobalSettingsWaterDrainSoundsEnabled__b";
    assert!(!w.pworld.water_drain_sounds);
    native(&mut w, "Game", drain, &[flag(true)]).unwrap();
    assert!(w.pworld.water_drain_sounds);
    // A wrong argument type is an error.
    let bad = [Value::Int(1), flag(false)];
    assert!(native(&mut w, "Game", "SetGlobalSettingsGravityTo__fb", &bad).is_err());

    // Without a PWorld the setters do nothing and the getters give 0.0.
    let mut bare = LevelWorld::default();
    let before = bare.pworld.clone();
    set(&mut bare, "WaterLevel", 3., false);
    lighting(&mut bare, [1.; 5], false);
    factor(&mut bare, 0, 0.5).unwrap();
    native(&mut bare, "Game", drain, &[flag(true)]).unwrap();
    assert_eq!(bare.pworld, before);
    assert_eq!(get(&mut bare, "WaterBits"), Ok(float(0.)));
    assert_eq!(get(&mut bare, "WaterLevel"), Ok(float(0.)));
}

#[test]
fn d263_game_state_queries_and_the_scoring_slot_test() {
    let int = Value::Int;
    let mut slot = base();
    slot.class_name = "SlotID".into();
    slot.strings_a = ["SlotID", "Type", "SlotNumber", "Mode__"]
        .map(String::from)
        .to_vec();
    slot.type_refs = vec![ty(3, 0), ty(3, 0)];
    let mut vm = Vm::default();
    vm.load(slot, Some(BASE));

    let mut things = level();
    things[0].data.world = Some(WorldPart {
        frame: 40,
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&things);
    // The level's slot is known to the caller here; the primary slot stays empty.
    w.level_slot_id = (1, 7);
    let mut host = Bindings::new(w);
    assert!(lbp_runtime::add_engine_objects(&mut vm, &mut host, 1000).is_empty());
    let w = &mut host.world;
    let (level_slot, primary) = (w.level_slot.unwrap(), w.primary_level_slot.unwrap());
    assert_eq!(vm.field(level_slot, "Type"), Some(&int(1)));
    assert_eq!(vm.field(level_slot, "SlotNumber"), Some(&int(7)));
    assert_eq!(vm.field(primary, "Type"), Some(&int(0)));

    // The frame is the PWorld's, and steps with it.
    let game = |w: &mut LevelWorld, name: &str| native(w, "Game", name, &[]);
    assert_eq!(game(w, "GetGameFrame__"), Ok(int(40)));
    w.advance_frame();
    assert_eq!(game(w, "GetGameFrame__"), Ok(int(41)));
    assert_eq!(
        game(&mut LevelWorld::default(), "GetGameFrame__"),
        Ok(int(0))
    );

    // Sackboys in the level, and the score data type: players - 1, or 6 in versus.
    let me = Value::Thing(Some(1));
    let data_type = |w: &mut LevelWorld| {
        let me = [Value::Thing(Some(1))];
        native(w, "Score_Board", "GetGameScoreDataType__", &me)
    };
    assert_eq!(game(w, "GetNumYellowHeadAvatars__"), Ok(int(0)));
    assert_eq!(data_type(w), Ok(int(0)));
    w.add_player([0.; 3], (0., 0.));
    w.add_player([0.; 3], (0., 0.));
    assert_eq!(game(w, "GetNumYellowHeadAvatars__"), Ok(int(2)));
    assert_eq!(data_type(w), Ok(int(1)));
    w.pworld.game_mode = 1;
    assert_eq!(data_type(w), Ok(int(6)));

    // Message strings are filtered unless the slot's type is 2.
    let filter = |w: &mut LevelWorld| {
        let name = "Messaging_ShouldFilterMessageStrings__";
        native(w, "GameMessagingManager", name, &[])
    };
    assert_eq!(filter(w), Ok(Value::Bool(true)));

    // Valid for scoring: type 0, 1 or 8, and not the empty slot (0, 0).
    let valid = |w: &mut LevelWorld, slot: Value| {
        let name = "IsSlotIDValidForScoring__Q6SlotIDi";
        native(w, "Score_Board", name, &[me.clone(), slot, int(0)])
    };
    let object = |o| Value::Object(Some(o));
    assert_eq!(valid(w, object(level_slot)), Ok(Value::Bool(true)));
    assert_eq!(valid(w, object(primary)), Ok(Value::Bool(false)));
    for (kind, number, expected) in [(0, 5, true), (8, 0, true), (2, 7, false), (3, 1, false)] {
        w.level_slot_id = (kind, number);
        assert_eq!(
            valid(w, object(level_slot)),
            Ok(Value::Bool(expected)),
            "slot ({kind}, {number})"
        );
    }
    w.level_slot_id = (2, 7);
    assert_eq!(filter(w), Ok(Value::Bool(false)));
    // The group of a slot: the empty slot is in no group, and the answer is a SlotID
    // (0, 0) of its own, never null. Any other slot needs the slot database: an error.
    let group =
        |w: &mut LevelWorld, slot: Value| native(w, "Slot", "GetSlotGroup__Q6SlotID", &[slot]);
    let none = w.no_group_slot.unwrap();
    assert!(none != level_slot && none != primary);
    assert_eq!(group(w, object(primary)), Ok(object(none)));
    assert_eq!(group(w, object(none)), Ok(object(none)));
    assert_eq!(vm.field(none, "Type"), Some(&int(0)));
    assert_eq!(vm.field(none, "SlotNumber"), Some(&int(0)));
    assert!(group(w, object(level_slot)).is_err());
    assert!(group(w, Value::Object(None)).is_err());
    assert_eq!(valid(w, object(none)), Ok(Value::Bool(false)));
    // A SlotID the world did not make cannot be read: an error, not a guess.
    assert!(valid(w, Value::Object(None)).is_err());

    // Constants for one local player (INFERRED) and the two the executable returns.
    assert_eq!(
        native(
            w,
            "OffScreenIndicator",
            "GetPlayerIndicatorActivityType__i",
            &[int(0)]
        ),
        Ok(int(0))
    );
    assert_eq!(
        native(
            w,
            "Game",
            "GetNumKeyNames__Q25Gooey.EGooeyNetworkAction",
            &[int(80)]
        ),
        Ok(int(0))
    );
    assert_eq!(
        native(
            w,
            "Game",
            "GetKeyIndex__Q25Gooey.EGooeyNetworkActionQ6String",
            &[int(80), Value::Str("Larry".into())]
        ),
        Ok(int(-1))
    );
    assert_eq!(
        native(w, "ModalOverlay", "IsPartyLeaderOverlayActive__", &[]),
        Ok(Value::Bool(false))
    );
    assert_eq!(game(w, "GetNumKnownItemsCollectedAtStart__"), Ok(int(0)));
    assert_eq!(game(w, "IsTarsierDLCEnabled__"), Ok(Value::Bool(true)));
    assert_eq!(native(w, "Poppet", "GetTextEntryType__", &[]), Ok(int(2)));
    assert_eq!(
        native(
            w,
            "Resource",
            "LoadAnimatedTextureByFilename__g",
            &[int(4242)]
        ),
        Ok(Value::Ref(4242))
    );
    // Every state query in the list has an answer of its declared type.
    for (name, declared) in lbp_script_vm::natives::STATE_QUERIES {
        let got = w.state_query(name);
        let ok = match declared {
            "bool" => matches!(got, Ok(Value::Bool(_))),
            _ => matches!(got, Ok(Value::Int(_))),
        };
        assert!(ok, "{name}: {got:?}");
    }
}

#[test]
fn d263_recorded_calls_keep_their_arguments_for_a_later_stage() {
    use lbp_runtime::{EngineRequest, KEPT_REQUESTS, MAX_REQUESTS};
    use lbp_script_vm::natives::{MESSAGING, RECORDED_ONLY};
    let mut things = level();
    things[0].data.world = Some(WorldPart {
        frame: 9,
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&things);
    let me = Value::Thing(Some(1));
    let (text, flag, int) = (|t: &str| Value::Str(t.into()), Value::Bool, Value::Int);
    let gooey = Value::Object(None);
    let message = vec![
        text("Hi"),
        text("Larry"),
        me.clone(),
        flag(true),
        int(2),
        flag(false),
    ];
    let mut audio = vec![int(3)];
    audio.extend((0..11).map(|i| Value::Float(i as f32)));
    let manager = "GameMessagingManager";
    let calls: Vec<(&str, &str, Vec<Value>)> = vec![
        (
            manager,
            "Messaging_NewTriggerMessage__Q6StringQ6StringQ5Thingbib",
            message.clone(),
        ),
        (
            manager,
            "Messaging_NewTriggerMessageNoPrompt__Q6StringQ6StringQ5Thingbib",
            message.clone(),
        ),
        (
            manager,
            "Messaging_NewTriggerMessageSubtitles__Q6StringQ5Thingbb",
            vec![text("Sub"), me.clone(), flag(false), flag(true)],
        ),
        (
            manager,
            "Messaging_NewLevelLinkPrompt__Q5Thing",
            vec![me.clone()],
        ),
        (
            manager,
            "Messaging_NewWormHolePrompt__Q5Thing",
            vec![Value::Thing(None)],
        ),
        (
            manager,
            "Messaging_NewGameEvent__iiQ5Thingb",
            vec![int(4), int(3), me.clone(), flag(false)],
        ),
        (
            "HUD",
            "RegisterOverlayCallback__Q5ThingQ6String",
            vec![me.clone(), text("RenderHUD")],
        ),
        (
            "HUD",
            "PreloadGlyphs__Q11Fonts.EFontQ6String",
            vec![int(6), text("Hi")],
        ),
        ("Game", "TrackSpeechKey__Q6String", vec![text("KEY")]),
        (
            "Trigger_Speech_Bubble",
            "NotifyNpcsOfMouthStart__i",
            vec![me.clone(), int(-1)],
        ),
        ("Thing", "AddToAnimatingDivergentList__", vec![me.clone()]),
        ("Thing", "SetAnimLoop__b", vec![me.clone(), flag(true)]),
        (
            "Gooey",
            "SetLastNavSelectionMode__b",
            vec![gooey.clone(), flag(true)],
        ),
        (
            "Gooey",
            "PerFrameUpdate__Q18Game.EPlayerNumberi",
            vec![gooey.clone(), int(0), int(1)],
        ),
        (
            "Gooey",
            "SetStylesheetScalingFactor__f",
            vec![gooey, Value::Float(1.5)],
        ),
        (
            "Game",
            "SetGlobalSettingsAudioTo__ifffffffffff",
            audio.clone(),
        ),
        (
            manager,
            "Messaging_NewTriggerMessageCaption__Q6StringQ6StringQ5Thingbib",
            message.clone(),
        ),
        (
            manager,
            "Messaging_NewTriggerMessageCaptionNoPrompt__Q6StringQ6StringQ5Thingbib",
            message.clone(),
        ),
        (
            manager,
            "Messaging_NewTriggerMessageThink__Q6StringQ6StringQ5Thingbib",
            message.clone(),
        ),
        (
            manager,
            "Messaging_NewTriggerMessageThinkNoPrompt__Q6StringQ6StringQ5Thingbib",
            message.clone(),
        ),
    ];
    for (class, name, args) in &calls {
        assert_eq!(native(&mut w, class, name, args), Ok(Value::Void), "{name}");
        // The messaging natives run the message list; the others are recorded only.
        let messaging = *class == manager && MESSAGING.contains(name);
        assert_ne!(
            messaging,
            RECORDED_ONLY.contains(&(*class, *name)),
            "{name}"
        );
    }
    // With the 11 of stage 4 that are still recorded only, the 2 of D270 (the Pod) and
    // the 1 of D271 (a video stop), these are all the recorded-only and messaging natives.
    assert_eq!(
        calls.len() + 11 + 2 + 1,
        RECORDED_ONLY.len() + MESSAGING.len()
    );
    // Every call is counted once by name.
    assert_eq!(w.engine_calls.len(), calls.len());
    assert!(w.engine_calls.values().all(|n| *n == 1));
    // Fourteen of them are kept with their arguments, in call order, with the frame.
    let kept: Vec<&str> = w.requests.iter().map(|r| r.name).collect();
    assert_eq!(kept, KEPT_REQUESTS);
    assert_eq!(
        w.requests[0],
        EngineRequest {
            name: "Messaging_NewTriggerMessage",
            thing: Some(1),
            args: message,
            frame: 9,
        }
    );
    assert_eq!(w.requests[2].args[0], text("Sub"));
    assert_eq!(w.requests[4].thing, None, "a prompt for a null Thing");
    assert_eq!(w.requests[5].args[..2], [int(4), int(3)]);
    assert_eq!(w.requests[6].args[1], text("RenderHUD"));
    assert_eq!(w.requests[7].args, [text("KEY")]);
    assert_eq!(w.requests[8].thing, Some(1));
    assert_eq!(w.requests[8].args, [int(-1)]);
    assert_eq!(w.requests[9].args, audio);
    // The loop flag is kept on its Thing.
    assert_eq!(w.things[1].anim_loop, Some(true));
    assert_eq!(w.things[2].anim_loop, None);
    // A wrong argument is still an error, and nothing is recorded for it.
    let bad = [me, int(1)];
    assert!(native(&mut w, "Thing", "SetAnimLoop__b", &bad).is_err());
    assert_eq!(w.engine_calls["SetAnimLoop"], 1);
    // The list of kept requests is bounded; the rest are counted.
    let key = [text("K")];
    for _ in 0..MAX_REQUESTS {
        native(&mut w, "Game", "TrackSpeechKey__Q6String", &key).unwrap();
    }
    assert_eq!(w.requests.len(), MAX_REQUESTS);
    assert_eq!(w.requests_dropped, KEPT_REQUESTS.len() as u64);
}

#[test]
fn d263_a_script_cast_tests_the_things_class_chain() {
    let mut vm = Vm::default();
    vm.load(base(), Some(BASE));
    vm.load(lamp(), Some(GUID));
    let mut w = LevelWorld::from_things(&level());
    // Before the classes are resolved nothing is known and every cast passes.
    assert!(w.thing_is(1, "Other") && w.thing_is(2, "Lamp"));
    w.add_class_defaults(&vm);
    // Thing 1 carries a Lamp, which derives from Base; Thing 2 has no script.
    assert_eq!(w.things[1].script_classes, ["Lamp", "Base"]);
    assert!(w.thing_is(1, "Lamp") && w.thing_is(1, "Base"));
    assert!(!w.thing_is(1, "Other"));
    assert!(!w.thing_is(2, "Lamp") && !w.thing_is(99, "Lamp"));
}

#[test]
fn d264_sensors_read_the_level_parts() {
    use lbp_level::parts::SwitchKey;
    let mut t: Vec<ThingRecord> = (0..5)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    let at = |x: f32| {
        let mut m = identity();
        m[12] = x;
        pos(m)
    };
    let output = || Some(SwitchOutput::default());
    // Thing 0: a tag sensor for (2, "Door"). Things 1 and 2: tags, one with another name.
    // Thing 3: a player sensor. Thing 4: nothing.
    t[0].data.pos = at(0.);
    t[0].data.switch = Some(Switch {
        switch_type: 4,
        radius: 100.,
        angle_range: 180.,
        colour_index: 2,
        name: "Door".into(),
        bullets_required: 1,
        outputs: vec![output()],
        ..Default::default()
    });
    t[1].data.pos = at(30.);
    t[1].data.switch_key = Some(SwitchKey {
        colour_index: 2,
        name: "Gate".into(),
    });
    t[2].data.pos = at(50.);
    t[2].data.switch_key = Some(SwitchKey {
        colour_index: 2,
        name: "Door".into(),
    });
    t[3].data.pos = at(1000.);
    t[3].data.switch = Some(Switch {
        switch_type: 3,
        radius: 40.,
        min_radius: 10.,
        angle_range: 180.,
        bullets_required: 1,
        require_all: true,
        player_mode: 1,
        outputs: vec![output()],
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&t);
    // Equal names share an index; the sensor's key is the second tag's.
    assert_eq!(w.tag_names, ["Door", "Gate", ""]);
    let key = w.things[0].switch.as_ref().unwrap().key;
    assert_eq!(w.things[2].switch_key, Some(key));
    assert_ne!(w.things[1].switch_key, Some(key));
    let s = w.things[3].switch.as_ref().unwrap();
    assert_eq!((s.radius, s.min_radius, s.require_all), (40., 10., true));
    assert_eq!((s.player_mode, s.angle_range), (1, 180.));
    let out = |w: &LevelWorld, thing: usize| {
        let s = w.things[thing].switch.as_ref().unwrap();
        s.outputs[0].as_ref().unwrap().signal
    };
    // The tag sensor finds its tag 50 away; the player sensor has nobody.
    w.update_switches();
    assert_eq!(w.tags[&key].len(), 1);
    let s = out(&w, 0);
    assert!((s.activation - 0.5 / 0.75).abs() < 1e-5 && s.ternary == 1);
    assert_eq!(out(&w, 3).ternary, 0);
    // A player between the two edges of the player sensor turns it on.
    let p = w.add_player([1025., 0., 0.], (-1., 1.));
    w.update_switches();
    let s = out(&w, 3);
    assert_eq!((s.activation, s.ternary, s.player), (0.5, 1, 0));
    w.move_player(p, [1005., 0., 0.], (-1., 1.));
    w.update_switches();
    assert_eq!(out(&w, 3).ternary, 0);
}

/// The six arguments of a bubble native.
fn bubble(
    text: &str,
    actor: Option<&str>,
    thing: Option<u32>,
    flags: [bool; 2],
    pos: i32,
) -> Vec<Value> {
    vec![
        Value::Str(text.into()),
        actor.map_or(Value::Object(None), |a| Value::Str(a.into())),
        Value::Thing(thing),
        Value::Bool(flags[0]),
        Value::Int(pos),
        Value::Bool(flags[1]),
    ]
}

#[test]
fn d265_the_messaging_natives_open_and_close_the_worlds_messages() {
    use lbp_resource::translation::lams_key;
    use lbp_runtime::{BubbleStyle, MessageKind, CLOSE_PROMPT_KEY};
    let mut things = chip_level();
    things[0].data.world = Some(WorldPart {
        frame: 40,
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&things);
    let manager = "GameMessagingManager";
    let six = "__Q6StringQ6StringQ5Thingbib";
    let say = |w: &mut LevelWorld, form: &str, args: &[Value]| {
        let name = format!("Messaging_NewTriggerMessage{form}{six}");
        assert_eq!(native(w, manager, &name, args), Ok(Value::Void), "{name}");
    };
    let event = |w: &mut LevelWorld, m: i32, e: i32, t: Option<u32>| {
        let args = [
            Value::Int(m),
            Value::Int(e),
            Value::Thing(t),
            Value::Bool(false),
        ];
        native(w, manager, "Messaging_NewGameEvent__iiQ5Thingb", &args).unwrap();
    };
    w.translations = vec![translation_table(&[(
        lams_key("ACTOR_TAG"),
        "Made-up Name",
    )])];

    // A bubble with a prompt from Thing 8, which has its own position. The actor name is
    // a tag of the loaded table (the level slot's type is 0).
    say(
        &mut w,
        "",
        &bubble("Hello there", Some("ACTOR_TAG"), Some(8), [true, false], 7),
    );
    let m = w.messages.open[0].clone();
    assert_eq!(
        (m.kind, m.style),
        (MessageKind::Bubble, BubbleStyle::Speech)
    );
    assert_eq!((m.message_type(), m.id), (1, 1));
    assert_eq!(
        (m.text.as_str(), m.actor.as_deref()),
        ("Hello there", Some("Made-up Name"))
    );
    assert_eq!((m.thing, m.anchor, m.bubble_pos), (Some(8), Some(8), 7));
    assert!(m.triggered_once && !m.prevent_close && m.has_close_prompt());
    assert_eq!((m.frame, m.age(45)), (40, 5));
    assert_eq!(w.message_position(&m), Some([1., 2., 3.]));
    assert_eq!(CLOSE_PROMPT_KEY, 0xfc6e3b66);
    // The same Thing asks again: dropped. The request is still kept.
    say(
        &mut w,
        "",
        &bubble("Again", None, Some(8), [false, false], 0),
    );
    assert_eq!(w.messages.open.len(), 1);
    assert_eq!((w.requests.len(), w.messages.totals().duplicate), (2, 1));

    // A bubble without a prompt from Thing 3 on a chip: placed by the chip, Thing 1. A
    // tag in no table and a null name give no actor; prevent close is kept.
    say(
        &mut w,
        "NoPrompt",
        &bubble("On a chip", Some("NO_SUCH_TAG"), Some(3), [false, true], 0),
    );
    let m = w.messages.open[1].clone();
    assert_eq!((m.kind, m.message_type()), (MessageKind::BubbleNoPrompt, 2));
    assert_eq!(
        (m.thing, m.anchor, m.actor.as_deref()),
        (Some(3), Some(1), None)
    );
    assert!(m.prevent_close && !m.has_close_prompt());
    assert_eq!(m.actor_tag.as_deref(), Some("NO_SUCH_TAG"));
    assert_eq!(w.message_position(&m), Some([50., 0., 0.]));
    // Another Thing's bubble without a prompt takes its place; the prompt bubble stays.
    say(
        &mut w,
        "NoPrompt",
        &bubble("Second", None, Some(6), [false, false], 0),
    );
    let texts: Vec<&str> = w.messages.open.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, ["Hello there", "Second"]);
    assert_eq!(
        (w.messages.open[1].id, w.messages.open[1].anchor),
        (2, Some(4))
    );

    // The caption and think forms are types of their own, with and without a prompt.
    for (form, kind, style, ty) in [
        ("Caption", MessageKind::Bubble, BubbleStyle::Caption, 0x5d),
        (
            "CaptionNoPrompt",
            MessageKind::BubbleNoPrompt,
            BubbleStyle::Caption,
            0x5e,
        ),
        ("Think", MessageKind::Bubble, BubbleStyle::Think, 0x5f),
        (
            "ThinkNoPrompt",
            MessageKind::BubbleNoPrompt,
            BubbleStyle::Think,
            0x60,
        ),
    ] {
        say(
            &mut w,
            form,
            &bubble(form, None, Some(8), [false, false], 0),
        );
        let m = w.messages.open.last().unwrap();
        assert_eq!(
            (m.kind, m.style, m.message_type()),
            (kind, style, ty),
            "{form}"
        );
        assert_eq!(m.text, form);
    }
    assert_eq!(w.messages.open.len(), 6);

    // Subtitles: (text, Thing, flag, has voice). A second one is written over the first.
    let subtitles = |w: &mut LevelWorld, text: &str, thing: u32| {
        let args = [
            Value::Str(text.into()),
            Value::Thing(Some(thing)),
            Value::Bool(false),
            Value::Bool(true),
        ];
        let name = "Messaging_NewTriggerMessageSubtitles__Q6StringQ5Thingbb";
        native(w, manager, name, &args).unwrap();
    };
    subtitles(&mut w, "Line one", 8);
    subtitles(&mut w, "Line two", 6);
    let m = w.messages.open.last().unwrap();
    assert_eq!(
        (m.kind, m.text.as_str(), m.has_voice),
        (MessageKind::Subtitles, "Line two", true)
    );
    // A level-link prompt has no text.
    let link = [Value::Thing(Some(8))];
    native(
        &mut w,
        manager,
        "Messaging_NewLevelLinkPrompt__Q5Thing",
        &link,
    )
    .unwrap();
    assert_eq!(w.messages.open.len(), 8);
    let m = w.messages.open.last().unwrap();
    assert_eq!((m.kind, m.anchor), (MessageKind::LevelLinkPrompt, Some(8)));
    assert!(m.text.is_empty());

    // A script closes its own messages: event 3 on manager 4 with its Thing. Thing 8
    // has the prompt bubble, the four caption and think bubbles and the link prompt.
    event(&mut w, 0, 3, Some(8));
    event(&mut w, 4, 2, Some(8));
    assert_eq!(w.messages.open.len(), 8);
    event(&mut w, 4, 3, Some(8));
    let left: Vec<_> = w.messages.open.iter().map(|m| (m.kind, m.thing)).collect();
    assert_eq!(
        left,
        [
            (MessageKind::BubbleNoPrompt, Some(6)),
            (MessageKind::Subtitles, Some(6)),
        ]
    );
    assert_eq!((w.messages.closed, w.messages.events_elsewhere), (6, 1));
    // A wormhole prompt about no Thing: no anchor and no position.
    let hole = [Value::Thing(None)];
    native(
        &mut w,
        manager,
        "Messaging_NewWormHolePrompt__Q5Thing",
        &hole,
    )
    .unwrap();
    let m = w.messages.open.last().unwrap();
    assert_eq!(
        (m.kind, m.thing, m.anchor),
        (MessageKind::WormHolePrompt, None, None)
    );
    assert!(w.message_position(m).is_none());
    // Thing 8 may speak again, and gets a new id.
    say(
        &mut w,
        "",
        &bubble("Back", None, Some(8), [false, false], 0),
    );
    assert_eq!(w.messages.open.last().unwrap().id, 10);
    // In a slot of another type the actor name is shown as passed.
    w.level_slot_id = (2, 5);
    say(
        &mut w,
        "Think",
        &bubble("Hm", Some("ACTOR_TAG"), Some(8), [false, false], 0),
    );
    assert_eq!(
        w.messages.open.last().unwrap().actor.as_deref(),
        Some("ACTOR_TAG")
    );
    let sum = w.messages.totals();
    assert_eq!(
        (sum.requested, sum.opened, sum.replaced, sum.duplicate),
        (14, 11, 2, 1)
    );
}

#[test]
fn d266_the_overlay_reads_the_worlds_messages_and_tables() {
    use lbp_resource::translation::lams_key;
    use lbp_runtime::{display_text, CLOSE_PROMPT_KEY};
    let mut w = LevelWorld::from_things(&chip_level());
    let manager = "GameMessagingManager";
    let six = "__Q6StringQ6StringQ5Thingbib";
    let say = |w: &mut LevelWorld, form: &str, args: &[Value]| {
        let name = format!("Messaging_NewTriggerMessage{form}{six}");
        assert_eq!(native(w, manager, &name, args), Ok(Value::Void), "{name}");
    };
    // No table: the viewer has no prompt string, and nothing to draw yet.
    assert_eq!(w.close_prompt(), None);
    assert_eq!(w.message_overlay(), Default::default());
    // A prompt bubble from Thing 8 and one without a prompt from Thing 3 on its chip
    // (Thing 1), with made-up markup; a bubble with no text; a subtitle.
    let marked = "Hello <em>there</em><br>friend";
    say(
        &mut w,
        "",
        &bubble(marked, Some("ACTOR_TAG"), Some(8), [false, false], 0),
    );
    say(
        &mut w,
        "NoPrompt",
        &bubble("Press <icon cross>", None, Some(3), [false, false], 0),
    );
    say(
        &mut w,
        "Think",
        &bubble("", None, Some(8), [false, false], 0),
    );
    let subtitle = [
        Value::Str("<i>Spoken</i> line".into()),
        Value::Thing(Some(8)),
        Value::Bool(false),
        Value::Bool(true),
    ];
    let name = "Messaging_NewTriggerMessageSubtitles__Q6StringQ5Thingbb";
    native(&mut w, manager, name, &subtitle).unwrap();
    let o = w.message_overlay();
    let drawn: Vec<_> = o
        .bubbles
        .iter()
        .map(|b| (b.anchor, b.position, b.text.as_str()))
        .collect();
    assert_eq!(
        drawn,
        [
            (8, [1., 2., 3.], "Hello there\nfriend"),
            (1, [50., 0., 0.], "Press [cross]"),
        ]
    );
    // The tag is in no table yet: no name and no prompt line.
    assert_eq!((&o.bubbles[0].name, &o.bubbles[0].prompt), (&None, &None));
    assert_eq!(o.subtitle.as_deref(), Some("Spoken line"));
    assert_eq!((o.empty_not_drawn, o.prompts_not_drawn), (1, 0));
    // The message keeps the text as the script passed it.
    assert_eq!(w.messages.open[0].text, marked);
    assert_eq!(display_text(marked), o.bubbles[0].text);

    // With a table the prompt bubble gets the close prompt's string, patch table first.
    let base = translation_table(&[(CLOSE_PROMPT_KEY, "Shut"), (lams_key("OTHER"), "x")]);
    w.translations = vec![base.clone()];
    assert_eq!(w.close_prompt(), Some("Shut"));
    w.translations = vec![translation_table(&[(CLOSE_PROMPT_KEY, "Go away")]), base];
    assert_eq!(w.close_prompt(), Some("Go away"));
    let o = w.message_overlay();
    assert_eq!(o.bubbles[0].prompt.as_deref(), Some("Go away"));
    assert_eq!(o.bubbles[1].prompt, None);
    // The anchor moves, and the bubble with it.
    w.things[8].world_matrix.as_mut().unwrap()[12] = 70.;
    assert_eq!(w.message_overlay().bubbles[0].position, [70., 2., 3.]);
}

/// `Tweaker`, a made-up global-settings script: `ApplyGlobalSettings__` adds `step` to
/// its member `t` and writes that as the blend factor of the lighting group.
fn tweaker() -> Script {
    let mut s = lamp();
    s.class_name = "Tweaker".into();
    s.super_class = None;
    s.strings_a = [
        "Tweaker",
        "Game",
        "t",
        "step",
        lbp_runtime::APPLY_GLOBAL_SETTINGS,
        "SetGlobalSettingsBlendFactor__Q24Game.EGlobalSettingsTypef",
    ]
    .map(String::from)
    .to_vec();
    s.type_refs = vec![ty(10, 0), ty(10, 1)];
    s.field_refs = vec![
        MemberRef {
            type_ref: 0,
            name: 2,
        },
        MemberRef {
            type_ref: 0,
            name: 3,
        },
    ];
    s.function_refs = vec![MemberRef {
        type_ref: 1,
        name: 5,
    }];
    s.bytecode = vec![
        i(0x95, 4, 4, 0, 0),                    // r4 = thing(r0).t
        i(0x95, 4, 8, 0, 1),                    // r8 = thing(r0).step
        i(0x3d, 4, 4, 4, 8),                    // r4 = r4 + r8 (ADDf)
        i(0x97, 4, 4, 0, 0),                    // thing(r0).t = r4
        u64::from(0x03u8) | 12 << 16 | 5 << 32, // r12 = 5, the lighting group
        i(0xa3, 3, 12, 0, 0),                   // ARG @0 = r12
        i(0xa3, 4, 4, 4, 0),                    // ARG @4 = r4
        i(0xa4, 0, 0xffff, 0, 0),               // CALL Game.SetGlobalSettingsBlendFactor
        i(0xa5, 0, 0, 0, 0),                    // RETURN
    ];
    let mut f = function(0, 4, 0, 9);
    f.stack_size = 32;
    s.function_defs = vec![f];
    s
}

/// D267: the world update calls `ApplyGlobalSettings__` on the Thing of each
/// global-settings slot, every tick; the settings in force follow.
#[test]
fn d267_the_slot_holders_apply_their_global_settings_each_tick() {
    use lbp_runtime::{GlobalSettingsReport, SlotHolder, APPLY_GLOBAL_SETTINGS};
    // Thing 1 (UID 7, a Tweaker) holds the lighting slot. Slot 1: Thing 3 (UID 9), whose
    // script has no such function. Slot 2: a UID of no Thing. Slot 6: Thing 2 (UID 8),
    // which has no script. Slot 0 is empty but stores a priority.
    let mut things = level();
    things.push(ThingRecord {
        index: 3,
        uid: Some(9),
        ..Default::default()
    });
    things[2].uid = Some(8);
    things[1].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![
            field("t", ScriptValue::Float(0.)),
            field("step", ScriptValue::Float(0.25)),
        ],
    });
    things[3].data.script = Some(ScriptPart {
        script: Some(desc(BASE)),
        fields: vec![field("count", ScriptValue::Int(0))],
    });
    things[0].data.world = Some(WorldPart {
        global_settings_thing_uids: vec![0, 9, 40, 0, 0, 7, 8],
        global_settings_thing_priority: vec![3, 1, 1, 0, 0, 1, 2],
        gravity: 2.,
        lighting: [0.2, 0., 0., 0., 0.],
        gs_from: [0.2, 0., 0., 0., 0.],
        gs_target: [1., 0., 0., 0., 0.5],
        gs_blend: 0.,
        ..Default::default()
    });
    let mut vm = Vm::default();
    vm.load(tweaker(), Some(GUID));
    vm.load(reactor("Mute", "Tick", 1, 4), Some(BASE));
    let mut host = Bindings::new(LevelWorld::from_things(&things));
    let at_load = host.world.settings_at_load;
    assert_eq!(at_load.lighting, [0.2, 0., 0., 0., 0.]);
    assert_eq!(at_load.gravity, 2.);
    assert_eq!(host.world.pworld.lighting_stored, at_load.lighting);
    assert_eq!(host.world.settings_unlike_load(), [false; 8]);
    let mut level = LevelScripts::new(&vm, &host.world);

    // The first tick: the empty slot loses its priority, the two slots without a
    // scripted Thing are cleared, the Tweaker is called once.
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert!(t.faults.is_empty(), "{:?}", t.faults);
    let mut want = GlobalSettingsReport {
        no_function: 1,
        slots_cleared: 2,
        ..Default::default()
    };
    want.calls[5] = 1;
    assert_eq!(t.global_settings, want);
    assert!(host.unbound.is_empty(), "{:?}", host.unbound);
    let p = &host.world.pworld;
    assert_eq!(p.global_settings_thing_uids, [0, 9, 0, 0, 0, 7, 0, 0]);
    assert_eq!(p.global_settings_thing_priority, [0, 1, 0, 0, 0, 1, 0, 0]);
    assert_eq!(p.lighting_blend, 0.25);
    let now = p.settings_in_force();
    assert_eq!(now.lighting, [0.4, 0., 0., 0., 0.125]);
    assert_eq!(now.light(), [0.4, 0.125]);
    // Only the lighting group is unlike the level file; the stored values are kept.
    let mut unlike = [false; 8];
    unlike[5] = true;
    assert_eq!(host.world.settings_unlike_load(), unlike);
    assert_eq!(host.world.settings_at_load, at_load);
    assert_eq!(host.world.pworld.lighting_stored, [0.2, 0., 0., 0., 0.]);

    // Every later tick calls again: nothing left to clear, the blend goes on.
    want.slots_cleared = 0;
    for step in 2..=4 {
        let t = level.tick_level(&mut vm, &mut host, 1000);
        assert_eq!(t.global_settings, want, "tick {step}");
        assert_eq!(host.world.pworld.lighting_blend, 0.25 * step as f32);
    }
    assert_eq!(
        host.world.pworld.settings_in_force().lighting,
        [1., 0., 0., 0., 0.5]
    );
    assert_eq!(level.stopped.len(), 0);

    // The pass alone, slot by slot: the holder, an empty slot, a slot out of range.
    assert_eq!(host.world.global_settings_holder(5), SlotHolder::Thing(1));
    assert_eq!(host.world.global_settings_holder(0), SlotHolder::Empty);
    assert_eq!(host.world.global_settings_holder(8), SlotHolder::Empty);
    let t = level.apply_global_settings(&mut vm, &mut host, 1000);
    assert_eq!((t.global_settings.total_calls(), t.done), (1, 1));
    assert_eq!(host.world.pworld.lighting_blend, 1.25);

    // The holder's UID goes to a Thing without a script: the slot is cleared.
    host.world.things[1].uid = Some(70);
    host.world.things[2].uid = Some(7);
    assert_eq!(host.world.global_settings_holder(5), SlotHolder::Cleared);
    assert_eq!(host.world.pworld.global_settings_thing_uids[5], 0);
    assert_eq!(host.world.pworld.global_settings_thing_priority[5], 0);
    assert_eq!(host.world.global_settings_holder(5), SlotHolder::Empty);

    // A holder stopped by a fault is not called and keeps its slot.
    host.world.things[1].uid = Some(7);
    host.world.things[2].uid = Some(8);
    host.world.pworld.global_settings_thing_uids[5] = 7;
    level.stopped.insert(1);
    let t = level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(
        (
            t.global_settings.to_stopped,
            t.global_settings.total_calls()
        ),
        (1, 0)
    );
    assert_eq!(host.world.pworld.lighting_blend, 1.25);
    assert_eq!(host.world.pworld.global_settings_thing_uids[5], 7);
    assert!(vm.has_code(vm.class_by_guid(GUID).unwrap(), APPLY_GLOBAL_SETTINGS));

    // A level without a PWorld has no slots.
    let mut bare = LevelWorld::from_things(&[ThingRecord::default()]);
    assert_eq!(bare.global_settings_holder(5), SlotHolder::Empty);
    assert_eq!(bare.settings_unlike_load(), [false; 8]);
}

/// A level for the display natives. Thing 1 (POS, mesh) has the children 2 (POS, mesh),
/// 3 (mesh and a shape), 5 (no mesh) and 7 (a globe mesh); 4 (mesh) is under 2, 6 (mesh)
/// under 5 and 8 (mesh) under 7.
fn display_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..9)
        .map(|index| ThingRecord {
            index,
            parent: (index > 0).then_some([0, 0, 1, 1, 2, 1, 5, 1, 7][index]),
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    for (n, guid) in [
        (1, 0x10),
        (2, 0x11),
        (3, 0x12),
        (4, 0x13),
        (6, 0x14),
        (7, 0x27bd),
        (8, 0x15),
    ] {
        t[n].data.render_mesh = render_mesh(guid).map(|mut m| {
            m.visibility_flags = 3;
            m.editor_color = 0xffff_ffff;
            m
        });
        t[n].parts.push("RENDER_MESH");
    }
    t[3].parts.push("SHAPE");
    for (n, x) in [(1, 10.), (2, 14.), (4, 15.)] {
        let mut world = lbp_runtime::transform::IDENTITY;
        world[12] = x;
        t[n].data.pos = Some(Pos {
            bone_of: None,
            anim_hash: 0,
            world,
        });
        t[n].parts.push("POS");
    }
    t
}

#[test]
fn d268_hidden_coloured_and_moved_things_are_listed_for_a_display() {
    use lbp_runtime::{pack_colour, DisplayChange, DisplaySummary, MESH_VISIBILITY_MASK};
    let mut w = LevelWorld::from_things(&display_level());
    w.derive_local_matrices();
    w.things[3].shape_colour = Some([0., 1., 0., 1.]);
    let thing = |n: u32| Value::Thing(Some(n));
    let null = Value::Thing(None);
    let at_load: Vec<_> = (0..9).map(|n| w.thing_display(n).unwrap()).collect();
    assert_eq!(at_load[1].mesh_visibility, Some(3));
    assert_eq!(at_load[1].mesh_colour, Some([1.; 4]));
    assert_eq!(
        (at_load[5].mesh_visibility, at_load[5].mesh_colour),
        (None, None)
    );
    assert!(w.take_display_dirty().is_empty() && w.display_changed().is_empty());

    // Hide: the Thing, and down its children with a mesh and no shape. Thing 3 has a
    // shape, Thing 6 hangs under a Thing without a mesh, Thing 8 under a globe mesh.
    let hide = |w: &mut LevelWorld, t: &Value| {
        native(
            w,
            "Thing",
            "HideRenderMesh__Q5Thing",
            std::slice::from_ref(t),
        )
    };
    assert_eq!(hide(&mut w, &thing(1)), Ok(Value::Void));
    let hidden = |w: &LevelWorld| -> Vec<u32> {
        (0..9)
            .filter(|&n| w.thing_display(n).unwrap().mesh_hidden())
            .collect()
    };
    assert_eq!(hidden(&w), [1, 2, 4, 7]);
    let visibility = |things: &[u32]| -> Vec<(u32, DisplayChange)> {
        things
            .iter()
            .map(|&t| (t, DisplayChange::VISIBILITY))
            .collect()
    };
    assert_eq!(w.display_dirty_len(), 4);
    assert_eq!(w.take_display_dirty(), visibility(&[1, 2, 4, 7]));
    // Taken once; hiding again changes nothing; a null Thing and one without a mesh do
    // nothing.
    assert!(w.take_display_dirty().is_empty());
    for t in [thing(1), null.clone(), thing(5), thing(0)] {
        assert_eq!(hide(&mut w, &t), Ok(Value::Void));
    }
    assert!(w.take_display_dirty().is_empty());
    assert_eq!(w.display_changed(), visibility(&[1, 2, 4, 7]));
    // A Thing with a shape is hidden when it is the one named, and nothing below it.
    assert_eq!(hide(&mut w, &thing(3)), Ok(Value::Void));
    assert_eq!(hide(&mut w, &thing(7)), Ok(Value::Void));
    assert_eq!(w.take_display_dirty(), visibility(&[3]));
    // Only the two bits of the mask change.
    w.things[8].render_mesh_visibility = Some(0xf4 | MESH_VISIBILITY_MASK);
    assert_eq!(hide(&mut w, &thing(8)), Ok(Value::Void));
    assert_eq!(w.things[8].render_mesh_visibility, Some(0xf4));
    // The note's own question reads the play-mode bit; false without a mesh.
    let in_play = |w: &mut LevelWorld, t: u32| {
        let me = [Value::Thing(Some(t))];
        native(w, "PostItNote", "IsVisibleInPlayMode__", &me)
    };
    assert_eq!(in_play(&mut w, 1), Ok(Value::Bool(false)));
    assert_eq!(in_play(&mut w, 6), Ok(Value::Bool(true)));
    assert_eq!(in_play(&mut w, 5), Ok(Value::Bool(false)));
    // Show sets them again, down the same children.
    let show = [thing(1)];
    assert_eq!(
        native(&mut w, "Thing", "ShowRenderMesh__Q5Thing", &show),
        Ok(Value::Void)
    );
    assert_eq!(hidden(&w), [3, 8]);
    assert_eq!(w.things[2].render_mesh_visibility, Some(3));
    assert_eq!(w.take_display_dirty(), visibility(&[1, 2, 4, 7, 8]));
    // Back at the file's values, Things 1, 2, 4 and 7 are no longer unlike it.
    assert_eq!(w.display_changed(), visibility(&[3, 8]));

    // Colours: the mesh colour and tint need a PRenderMesh, the shape's a PShape.
    let red = Value::Vec4([1., 0., 0., 1.]);
    let colour = |w: &mut LevelWorld, name: &str, t: &Value, c: &Value| {
        native(w, "Thing", name, &[t.clone(), c.clone()])
    };
    for name in [
        "SetRenderMeshEditorColour__Q5Thingr",
        "SetRenderMeshEditorColourTint__Q5Thingr",
        "SetShapeEditorColour__Q5Thingr",
        "SetShapeEditorColourTint__Q5Thingr",
    ] {
        assert_eq!(colour(&mut w, name, &null, &red), Ok(Value::Void), "{name}");
        assert_eq!(colour(&mut w, name, &thing(5), &red), Ok(Value::Void));
        assert!(colour(&mut w, name, &thing(1), &Value::Int(1)).is_err());
        assert!(colour(&mut w, name, &thing(99), &red).is_err(), "{name}");
    }
    assert!(w.take_display_dirty().is_empty());
    assert_eq!(
        colour(
            &mut w,
            "SetRenderMeshEditorColour__Q5Thingr",
            &thing(1),
            &red
        ),
        Ok(Value::Void)
    );
    assert_eq!(
        colour(
            &mut w,
            "SetRenderMeshEditorColourTint__Q5Thingr",
            &thing(2),
            &red
        ),
        Ok(Value::Void)
    );
    for name in [
        "SetShapeEditorColour__Q5Thingr",
        "SetShapeEditorColourTint__Q5Thingr",
    ] {
        assert_eq!(colour(&mut w, name, &thing(3), &red), Ok(Value::Void));
        // Thing 1 has no shape.
        assert_eq!(colour(&mut w, name, &thing(1), &red), Ok(Value::Void));
    }
    assert_eq!(w.things[1].render_mesh_colour, Some([1., 0., 0., 1.]));
    assert_eq!(w.things[2].render_mesh_tint, Some([1., 0., 0., 1.]));
    assert_eq!(w.things[2].render_mesh_colour, Some([1.; 4]));
    assert_eq!(w.things[3].shape_colour, Some([1., 0., 0., 1.]));
    assert_eq!(w.things[3].shape_tint, Some([1., 0., 0., 1.]));
    assert_eq!(w.things[1].shape_colour, None);
    assert_eq!(
        w.take_display_dirty(),
        [1, 2, 3].map(|t| (t, DisplayChange::COLOUR))
    );
    // The file's colour set again, and a white tint on a mesh never tinted: no change.
    let white = Value::Vec4([1.; 4]);
    assert_eq!(
        colour(
            &mut w,
            "SetRenderMeshEditorColour__Q5Thingr",
            &thing(4),
            &white
        ),
        Ok(Value::Void)
    );
    assert_eq!(
        colour(
            &mut w,
            "SetRenderMeshEditorColourTint__Q5Thingr",
            &thing(4),
            &white
        ),
        Ok(Value::Void)
    );
    assert!(w.take_display_dirty().is_empty());
    assert_eq!(
        pack_colour(w.things[1].render_mesh_colour.unwrap()),
        0xffff_0000
    );

    // Moving Thing 1 moves its descendants with a PPos: 2 and 4.
    let mut to = lbp_runtime::transform::IDENTITY;
    to[12..15].copy_from_slice(&[20., 5., 0.]);
    w.set_world_transform(1, to).unwrap();
    assert_eq!(
        w.take_display_dirty(),
        [1, 2, 4].map(|t| (t, DisplayChange::TRANSFORM))
    );
    let now = w.thing_display(4).unwrap();
    assert_eq!(&now.world.unwrap()[12..15], &[25., 5., 0.]);
    assert_eq!(w.display_at_load(4).unwrap().world, at_load[4].world);
    let by = now.moved_from(&w.display_at_load(4).unwrap()).unwrap();
    assert_eq!(&by[12..15], &[10., 5., 0.]);
    assert!(now.moved_far(&at_load[4]));
    // The same matrix again is no change; the local form and the member mark too.
    w.set_world_transform(1, to).unwrap();
    assert!(w.take_display_dirty().is_empty());
    let mut local = lbp_runtime::transform::IDENTITY;
    local[12] = 2.;
    w.set_local_transform(4, local).unwrap();
    assert_eq!(w.take_display_dirty(), [(4, DisplayChange::TRANSFORM)]);
    let ppos = part(&mut w, 2, "Pos");
    let member = Value::Mat44(lbp_runtime::transform::IDENTITY);
    w.set_member(MemberKind::RefPtr, &ppos, "PPos", "WorldPosition", member)
        .unwrap();
    assert_eq!(w.take_display_dirty(), [(2, DisplayChange::TRANSFORM)]);
    // A player the caller moves is not a change of the level's Things.
    let player = w.add_player([0.; 3], (0., 0.));
    w.move_player(player, [5., 5., 0.], (0., 0.));
    assert!(w.take_display_dirty().is_empty());

    // Several changes of one Thing are one entry.
    hide(&mut w, &thing(4)).unwrap();
    colour(
        &mut w,
        "SetRenderMeshEditorColour__Q5Thingr",
        &thing(4),
        &red,
    )
    .unwrap();
    w.set_local_transform(4, lbp_runtime::transform::IDENTITY)
        .unwrap();
    let all = DisplayChange {
        visibility: true,
        colour: true,
        transform: true,
        mesh: false,
    };
    assert_eq!(w.take_display_dirty(), [(4, all)]);

    // The summary counts the Things unlike the file.
    let s = w.display_summary();
    assert_eq!(s.changed, w.display_changed().len());
    let want = DisplaySummary {
        changed: s.changed,
        mesh_hidden: 3,
        mesh_visibility: 3,
        mesh_colour: 2,
        mesh_tint: 1,
        shape_colour: 1,
        shape_tint: 1,
        moved: s.moved,
        moved_far: s.moved_far,
        moved_far_with_mesh: s.moved_far,
        moved_far_with_shape: 0,
        ..Default::default()
    };
    assert_eq!(s, want);
    assert!(s.moved >= 2 && s.moved_far >= 2 && s.moved_far <= s.moved);
    assert!(w.display.marks[..3].iter().all(|n| *n > 0));
    assert_eq!(w.display.marks[3], 0);
    assert!(w.thing_display(99).is_none() && w.display_at_load(99).is_none());
}

/// `Watcher`, a made-up script: `Update__` asks for the screen position of its own
/// Thing and keeps x, y and the screen's width in its members `sx`, `sy` and `xres`.
fn watcher() -> Script {
    let mut s = lamp();
    s.class_name = "Watcher".into();
    s.super_class = None;
    s.strings_a = [
        "Watcher",
        "Thing",
        "sx",
        "GetWorldPosition__",
        lbp_runtime::UPDATE,
        "GetScreenSpacePosition__r",
        "HUD",
        "sy",
        "GetXRes__",
        "xres",
    ]
    .map(String::from)
    .to_vec();
    s.type_refs = vec![ty(10, 0), ty(10, 1), ty(10, 6)];
    let member = |type_ref, name| MemberRef { type_ref, name };
    s.field_refs = vec![member(0, 2), member(0, 7), member(0, 9)];
    s.function_refs = vec![member(1, 3), member(2, 5), member(2, 8)];
    s.bytecode = vec![
        i(0xa3, 10, 0, 0, 0),  // ARG @0 = this
        i(0xa4, 6, 4, 0, 0),   // r4 = CALL Thing.GetWorldPosition__ (m44)
        i(0x78, 0, 8, 4, 0),   // r8 = GET_M44_cW r4
        i(0xa3, 5, 8, 0, 0),   // ARG @0 = r8
        i(0xa4, 5, 16, 1, 0),  // r16 = CALL HUD.GetScreenSpacePosition__r
        i(0x5a, 0, 12, 16, 0), // r12 = GET_V4_X r16
        i(0x97, 4, 12, 0, 0),  // thing(r0).sx = r12
        i(0x5b, 0, 12, 16, 0), // r12 = GET_V4_Y r16
        i(0x97, 4, 12, 0, 1),  // thing(r0).sy = r12
        i(0xa4, 3, 20, 2, 0),  // r20 = CALL HUD.GetXRes__
        i(0x97, 3, 20, 0, 2),  // thing(r0).xres = r20
        i(0xa5, 0, 0, 0, 0),   // RETURN
    ];
    let mut f = function(0, 4, 0, 12);
    f.stack_size = 64;
    s.function_defs = vec![f];
    s
}

/// D269: the override camera's two stores and its blend, the camera matrix a script
/// reads, and the screen position of a level point.
#[test]
fn d269_the_override_camera_blends_in_and_the_screen_natives_use_the_view() {
    use lbp_runtime::{
        view_projection, CameraPose, CameraView, OVERRIDE_FAR, OVERRIDE_FOV_DEG, OVERRIDE_NEAR,
        SCREEN,
    };
    let mut w = LevelWorld::from_things(&level());
    let matrix = |w: &mut LevelWorld| match native(w, "Cam", "GetCameraMatrix__", &[]) {
        Ok(Value::Mat44(m)) => m,
        other => panic!("{other:?}"),
    };
    let v = |x: f32, y: f32, z: f32, w: f32| Value::Vec4([x, y, z, w]);
    let void = |r: Result<Value, String>| assert_eq!(r, Ok(Value::Void));
    let activate = |w: &mut LevelWorld, on: bool, rate: f32| {
        let args = [Value::Bool(on), Value::Float(rate)];
        void(native(w, "Cam", "SetOverrideCameraActivation__bf", &args));
    };

    // The screen is 1280 x 720.
    assert_eq!(
        native(&mut w, "HUD", "GetXRes__", &[]),
        Ok(Value::Int(1280))
    );
    assert_eq!(native(&mut w, "HUD", "GetYRes__", &[]), Ok(Value::Int(720)));
    assert_eq!(SCREEN, [1280, 720]);

    // No view and no player: the identity. With a player: the default follow camera.
    assert_eq!(matrix(&mut w), identity());
    assert_eq!(w.default_follow_pose(), None);
    w.add_player([10., 20., 30.], (0., 60.));
    let follow = CameraPose {
        eye: [306., 188., 1248.],
        direction: [0., 0., -1.],
    };
    assert_eq!(w.default_follow_pose(), Some(follow));
    assert_eq!(matrix(&mut w), translation(306., 188., 1248.));
    assert_eq!(w.camera.matrix_queries, 2);

    // The override's pair is kept as passed, the last one wins, and nothing blends
    // before the activation.
    let set = |w: &mut LevelWorld, at: Value, dir: Value| {
        void(native(w, "Cam", "SetOverrideCamera__rr", &[at, dir]));
    };
    set(&mut w, v(0., 0., 0., 1.), v(0., 1., 0., 0.));
    set(&mut w, v(1306., 688., 248., 1.), v(2., 0., 0., 0.));
    let o = w.camera.override_camera;
    assert_eq!(o.position, [1306., 688., 248., 1.]);
    assert_eq!(o.direction, [2., 0., 0., 0.]);
    assert_eq!((o.set_calls, o.blend, o.target), (2, 0., 0.));
    for _ in 0..5 {
        w.step_camera();
    }
    assert_eq!(w.camera.override_camera.blend, 0.);
    assert_eq!(w.camera_pose(follow.eye, follow.direction), None);

    // The activation sets the target and the rate, not the blend; the blend then takes
    // 20 ticks at the scoreboard's rate.
    activate(&mut w, true, 0.05);
    let o = w.camera.override_camera;
    assert_eq!((o.target, o.rate, o.blend), (1., 0.05, 0.));
    assert_eq!((o.activation_calls, o.activations_on), (1, 1));
    w.step_camera();
    assert!(w.camera.override_camera.is_in());
    let first = w.camera_pose(follow.eye, follow.direction).unwrap();
    assert!(first.eye[0] > 306. && first.eye[0] < 320., "{first:?}");
    for _ in 1..10 {
        w.step_camera();
    }
    // Half way: the midpoint and the half angle (the ease of 0.5 is 0.5).
    let o = w.camera.override_camera;
    assert!((o.blend - 0.5).abs() < 1e-5 && (o.weight() - 0.5).abs() < 1e-5);
    let half = w.camera_pose(follow.eye, follow.direction).unwrap();
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 0.01);
    assert!(near(half.eye, [806., 438., 748.]), "{half:?}");
    let r = std::f32::consts::FRAC_1_SQRT_2;
    assert!(near(half.direction, [r, 0., -r]), "{half:?}");
    // Without a view the camera matrix a script reads is that camera's.
    let m = matrix(&mut w);
    assert!(near([m[12], m[13], m[14]], half.eye));
    assert!(near([-m[8], -m[9], -m[10]], half.direction));
    for _ in 10..20 {
        w.step_camera();
    }
    assert_eq!(w.camera.override_camera.blend, 1.);
    let full = w.camera_pose(follow.eye, follow.direction).unwrap();
    assert_eq!(full.eye, [1306., 688., 248.]);
    assert!(near(full.direction, [1., 0., 0.]));
    // It stays in until a script turns it off; then 20 ticks back.
    for _ in 0..100 {
        w.step_camera();
    }
    assert_eq!(w.camera.override_camera.blend, 1.);
    activate(&mut w, false, 0.05);
    assert_eq!(w.camera.override_camera.blend, 1.);
    for _ in 0..19 {
        w.step_camera();
    }
    assert!(w.camera.override_camera.is_in());
    w.step_camera();
    let o = w.camera.override_camera;
    assert_eq!((o.blend, o.target, o.blend_max), (0., 0., 1.));
    assert_eq!((o.activation_calls, o.activations_on), (2, 1));
    assert_eq!(o.ticks_in, 20 + 100 + 19);
    assert_eq!(w.camera_pose(follow.eye, follow.direction), None);
    // A follow direction that is no direction gives no pose.
    activate(&mut w, true, 1.);
    w.step_camera();
    assert_eq!(w.camera_pose(follow.eye, [0.; 3]), None);
    activate(&mut w, false, 1.);
    w.step_camera();

    // The screen position. Without a view: the middle of the screen, counted.
    let screen = |w: &mut LevelWorld, at: Value| match native(
        w,
        "HUD",
        "GetScreenSpacePosition__r",
        &[at],
    ) {
        Ok(Value::Vec4(p)) => p,
        other => panic!("{other:?}"),
    };
    assert_eq!(screen(&mut w, v(1e6, 0., 0., 1.)), [640., 360., 0., 0.]);
    let c = &w.camera;
    assert_eq!(
        (
            c.screen_queries,
            c.screen_queries_without_view,
            c.screen_queries_off_screen
        ),
        (1, 1, 0)
    );
    // With the view a display wrote: its matrix is the camera matrix, and points are
    // projected by its view-projection.
    let camera = CameraPose {
        eye: [0., 0., 1000.],
        direction: [0., 0., -1.],
    }
    .matrix();
    let view_proj = view_projection(
        &camera,
        OVERRIDE_FOV_DEG,
        16. / 9.,
        OVERRIDE_NEAR,
        OVERRIDE_FAR,
    )
    .unwrap();
    w.camera.view = Some(CameraView { camera, view_proj });
    assert_eq!(matrix(&mut w), camera);
    let mid = screen(&mut w, v(0., 0., 0., 1.));
    assert!((mid[0] - 640.).abs() < 0.01 && (mid[1] - 360.).abs() < 0.01);
    assert_eq!(mid[3], 0.);
    // Up on the screen is the smaller y; the point's own w is ignored.
    let up = screen(&mut w, v(0., 100., 0., 0.));
    assert!(up[1] < 360. - 50. && (up[0] - 640.).abs() < 0.01);
    // Far to the right: more than the margin outside, counted as off screen.
    let right = screen(&mut w, v(5000., 0., 0., 1.));
    assert!(right[0] > 1280. + 100.);
    let c = &w.camera;
    assert_eq!(
        (
            c.screen_queries,
            c.screen_queries_without_view,
            c.screen_queries_off_screen
        ),
        (4, 1, 1)
    );

    // A level without a PWorld has no level camera: the calls are counted and change
    // nothing.
    let mut bare = LevelWorld::from_things(&[ThingRecord::default()]);
    set(&mut bare, v(1., 2., 3., 1.), v(1., 0., 0., 0.));
    activate(&mut bare, true, 0.5);
    bare.step_camera();
    bare.step_camera();
    let o = bare.camera.override_camera;
    assert_eq!((o.set_calls, o.activation_calls), (1, 1));
    assert_eq!((o.position, o.target, o.blend), ([0.; 4], 0., 0.));
}

/// D269: a script reads the screen position of its Thing each tick, as the speech bubble
/// does for its off-screen timer, and the tick steps the camera's blend.
#[test]
fn d269_a_script_reads_its_screen_position_each_tick() {
    use lbp_runtime::{view_projection, CameraPose, CameraView, OVERRIDE_FOV_DEG};
    let mut things = level();
    things[1].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![
            field("sx", ScriptValue::Float(-1.)),
            field("sy", ScriptValue::Float(-1.)),
            field("xres", ScriptValue::Int(0)),
        ],
    });
    let mut vm = Vm::default();
    vm.load(watcher(), Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&things));
    let mut level = LevelScripts::new(&vm, &host.world);
    let member = |host: &mut Bindings<LevelWorld>, name: &str| {
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(1)), "Watcher", name)
            .unwrap()
    };
    // No view: the middle of the screen every tick, never off screen.
    for _ in 0..61 {
        let t = level.tick_level(&mut vm, &mut host, 1000);
        assert!(t.faults.is_empty(), "{:?}", t.faults);
    }
    assert!(host.unbound.is_empty(), "{:?}", host.unbound);
    assert_eq!(member(&mut host, "sx"), Value::Float(640.));
    assert_eq!(member(&mut host, "sy"), Value::Float(360.));
    assert_eq!(member(&mut host, "xres"), Value::Int(1280));
    let c = host.world.camera.clone();
    assert_eq!(
        (
            c.screen_queries,
            c.screen_queries_without_view,
            c.screen_queries_off_screen
        ),
        (61, 61, 0)
    );
    // A camera that looks at the Thing (at (1, 2, 3)): the middle again, now measured.
    let view_at = |eye: [f32; 3]| {
        let camera = CameraPose {
            eye,
            direction: [0., 0., -1.],
        }
        .matrix();
        let view_proj = view_projection(&camera, OVERRIDE_FOV_DEG, 16. / 9., 10., 40_000.).unwrap();
        CameraView { camera, view_proj }
    };
    host.world.camera.view = Some(view_at([1., 2., 1003.]));
    level.tick_level(&mut vm, &mut host, 1000);
    let (Value::Float(x), Value::Float(y)) = (member(&mut host, "sx"), member(&mut host, "sy"))
    else {
        panic!()
    };
    assert!(
        (x - 640.).abs() < 0.01 && (y - 360.).abs() < 0.01,
        "{x} {y}"
    );
    assert_eq!(host.world.camera.screen_queries_off_screen, 0);
    // A camera far to the left of it: the Thing is more than 100 pixels past the right
    // edge on each of 61 ticks, which is when a speech bubble would close.
    host.world.camera.view = Some(view_at([-3000., 2., 1003.]));
    for _ in 0..61 {
        level.tick_level(&mut vm, &mut host, 1000);
    }
    let Value::Float(x) = member(&mut host, "sx") else {
        panic!()
    };
    assert!(x > 1280. + 100., "{x}");
    let c = &host.world.camera;
    assert_eq!(
        (
            c.screen_queries,
            c.screen_queries_without_view,
            c.screen_queries_off_screen
        ),
        (123, 61, 61)
    );
    // The tick steps the blend once.
    use lbp_script_vm::natives::dispatch;
    let on = [Value::Bool(true), Value::Float(0.25)];
    dispatch(
        &mut host.world,
        "Cam",
        "SetOverrideCameraActivation__bf",
        &on,
    )
    .unwrap()
    .unwrap();
    for want in [0.25, 0.5, 0.75, 1., 1.] {
        level.tick_level(&mut vm, &mut host, 1000);
        assert_eq!(host.world.camera.override_camera.blend, want);
    }
    assert_eq!(host.world.camera.override_camera.ticks_in, 5);
}

/// D270. World Thing 0 and seven Things under it, with made-up names and meshes:
/// 1 scripted; 2 a mesh, named "gate"; 3 `first` as its mesh, an empty name; 4 `second`
/// as its mesh, named "Gate"; 5 no mesh, named "gate" again; 6 a name stored with
/// nothing in it; 7 nothing.
fn named_level(first: u32, second: u32) -> Vec<ThingRecord> {
    use lbp_level::parts::ScriptName;
    let mut t: Vec<ThingRecord> = (0..8)
        .map(|index| ThingRecord {
            index,
            parent: (index > 0).then_some(0),
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    for (n, guid) in [(2, 0x10), (3, first), (4, second)] {
        t[n].data.render_mesh = render_mesh(guid);
        t[n].parts.push("RENDER_MESH");
    }
    for (n, name) in [(2, "gate"), (3, ""), (4, "Gate"), (5, "gate"), (6, "")] {
        t[n].data.script_name = Some(ScriptName {
            stored: name.len(),
            name: name.as_bytes().to_vec(),
        });
        t[n].parts.push("SCRIPT_NAME");
    }
    t
}

/// D270: `Thing.FindThing` compares the whole name, byte for byte, and the first Thing
/// in Thing order wins.
#[test]
fn d270_things_are_found_by_their_script_name() {
    let mut w = LevelWorld::from_things(&named_level(0x11, 0x12));
    assert_eq!(w.things[2].script_name.as_deref(), Some(&b"gate"[..]));
    assert_eq!(w.things[1].script_name, None);
    assert_eq!(w.script_names(), (5, 3));
    let find = |w: &mut LevelWorld, name: Value| {
        native(w, "Thing", "FindThing__Q7StringA", &[name]).unwrap()
    };
    let text = |s: &str| Value::Str(s.into());
    // Two Things have the name: the first in Thing order.
    assert_eq!(find(&mut w, text("gate")), Value::Thing(Some(2)));
    // Another case is another name; a part of a name, or a longer one, is no match.
    assert_eq!(find(&mut w, text("Gate")), Value::Thing(Some(4)));
    assert_eq!(find(&mut w, text("GATE")), Value::Thing(None));
    assert_eq!(find(&mut w, text("gat")), Value::Thing(None));
    assert_eq!(find(&mut w, text("gate ")), Value::Thing(None));
    // The empty string is the name of the first Thing with an empty name.
    assert_eq!(find(&mut w, text("")), Value::Thing(Some(3)));
    // A null string finds nothing.
    assert_eq!(find(&mut w, Value::Object(None)), Value::Thing(None));
    assert_eq!((w.pod.find_thing_calls, w.pod.find_thing_hits), (6, 3));
    // A level without a name.
    let mut bare = LevelWorld::from_things(&level());
    assert_eq!(bare.script_names(), (0, 0));
    assert_eq!(find(&mut bare, text("gate")), Value::Thing(None));
    assert_eq!(find(&mut bare, text("")), Value::Thing(None));
}

/// D270: the pod Thing is the first Thing with one of the two pod meshes, by the mesh
/// its PRenderMesh shows now; the player's avatar is his Sackboy.
#[test]
fn d270_the_pod_thing_follows_the_mesh_and_the_avatar_is_the_sackboy() {
    use lbp_runtime::{DisplayChange, DisplaySummary};
    use lbp_script_vm::natives::{DEFAULT_PLANETS_BACKGROUND, POD_MESH, POD_PLATFORM_MESH};
    let pod = |w: &mut LevelWorld, platform_too: bool| {
        native(w, "Thing", "GetPodThing__b", &[Value::Bool(platform_too)]).unwrap()
    };
    let load = |w: &mut LevelWorld, guid: u32| {
        let g = [Value::Int(guid as i32)];
        native(w, "Resource", "LoadMeshByFilename__g", &g).unwrap()
    };
    let set = |w: &mut LevelWorld, thing: Option<u32>, mesh: Value| {
        let args = [Value::Thing(thing), mesh];
        native(w, "Thing", "SetRenderMeshMesh__Q5ThingQ8Resource", &args)
    };
    // The pod mesh alone: found with and without the platform.
    let mut w = LevelWorld::from_things(&named_level(0x11, POD_MESH));
    assert_eq!(pod(&mut w, true), Value::Thing(Some(4)));
    assert_eq!(pod(&mut w, false), Value::Thing(Some(4)));
    // Neither mesh: null.
    let mut none = LevelWorld::from_things(&named_level(0x11, 0x12));
    assert_eq!(pod(&mut none, true), Value::Thing(None));
    assert_eq!(pod(&mut none, false), Value::Thing(None));
    assert_eq!((none.pod.pod_thing_calls, none.pod.pod_thing_hits), (2, 0));
    // The platform before the pod: the platform's Thing for true; for false the search
    // ends at the platform with null, though a pod mesh comes after it.
    let mut w2 = LevelWorld::from_things(&named_level(POD_PLATFORM_MESH, POD_MESH));
    assert_eq!(pod(&mut w2, true), Value::Thing(Some(3)));
    assert_eq!(pod(&mut w2, false), Value::Thing(None));
    assert_eq!((w2.pod.pod_thing_calls, w2.pod.pod_thing_hits), (2, 1));

    // A script gives Thing 3 another mesh: the answer follows, and a display is told.
    assert!(w2.take_display_dirty().is_empty());
    let other = load(&mut w2, 0x10);
    assert_eq!(other, Value::Ref(0x10));
    assert_eq!(set(&mut w2, Some(3), other.clone()), Ok(Value::Void));
    assert_eq!(w2.things[3].mesh_guid, Some(0x10));
    assert_eq!(pod(&mut w2, false), Value::Thing(Some(4)));
    assert_eq!(pod(&mut w2, true), Value::Thing(Some(4)));
    assert_eq!(w2.take_display_dirty(), [(3, DisplayChange::MESH)]);
    assert_eq!(w2.thing_display(3).unwrap().mesh, Some(0x10));
    assert_eq!(w2.display_at_load(3).unwrap().mesh, Some(POD_PLATFORM_MESH));
    // The same mesh again is no change.
    assert_eq!(set(&mut w2, Some(3), other), Ok(Value::Void));
    assert!(w2.take_display_dirty().is_empty());
    // The pod mesh put on an earlier Thing makes that Thing the pod.
    let pod_mesh = load(&mut w2, POD_MESH);
    assert_eq!(set(&mut w2, Some(2), pod_mesh), Ok(Value::Void));
    assert_eq!(pod(&mut w2, false), Value::Thing(Some(2)));
    // A resource that is not a mesh (a texture's handle), a zero handle and a null
    // object clear the mesh; then Thing 2 is no pod.
    let texture = native(
        &mut w2,
        "Resource",
        "LoadTextureByFilename__g",
        &[Value::Int(0x77)],
    )
    .unwrap();
    for mesh in [texture, Value::Ref(0), Value::Object(None)] {
        w2.things[2].mesh_guid = Some(POD_MESH);
        assert_eq!(set(&mut w2, Some(2), mesh), Ok(Value::Void));
        assert_eq!(w2.things[2].mesh_guid, None);
    }
    assert_eq!(pod(&mut w2, false), Value::Thing(Some(4)));
    // Nothing for a null Thing or a Thing without a PRenderMesh; a Thing the level
    // does not have is an error.
    let platform = load(&mut w2, POD_PLATFORM_MESH);
    w2.take_display_dirty();
    let sets = w2.pod.mesh_sets;
    assert_eq!(set(&mut w2, None, platform.clone()), Ok(Value::Void));
    assert_eq!(set(&mut w2, Some(5), platform.clone()), Ok(Value::Void));
    assert_eq!(w2.things[5].mesh_guid, None);
    assert!(set(&mut w2, Some(99), platform).is_err());
    assert!(w2.take_display_dirty().is_empty());
    assert_eq!(w2.pod.mesh_sets, sets);
    // The summary counts the meshes unlike the file's.
    let s = w2.display_summary();
    let want = DisplaySummary {
        changed: 2,
        mesh_swapped: 2,
        ..Default::default()
    };
    assert_eq!(s, want);
    assert_eq!(w2.display.marks[3], 5);

    // The avatar of a player: his Sackboy, once he has one.
    let avatar = |w: &mut LevelWorld, n: i32| game(w, "GetAvatarFromPlayerID__i", &[Value::Int(n)]);
    assert_eq!(avatar(&mut w, 0), Value::Thing(None));
    let sackboy = w.add_player([0.; 3], (-10., 10.));
    assert_eq!(avatar(&mut w, 0), Value::Thing(Some(sackboy)));
    assert_eq!(avatar(&mut w, 1), Value::Thing(None));
    assert_eq!(avatar(&mut w, -1), Value::Thing(None));

    // The pod computer's values that are only kept.
    let background = native(&mut w, "PodComputer", "GetPlanetsBackgroundGUID__", &[]);
    assert_eq!(
        background,
        Ok(Value::Int(DEFAULT_PLANETS_BACKGROUND as i32))
    );
    w.pod.planets_background = Some(0x20493);
    let background = native(&mut w, "PodComputer", "GetPlanetsBackgroundGUID__", &[]);
    assert_eq!(background, Ok(Value::Int(0x20493)));
    let pc = |w: &mut LevelWorld, name: &str, v: Value| native(w, "PodComputer", name, &[v]);
    assert_eq!(w.pod.fade, None);
    for f in [0.25, 0.75] {
        let fade = pc(&mut w, "SetFadePodDecorations__f", Value::Float(f));
        assert_eq!(fade, Ok(Value::Void));
    }
    assert_eq!((w.pod.fade, w.pod.fade_calls), (Some(0.75), 2));
    w.pworld.frame = 9;
    for b in [true, false] {
        let request = pc(&mut w, "RequestHostUsePod__b", Value::Bool(b));
        assert_eq!(request, Ok(Value::Void));
    }
    let asked: Vec<_> = w
        .pod
        .host_use_pod_requests
        .iter()
        .map(|r| (r.in_use, r.frame))
        .collect();
    assert_eq!(asked, [(true, 9), (false, 9)]);
    // A request changes nothing else: nobody is at the pod computer.
    let in_use = native(&mut w, "PodComputer", "GetPodComputerInUse__", &[]);
    assert_eq!(in_use, Ok(Value::Bool(false)));
    assert_eq!(w.engine_calls["SetFadePodDecorations"], 2);
    assert_eq!(w.engine_calls["RequestHostUsePod"], 2);
    // The two state queries.
    assert_eq!(game(&mut w, "IsStartMenuOpen__", &[]), Value::Bool(false));
    let photo = [Value::Thing(Some(sackboy))];
    let taking = native(&mut w, "Pod", "IsTakingPhoto__Q5Thing", &photo);
    assert_eq!(taking, Ok(Value::Bool(false)));
}

/// `Shell`, a made-up script shaped like the Pod's change-over. `Update__`: its member
/// `t` moves toward its member `target` by 0.05 a tick; the pod Thing gets the pod mesh
/// while `t` is below 1 and the platform mesh from then on; `t` goes to the fade.
fn shell() -> Script {
    use lbp_script_vm::natives::{POD_MESH, POD_PLATFORM_MESH};
    let mut s = lamp();
    s.class_name = "Shell".into();
    s.super_class = None;
    s.strings_a = [
        "Shell",
        "Math",
        "Thing",
        "FileList",
        "Resource",
        "PodComputer",
        "t",
        "target",
        lbp_runtime::UPDATE,
        "Delt__fff",
        "GetPodThing__b",
        "AsGUID__i",
        "LoadMeshByFilename__g",
        "SetRenderMeshMesh__Q5ThingQ8Resource",
        "SetFadePodDecorations__f",
    ]
    .map(String::from)
    .to_vec();
    s.type_refs = (0..6).map(|name| ty(10, name)).collect();
    let member = |type_ref, name| MemberRef { type_ref, name };
    s.field_refs = vec![member(0, 6), member(0, 7)];
    s.function_refs = vec![
        member(1, 9),
        member(2, 10),
        member(3, 11),
        member(4, 12),
        member(2, 13),
        member(5, 14),
    ];
    let constant =
        |op: u8, r: u16, bits: u32| u64::from(op) | u64::from(r) << 16 | u64::from(bits) << 32;
    let float = |r: u16, v: f32| constant(0x04, r, v.to_bits());
    s.bytecode = vec![
        i(0x95, 4, 4, 0, 0),                   // r4 = thing(r0).t
        i(0x95, 4, 8, 0, 1),                   // r8 = thing(r0).target
        float(12, 0.05),                       // r12 = 0.05
        i(0xa3, 4, 4, 0, 0),                   // ARG @0 = r4
        i(0xa3, 4, 8, 4, 0),                   // ARG @4 = r8
        i(0xa3, 4, 12, 8, 0),                  // ARG @8 = r12
        i(0xa4, 4, 4, 0, 0),                   // r4 = CALL Math.Delt__fff
        i(0x97, 4, 4, 0, 0),                   // thing(r0).t = r4
        constant(0x01, 16, 0x8000_0001),       // r16 = true
        i(0xa3, 1, 16, 0, 0),                  // ARG @0 = r16
        i(0xa4, 10, 20, 1, 0),                 // r20 = CALL Thing.GetPodThing__b
        float(24, 1.),                         // r24 = 1.0
        i(0x43, 4, 28, 4, 24),                 // r28 = r4 < r24 (LTf)
        constant(0x03, 32, POD_PLATFORM_MESH), // r32 = the platform mesh
        i(0xa7, 0, 28, 2, 0),                  // BEZ r28 -> +2
        constant(0x03, 32, POD_MESH),          // r32 = the pod mesh
        i(0xa3, 3, 32, 0, 0),                  // ARG @0 = r32
        i(0xa4, 3, 36, 2, 0),                  // r36 = CALL FileList.AsGUID__i
        i(0xa3, 3, 36, 0, 0),                  // ARG @0 = r36
        i(0xa4, 9, 40, 3, 0),                  // r40 = CALL Resource.LoadMeshByFilename__g
        i(0xa3, 10, 20, 0, 0),                 // ARG @0 = r20
        i(0xa3, 9, 40, 4, 0),                  // ARG @4 = r40
        i(0xa4, 0, 0xffff, 4, 0),              // CALL Thing.SetRenderMeshMesh
        i(0xa3, 4, 4, 0, 0),                   // ARG @0 = r4
        i(0xa4, 0, 0xffff, 5, 0),              // CALL PodComputer.SetFadePodDecorations__f
        i(0xa5, 0, 0, 0, 0),                   // RETURN
    ];
    let mut f = function(0, 8, 0, 26);
    f.stack_size = 64;
    s.function_defs = vec![f];
    s
}

/// D270: a script moves its change-over value by 0.05 a tick and swaps the pod Thing's
/// mesh on the tick the value reaches 1, as the Pod's world script does.
#[test]
fn d270_a_script_swaps_the_pod_mesh_when_its_transition_ends() {
    use lbp_runtime::DisplayChange;
    use lbp_script_vm::natives::{POD_MESH, POD_PLATFORM_MESH};
    let mut things = named_level(0x11, POD_MESH);
    things[1].parts.push("SCRIPT");
    things[1].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![
            field("t", ScriptValue::Float(0.)),
            field("target", ScriptValue::Float(1.)),
        ],
    });
    let mut vm = Vm::default();
    vm.load(shell(), Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&things));
    let mut level = LevelScripts::new(&vm, &host.world);
    let t = |host: &mut Bindings<LevelWorld>| {
        let me = Value::Thing(Some(1));
        match host.world.get_member(MemberKind::Thing, &me, "Shell", "t") {
            Ok(Value::Float(t)) => t,
            other => panic!("{other:?}"),
        }
    };
    // 19 ticks: the value climbs, the pod keeps its mesh, nothing is marked.
    for tick in 1..=19 {
        let r = level.tick_level(&mut vm, &mut host, 1000);
        assert!(r.faults.is_empty(), "{:?}", r.faults);
        assert!((t(&mut host) - 0.05 * tick as f32).abs() < 1e-5);
        assert_eq!(host.world.things[4].mesh_guid, Some(POD_MESH));
    }
    assert!(host.unbound.is_empty(), "{:?}", host.unbound);
    assert!(host.world.take_display_dirty().is_empty());
    assert_eq!(
        (host.world.pod.mesh_sets, host.world.pod.mesh_changes),
        (19, 0)
    );
    // Tick 20: the value lands on 1 and the mesh is the platform's.
    level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(t(&mut host), 1.);
    assert_eq!(host.world.things[4].mesh_guid, Some(POD_PLATFORM_MESH));
    assert_eq!(host.world.take_display_dirty(), [(4, DisplayChange::MESH)]);
    assert_eq!(host.world.pod.fade, Some(1.));
    // The script asks with true, so it still finds its pod; a caller that asks with
    // false does not.
    assert_eq!(host.world.pod_thing(true), Some(4));
    assert_eq!(host.world.pod_thing(false), None);
    for _ in 0..5 {
        level.tick_level(&mut vm, &mut host, 1000);
    }
    assert!(host.world.take_display_dirty().is_empty());
    assert_eq!(
        (
            host.world.pod.pod_thing_calls,
            host.world.pod.pod_thing_hits
        ),
        (25, 25)
    );
    // Back toward 0: the pod mesh returns on the first tick below 1.
    let me = Value::Thing(Some(1));
    host.world
        .set_member(MemberKind::Thing, &me, "Shell", "target", Value::Float(0.))
        .unwrap();
    level.tick_level(&mut vm, &mut host, 1000);
    assert!((t(&mut host) - 0.95).abs() < 1e-5);
    assert_eq!(host.world.things[4].mesh_guid, Some(POD_MESH));
    assert_eq!(host.world.take_display_dirty(), [(4, DisplayChange::MESH)]);
    // The mesh is the file's again: nothing is unlike the level file.
    assert!(host.world.display_changed().is_empty());
    assert_eq!(host.world.pod.mesh_changes, 2);
}

/// D271: the progression flags are bit masks on one word, and the video and voice-over
/// natives keep their request and answer that nothing started.
#[test]
fn d271_flags_are_bit_masks_and_media_requests_are_kept_and_not_played() {
    use lbp_runtime::{VideoRequest, VoiceOverRequest, MAX_MEDIA_REQUESTS};
    use lbp_script_vm::natives::{NO_VIDEO, RECORDED_ONLY, RECORDED_REQUESTS};
    let mut things = level();
    things[0].data.world = Some(WorldPart {
        frame: 40,
        ..Default::default()
    });
    let mut w = LevelWorld::from_things(&things);
    let (text, flag, int) = (|t: &str| Value::Str(t.into()), Value::Bool, Value::Int);
    // A fresh profile: no bit is set. A mask is set when all of its bits are.
    let is_set = |w: &mut LevelWorld, mask: i32| {
        native(
            w,
            "Profile",
            "IsLBP2GameProgressionFlagSet__i",
            &[int(mask)],
        )
    };
    let set = |w: &mut LevelWorld, mask: i32| {
        native(w, "Profile", "SetLBP2GameProgressionFlag__i", &[int(mask)])
    };
    assert_eq!(w.progression_flags, 0);
    for mask in [1, 2, 3, 4] {
        assert_eq!(is_set(&mut w, mask), Ok(flag(false)), "mask {mask}");
    }
    assert_eq!(set(&mut w, 3), Ok(Value::Void));
    assert_eq!(w.progression_flags, 3);
    for (mask, on) in [(1, true), (2, true), (3, true), (4, false)] {
        assert_eq!(is_set(&mut w, mask), Ok(flag(on)), "mask {mask}");
    }
    // Two bits of which one is set: not set. Setting is an OR: nothing is cleared.
    assert_eq!(is_set(&mut w, 6), Ok(flag(false)));
    assert_eq!(set(&mut w, 0x1ff8), Ok(Value::Void));
    assert_eq!(w.progression_flags, 0x1ffb);
    assert_eq!(is_set(&mut w, 6), Ok(flag(false)));
    assert_eq!(is_set(&mut w, 0x18), Ok(flag(true)));
    // The top bit is a flag like the others (the script's s32 is the word's bits).
    assert_eq!(is_set(&mut w, i32::MIN), Ok(flag(false)));
    assert_eq!(set(&mut w, i32::MIN), Ok(Value::Void));
    assert_eq!(is_set(&mut w, i32::MIN), Ok(flag(true)));
    assert_eq!(w.progression_flags, 0x8000_1ffb);

    // A video request: kept as made, and the script is told that no video started.
    let video = "StartVideo__Q5Thinggb";
    let start = [Value::Thing(None), int(77), flag(false)];
    assert_eq!(native(&mut w, "Video", video, &start), Ok(int(NO_VIDEO)));
    let on_thing = [Value::Thing(Some(1)), Value::Raw(78), flag(true)];
    assert_eq!(native(&mut w, "Video", video, &on_thing), Ok(int(-1)));
    assert_eq!(
        w.media.videos,
        [
            VideoRequest {
                thing: None,
                guid: 77,
                flag: false,
                frame: 40
            },
            VideoRequest {
                thing: Some(1),
                guid: 78,
                flag: true,
                frame: 40
            },
        ]
    );
    // No video has a handle, so none is playing and none is stepped.
    for handle in [-1, 0, 78] {
        let step = [int(handle), flag(false)];
        assert_eq!(
            native(&mut w, "Video", "UpdateVideo__ib", &step),
            Ok(flag(false))
        );
    }
    assert_eq!(
        native(&mut w, "Video", "IsVideoPlaying__", &[]),
        Ok(flag(false))
    );
    assert_eq!(
        native(&mut w, "Video", "StopVideo__i", &[int(-1)]),
        Ok(Value::Void)
    );
    assert_eq!(w.media.video_stops, 1);

    // A voice-over request: kept, answered with false, and no line plays afterwards.
    let request = "RequestVoiceOver__Q7StringAbb";
    let line = [text("made_up_line"), flag(true), flag(false)];
    assert_eq!(native(&mut w, "Audio", request, &line), Ok(flag(false)));
    let null_line = [Value::Object(None), flag(false), flag(true)];
    assert_eq!(
        native(&mut w, "Audio", request, &null_line),
        Ok(flag(false))
    );
    assert_eq!(
        w.media.voice_overs,
        [
            VoiceOverRequest {
                name: Some("made_up_line".into()),
                flags: [true, false],
                frame: 40
            },
            VoiceOverRequest {
                name: None,
                flags: [false, true],
                frame: 40
            },
        ]
    );
    assert_eq!(
        native(&mut w, "Audio", "IsVoiceOverPlaying__", &[]),
        Ok(flag(false))
    );
    assert_eq!(
        native(&mut w, "Audio", "StopVoiceOver__", &[]),
        Ok(flag(true))
    );
    assert_eq!(w.media.voice_over_stops, 1);
    let mix = [text("Made Up Mix"), Value::Float(0.2)];
    for _ in 0..2 {
        let removed = native(&mut w, "Audio", "RemoveMixOverlay__Q7StringAf", &mix);
        assert_eq!(removed, Ok(flag(false)));
    }
    assert_eq!(
        native(&mut w, "Audio", "SetMixOverlay__Q7StringAf", &mix),
        Ok(flag(false))
    );
    assert_eq!(w.media.mix_overlays_set["Made Up Mix"], 1);
    assert_eq!(w.media.mix_overlays_removed["Made Up Mix"], 2);
    // A profile without the first game's save data.
    for name in [
        "HasPlayedLBP1AndUnlockedPod__",
        "HasPlayedLBP1AndUnlockedMoon__",
    ] {
        assert_eq!(native(&mut w, "Profile", name, &[]), Ok(flag(false)));
    }
    // Every call is counted by name with the other recorded calls, and none of them
    // went to the list of kept engine requests or opened a message.
    for (name, calls) in [
        ("StartVideo", 2),
        ("StopVideo", 1),
        ("RequestVoiceOver", 2),
        ("StopVoiceOver", 1),
        ("SetMixOverlay", 1),
        ("RemoveMixOverlay", 2),
    ] {
        assert_eq!(w.engine_calls[name], calls, "{name}");
    }
    assert!(w.requests.is_empty() && w.messages.open.is_empty());
    // The natives that answer are listed apart from the ones that return nothing.
    assert!(RECORDED_ONLY.contains(&("Video", "StopVideo__i")));
    for (class, name, _) in RECORDED_REQUESTS {
        assert!(!RECORDED_ONLY.contains(&(class, name)), "{name}");
    }
    assert_eq!(RECORDED_REQUESTS[0], ("Video", video, "-1"));
    // A wrong argument is an error and nothing is kept for it.
    assert!(native(&mut w, "Video", video, &start[..2]).is_err());
    assert!(native(&mut w, "Audio", request, &mix).is_err());
    assert_eq!((w.media.videos.len(), w.media.voice_overs.len()), (2, 2));
    // Both lists are bounded; the calls past the bound are counted.
    for _ in 0..MAX_MEDIA_REQUESTS {
        native(&mut w, "Video", video, &start).unwrap();
        native(&mut w, "Audio", request, &line).unwrap();
    }
    assert_eq!(w.media.videos.len(), MAX_MEDIA_REQUESTS);
    assert_eq!(w.media.voice_overs.len(), MAX_MEDIA_REQUESTS);
    assert_eq!(
        (w.media.videos_dropped, w.media.voice_overs_dropped),
        (2, 2)
    );
    assert_eq!(w.media.video_requests(), MAX_MEDIA_REQUESTS as u64 + 2);
    assert_eq!(w.media.voice_over_requests(), MAX_MEDIA_REQUESTS as u64 + 2);
}

/// `Thread` with a made-up main function shaped like the Pod's intro: it asks for a
/// video and keeps the handle, asks for a voice-over line, yields five times, waits
/// while a line plays or the handle is not -1, then sets the progression mask 3.
/// `Start__` is the one of [`thread_class`].
fn intro_thread() -> Script {
    let mut s = thread_class();
    s.strings_a = [
        "Thread",
        "System",
        "Yield__",
        "Start__",
        lbp_runtime::THREAD_MAIN,
        "StartImpl__",
        "Video",
        "Audio",
        "Profile",
        "StartVideo__Q5Thinggb",
        "RequestVoiceOver__Q7StringAbb",
        "IsVoiceOverPlaying__",
        "SetLBP2GameProgressionFlag__i",
        "made_up_line",
    ]
    .map(String::from)
    .to_vec();
    s.type_refs = [0, 1, 6, 7, 8].map(|name| ty(11, name)).to_vec();
    let member = |type_ref, name| MemberRef { type_ref, name };
    s.function_refs = vec![
        member(0, 5),
        member(1, 2),
        member(2, 9),
        member(3, 10),
        member(3, 11),
        member(4, 12),
    ];
    let constant =
        |op: u8, r: u16, bits: u32| u64::from(op) | u64::from(r) << 16 | u64::from(bits) << 32;
    s.bytecode = vec![
        i(0xa3, 11, 0, 0, 0),                 //  0 ARG @0 = this
        i(0xa4, 0, 0, 0, 0),                  //  1 CALL Thread.StartImpl__
        i(0xa5, 0, 0, 0, 0),                  //  2 RETURN
        constant(0x06, 4, 0),                 //  3 r4 = null Thing
        constant(0x03, 8, 77),                //  4 r8 = the video's GUID
        constant(0x01, 12, 0),                //  5 r12 = false
        i(0xa3, 10, 4, 0, 0),                 //  6 ARG @0 = r4
        i(0xa3, 3, 8, 4, 0),                  //  7 ARG @4 = r8
        i(0xa3, 1, 12, 8, 0),                 //  8 ARG @8 = r12
        i(0xa4, 3, 16, 2, 0),                 //  9 r16 = CALL Video.StartVideo
        i(0xa2, 3, 0, 16, 0),                 // 10 WRITE r16 (the handle)
        constant(0xbc, 20, 13),               // 11 r20 = the line's name
        constant(0x01, 24, 0x8000_0001),      // 12 r24 = true
        i(0xa3, 11, 20, 0, 0),                // 13 ARG @0 = r20
        i(0xa3, 1, 24, 4, 0),                 // 14 ARG @4 = r24
        i(0xa3, 1, 24, 5, 0),                 // 15 ARG @5 = r24
        i(0xa4, 0, 0xffff, 3, 0),             // 16 CALL Audio.RequestVoiceOver
        constant(0x03, 28, 0),                // 17 r28 = 0
        i(0x0b, 0, 28, 28, 0),                // 18 INCi r28
        i(0xa4, 0, 0, 1, 0),                  // 19 CALL System.Yield
        constant(0x03, 32, 5),                // 20 r32 = 5
        i(0x38, 0, 36, 32, 28),               // 21 r36 = r32 <= r28
        constant(0xa7, 36, -4i32 as u32),     // 22 BEZ r36 -> 18
        i(0xa4, 1, 40, 4, 0),                 // 23 r40 = CALL Audio.IsVoiceOverPlaying
        constant(0x03, 44, -1i32 as u32),     // 24 r44 = -1
        i(0x3c, 0, 48, 16, 44),               // 25 r48 = r16 != r44
        i(0xbd, 0, 52, 40, 48),               // 26 r52 = r40 | r48
        constant(0xa7, 52, 3),                // 27 BEZ r52 -> 30
        i(0xa4, 0, 0, 1, 0),                  // 28 CALL System.Yield
        constant(0xa6, 0xffff, -6i32 as u32), // 29 B -> 23
        constant(0x03, 56, 3),                // 30 r56 = the mask 3
        i(0xa3, 3, 56, 0, 0),                 // 31 ARG @0 = r56
        i(0xa4, 0, 0xffff, 5, 0),             // 32 CALL Profile.SetLBP2GameProgressionFlag
        i(0xa5, 0, 0, 0, 0),                  // 33 RETURN
    ];
    let mut main = function(0, 4, 3, 34);
    main.stack_size = 64;
    s.function_defs = vec![function(0, 3, 0, 3), main];
    s
}

/// D271: a thread shaped like the Pod's intro runs to its end. The video handle it gets
/// is -1 and no line plays, so its wait ends on the tick after its last yield; the flag
/// mask it sets is on the profile and its two requests are kept.
#[test]
fn d271_a_thread_that_waits_on_video_and_voice_over_ends_and_sets_its_flags() {
    let mut vm = Vm::default();
    let thread = vm.load(intro_thread(), Some(0xb25d));
    vm.load(spawner(), Some(GUID));
    let t = vm.instantiate(thread).unwrap();
    let mut world = LevelWorld::from_things(&level());
    world.things[1]
        .members
        .as_mut()
        .unwrap()
        .insert("t".into(), Value::Object(Some(t)));
    let mut host = Bindings::new(world);
    let mut level = LevelScripts::new(&vm, &host.world);
    let created = level.create(&mut vm, &mut host, 1000);
    assert!(
        created.thread_faults.is_empty(),
        "{:?}",
        created.thread_faults
    );
    assert_eq!((created.threads_started, created.threads_done), (1, 0));
    // Both requests are made before the first yield; the handle is the game's -1.
    assert_eq!(host.world.written, ["Int(-1)"]);
    assert_eq!(host.world.media.videos.len(), 1);
    assert_eq!(host.world.media.videos[0].guid, 77);
    let line = &host.world.media.voice_overs[0];
    assert_eq!(line.name.as_deref(), Some("made_up_line"));
    assert_eq!(line.flags, [true, true]);
    // Four more yields: the thread is still there and no flag is set.
    for tick in 1..=4 {
        let r = level.tick_level(&mut vm, &mut host, 1000);
        assert!(r.thread_faults.is_empty(), "{:?}", r.thread_faults);
        assert_eq!((r.threads_done, level.threads()), (0, 1), "tick {tick}");
        assert_eq!(host.world.progression_flags, 0);
    }
    // The tick after the last yield: nothing plays, so the wait is over at once.
    let r = level.tick_level(&mut vm, &mut host, 1000);
    assert!(r.thread_faults.is_empty(), "{:?}", r.thread_faults);
    assert_eq!((r.threads_done, level.threads()), (1, 0));
    assert_eq!(host.world.progression_flags, 3);
    assert!(host.unbound.is_empty(), "{:?}", host.unbound);
    // Nothing was started and nothing more is asked for.
    for _ in 0..3 {
        let r = level.tick_level(&mut vm, &mut host, 1000);
        assert_eq!((r.threads_started, r.threads_done), (0, 0));
    }
    assert_eq!(host.world.media.video_requests(), 1);
    assert_eq!(host.world.media.voice_over_requests(), 1);
    assert_eq!(host.calls["Audio.IsVoiceOverPlaying__"], 1);
}

/// D272: a camera written with `Cam.SetCameraFromToUp` is the camera a display draws
/// with for as long as it is written each tick, and the world Thing a script moves takes
/// the level with it. Made-up numbers.
#[test]
fn d272_a_written_camera_stands_for_its_tick_and_the_world_thing_moves_the_level() {
    use lbp_runtime::{CameraSource, DisplayChange};
    // The world Thing has a stored place of its own.
    let mut things = level();
    things[0].data.pos = Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world: translation(5., 0., 0.),
    });
    let mut vm = Vm::default();
    let mut host = Bindings::new(LevelWorld::from_things(&things));
    let mut level = LevelScripts::new(&vm, &host.world);
    let v = |x: f32, y: f32, z: f32, w: f32| Value::Vec4([x, y, z, w]);
    let f = Value::Float;
    // What a script that wants the camera does each tick.
    let write = |w: &mut LevelWorld, eye_z: f32| {
        let args = [
            v(0., 10., eye_z, 1.),
            v(0., 10., -80., 1.),
            v(0., 1., 0., 0.),
            f(10.),
            f(1e5),
            f(0.),
            f(1.),
        ];
        let r = native(w, "Cam", "SetCameraFromToUp__rrrffff", &args);
        assert_eq!(r, Ok(Value::Void));
    };
    let follow = ([0., 0., 1000.], [0., 0., -1.]);
    let shown = |w: &LevelWorld| w.display_camera(follow.0, follow.1);
    assert_eq!(shown(&host.world), None);
    for n in 0..3 {
        let eye_z = 20. + n as f32;
        write(&mut host.world, eye_z);
        level.tick_level(&mut vm, &mut host, 1000);
        let camera = shown(&host.world).unwrap();
        assert_eq!(camera.source, CameraSource::Script);
        assert_eq!(camera.pose.eye, [0., 10., eye_z]);
        assert_eq!(camera.pose.direction, [0., 0., -1.]);
        assert_eq!((camera.near, camera.far), (10., 1e5));
        assert!((camera.fov_y_deg.unwrap() - 57.295_78).abs() < 1e-3);
    }
    // The matrix a script reads back is the one the call wrote.
    assert_eq!(
        native(&mut host.world, "Cam", "GetCameraMatrix__", &[]),
        Ok(Value::Mat44(translation(0., 10., 22.)))
    );
    // One tick without the call and the display has its own camera again.
    level.tick_level(&mut vm, &mut host, 1000);
    assert_eq!(shown(&host.world), None);
    assert_eq!(host.world.script_camera(), None);
    let c = &host.world.camera;
    assert_eq!(
        (
            c.from_to_up_calls,
            c.script_camera_ticks,
            c.script_camera_ticks_with_override
        ),
        (3, 3, 0)
    );
    assert_eq!(c.from_to_up.unwrap().from, [0., 10., 22., 1.]);

    // The world Thing: where the level file has it, the level is not moved.
    assert_eq!(host.world.world_thing_moved(), Some(identity()));
    host.world.take_display_dirty();
    // A script moves it: twice the size, at (1, 2, 3).
    let mut to = identity();
    (to[0], to[5], to[10]) = (2., 2., 2.);
    to[12..15].copy_from_slice(&[1., 2., 3.]);
    let moved = native(
        &mut host.world,
        "Thing",
        "SetWorldPosition__m",
        &[Value::Thing(Some(0)), Value::Mat44(to)],
    );
    assert_eq!(moved, Ok(Value::Void));
    // From the stored place (5, 0, 0): the matrix that scales by 2 and puts the stored
    // origin's point where the new matrix has it.
    let by = host.world.world_thing_moved().unwrap();
    let mut want = to;
    want[12..15].copy_from_slice(&[1. - 10., 2., 3.]);
    assert_eq!(by, want);
    let dirty = host.world.take_display_dirty();
    assert!(dirty.contains(&(0, DisplayChange::TRANSFORM)), "{dirty:?}");
    // No world Thing, or one without a place: nothing to hang a level under.
    assert_eq!(LevelWorld::default().world_thing_moved(), None);
    assert_eq!(
        LevelWorld::from_things(&self::level()).world_thing_moved(),
        None
    );
}
