//! Rq VM on authored synthetic scripts (no game bytecode). docs/script-vm.md.
use lbp_level::script::{FieldDef, FunctionDef, MemberRef, Script, Span, TypeOffset, TypeRef};
use lbp_level::serial::Descriptor;
use lbp_script_vm::natives::{Bindings, World};
use lbp_script_vm::{check::check, Fault, Host, LcbRule, MemberKind, Rules, Step, Value, Vm};
use std::collections::BTreeMap;

/// `op | ty << 8 | a << 16 | b << 32 | c << 48` (Toolkit field positions).
fn i(op: u8, ty: u8, a: u16, b: u16, c: u16) -> u64 {
    u64::from(op)
        | u64::from(ty) << 8
        | u64::from(a) << 16
        | u64::from(b) << 32
        | u64::from(c) << 48
}
/// Instruction with a 32-bit immediate in the high word.
fn hi(op: u8, a: u16, value: u32) -> u64 {
    u64::from(op) | u64::from(a) << 16 | u64::from(value) << 32
}

const INT: u8 = 3;
const FLOAT: u8 = 4;
const OBJ: u8 = 11;

struct Func {
    name: &'static str,
    native: bool,
    args: Vec<i32>,
    stack: i32,
    code: Vec<u64>,
}

fn func(name: &'static str, args: &[i32], code: Vec<u64>) -> Func {
    Func {
        name,
        native: false,
        args: args.to_vec(),
        stack: 64,
        code,
    }
}

fn desc(guid: u32) -> Descriptor {
    Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(guid),
        sha1: None,
    }
}

/// Type refs, field refs and function refs are (machine type, type name, script guid)
/// and (type ref, name) pairs over one shared ASCII table.
struct Builder {
    strings: Vec<String>,
    type_refs: Vec<TypeRef>,
    field_refs: Vec<MemberRef>,
    function_refs: Vec<MemberRef>,
    fields: Vec<FieldDef>,
    f32s: Vec<f32>,
}

impl Builder {
    fn new() -> Self {
        Self {
            strings: vec![],
            type_refs: vec![],
            field_refs: vec![],
            function_refs: vec![],
            fields: vec![],
            f32s: vec![],
        }
    }
    fn s(&mut self, v: &str) -> u32 {
        if let Some(p) = self.strings.iter().position(|x| x == v) {
            return p as u32;
        }
        self.strings.push(v.into());
        self.strings.len() as u32 - 1
    }
    fn ty(&mut self, machine: u8, name: &str, guid: Option<u32>, array_base: u8) -> u16 {
        let type_name = self.s(name);
        self.type_refs.push(TypeRef {
            machine_type: machine,
            builtin_type: 0,
            dimension_count: 0,
            array_base_machine_type: array_base,
            script: guid.map(desc),
            type_name,
        });
        self.type_refs.len() as u16 - 1
    }
    fn fref(&mut self, ty: u16, name: &str) -> u16 {
        let name = self.s(name);
        self.function_refs.push(MemberRef {
            type_ref: u32::from(ty),
            name,
        });
        self.function_refs.len() as u16 - 1
    }
    fn field(&mut self, ty: u16, name: &str, defined: bool) -> u16 {
        let name = self.s(name);
        self.field_refs.push(MemberRef {
            type_ref: u32::from(ty),
            name,
        });
        if defined {
            self.fields.push(FieldDef {
                modifiers: 0,
                type_ref: u32::from(ty),
                name,
            });
        }
        self.field_refs.len() as u16 - 1
    }
    fn build(mut self, class: &str, super_guid: Option<u32>, funcs: Vec<Func>) -> Script {
        let mut bytecode = vec![];
        let mut shared = vec![];
        let mut defs = vec![];
        for f in funcs {
            let name = self.s(f.name);
            let a0 = shared.len() as u32;
            for &o in &f.args {
                shared.push(TypeOffset {
                    type_ref: 0,
                    offset: o,
                });
            }
            let b0 = bytecode.len() as u32;
            bytecode.extend(&f.code);
            defs.push(FunctionDef {
                modifiers: if f.native { 2 } else { 0 },
                type_ref: 0,
                name,
                arguments: Span {
                    begin: a0,
                    end: shared.len() as u32,
                },
                bytecode: Span {
                    begin: b0,
                    end: bytecode.len() as u32,
                },
                line_numbers: Span { begin: 0, end: 0 },
                locals: Span { begin: 0, end: 0 },
                stack_size: f.stack,
            });
        }
        Script {
            up_to_date_script: None,
            class_name: class.into(),
            super_class: super_guid.map(desc),
            modifiers: 0,
            type_refs: self.type_refs,
            field_refs: self.field_refs,
            function_refs: self.function_refs,
            field_defs: self.fields,
            property_defs: vec![],
            function_defs: defs,
            shared_arguments: shared,
            bytecode,
            line_numbers: vec![],
            locals: vec![],
            strings_a: self.strings,
            strings_w: vec![],
            constants_s64: Some(vec![]),
            constants_f32: self.f32s,
            depending_guids: Some(vec![]),
        }
    }
}

