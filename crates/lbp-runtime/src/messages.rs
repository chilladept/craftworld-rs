//! The messages scripts open: speech bubbles, subtitles and prompts (docs/level-scripts.md
//! part 16).
//!
//! FOUND in the 01.33 executable (the cores of the natives 0x7382e0, 0x7380f8, 0x737320,
//! 0x737f64, 0x737d7c, 0x737be8, 0x737a00; the manager's add 0x3e8dc0, find 0x3e4124, event
//! 0x3e8aa0 and sort 0x3e8928; the type records filled by 0x3e3248; the action switch
//! 0x3e528c; the actor name 0x737910):
//! - The trigger messages and the two prompts go to message manager 4 of 5. A manager
//!   holds at most 8 messages. A message keeps its type, up to five strings, its Thing, two
//!   counters that start at 0 and some flags.
//! - A native first asks the manager for a message of its type about its Thing that is
//!   not closing; when there is one the call does nothing.
//! - With 8 messages held the eighth is cleared to make room, unless its type's priority
//!   number is below the new type's. Every type here has priority 10.
//! - A type record has a flag (bit 0) for "one at a time": the new message is written over
//!   the first message of that type instead of being added, and keeps its id. The bubbles
//!   without a prompt and the subtitles have it; the bubbles with a prompt and the two
//!   prompts do not.
//! - Event 3 reaches the messages about the Thing passed with it; events 1, 2, 5, 6, 7 and
//!   8 only the first message of the list; any other event every message. A message then
//!   runs the action its type lists for the event. All types here list action 0x10 for
//!   event 3, and the subtitles also for event 9. Action 0x10 marks the message as closing.
//! - None of these types has a timed action, so nothing closes them by time.
//! - The prompt bubble keeps a prompt string (LAMS key [`CLOSE_PROMPT_KEY`]) unless the
//!   native's last argument, prevent close, is set. Neither player field of a trigger
//!   message is set (both -1).
//!
//! INFERRED:
//! - A closing message leaves the list at once. The game keeps it, flagged, until a later
//!   step that was not read; a flagged message no longer blocks a new one of its type.
//! - The list stays in the order the messages were added. The game sorts the list after
//!   each change with a comparison that was not read.
//! - A message written over starts its age again (FOUND for the prompt bubble's request,
//!   which is never written over; not read for the others).
//!
//! Not modelled:
//! - Event 2, the player's button: for a bubble with a prompt whose prevent close is not
//!   set, action 0x17 calls a function with a part of the Thing and sets the message's
//!   "triggered once" byte; for a bubble without a prompt, action 0x37 (not read). What
//!   either does to the script was not read.
//! - The state the sort gives each message (one value for a message with the "triggered
//!   once" byte, one for the first other message when it heads the list, one for the
//!   rest), and so which of several bubbles is drawn.
//! - Messages about another Thing that the game closes when a one-at-a-time message is
//!   added (0x3e4af0, by a Thing flag that was not identified).
//! - What a subtitle's text goes through before it is kept (0x736f1c), the text of the
//!   level-link and wormhole prompts (the HUD script builds it), bubble placement.
use crate::LevelWorld;
use lbp_resource::translation::lams_key;
use lbp_script_vm::Value;
use std::collections::BTreeMap;

/// Most messages a manager holds (FOUND, 0x3e8dc0).
pub const MAX_MESSAGES: usize = 8;

/// The manager the trigger messages and the prompts go to, and the one
/// `Messaging_NewGameEvent` picks for an index past 4 (FOUND, 0x7353ec).
pub const TRIGGER_MANAGER: u32 = 4;

/// The event a script sends to close its own message (FOUND: the only event whose
/// messages are picked by Thing, 0x3e8aa0).
pub const EVENT_CLOSE: i32 = 3;

/// The event that closes every subtitle (FOUND in the type record; no script sends it in
/// the measured runs).
pub const EVENT_CLOSE_SUBTITLES: i32 = 9;

/// LAMS key of the prompt string a bubble with a prompt carries (FOUND, 0x7382e0).
pub const CLOSE_PROMPT_KEY: u32 = 0xfc6e_3b66;

