//! The pointer on the room while living in it: a resident dragged is carried and put down, a
//! right-click gives it a pat, and a click on something opens what whoever is chosen could do
//! there, or sends it to the floor clicked.

use super::*;
use crate::life::{Asked, QUEUE_LIMIT, choices};

impl HomeApp {
    pub(super) fn ask(&mut self, id: Id, act: Act) {
        let now = self.now();
        let label = act.label(&self.household, &self.house, &self.household.snapshot);
        let name = self.name(id);
        match self.life.ask(id, act, now) {
            Asked::Queued => self.say(format!("{name}: {}", label.to_lowercase())),
            Asked::Full => self.say(format!(
                "{name} has {QUEUE_LIMIT} things to do already. Take one back in the drawer, or wait."
            )),
        }
    }

    pub(super) fn live_pointer(
        &mut self,
        response: &egui::Response,
        pointer: Option<(f32, f32)>,
        released: bool,
        now: f32,
    ) {
        if let Some(id) = self.carried {
            if let Some((x, y)) = pointer {
                let (fx, fy) = self.scene.view.floor_at(x, y + 10.0);
                let (w, d) = (f32::from(self.house.width), f32::from(self.house.depth));
                self.life
                    .carry(id, (fx.clamp(0.2, w - 0.2), fy.clamp(0.2, d - 0.2)));
            }
            if released || response.drag_stopped() {
                self.life.put_down(&self.house, id, now);
                self.carried = None;
            }
            return;
        }
        if response.drag_started()
            && let Some(Target::Resident(id)) = self.hovered.clone()
        {
            self.menu = None;
            self.life.pick_up(id, now);
            self.carried = Some(id);
            self.selected = Some(id);
            return;
        }
        // Dragged across anything else, a house seen close moves about under the pointer.
        if response.drag_started() {
            self.zoom.panning = self.zoom.closer_than_fits();
            return;
        }
        if response.secondary_clicked()
            && let Some(Target::Resident(id)) = self.hovered.clone()
        {
            self.life.pet(&self.household, id, now);
            return;
        }
        if !response.clicked() {
            return;
        }
        // A click away from an open menu only closes it.
        if self.menu.take().is_some() {
            return;
        }
        let Some(target) = self.hovered.clone() else {
            self.menu = None;
            return;
        };
        let at = response.interact_pointer_pos().unwrap_or_default();
        self.click_live(target, at);
    }

    fn click_live(&mut self, target: Target, at: egui::Pos2) {
        let now = self.now();
        let house = self.house.clone();
        let snapshot = &self.household.snapshot;
        match (target, self.selected) {
            (Target::Resident(id), Some(chosen)) if id == chosen => {
                self.menu = Some(Menu {
                    at,
                    title: self.name(id),
                    entries: {
                        let mut entries = vec![(Entry::Pet, "Give a pat".to_owned())];
                        entries.extend(self.stay_over_entry(id));
                        entries.extend(self.move_in_entry(id));
                        entries
                    },
                    opened: now,
                });
            }
            (Target::Resident(id), None) => {
                self.selected = Some(id);
                self.menu = None;
            }
            (Target::Floor(x, y), Some(chosen)) => {
                self.menu = None;
                self.ask(chosen, Act::GoTo(x, y));
            }
            (target, Some(chosen)) => {
                let present = self.life.present();
                let guests = self.life.guests();
                let acts = choices(
                    &self.household,
                    &house,
                    snapshot,
                    &present,
                    &guests,
                    chosen,
                    &target,
                );
                let mut entries: Vec<(Entry, String)> = acts
                    .into_iter()
                    .map(|act| {
                        let label = act.label(&self.household, &house, snapshot);
                        (Entry::Ask(act), label)
                    })
                    .collect();
                let title = match &target {
                    Target::Resident(other) => {
                        entries.extend(self.stay_over_entry(*other));
                        entries.extend(self.move_in_entry(*other));
                        entries.push((
                            Entry::Choose(*other),
                            format!("Choose {} instead", self.name(*other)),
                        ));
                        format!("{} and {}", self.name(chosen), self.name(*other))
                    }
                    Target::Shown(item) => snapshot
                        .item(item)
                        .map_or_else(String::new, |item| item.name.clone()),
                    Target::Piece(uid) => house
                        .piece(*uid)
                        .and_then(|placed| catalog::piece(&placed.piece))
                        .map_or_else(String::new, |piece| piece.name.to_owned()),
                    Target::Floor(..) => String::new(),
                };
                if entries.is_empty() {
                    self.say(format!("Nothing there for {} to do.", self.name(chosen)));
                    self.menu = None;
                } else {
                    self.menu = Some(Menu {
                        at,
                        title,
                        entries,
                        opened: now,
                    });
                }
            }
            (_, None) => {
                self.say("Click a resident first, to choose who to ask.");
            }
        }
    }
}