#[derive(Default)]
struct TestHost {
    calls: Vec<(String, String, Vec<Value>)>,
    members: BTreeMap<String, Value>,
    log: Vec<String>,
}

impl Host for TestHost {
    fn call_native(&mut self, class: &str, f: &str, args: &[Value]) -> Result<Value, String> {
        self.calls.push((class.into(), f.into(), args.to_vec()));
        match f {
            "Twice" => match args {
                [Value::Int(x)] => Ok(Value::Int(x * 2)),
                _ => Err("bad args".into()),
            },
            _ => Ok(Value::Void),
        }
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        assert_eq!((kind, base), (MemberKind::Thing, &Value::Thing(Some(7))));
        self.members
            .get(&format!("{class}.{field}"))
            .cloned()
            .ok_or_else(|| "no member".into())
    }
    fn set_member(
        &mut self,
        _: MemberKind,
        _: &Value,
        class: &str,
        field: &str,
        v: Value,
    ) -> Result<(), String> {
        self.members.insert(format!("{class}.{field}"), v);
        Ok(())
    }
    fn write(&mut self, text: &str) {
        self.log.push(text.into());
    }
}

/// sum(n) = 1 + 2 + ... + n with a backwards branch. Registers: n@0, i@4, acc@8, c@12.
fn sum_script(from_current: bool) -> Script {
    let back = if from_current { -3i32 } else { -4 };
    Builder::new().build(
        "Sum",
        None,
        vec![func(
            "Sum",
            &[0],
            vec![
                hi(0x03, 4, 1),            // 0 LCi i = 1
                hi(0x03, 8, 0),            // 1 LCi acc = 0
                i(0x2a, 0, 8, 8, 4),       // 2 ADDi acc += i
                i(0x0b, 0, 4, 4, 0),       // 3 INCi i
                i(0x38, 0, 12, 4, 0),      // 4 LTEi c = i <= n
                hi(0xa8, 12, back as u32), // 5 BNEZ c -> 2
                i(0xa5, INT, 0, 8, 0),     // 6 RETURN acc
            ],
        )],
    )
}

#[test]
fn loops_branch_both_rules_and_budget() {
    let mut host = TestHost::default();
    for rule in [true, false] {
        let mut vm = Vm::new(Rules {
            branch_from_current: rule,
            ..Rules::default()
        });
        let c = vm.load(sum_script(rule), None);
        let mut budget = 10_000;
        assert_eq!(
            vm.call(&mut host, c, "Sum", &[Value::Int(10)], &mut budget),
            Ok(Value::Int(55))
        );
        let mut tight = 20;
        assert_eq!(
            vm.call(&mut host, c, "Sum", &[Value::Int(100)], &mut tight),
            Err(Fault::Budget)
        );
    }
    // The census check tells the two encodings apart.
    let a = check(&sum_script(true)).branch;
    assert_eq!(
        (a.branches, a.in_range_from_current, a.in_range_from_next),
        (1, 1, 1)
    );
    let mut far = sum_script(true);
    far.bytecode[5] = hi(0xa8, 12, -6i32 as u32);
    let b = check(&far).branch;
    assert_eq!((b.in_range_from_current, b.in_range_from_next), (0, 1));
}