/// What a message is, by the native that opens it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageKind {
    /// `Messaging_NewTriggerMessage` and its Caption and Think forms.
    Bubble,
    /// The `NoPrompt` forms.
    BubbleNoPrompt,
    Subtitles,
    LevelLinkPrompt,
    WormHolePrompt,
}

/// The look of a bubble. The other kinds are [`BubbleStyle::Speech`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum BubbleStyle {
    #[default]
    Speech,
    Caption,
    Think,
}

impl MessageKind {
    /// FOUND (bit 0 of the type record's flags): a new message of the type is written
    /// over the one of that type already in the list.
    pub fn one_at_a_time(self) -> bool {
        matches!(self, Self::BubbleNoPrompt | Self::Subtitles)
    }
}

/// The game's message type number (FOUND: the constant each native's core passes).
pub fn message_type(kind: MessageKind, style: BubbleStyle) -> u32 {
    match (kind, style) {
        (MessageKind::Bubble, BubbleStyle::Speech) => 1,
        (MessageKind::BubbleNoPrompt, BubbleStyle::Speech) => 2,
        (MessageKind::Subtitles, _) => 3,
        (MessageKind::LevelLinkPrompt, _) => 0x2b,
        (MessageKind::WormHolePrompt, _) => 0x59,
        (MessageKind::Bubble, BubbleStyle::Caption) => 0x5d,
        (MessageKind::BubbleNoPrompt, BubbleStyle::Caption) => 0x5e,
        (MessageKind::Bubble, BubbleStyle::Think) => 0x5f,
        (MessageKind::BubbleNoPrompt, BubbleStyle::Think) => 0x60,
    }
}

/// One message of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// Unique in its [`Messages`], never 0. A message written over keeps it.
    pub id: u32,
    pub kind: MessageKind,
    pub style: BubbleStyle,
    /// The text as the script passed it, already translated by the script. Empty for the
    /// two prompts.
    pub text: String,
    /// The speaker's name to show, when the script passed one ([`LevelWorld::actor_name`]).
    pub actor: Option<String>,
    /// The actor name argument as the script passed it, when it is not empty.
    pub actor_tag: Option<String>,
    /// The Thing the message is about: the bubble's or the link's own Thing.
    pub thing: Option<u32>,
    /// The Thing to place the message by ([`LevelWorld::message_anchor`]).
    pub anchor: Option<u32>,
    /// The bubble position argument, 0 to 8 (meaning not established; 0 otherwise).
    pub bubble_pos: i32,
    /// The "triggered once" argument of a bubble.
    pub triggered_once: bool,
    /// The "prevent close" argument of a bubble: the player cannot close it.
    pub prevent_close: bool,
    /// The last argument of the subtitles native.
    pub has_voice: bool,
    /// `PWorld.Frame` when the message was opened or last written over.
    pub frame: i32,
}

impl Message {
    pub fn message_type(&self) -> u32 {
        message_type(self.kind, self.style)
    }
    /// Frames since the message was opened or written over, at `PWorld.Frame` `now`. The
    /// game counts in the message itself; these types have no time limit to compare with.
    pub fn age(&self, now: i32) -> i32 {
        now.wrapping_sub(self.frame)
    }
    /// The message carries the prompt string of [`CLOSE_PROMPT_KEY`] (FOUND for the bubble
    /// with a prompt; the forms without one look the key up under a test not read).
    pub fn has_close_prompt(&self) -> bool {
        self.kind == MessageKind::Bubble && !self.prevent_close
    }
}

/// What [`Messages::open`] did with a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// Added to the list, with this id.
    New(u32),
    /// Written over the message of its type, which keeps this id.
    Replaced(u32),
    /// Dropped: the Thing already has a message of the type.
    Duplicate,
}

/// Counts per message type ([`Messages::by_type`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeCounts {
    /// Calls of the native.
    pub requested: u64,
    /// Added as a new message.
    pub opened: u64,
    /// Written over the open message of the type.
    pub replaced: u64,
    /// Dropped because the Thing already had one.
    pub duplicate: u64,
}

