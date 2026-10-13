//! What a viewer shows of the open messages, without a window: which messages are drawn,
//! their text without the game's markup, and where the boxes go on the screen
//! (docs/level-scripts.md part 17). The Bevy viewer's `speech_overlay.rs` measures the text
//! and paints; every decision is here, where CI tests it.
//!
//! FOUND: nothing new in the executable. The message list and its rules are part 16
//! ([`crate::Messages`]).
//!
//! MEASURED (PC, the two English translation tables, 29,477 strings): the markup in the
//! strings is tags in angle brackets. Emphasis, a line break, a pad button or stick icon,
//! font face, colour and size, a player's icon and a level slot's icon. Emphasis is in
//! 1,838 places, the line break in 280, icons in 299, the font tag in 68, the player and
//! slot icons in 46. Square brackets are plain text (stage directions in subtitles), and
//! so are three strings with other words in angle brackets.
//!
//! INFERRED, all of it open (where the HUD script draws a bubble, its art and fonts were
//! not read):
//! - A bubble is a box above its anchor Thing, centred on it, and is pushed back inside
//!   the screen when the anchor is near or past an edge.
//! - Every open bubble is drawn. The game gives queued bubbles different states by a sort
//!   that was not read, so it may draw fewer.
//! - Several bubbles placed by one Thing are stacked upwards, the first of the list
//!   nearest the Thing.
//! - A caption and a think bubble are drawn like a speech bubble.
//! - A bubble whose text is empty is not drawn (66 of the pulsed run's messages).
//! - The level-link and wormhole prompts are not drawn: the HUD script builds their text.
//! - The subtitle is one box at the bottom centre; of several open ones the last.
use crate::{BubbleStyle, LevelWorld, Message, MessageKind, CLOSE_PROMPT_KEY};

/// The tags that only change how the text between them looks; they are left out and the
/// text between them is kept (MEASURED, the list at the top).
const LOOK_TAGS: [&str; 5] = ["em", "i", "font", "player", "slot"];

/// A message string for a viewer that has neither the game's fonts nor its icons: the
/// markup tags are taken out, a line break tag is a new line and an icon tag is the
/// icon's name in square brackets. Anything else in angle brackets is text and stays.
/// Control characters other than the new line are dropped, and the ends are trimmed.
///
/// INFERRED: what the game does with a tag it does not know was not read.
pub fn display_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let tail = &rest[open..];
        match tail.find('>').and_then(|close| {
            let shown = tag_text(&tail[1..close])?;
            Some((shown, close))
        }) {
            Some((shown, close)) => {
                out.push_str(&shown);
                rest = &tail[close + 1..];
            }
            None => {
                out.push('<');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out.retain(|c| c == '\n' || !c.is_control());
    out.trim().to_owned()
}

/// What stands in for the tag with this inside, or `None` when it is not a known tag.
fn tag_text(inside: &str) -> Option<String> {
    let inside = inside.trim();
    let inside = inside.strip_prefix('/').unwrap_or(inside);
    let inside = inside.strip_suffix('/').unwrap_or(inside).trim();
    let end = inside
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(inside.len());
    let (name, arguments) = inside.split_at(end);
    // `<i>` is a tag; a word that starts with those letters is not.
    if !arguments.is_empty() && !arguments.starts_with(char::is_whitespace) {
        return None;
    }
    let name = name.to_ascii_lowercase();
    let arguments = arguments.trim();
    match name.as_str() {
        "br" => Some("\n".into()),
        "icon" if arguments.is_empty() => Some(String::new()),
        "icon" => Some(format!("[{arguments}]")),
        _ if LOOK_TAGS.contains(&name.as_str()) => Some(String::new()),
        _ => None,
    }
}

/// One bubble to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct BubbleView {
    /// The message's id ([`Message::id`]).
    pub id: u32,
    /// The Thing the bubble is placed by. Bubbles with the same one are stacked.
    pub anchor: u32,
    /// The anchor's place in level units.
    pub position: [f32; 3],
    pub style: BubbleStyle,
    /// The speaker's name, when the script passed one that is shown.
    pub name: Option<String>,
    pub text: String,
    /// The close prompt's string, when the message carries one and the tables have it.
    pub prompt: Option<String>,
}