#[test]
fn script_calls_native_calls_and_returns() {
    let mut b = Builder::new();
    let me = b.ty(OBJ, "Calc", Some(0x100), 0);
    let native = b.ty(OBJ, "Engine", None, 0);
    let add_one = b.fref(me, "AddOne");
    let twice = b.fref(native, "Twice");
    let s = b.build(
        "Calc",
        None,
        vec![
            func(
                "Run",
                &[0],
                vec![
                    i(0xa3, INT, 0, 0, 0),       // ARG @0 = r0
                    i(0xa4, INT, 4, add_one, 0), // r4 = AddOne(r0)
                    i(0xa3, INT, 4, 0, 0),       // ARG @0 = r4
                    i(0xa4, INT, 8, twice, 0),   // r8 = Engine.Twice(r4)
                    i(0xa2, INT, 0, 8, 0),       // WRITE r8
                    i(0xa5, INT, 0, 8, 0),       // RETURN r8
                ],
            ),
            func(
                "AddOne",
                &[0],
                vec![i(0x0b, 0, 0, 0, 0), i(0xa5, INT, 0, 0, 0)],
            ),
        ],
    );
    let mut vm = Vm::default();
    let c = vm.load(s, Some(0x100));
    let mut host = TestHost::default();
    let mut budget = 100;
    assert_eq!(
        vm.call(&mut host, c, "Run", &[Value::Int(20)], &mut budget),
        Ok(Value::Int(42))
    );
    assert_eq!(
        host.calls,
        [("Engine".into(), "Twice".into(), vec![Value::Int(21)])]
    );
    assert_eq!(host.log, ["Int(42)"]);
    assert_eq!(
        vm.call(&mut host, c, "Run", &[], &mut budget),
        Err(Fault::Arity {
            expected: 1,
            got: 0
        })
    );
}

#[test]
fn objects_arrays_vectors_and_thing_members() {
    let mut b = Builder::new();
    let me = b.ty(OBJ, "Bag", Some(0x200), 0);
    let ints = b.ty(OBJ, "int[]", None, INT);
    let int_ty = b.ty(INT, "int", None, 0);
    let count = b.field(int_ty, "count", true);
    let thing_ty = b.ty(10, "Lamp", None, 0);
    let on = b.field(thing_ty, "on", false);
    b.f32s = vec![1., 0., 0., 0., 0., 1., 0., 0.];
    let s = b.build(
        "Bag",
        None,
        vec![func(
            "Run",
            &[0],
            vec![
                i(0xb5, 0, 4, 0, me),       // 0 r4 = new Bag
                hi(0x03, 8, 3),             // 1 r8 = 3
                i(0xb4, INT, 8, 4, count),  // 2 r4.count = r8
                i(0xb3, INT, 12, 4, count), // 3 r12 = r4.count
                hi(0x03, 16, 0),            // 4 r16 = 0
                i(0x9c, 0, 20, 16, ints),   // 5 r20 = new int[r16]
                i(0x9e, INT, 12, 20, 0),    // 6 append r12
                i(0x9e, INT, 8, 20, 0),     // 7 append r8
                i(0x9b, 0, 24, 20, 0),      // 8 r24 = len
                i(0xa0, INT, 28, 20, 8),    // 9 r28 = find r8 -> 0
                hi(0xb8, 32, 0),            // 10 r32 = (1,0,0,0)
                hi(0xb8, 36, 4),            // 11 r36 = (0,1,0,0)
                i(0x50, 0, 40, 32, 36),     // 12 r40 = cross -> (0,0,1,0)
                i(0x5c, 0, 44, 40, 0),      // 13 r44 = r40.z
                i(0x95, 1, 48, 0, on),      // 14 r48 = thing(r0).on
                i(0x97, 1, 44, 0, on),      // 15 thing(r0).on = r44
                i(0xa5, OBJ, 0, 20, 0),     // 16 RETURN array
            ],
        )],
    );
    let mut vm = Vm::default();
    let c = vm.load(s, Some(0x200));
    let mut host = TestHost::default();
    host.members.insert("Lamp.on".into(), Value::Bool(true));
    let mut budget = 100;
    let out = vm
        .call(&mut host, c, "Run", &[Value::Thing(Some(7))], &mut budget)
        .unwrap();
    let Value::Object(Some(id)) = out else {
        panic!("{out:?}")
    };
    assert_eq!(vm.array(id).unwrap(), [Value::Int(3), Value::Int(3)]);
    assert_eq!(host.members["Lamp.on"], Value::Float(1.0));
}

