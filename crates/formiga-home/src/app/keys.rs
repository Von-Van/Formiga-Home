//! The keys: the modes, the notes' pages, whoever is chosen, the sticky note, closer and further,
//! a photo; and while arranging, turning what is carried, putting it away, undoing and doing again.

use super::*;

impl HomeApp {
    pub(super) fn keys(&mut self, ctx: &egui::Context) {
        let (escape, turn, away, undo, redo, photo) = ctx.input(|input| {
            let command = input.modifiers.command;
            (
                input.key_pressed(egui::Key::Escape),
                input.key_pressed(egui::Key::R),
                input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace),
                command && !input.modifiers.shift && input.key_pressed(egui::Key::Z),
                command && input.modifiers.shift && input.key_pressed(egui::Key::Z),
                input.key_pressed(egui::Key::P),
            )
        });
        // Closer and further, without ⌘ or Ctrl, which zoom the whole window instead.
        let (closer, further, fit) = ctx.input(|input| {
            let plain = !input.modifiers.command;
            (
                plain
                    && (input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals)),
                plain && input.key_pressed(egui::Key::Minus),
                plain && input.key_pressed(egui::Key::Num0),
            )
        });
        // The notebook by the keys: the modes, its pages, whoever is chosen, and its note.
        let (live, arrange, back, on, next, previous, note) = ctx.input(|input| {
            let plain = !input.modifiers.command && !input.modifiers.alt;
            let shift = input.modifiers.shift;
            (
                plain && input.key_pressed(egui::Key::L),
                plain && input.key_pressed(egui::Key::A),
                plain && input.key_pressed(egui::Key::OpenBracket),
                plain && input.key_pressed(egui::Key::CloseBracket),
                plain && !shift && input.key_pressed(egui::Key::N),
                plain && shift && input.key_pressed(egui::Key::N),
                plain
                    && (input.key_pressed(egui::Key::H)
                        || input.key_pressed(egui::Key::Questionmark)),
            )
        });
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if live {
            self.set_mode(Mode::Live);
        }
        if arrange {
            self.set_mode(Mode::Arrange);
        }
        if back || on {
            self.turn_page(if on { 1 } else { -1 });
        }
        if (next || previous) && self.mode == Mode::Live {
            self.choose_along(if next { 1 } else { -1 });
        }
        if note {
            self.help = !self.help;
        }
        if closer {
            self.zoom.step(1, egui::Vec2::ZERO);
        }
        if further {
            self.zoom.step(-1, egui::Vec2::ZERO);
        }
        if fit {
            self.zoom.fit();
        }
        if photo {
            self.save_photo();
        }
        if escape {
            if self.placing.take().is_none()
                && self.menu.take().is_none()
                && self.arranging.carrying.take().is_none()
            {
                self.selected = None;
            }
            self.arranging.dragged = false;
        }
        if self.mode != Mode::Arrange {
            return;
        }
        if turn {
            self.arranging.turn();
        }
        if away
            && self
                .arranging
                .put_away(&mut self.state, self.keeper, &self.house)
        {
            self.changed("Put away.");
        }
        if undo && self.arranging.undo(&mut self.state) {
            self.changed("Undone.");
        }
        if redo && self.arranging.redo(&mut self.state) {
            self.changed("Done again.");
        }
    }

    /// The next page of the notes, or the one before, in the mode the house is in.
    fn turn_page(&mut self, by: i32) {
        let pages = notebook::pages(self.mode);
        let at = pages
            .iter()
            .position(|(page, _)| *page == self.page())
            .unwrap_or(0) as i32;
        let (page, _) = pages[(at + by).rem_euclid(pages.len() as i32) as usize];
        match self.mode {
            Mode::Live => self.live_page = page,
            Mode::Arrange => {
                self.drawer = page;
                self.placing = None;
            }
        }
    }

    /// Choose the next one in the house, or the one before: residents, then visitors.
    fn choose_along(&mut self, by: i32) {
        let present = self.life.present();
        let order: Vec<Id> = self
            .household
            .everyone()
            .map(|resident| resident.id)
            .filter(|id| present.contains(id))
            .collect();
        if order.is_empty() {
            return;
        }
        let at = self
            .selected
            .and_then(|id| order.iter().position(|other| *other == id))
            .map_or(if by > 0 { -1 } else { 0 }, |at| at as i32);
        self.selected = Some(order[(at + by).rem_euclid(order.len() as i32) as usize]);
        self.menu = None;
    }
}
