//! Rq: register VM for compiled Fish scripts (RScript), built on `lbp_level::script`.
//!
//! Runtime crate (AGENTS.md, 2026-10-08 scope): the parsing crates never depend on it.
//! Scripts come from the user's own game data; nothing here is translated from game code.
//! Instruction layouts are the U10 reader's (Toolkit d6a2ef80, CONFIRMED_FROM_COMMUNITY_CODE).
//! Everything about *execution* is INFERRED from mnemonics and operand names and is listed
//! in docs/script-vm.md; the uncertain rules are switchable through [`Rules`] so that a
//! measurement (branch census, RPCS3 comparison) can settle them without code changes.
//!
//! Model: each call gets a frame of `stack_size` register slots addressed by the 16-bit
//! operand (one slot per address; a value never spans slots). `ARG` stages a value at an
//! argument address; `CALL` copies staged values into the callee frame at the same
//! addresses. Native functions, Thing members and native members go to a [`Host`].
use lbp_level::script::{class, Class, Script};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub mod check;
pub mod natives;
pub mod signature;

/// Hard limits; every one fails with a [`Fault`], never a panic.
pub const MAX_CALL_DEPTH: usize = 256;
pub const MAX_HEAP_OBJECTS: usize = 1 << 20;
pub const MAX_ARRAY_LEN: usize = 1 << 20;

/// Operand value meaning "no register" (D138 listings: a discarded CALL result, `B`'s
/// unused source). Reads give `Void`; writes are dropped.
pub const NO_REGISTER: u16 = 0xffff;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u32);

/// One register value. Pointer kinds stay distinct so the host can tell Things
/// (`safe_ptr`) from engine resources (`raw_ptr`/`ref_ptr`) and script objects.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Void,
    Bool(bool),
    Char(u16),
    Int(i32),
    Float(f32),
    Vec4([f32; 4]),
    /// In the engine's memory order: element (row `r`, column `c`) at `c * 4 + r`, the
    /// translation at 12..14 (see [`m44_index`]).
    Mat44([f32; 16]),
    Long(i64),
    Double(f64),
    Str(Arc<str>),
    /// Script object or array on the VM heap; `None` = null.
    Object(Option<ObjectId>),
    /// Thing handle owned by the host; `None` = null.
    Thing(Option<u32>),
    Raw(u32),
    Ref(u32),
}

impl Value {
    fn truthy(&self) -> Option<bool> {
        Some(match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Char(c) => *c != 0,
            Value::Long(l) => *l != 0,
            Value::Object(o) => o.is_some(),
            Value::Thing(t) => t.is_some(),
            _ => return None,
        })
    }
    /// Default for a Toolkit MachineType byte (fields, locals, new arrays). Every type
    /// defaults to zeroed storage, m44 included (INFERRED from the test script, D144:
    /// TestMatrix44 expects a zero W row from the product of two never-written m44 locals;
    /// an identity default gives W.w = 1).
    pub fn default_for(machine_type: u8) -> Value {
        match machine_type {
            1 => Value::Bool(false),
            2 => Value::Char(0),
            3 => Value::Int(0),
            4 => Value::Float(0.),
            5 => Value::Vec4([0.; 4]),
            6 => Value::Mat44([0.; 16]),
            8 => Value::Raw(0),
            9 => Value::Ref(0),
            10 => Value::Thing(None),
            11 => Value::Object(None),
            12 => Value::Long(0),
            13 => Value::Double(0.),
            _ => Value::Void,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Fault {
    UnknownOpcode {
        pc: usize,
        opcode: u8,
    },
    Register {
        pc: usize,
        register: u16,
    },
    Type {
        pc: usize,
        op: &'static str,
    },
    DivideByZero {
        pc: usize,
    },
    Branch {
        pc: usize,
    },
    Index {
        pc: usize,
        what: &'static str,
    },
    NullObject {
        pc: usize,
    },
    MissingClass(String),
    MissingFunction {
        class: String,
        function: String,
    },
    Arity {
        expected: usize,
        got: usize,
    },
    Assert {
        pc: usize,
    },
    Native(String),
    Budget,
    CallDepth,
    Heap,
    /// `System.Yield` reached from [`Vm::call`], which cannot suspend; use [`Vm::start`].
    YieldInCall,
}

/// An instruction's place: class, function and pc relative to the function's first
/// instruction (as listings number them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub class: String,
    pub function: String,
    pub pc: usize,
}

impl std::fmt::Display for Site {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}+{}", self.class, self.function, self.pc)
    }
}

/// Result of [`Vm::start`] / [`Vm::resume`].
#[derive(Debug)]
pub enum Step {
    Done(Value),
    /// `System.Yield` suspended the script; resume it on a later tick.
    Yielded(Task),
}

/// A suspended script call (its frames), resumed with [`Vm::resume`].
#[derive(Debug)]
pub struct Task {
    stack: Vec<Frame>,
}

/// Which member access reaches the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    /// `GET/SET_SP_MEMBER`: script member of a Thing (PScript instance data).
    Thing,
    /// `GET/SET_RP_MEMBER`.
    RefPtr,
    /// `GET_SP_NATIVE_MEMBER`: engine field of a Thing.
    Native,
}

/// Engine side of the VM: native functions and members, by name.
pub trait Host {
    fn call_native(&mut self, class: &str, function: &str, args: &[Value])
        -> Result<Value, String>;
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String>;
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String>;
    /// `CASTsp` on a Thing: does it carry a script of this class?
    fn thing_is(&mut self, _thing: u32, _class: &str) -> bool {
        true
    }
    /// The GUID of the script resource a Thing carries, for a virtual call on the Thing
    /// (`CALLVsp`). `None`: not known, and the call goes to the class the caller names.
    fn thing_script(&mut self, _thing: u32) -> Option<u32> {
        None
    }
    /// D326: a native that makes a script object (the engine's factory, e.g.
    /// `SearchDetails.CreateStoryLevels__`, whose declared return type is a script class
    /// with code): the class to make. The VM then makes it as `NEW_OBJECT` does and runs
    /// its `.init__` and `.ctor__`, and the call answers the new object. `None`: an ordinary
    /// native call.
    fn native_object_class(&mut self, _class: &str, _function: &str) -> Option<&'static str> {
        None
    }
    fn write(&mut self, _text: &str) {}
}

/// `LCb` constant position: Toolkit's bit 63 (default), any high-word bit (D140: broke
/// TestLogical), or bit 32 (the low bit of the high word, where `LCi` keeps its value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LcbRule {
    Bit63,
    AnyHighBit,
    Bit32,
}

/// Uncertain execution rules (docs/script-vm.md). Defaults are the current best guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rules {
    /// Branch target = `pc + offset` (true) or `pc + 1 + offset` (false).
    pub branch_from_current: bool,
    /// Which bits of an `LCb` instruction hold the constant.
    pub lcb: LcbRule,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            branch_from_current: true,
            lcb: LcbRule::Bit63,
        }
    }
}

struct LoadedClass {
    script: Script,
    name: String,
    guid: Option<u32>,
    /// Function name -> function definition index (first definition wins).
    functions: HashMap<String, usize>,
}

enum HeapObject {
    Instance {
        class: ClassId,
        fields: BTreeMap<String, Value>,
    },
    Array(Vec<Value>),
}

#[derive(Debug)]
struct Frame {
    class: ClassId,
    function: usize,
    pc: usize,
    regs: Vec<Value>,
    /// Caller register receiving the return value.
    dest: Option<u16>,
}