/// The message list of manager 4 and what happened to it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Messages {
    /// The open messages in list order, at most [`MAX_MESSAGES`].
    pub open: Vec<Message>,
    /// Ids handed out so far.
    pub ids: u32,
    pub by_type: BTreeMap<(MessageKind, BubbleStyle), TypeCounts>,
    /// Messages cleared from the eighth place to make room.
    pub pushed_out: u64,
    /// Messages closed by an event.
    pub closed: u64,
    /// Events for this manager that closed nothing (no message matched, or the event has
    /// no action here).
    pub events_without_effect: u64,
    /// Events for one of the other four managers.
    pub events_elsewhere: u64,
    /// Every message that entered the list, new or written over another, oldest first,
    /// while fewer than `log_limit` are kept. For reports; 0 (the default) keeps none.
    pub log: Vec<Message>,
    pub log_limit: usize,
}

impl Messages {
    /// A request to open `new` (its id is assigned here), by the rules at the top.
    pub fn open(&mut self, mut new: Message) -> Opened {
        let ty = new.message_type();
        let counts = self.by_type.entry((new.kind, new.style)).or_default();
        counts.requested += 1;
        let same_type = |m: &Message| m.message_type() == ty;
        if self
            .open
            .iter()
            .any(|m| same_type(m) && m.thing == new.thing)
        {
            counts.duplicate += 1;
            return Opened::Duplicate;
        }
        // Every type here has the same priority, so the eighth always gives way.
        if self.open.len() >= MAX_MESSAGES {
            self.open.truncate(MAX_MESSAGES - 1);
            self.pushed_out += 1;
        }
        let over = if new.kind.one_at_a_time() {
            self.open.iter_mut().find(|m| same_type(m))
        } else {
            None
        };
        let done = match over {
            Some(old) => {
                new.id = old.id;
                *old = new.clone();
                counts.replaced += 1;
                Opened::Replaced(new.id)
            }
            None => {
                self.ids += 1;
                new.id = self.ids;
                self.open.push(new.clone());
                counts.opened += 1;
                Opened::New(new.id)
            }
        };
        if self.log.len() < self.log_limit {
            self.log.push(new);
        }
        done
    }

    /// `Messaging_NewGameEvent(manager, event, thing, flag)`. Returns the messages closed.
    pub fn event(&mut self, manager: i32, event: i32, thing: Option<u32>) -> usize {
        // FOUND (0x7353ec): an index above 4, as an unsigned number, means manager 4.
        if (manager as u32) < TRIGGER_MANAGER {
            self.events_elsewhere += 1;
            return 0;
        }
        let before = self.open.len();
        match event {
            EVENT_CLOSE => self.open.retain(|m| m.thing != thing),
            EVENT_CLOSE_SUBTITLES => self.open.retain(|m| m.kind != MessageKind::Subtitles),
            _ => {}
        }
        let closed = before - self.open.len();
        self.closed += closed as u64;
        self.events_without_effect += u64::from(closed == 0);
        closed
    }

    /// The open messages about a Thing.
    pub fn of_thing(&self, thing: u32) -> impl Iterator<Item = &Message> {
        self.open.iter().filter(move |m| m.thing == Some(thing))
    }

    /// Requests, and messages that entered the list (new or written over another).
    pub fn totals(&self) -> TypeCounts {
        self.by_type
            .values()
            .fold(TypeCounts::default(), |a, c| TypeCounts {
                requested: a.requested + c.requested,
                opened: a.opened + c.opened,
                replaced: a.replaced + c.replaced,
                duplicate: a.duplicate + c.duplicate,
            })
    }
}

/// The kind and style a messaging native opens, by the name [`LevelWorld`] records it
/// under.
pub fn message_native(name: &str) -> Option<(MessageKind, BubbleStyle)> {
    use BubbleStyle::{Caption, Speech, Think};
    use MessageKind::{Bubble, BubbleNoPrompt, LevelLinkPrompt, Subtitles, WormHolePrompt};
    Some(match name {
        "Messaging_NewTriggerMessage" => (Bubble, Speech),
        "Messaging_NewTriggerMessageNoPrompt" => (BubbleNoPrompt, Speech),
        "Messaging_NewTriggerMessageCaption" => (Bubble, Caption),
        "Messaging_NewTriggerMessageCaptionNoPrompt" => (BubbleNoPrompt, Caption),
        "Messaging_NewTriggerMessageThink" => (Bubble, Think),
        "Messaging_NewTriggerMessageThinkNoPrompt" => (BubbleNoPrompt, Think),
        "Messaging_NewTriggerMessageSubtitles" => (Subtitles, Speech),
        "Messaging_NewLevelLinkPrompt" => (LevelLinkPrompt, Speech),
        "Messaging_NewWormHolePrompt" => (WormHolePrompt, Speech),
        _ => return None,
    })
}