/// What is drawn of a message list, and what is left out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MessageOverlay {
    /// In list order.
    pub bubbles: Vec<BubbleView>,
    pub subtitle: Option<String>,
    /// Level-link and wormhole prompts in the list (not drawn).
    pub prompts_not_drawn: usize,
    /// Bubbles and subtitles with no text to show (not drawn).
    pub empty_not_drawn: usize,
    /// Bubbles with text whose anchor has no position (not drawn).
    pub unplaced_not_drawn: usize,
}

impl MessageOverlay {
    /// Picks what to draw of `open` by the rules at the top. `position` gives a message's
    /// place in level units; `close_prompt` is the string of [`CLOSE_PROMPT_KEY`].
    pub fn of(
        open: &[Message],
        position: impl Fn(&Message) -> Option<[f32; 3]>,
        close_prompt: Option<&str>,
    ) -> Self {
        let mut out = Self::default();
        for m in open {
            let text = display_text(&m.text);
            match m.kind {
                MessageKind::LevelLinkPrompt | MessageKind::WormHolePrompt => {
                    out.prompts_not_drawn += 1;
                }
                _ if text.is_empty() => out.empty_not_drawn += 1,
                MessageKind::Subtitles => out.subtitle = Some(text),
                MessageKind::Bubble | MessageKind::BubbleNoPrompt => {
                    let (Some(anchor), Some(position)) = (m.anchor, position(m)) else {
                        out.unplaced_not_drawn += 1;
                        continue;
                    };
                    let shown = |s: &str| Some(display_text(s)).filter(|s| !s.is_empty());
                    out.bubbles.push(BubbleView {
                        id: m.id,
                        anchor,
                        position,
                        style: m.style,
                        name: m.actor.as_deref().and_then(shown),
                        text,
                        prompt: close_prompt
                            .filter(|_| m.has_close_prompt())
                            .and_then(shown),
                    });
                }
            }
        }
        out
    }
}

impl LevelWorld {
    /// The string of the close prompt ([`CLOSE_PROMPT_KEY`]) in the loaded translation
    /// tables; `None` when no table has it, and then a viewer shows no prompt.
    pub fn close_prompt(&self) -> Option<&str> {
        self.translations
            .iter()
            .find(|t| t.contains_key(CLOSE_PROMPT_KEY))?
            .get(CLOSE_PROMPT_KEY)
    }

    /// What a viewer draws of the open messages now.
    pub fn message_overlay(&self) -> MessageOverlay {
        MessageOverlay::of(
            &self.messages.open,
            |m| self.message_position(m),
            self.close_prompt(),
        )
    }
}

/// Screen pixels between a bubble's lower edge and its anchor (INFERRED: above the Thing).
pub const BUBBLE_GAP: f32 = 24.;
/// Screen pixels between stacked bubbles of one Thing.
pub const STACK_SPACING: f32 = 6.;
/// Screen pixels kept free between a box and the screen's edge.
pub const EDGE_MARGIN: f32 = 8.;
/// Screen pixels between the subtitle's lower edge and the screen's (INFERRED).
pub const SUBTITLE_BOTTOM: f32 = 48.;
/// Widest text of a bubble in screen pixels; longer lines wrap (INFERRED).
pub const BUBBLE_TEXT_WIDTH: f32 = 300.;
/// Widest subtitle text as a part of the screen's width (INFERRED).
pub const SUBTITLE_WIDTH: f32 = 0.7;

/// A bubble to place: its anchor Thing, where that is on the screen (`None` when it is
/// behind the camera) and the box's size, all in screen pixels with y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleBox {
    pub anchor: u32,
    pub at: Option<[f32; 2]>,
    pub size: [f32; 2],
}

/// Where a bubble's box goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// The box's upper left corner.
    pub min: [f32; 2],
    /// The box was moved to stay on the screen, so it is no longer above its anchor.
    pub clamped: bool,
}

