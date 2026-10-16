//! D319 (docs/level-scripts.md part 70): the pod computer's menus as the planets level's
//! scripts build them. The Gooey (UI) natives a menu state calls in its `UpdateUI__Q5Gooey`
//! describe a tree of frames, buttons and breaks; its `Draw__Q5Gooeym` hands the engine a
//! matrix for the menu's root frame (`Gooey.SetItemViewMatrix__im`). [`GooeyState`] keeps
//! both, and [`LevelScripts::pod_computer_draw`] makes the engine's per-frame draw call
//! that leads to the matrix. [`layout`] places a recorded frame in the root frame's units.
//!
//! What each Gooey native does is known from its name and arguments only (the engine's
//! Gooey code was not read): every answer and every layout rule below is INFERRED unless it
//! says otherwise, and each was chosen to match the main menu in the game's frame
//! (`rt20_9601`, tick 619), as part 70 measures.
use crate::{LevelScripts, LevelWorld, StepReport};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Value, Vm};
use std::collections::{BTreeMap, HashMap};

/// The pod computer's script function the engine's per-frame draw function calls with one
/// matrix ([`LevelScripts::pod_computer_draw`]).
pub const POD_COMPUTER_DRAW: &str = "Draw__m";

/// The register of `Draw__m`'s matrix argument (its argument row in the 01.33 script).
const DRAW_MATRIX_REGISTER: u16 = 64;

/// Frames nested deeper than this are not opened (a guard; counted in
/// [`GooeyState::dropped`]).
pub const MAX_GOOEY_DEPTH: usize = 64;
/// Items one frame keeps (a guard; the rest are counted in [`GooeyState::dropped`]).
pub const MAX_GOOEY_ITEMS: usize = 256;

/// The matrix the engine hands `PodComputer.Draw__m` every frame, in the engine's order
/// (element (row r, column c) at `c * 4 + r`, translation at 12..14).
///
/// FOUND (01.33): the pod computer object's draw function (0x404598) builds it in a local
/// (0x404070) from constants alone and passes it to the script call 0x89470c -> 0x5038a4
/// -> 0x6585e8, the call of the PodComputer export that takes one matrix (`Draw__m` is the
/// class's only such export, INFERRED to be it). The builder is `B Rx(a) Rz(b) T(0, -100,
/// 550)` with `a` = -1.24791 and `b` = 0.00706858 radians and the basis `B` below (its x
/// axis, y axis, z axis and translation). With the main menu's own scale and offset
/// (`SetupFrameMatrix`) it puts the 2400 x 1250 root frame over the whole screen in front
/// of the planets level's camera (part 70).
pub fn pod_computer_draw_matrix() -> [f32; 16] {
    let basis = [
        [1.08496, 0.010962, 0., 0.],
        [0., 0., -1.08502, 0.],
        [-0.010962, 1.08496, 0., 0.],
        [0., -159.982, 168.926, 1.],
    ];
    let (a, b) = (-1.24791f32, 0.007_068_58f32);
    let rx = rotation_x(a);
    let rz = rotation_z(b);
    let t = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., -100., 550., 1.],
    ];
    let m = mul(&mul(&mul(&basis, &rx), &rz), &t);
    let mut out = [0.; 16];
    for (c, col) in m.iter().enumerate() {
        out[c * 4..c * 4 + 4].copy_from_slice(col);
    }
    out
}

/// Column-major 4 x 4 (`m[column][row]`).
type Cols = [[f32; 4]; 4];

fn mul(a: &Cols, b: &Cols) -> Cols {
    let mut out = [[0.; 4]; 4];
    for (c, col) in b.iter().enumerate() {
        for (r, cell) in out[c].iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * col[k]).sum();
        }
    }
    out
}

fn rotation_x(a: f32) -> Cols {
    let (s, c) = a.sin_cos();
    [
        [1., 0., 0., 0.],
        [0., c, s, 0.],
        [0., -s, c, 0.],
        [0., 0., 0., 1.],
    ]
}

fn rotation_z(a: f32) -> Cols {
    let (s, c) = a.sin_cos();
    [
        [c, s, 0., 0.],
        [-s, c, 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ]
}

/// A frame (`Gooey.StartFrame*` ... `EndFrame*`) with the properties the script set on it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GooeyFrame {
    /// The id of `StartFrameNamed`, `None` for an unnamed frame.
    pub id: Option<i32>,
    /// The "UID high bits" in force when the frame was opened
    /// (`Gooey.SetScriptUIDHighBits__i`): each pod computer state's ids are its own.
    pub high_bits: i32,
    /// `SetFrameLayout`: two codes and an optional float, as passed.
    pub layout: Option<(i32, i32, f32)>,
    /// `SetFrameSizing`: width and height in the root frame's units.
    pub sizing: Option<[f32; 2]>,
    /// `SetFrameBorders`: left, top, right, bottom (the two-float form gives x, y, x, y).
    pub borders: Option<[f32; 4]>,
    /// `SetFrameDefaultChildSpacing`: two floats as passed.
    pub spacing: Option<[f32; 2]>,
    /// `AddFrameColumn`: width and alignment code, in order.
    pub columns: Vec<(f32, i32)>,
    /// `SetFrameTransitionTime`: in and out, in frames (INFERRED unit).
    pub transition: Option<[f32; 2]>,
    /// Every other `SetFrame*` call, by native name, with its numbers.
    pub other: BTreeMap<String, Vec<f32>>,
    pub children: Vec<GooeyItem>,
}

/// One button (`DoSpaceButton*Named`) as passed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GooeyButton {
    pub id: i32,
    pub label: String,
    /// `Gooey.EGooeyTextStyle`, `Gooey.EGooeyButtonState` and the int after them.
    pub text_style: i32,
    pub state: i32,
    pub flags: i32,
    /// The last float: MEASURED to be the pill's turn in radians, clockwise on the screen
    /// (part 70: 0.2 / 0.075 / -0.075 / -0.2 / 0 against the game frame's pills).
    pub angle: f32,
    /// The font scale in force (`Gooey.set.FontScale`).
    pub font_scale: f32,
    /// The `...WithIcon...` form.
    pub icon: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GooeyItem {
    Frame(GooeyFrame),
    Button(GooeyButton),
    /// `DoBreak__`: the next item starts a new row.
    Break,
    /// `DoHorizontalBreak*`, with its numbers.
    HorizontalBreak(Vec<f32>),
    /// `DoText*`: the string, the text style and the font scale in force.
    Text {
        text: String,
        style: i32,
        font_scale: f32,
    },
    /// `DoPlayerBigIcon*`: the player's picture card; the id when named.
    PlayerIcon {
        id: Option<i32>,
    },
    /// Any other `Do*` native, by name.
    Other(String),
}

/// What the Gooey natives built and were told (one per level world). The tree is rebuilt
/// each time a state's `UpdateUI` runs; a top-level frame that closes replaces the one
/// with the same high bits and id.
#[derive(Debug, Clone, Default)]
pub struct GooeyState {
    open: Vec<GooeyFrame>,
    /// Closed top-level frames by (high bits, id; -1 = unnamed).
    pub frames: BTreeMap<(i32, i32), GooeyFrame>,
    /// `PWorld.Frame` when each top-level frame was first closed (for its transition).
    pub first_built: BTreeMap<(i32, i32), i32>,
    /// Builds of top-level frames.
    pub builds: u64,
    /// The node that has the focus (high bits, id), by `EnsureNodeOrDescendantHasFocus`.
    pub focus: Option<(i32, i32)>,
    pub high_bits: i32,
    next_high_bits: i32,
    font_scale: Option<f32>,
    /// `SetItemViewMatrix(id, m)` by (high bits, id): the matrix the root frame is drawn
    /// with, from its units to the planets level's world (level units).
    pub item_matrices: BTreeMap<(i32, i32), [f32; 16]>,
    /// `PWorld.Frame` of the last `SetItemViewMatrix` of each key.
    pub item_matrix_frames: BTreeMap<(i32, i32), i32>,
    /// `SetColoursFromPlayer`: the player the Gooey takes its colours from.
    pub colours_player: Option<i32>,
    /// `RenderToScreen` calls.
    pub renders: u64,
    /// Calls answered, by `Class.mangled`.
    pub calls: BTreeMap<String, u64>,
    /// Frames or items not kept (the depth and item guards, an `EndFrame` with no frame).
    pub dropped: u64,
    /// D325: `PWorld.Frame` when the focus last moved to another node (the focused pill's
    /// spring starts there).
    pub focus_since: Option<i32>,
    /// D325: the pad's menu input waiting for the next build of the menu.
    pub input: GooeyInput,
    /// D325: `PWorld.Frame` of the first builds of top-level frames (at most 256).
    pub build_frames: Vec<i32>,
    /// D325: the focus moved since the last build of a top-level frame.
    pub focus_moved: bool,
}

