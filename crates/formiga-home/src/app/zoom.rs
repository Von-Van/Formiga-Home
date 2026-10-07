//! How close the house is seen on its page, and what brings it closer, takes it further and moves
//! it about: a pinch, a scroll, a drag across the floor, the keys, and three small buttons at the
//! page's corner.

use super::*;

/// How the house is shown on its page: as big as fits, or zoomed in by whole pixels and moved
/// about, for a house that has grown too big to see closely all at once.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Zoom {
    /// Screen pixels to each of the picture's; none for as many as fit.
    pixels: Option<f32>,
    /// As many as fit, on the page as it was last drawn.
    pub(super) fit: f32,
    /// How far the picture is moved from the middle of its page, in points.
    pan: egui::Vec2,
    /// Being moved about by a drag across the floor.
    pub(super) panning: bool,
    /// A pinch, or a scroll with Ctrl or ⌘, adding up until it makes a step.
    pinch: f32,
}

/// The closest the house can be seen: this many screen pixels to each of its own.
const CLOSEST: f32 = 12.0;

impl Zoom {
    /// Where the picture goes on its page, `size` pixels of it: centred where it fits, and moved
    /// no further than keeps the page covered where it does not.
    pub(super) fn place(
        &mut self,
        page: egui::Rect,
        pixels_per_point: f32,
        size: (u32, u32),
    ) -> egui::Rect {
        let pixels = self.pixels.unwrap_or(self.fit);
        let shown = egui::vec2(size.0 as f32, size.1 as f32) * pixels / pixels_per_point;
        let room = ((shown - page.size()) / 2.0).max(egui::Vec2::ZERO);
        self.pan = self.pan.clamp(-room, room);
        egui::Rect::from_center_size(page.center() + self.pan, shown)
    }

    pub(super) fn closer_than_fits(&self) -> bool {
        self.pixels.is_some_and(|pixels| pixels > self.fit)
    }

    /// A whole pixel closer (`by` 1) or further (-1), keeping what is at `anchor`, a point from
    /// the page's middle, where it is. Never further than fits.
    pub(super) fn step(&mut self, by: i32, anchor: egui::Vec2) {
        let old = self.pixels.unwrap_or(self.fit).max(0.1);
        let new = old.floor() + by as f32;
        self.pixels = if new <= self.fit.floor() {
            None
        } else {
            Some(new.min(CLOSEST))
        };
        let new = self.pixels.unwrap_or(self.fit).max(0.1);
        self.pan = anchor - (anchor - self.pan) * (new / old);
        if self.pixels.is_none() {
            self.pan = egui::Vec2::ZERO;
        }
    }

    pub(super) fn fit(&mut self) {
        self.pixels = None;
        self.pan = egui::Vec2::ZERO;
    }
}

/// How many screen pixels to each of the house's picture fit its page: a whole number where at
/// least one does, so every pixel stays square.
pub(super) fn fit_pixels(page: egui::Rect, pixels_per_point: f32, size: (u32, u32)) -> f32 {
    let fit = (page.width() * pixels_per_point / size.0 as f32)
        .min(page.height() * pixels_per_point / size.1 as f32);
    if fit >= 1.0 {
        fit.floor()
    } else {
        fit.max(0.1)
    }
}

impl HomeApp {
    /// Moving about a house seen close, and coming closer or going further: a scroll moves it,
    /// a drag across the floor moves it, and a pinch, or a scroll with Ctrl or ⌘, zooms.
    pub(super) fn zoom_input(
        &mut self,
        response: &egui::Response,
        ctx: &egui::Context,
        page: egui::Rect,
    ) {
        let (scroll, pinch, moved, down, at) = ctx.input(|input| {
            (
                input.smooth_scroll_delta,
                input.zoom_delta(),
                input.pointer.delta(),
                input.pointer.primary_down(),
                input.pointer.latest_pos(),
            )
        });
        if self.zoom.panning {
            if down {
                self.zoom.pan += moved;
            } else {
                self.zoom.panning = false;
            }
        }
        if !response.contains_pointer() {
            return;
        }
        let anchor = at.map_or(egui::Vec2::ZERO, |at| at - page.center());
        if (pinch - 1.0).abs() > f32::EPSILON {
            self.zoom.pinch += pinch.ln();
            if self.zoom.pinch > 0.2 {
                self.zoom.step(1, anchor);
                self.zoom.pinch = 0.0;
            } else if self.zoom.pinch < -0.2 {
                self.zoom.step(-1, anchor);
                self.zoom.pinch = 0.0;
            }
        } else if scroll != egui::Vec2::ZERO && self.zoom.closer_than_fits() {
            self.zoom.pan += scroll;
        }
    }

    /// Closer, as big as fits, and further: three small buttons on a card at the page's corner.
    pub(super) fn zoom_controls(&mut self, ui: &mut egui::Ui, page: egui::Rect, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let cell = egui::vec2(15.0 * unit, 12.0 * unit);
        let card = egui::Rect::from_min_size(
            page.max - egui::vec2(cell.x * 3.0 + 3.0 * unit, cell.y + 3.0 * unit),
            egui::vec2(cell.x * 3.0, cell.y),
        );
        let painter = ui.painter().clone();
        painter.rect_filled(card.shrink(unit), 0.0, notebook::ink::card(dark));
        pages::stepped(
            &painter,
            card,
            unit,
            notebook::ink::line(dark).gamma_multiply(0.7),
        );
        let fitted = self.zoom.pixels.is_none();
        let cells = [
            ("out", "\u{2212}", "Further away (\u{2212})"),
            ("fit", "Fit", "The whole house (0)"),
            ("in", "+", "Closer (+)"),
        ];
        for (index, (name, label, hint)) in cells.into_iter().enumerate() {
            let rect =
                egui::Rect::from_min_size(card.min + egui::vec2(cell.x * index as f32, 0.0), cell);
            let response = ui
                .interact(rect, egui::Id::new(("zoom", name)), egui::Sense::click())
                .on_hover_text(hint);
            let lit = response.hovered() || (name == "fit" && fitted);
            if lit {
                painter.rect_filled(
                    rect.shrink(unit),
                    0.0,
                    notebook::ink::mint(dark).gamma_multiply(if response.hovered() {
                        1.0
                    } else {
                        0.6
                    }),
                );
            }
            let galley = painter.layout_no_wrap(
                label.to_owned(),
                egui::FontId::proportional(if name == "fit" { 11.0 } else { 14.0 }),
                notebook::ink::page(dark),
            );
            painter.galley(
                rect.center() - galley.size() / 2.0,
                galley,
                notebook::ink::page(dark),
            );
            if response.clicked() {
                match name {
                    "in" => self.zoom.step(1, egui::Vec2::ZERO),
                    "out" => self.zoom.step(-1, egui::Vec2::ZERO),
                    _ => self.zoom.fit(),
                }
            }
        }
    }
}