/// Places the boxes on a screen of `screen` pixels, in the order given. The bubbles of
/// one anchor form a stack above it, the first lowest; the stack as a whole is moved back
/// inside the screen (less [`EDGE_MARGIN`]) when it would leave it. A stack larger than
/// the screen keeps its upper left corner inside. A bubble whose anchor is behind the
/// camera, or not a number, is not placed.
///
/// INFERRED: the whole rule. Stacks of different Things may overlap.
pub fn place_bubbles(boxes: &[BubbleBox], screen: [f32; 2]) -> Vec<Option<Placed>> {
    let mut placed = vec![None; boxes.len()];
    for (first, b) in boxes.iter().enumerate() {
        // Each stack is laid out when its first bubble comes up.
        if boxes[..first].iter().any(|e| e.anchor == b.anchor) {
            continue;
        }
        let Some(at) = b.at.filter(|at| at.iter().all(|v| v.is_finite())) else {
            continue;
        };
        let stack: Vec<usize> = (first..boxes.len())
            .filter(|&i| boxes[i].anchor == b.anchor)
            .collect();
        let width = stack.iter().map(|&i| boxes[i].size[0]).fold(0., f32::max);
        let height = stack.iter().map(|&i| boxes[i].size[1]).sum::<f32>()
            + STACK_SPACING * (stack.len() - 1) as f32;
        let wanted = [at[0] - width / 2., at[1] - BUBBLE_GAP - height];
        let inside =
            |v: f32, size: f32, screen: f32| v.min(screen - EDGE_MARGIN - size).max(EDGE_MARGIN);
        let min = [
            inside(wanted[0], width, screen[0]),
            inside(wanted[1], height, screen[1]),
        ];
        let clamped = min != wanted;
        let mut bottom = min[1] + height;
        for i in stack {
            let [w, h] = boxes[i].size;
            placed[i] = Some(Placed {
                min: [min[0] + (width - w) / 2., bottom - h],
                clamped,
            });
            bottom -= h + STACK_SPACING;
        }
    }
    placed
}