/// D325: the menu input the pad gives the Gooey for its next build. FOUND (the script
/// listings, `PodStateMainMenu.UpdateUI`): a button's answer is tested only for its
/// activate bit (256, passed as the button's mask when the entry is open) and the root
/// frame's `EndFrameInput(512)` only for 512, which pops the state. INFERRED: which pad
/// button gives which (the game's engine code was not read): the viewer's choose and back
/// keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GooeyInput {
    /// The focused button is activated: its button call answers 256 once.
    pub choose: bool,
    /// Back: an `EndFrameInput` whose mask has 512 answers 512 once.
    pub back: bool,
}

/// D325: the activate bit of a button's answer and mask.
pub const GOOEY_ACTIVATE: i32 = 256;
/// D325: the back bit of `EndFrameInput`'s answer and mask.
pub const GOOEY_BACK: i32 = 512;

impl GooeyState {
    fn font_scale(&self) -> f32 {
        self.font_scale.unwrap_or(1.)
    }

    /// `Gooey.SetStylesheetScalingFactor` (the `FontScale` property's setter).
    pub(crate) fn set_font_scale(&mut self, scale: f32) {
        self.font_scale = Some(scale);
    }

    fn push(&mut self, item: GooeyItem) {
        match self.open.last_mut() {
            Some(f) if f.children.len() < MAX_GOOEY_ITEMS => f.children.push(item),
            _ => self.dropped += 1,
        }
    }

    fn start(&mut self, id: Option<i32>) {
        if self.open.len() >= MAX_GOOEY_DEPTH {
            self.dropped += 1;
            return;
        }
        self.open.push(GooeyFrame {
            id,
            high_bits: self.high_bits,
            ..Default::default()
        });
    }

    /// Sets the focus, noting the frame when it moves to another node.
    fn set_focus(&mut self, focus: (i32, i32), frame: i32) {
        if self.focus != Some(focus) {
            self.focus = Some(focus);
            self.focus_since = Some(frame);
            self.focus_moved = true;
        }
    }

    /// D325: moves the focus of the drawn menu to the button nearest the focused one in the
    /// direction `dir` (root frame units, x right, y down), as laid out with `fonts`; false
    /// when no button lies that way. INFERRED (the engine's navigation code was not read):
    /// the buttons whose centre lies within 60 degrees of the direction, the nearest by the
    /// distance along it plus twice the distance across; locked entries can take the focus.
    pub fn move_focus(&mut self, dir: [f32; 2], fonts: Option<&GooeyFonts>, frame: i32) -> bool {
        let Some(menu) = self.drawn_menu() else {
            return false;
        };
        let bits = self.focus.map_or(self.high_bits, |f| f.0);
        let focused = menu.focus;
        let placed = layout_with(menu.frame, focused, fonts);
        let buttons: Vec<(i32, [f32; 2])> = placed
            .iter()
            .filter_map(|p| match p.kind {
                PlacedKind::Button { id, .. } => Some((id, p.centre)),
                _ => None,
            })
            .collect();
        let Some(&(_, from)) = buttons.iter().find(|(id, _)| Some(*id) == focused) else {
            return false;
        };
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if len <= 0. {
            return false;
        }
        let d = [dir[0] / len, dir[1] / len];
        let best = buttons
            .iter()
            .filter(|(id, _)| Some(*id) != focused)
            .filter_map(|&(id, c)| {
                let v = [c[0] - from[0], c[1] - from[1]];
                let along = v[0] * d[0] + v[1] * d[1];
                let across = (v[0] * d[1] - v[1] * d[0]).abs();
                (along > 0. && across <= along * 3f32.sqrt()).then_some((along + 2. * across, id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match best {
            Some((_, id)) => {
                self.set_focus((bits, id), frame);
                true
            }
            None => false,
        }
    }

    fn end(&mut self, frame: i32) {
        let Some(f) = self.open.pop() else {
            self.dropped += 1;
            return;
        };
        if self.open.is_empty() {
            // D325: a choice no focused button took lapses with the build.
            self.input.choose = false;
            self.focus_moved = false;
            let key = (f.high_bits, f.id.unwrap_or(-1));
            self.first_built.entry(key).or_insert(frame);
            self.frames.insert(key, f);
            self.builds += 1;
            if self.build_frames.len() < 256 {
                self.build_frames.push(frame);
            }
        } else {
            self.push(GooeyItem::Frame(f));
        }
    }

    /// The frame with this id under the current high bits: an open one first, then a
    /// closed top-level one.
    fn find(&self, id: i32) -> Option<&GooeyFrame> {
        fn within(f: &GooeyFrame, id: i32) -> Option<&GooeyFrame> {
            if f.id == Some(id) {
                return Some(f);
            }
            f.children.iter().find_map(|c| match c {
                GooeyItem::Frame(c) => within(c, id),
                _ => None,
            })
        }
        let bits = self.high_bits;
        self.open
            .iter()
            .rev()
            .filter(|f| f.high_bits == bits)
            .find_map(|f| within(f, id))
            .or_else(|| {
                self.frames
                    .iter()
                    .filter(|((b, _), _)| *b == bits)
                    .find_map(|(_, f)| within(f, id))
            })
    }

    /// The root frame of a menu that has been drawn: the top-level frame with an item
    /// matrix, the last one set.
    pub fn drawn_menu(&self) -> Option<DrawnMenu<'_>> {
        let (key, frame_set) = self.item_matrix_frames.iter().max_by_key(|(_, f)| **f)?;
        Some(DrawnMenu {
            frame: self.frames.get(key)?,
            matrix: *self.item_matrices.get(key)?,
            first_built: self.first_built.get(key).copied(),
            matrix_frame: *frame_set,
            focus: self.focus.filter(|f| f.0 == key.0).map(|f| f.1),
        })
    }
}

/// A menu root frame with the matrix it is drawn with ([`GooeyState::drawn_menu`]).
#[derive(Debug, Clone, Copy)]
pub struct DrawnMenu<'a> {
    pub frame: &'a GooeyFrame,
    /// Root frame units (x right, y down) to the planets level's world.
    pub matrix: [f32; 16],
    /// `PWorld.Frame` when the frame was first built, and when the matrix was last set.
    pub first_built: Option<i32>,
    pub matrix_frame: i32,
    /// The id of the focused node under the frame's high bits.
    pub focus: Option<i32>,
}

fn num(v: Option<&Value>) -> f32 {
    match v {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(i)) => *i as f32,
        _ => 0.,
    }
}

fn int(v: Option<&Value>) -> i32 {
    match v {
        Some(Value::Int(i)) => *i,
        Some(Value::Float(f)) => *f as i32,
        Some(Value::Bool(b)) => i32::from(*b),
        _ => 0,
    }
}

fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::Str(s)) => s.to_string(),
        _ => String::new(),
    }
}

fn numbers(args: &[Value]) -> Vec<f32> {
    args.iter()
        .filter(|v| matches!(v, Value::Float(_) | Value::Int(_)))
        .map(|v| num(Some(v)))
        .collect()
}

/// The base name of a mangled native (`SetFrameLayout__iif` -> `SetFrameLayout`).
fn base(mangled: &str) -> &str {
    mangled.split("__").next().unwrap_or(mangled)
}