pub struct Vm {
    pub rules: Rules,
    classes: Vec<LoadedClass>,
    by_guid: HashMap<u32, ClassId>,
    by_name: HashMap<String, ClassId>,
    /// GUID -> class name, learned from every loaded script's type references, so a
    /// superclass named only by GUID resolves when its script was loaded without one.
    guid_names: HashMap<u32, String>,
    heap: Vec<HeapObject>,
    /// Calls into functions that matched more than one definition by name.
    pub ambiguous_calls: usize,
    /// (class, function, absolute pc) of the instruction being executed.
    at: Option<(ClassId, usize, usize)>,
    /// Thread objects passed to `Thread.StartImpl__`, for the host's scheduler
    /// ([`Vm::take_started_threads`]).
    started_threads: Vec<ObjectId>,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new(Rules::default())
    }
}

impl Vm {
    pub fn new(rules: Rules) -> Self {
        Self {
            rules,
            classes: Vec::new(),
            by_guid: HashMap::new(),
            by_name: HashMap::new(),
            guid_names: HashMap::new(),
            heap: Vec::new(),
            ambiguous_calls: 0,
            at: None,
            started_threads: Vec::new(),
        }
    }

    /// Registers a script class. A GUID lets other scripts' type references find it.
    pub fn load(&mut self, script: Script, guid: Option<u32>) -> ClassId {
        let id = ClassId(self.classes.len() as u32);
        let mut functions = HashMap::new();
        for (i, f) in script.function_defs.iter().enumerate() {
            if let Some(name) = script.name(f.name) {
                functions.entry(name.to_owned()).or_insert(i);
            }
        }
        let name = script.class_name.clone();
        if let Some(g) = guid {
            self.by_guid.insert(g, id);
        }
        self.by_name.entry(name.clone()).or_insert(id);
        for t in &script.type_refs {
            if let (Some(g), Some(n)) = (t.script.and_then(|d| d.guid), script.name(t.type_name)) {
                self.guid_names.entry(g).or_insert_with(|| n.to_owned());
            }
        }
        self.classes.push(LoadedClass {
            script,
            name,
            guid,
            functions,
        });
        id
    }

    pub fn class_by_name(&self, name: &str) -> Option<ClassId> {
        self.by_name.get(name).copied()
    }

    pub fn class_by_guid(&self, guid: u32) -> Option<ClassId> {
        self.by_guid.get(&guid).copied()
    }

    /// Field names of a class and its superclasses (the layout an instance gets).
    pub fn field_names(&self, class: ClassId) -> Vec<String> {
        let mut out = Vec::new();
        let mut at = Some(class);
        let mut depth = 0;
        while let Some(c) = at {
            let s = &self.classes[c.0 as usize].script;
            out.extend(
                s.field_defs
                    .iter()
                    .filter_map(|f| s.name(f.name).map(str::to_owned)),
            );
            at = self.super_class(c);
            depth += 1;
            if depth > MAX_CALL_DEPTH {
                break;
            }
        }
        out
    }

    /// Names of the class's own functions that have code, sorted.
    pub fn function_names(&self, class: ClassId) -> Vec<&str> {
        let c = &self.classes[class.0 as usize];
        let mut out: Vec<&str> = c
            .functions
            .iter()
            .filter(|(_, &i)| {
                let f = &c.script.function_defs[i];
                f.bytecode.begin != f.bytecode.end
            })
            .map(|(n, _)| n.as_str())
            .collect();
        out.sort_unstable();
        out
    }

    pub fn class_name(&self, id: ClassId) -> &str {
        &self.classes[id.0 as usize].name
    }

    pub fn class_guid(&self, id: ClassId) -> Option<u32> {
        self.classes[id.0 as usize].guid
    }

    /// Field of a script object (for hosts and tests).
    pub fn field(&self, object: ObjectId, name: &str) -> Option<&Value> {
        match self.heap.get(object.0 as usize)? {
            HeapObject::Instance { fields, .. } => fields.get(name),
            HeapObject::Array(_) => None,
        }
    }

    /// Sets an existing field of a script object (for hosts that supply engine-owned
    /// objects). False when the object or field does not exist.
    pub fn set_field(&mut self, object: ObjectId, name: &str, value: Value) -> bool {
        match self.heap.get_mut(object.0 as usize) {
            Some(HeapObject::Instance { fields, .. }) => match fields.get_mut(name) {
                Some(v) => {
                    *v = value;
                    true
                }
                None => false,
            },
            _ => false,
        }
    }

    /// Null `object_ref` fields of `object` (its class and superclasses) whose declared
    /// type is a loaded script class, by GUID or name, in declaration order.
    pub fn null_script_fields(&self, object: ObjectId) -> Vec<(String, ClassId)> {
        let Some(HeapObject::Instance { class, fields }) = self.heap.get(object.0 as usize) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut at = Some(*class);
        for _ in 0..MAX_CALL_DEPTH {
            let Some(c) = at else { break };
            let s = &self.classes[c.0 as usize].script;
            for f in &s.field_defs {
                let (Some(name), Some(t)) = (s.name(f.name), s.type_refs.get(f.type_ref as usize))
                else {
                    continue;
                };
                if t.machine_type != 11 || fields.get(name) != Some(&Value::Object(None)) {
                    continue;
                }
                let by_guid = t
                    .script
                    .and_then(|d| d.guid)
                    .and_then(|g| self.by_guid.get(&g).copied());
                let by_name = || {
                    s.name(t.type_name)
                        .and_then(|n| self.by_name.get(n).copied())
                };
                if let Some(k) = by_guid.or_else(by_name) {
                    if !out.iter().any(|(n, _)| n == name) {
                        out.push((name.to_owned(), k));
                    }
                }
            }
            at = self.super_class(c);
        }
        out
    }

    /// Thread objects started since the last call, in start order.
    pub fn take_started_threads(&mut self) -> Vec<ObjectId> {
        std::mem::take(&mut self.started_threads)
    }

    /// Class of a script instance (None for arrays and unknown objects).
    pub fn class_of(&self, object: ObjectId) -> Option<ClassId> {
        match self.heap.get(object.0 as usize)? {
            HeapObject::Instance { class, .. } => Some(*class),
            HeapObject::Array(_) => None,
        }
    }

    pub fn array(&self, object: ObjectId) -> Option<&[Value]> {
        match self.heap.get(object.0 as usize)? {
            HeapObject::Array(v) => Some(v),
            HeapObject::Instance { .. } => None,
        }
    }

    /// Mutable elements of a script array (for hosts that load stored objects).
    pub fn array_mut(&mut self, object: ObjectId) -> Option<&mut Vec<Value>> {
        match self.heap.get_mut(object.0 as usize)? {
            HeapObject::Array(v) => Some(v),
            HeapObject::Instance { .. } => None,
        }
    }

    /// Mutable fields of a script instance, including fields its class does not declare
    /// (for hosts that load stored objects).
    pub fn fields_mut(&mut self, object: ObjectId) -> Option<&mut BTreeMap<String, Value>> {
        match self.heap.get_mut(object.0 as usize)? {
            HeapObject::Instance { fields, .. } => Some(fields),
            HeapObject::Array(_) => None,
        }
    }

    /// Creates a script array holding `items`.
    pub fn new_array(&mut self, items: Vec<Value>) -> Result<ObjectId, Fault> {
        self.alloc(HeapObject::Array(items))
    }

    /// Creates an instance of a loaded class with default field values.
    pub fn instantiate(&mut self, class: ClassId) -> Result<ObjectId, Fault> {
        let fields = self.field_defaults(class)?;
        self.alloc(HeapObject::Instance { class, fields })
    }