#[test]
fn virtual_calls_dispatch_on_the_receiver_class() {
    // Base.Name returns 1; Derived overrides it with 2. Base.Ask calls Name virtually.
    let mut b = Builder::new();
    let base_ty = b.ty(OBJ, "Base", Some(0x300), 0);
    let name = b.fref(base_ty, "Name");
    let base = b.build(
        "Base",
        None,
        vec![
            func("Name", &[0], vec![hi(0x03, 4, 1), i(0xa5, INT, 0, 4, 0)]),
            func(
                "Ask",
                &[0],
                vec![
                    i(0xa3, OBJ, 0, 0, 0),
                    i(0xc0, INT, 4, name, 0),
                    i(0xa5, INT, 0, 4, 0),
                ],
            ),
        ],
    );
    let derived = Builder::new().build(
        "Derived",
        Some(0x300),
        vec![func(
            "Name",
            &[0],
            vec![hi(0x03, 4, 2), i(0xa5, INT, 0, 4, 0)],
        )],
    );
    let mut vm = Vm::default();
    let base_id = vm.load(base, Some(0x300));
    let derived_id = vm.load(derived, Some(0x301));
    let mut host = TestHost::default();
    let mut budget = 100;
    for (class, want) in [(base_id, 1), (derived_id, 2)] {
        let o = vm.instantiate(class).unwrap();
        // Ask is found on Base for both (inherited).
        assert_eq!(
            vm.call(
                &mut host,
                class,
                "Ask",
                &[Value::Object(Some(o))],
                &mut budget
            ),
            Ok(Value::Int(want))
        );
    }
}

#[test]
fn virtual_calls_on_a_thing_dispatch_on_the_script_it_carries() {
    /// A host that knows the script of some Things.
    struct Scripted(BTreeMap<u32, u32>);
    impl Host for Scripted {
        fn call_native(&mut self, _: &str, _: &str, _: &[Value]) -> Result<Value, String> {
            Ok(Value::Void)
        }
        fn get_member(
            &mut self,
            _: MemberKind,
            _: &Value,
            _: &str,
            _: &str,
        ) -> Result<Value, String> {
            Err("no member".into())
        }
        fn set_member(
            &mut self,
            _: MemberKind,
            _: &Value,
            _: &str,
            _: &str,
            _: Value,
        ) -> Result<(), String> {
            Ok(())
        }
        fn thing_script(&mut self, thing: u32) -> Option<u32> {
            self.0.get(&thing).copied()
        }
    }
    // Base.Name returns 1; Derived overrides it with 2; Other (no relation) has a Name
    // that returns 3. Base.Ask calls Name virtually on the Thing in register 0, and
    // Base.Plain calls it with the plain call.
    const SP: u8 = 10;
    let mut b = Builder::new();
    let base_ty = b.ty(SP, "Base", Some(0x300), 0);
    let name = b.fref(base_ty, "Name");
    let ask = |op: u8| {
        vec![
            i(0xa3, SP, 0, 0, 0),
            i(op, INT, 4, name, 0),
            i(0xa5, INT, 0, 4, 0),
        ]
    };
    let number = |n: u32| vec![hi(0x03, 4, n), i(0xa5, INT, 0, 4, 0)];
    let base = b.build(
        "Base",
        None,
        vec![
            func("Name", &[0], number(1)),
            func("Ask", &[0], ask(0xc1)),
            func("Plain", &[0], ask(0xa4)),
        ],
    );
    let derived = Builder::new().build("Derived", Some(0x300), vec![func("Name", &[0], number(2))]);
    let other = Builder::new().build("Other", None, vec![func("Name", &[0], number(3))]);
    let mut vm = Vm::default();
    let base_id = vm.load(base, Some(0x300));
    vm.load(derived, Some(0x301));
    vm.load(other, Some(0x302));
    // Thing 1 carries Base, 2 Derived, 3 a class that is no Base, 4 a script that is
    // not loaded; Thing 5 is not known to the host.
    let mut host = Scripted([(1, 0x300), (2, 0x301), (3, 0x302), (4, 0x999)].into());
    let mut budget = 1000;
    let mut call = |f: &str, thing: Option<u32>| {
        vm.call(&mut host, base_id, f, &[Value::Thing(thing)], &mut budget)
    };
    for (thing, want) in [(1, 1), (2, 2), (3, 1), (4, 1), (5, 1)] {
        assert_eq!(call("Ask", Some(thing)), Ok(Value::Int(want)), "{thing}");
        // The plain call goes to the class the caller names.
        assert_eq!(call("Plain", Some(thing)), Ok(Value::Int(1)), "{thing}");
    }
    // A null Thing is called as named.
    assert_eq!(call("Ask", None), Ok(Value::Int(1)));
}