impl LevelWorld {
    /// [`lbp_script_vm::natives::World::ui_call`]: the Gooey and callout natives and the
    /// queries around the pod computer's menus. Every answer is INFERRED (the engine's
    /// code for them was not read) unless its line says otherwise; `None` for a native
    /// not listed here, which then counts as unbound.
    pub fn ui_call(
        &mut self,
        class: &str,
        mangled: &str,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Option<Value> {
        let frame = self.pworld.frame;
        let answer = match class {
            "Gooey" => self.gooey_call(mangled, args, frame),
            "Callout" => Some(callout(mangled)),
            _ => self.menu_query(class, mangled, receiver, args),
        }?;
        *self
            .gooey
            .calls
            .entry(format!("{class}.{mangled}"))
            .or_default() += 1;
        Some(answer)
    }

    fn gooey_call(&mut self, mangled: &str, args: &[Value], frame: i32) -> Option<Value> {
        let g = &mut self.gooey;
        let name = base(mangled);
        let a = |i: usize| args.get(i);
        Some(match name {
            // A frame opens and the script fills it (true: the menus are drawn, so the
            // game's answer is "open" while the menu shows).
            "StartFrame" => {
                g.start(None);
                Value::Bool(true)
            }
            "StartFrameNamed" => {
                g.start(Some(int(a(0))));
                Value::Bool(true)
            }
            "EndFrame" => {
                g.end(frame);
                Value::Void
            }
            // No input reached the frame: no bit of the mask.
            // D325: the back bit when the pad asked for it and the mask has it.
            "EndFrameInput" => {
                g.end(frame);
                let back = g.input.back && int(a(0)) & GOOEY_BACK != 0;
                // A back no root frame took lapses with the build.
                if back || g.open.is_empty() {
                    g.input.back = false;
                }
                Value::Int(if back { GOOEY_BACK } else { 0 })
            }
            "SetFrameLayout" => {
                if let Some(f) = g.open.last_mut() {
                    f.layout = Some((int(a(0)), int(a(1)), num(a(2))));
                }
                Value::Void
            }
            "SetFrameSizing" => {
                if let Some(f) = g.open.last_mut() {
                    f.sizing = Some([num(a(0)), num(a(1))]);
                }
                Value::Void
            }
            "SetFrameBorders" => {
                if let Some(f) = g.open.last_mut() {
                    f.borders = Some(match args.len() {
                        4 => [num(a(0)), num(a(1)), num(a(2)), num(a(3))],
                        _ => [num(a(0)), num(a(1)), num(a(0)), num(a(1))],
                    });
                }
                Value::Void
            }
            "SetFrameDefaultChildSpacing" => {
                if let Some(f) = g.open.last_mut() {
                    f.spacing = Some([num(a(0)), num(a(1))]);
                }
                Value::Void
            }
            "SetFrameTransitionTime" => {
                if let Some(f) = g.open.last_mut() {
                    f.transition = Some([num(a(0)), num(a(1))]);
                }
                Value::Void
            }
            "AddFrameColumn" => {
                if let Some(f) = g.open.last_mut() {
                    f.columns.push((num(a(0)), int(a(1))));
                }
                Value::Void
            }
            n if n.starts_with("SetFrame") => {
                if let Some(f) = g.open.last_mut() {
                    f.other.insert(n.to_owned(), numbers(args));
                }
                Value::Void
            }
            // A button: (id, label, text style, state, int, angle). 0: not pressed (the
            // 256 bit the menu tests is "activated").
            "DoSpaceButtonNamed" | "DoSpaceButtonWithIconNamed" => {
                let font_scale = g.font_scale();
                g.push(GooeyItem::Button(GooeyButton {
                    id: int(a(0)),
                    label: text(a(1)),
                    text_style: int(a(2)),
                    state: int(a(3)),
                    flags: int(a(4)),
                    angle: num(args.last().filter(|v| matches!(v, Value::Float(_)))),
                    font_scale,
                    icon: name.contains("Icon"),
                }));
                // D325: the focused button answers the activate bit once when chosen and
                // its mask has the bit (a locked entry passes 0).
                let id = int(a(0));
                let chosen = g.input.choose
                    && g.focus == Some((g.high_bits, id))
                    && int(a(4)) & GOOEY_ACTIVATE != 0;
                if chosen {
                    g.input.choose = false;
                }
                Value::Int(if chosen { GOOEY_ACTIVATE } else { 0 })
            }
            "DoBreak" => {
                g.push(GooeyItem::Break);
                Value::Void
            }
            "DoHorizontalBreak" => {
                g.push(GooeyItem::HorizontalBreak(numbers(args)));
                Value::Void
            }
            "DoText" => {
                let font_scale = g.font_scale();
                g.push(GooeyItem::Text {
                    text: text(a(0)),
                    style: int(a(1)),
                    font_scale,
                });
                Value::Int(0)
            }
            "DoPlayerBigIcon" | "DoPlayerBigIconNamed" => {
                let id = (name == "DoPlayerBigIconNamed").then(|| int(a(0)));
                g.push(GooeyItem::PlayerIcon { id });
                Value::Int(0)
            }
            n if n.starts_with("Do") => {
                g.push(GooeyItem::Other(n.to_owned()));
                Value::Int(0)
            }
            // The menu keeps its focus; a focus on no button moves to the node asked for.
            "EnsureNodeOrDescendantHasFocus" => {
                let id = int(a(0));
                let bits = g.high_bits;
                let on_button = g.focus.is_some_and(|(b, f)| {
                    b == bits
                        && g.frames.values().chain(g.open.iter()).any(|fr| {
                            fn has(fr: &GooeyFrame, id: i32) -> bool {
                                fr.children.iter().any(|c| match c {
                                    GooeyItem::Button(b) => b.id == id,
                                    GooeyItem::Frame(f) => has(f, id),
                                    _ => false,
                                })
                            }
                            has(fr, f)
                        })
                });
                if !on_button {
                    g.set_focus((bits, id), frame);
                }
                Value::Bool(true)
            }
            "IsValidNode" => Value::Bool(g.find(int(a(0))).is_some()),
            "SetScriptUIDHighBits" => {
                g.high_bits = int(a(0));
                Value::Void
            }
            "ResetScriptUIDHighBits" => {
                g.high_bits = 0;
                Value::Void
            }
            // A new block of ids for a state: 1, 2, ... (the game's values were not read).
            "GetUniqueContainerIDHighBits" => {
                g.next_high_bits += 1;
                Value::Int(g.next_high_bits)
            }
            // The frame's own size (its `SetFrameSizing`): the layout pass that may change
            // it is not modelled.
            "GetFrameSize" | "GetFrameSizeTarget" => {
                let s = g.find(int(a(0))).and_then(|f| f.sizing).unwrap_or([0.; 2]);
                Value::Vec4([s[0], s[1], 0., 0.])
            }
            "SetItemViewMatrix" => {
                if let Some(Value::Mat44(m)) = a(1) {
                    let key = (g.high_bits, int(a(0)));
                    g.item_matrices.insert(key, *m);
                    g.item_matrix_frames.insert(key, frame);
                }
                Value::Void
            }
            "RenderToScreen" => {
                g.renders += 1;
                Value::Void
            }
            // D325: the pad moved the focus or waits to choose or go back. FOUND (the
            // listings, `PodComputer.NeedUpdateUI__`): this answer is or-ed into the pod
            // computer's rebuild test, and without it the main menu is rebuilt only when a
            // state asks (every ~30 frames in the session). INFERRED (the engine's code was
            // not read): the game's Gooey answers true on the frame its input changes, as it
            // reacts to the pad at once.
            "NeedsUpdate" => Value::Bool(g.focus_moved || g.input.choose || g.input.back),
            "SetColoursFromPlayer" => {
                g.colours_player = Some(int(a(0)));
                Value::Void
            }
            // D326 FOUND (the 01.33 listings): the script property `Gooey.FontScale` has
            // code that calls `GetStylesheetScalingFactor__` / `SetStylesheetScalingFactor__f`
            // (the setter is a table native and reaches `engine_call`).
            "GetStylesheetScalingFactor" => Value::Float(g.font_scale()),
            // The player's colour the Gooey took (`SetColoursFromPlayer`), as a colour
            // vector; the first record's without one.
            "GetPrimaryColour" => {
                let number = g.colours_player;
                let record = self
                    .player_records
                    .iter()
                    .find(|r| Some(r.number) == number)
                    .or(self.player_records.first());
                Value::Vec4(lbp_script_vm::natives::colour_vector(
                    record.map_or(lbp_script_vm::natives::DEFAULT_PLAYER_COLOUR, |r| r.colour),
                ))
            }
            _ => return None,
        })
    }

    fn menu_query(
        &mut self,
        class: &str,
        mangled: &str,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Option<Value> {
        if let Some(v) = self.search_query(class, mangled, receiver, args) {
            return Some(v);
        }
        Some(match (class, mangled) {
            // No overlay of the planets changed.
            ("Planets", "CheckOverlayNeedsUpdate__") => Value::Bool(false),
            // The primary local player uses the pod computer.
            ("PodComputer", "GetPodComputerUserPlayerNumber__") => {
                Value::Int(self.primary_player().unwrap_or(0))
            }
            // The title before (none).
            ("PodComputer", "SetActivityTitle__Q28SearchDetails.EActivityTitle") => Value::Int(0),
            // Offline, no crossplay, no friends, no streams, no DLC, the retail game, a
            // profile that has not played the first game.
            ("PodComputer", "HasPSNDemoBuyButton__")
            | ("Network", "IsCrossPlayConnected__")
            | ("Network", "IsCrossPlayConnectionError__")
            | ("System", "IsOnline__")
            | ("System", "IsBeta__")
            | ("Profile", "HasPlayedLBP1__")
            | ("Profile", "HasCrossplayDLC__")
            | ("Profile", "HasMoveDLC__") => Value::Bool(false),
            // The menu offers Story for a profile that has played the intro levels; the
            // game frame's profile has (INFERRED from the frame: Story is there).
            ("Profile", "HasPlayedIntroLevels__") => Value::Bool(true),
            ("PartyManagement", "GetNumFriends__") | ("Store", "GetNumDLCStoryPacks__") => {
                Value::Int(0)
            }
            ("Start_Menu", "GetFriendStatus__Q13PodFriendData") => Value::Int(0),
            ("Network", "CrossPlayDisconnect__")
            | ("Network", "SendPodStateChangedTelemetryMessage__Q6String")
            | ("PartyManagement", "GatherFriendData__[Q13PodFriendData") => Value::Void,
            ("StreamManager", _) | ("Game", "GetLocalPlayerOnlineID__") => Value::Object(None),
            ("HUD", "GetScreenEdgeCrop__") => Value::Float(0.),
            ("HUD", "SDF_INFO__iii") => Value::Vec4([0.; 4]),
            ("Audio", "RequestVoiceOverMovePack__Q7StringAbbb") => Value::Bool(false),
            // D326: English (the PS3's CELL_SYSUTIL_LANG_ENGLISH_US, 1). FOUND (the 01.33
            // listings, `IsCJK__`): 0 is one of the CJK languages, so the unbound default
            // drew the Story title at the CJK font scale (x1.5, not x2.5).
            ("System", "GetLanguageID__") => Value::Int(1),
            _ => return None,
        })
    }
}

impl LevelWorld {
    /// D327 (part 78): the natives the pod's search state (`PodStateSearch`) calls on its
    /// SearchDetails, its results and the planets. FOUND (the 01.33 listings): the return
    /// types (`GetResults__` a SearchResults, `GetFocusPlanet__` a Planet, `GetPlanet__`,
    /// `GetCurrentPage__` s32, the rest bool or void) and that `GetFocusPlanet__` and
    /// `GetResults__` are used without a null check. The answers are INFERRED unless the
    /// line says otherwise.
    fn search_query(
        &mut self,
        class: &str,
        mangled: &str,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Option<Value> {
        if let Some(v) = self.story_result_query(class, mangled, receiver, args) {
            return Some(v);
        }
        let int = || match args.first() {
            Some(Value::Int(i)) => Some(*i),
            _ => None,
        };
        let search = &mut self.pod.search;
        Some(match (class, mangled) {
            // INFERRED: the story levels are on Earth (crafteroid 1): the search state's
            // `GetPlanetsViewType__` has its Story branch (views 6 and 7) under planet 1,
            // and `OnActivateImpl` selects the planet this answers.
            ("SearchDetails", "GetPlanet__") => Value::Int(1),
            ("Planets", "SelectPlanet__i") => {
                search.selected_planet = int();
                Value::Void
            }
            // The selected crafteroid; null before a planet is selected.
            ("Planets", "GetFocusPlanet__") => Value::Object(
                search
                    .selected_planet
                    .and_then(|i| usize::try_from(i).ok()?.checked_sub(1))
                    .and_then(|i| self.crafteroids.get(i))
                    .map(|(o, _)| *o),
            ),
            ("Planets", "SetCursorMode__i") => {
                search.cursor_mode = int();
                Value::Void
            }
            ("Planets", "SetBrowseMode__Q25Planets.EPlanetBrowseMode") => {
                search.browse_mode = int();
                Value::Void
            }
            ("Planets", "SetAllowedBrowseModes__i") => {
                search.allowed_browse_modes = int();
                Value::Void
            }
            ("Planets", "SetPublishCursorBadge__Q18SearchResultHandle") => {
                search.publish_cursor_badge_calls += 1;
                Value::Void
            }
            ("PodComputer", "SetLoader__Q23PodComputer.ELoaderType") => {
                search.loader = int();
                Value::Void
            }
            ("PodComputer", "SetEarthHasControl__b") => {
                search.earth_has_control = match args.first() {
                    Some(Value::Bool(b)) => Some(*b),
                    _ => None,
                };
                Value::Void
            }
            ("PodComputerPlanets", "SetWantSearchDetails__Q13SearchDetailsbbb") => {
                search.want_search_details_calls += 1;
                Value::Void
            }
            // The search's results: one SearchResults object (D328: its results are
            // `LevelWorld::story_results`), obtained at once, on one page.
            ("SearchDetails", "GetResults__") => Value::Object(self.search_results),
            ("SearchResults", "GetWantZoom__") => Value::Bool(false),
            ("SearchDetails", "IsBusyObtainingResults__")
            | ("SearchDetails", "IsPaged__")
            | ("SearchDetails", "RequiresOnline__")
            | ("SearchDetails", "CheckOverlayNeedsUpdate__")
            | ("PodComputer", "IsLaunchingLevel__")
            | ("PodComputerPlanets", "IsLoading__") => Value::Bool(false),
            ("SearchDetails", "GetCurrentPage__")
            | ("SearchDetails", "GetCrossplaySearchMode__") => Value::Int(0),
            ("SearchDetails", "ShowErrorMessage__") => {
                search.error_message_calls += 1;
                Value::Void
            }
            // No grief report is open.
            ("GriefReport", "ClearContext__") => Value::Void,
            _ => return None,
        })
    }
}

impl LevelWorld {
    /// D328: the natives on the Story search's results ([`LevelWorld::story_results`]),
    /// their handles and the Earth planet's cursor. FOUND (the 01.33 listings): the return
    /// types (`GetResult__i`, `FindResult__` a SearchResult, `FindResultSlot__` a
    /// SearchResultSlot, `GetHandle__` / `GetParent__` / `GetCurResult__` /
    /// `GetDefaultResult__` a SearchResultHandle, `GetResultType__` s32 compared with 1,
    /// `GetLocation__` a vector) and that the pod computer treats a result whose parent
    /// handle is not `Empty__` as inside a group. FOUND (01.33, 0x605b5c):
    /// `GetNumResults__` is the size of the flat result array (groups and levels). A
    /// handle or SlotID is found by object identity: only the objects made here match
    /// (INFERRED enough for the search state, which hands back what it was given).
    fn story_result_query(
        &mut self,
        class: &str,
        mangled: &str,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Option<Value> {
        let object = |v: Option<&Value>| match v {
            Some(Value::Object(Some(o))) => Some(*o),
            _ => None,
        };
        let results = &self.story_results;
        let slots = &self.story_slots;
        let this = object(receiver).and_then(|o| results.iter().position(|r| r.result == o));
        let by_handle =
            |v: Option<&Value>| object(v).and_then(|o| results.iter().position(|r| r.handle == o));
        let empty = self.empty_result_handle;
        let handle_of = |i: Option<usize>| Value::Object(i.map(|i| results[i].handle).or(empty));
        let result_of = |i: Option<usize>| Value::Object(i.map(|i| results[i].result));
        let search = &mut self.pod.search;
        Some(match (class, mangled) {
            ("ResultCollection", "GetNumResults__") => {
                Value::Int(i32::try_from(results.len()).unwrap_or(i32::MAX))
            }
            ("ResultCollection", "GetResult__i") => result_of(match args.first() {
                Some(Value::Int(i)) => usize::try_from(*i).ok().filter(|i| *i < results.len()),
                _ => None,
            }),
            ("ResultCollection", "FindResult__Q18SearchResultHandle") => {
                result_of(by_handle(args.first()))
            }
            ("ResultCollection", "FindResultSlot__Q6SlotID") => result_of(
                object(args.first()).and_then(|o| results.iter().position(|r| r.slot_id == o)),
            ),
            ("ResultCollection", "FindResultIndex__Q18SearchResultHandle") => {
                Value::Int(by_handle(args.first()).map_or(-1, |i| i32::try_from(i).unwrap_or(-1)))
            }
            ("SearchResults", "GetDefaultResult__") => handle_of(search.default_result),
            ("SearchResults", "SetDefaultResult__Q18SearchResultHandle") => {
                search.default_result = by_handle(args.first());
                Value::Void
            }
            // INFERRED: 1 is the slot kind (the SearchResultHandle of a slot has `_Type` 1,
            // and the pod computer compares the type with 1 before reading the parent).
            ("SearchResult", "GetResultType__") => Value::Int(i32::from(this.is_some())),
            ("SearchResult", "GetHandle__") => handle_of(this),
            ("SearchResult", "GetParent__") => handle_of(this.and_then(|i| results[i].parent)),
            ("SearchResult", "GetLocation__") => Value::Vec4(
                this.and_then(|i| slots.get(i))
                    .map_or([0.; 4], |s| s.location),
            ),
            // INFERRED: every story result can be picked (no lock state was read).
            ("SearchResult", "IsSelectable__") | ("SearchResult", "IsClickable__") => {
                Value::Bool(this.is_some())
            }
            ("SearchResultSlot", "GetSlotID__") => Value::Object(this.map(|i| results[i].slot_id)),
            // DEVELOPER_GROUP (4).
            ("SearchResultSlot", "IsGroup__") => {
                Value::Bool(this.and_then(|i| slots.get(i)).is_some_and(|s| s.id.0 == 4))
            }
            ("SearchResultSlot", "IsEmptyMoon__") => Value::Bool(false),
            // The Earth planet's results and cursor. INFERRED: the cursor is on the
            // default result until it is moved (`PickDefaultResult__` puts it back).
            ("Planet", "SetSearchResults__Q13SearchResults") => {
                search.set_search_results_calls += 1;
                Value::Void
            }
            ("Planet", "GetResults__") => Value::Object(self.search_results),
            ("Planet", "PickDefaultResult__") => {
                search.cur_result = search.default_result;
                Value::Void
            }
            ("Planet", "MoveCursorToResult__Q18SearchResultHandleb") => {
                if let Some(i) = by_handle(args.first()) {
                    search.cur_result = Some(i);
                }
                Value::Void
            }
            ("Planet", "GetCurResult__") => handle_of(search.cur_result.or(search.default_result)),
            // No grief report is open; the context the search state hands it is not kept.
            ("GriefReport", "SetContext__Q18SearchResultHandle") => Value::Void,
            _ => return None,
        })
    }
}

/// The callouts (hint bubbles): none is made (`CreateCallout` gives null), so none is
/// valid, active or drawn.
fn callout(mangled: &str) -> Value {
    match base(mangled) {
        "IsValid" | "IsActive" => Value::Bool(false),
        "GetFramesInState" => Value::Int(0),
        "CreateCallout" => Value::Object(None),
        _ => Value::Void,
    }
}

impl LevelScripts {
    /// The engine's per-frame draw call of the pod computer: `Draw__m` with
    /// [`pod_computer_draw_matrix`] on its PodComputer script object, in the planets level
    /// only. FOUND (01.33, 0x404598): the pod computer object's draw function makes this
    /// call and then the video call ([`LevelScripts::update_video`]), once a rendered
    /// frame. The script hands the matrix to each state's `OnDraw`; the main menu's
    /// `SetupFrameMatrix` turns it into its root frame's matrix (`SetItemViewMatrix`,
    /// [`GooeyState::item_matrices`]). A call that faults or yields is reported once in
    /// [`StepReport::thread_faults`] and not made again.
    pub fn pod_computer_draw(
        &mut self,
        vm: &mut Vm,
        host: &mut Bindings<LevelWorld>,
        budget: u64,
    ) -> StepReport {
        let mut report = StepReport::default();
        if self.pod_computer_draw_stopped || !self.is_planets_level(vm, &host.world) {
            return report;
        }
        let Some(object) = host.world.pod_computer else {
            return report;
        };
        let Some(class) = vm.class_of(object) else {
            return report;
        };
        if !vm.has_code(class, POD_COMPUTER_DRAW) {
            return report;
        }
        let args = [(
            DRAW_MATRIX_REGISTER,
            Value::Mat44(pod_computer_draw_matrix()),
        )];
        match crate::run_on_object_with(vm, host, class, object, POD_COMPUTER_DRAW, &args, budget) {
            Ok(()) => report.pod_computer_draws += 1,
            Err(e) => {
                self.pod_computer_draw_stopped = true;
                report.thread_faults.push(format!("PodComputer {e}"));
            }
        }
        report
    }
}

/// One placed item of a menu, in the root frame's units (x right, y down from its top
/// left corner): the centre, the size before the turn and the turn ([`layout`]).
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedItem {
    pub kind: PlacedKind,
    pub centre: [f32; 2],
    pub size: [f32; 2],
    /// Radians, clockwise on the screen.
    pub angle: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlacedKind {
    Button {
        id: i32,
        label: String,
        focused: bool,
        state: i32,
        /// `Gooey.EGooeyTextStyle` and the font scale in force ([`text_style`]).
        style: i32,
        font_scale: f32,
    },
    PlayerIcon,
    Text {
        text: String,
        style: i32,
        font_scale: f32,
    },
}

/// The Gooey's fonts, in the order of the script enum's names (`FONT_HELVETICA`,
/// `FONT_HEAVY`, `FONT_CRIKEY_SQUAT`, `FONT_CRIKEY_TALL`, `FONT_ICON_PATCH`; the 01.33
/// EBOOT's enum name table); INFERRED that this order is their values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GooeyFont {
    Helvetica,
    Heavy,
    CrikeySquat,
    CrikeyTall,
    IconPatch,
}

impl GooeyFont {
    pub const ALL: [Self; 5] = [
        Self::Helvetica,
        Self::Heavy,
        Self::CrikeySquat,
        Self::CrikeyTall,
        Self::IconPatch,
    ];

    pub fn from_index(i: u8) -> Option<Self> {
        Self::ALL.get(usize::from(i)).copied()
    }

    /// The font file the engine loads for it: FOUND (01.33, the GUID table at 0xc26178:
    /// 000189f0, 000189f1, 000203db, 00004144, 000305bc, after the Chinese table's four)
    /// these five fonts; which file is which enum value is INFERRED from the names
    /// (`FONT_HEAVY` the black weight of Eigerdals) and MEASURED for `FONT_HEAVY` (the
    /// main menu's letters in the game frame are Eigerdals Black's, part 72).
    pub fn map_path(self) -> &'static str {
        match self {
            Self::Helvetica => "gamedata/fonts/helveticaneueltstd-md.fnt",
            Self::Heavy => "gamedata/fonts/eigerdalsbla.fnt",
            Self::CrikeySquat => "gamedata/fonts/crikey_squat.fnt",
            Self::CrikeyTall => "gamedata/fonts/crikey_tall.fnt",
            Self::IconPatch => "gamedata/fonts/icon_patch_font.fnt",
        }
    }
}

/// FOUND (01.33, the table the Gooey's `GetFont` / `GetFontSize` natives read through
/// 0x48a518 / 0x48a584, at 0xc25cec): each `EGooeyTextStyle` value's font (an index of
/// [`GooeyFont`]) and size. A size is multiplied by a scale the engine keeps per Gooey
/// (not read; INFERRED 1 here). The main menu's buttons pass style 1: font 1, size 32.
pub const TEXT_STYLES: [(u8, f32); 26] = [
    (1, 36.),
    (1, 32.),
    (1, 32.),
    (1, 26.),
    (1, 26.),
    (2, 22.),
    (0, 26.),
    (1, 20.),
    (1, 24.),
    (1, 28.),
    (1, 32.),
    (1, 36.),
    (1, 48.),
    (1, 64.),
    (2, 22.),
    (2, 18.),
    (3, 48.),
    (2, 18.),
    (3, 48.),
    (2, 40.),
    (2, 40.),
    (2, 32.),
    (2, 32.),
    (2, 24.),
    (2, 24.),
    (0, 22.),
];

/// The size the engine gives a style outside the table (FOUND, 01.33: the constant the
/// size function falls back on), with the table's first font.
pub const DEFAULT_TEXT_SIZE: f32 = 16.;

/// The font and size of a text style ([`TEXT_STYLES`]).
pub fn text_style(style: i32) -> (GooeyFont, f32) {
    usize::try_from(style)
        .ok()
        .and_then(|i| TEXT_STYLES.get(i))
        .and_then(|&(f, size)| Some((GooeyFont::from_index(f)?, size)))
        .unwrap_or((GooeyFont::Helvetica, DEFAULT_TEXT_SIZE))
}

/// Root-frame units per font pixel at a text size: INFERRED size / 16 (a font's glyphs
/// are made for size 16). MEASURED (part 72, game frame `rt20_9601`): style 1 (size 32)
/// draws Eigerdals Black at 1.96 to 2.11 units per font pixel (cap height and ink widths of
/// Me, Store and Recent Activity mapped through the menu's matrix).
pub fn units_per_font_pixel(size: f32) -> f32 {
    size / DEFAULT_TEXT_SIZE
}

/// A loaded font's advances in whole font pixels by character. INFERRED: each advance
/// (1/64 pixel in the font file) is rounded up to a whole pixel; MEASURED (part 72) that
/// gives the game frame's ink widths of Recent, Activity, Store and Me within 2 units,
/// where unrounded advances are 6 to 19 units short on the longer words.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FontAdvances {
    by_char: HashMap<char, f32>,
}

impl FontAdvances {
    /// From (UTF-16 code unit, advance in 1/64 pixel) pairs, as a font face's glyph
    /// records give them.
    pub fn new(glyphs: impl IntoIterator<Item = (u16, i16)>) -> Self {
        let by_char = glyphs
            .into_iter()
            .filter_map(|(c, a)| Some((char::from_u32(u32::from(c))?, advance_pixels(a))))
            .collect();
        Self { by_char }
    }

    /// The advance of a text in font pixels; `None` when a character has no glyph.
    pub fn width(&self, text: &str) -> Option<f32> {
        text.chars().map(|c| self.by_char.get(&c).copied()).sum()
    }
}

/// An advance in 1/64 font pixel, rounded up to whole pixels ([`FontAdvances`]).
pub fn advance_pixels(advance: i16) -> f32 {
    (f32::from(advance) / 64.).ceil()
}

/// The Gooey's loaded fonts (any may be missing; text in a missing font is measured by the
/// fitted [`text_ems`] model instead).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GooeyFonts {
    pub faces: BTreeMap<GooeyFont, FontAdvances>,
}

impl GooeyFonts {
    /// A text's width in root-frame units at a style and font scale.
    pub fn text_width(&self, text: &str, style: i32, font_scale: f32) -> Option<f32> {
        let (font, size) = text_style(style);
        let width = self.faces.get(&font)?.width(text)?;
        Some(width * units_per_font_pixel(size) * font_scale)
    }
}

/// Gooey metrics in the root frame's units, fitted to the game frame's main menu (part 70;
/// the engine's sizes were not read). A button's text is this many units per em at font
/// scale 1 ...
pub const BUTTON_EM: f32 = 56.3;
/// ... with this much pill at the two ends together (the pill is as long as the node) ...
pub const BUTTON_PAD: f32 = 58.9;
/// ... or, measured with the game's font ([`GooeyFonts`]), this much (fitted, part 72: the
/// game frame's pills less their texts' rounded advances at 2 units a font pixel, 49.0
/// to 57.7 units over the five buttons).
pub const BUTTON_PAD_FONT: f32 = 52.6;
/// ... in a node this tall (a row: rows have no gap); the pill drawn in it is
/// [`PILL_HEIGHT`] tall.
pub const BUTTON_ROW: f32 = 103.6;
pub const PILL_HEIGHT: f32 = 82.;
/// The focused button is drawn this much larger about its centre; the layout keeps its
/// unfocused size.
pub const FOCUS_SCALE: f32 = 1.56;
/// The player's card (`DoPlayerBigIcon`): its node, and the card drawn at the node's top,
/// centred across (wider than the node).
pub const PLAYER_ICON: [f32; 2] = [167., 472.];
pub const PLAYER_CARD: [f32; 2] = [234., 291.];
/// The root frame's rows sit this much lower than the bottom-aligned rule puts them
/// (fitted: the game frame's three columns are all 14.5 units lower; not explained).
pub const ROOT_ROWS_DOWN: f32 = 14.5;

/// Width of a text in ems: a rounded sans serif's widths by character class (INFERRED,
/// fitted to the game frame's pill lengths, part 70).
pub fn text_ems(text: &str) -> f32 {
    text.chars()
        .map(|c| match c {
            'i' | 'l' | 'j' | 't' | 'f' | 'r' | ' ' | '.' | ',' | '\'' | 'I' => 0.3,
            'm' | 'w' | 'M' | 'W' => 0.85,
            c if c.is_uppercase() => 0.68,
            _ => 0.55,
        })
        .sum()
}

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Alignment of a Gooey code along one axis: the share of the free space before the item
/// (INFERRED from the main menu: 0 start, 1 end, 3 centre; the root's (3, 1) puts its rows
/// at the bottom, the left column's (1, 3) puts its pills at the right).
fn share(code: i32) -> f32 {
    match code {
        1 => 1.,
        3 => 0.5,
        _ => 0.,
    }
}

/// The size an item asks for; text measured in the game's font when it is loaded.
fn measure(item: &GooeyItem, fonts: Option<&GooeyFonts>) -> [f32; 2] {
    let font = |text: &str, style: i32, scale: f32| fonts?.text_width(text, style, scale);
    match item {
        GooeyItem::Frame(f) => frame_size(f, None, fonts),
        GooeyItem::Button(b) => [
            font(&b.label, b.text_style, b.font_scale).map_or_else(
                || BUTTON_PAD + BUTTON_EM * b.font_scale * text_ems(&b.label),
                |w| BUTTON_PAD_FONT + w,
            ),
            BUTTON_ROW,
        ],
        GooeyItem::PlayerIcon { .. } => PLAYER_ICON,
        GooeyItem::Text {
            text,
            style,
            font_scale,
        } => [
            font(text, *style, *font_scale)
                .unwrap_or_else(|| BUTTON_EM * font_scale * text_ems(text)),
            BUTTON_ROW,
        ],
        _ => [0., 0.],
    }
}

fn borders(f: &GooeyFrame) -> [f32; 4] {
    f.borders.unwrap_or([0.; 4])
}

/// Rows of items: a break ends a row.
fn rows(f: &GooeyFrame) -> Vec<Vec<&GooeyItem>> {
    let mut out = vec![Vec::new()];
    for c in &f.children {
        match c {
            GooeyItem::Break | GooeyItem::HorizontalBreak(_) => out.push(Vec::new()),
            c => out.last_mut().expect("a row").push(c),
        }
    }
    out.retain(|r| !r.is_empty());
    out
}

/// The content size of a frame's children (columns side by side, else rows stacked).
fn content(f: &GooeyFrame, fonts: Option<&GooeyFonts>) -> [f32; 2] {
    if !f.columns.is_empty() {
        let sizes: Vec<[f32; 2]> = f.children.iter().map(|c| measure(c, fonts)).collect();
        return [
            sizes.iter().map(|s| s[0]).sum(),
            sizes.iter().map(|s| s[1]).fold(0., f32::max),
        ];
    }
    let rows = rows(f);
    let mut w: f32 = 0.;
    let mut h = 0.;
    for r in &rows {
        let sizes: Vec<[f32; 2]> = r.iter().map(|c| measure(c, fonts)).collect();
        w = w.max(sizes.iter().map(|s| s[0]).sum());
        h += sizes.iter().map(|s| s[1]).fold(0., f32::max);
    }
    [w, h]
}

/// A frame's size: `SetFrameSizing` per axis (above 0: that size; below 0: the space
/// given, `fill`; 0 or none: the content and borders).
fn frame_size(f: &GooeyFrame, fill: Option<[f32; 2]>, fonts: Option<&GooeyFonts>) -> [f32; 2] {
    let b = borders(f);
    let c = content(f, fonts);
    let fit = [c[0] + b[0] + b[2], c[1] + b[1] + b[3]];
    let s = f.sizing.unwrap_or([0., 0.]);
    let axis = |i: usize| {
        if s[i] > 0. {
            s[i]
        } else if s[i] < 0. {
            fill.map_or(fit[i], |f| f[i])
        } else {
            fit[i]
        }
    };
    [axis(0), axis(1)]
}

fn place(
    item: &GooeyItem,
    r: Rect,
    ctx: (Option<i32>, Option<&GooeyFonts>),
    out: &mut Vec<PlacedItem>,
) {
    let focus = ctx.0;
    let centre = [r.x + r.w / 2., r.y + r.h / 2.];
    match item {
        GooeyItem::Frame(f) => place_frame(f, r, ctx, out),
        GooeyItem::Button(b) => {
            // The pill: as long as the node, [`PILL_HEIGHT`] tall, larger when focused.
            let focused = focus == Some(b.id);
            let k = if focused { FOCUS_SCALE } else { 1. };
            out.push(PlacedItem {
                kind: PlacedKind::Button {
                    id: b.id,
                    label: b.label.clone(),
                    focused,
                    state: b.state,
                    style: b.text_style,
                    font_scale: b.font_scale,
                },
                centre,
                size: [r.w * k, PILL_HEIGHT * k],
                angle: b.angle,
            })
        }
        GooeyItem::PlayerIcon { .. } => out.push(PlacedItem {
            kind: PlacedKind::PlayerIcon,
            centre: [centre[0], r.y + PLAYER_CARD[1] / 2.],
            size: PLAYER_CARD,
            angle: 0.,
        }),
        GooeyItem::Text {
            text,
            style,
            font_scale,
        } => out.push(PlacedItem {
            kind: PlacedKind::Text {
                text: text.clone(),
                style: *style,
                font_scale: *font_scale,
            },
            centre,
            size: [r.w, r.h],
            angle: 0.,
        }),
        _ => {}
    }
}

fn place_frame(
    f: &GooeyFrame,
    r: Rect,
    ctx: (Option<i32>, Option<&GooeyFonts>),
    out: &mut Vec<PlacedItem>,
) {
    let fonts = ctx.1;
    let b = borders(f);
    let inner = Rect {
        x: r.x + b[0],
        y: r.y + b[1],
        w: (r.w - b[0] - b[2]).max(0.),
        h: (r.h - b[1] - b[3]).max(0.),
    };
    let (ax, ay) = f.layout.map_or((0., 0.), |(x, y, _)| (share(x), share(y)));
    if !f.columns.is_empty() {
        // Columns: a width of 0 is the child's own, a negative width a share of what the
        // others leave; each child goes in the next column, aligned by the column's code
        // across and by the frame's code down.
        let sizes: Vec<[f32; 2]> = f.children.iter().map(|c| measure(c, fonts)).collect();
        let fixed: f32 = f
            .columns
            .iter()
            .zip(&sizes)
            .filter(|((w, _), _)| *w >= 0.)
            .map(|((w, _), s)| if *w > 0. { *w } else { s[0] })
            .sum();
        let weights: f32 = f
            .columns
            .iter()
            .filter(|(w, _)| *w < 0.)
            .map(|(w, _)| -w)
            .sum();
        let free = (inner.w - fixed).max(0.);
        let mut x = inner.x;
        for (i, child) in f.children.iter().enumerate() {
            let Some((w, align)) = f.columns.get(i) else {
                break;
            };
            let s = sizes[i];
            let cw = if *w > 0. {
                *w
            } else if *w < 0. && weights > 0. {
                free * -w / weights
            } else {
                s[0]
            };
            let cell = Rect {
                x: x + (cw - s[0]) * share(*align),
                y: inner.y + (inner.h - s[1]) * ay,
                w: s[0],
                h: s[1],
            };
            place(child, cell, ctx, out);
            x += cw;
        }
        return;
    }
    // Rows: stacked down from the top of the content block, which the frame's codes align;
    // in a row the items go side by side, the row aligned across by the first code.
    let rows = rows(f);
    let c = content(f, fonts);
    let mut y = inner.y + (inner.h - c[1]) * ay;
    for row in rows {
        let sizes: Vec<[f32; 2]> = row
            .iter()
            .map(|it| match it {
                GooeyItem::Frame(fr) => frame_size(fr, Some([inner.w, inner.h]), fonts),
                it => measure(it, fonts),
            })
            .collect();
        let rw: f32 = sizes.iter().map(|s| s[0]).sum();
        let rh = sizes.iter().map(|s| s[1]).fold(0., f32::max);
        let mut x = inner.x + (inner.w - rw) * ax;
        for (it, s) in row.iter().zip(&sizes) {
            let cell = Rect {
                x,
                y: y + (rh - s[1]) * 0.5,
                w: s[0],
                h: s[1],
            };
            place(it, cell, ctx, out);
            x += s[0];
        }
        y += rh;
    }
}

/// Places a menu's root frame and everything in it, in the root frame's units
/// ([`PlacedItem`]); `focus` is the focused node's id. Text is measured by the fitted
/// [`text_ems`] model ([`layout_with`] measures it in the game's fonts).
pub fn layout(root: &GooeyFrame, focus: Option<i32>) -> Vec<PlacedItem> {
    layout_with(root, focus, None)
}

/// [`layout`] with text measured in the game's fonts where they are loaded (part 72).
pub fn layout_with(
    root: &GooeyFrame,
    focus: Option<i32>,
    fonts: Option<&GooeyFonts>,
) -> Vec<PlacedItem> {
    let size = frame_size(root, None, fonts);
    let mut out = Vec::new();
    place_frame(
        root,
        Rect {
            x: 0.,
            y: 0.,
            w: size[0],
            h: size[1],
        },
        (focus, fonts),
        &mut out,
    );
    for item in &mut out {
        item.centre[1] += ROOT_ROWS_DOWN;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: &str) -> Value {
        Value::Str(t.into())
    }

    #[test]
    fn d319_the_draw_matrix_puts_the_root_frame_before_the_in_use_camera() {
        // The menu's chain at rest: the draw matrix, the menu's scale 2.8 and its offset
        // (-w/2, -h/2 - 50, 800) for its 2400 x 1250 root (part 70).
        let m = pod_computer_draw_matrix();
        let at = |x: f32, y: f32| {
            let (u, v, w) = ((x - 1200.) * 2.8, (y - 675.) * 2.8, 800. * 2.8);
            [0, 1, 2].map(|r| m[r] * u + m[4 + r] * v + m[8 + r] * w + m[12 + r])
        };
        let corner = at(0., 0.);
        for (got, want) in corner.iter().zip([-3660.8, 2835.7, -2008.6]) {
            assert!((got - want).abs() < 2., "{corner:?}");
        }
        // Root x runs along world x, root y down the picture (world y falls).
        let (right, below) = (at(100., 0.), at(0., 100.));
        assert!(right[0] - corner[0] > 300. && (right[1] - corner[1]).abs() < 2.);
        assert!(below[1] < corner[1]);
    }

    fn menu_tree(w: &mut LevelWorld) {
        let g =
            |w: &mut LevelWorld, name: &str, args: &[Value]| w.ui_call("Gooey", name, None, args);
        let f = |v: f32| Value::Float(v);
        let i = Value::Int;
        g(w, "SetScriptUIDHighBits__i", &[i(1)]);
        g(
            w,
            "StartFrameNamed__iQ23Gooey.EGooeyBubbleStyle",
            &[i(1), i(0)],
        );
        g(w, "SetFrameLayout__iif", &[i(3), i(1), f(0.5)]);
        g(w, "SetFrameSizing__ff", &[f(2400.), f(1250.)]);
        g(w, "SetFrameBorders__ffff", &[f(0.), f(0.), f(0.), f(150.)]);
        g(w, "SetFrameTransitionTime__ff", &[f(15.), f(15.)]);
        g(w, "StartFrame__Q23Gooey.EGooeyBubbleStyle", &[i(0)]);
        g(w, "SetFrameLayout__iif", &[i(3), i(3), f(0.5)]);
        g(w, "SetFrameSizing__ff", &[f(-1.), f(500.)]);
        for (width, align) in [(-0.5, 1), (0., 3), (-0.5, 0)] {
            g(w, "AddFrameColumn__fi", &[f(width), i(align)]);
        }
        g(w, "StartFrame__Q23Gooey.EGooeyBubbleStyle", &[i(0)]);
        g(w, "SetFrameLayout__ii", &[i(1), i(3)]);
        g(w, "SetFrameBorders__ff", &[f(32.), f(32.)]);
        let button =
            "DoSpaceButtonNamed__iQ6StringQ21Gooey.EGooeyTextStyleQ23Gooey.EGooeyButtonStateif";
        for (n, (id, label, angle)) in [
            (6, "Story", 0.2),
            (9, "Community", 0.075),
            (11, "Store", -0.2),
        ]
        .into_iter()
        .enumerate()
        {
            if n > 0 {
                g(w, "DoBreak__", &[]);
            }
            let r = g(w, button, &[i(id), s(label), i(1), i(0), i(256), f(angle)]);
            assert_eq!(r, Some(Value::Int(0)));
        }
        g(w, "EnsureNodeOrDescendantHasFocus__i", &[i(6)]);
        g(w, "EndFrame__", &[]);
        g(w, "StartFrame__Q23Gooey.EGooeyBubbleStyle", &[i(0)]);
        g(
            w,
            "SetFrameBorders__ffff",
            &[f(64.), f(128.), f(64.), f(0.)],
        );
        g(
            w,
            "DoPlayerBigIcon__Q7StringAQ21Gooey.EGooeyTextStyleQ23Gooey.EGooeyButtonStateibb",
            &[],
        );
        g(w, "EndFrame__", &[]);
        g(w, "StartFrame__Q23Gooey.EGooeyBubbleStyle", &[i(0)]);
        g(w, "SetFrameLayout__ii", &[i(0), i(3)]);
        g(w, "SetFrameBorders__ff", &[f(32.), f(32.)]);
        g(w, button, &[i(12), s("Me"), i(1), i(6), i(0), f(0.)]);
        g(w, "EndFrame__", &[]);
        g(w, "EndFrame__", &[]);
        g(w, "EndFrameInput__i", &[i(512)]);
        g(w, "ResetScriptUIDHighBits__", &[]);
    }

    #[test]
    fn d319_the_gooey_calls_build_the_menu_and_its_matrix() {
        let mut w = LevelWorld::default();
        menu_tree(&mut w);
        let g = &w.gooey;
        assert_eq!(g.builds, 1);
        assert_eq!(g.focus, Some((1, 6)));
        let root = &g.frames[&(1, 1)];
        assert_eq!(root.sizing, Some([2400., 1250.]));
        assert_eq!(root.borders, Some([0., 0., 0., 150.]));
        let GooeyItem::Frame(columns) = &root.children[0] else {
            panic!("{root:?}");
        };
        assert_eq!(columns.columns, [(-0.5, 1), (0., 3), (-0.5, 0)]);
        // No matrix yet: nothing drawn. The draw step asks the root's size and sets it.
        assert!(g.drawn_menu().is_none());
        let call =
            |w: &mut LevelWorld, name: &str, args: &[Value]| w.ui_call("Gooey", name, None, args);
        call(&mut w, "SetScriptUIDHighBits__i", &[Value::Int(1)]);
        assert_eq!(
            call(&mut w, "GetFrameSize__i", &[Value::Int(1)]),
            Some(Value::Vec4([2400., 1250., 0., 0.]))
        );
        let m = Value::Mat44(pod_computer_draw_matrix());
        call(&mut w, "SetItemViewMatrix__im", &[Value::Int(1), m]);
        let menu = w.gooey.drawn_menu().expect("drawn");
        assert_eq!(menu.focus, Some(6));
        assert_eq!(menu.first_built, Some(0));
        // A native the world does not answer counts as unbound.
        assert_eq!(w.ui_call("Gooey", "NoSuchNative__", None, &[]), None);
        assert_eq!(
            w.ui_call("Callout", "IsValid__Q5Gooey", None, &[]),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn d319_the_layout_puts_the_columns_about_the_card() {
        let mut w = LevelWorld::default();
        menu_tree(&mut w);
        let placed = layout(&w.gooey.frames[&(1, 1)], Some(6));
        let button = |label: &str| {
            placed
                .iter()
                .find(|p| matches!(&p.kind, PlacedKind::Button { label: l, .. } if l == label))
                .expect(label)
        };
        let (story, community, store, me) = (
            button("Story"),
            button("Community"),
            button("Store"),
            button("Me"),
        );
        let card = placed
            .iter()
            .find(|p| p.kind == PlacedKind::PlayerIcon)
            .expect("card");
        // The card in the middle; the left column's pills end at one right edge (the
        // focused one is drawn larger about its own centre); Me starts right of the card.
        assert!((card.centre[0] - 1200.).abs() < 0.01);
        let right = |p: &PlacedItem, k: f32| p.centre[0] + p.size[0] / k / 2.;
        assert!((right(community, 1.) - right(store, 1.)).abs() < 0.01);
        assert!((right(story, FOCUS_SCALE) - right(store, 1.)).abs() < 0.01);
        assert!(right(store, 1.) < card.centre[0] - PLAYER_ICON[0] / 2.);
        assert!(me.centre[0] - me.size[0] / 2. > card.centre[0] + PLAYER_ICON[0] / 2.);
        // Rows one node apart, the column and Me centred on one line.
        assert!((community.centre[1] - story.centre[1] - BUTTON_ROW).abs() < 0.01);
        assert!(((story.centre[1] + store.centre[1]) / 2. - me.centre[1]).abs() < 0.01);
        assert_eq!(story.size[1], PILL_HEIGHT * FOCUS_SCALE);
        assert!((story.angle - 0.2).abs() < 1e-6 && me.angle == 0.);
    }

    #[test]
    fn d321_text_styles_and_font_widths_size_the_pills() {
        // The menu's buttons: style 1 is the heavy font at size 32, 2 units a font pixel;
        // a style outside the table falls back to the first font at 16.
        assert_eq!(text_style(1), (GooeyFont::Heavy, 32.));
        assert_eq!(text_style(26), (GooeyFont::Helvetica, DEFAULT_TEXT_SIZE));
        assert_eq!(text_style(-1), (GooeyFont::Helvetica, DEFAULT_TEXT_SIZE));
        assert_eq!(units_per_font_pixel(32.), 2.);
        // Advances round up to whole font pixels: 'M' 1663/64 -> 26, 'e' 900/64 -> 15.
        let advances = FontAdvances::new([(u16::from(b'M'), 1663), (u16::from(b'e'), 900)]);
        assert_eq!(advance_pixels(1663), 26.);
        assert_eq!(advances.width("Me"), Some(41.));
        assert_eq!(advances.width("Mx"), None);
        let fonts = GooeyFonts {
            faces: [(GooeyFont::Heavy, advances)].into_iter().collect(),
        };
        assert_eq!(fonts.text_width("Me", 1, 1.), Some(82.));
        assert_eq!(fonts.text_width("Me", 1, 0.5), Some(41.));
        assert_eq!(fonts.text_width("Me", 6, 1.), None);
        // In the layout Me's pill is the pad and its text; the others (characters the
        // font lacks here) keep the fitted model.
        let mut w = LevelWorld::default();
        menu_tree(&mut w);
        let placed = layout_with(&w.gooey.frames[&(1, 1)], Some(6), Some(&fonts));
        let size = |label: &str| {
            placed
                .iter()
                .find(|p| matches!(&p.kind, PlacedKind::Button { label: l, .. } if l == label))
                .expect(label)
                .size
        };
        assert!((size("Me")[0] - (BUTTON_PAD_FONT + 82.)).abs() < 1e-3);
        let fitted = layout(&w.gooey.frames[&(1, 1)], Some(6));
        let store = fitted
            .iter()
            .find(|p| matches!(&p.kind, PlacedKind::Button { label, style: 1, .. } if label == "Store"))
            .expect("Store");
        assert_eq!(size("Store"), store.size);
    }

    #[test]
    fn d325_input_asks_for_a_build_and_answers_it_once() {
        let mut w = LevelWorld::default();
        let call =
            |w: &mut LevelWorld, name: &str, args: &[Value]| w.ui_call("Gooey", name, None, args);
        let needs = |w: &mut LevelWorld| call(w, "NeedsUpdate__", &[]);
        menu_tree(&mut w);
        // The first focus came with the build: nothing waits.
        assert_eq!(needs(&mut w), Some(Value::Bool(false)));
        call(&mut w, "SetScriptUIDHighBits__i", &[Value::Int(1)]);
        let m = Value::Mat44(pod_computer_draw_matrix());
        call(&mut w, "SetItemViewMatrix__im", &[Value::Int(1), m]);
        w.pworld.frame = 40;
        assert!(w.gooey.move_focus([0., 1.], None, 40));
        let id = w.gooey.focus.expect("focus").1;
        assert_ne!(id, 6);
        assert_eq!(w.gooey.focus_since, Some(40));
        assert_eq!(needs(&mut w), Some(Value::Bool(true)));
        menu_tree(&mut w);
        assert_eq!(needs(&mut w), Some(Value::Bool(false)));
        // Choose: the focused button answers 256 once, then the build is done.
        w.gooey.input.choose = true;
        assert_eq!(needs(&mut w), Some(Value::Bool(true)));
        call(&mut w, "SetScriptUIDHighBits__i", &[Value::Int(1)]);
        let button =
            "DoSpaceButtonNamed__iQ6StringQ21Gooey.EGooeyTextStyleQ23Gooey.EGooeyButtonStateif";
        call(
            &mut w,
            "StartFrameNamed__iQ23Gooey.EGooeyBubbleStyle",
            &[Value::Int(1), Value::Int(0)],
        );
        let pressed = |w: &mut LevelWorld| {
            call(
                w,
                button,
                &[
                    Value::Int(id),
                    s("Community"),
                    Value::Int(1),
                    Value::Int(0),
                    Value::Int(256),
                    Value::Float(0.),
                ],
            )
        };
        assert_eq!(pressed(&mut w), Some(Value::Int(GOOEY_ACTIVATE)));
        assert_eq!(pressed(&mut w), Some(Value::Int(0)));
        // Back: the root's EndFrameInput answers 512 once.
        w.gooey.input.back = true;
        assert_eq!(needs(&mut w), Some(Value::Bool(true)));
        assert_eq!(
            call(&mut w, "EndFrameInput__i", &[Value::Int(512)]),
            Some(Value::Int(GOOEY_BACK))
        );
        assert_eq!(needs(&mut w), Some(Value::Bool(false)));
    }
}