    /// Every field of a class and its superclasses at the default of its machine type (the
    /// nearest declaration wins). A scripted Thing's members start from these too.
    pub fn field_defaults(&self, class: ClassId) -> Result<BTreeMap<String, Value>, Fault> {
        let mut fields = BTreeMap::new();
        let mut at = Some(class);
        let mut depth = 0;
        while let Some(c) = at {
            let s = &self.classes[c.0 as usize].script;
            for f in &s.field_defs {
                let ty = s
                    .type_refs
                    .get(f.type_ref as usize)
                    .map_or(0, |t| t.machine_type);
                if let Some(name) = s.name(f.name) {
                    fields
                        .entry(name.to_owned())
                        .or_insert_with(|| Value::default_for(ty));
                }
            }
            at = self.super_class(c);
            depth += 1;
            if depth > MAX_CALL_DEPTH {
                return Err(Fault::MissingClass("superclass cycle".into()));
            }
        }
        Ok(fields)
    }

    fn alloc(&mut self, o: HeapObject) -> Result<ObjectId, Fault> {
        if self.heap.len() >= MAX_HEAP_OBJECTS {
            return Err(Fault::Heap);
        }
        self.heap.push(o);
        Ok(ObjectId(self.heap.len() as u32 - 1))
    }

    fn super_class(&self, c: ClassId) -> Option<ClassId> {
        let g = self.classes[c.0 as usize].script.super_class?.guid?;
        self.by_guid
            .get(&g)
            .copied()
            .or_else(|| {
                self.guid_names
                    .get(&g)
                    .and_then(|n| self.by_name.get(n))
                    .copied()
            })
            .filter(|&s| s != c)
    }

    /// `class` and its superclasses, nearest first.
    pub fn class_chain(&self, class: ClassId) -> Vec<ClassId> {
        let mut out = Vec::new();
        let mut at = Some(class);
        while let Some(c) = at {
            if out.len() > MAX_CALL_DEPTH {
                break;
            }
            out.push(c);
            at = self.super_class(c);
        }
        out
    }

    /// Class names from `class` up its superclass chain (for diagnostics).
    pub fn super_chain(&self, class: ClassId) -> Vec<String> {
        self.class_chain(class)
            .into_iter()
            .map(|c| self.class_name(c).to_owned())
            .collect()
    }

    /// Frame running `.init__` (field initialisers) on a new object, if the class or a
    /// superclass has one. `this` goes to the function's single argument row, else
    /// register 0 (INFERRED, docs/script-vm.md).
    fn init_frame(&self, class: ClassId, this: ObjectId) -> Result<Option<Frame>, Fault> {
        self.this_frame(class, this, ".init__")
    }

    /// Frame running a function of no arguments but `this` (`.init__`, `.ctor__`) on an
    /// object, if the class or a superclass has it with code.
    fn this_frame(
        &self,
        class: ClassId,
        this: ObjectId,
        name: &str,
    ) -> Result<Option<Frame>, Fault> {
        let Some((c, f)) = self.find(class, name) else {
            return Ok(None);
        };
        let s = &self.classes[c.0 as usize].script;
        let def = &s.function_defs[f];
        let rows = s
            .shared_arguments
            .get(def.arguments.begin as usize..def.arguments.end as usize)
            .unwrap_or(&[]);
        let at = match rows {
            [row] => u16::try_from(row.offset).unwrap_or(0),
            _ => 0,
        };
        let staged = BTreeMap::from([(at, Value::Object(Some(this)))]);
        self.frame(c, f, staged, None).map(Some)
    }

    /// `NEW_OBJECT` from the host: allocates the fields of the class and its superclasses
    /// and runs `.init__`. The constructor (`.ctor__`) is the caller's to run, as scripts do.
    pub fn construct(
        &mut self,
        host: &mut dyn Host,
        class: ClassId,
        budget: &mut u64,
    ) -> Result<ObjectId, Fault> {
        let id = self.instantiate(class)?;
        if let Some(frame) = self.init_frame(class, id)? {
            if let Step::Yielded(_) = self.run(host, vec![frame], budget)? {
                return Err(Fault::YieldInCall);
            }
        }
        Ok(id)
    }

    /// Finds a function with code by name in a class or its superclasses.
    fn find(&self, class: ClassId, name: &str) -> Option<(ClassId, usize)> {
        let mut at = Some(class);
        for _ in 0..MAX_CALL_DEPTH {
            let c = at?;
            let lc = &self.classes[c.0 as usize];
            if let Some(&i) = lc.functions.get(name) {
                let f = &lc.script.function_defs[i];
                // Native functions have no bytecode (modifier bit 1).
                if f.modifiers & 2 == 0 {
                    return Some((c, i));
                }
                return None;
            }
            at = self.super_class(c);
        }
        None
    }

    /// The class in `class`'s superclass chain that declares `name` as a native (no
    /// bytecode, modifier bit 1). A call through a subclass's function reference names the
    /// subclass (`Score_Board.GetShapeEditorColour__Q5Thing`), while the declaration is on
    /// `Thing`.
    fn native_owner(&self, class: ClassId, name: &str) -> Option<ClassId> {
        let mut at = Some(class);
        for _ in 0..MAX_CALL_DEPTH {
            let c = at?;
            let lc = &self.classes[c.0 as usize];
            if let Some(&i) = lc.functions.get(name) {
                return (lc.script.function_defs[i].modifiers & 2 != 0).then_some(c);
            }
            at = self.super_class(c);
        }
        None
    }

    /// Calls `function` on `class` from the host. `args` fill the function's argument
    /// rows in order (receiver first for member functions, INFERRED). A script that
    /// yields fails with [`Fault::YieldInCall`]; coroutines go through [`Vm::start`].
    pub fn call(
        &mut self,
        host: &mut dyn Host,
        class: ClassId,
        function: &str,
        args: &[Value],
        budget: &mut u64,
    ) -> Result<Value, Fault> {
        match self.start(host, class, function, args, budget)? {
            Step::Done(v) => Ok(v),
            Step::Yielded(_) => Err(Fault::YieldInCall),
        }
    }

    /// Like [`Vm::call`], but `System.Yield` suspends the call and returns a [`Task`].
    pub fn start(
        &mut self,
        host: &mut dyn Host,
        class: ClassId,
        function: &str,
        args: &[Value],
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        let (c, f) = self
            .find(class, function)
            .ok_or_else(|| Fault::MissingFunction {
                class: self.class_name(class).to_owned(),
                function: function.to_owned(),
            })?;
        let s = &self.classes[c.0 as usize].script;
        let def = &s.function_defs[f];
        let rows = s
            .shared_arguments
            .get(def.arguments.begin as usize..def.arguments.end as usize)
            .ok_or(Fault::Index {
                pc: 0,
                what: "argument rows",
            })?;
        if rows.len() != args.len() {
            return Err(Fault::Arity {
                expected: rows.len(),
                got: args.len(),
            });
        }
        let mut staged = BTreeMap::new();
        for (row, v) in rows.iter().zip(args) {
            let at = u16::try_from(row.offset).map_err(|_| Fault::Index {
                pc: 0,
                what: "argument offset",
            })?;
            staged.insert(at, v.clone());
        }
        let frame = self.frame(c, f, staged, None)?;
        self.run(host, vec![frame], budget)
    }

    /// Continues a task after the instruction that yielded.
    pub fn resume(
        &mut self,
        host: &mut dyn Host,
        task: Task,
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        self.run(host, task.stack, budget)
    }

