//! The card Home opens on when someone opens it themselves, from Applications or the Start menu,
//! rather than Formiga Desktop opening a house. Desktop's sample household is at home as it
//! always has been, and the card over its page says where a house of the person's own comes
//! from: their village in Formiga. Looking around with the sample puts the card away; closing
//! closes the notebook.

use super::*;

const TITLE: &str = "Your colony comes from Formiga";

/// In the words Desktop's own house menu uses.
const OPENS: &str = "Formiga Home opens a house from your village: in Formiga, click a house and \
                     choose \u{201c}Look inside\u{201d}.";

const GET_FORMIGA: &str = "Don't have Formiga yet? It's at formigaworld.com.";

pub(super) const LOOK_AROUND: &str = "Look around with a sample colony";
pub(super) const CLOSE: &str = "Close";

/// The widest the card is, in points.
const WIDTH: f32 = 300.0;

impl HomeApp {
    /// Opens on the welcome card: Home was opened on its own, so the house is the sample's, and
    /// the card says how one of the person's own is opened.
    pub fn welcome(&mut self) {
        self.welcome = true;
    }

    /// The welcome card, over the middle of the house's page, as the menu's card is drawn: the
    /// card's face, the notebook's stepped edge and a shadow under it.
    pub(super) fn welcome_card(&mut self, ctx: &egui::Context, page: egui::Rect) {
        if !self.welcome {
            return;
        }
        let unit = notebook::unit(ctx.pixels_per_point());
        let width = WIDTH.min(page.width() - 32.0 * unit).max(120.0);
        let mut look_around = false;
        let mut close = false;
        egui::Area::new(egui::Id::new("welcome"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(page.center())
            .constrain_to(page)
            .show(ctx, |ui| {
                let dark = ui.visuals().dark_mode;
                let shadow = ui.painter().add(egui::Shape::Noop);
                let shown = egui::Frame::new()
                    .fill(notebook::ink::card(dark))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_width(width);
                        notebook::kicker(ui, TITLE);
                        ui.add_space(4.0);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(OPENS).color(notebook::ink::page(dark)),
                            )
                            .wrap(),
                        );
                        ui.add_space(2.0);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(GET_FORMIGA)
                                    .small()
                                    .italics()
                                    .color(notebook::ink::muted(dark)),
                            )
                            .wrap(),
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            look_around = ui.button(LOOK_AROUND).clicked();
                            close = ui.button(CLOSE).clicked();
                        });
                    });
                let rect = shown.response.rect;
                ui.painter().set(
                    shadow,
                    egui::Shape::rect_filled(
                        rect.translate(egui::vec2(2.0 * unit, 2.0 * unit)),
                        0.0,
                        egui::Color32::from_black_alpha(60),
                    ),
                );
                pages::stepped(
                    ui.painter(),
                    rect.expand(unit),
                    unit,
                    notebook::ink::line(dark),
                );
            });
        if look_around {
            self.welcome = false;
        }
        if close {
            // Closing is leaving the house, as the cover's own stud does.
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
