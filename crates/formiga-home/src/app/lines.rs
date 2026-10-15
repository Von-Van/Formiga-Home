//! Writing on the notes page's ruled lines. The page's lines scroll with what is written on it
//! (`pages::ruling`); this is how what is written finds them. Each line of text sits on a rule,
//! however it wraps, a row of things sits on one rule together, and whatever comes after a
//! picture or a tile goes back to the first rule clear of it. Desktop's notebook writes on its
//! lines the same way.
//!
//! The grid is kept for the frame while the notes are drawn, and only then: anything written off
//! the page, or on a card over it, is an ordinary label.

use eframe::egui::{self, Rect, Ui, pos2, vec2};
use std::sync::Arc;

/// Where the notes page's ruled lines are this frame: one every `pitch` below `top`, wherever the
/// scroll has put the page's writing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuleGrid {
    pub top: f32,
    pub pitch: f32,
}

impl RuleGrid {
    fn id() -> egui::Id {
        egui::Id::new("notes-rule-grid")
    }

    /// Keeps this grid for what is written on the page from here on.
    pub fn keep(self, ctx: &egui::Context) {
        ctx.data_mut(|data| data.insert_temp(Self::id(), self));
    }

    /// Lets the grid go once the page has been written, so nothing off it lines up with it.
    pub fn forget(ctx: &egui::Context) {
        ctx.data_mut(|data| data.remove::<Self>(Self::id()));
    }

    pub fn of(ctx: &egui::Context) -> Option<Self> {
        ctx.data(|data| data.get_temp(Self::id()))
    }

    /// The first ruled line at or below `y`.
    pub fn rule_at_or_below(self, y: f32) -> f32 {
        self.top + ((y - self.top) / self.pitch - 1e-3).ceil() * self.pitch
    }
}

/// How far above its rule a line of writing's baseline sits.
const LIFT: f32 = 2.0;

/// Moves what is written next down to the page's next ruled line: the first at or below the
/// bottom of whatever came before. Nothing happens off the page or within a row.
pub fn to_next_rule(ui: &mut Ui) {
    let Some(grid) = RuleGrid::of(ui.ctx()) else {
        return;
    };
    if ui.layout().main_dir().is_horizontal() {
        return;
    }
    let y = ui.cursor().top();
    let rule = grid.rule_at_or_below(y - ui.spacing().item_spacing.y);
    ui.add_space(rule - y);
}

/// Writes `text` on the next ruled line, each line of it, however it wraps, on a rule of its own;
/// text too tall for one line takes two.
pub fn write(ui: &mut Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    write_with(ui, text, egui::Sense::hover())
}

/// `write`, answering to `sense`: a link written on the page, say.
pub fn write_with(
    ui: &mut Ui,
    text: impl Into<egui::WidgetText>,
    sense: egui::Sense,
) -> egui::Response {
    let text = text.into();
    let Some(grid) = RuleGrid::of(ui.ctx()) else {
        return ui.add(egui::Label::new(text).wrap().sense(sense));
    };
    to_next_rule(ui);
    let width = ui.available_width().max(24.0);
    let mut job = Arc::unwrap_or_clone(text.into_layout_job(
        ui.style(),
        egui::FontSelection::Default,
        egui::Align::Min,
    ));
    job.wrap.max_width = width;
    let natural = ui
        .painter()
        .layout_job(job.clone())
        .rows
        .iter()
        .map(|row| row.height())
        .fold(0.0_f32, f32::max);
    let line = (natural / grid.pitch).ceil().max(1.0) * grid.pitch;
    for section in &mut job.sections {
        section.format.line_height = Some(line);
    }
    let galley = ui.painter().layout_job(job);
    let rows = galley.rows.len().max(1) as f32;
    let baseline = baseline_of(&galley).unwrap_or(line - LIFT);
    let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x, rows * line), sense);
    if ui.is_rect_visible(rect) {
        let at = pos2(rect.left(), rect.top() + line - LIFT - baseline);
        ui.painter().galley(at, galley, ui.visuals().text_color());
    }
    if sense.senses_click() && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Where a galley's first line rests, down from its top.
pub fn baseline_of(galley: &egui::Galley) -> Option<f32> {
    galley
        .rows
        .first()
        .and_then(|row| row.glyphs.first())
        .map(|glyph| glyph.pos.y)
}

/// Where to put `galley`'s top so its first line rests on the rule at `rule`.
pub fn on_rule(galley: &egui::Galley, rule: f32) -> f32 {
    rule - LIFT - baseline_of(galley).unwrap_or(galley.size().y)
}

/// A row of things written on the next ruled line, wrapping onto the lines below: each thing in
/// it written with `write` sits on its row's rule.
pub fn ruled_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    to_next_rule(ui);
    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
        |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui)
        },
    )
}

/// A band `lines` rules tall on the next ruled line, for something drawn by hand to sit on its
/// last rule, as a resident's row does: its portrait standing and its name resting on that rule.
pub fn band(ui: &mut Ui, lines: f32, fallback: f32) -> Rect {
    let height = match RuleGrid::of(ui.ctx()) {
        Some(grid) => {
            to_next_rule(ui);
            lines * grid.pitch
        }
        None => fallback,
    };
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_rule_is_never_above_where_it_is_asked_from() {
        let grid = RuleGrid {
            top: 10.0,
            pitch: 18.0,
        };
        assert_eq!(grid.rule_at_or_below(10.0), 10.0);
        assert_eq!(grid.rule_at_or_below(11.0), 28.0);
        assert_eq!(grid.rule_at_or_below(28.0), 28.0);
        assert_eq!(grid.rule_at_or_below(-30.0), -26.0);
    }
}