#[test]
fn faults_are_explicit() {
    let run = |code: Vec<u64>, stack: i32| {
        let mut f = func("F", &[], code);
        f.stack = stack;
        let mut vm = Vm::default();
        let c = vm.load(Builder::new().build("F", None, vec![f]), None);
        let mut budget = 1000;
        vm.call(&mut TestHost::default(), c, "F", &[], &mut budget)
    };
    assert_eq!(
        run(vec![0xff], 8),
        Err(Fault::UnknownOpcode {
            pc: 0,
            opcode: 0xff
        })
    );
    assert_eq!(
        run(vec![hi(0x03, 9, 1)], 8),
        Err(Fault::Register { pc: 0, register: 9 })
    );
    assert_eq!(
        run(vec![hi(0x03, 0, 1), hi(0x03, 4, 0), i(0x2d, 0, 0, 0, 4)], 8),
        Err(Fault::DivideByZero { pc: 2 })
    );
    assert_eq!(run(vec![hi(0x03, 0, 1)], 8), Err(Fault::Branch { pc: 1 })); // falls off the end
    assert_eq!(run(vec![hi(0xa6, 0, 5)], 8), Err(Fault::Branch { pc: 5 }));
    assert!(matches!(
        run(vec![i(0x0b, 0, 0, 0, 0)], 8),
        Err(Fault::Type { pc: 0, op: "INCi" })
    ));
    assert_eq!(
        run(vec![hi(0xb9, 0, 0), i(0xb3, INT, 4, 0, 0)], 8),
        Err(Fault::Index {
            pc: 1,
            what: "field reference"
        })
    );
    assert_eq!(
        run(vec![hi(0x01, 0, 0), i(0xc2, 0, 0, 0, 0)], 8),
        Err(Fault::Assert { pc: 1 })
    );
    // Unbounded recursion stops at the call-depth limit.
    let mut b = Builder::new();
    let me = b.ty(OBJ, "R", Some(0x400), 0);
    let r = b.fref(me, "F");
    let mut vm = Vm::default();
    let c = vm.load(
        b.build("R", None, vec![func("F", &[], vec![i(0xa4, 0, 0, r, 0)])]),
        Some(0x400),
    );
    let mut budget = 1_000_000;
    assert_eq!(
        vm.call(&mut TestHost::default(), c, "F", &[], &mut budget),
        Err(Fault::CallDepth)
    );
}

#[test]
fn yield_suspends_and_resumes() {
    let mut b = Builder::new();
    let system = b.ty(OBJ, "System", None, 0);
    let yield_ = b.fref(system, "Yield__");
    // count(n): i = 0; do { i++; Yield(); } while (!(n <= i)); return i.
    let s = b.build(
        "Ticker",
        None,
        vec![func(
            "Count",
            &[0],
            vec![
                hi(0x03, 4, 0),            // 0 LCi i = 0
                i(0x0b, 0, 4, 4, 0),       // 1 INCi i
                i(0xa4, 0, 0, yield_, 0),  // 2 CALL System.Yield
                i(0x38, 0, 8, 0, 4),       // 3 LTEi c = n <= i
                hi(0xa7, 8, -3i32 as u32), // 4 BEZ c -> 1
                i(0xa5, INT, 0, 4, 0),     // 5 RETURN i
            ],
        )],
    );
    let mut vm = Vm::default();
    let c = vm.load(s, None);
    let mut host = TestHost::default();
    let mut budget = 1_000;
    let mut step = vm
        .start(&mut host, c, "Count", &[Value::Int(3)], &mut budget)
        .unwrap();
    let mut yields = 0;
    let done = loop {
        match step {
            Step::Yielded(task) => {
                yields += 1;
                step = vm.resume(&mut host, task, &mut budget).unwrap();
            }
            Step::Done(v) => break v,
        }
    };
    assert_eq!((yields, done), (3, Value::Int(3)));
    // Yield never reaches the host, and `call` cannot suspend.
    assert!(host.calls.is_empty());
    assert_eq!(
        vm.call(&mut host, c, "Count", &[Value::Int(3)], &mut budget),
        Err(Fault::YieldInCall)
    );
}

