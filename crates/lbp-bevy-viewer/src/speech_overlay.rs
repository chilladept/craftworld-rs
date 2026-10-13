//! The open messages of a played level on the screen (docs/level-scripts.md part 17): a
//! box per speech bubble at its anchor Thing, the subtitle at the bottom.
//!
//! Everything decided is in `lbp_runtime::message_view` (which messages, the text without
//! markup, where the boxes go); here the text is measured and painted with egui in the
//! look of the viewer's panels. INFERRED: the look. The game's bubble art, fonts and
//! placement were not read; this is a plain readable box, not the game's bubble. The
//! close prompt is only shown: no key closes a bubble yet.
use bevy_egui::egui;
use lbp_runtime::{
    place_bubbles, place_subtitle, BubbleBox, BubbleView, MessageOverlay, BUBBLE_TEXT_WIDTH,
    SUBTITLE_WIDTH,
};
use std::sync::Arc;

/// Space between a box's edge and its text, and between its lines, in screen pixels.
const PADDING: f32 = 8.;
const LINE_GAP: f32 = 4.;

/// The laid-out lines of one box, top to bottom.
struct Lines {
    galleys: Vec<Arc<egui::Galley>>,
    size: egui::Vec2,
}

impl Lines {
    fn new(galleys: Vec<Arc<egui::Galley>>) -> Self {
        let width = galleys.iter().map(|g| g.size().x).fold(0., f32::max);
        let height = galleys.iter().map(|g| g.size().y).sum::<f32>()
            + LINE_GAP * galleys.len().saturating_sub(1) as f32;
        Self {
            galleys,
            size: egui::vec2(width, height) + egui::Vec2::splat(2. * PADDING),
        }
    }

    /// Paints the box with its upper left corner at `min`; `centred` centres each line.
    fn paint(self, painter: &egui::Painter, style: &egui::Style, min: egui::Pos2, centred: bool) {
        let v = &style.visuals;
        painter.rect(
            egui::Rect::from_min_size(min, self.size),
            v.window_corner_radius,
            v.window_fill(),
            v.window_stroke(),
            egui::StrokeKind::Inside,
        );
        let mut y = min.y + PADDING;
        for g in self.galleys {
            let free = self.size.x - 2. * PADDING - g.size().x;
            let x = min.x + PADDING + if centred { free / 2. } else { 0. };
            let height = g.size().y;
            painter.galley(egui::pos2(x, y), g, v.text_color());
            y += height + LINE_GAP;
        }
    }
}

fn bubble_lines(painter: &egui::Painter, style: &egui::Style, b: &BubbleView) -> Lines {
    let v = &style.visuals;
    let body = egui::TextStyle::Body.resolve(style);
    let small = egui::TextStyle::Small.resolve(style);
    let mut galleys = Vec::new();
    if let Some(name) = &b.name {
        galleys.push(painter.layout(
            name.clone(),
            body.clone(),
            v.strong_text_color(),
            BUBBLE_TEXT_WIDTH,
        ));
    }
    galleys.push(painter.layout(b.text.clone(), body, v.text_color(), BUBBLE_TEXT_WIDTH));
    if let Some(prompt) = &b.prompt {
        galleys.push(painter.layout(
            prompt.clone(),
            small,
            v.weak_text_color(),
            BUBBLE_TEXT_WIDTH,
        ));
    }
    Lines::new(galleys)
}

/// Draws the overlay behind the viewer's windows. `to_screen` maps a place in level units
/// to screen pixels, or `None` when it is behind the camera.
pub fn draw(
    ctx: &egui::Context,
    overlay: &MessageOverlay,
    to_screen: impl Fn([f32; 3]) -> Option<[f32; 2]>,
) {
    if overlay.bubbles.is_empty() && overlay.subtitle.is_none() {
        return;
    }
    let style = ctx.global_style();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("speech_overlay"),
    ));
    let screen = ctx.content_rect();
    let origin = screen.min.to_vec2();
    let size = [screen.width(), screen.height()];
    let lines: Vec<Lines> = overlay
        .bubbles
        .iter()
        .map(|b| bubble_lines(&painter, &style, b))
        .collect();
    let boxes: Vec<BubbleBox> = overlay
        .bubbles
        .iter()
        .zip(&lines)
        .map(|(b, l)| BubbleBox {
            anchor: b.anchor,
            at: to_screen(b.position),
            size: l.size.into(),
        })
        .collect();
    for (l, placed) in lines.into_iter().zip(place_bubbles(&boxes, size)) {
        if let Some(p) = placed {
            l.paint(&painter, &style, egui::Pos2::from(p.min) + origin, false);
        }
    }
    if let Some(text) = &overlay.subtitle {
        let l = Lines::new(vec![painter.layout(
            text.clone(),
            egui::TextStyle::Heading.resolve(&style),
            style.visuals.strong_text_color(),
            size[0] * SUBTITLE_WIDTH,
        )]);
        let min = place_subtitle(l.size.into(), size);
        l.paint(&painter, &style, egui::Pos2::from(min) + origin, true);
    }
}