    /// Where the last executed instruction was; after a [`Fault`] this is the faulting
    /// instruction (the budget fault names the next one).
    pub fn last_site(&self) -> Option<Site> {
        let (c, f, pc) = self.at?;
        let lc = &self.classes[c.0 as usize];
        let def = lc.script.function_defs.get(f)?;
        Some(Site {
            class: lc.name.clone(),
            function: lc.script.name(def.name).unwrap_or("").to_owned(),
            pc: pc.saturating_sub(def.bytecode.begin as usize),
        })
    }

    /// Does `function` resolve to script code in `class` or a superclass? `false` means
    /// a call to it goes to the host as a native.
    pub fn has_code(&self, class: ClassId, function: &str) -> bool {
        self.find(class, function).is_some()
    }

    /// Starts `function` with registers staged directly (`(register, value)` pairs), for
    /// hosts that place the receiver themselves. Registers past the stack size fault.
    pub fn start_registers(
        &mut self,
        host: &mut dyn Host,
        class: ClassId,
        function: &str,
        registers: &[(u16, Value)],
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        let (c, f) = self
            .find(class, function)
            .ok_or_else(|| Fault::MissingFunction {
                class: self.class_name(class).to_owned(),
                function: function.to_owned(),
            })?;
        let staged = registers.iter().cloned().collect();
        let frame = self.frame(c, f, staged, None)?;
        self.run(host, vec![frame], budget)
    }

    fn frame(
        &self,
        class: ClassId,
        function: usize,
        staged: BTreeMap<u16, Value>,
        dest: Option<u16>,
    ) -> Result<Frame, Fault> {
        let script = &self.classes[class.0 as usize].script;
        let def = &script.function_defs[function];
        let size = usize::try_from(def.stack_size).unwrap_or(0).min(1 << 16);
        let mut regs = vec![Value::Void; size];
        // Declared locals start at their type's default (INFERRED from the D138 test
        // script: reads of never-written locals).
        let locals = script
            .locals
            .get(def.locals.begin as usize..def.locals.end as usize)
            .unwrap_or(&[]);
        for l in locals {
            let ty = script
                .type_refs
                .get(l.type_ref as usize)
                .map_or(0, |t| t.machine_type);
            if let Some(slot) = usize::try_from(l.offset).ok().and_then(|o| regs.get_mut(o)) {
                *slot = Value::default_for(ty);
            }
        }
        for (at, v) in staged {
            *regs.get_mut(usize::from(at)).ok_or(Fault::Register {
                pc: def.bytecode.begin as usize,
                register: at,
            })? = v;
        }
        Ok(Frame {
            class,
            function,
            pc: def.bytecode.begin as usize,
            regs,
            dest,
        })
    }