#[test]
fn bindings_run_natives_and_count_unbound_ones() {
    struct NoWorld;
    impl World for NoWorld {}
    let mut b = Builder::new();
    let math = b.ty(OBJ, "Math", None, 0);
    let gooey = b.ty(OBJ, "Gooey", None, 0);
    let floor = b.fref(math, "floor__f");
    let end_frame = b.fref(gooey, "EndFrame__");
    let s = b.build(
        "UsesNatives",
        None,
        vec![
            func(
                "Floor",
                &[0],
                vec![
                    i(0xa3, FLOAT, 0, 0, 0),     // ARG @0 = x
                    i(0xa4, FLOAT, 4, floor, 0), // r4 = Math.floor(x)
                    i(0xa5, FLOAT, 0, 4, 0),     // RETURN r4
                ],
            ),
            func(
                "Ui",
                &[],
                vec![i(0xa4, 0, 0, end_frame, 0), i(0xa5, 0, 0, 0, 0)],
            ),
        ],
    );
    let mut vm = Vm::default();
    let c = vm.load(s, None);
    let mut host = Bindings::new(NoWorld);
    let mut budget = 100;
    assert_eq!(
        vm.call(&mut host, c, "Floor", &[Value::Float(2.75)], &mut budget),
        Ok(Value::Float(2.))
    );
    assert!(matches!(
        vm.call(&mut host, c, "Ui", &[], &mut budget),
        Err(Fault::Native(_))
    ));
    assert_eq!(vm.last_site().unwrap().to_string(), "UsesNatives.Ui+0");
    assert_eq!(host.unbound.get("Gooey.EndFrame__"), Some(&1));
}

#[test]
fn locals_start_at_defaults_and_no_register_operands_are_ignored() {
    let mut b = Builder::new();
    let int_ty = b.ty(INT, "int", None, 0);
    let native = b.ty(OBJ, "Engine", None, 0);
    let twice = b.fref(native, "Twice");
    let mut s = b.build(
        "Locals",
        None,
        vec![func(
            "Run",
            &[],
            vec![
                i(0x0b, 0, 4, 4, 0),            // INCi local (never written) += 1
                i(0xa3, INT, 4, 0, 0),          // ARG @0 = local
                i(0xa4, INT, 0xffff, twice, 0), // typed CALL, result discarded
                hi(0xa6, 0xffff, 1),            // B +1 with the "no register" source
                i(0xa5, INT, 0, 4, 0),          // RETURN local
            ],
        )],
    );
    let name = s.function_defs[0].name;
    s.locals.push(lbp_level::script::LocalVariable {
        modifiers: 0,
        type_ref: u32::from(int_ty),
        name,
        offset: 4,
    });
    s.function_defs[0].locals = Span { begin: 0, end: 1 };
    let mut vm = Vm::default();
    let c = vm.load(s, None);
    let mut host = TestHost::default();
    let mut budget = 100;
    let r = vm.call(&mut host, c, "Run", &[], &mut budget);
    assert_eq!(r, Ok(Value::Int(1)));
    assert_eq!(host.calls.len(), 1);
}

#[test]
fn m44_locals_default_to_zero() {
    // Product of two never-written m44 locals: W.w is 0 with zeroed storage (1 with identity).
    let mut b = Builder::new();
    let m44 = b.ty(6, "m44", None, 0);
    let mut s = b.build(
        "Mats",
        None,
        vec![func(
            "Run",
            &[],
            vec![
                i(0x51, 6, 12, 4, 8),      // MULm44 r12 = r4 * r8
                i(0x70, FLOAT, 16, 12, 0), // GET_M44_WW r16 = r12.ww
                i(0xa5, FLOAT, 0, 16, 0),  // RETURN r16
            ],
        )],
    );
    let name = s.function_defs[0].name;
    for offset in [4, 8] {
        s.locals.push(lbp_level::script::LocalVariable {
            modifiers: 0,
            type_ref: u32::from(m44),
            name,
            offset,
        });
    }
    s.function_defs[0].locals = Span { begin: 0, end: 2 };
    let mut vm = Vm::default();
    let c = vm.load(s, None);
    let mut budget = 100;
    assert_eq!(
        vm.call(&mut TestHost::default(), c, "Run", &[], &mut budget),
        Ok(Value::Float(0.))
    );
}