/// The upper left corner of the subtitle's box of `size` on a screen of `screen` pixels:
/// centred, [`SUBTITLE_BOTTOM`] above the lower edge, and inside the screen when it fits.
pub fn place_subtitle(size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    [
        ((screen[0] - size[0]) / 2.).max(EDGE_MARGIN),
        (screen[1] - SUBTITLE_BOTTOM - size[1]).max(EDGE_MARGIN),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(kind: MessageKind, thing: u32, text: &str) -> Message {
        Message {
            id: thing + 100,
            kind,
            style: BubbleStyle::Speech,
            text: text.into(),
            actor: None,
            actor_tag: None,
            thing: Some(thing),
            anchor: Some(thing),
            bubble_pos: 0,
            triggered_once: false,
            prevent_close: false,
            has_voice: false,
            frame: 0,
        }
    }

    #[test]
    fn markup_tags_do_not_reach_the_screen() {
        // Made-up strings in the shape of the game's markup.
        for (raw, shown) in [
            ("Plain words.", "Plain words."),
            (
                "A <em>loud</em> word and an <i>odd</i> one",
                "A loud word and an odd one",
            ),
            (
                "First line<br>second<br/>third<BR />fourth",
                "First line\nsecond\nthird\nfourth",
            ),
            (
                "Press <icon cross> or tilt <icon lstick>!",
                "Press [cross] or tilt [lstick]!",
            ),
            (
                "<font color=\"white\" face=\"tall\" size=60>Big</font> small",
                "Big small",
            ),
            (
                "<player icon=%1>Someone</player> went to <slot icon=%3>a place</slot>",
                "Someone went to a place",
            ),
            ("<icon>", ""),
            // Not tags: other words in angle brackets, a lone bracket, square brackets.
            ("<NOISE OFF> he said", "<NOISE OFF> he said"),
            ("<island> <i'm> <fonts>", "<island> <i'm> <fonts>"),
            ("3 < 4 and 5 > 2", "3 < 4 and 5 > 2"),
            ("a < b <em>c</em>", "a < b c"),
            ("[sighs] Well.", "[sighs] Well."),
            ("unclosed <em", "unclosed <em"),
            // Control characters go, the new line stays, the ends are trimmed.
            ("  tab\there\r\nnext\u{7}  ", "tabhere\nnext"),
            ("<em></em>", ""),
        ] {
            assert_eq!(display_text(raw), shown, "{raw:?}");
        }
    }

    #[test]
    fn the_overlay_draws_bubbles_with_text_and_the_last_subtitle() {
        use MessageKind::*;
        let mut prompt = message(Bubble, 1, "With a <em>prompt</em>");
        prompt.actor = Some("<font face=\"tall\">Made-up Name</font>".into());
        let mut held = message(Bubble, 2, "Cannot be closed");
        held.prevent_close = true;
        let mut caption = message(BubbleNoPrompt, 3, "A caption");
        caption.style = BubbleStyle::Caption;
        let mut on_chip = message(BubbleNoPrompt, 4, "On a chip");
        on_chip.anchor = Some(9);
        let mut loose = message(Bubble, 5, "No anchor");
        loose.anchor = None;
        let open = [
            prompt,
            held,
            caption,
            on_chip,
            loose,
            message(Bubble, 6, "Nowhere"),
            message(BubbleNoPrompt, 7, "<em></em>"),
            message(Subtitles, 1, "First <i>line</i>"),
            message(Subtitles, 2, ""),
            message(LevelLinkPrompt, 1, ""),
            message(WormHolePrompt, 1, ""),
        ];
        // Thing 6 has no position; every other Thing sits at its number.
        let position = |m: &Message| m.anchor.filter(|&a| a != 6).map(|a| [a as f32, 0., 0.]);
        let o = MessageOverlay::of(&open, position, Some("Shut"));
        let drawn: Vec<_> = o
            .bubbles
            .iter()
            .map(|b| (b.id, b.anchor, b.text.as_str(), b.prompt.as_deref()))
            .collect();
        assert_eq!(
            drawn,
            [
                (101, 1, "With a prompt", Some("Shut")),
                (102, 2, "Cannot be closed", None),
                (103, 3, "A caption", None),
                (104, 9, "On a chip", None),
            ]
        );
        assert_eq!(o.bubbles[0].name.as_deref(), Some("Made-up Name"));
        assert_eq!(o.bubbles[1].name, None);
        assert_eq!(o.bubbles[2].style, BubbleStyle::Caption);
        assert_eq!(o.bubbles[3].position, [9., 0., 0.]);
        // The empty subtitle does not hide the one with text.
        assert_eq!(o.subtitle.as_deref(), Some("First line"));
        assert_eq!(
            (o.prompts_not_drawn, o.empty_not_drawn, o.unplaced_not_drawn),
            (2, 2, 2)
        );
        // Without the tables there is no prompt string, so no prompt line.
        let o = MessageOverlay::of(&open[..1], position, None);
        assert_eq!(o.bubbles[0].prompt, None);
        // Of two subtitles with text the later one is shown.
        let two = [message(Subtitles, 1, "one"), message(Subtitles, 2, "two")];
        let o = MessageOverlay::of(&two, position, None);
        assert_eq!((o.subtitle.as_deref(), o.bubbles.len()), (Some("two"), 0));
    }

    const SCREEN: [f32; 2] = [1280., 720.];

    fn one(at: [f32; 2], size: [f32; 2]) -> Placed {
        let b = BubbleBox {
            anchor: 1,
            at: Some(at),
            size,
        };
        place_bubbles(&[b], SCREEN)[0].unwrap()
    }

    #[test]
    fn a_bubble_sits_above_its_anchor_and_stays_on_the_screen() {
        // In the open: centred on the anchor, its lower edge the gap above it.
        let p = one([640., 400.], [200., 80.]);
        assert_eq!(p.min, [540., 400. - BUBBLE_GAP - 80.]);
        assert!(!p.clamped);
        // Past each edge, and past a corner: moved back inside, less the margin.
        let right = SCREEN[0] - EDGE_MARGIN - 200.;
        let low = SCREEN[1] - EDGE_MARGIN - 80.;
        for (at, min) in [
            ([-500., 400.], [EDGE_MARGIN, 296.]),
            ([5000., 400.], [right, 296.]),
            ([640., -300.], [540., EDGE_MARGIN]),
            ([640., 50.], [540., EDGE_MARGIN]),
            ([640., 2000.], [540., low]),
            ([-1., -1.], [EDGE_MARGIN, EDGE_MARGIN]),
            ([1e9, 1e9], [right, low]),
        ] {
            let p = one(at, [200., 80.]);
            assert_eq!(p.min, min, "{at:?}");
            assert!(p.clamped, "{at:?}");
        }
        // A box larger than the screen keeps its upper left corner inside.
        let p = one([640., 400.], [2000., 900.]);
        assert_eq!(p.min, [EDGE_MARGIN, EDGE_MARGIN]);
    }

    #[test]
    fn bubbles_of_one_thing_stack_upwards_and_others_are_placed_apart() {
        let b = |anchor, at: [f32; 2], size| BubbleBox {
            anchor,
            at: Some(at),
            size,
        };
        let boxes = [
            b(1, [300., 500.], [200., 60.]),
            b(2, [900., 300.], [100., 40.]),
            b(1, [300., 500.], [120., 30.]),
            b(1, [300., 500.], [260., 50.]),
        ];
        let p: Vec<Placed> = place_bubbles(&boxes, SCREEN)
            .into_iter()
            .map(Option::unwrap)
            .collect();
        // The stack is as wide as its widest bubble; each bubble is centred in it.
        let bottom = 500. - BUBBLE_GAP;
        assert_eq!(p[0].min, [200., bottom - 60.]);
        assert_eq!(p[2].min, [240., bottom - 60. - STACK_SPACING - 30.]);
        assert_eq!(
            p[3].min,
            [170., bottom - 60. - 30. - 2. * STACK_SPACING - 50.]
        );
        // The other Thing's bubble is on its own.
        assert_eq!(p[1].min, [850., 300. - BUBBLE_GAP - 40.]);
        assert!(p.iter().all(|p| !p.clamped));
        // No two boxes of the stack overlap, and the order holds: first lowest.
        assert!(p[0].min[1] >= p[2].min[1] + 30. + STACK_SPACING);
        assert!(p[2].min[1] >= p[3].min[1] + 50. + STACK_SPACING);

        // Near the top the whole stack moves down together and keeps its order.
        let high: Vec<BubbleBox> = [60., 30., 50.]
            .into_iter()
            .map(|h| b(7, [300., 40.], [200., h]))
            .collect();
        let p: Vec<Placed> = place_bubbles(&high, SCREEN)
            .into_iter()
            .map(Option::unwrap)
            .collect();
        assert!(p.iter().all(|p| p.clamped));
        assert_eq!(p[2].min[1], EDGE_MARGIN);
        assert_eq!(p[1].min[1], EDGE_MARGIN + 50. + STACK_SPACING);
        assert_eq!(p[0].min[1], EDGE_MARGIN + 50. + 30. + 2. * STACK_SPACING);
    }

    #[test]
    fn an_anchor_behind_the_camera_places_nothing() {
        let boxes = [
            BubbleBox {
                anchor: 1,
                at: None,
                size: [100., 40.],
            },
            BubbleBox {
                anchor: 2,
                at: Some([f32::NAN, 10.]),
                size: [100., 40.],
            },
            BubbleBox {
                anchor: 3,
                at: Some([640., 360.]),
                size: [100., 40.],
            },
            BubbleBox {
                anchor: 1,
                at: None,
                size: [100., 40.],
            },
        ];
        let p = place_bubbles(&boxes, SCREEN);
        assert_eq!(
            p.iter().map(Option::is_some).collect::<Vec<_>>(),
            [false, false, true, false]
        );
        assert!(place_bubbles(&[], SCREEN).is_empty());
    }

    #[test]
    fn the_subtitle_is_centred_at_the_bottom() {
        assert_eq!(
            place_subtitle([400., 50.], SCREEN),
            [440., 720. - SUBTITLE_BOTTOM - 50.]
        );
        // Wider or taller than the screen: the upper left corner stays inside.
        assert_eq!(
            place_subtitle([2000., 900.], SCREEN),
            [EDGE_MARGIN, EDGE_MARGIN]
        );
    }
}