    fn run(
        &mut self,
        host: &mut dyn Host,
        mut stack: Vec<Frame>,
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        let mut staged: BTreeMap<u16, Value> = BTreeMap::new();
        loop {
            if *budget == 0 {
                return Err(Fault::Budget);
            }
            *budget -= 1;
            let depth = stack.len();
            let fr = stack.last_mut().expect("non-empty stack");
            self.at = Some((fr.class, fr.function, fr.pc));
            let script = &self.classes[fr.class.0 as usize].script;
            let def = &script.function_defs[fr.function];
            let pc = fr.pc;
            if pc < def.bytecode.begin as usize || pc >= def.bytecode.end as usize {
                return Err(Fault::Branch { pc });
            }
            let bits = *script.bytecode.get(pc).ok_or(Fault::Branch { pc })?;
            let op = (bits & 0xff) as u8;
            let ty = ((bits >> 8) & 0xff) as u8;
            let r16 = |s: u32| ((bits >> s) & 0xffff) as u16;
            let hi32 = (bits >> 32) as u32;
            let cls = class(op).ok_or(Fault::UnknownOpcode { pc, opcode: op })?;
            let mut next = pc + 1;
            macro_rules! get {
                ($r:expr) => {{
                    let r: u16 = $r;
                    if r == NO_REGISTER {
                        Value::Void
                    } else {
                        fr.regs
                            .get(usize::from(r))
                            .cloned()
                            .ok_or(Fault::Register { pc, register: r })?
                    }
                }};
            }
            macro_rules! set {
                ($r:expr, $v:expr) => {{
                    let r: u16 = $r;
                    let v = $v;
                    if r != NO_REGISTER {
                        *fr.regs
                            .get_mut(usize::from(r))
                            .ok_or(Fault::Register { pc, register: r })? = v;
                    }
                }};
            }
            let mismatch = || Fault::Type {
                pc,
                op: lbp_level::script::MNEMONICS[op as usize],
            };
            match cls {
                Class::Nop => {}
                Class::LoadConst => {
                    let v = match op {
                        // Toolkit reads bit 63; D138 showed `true` constants reading false,
                        // so any set bit in the high word counts (INFERRED).
                        0x01 => Value::Bool(match self.rules.lcb {
                            LcbRule::Bit63 => bits >> 63 != 0,
                            LcbRule::AnyHighBit => hi32 != 0,
                            LcbRule::Bit32 => hi32 & 1 != 0,
                        }),
                        0x02 => Value::Char(r16(48)),
                        0x03 => Value::Int(hi32 as i32),
                        0x04 => Value::Float(f32::from_bits(hi32)),
                        0x05 => Value::Str(
                            script
                                .strings_w
                                .get(hi32 as usize)
                                .ok_or(Fault::Index {
                                    pc,
                                    what: "wide string",
                                })?
                                .as_str()
                                .into(),
                        ),
                        0xbc => Value::Str(
                            script
                                .strings_a
                                .get(hi32 as usize)
                                .ok_or(Fault::Index { pc, what: "string" })?
                                .as_str()
                                .into(),
                        ),
                        0x06 => Value::Thing(None),
                        0xb9 => Value::Object(None),
                        0xc3 => Value::Long(
                            *script
                                .constants_s64
                                .as_ref()
                                .and_then(|c| c.get(hi32 as usize))
                                .ok_or(Fault::Index {
                                    pc,
                                    what: "s64 constant",
                                })?,
                        ),
                        // LCv4: four consecutive f32 constants (INFERRED).
                        0xb8 => {
                            let i = hi32 as usize;
                            let c = script
                                .constants_f32
                                .get(i..i.checked_add(4).ok_or(mismatch())?)
                                .ok_or(Fault::Index {
                                    pc,
                                    what: "f32 constants",
                                })?;
                            Value::Vec4([c[0], c[1], c[2], c[3]])
                        }
                        _ => return Err(mismatch()),
                    };
                    set!(r16(16), v);
                }
                Class::Unary => {
                    let v = unary(op, get!(r16(32))).ok_or_else(mismatch)?;
                    set!(r16(16), v);
                }
                Class::Binary => {
                    let v = binary(op, get!(r16(32)), get!(r16(48)))
                        .map_err(|e| e.unwrap_or_else(mismatch))?;
                    let v = match v {
                        Err(()) => return Err(Fault::DivideByZero { pc }),
                        Ok(v) => v,
                    };
                    set!(r16(16), v);
                }
                Class::GetBuiltinMember => {
                    let base = get!(r16(32));
                    let v = if op == 0x9b {
                        match base {
                            Value::Object(Some(o)) => match self.heap.get(o.0 as usize) {
                                Some(HeapObject::Array(a)) => Value::Int(a.len() as i32),
                                _ => return Err(mismatch()),
                            },
                            Value::Object(None) => return Err(Fault::NullObject { pc }),
                            _ => return Err(mismatch()),
                        }
                    } else {
                        get_builtin(op, &base).ok_or_else(mismatch)?
                    };
                    set!(r16(16), v);
                }
                Class::SetBuiltinMember => {
                    let src = get!(r16(16));
                    let base = r16(32);
                    let cur = get!(base);
                    set!(base, set_builtin(op, cur, src).ok_or_else(mismatch)?);
                }
                Class::GetMember | Class::SetMember => {
                    let base = get!(r16(32));
                    let (owner, field) = member_name(script, r16(48)).ok_or(Fault::Index {
                        pc,
                        what: "field reference",
                    })?;
                    let get = cls == Class::GetMember;
                    let kind = match op {
                        0x95 | 0x97 => Some(MemberKind::Thing),
                        0x96 | 0x98 => Some(MemberKind::RefPtr),
                        0xbb => Some(MemberKind::Native),
                        _ => None, // 0xb3 / 0xb4: script object member
                    };
                    if let Some(kind) = kind {
                        if get {
                            let v = host
                                .get_member(kind, &base, &owner, &field)
                                .map_err(Fault::Native)?;
                            set!(r16(16), v);
                        } else {
                            let v = get!(r16(16));
                            host.set_member(kind, &base, &owner, &field, v)
                                .map_err(Fault::Native)?;
                        }
                    } else {
                        let Value::Object(Some(o)) = base else {
                            return Err(if base == Value::Object(None) {
                                Fault::NullObject { pc }
                            } else {
                                mismatch()
                            });
                        };
                        let fields = match self.heap.get_mut(o.0 as usize) {
                            Some(HeapObject::Instance { fields, .. }) => fields,
                            _ => return Err(mismatch()),
                        };
                        if get {
                            let v = fields.get(&field).cloned().ok_or(Fault::Index {
                                pc,
                                what: "object field",
                            })?;
                            set!(r16(16), v);
                        } else {
                            let v = get!(r16(16));
                            fields.insert(field, v);
                        }
                    }
                }
                Class::GetElement => {
                    let (o, arg) = (get!(r16(32)), get!(r16(48)));
                    let a =
                        array_ref(&self.heap, &o, pc).map_err(|e| e.unwrap_or_else(mismatch))?;
                    let v = if op == 0xa0 {
                        Value::Int(a.iter().position(|x| *x == arg).map_or(-1, |i| i as i32))
                    } else {
                        let Value::Int(i) = arg else {
                            return Err(mismatch());
                        };
                        a.get(usize::try_from(i).map_err(|_| Fault::Index {
                            pc,
                            what: "element",
                        })?)
                        .cloned()
                        .ok_or(Fault::Index {
                            pc,
                            what: "element",
                        })
                        .map(|v| typed(v, ty))?
                    };
                    set!(r16(16), v);
                }
                Class::SetElement => {
                    let o = get!(r16(32));
                    let index = get!(r16(48));
                    let src = typed(get!(r16(16)), ty);
                    let Value::Object(Some(id)) = o else {
                        return Err(if o == Value::Object(None) {
                            Fault::NullObject { pc }
                        } else {
                            mismatch()
                        });
                    };
                    let Some(HeapObject::Array(a)) = self.heap.get_mut(id.0 as usize) else {
                        return Err(mismatch());
                    };
                    let idx = |v: &Value| match v {
                        Value::Int(i) => usize::try_from(*i).ok(),
                        _ => None,
                    };
                    let bad = Fault::Index {
                        pc,
                        what: "element",
                    };
                    match op {
                        0x9a => *a.get_mut(idx(&index).ok_or(bad.clone())?).ok_or(bad)? = src,
                        0x9d => {
                            // The index is clamped to [0, len]: -1 inserts at the front and
                            // an index past the end appends (INFERRED from the test script's
                            // TestArray, D145: inserts at 1, -1 and 100 then length and
                            // element checks).
                            let i = match index {
                                Value::Int(i) => usize::try_from(i).map_or(0, |i| i.min(a.len())),
                                _ => return Err(bad),
                            };
                            if a.len() >= MAX_ARRAY_LEN {
                                return Err(Fault::Heap);
                            }
                            a.insert(i, src);
                        }
                        0x9e => {
                            if a.len() >= MAX_ARRAY_LEN {
                                return Err(Fault::Heap);
                            }
                            a.push(src);
                        }
                        0x9f => {
                            let i = idx(&index).filter(|&i| i < a.len()).ok_or(bad)?;
                            a.remove(i);
                        }
                        0xa1 => a.clear(),
                        0xb6 => {
                            let n = idx(&index)
                                .filter(|&n| n <= MAX_ARRAY_LEN)
                                .ok_or(Fault::Heap)?;
                            let fill = typed(a.first().map_or(Value::Void, zero_like), ty);
                            a.resize(n, fill);
                        }
                        0xb7 => {} // ARRAY_RESERVE: capacity hint only
                        _ => return Err(mismatch()),
                    }
                }
                Class::NewArray => {
                    let Value::Int(n) = get!(r16(32)) else {
                        return Err(mismatch());
                    };
                    let n = usize::try_from(n)
                        .ok()
                        .filter(|&n| n <= MAX_ARRAY_LEN)
                        .ok_or(Fault::Heap)?;
                    let t = script
                        .type_refs
                        .get(usize::from(r16(48)))
                        .ok_or(Fault::Index {
                            pc,
                            what: "type reference",
                        })?;
                    let elem = Value::default_for(t.array_base_machine_type);
                    let id = self.alloc(HeapObject::Array(vec![elem; n]))?;
                    let fr = stack.last_mut().expect("frame");
                    *fr.regs
                        .get_mut(usize::from(r16(16)))
                        .ok_or(Fault::Register {
                            pc,
                            register: r16(16),
                        })? = Value::Object(Some(id));
                    fr.pc = next;
                    continue;
                }
                Class::NewObject => {
                    let target = self.type_class(script, r16(48), pc)?;
                    let id = self.instantiate(target)?;
                    let init = self.init_frame(target, id)?;
                    let fr = stack.last_mut().expect("frame");
                    *fr.regs
                        .get_mut(usize::from(r16(16)))
                        .ok_or(Fault::Register {
                            pc,
                            register: r16(16),
                        })? = Value::Object(Some(id));
                    fr.pc = next;
                    if let Some(frame) = init {
                        if depth >= MAX_CALL_DEPTH {
                            return Err(Fault::CallDepth);
                        }
                        stack.push(frame);
                    }
                    continue;
                }
                Class::Write => {
                    let v = get!(r16(32));
                    if op == 0xc2 {
                        if v.truthy() != Some(true) {
                            return Err(Fault::Assert { pc });
                        }
                    } else {
                        host.write(&format!("{v:?}"));
                    }
                }
                Class::Arg => {
                    staged.insert(r16(32), get!(r16(16)));
                }
                Class::Cast => {
                    let v = get!(r16(32));
                    let target =
                        script
                            .type_refs
                            .get(usize::from(r16(48)))
                            .ok_or(Fault::Index {
                                pc,
                                what: "type reference",
                            })?;
                    let name = script.name(target.type_name).unwrap_or("").to_owned();
                    let out = match v {
                        Value::Thing(Some(t)) if op == 0xa9 => {
                            if host.thing_is(t, &name) {
                                v
                            } else {
                                Value::Thing(None)
                            }
                        }
                        Value::Object(Some(o)) if op == 0xba => {
                            let ok = match self.heap.get(o.0 as usize) {
                                Some(HeapObject::Instance { class, .. }) => {
                                    self.is_a(*class, &name)
                                }
                                _ => false,
                            };
                            if ok {
                                v
                            } else {
                                Value::Object(None)
                            }
                        }
                        Value::Thing(None) | Value::Object(None) => v,
                        _ => return Err(mismatch()),
                    };
                    set!(r16(16), out);
                }
                Class::Call => {
                    let dest = (ty != 0 && r16(16) != NO_REGISTER).then(|| r16(16));
                    let (owner, function, owner_class) =
                        {
                            let fref = script.function_refs.get(usize::from(r16(32))).ok_or(
                                Fault::Index {
                                    pc,
                                    what: "function reference",
                                },
                            )?;
                            let name = script.name(fref.name).ok_or(Fault::Index {
                                pc,
                                what: "function name",
                            })?;
                            let t = script.type_refs.get(fref.type_ref as usize);
                            let owner = t
                                .and_then(|t| script.name(t.type_name))
                                .unwrap_or("")
                                .to_owned();
                            let owner_class = t
                                .and_then(|t| t.script)
                                .and_then(|d| d.guid)
                                .and_then(|g| self.by_guid.get(&g).copied())
                                .or_else(|| self.by_name.get(&owner).copied());
                            (owner, name.to_owned(), owner_class)
                        };
                    let args = std::mem::take(&mut staged);
                    // Virtual calls dispatch on the receiver: the lowest argument address.
                    let receiver_class = if op == 0xc0 {
                        match args.values().next() {
                            Some(Value::Object(Some(o))) => match self.heap.get(o.0 as usize) {
                                Some(HeapObject::Instance { class, .. }) => Some(*class),
                                _ => None,
                            },
                            Some(Value::Object(None)) => return Err(Fault::NullObject { pc }),
                            _ => None,
                        }
                    } else if op == 0xc1 {
                        // The virtual call on a Thing goes to the class of the script the
                        // Thing carries, when that class is, or derives from, the class
                        // the caller names (MEASURED: the Pod's Earth and Moon stand in
                        // the running game where the `GetPosition__` of their own
                        // classes puts them, not where their base class's does). A null
                        // Thing, or one whose script is not known, is called as named.
                        match args.values().next() {
                            Some(Value::Thing(Some(t))) => host
                                .thing_script(*t)
                                .and_then(|g| self.by_guid.get(&g).copied())
                                .filter(|c| self.is_a(*c, &owner)),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let target = receiver_class
                        .or(owner_class)
                        .and_then(|c| self.find(c, &function));
                    let fr = stack.last_mut().expect("frame");
                    fr.pc = next;
                    match target {
                        Some((c, f)) => {
                            if depth >= MAX_CALL_DEPTH {
                                return Err(Fault::CallDepth);
                            }
                            if self.classes[c.0 as usize]
                                .script
                                .function_defs
                                .iter()
                                .filter(|d| {
                                    self.classes[c.0 as usize].script.name(d.name)
                                        == Some(function.as_str())
                                })
                                .count()
                                > 1
                            {
                                self.ambiguous_calls += 1;
                            }
                            let callee = self.frame(c, f, args, dest)?;
                            stack.push(callee);
                        }
                        // INFERRED: Yield suspends the script until the next tick
                        // (docs/script-natives.md).
                        None if owner == "System" && function == "Yield__" => {
                            return Ok(Step::Yielded(Task { stack }));
                        }
                        None => {
                            let values: Vec<Value> = args.into_values().collect();
                            // A native inherited through a subclass reference reaches the
                            // host under the class that declares it (Rt part 2).
                            let owner = owner_class
                                .and_then(|c| self.native_owner(c, &function))
                                .map_or(owner, |c| self.classes[c.0 as usize].name.clone());
                            // D326: a factory native makes a script object: its fields,
                            // then `.init__`, then `.ctor__`, before the caller goes on.
                            let made = host
                                .native_object_class(&owner, &function)
                                .and_then(|n| self.by_name.get(n).copied());
                            if let Some(class) = made {
                                let id = self.instantiate(class)?;
                                if let Some(d) = dest {
                                    let fr = stack.last_mut().expect("frame");
                                    *fr.regs
                                        .get_mut(usize::from(d))
                                        .ok_or(Fault::Register { pc, register: d })? =
                                        Value::Object(Some(id));
                                }
                                if depth + 2 >= MAX_CALL_DEPTH {
                                    return Err(Fault::CallDepth);
                                }
                                if let Some(ctor) = self.this_frame(class, id, ".ctor__")? {
                                    stack.push(ctor);
                                }
                                if let Some(init) = self.init_frame(class, id)? {
                                    stack.push(init);
                                }
                                continue;
                            }
                            // Script threads (docs/script-vm.md): `Thread.Start__` passes
                            // only `this`; the scheduler runs `ThreadMain__` on it.
                            let v = if owner == "Thread" && function == "StartImpl__" {
                                match values.first() {
                                    Some(Value::Object(Some(o))) => self.started_threads.push(*o),
                                    _ => return Err(Fault::NullObject { pc }),
                                }
                                Value::Void
                            } else {
                                host.call_native(&owner, &function, &values)
                                    .map_err(Fault::Native)?
                            };
                            // A host that returns nothing for a typed call (stubs) gets the
                            // default of the instruction's return machine type.
                            let v = if v == Value::Void && ty != 0 {
                                Value::default_for(ty)
                            } else {
                                v
                            };
                            if let Some(d) = dest {
                                let fr = stack.last_mut().expect("frame");
                                *fr.regs
                                    .get_mut(usize::from(d))
                                    .ok_or(Fault::Register { pc, register: d })? = v;
                            }
                        }
                    }
                    continue;
                }
                Class::Return => {
                    let v = if ty == 0 { Value::Void } else { get!(r16(32)) };
                    let done = stack.pop().expect("frame");
                    staged.clear();
                    match stack.last_mut() {
                        None => return Ok(Step::Done(v)),
                        Some(caller) => {
                            if let Some(d) = done.dest {
                                *caller
                                    .regs
                                    .get_mut(usize::from(d))
                                    .ok_or(Fault::Register { pc, register: d })? = v;
                            }
                        }
                    }
                    continue;
                }
                Class::Branch => {
                    let take = match op {
                        0xa6 => true,
                        0xa7 => !get!(r16(16)).truthy().ok_or_else(mismatch)?,
                        _ => get!(r16(16)).truthy().ok_or_else(mismatch)?,
                    };
                    if take {
                        let base = if self.rules.branch_from_current {
                            pc
                        } else {
                            pc + 1
                        };
                        next = base
                            .checked_add_signed(hi32 as i32 as isize)
                            .ok_or(Fault::Branch { pc })?;
                    }
                }
            }
            stack.last_mut().expect("frame").pc = next;
        }
    }

    fn type_class(&self, script: &Script, type_ref: u16, pc: usize) -> Result<ClassId, Fault> {
        let t = script
            .type_refs
            .get(usize::from(type_ref))
            .ok_or(Fault::Index {
                pc,
                what: "type reference",
            })?;
        let name = script.name(t.type_name).unwrap_or("");
        t.script
            .and_then(|d| d.guid)
            .and_then(|g| self.by_guid.get(&g).copied())
            .or_else(|| self.by_name.get(name).copied())
            .ok_or_else(|| Fault::MissingClass(name.to_owned()))
    }

    fn is_a(&self, class: ClassId, name: &str) -> bool {
        let mut at = Some(class);
        for _ in 0..MAX_CALL_DEPTH {
            match at {
                Some(c) if self.classes[c.0 as usize].name == name => return true,
                Some(c) => at = self.super_class(c),
                None => return false,
            }
        }
        false
    }
}

/// (owning class name, field name) of a field reference.
fn member_name(script: &Script, field: u16) -> Option<(String, String)> {
    let r = script.field_refs.get(usize::from(field))?;
    let name = script.name(r.name)?.to_owned();
    let owner = script
        .type_refs
        .get(r.type_ref as usize)
        .and_then(|t| script.name(t.type_name))
        .unwrap_or("")
        .to_owned();
    Some((owner, name))
}

/// `Err(None)` = type mismatch, `Err(Some(fault))` = a specific fault.
fn array_ref<'a>(
    heap: &'a [HeapObject],
    o: &Value,
    pc: usize,
) -> Result<&'a [Value], Option<Fault>> {
    match o {
        Value::Object(Some(id)) => match heap.get(id.0 as usize) {
            Some(HeapObject::Array(a)) => Ok(a),
            _ => Err(None),
        },
        Value::Object(None) => Err(Some(Fault::NullObject { pc })),
        _ => Err(None),
    }
}