impl LevelWorld {
    /// The Thing a message about `thing` is placed by: its microchip when it sits on one,
    /// else itself; `None` when that Thing has no PPos.
    ///
    /// INFERRED: where the HUD draws a bubble was not read. The bubble's script measures
    /// its own distance from the screen at this Thing (FOUND, the script's off-screen
    /// test goes through the microchip proxy, [`LevelWorld::sensor_proxy`]).
    pub fn message_anchor(&self, thing: u32) -> Option<u32> {
        let has_pos = |t: u32| {
            let state = self.things.get(t as usize)?;
            state.world_matrix.map(|_| t)
        };
        has_pos(self.sensor_proxy(thing)).or_else(|| has_pos(thing))
    }

    /// Where the message's anchor is now, in level units.
    pub fn message_position(&self, message: &Message) -> Option<[f32; 3]> {
        let m = self.things.get(message.anchor? as usize)?.world_matrix?;
        Some([m[12], m[13], m[14]])
    }

    /// The speaker's name as the game shows it.
    ///
    /// FOUND (0x737910): an empty name is no name. When the level slot's type is 0 or 8
    /// the name is a translation tag: it is hashed and looked up in the loaded tables,
    /// and a tag in no table gives the game's default string. For any other slot type the
    /// name is shown as passed. INFERRED: the default string is empty, so no name.
    pub fn actor_name(&self, name: &str) -> Option<String> {
        if name.is_empty() {
            return None;
        }
        if !matches!(self.level_slot_id.0, 0 | 8) {
            return Some(name.to_owned());
        }
        let key = lams_key(name);
        let table = self.translations.iter().find(|t| t.contains_key(key))?;
        table.get(key).map(str::to_owned)
    }

