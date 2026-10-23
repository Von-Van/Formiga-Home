//! The house by the keyboard, for anyone who would rather not reach for the mouse: the arrow keys
//! step through who and what is in the house, Enter opens it as a click would, and the menu that
//! opens answers to the arrows and Enter too. Arranging, Enter picks up what the arrows have come
//! to, the arrows carry it a tile at a time, and Enter puts it down. With Shift, the arrows move
//! about a house seen close.

use super::*;

/// How far Shift and an arrow move a house seen close, in points.
const PAN_STEP: f32 = 40.0;

impl HomeApp {
    pub(super) fn house_keys(&mut self, ctx: &egui::Context) {
        let (left, right, up, down, enter, shift) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::ArrowLeft),
                input.key_pressed(egui::Key::ArrowRight),
                input.key_pressed(egui::Key::ArrowUp),
                input.key_pressed(egui::Key::ArrowDown),
                input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space),
                input.modifiers.shift,
            )
        });
        if !(left || right || up || down || enter) {
            return;
        }
        let across = i32::from(right) - i32::from(left);
        let along = i32::from(down) - i32::from(up);
        if shift {
            self.zoom
                .pan_by(-egui::vec2(across as f32, along as f32) * PAN_STEP);
            return;
        }
        if self.only_shown() {
            return;
        }
        if self.menu.is_some() {
            self.menu_keys(along + across, enter);
            return;
        }
        if self.mode == Mode::Arrange && self.arranging.carrying.is_some() {
            if enter && self.key_tile.is_some() {
                let point = self
                    .key_tile
                    .map(|(x, y)| self.scene.view.tile_centre(x, y));
                self.put_carried(point);
                self.key_tile = None;
            } else {
                self.carry_by(across, along);
            }
            return;
        }
        if across + along != 0 {
            self.step_focus(across + along);
        }
        if enter && let Some(target) = self.focus.clone() {
            match self.mode {
                Mode::Live => {
                    let at = self.on_window(&target).unwrap_or_default();
                    self.click_live(target, at);
                }
                Mode::Arrange => self.pick_up(&target),
            }
        }
    }

    /// Up and down the open menu, and Enter for the entry come to.
    fn menu_keys(&mut self, by: i32, enter: bool) {
        let Some(menu) = &mut self.menu else {
            return;
        };
        let count = menu.entries.len() as i32;
        if by != 0 && count > 0 {
            let at = menu
                .highlight
                .map_or(if by > 0 { -1 } else { 0 }, |at| at as i32);
            menu.highlight = Some((at + by).rem_euclid(count) as usize);
        }
        if enter && let Some(entry) = menu.highlight.and_then(|at| menu.entries.get(at)) {
            let entry = entry.0.clone();
            self.choose_from_menu(entry);
        }
    }

    /// Who and what the arrows step through, in the house as it stands: everyone at home first,
    /// while living in it, then each piece and each thing on show, from the far corner forward.
    /// A find someone is wearing is not where it is shown, so it is passed over.
    pub(super) fn targets(&self) -> Vec<Target> {
        let mut targets = Vec::new();
        if self.mode == Mode::Live {
            let present = self.life.present();
            targets.extend(
                self.household
                    .everyone()
                    .map(|resident| resident.id)
                    .filter(|id| present.contains(id))
                    .map(Target::Resident),
            );
        }
        let mut pieces: Vec<_> = self
            .house
            .pieces
            .iter()
            .map(|placed| {
                let (x, y) = placement::footprint(placed).centre();
                ((x + y, x), Target::Piece(placed.uid))
            })
            .collect();
        pieces.sort_by(|(a, _), (b, _)| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        targets.extend(pieces.into_iter().map(|(_, target)| target));
        let worn = self.life.worn();
        targets.extend(
            self.house
                .shown
                .iter()
                .filter(|shown| !worn.contains(&shown.item))
                .map(|shown| Target::Shown(shown.item.clone())),
        );
        targets
    }

    /// The next thing in the house, or the one before.
    fn step_focus(&mut self, by: i32) {
        let targets = self.targets();
        if targets.is_empty() {
            self.focus = None;
            return;
        }
        let at = self
            .focus
            .as_ref()
            .and_then(|focus| targets.iter().position(|target| target == focus))
            .map_or(if by > 0 { -1 } else { 0 }, |at| at as i32);
        self.focus = Some(targets[(at + by).rem_euclid(targets.len() as i32) as usize].clone());
    }

    /// Where `target` is on the scene: the middle of someone, the middle of a piece's front, or
    /// where a thing is shown.
    fn on_scene(&mut self, target: &Target) -> Option<(f32, f32)> {
        let view = self.scene.view;
        match target {
            Target::Resident(id) => {
                let now = self.now();
                let actor = self.life.actors.iter_mut().find(|actor| actor.id == *id)?;
                let (l, t, r, b) = actor.bounds(&view, now);
                Some(((l + r) as f32 / 2.0, (t + b) as f32 / 2.0))
            }
            Target::Piece(uid) => {
                let placed = self.house.piece(*uid)?;
                let piece = catalog::piece(&placed.piece)?;
                let (x, y) = placement::footprint(placed).centre();
                let (sx, sy) = view.screen(x, y);
                Some((sx, sy - piece.height as f32 / 2.0))
            }
            Target::Shown(item) => {
                let at = self
                    .house
                    .shown
                    .iter()
                    .find(|shown| shown.item == *item)?
                    .at;
                let (x, y) = self.scene.spot_anchor(&self.house, at)?;
                Some((x as f32, y as f32))
            }
            Target::Floor(x, y) => Some(view.tile_centre(*x, *y)),
        }
    }

    /// Where `target` is on the window, as it was drawn last.
    fn on_window(&mut self, target: &Target) -> Option<egui::Pos2> {
        let rect = self.room_rect?;
        let scale = rect.width() / self.scene.view.size.0 as f32;
        let (x, y) = self.on_scene(target)?;
        Some(rect.min + egui::vec2(x, y) * scale)
    }

    /// Pick up a piece or a thing the arrows have come to, to carry it by the keys.
    fn pick_up(&mut self, target: &Target) {
        let carry = match target {
            Target::Piece(uid) => self.house.piece(*uid).map(|placed| Carry::Piece {
                name: *uid,
                turn: placed.turn,
            }),
            Target::Shown(item) => Some(Carry::Thing(item.clone())),
            _ => None,
        };
        let Some(carry) = carry else {
            return;
        };
        self.key_tile = self.tile_of(target);
        self.arranging.carrying = Some(carry);
        self.arranging.dragged = false;
    }

    /// The floor tile something is over. A piece is held by its middle, as the pointer holds it,
    /// or the tile nearer its back corner where a side is even: put down there, it lands where
    /// it stood.
    fn tile_of(&mut self, target: &Target) -> Option<(u8, u8)> {
        if let Target::Piece(uid) = target {
            let placed = self.house.piece(*uid)?;
            let footprint = placement::footprint(placed);
            return Some((
                footprint.x + footprint.w.saturating_sub(1) / 2,
                footprint.y + footprint.d.saturating_sub(1) / 2,
            ));
        }
        let (sx, sy) = self.on_scene(target)?;
        let (x, y) = self.scene.view.floor_at(sx, sy);
        let (w, d) = (self.house.width, self.house.depth);
        Some((
            (x.max(0.0) as u8).min(w.saturating_sub(1)),
            (y.max(0.0) as u8).min(d.saturating_sub(1)),
        ))
    }

    /// Carry what is held a tile across the floor: across the room's width with left and right,
    /// along its depth with up and down.
    fn carry_by(&mut self, across: i32, along: i32) {
        let (w, d) = (i32::from(self.house.width), i32::from(self.house.depth));
        let (x, y) = self
            .key_tile
            .map_or((w / 2, d / 2), |(x, y)| (i32::from(x), i32::from(y)));
        self.key_tile = Some((
            (x + across).clamp(0, w - 1) as u8,
            (y + along).clamp(0, d - 1) as u8,
        ));
    }
}