/// Array elements are zeroed storage of the instruction's machine type (INFERRED: the
/// array instructions carry the element type, e.g. `GET_ELEMENT t=3` on an int array; the
/// D146 TestArray run read a Void element back after an `EQi`). A Void element, value
/// written or resize fill becomes that type's default.
fn typed(v: Value, ty: u8) -> Value {
    match v {
        Value::Void => Value::default_for(ty),
        v => v,
    }
}

fn zero_like(v: &Value) -> Value {
    match v {
        Value::Bool(_) => Value::Bool(false),
        Value::Char(_) => Value::Char(0),
        Value::Int(_) => Value::Int(0),
        Value::Float(_) => Value::Float(0.),
        Value::Vec4(_) => Value::Vec4([0.; 4]),
        Value::Mat44(_) => Value::Mat44([0.; 16]),
        Value::Long(_) => Value::Long(0),
        Value::Double(_) => Value::Double(0.),
        Value::Object(_) => Value::Object(None),
        Value::Thing(_) => Value::Thing(None),
        _ => Value::Void,
    }
}

fn unary(op: u8, v: Value) -> Option<Value> {
    use Value::*;
    Some(match (op, v) {
        // Plain moves (MOVb/c/i/f/v4/m44/rp/cp/sp/o/s64); MOVs is a Nop-class opcode.
        (0x07 | 0x09 | 0x0a | 0x11 | 0x18 | 0x1a | 0x1c..=0x1f | 0xc4, v) => v,
        (0x08, Bool(b)) => Bool(!b),
        (0x0b, Int(i)) => Int(i.wrapping_add(1)),
        (0x0c, Int(i)) => Int(i.wrapping_sub(1)),
        (0x0d, Int(i)) => Int(i.wrapping_neg()),
        (0x0e, Int(i)) => Int(!i),
        (0x0f, Int(i)) => Bool(i == 0),
        (0x10, Int(i)) => Int(i.wrapping_abs()),
        (0x12, Float(f)) => Float(-f),
        (0x13, Float(f)) => Float(f.abs()),
        (0x14, Float(f)) => Float(f.sqrt()),
        (0x15, Float(f)) => Float(f.sin()),
        (0x16, Float(f)) => Float(f.cos()),
        (0x17, Float(f)) => Float(f.tan()),
        (0x19, Vec4(v)) => Vec4(v.map(|x| -x)),
        // Conversions: INTb INTc INTf FLOATb FLOATc FLOATi BOOLc BOOLi BOOLf.
        (0xaa, Bool(b)) => Int(i32::from(b)),
        (0xab, Char(c)) => Int(i32::from(c)),
        (0xac, Float(f)) => Int(f as i32),
        (0xad, Bool(b)) => Float(f32::from(u8::from(b))),
        (0xae, Char(c)) => Float(f32::from(c)),
        (0xaf, Int(i)) => Float(i as f32),
        (0xb0, Char(c)) => Bool(c != 0),
        (0xb1, Int(i)) => Bool(i != 0),
        (0xb2, Float(f)) => Bool(f != 0.),
        _ => return None,
    })
}