#[test]
fn new_object_runs_inherited_init_and_finds_superclass_by_learned_guid() {
    // Base (loaded without a GUID) initialises x = 7 in .init__.
    let mut b = Builder::new();
    let int_ty = b.ty(INT, "int", None, 0);
    let x = b.field(int_ty, "x", true);
    let base = b.build(
        "Base",
        None,
        vec![func(
            ".init__",
            &[0],
            vec![
                hi(0x03, 4, 7),        // r4 = 7
                i(0xb4, INT, 4, 0, x), // this.x = r4
                i(0xa5, 0, 0, 0, 0),   // RETURN
            ],
        )],
    );
    // Derived names Base only by GUID; its own type references carry that GUID's name.
    let mut b = Builder::new();
    let me = b.ty(OBJ, "Derived", Some(0x301), 0);
    b.ty(OBJ, "Base", Some(0x300), 0);
    let int_ty = b.ty(INT, "int", None, 0);
    let x = b.field(int_ty, "x", false);
    let derived = b.build(
        "Derived",
        Some(0x300),
        vec![func(
            "Make",
            &[],
            vec![
                i(0xb5, 0, 0, 0, me),  // r0 = new Derived (runs Base.init__)
                i(0xb3, INT, 4, 0, x), // r4 = r0.x
                i(0xa5, INT, 0, 4, 0), // RETURN r4
            ],
        )],
    );
    let mut vm = Vm::default();
    vm.load(base, None);
    let d = vm.load(derived, Some(0x301));
    let mut host = TestHost::default();
    let mut budget = 100;
    assert_eq!(
        vm.call(&mut host, d, "Make", &[], &mut budget),
        Ok(Value::Int(7))
    );
    let o = vm.construct(&mut host, d, &mut budget).unwrap();
    assert_eq!(vm.field(o, "x"), Some(&Value::Int(7)));
}

/// A test-script shape end to end: a class with a GUID-only superclass that has
/// `.init__`, a native `Test.Check` declared on the class, and a non-static test function
/// run on a constructed instance with `this` in register 0. Checks must reach the world.
#[test]
fn test_script_checks_reach_the_world_through_construct() {
    #[derive(Default)]
    struct Recorder(Vec<(bool, i32)>);
    impl World for Recorder {
        fn test_check(&mut self, passed: bool, _message: &str, line: i32) {
            self.0.push((passed, line));
        }
    }
    let mut b = Builder::new();
    let int_ty = b.ty(INT, "int", None, 0);
    let ready = b.field(int_ty, "ready", true);
    let base = b.build(
        "TestBase",
        None,
        vec![func(
            ".init__",
            &[0],
            vec![
                hi(0x03, 4, 1),
                i(0xb4, INT, 4, 0, ready),
                i(0xa5, 0, 0, 0, 0),
            ],
        )],
    );
    let mut b = Builder::new();
    let me = b.ty(OBJ, "Test", Some(0x401), 0);
    b.ty(OBJ, "TestBase", Some(0x400), 0);
    let check = b.fref(me, "Check__bQ7StringAi");
    let msg = b.s("msg");
    let test = b.build(
        "Test",
        Some(0x400),
        vec![
            Func {
                name: "Check__bQ7StringAi",
                native: true,
                args: vec![],
                stack: 0,
                code: vec![],
            },
            func(
                "TestSomething",
                &[],
                vec![
                    hi(0x01, 4, 1),               // LCb r4 = true (value in the high word)
                    hi(0xbc, 8, msg),             // LCsa r8
                    hi(0x03, 12, 42),             // LCi r12 = line
                    i(0xa3, OBJ, 0, 0, 0),        // ARG this
                    i(0xa3, 1, 4, 4, 0),          // ARG r4
                    i(0xa3, OBJ, 8, 8, 0),        // ARG r8
                    i(0xa3, INT, 12, 12, 0),      // ARG r12
                    i(0xa4, 0, 0xffff, check, 0), // CALL Test.Check
                    i(0xa5, 0, 0, 0, 0),          // RETURN
                ],
            ),
        ],
    );
    for (lcb, expect) in [
        (LcbRule::AnyHighBit, true),
        (LcbRule::Bit32, true),
        (LcbRule::Bit63, false),
    ] {
        let mut vm = Vm::new(Rules {
            lcb,
            ..Rules::default()
        });
        vm.load(base.clone(), None);
        let t = vm.load(test.clone(), None);
        assert_eq!(vm.super_chain(t), ["Test", "TestBase"]);
        let mut host = Bindings::new(Recorder::default());
        let mut budget = 1_000;
        let this = vm.construct(&mut host, t, &mut budget).unwrap();
        assert_eq!(vm.field(this, "ready"), Some(&Value::Int(1)));
        let step = vm
            .start_registers(
                &mut host,
                t,
                "TestSomething",
                &[(0, Value::Object(Some(this)))],
                &mut budget,
            )
            .unwrap();
        assert!(matches!(step, Step::Done(_)));
        assert_eq!(host.world.0, [(expect, 42)]);
        assert_eq!(host.calls.get("Test.Check__bQ7StringAi"), Some(&1));
    }
}

