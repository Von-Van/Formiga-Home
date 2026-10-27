//! Writing on the notes page's ruled lines. The page's lines scroll with what is written on it
//! (`pages::ruling`); this is how what is written finds them. Each line of text rests on a rule,
//! however it wraps, a row of things rests on one rule together, and whatever comes after a
//! picture or a tile goes back to the first rule clear of it. It is formiga-pencil's, so Home's
//! notebook writes on its lines exactly as Desktop's and Battle's do.
//!
//! The grid is kept for the frame while the notes are drawn, and only then: anything written off
//! the page, or on a card over it, is an ordinary label.

use eframe::egui::{self, Ui};

pub use formiga_pencil::ruled::{RuleGrid, band, on_rule, write};

/// `write`, answering to `sense`: a link written on the page, say.
pub fn write_with(
    ui: &mut Ui,
    text: impl Into<egui::WidgetText>,
    sense: egui::Sense,
) -> egui::Response {
    formiga_pencil::ruled::write_with(ui, text, None, sense, None)
}

/// A row of things written on the next ruled line, wrapping onto the lines below: each thing in
/// it written with `write` rests on its row's rule.
pub fn ruled_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    formiga_pencil::ruled::ruled_row_wrapping(ui, add)
}