type BinaryResult = Result<Result<Value, ()>, Option<Fault>>;

fn binary(op: u8, a: Value, b: Value) -> BinaryResult {
    use Value::*;
    let ok = |v| Ok(Ok(v));
    match (op, a, b) {
        (0x20, Bool(a), Bool(b)) => ok(Bool(a == b)),
        (0x21, Bool(a), Bool(b)) => ok(Bool(a != b)),
        (0x24..=0x29, Char(a), Char(b)) => ok(Bool(cmp(op - 0x24, a, b))),
        (0x2a..=0x3c, Int(a), Int(b)) => Ok(int_op(op, a, b)),
        (0x3d..=0x42, Float(a), Float(b)) => ok(Float(match op {
            0x3d => a + b,
            0x3e => a - b,
            0x3f => a * b,
            0x40 => a / b,
            0x41 => a.min(b),
            _ => a.max(b),
        })),
        (0x43..=0x48, Float(a), Float(b)) => ok(Bool(cmp(op - 0x43, a, b))),
        (0x49, Vec4(a), Vec4(b)) => ok(Vec4(std::array::from_fn(|i| a[i] + b[i]))),
        (0x4a, Vec4(a), Vec4(b)) => ok(Vec4(std::array::from_fn(|i| a[i] - b[i]))),
        (0x4b, Vec4(a), Float(s)) => ok(Vec4(a.map(|x| x * s))),
        (0x4c, Vec4(a), Float(s)) => ok(Vec4(a.map(|x| x / s))),
        (0x4d, Vec4(a), Vec4(b)) => ok(Float((0..4).map(|i| a[i] * b[i]).sum())),
        (0x4e, Vec4(a), Vec4(b)) => ok(Float((0..3).map(|i| a[i] * b[i]).sum())),
        (0x4f, Vec4(a), Vec4(b)) => ok(Float((0..2).map(|i| a[i] * b[i]).sum())),
        (0x50, Vec4(a), Vec4(b)) => ok(Vec4([
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
            0.,
        ])),
        // Column-major storage (element (row, col) at col * 4 + row, translation at 12..14)
        // and A * B in the engine's sense: the Pod World matrix read in RPCS3 is the
        // transpose of the row-major product (D180 trace, 2026-10-09).
        (0x51, Mat44(a), Mat44(b)) => ok(Mat44(std::array::from_fn(|i| {
            let (col, row) = (i / 4, i % 4);
            (0..4).map(|k| a[k * 4 + row] * b[col * 4 + k]).sum()
        }))),
        // Pointer/object/s64 equality: EQrp EQo EQsp EQs64 and their NE forms.
        (0x54 | 0x56 | 0x58 | 0xc6, a, b) => ok(Bool(a == b)),
        (0x55 | 0x57 | 0x59 | 0xc7, a, b) => ok(Bool(a != b)),
        (0xbd, Bool(a), Bool(b)) => ok(Bool(a | b)),
        (0xbe, Bool(a), Bool(b)) => ok(Bool(a & b)),
        (0xbf, Bool(a), Bool(b)) => ok(Bool(a ^ b)),
        (0xc5, Long(a), Long(b)) => ok(Long(a.wrapping_add(b))),
        (0xc8, Long(a), Long(b)) => ok(Long(a | b)),
        (0xc9, Long(a), Long(b)) => ok(Long(a & b)),
        (0xca, Long(a), Long(b)) => ok(Long(a ^ b)),
        _ => Err(None),
    }
}

fn cmp<T: PartialOrd>(k: u8, a: T, b: T) -> bool {
    match k {
        0 => a < b,
        1 => a <= b,
        2 => a > b,
        3 => a >= b,
        4 => a == b,
        _ => a != b,
    }
}