    /// Feeds one recorded messaging call to the message list. The arguments were checked
    /// by the native's binding; one of another shape is left out.
    pub(crate) fn message_call(&mut self, name: &str, thing: Option<u32>, args: &[Value]) {
        if name == "Messaging_NewGameEvent" {
            if let [Value::Int(manager), Value::Int(event), ..] = args {
                self.messages.event(*manager, *event, thing);
            }
            return;
        }
        let Some((kind, style)) = message_native(name) else {
            return;
        };
        let text = |v: Option<&Value>| match v {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let flag = |v: Option<&Value>| matches!(v, Some(Value::Bool(true)));
        let mut new = Message {
            id: 0,
            kind,
            style,
            text: String::new(),
            actor: None,
            actor_tag: None,
            thing,
            anchor: thing.and_then(|t| self.message_anchor(t)),
            bubble_pos: 0,
            triggered_once: false,
            prevent_close: false,
            has_voice: false,
            frame: self.pworld.frame,
        };
        match kind {
            // (text, actor name, Thing, triggered once, bubble position, prevent close)
            MessageKind::Bubble | MessageKind::BubbleNoPrompt => {
                new.text = text(args.first());
                let name = text(args.get(1));
                new.actor = self.actor_name(&name);
                new.actor_tag = Some(name).filter(|n| !n.is_empty());
                new.triggered_once = flag(args.get(3));
                if let Some(Value::Int(pos)) = args.get(4) {
                    new.bubble_pos = *pos;
                }
                new.prevent_close = flag(args.get(5));
            }
            // (text, Thing, a flag the scripts pass as false, has voice)
            MessageKind::Subtitles => {
                new.text = text(args.first());
                new.has_voice = flag(args.get(3));
            }
            MessageKind::LevelLinkPrompt | MessageKind::WormHolePrompt => {}
        }
        self.messages.open(new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(kind: MessageKind, style: BubbleStyle, thing: u32, text: &str) -> Message {
        Message {
            id: 0,
            kind,
            style,
            text: text.into(),
            actor: None,
            actor_tag: None,
            thing: Some(thing),
            anchor: None,
            bubble_pos: 0,
            triggered_once: false,
            prevent_close: false,
            has_voice: false,
            frame: 0,
        }
    }
    fn speech(kind: MessageKind, thing: u32, text: &str) -> Message {
        message(kind, BubbleStyle::Speech, thing, text)
    }
    fn texts(m: &Messages) -> Vec<&str> {
        m.open.iter().map(|m| m.text.as_str()).collect()
    }

    #[test]
    fn type_numbers_and_the_one_at_a_time_kinds() {
        use BubbleStyle::{Caption, Speech, Think};
        use MessageKind::*;
        let types = [
            (Bubble, Speech, 1),
            (BubbleNoPrompt, Speech, 2),
            (Subtitles, Speech, 3),
            (LevelLinkPrompt, Speech, 0x2b),
            (WormHolePrompt, Speech, 0x59),
            (Bubble, Caption, 0x5d),
            (BubbleNoPrompt, Caption, 0x5e),
            (Bubble, Think, 0x5f),
            (BubbleNoPrompt, Think, 0x60),
        ];
        for (kind, style, ty) in types {
            assert_eq!(message_type(kind, style), ty);
            let one = matches!(ty, 2 | 3 | 0x5e | 0x60);
            assert_eq!(kind.one_at_a_time(), one, "type {ty:#x}");
        }
    }

    #[test]
    fn a_second_request_of_the_type_about_the_thing_is_dropped() {
        let mut m = Messages::default();
        for kind in [
            MessageKind::Bubble,
            MessageKind::BubbleNoPrompt,
            MessageKind::Subtitles,
            MessageKind::LevelLinkPrompt,
            MessageKind::WormHolePrompt,
        ] {
            let first = m.open(speech(kind, 5, "first"));
            assert!(matches!(first, Opened::New(_)), "{kind:?}");
            assert_eq!(m.open(speech(kind, 5, "again")), Opened::Duplicate);
        }
        // Five types about one Thing are five messages; the second texts are not kept.
        assert_eq!(texts(&m), ["first"; 5]);
        let ids: Vec<u32> = m.open.iter().map(|m| m.id).collect();
        assert_eq!(ids, [1, 2, 3, 4, 5]);
        let sum = m.totals();
        assert_eq!((sum.requested, sum.opened, sum.duplicate), (10, 5, 5));
        // A style is a type of its own.
        let caption = message(MessageKind::Bubble, BubbleStyle::Caption, 5, "caption");
        assert_eq!(m.open(caption), Opened::New(6));
        assert_eq!(m.of_thing(5).count(), 6);
        assert_eq!(m.of_thing(6).count(), 0);
    }

    #[test]
    fn prompt_bubbles_queue_and_the_others_are_written_over() {
        let mut m = Messages::default();
        // Bubbles with a prompt from three Things: all three stay, in order.
        for (thing, text) in [(1, "a"), (2, "b"), (3, "c")] {
            m.open(speech(MessageKind::Bubble, thing, text));
        }
        assert_eq!(texts(&m), ["a", "b", "c"]);
        // A bubble without a prompt, then one from another Thing: the second takes the
        // place and the id of the first.
        assert_eq!(
            m.open(speech(MessageKind::BubbleNoPrompt, 4, "d")),
            Opened::New(4)
        );
        let mut later = speech(MessageKind::BubbleNoPrompt, 5, "e");
        later.frame = 30;
        assert_eq!(m.open(later), Opened::Replaced(4));
        assert_eq!(texts(&m), ["a", "b", "c", "e"]);
        assert_eq!(m.open[3].thing, Some(5));
        assert_eq!((m.open[3].age(42), m.open[0].age(42)), (12, 42));
        // The Thing whose bubble was written over may ask again.
        assert_eq!(
            m.open(speech(MessageKind::BubbleNoPrompt, 4, "d")),
            Opened::Replaced(4)
        );
        // Subtitles: one at a time too, and apart from the bubbles.
        m.open(speech(MessageKind::Subtitles, 6, "s1"));
        m.open(speech(MessageKind::Subtitles, 7, "s2"));
        assert_eq!(texts(&m), ["a", "b", "c", "d", "s2"]);
        // A think bubble without a prompt does not write over the speech one.
        let think = message(MessageKind::BubbleNoPrompt, BubbleStyle::Think, 8, "t");
        assert!(matches!(m.open(think), Opened::New(_)));
        assert_eq!(m.open.len(), 6);
        let no_prompt = m.by_type[&(MessageKind::BubbleNoPrompt, BubbleStyle::Speech)];
        assert_eq!((no_prompt.opened, no_prompt.replaced), (1, 2));
    }

    #[test]
    fn the_ninth_message_takes_the_eighth_place() {
        let mut m = Messages::default();
        for thing in 0..8 {
            m.open(speech(MessageKind::Bubble, thing, "x"));
        }
        assert_eq!((m.open.len(), m.pushed_out), (MAX_MESSAGES, 0));
        assert_eq!(
            m.open(speech(MessageKind::Bubble, 8, "new")),
            Opened::New(9)
        );
        assert_eq!((m.open.len(), m.pushed_out), (MAX_MESSAGES, 1));
        let things: Vec<_> = m.open.iter().filter_map(|m| m.thing).collect();
        assert_eq!(things, [0, 1, 2, 3, 4, 5, 6, 8]);
        // A duplicate is dropped before room is made.
        assert_eq!(
            m.open(speech(MessageKind::Bubble, 3, "dup")),
            Opened::Duplicate
        );
        assert_eq!(m.pushed_out, 1);
        // Room is made before a message is written over, as in the game.
        m.open(speech(MessageKind::Subtitles, 9, "s"));
        assert_eq!((m.open.len(), m.pushed_out), (MAX_MESSAGES, 2));
        assert_eq!(m.open[7].text, "s");
    }

    #[test]
    fn events_close_by_thing_and_only_on_manager_four() {
        let mut m = Messages::default();
        m.open(speech(MessageKind::Bubble, 1, "a"));
        m.open(speech(MessageKind::Subtitles, 1, "s"));
        m.open(speech(MessageKind::Bubble, 2, "b"));
        m.open(speech(MessageKind::Subtitles, 3, "s3"));
        // Another manager, another Thing, an event without an action here: nothing.
        assert_eq!(m.event(0, EVENT_CLOSE, Some(1)), 0);
        assert_eq!(m.event(4, EVENT_CLOSE, Some(9)), 0);
        assert_eq!(m.event(4, 2, Some(1)), 0);
        assert_eq!((m.events_elsewhere, m.events_without_effect), (1, 2));
        assert_eq!(m.open.len(), 3);
        // Event 3 closes every message about the Thing.
        assert_eq!(m.event(4, EVENT_CLOSE, Some(2)), 1);
        assert_eq!(texts(&m), ["a", "s3"]);
        // An index past 4 is manager 4. The Thing may open the type again afterwards.
        assert_eq!(m.event(7, EVENT_CLOSE, Some(1)), 1);
        assert!(matches!(
            m.open(speech(MessageKind::Bubble, 1, "a2")),
            Opened::New(_)
        ));
        // Event 9 closes the subtitles, whatever Thing comes with it.
        assert_eq!(m.event(4, EVENT_CLOSE_SUBTITLES, None), 1);
        assert_eq!(texts(&m), ["a2"]);
        assert_eq!(m.closed, 3);
        // A message without a Thing is closed by an event without one.
        let mut loose = speech(MessageKind::LevelLinkPrompt, 0, "");
        loose.thing = None;
        m.open(loose);
        assert_eq!(m.event(4, EVENT_CLOSE, None), 1);
    }

    #[test]
    fn the_log_keeps_what_entered_the_list_up_to_its_limit() {
        let mut m = Messages {
            log_limit: 2,
            ..Default::default()
        };
        m.open(speech(MessageKind::BubbleNoPrompt, 1, "a"));
        m.open(speech(MessageKind::BubbleNoPrompt, 1, "dropped"));
        m.open(speech(MessageKind::BubbleNoPrompt, 2, "b"));
        m.open(speech(MessageKind::BubbleNoPrompt, 3, "past the limit"));
        let log: Vec<_> = m.log.iter().map(|m| (m.id, m.text.as_str())).collect();
        assert_eq!(log, [(1, "a"), (1, "b")]);
        let mut close = speech(MessageKind::Bubble, 1, "");
        assert!(close.has_close_prompt());
        close.prevent_close = true;
        assert!(!close.has_close_prompt());
        assert!(!m.open[0].has_close_prompt());
    }
}