#[test]
fn array_resize_after_clear_fills_with_the_element_default() {
    let mut b = Builder::new();
    let ints = b.ty(OBJ, "int[]", None, INT);
    let s = b.build(
        "Arrays",
        None,
        vec![func(
            "Run",
            &[],
            vec![
                hi(0x03, 4, 0),              // r4 = 0 (size)
                i(0x9c, 0, 8, 4, ints),      // r8 = new int[0]
                hi(0x03, 12, 5),             // r12 = 5
                i(0x9e, INT, 12, 8, 0xffff), // append r12
                i(0xa1, INT, 0, 8, 0),       // clear r8
                hi(0x03, 16, 3),             // r16 = 3
                i(0xb6, INT, 0, 8, 16),      // resize r8 to 3
                hi(0x03, 20, 2),             // r20 = 2
                i(0x99, INT, 24, 8, 20),     // r24 = r8[2]
                i(0x3b, 1, 28, 24, 4),       // r28 = r24 == r4 (EQi)
                i(0xa5, 1, 0, 28, 0),        // RETURN r28
            ],
        )],
    );
    let mut vm = Vm::default();
    let c = vm.load(s, None);
    let mut budget = 100;
    assert_eq!(
        vm.call(&mut TestHost::default(), c, "Run", &[], &mut budget),
        Ok(Value::Bool(true))
    );
}

#[test]
fn array_insert_clamps_the_index() {
    let mut b = Builder::new();
    let ints = b.ty(OBJ, "int[]", None, INT);
    let s = b.build(
        "Arrays",
        None,
        vec![func(
            "Run",
            &[],
            vec![
                hi(0x03, 4, 0),              // r4 = 0 (size)
                i(0x9c, 0, 8, 4, ints),      // r8 = new int[0]
                hi(0x03, 12, 5),             // r12 = 5
                i(0x9e, INT, 12, 8, 0xffff), // append r12 (no index operand)
                hi(0x03, 16, -1i32 as u32),  // r16 = -1
                hi(0x03, 20, 8),             // r20 = 8
                i(0x9d, INT, 20, 8, 16),     // insert r20 at r16 = -1: front
                hi(0x03, 16, 100),           // r16 = 100
                hi(0x03, 20, 9),             // r20 = 9
                i(0x9d, INT, 20, 8, 16),     // insert r20 at r16 = 100: end
                hi(0x03, 24, 0),             // r24 = 0
                i(0x99, INT, 28, 8, 24),     // r28 = r8[0] = 8
                hi(0x03, 24, 2),             // r24 = 2
                i(0x99, INT, 32, 8, 24),     // r32 = r8[2] = 9
                hi(0x03, 24, 10),            // r24 = 10
                i(0x2c, INT, 28, 28, 24),    // r28 = r28 * 10 (MULi)
                i(0x2a, INT, 28, 28, 32),    // r28 = r28 + r32 (ADDi)
                i(0xa5, INT, 0, 28, 0),      // RETURN r28
            ],
        )],
    );
    let mut vm = Vm::default();
    let c = vm.load(s, None);
    let mut budget = 100;
    assert_eq!(
        vm.call(&mut TestHost::default(), c, "Run", &[], &mut budget),
        Ok(Value::Int(89))
    );
}