fn int_op(op: u8, a: i32, b: i32) -> Result<Value, ()> {
    use Value::*;
    Ok(match op {
        0x2a => Int(a.wrapping_add(b)),
        0x2b => Int(a.wrapping_sub(b)),
        0x2c => Int(a.wrapping_mul(b)),
        0x2d => Int(a.checked_div(b).ok_or(())?),
        0x2e => Int(a.checked_rem(b).ok_or(())?),
        0x2f => Int(a.min(b)),
        0x30 => Int(a.max(b)),
        0x31 => Int(a.wrapping_shl(b as u32)),
        0x32 => Int(a.wrapping_shr(b as u32)),
        0x33 => Int((a as u32).wrapping_shr(b as u32) as i32),
        0x34 => Int(a | b),
        0x35 => Int(a & b),
        0x36 => Int(a ^ b),
        _ => Bool(cmp(op - 0x37, a, b)),
    })
}

/// Where the element the scripts call row `r`, column `c` sits in a matrix kept in the
/// engine's memory order: `c * 4 + r`.
///
/// The `M44` instructions name an element by two letters and a row or column by one. What
/// the scripts do with them settles the layout. FOUND in the script tables (names and use
/// only): the game's own test script fills a matrix by rows and reads row X back as the
/// elements XX, XY, XZ, XW, so the first letter is the row. Level scripts take the position
/// of a world matrix as its column W (over a hundred uses, among them the PPos world matrix
/// of a Thing), take column Z as the forward axis, and one writes a position through XW,
/// YW and ZW. The engine keeps the translation of a world matrix in floats 12..14 (level
/// files; the Pod world matrix read in RPCS3). So column `c` is the four floats from
/// `c * 4`, and row `r` is every fourth float from `r`.
fn m44_index(r: usize, c: usize) -> usize {
    c * 4 + r
}

fn get_builtin(op: u8, base: &Value) -> Option<Value> {
    use Value::*;
    Some(match (op, base) {
        (0x5a..=0x5d, Vec4(v)) => Float(v[usize::from(op - 0x5a)]),
        (0x5e..=0x60, Vec4(v)) => {
            let n = usize::from(op - 0x5e) + 2;
            Float(v[..n].iter().map(|x| x * x).sum::<f32>().sqrt())
        }
        // GET_M44_XX .. WW: row letter first.
        (0x61..=0x70, Mat44(m)) => {
            let k = usize::from(op - 0x61);
            Float(m[m44_index(k / 4, k % 4)])
        }
        // GET_M44_rX .. rW.
        (0x71..=0x74, Mat44(m)) => {
            let r = usize::from(op - 0x71);
            Vec4(std::array::from_fn(|c| m[m44_index(r, c)]))
        }
        // GET_M44_cX .. cW.
        (0x75..=0x78, Mat44(m)) => {
            let c = usize::from(op - 0x75);
            Vec4(std::array::from_fn(|r| m[m44_index(r, c)]))
        }
        _ => return None,
    })
}

fn set_builtin(op: u8, base: Value, src: Value) -> Option<Value> {
    use Value::*;
    // A register never written reads as zeroed storage (INFERRED: v4/m44 locals are built
    // component by component; the PC U12 trace faulted on SET_V4_X of a fresh register).
    let base = match (op, base) {
        (0x79..=0x7c, Void) => Vec4([0.; 4]),
        (0x7d..=0x94, Void) => Mat44([0.; 16]),
        (_, b) => b,
    };
    Some(match (op, base, src) {
        (0x79..=0x7c, Vec4(mut v), Float(f)) => {
            v[usize::from(op - 0x79)] = f;
            Vec4(v)
        }
        (0x7d..=0x8c, Mat44(mut m), Float(f)) => {
            let k = usize::from(op - 0x7d);
            m[m44_index(k / 4, k % 4)] = f;
            Mat44(m)
        }
        (0x8d..=0x90, Mat44(mut m), Vec4(v)) => {
            let r = usize::from(op - 0x8d);
            for (c, x) in v.iter().enumerate() {
                m[m44_index(r, c)] = *x;
            }
            Mat44(m)
        }
        (0x91..=0x94, Mat44(mut m), Vec4(v)) => {
            let c = usize::from(op - 0x91);
            for (r, x) in v.iter().enumerate() {
                m[m44_index(r, c)] = *x;
            }
            Mat44(m)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows and columns of a matrix in memory order, as the scripts name them.
    #[test]
    fn m44_elements_rows_and_columns_follow_the_engine_layout() {
        // A world matrix: x axis (1, 2, 3), translation (10, 20, 30).
        let mut m = [0.; 16];
        m[..4].copy_from_slice(&[1., 2., 3., 0.]);
        m[12..].copy_from_slice(&[10., 20., 30., 1.]);
        let get = |op| get_builtin(op, &Value::Mat44(m)).unwrap();
        // cW is the translation, cX the x axis; rW the bottom row.
        assert_eq!(get(0x78), Value::Vec4([10., 20., 30., 1.]));
        assert_eq!(get(0x75), Value::Vec4([1., 2., 3., 0.]));
        assert_eq!(get(0x74), Value::Vec4([0., 0., 0., 1.]));
        assert_eq!(get(0x71), Value::Vec4([1., 0., 0., 10.]));
        // XW, YW, ZW are the position; YX is the x axis's y.
        assert_eq!(get(0x61 + 3), Value::Float(10.));
        assert_eq!(get(0x61 + 7), Value::Float(20.));
        assert_eq!(get(0x61 + 11), Value::Float(30.));
        assert_eq!(get(0x61 + 4), Value::Float(2.));
        // The writes go to the same places.
        let set = |op, v| match set_builtin(op, Value::Mat44([0.; 16]), v).unwrap() {
            Value::Mat44(m) => m,
            other => panic!("{other:?}"),
        };
        let v = Value::Vec4([5., 6., 7., 8.]);
        assert_eq!(set(0x94, v.clone())[12..], [5., 6., 7., 8.]);
        let row = set(0x8d + 1, v);
        assert_eq!([row[1], row[5], row[9], row[13]], [5., 6., 7., 8.]);
        assert_eq!(set(0x7d + 11, Value::Float(9.))[14], 9.);
        // Row X read back is XX, XY, XZ, XW (what the game's test script checks).
        let Value::Vec4(row_x) = get(0x71) else {
            panic!("row");
        };
        for (c, x) in row_x.iter().enumerate() {
            assert_eq!(get(0x61 + c as u8), Value::Float(*x));
        }
    }

    #[test]
    fn m44_product_is_column_major_a_times_b() {
        // A = translation (10, 0, 0), B = 90 degrees about z, both column-major.
        let mut a = [0.; 16];
        for d in 0..4 {
            a[d * 5] = 1.;
        }
        a[12] = 10.;
        let mut b = [0.; 16];
        b[1] = 1.; // column 0 = +y
        b[4] = -1.; // column 1 = -x
        b[10] = 1.;
        b[15] = 1.;
        let Ok(Ok(Value::Mat44(m))) = binary(0x51, Value::Mat44(a), Value::Mat44(b)) else {
            panic!("MULm44");
        };
        // A * B rotates first, then translates: the translation stays (10, 0, 0) and the
        // rotation is B's (B * A would give translation (0, 10, 0)).
        assert_eq!(&m[12..16], &[10., 0., 0., 1.]);
        assert_eq!(&m[0..2], &[0., 1.]);
        assert_eq!(&m[4..6], &[-1., 0.]);
    }
}
