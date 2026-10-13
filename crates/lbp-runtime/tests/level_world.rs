//! Rt on authored synthetic data (no game bytes): a three-Thing level and a small script
//! class with a superclass. docs/level-scripts.md.
use lbp_level::parts::{
    Pos, ScriptField, ScriptObject, ScriptPart, ScriptValue, Switch, SwitchOutput, SwitchSignal,
    Trigger, World as WorldPart,
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
    assert_eq!(w.world_position(1), Ok([1., 2., 3., 1.]));
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
    let mut world = LevelWorld::from_things(&things);
    assert_eq!(world.has_part(1, "PCameraTweak"), Ok(true));
    assert_eq!(world.has_part(2, "PCameraTweak"), Ok(false));
    world.set_anim_speed(1, 0.).unwrap();
    world.set_render_mesh_tint(2, [1., 0.5, 0., 1.]).unwrap();
    assert_eq!(world.things[1].anim_speed, Some(0.));
    assert_eq!(world.things[2].render_mesh_tint, Some([1., 0.5, 0., 1.]));
    assert!(world.set_anim_speed(99, 1.).is_err());
    assert_eq!(world.player_number(0), Ok(0));
    assert!(world.player_number(1).is_err());
    assert!(world.player_number(-1).is_err());
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
    let me = Value::Thing(Some(2));
    dispatch(
        &mut w,
        "Thing",
        "SetWorldPosition__m",
        &[me, Value::Mat44(m)],
    )
    .unwrap()
    .unwrap();
    assert_eq!(w.world_position(2), Ok([4., 5., 6., 1.]));
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
    w.progression_flags.insert(2);
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
    assert_eq!(w.score_multiplier_timers.get(&0), Some(&90));
    assert!(dispatch(&mut w, t, n, &[Value::Int(3), Value::Int(1)])
        .unwrap()
        .is_err());
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
