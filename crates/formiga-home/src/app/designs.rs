//! The Designs page of the arranging notes: the house, or one room of it, saved by name; the
//! gallery of what has been saved or opened; and a design tried in the house before it is used,
//! with whatever it cannot bring named first.

use super::lines;
use super::notebook::{ink, kicker};
use super::pages::and_list;
use super::*;
use crate::designs::{self, Design, Fitted, Omitted, Scope, Why};
use formiga_home_contract::HomeSnapshot;

/// A design shown in the house, not yet used.
pub(super) struct Trying {
    pub design: Design,
    /// Where it is in the gallery, if it is kept there.
    pub kept: Option<usize>,
    /// The room a one-room design goes into.
    pub room: u8,
    pub fitted: Fitted,
}

impl HomeApp {
    /// The house as a design would make it, if one is being tried.
    pub(super) fn tried_house(&self) -> Option<House> {
        (self.mode == Mode::Arrange && self.drawer == Drawer::Designs)
            .then_some(self.trying.as_ref())
            .flatten()
            .map(|trying| House::of(&trying.fitted.home.rooms))
    }

    /// Show `design` in the house, fitted into `room` if it is one room.
    pub(super) fn try_design(&mut self, design: Design, kept: Option<usize>, room: u8) {
        let fitted = designs::fit(
            &design,
            &self.state,
            self.keeper,
            &self.household.snapshot,
            room,
        );
        self.trying = fitted.map(|fitted| Trying {
            design,
            kept,
            room,
            fitted,
        });
        self.arranging.carrying = None;
    }

    /// Save the house, or one room of it, to the gallery.
    pub(super) fn save_design(&mut self, scope: Scope) {
        let name = match self.design_name.trim() {
            "" => self.suggested_name(scope),
            name => name.to_owned(),
        };
        let design = Design::of(
            home_of(&self.state, self.keeper),
            scope,
            &name,
            OffsetDateTime::now_utc(),
        );
        let saved = design.name.clone();
        match self.designs.keep(design, false) {
            Ok(_) => {
                self.design_name.clear();
                self.say(format!("Saved \u{201c}{saved}\u{201d} to the gallery."));
            }
            Err(error) => self.say(format!("It could not be saved: {error}.")),
        }
    }

    /// A name for a design nobody named: what it is of, and the day. Never anyone's name, since
    /// a design may be passed on.
    fn suggested_name(&self, scope: Scope) -> String {
        let day = crate::journal::day_of(OffsetDateTime::now_utc());
        let what = match scope {
            Scope::House => "The house",
            Scope::Room(room) => self.house.room(room).map_or("A room", |at| {
                catalog::room_name(at.kind.as_ref(), room == 0)
            }),
        };
        format!("{what}, {} {}", day.day(), day.month())
    }

    /// For review only: arranging, on the Designs page, trying the design in `path`.
    pub fn try_file(&mut self, path: &std::path::Path) {
        self.drawer = Drawer::Designs;
        self.set_mode(Mode::Arrange);
        self.open_design(path);
    }

    /// Open a design file, to try in the house before it is kept or used.
    pub(super) fn open_design(&mut self, path: &std::path::Path) {
        match designs::read(path) {
            Ok(design) => {
                let room = self.room_page;
                self.try_design(design, None, room);
                if self.trying.is_some() {
                    self.say("Here is how it would look. Nothing has changed yet.");
                }
            }
            Err(error) => self.say(format!("That file could not be opened: {error}.")),
        }
    }

    /// Write a design kept in the gallery as a file at `to`.
    pub(super) fn share_design(&mut self, index: usize, to: &std::path::Path) {
        let Some(kept) = self.designs.kept.get(index) else {
            return;
        };
        match designs::export(&kept.design, to) {
            Ok(()) => self.say("Saved the design as a file, to pass on."),
            Err(error) => self.say(format!("The file could not be saved: {error}.")),
        }
    }

    /// Use the design being tried: the house is arranged as it is shown.
    pub(super) fn use_design(&mut self) {
        let Some(trying) = self.trying.take() else {
            return;
        };
        self.arranging
            .use_design(&mut self.state, self.keeper, &trying.fitted);
        self.changed(&format!(
            "Arranged as \u{201c}{}\u{201d}. Undo takes it back.",
            trying.design.name
        ));
    }

    pub(super) fn designs_page(&mut self, ui: &mut egui::Ui, unit: f32) {
        if self.trying.is_some() {
            self.trying_page(ui, unit);
            return;
        }
        let dark = ui.visuals().dark_mode;
        kicker(ui, "Save the arrangement");
        let rooms = self.house.rooms.len();
        if rooms > 1 {
            ui.horizontal_wrapped(|ui| {
                for (index, room) in self.house.rooms.iter().enumerate() {
                    let name = catalog::room_name(room.kind.as_ref(), index == 0);
                    ui.selectable_value(&mut self.room_page, index as u8, name);
                }
            });
        }
        let hint = self.suggested_name(if rooms > 1 {
            Scope::House
        } else {
            Scope::Room(0)
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.design_name)
                .hint_text(hint)
                .char_limit(designs::MAX_NAME_CHARS)
                .desired_width(f32::INFINITY),
        );
        let mut save = None;
        ui.horizontal_wrapped(|ui| {
            if rooms > 1 {
                if ui
                    .button("Save the house")
                    .on_hover_text("Every room, to use again room for room")
                    .clicked()
                {
                    save = Some(Scope::House);
                }
                if ui
                    .button("Save this room")
                    .on_hover_text("The room chosen above, to use in any room")
                    .clicked()
                {
                    save = Some(Scope::Room(self.room_page));
                }
            } else if ui
                .button("Save")
                .on_hover_text("The room as it is, to use again or pass on")
                .clicked()
            {
                save = Some(Scope::Room(0));
            }
        });
        if let Some(scope) = save {
            self.save_design(scope);
        }
        ui.add_space(6.0);
        kicker(ui, "The gallery");
        if self.designs.kept.is_empty() {
            lines::write(
                ui,
                egui::RichText::new(
                    "Nothing saved yet. Arrange the house as you like it and save it here, to \
                     use again or pass on as a file.",
                )
                .italics()
                .color(ink::muted(dark)),
            );
        }
        let mut chosen = None;
        let mut share = None;
        let mut remove = None;
        for (index, kept) in self.designs.kept.iter().enumerate().rev() {
            let design = &kept.design;
            lines::write(ui, egui::RichText::new(&design.name).strong());
            let (pieces, things) = design.rooms.iter().fold((0, 0), |(p, t), room| {
                (p + room.pieces.len(), t + room.displays.len())
            });
            let what = if design.whole_house {
                format!(
                    "{} room{}",
                    design.rooms.len(),
                    if design.rooms.len() == 1 { "" } else { "s" }
                )
            } else {
                "One room".to_owned()
            };
            let when = if kept.opened {
                "opened from a file".to_owned()
            } else {
                let day = crate::journal::day_of(design.saved_at_utc);
                format!("saved {} {}", day.day(), day.month())
            };
            lines::write(
                ui,
                egui::RichText::new(format!(
                    "{what}, {pieces} piece{}, {things} thing{}; {when}",
                    if pieces == 1 { "" } else { "s" },
                    if things == 1 { "" } else { "s" },
                ))
                .small()
                .color(ink::muted(dark)),
            );
            if self.removing == Some(index) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Remove it from the gallery?").small());
                    if ui.small_button("Remove").clicked() {
                        remove = Some(index);
                    }
                    if ui.small_button("Keep it").clicked() {
                        self.removing = None;
                    }
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .small_button("Try it")
                        .on_hover_text("See it in the house before using it")
                        .clicked()
                    {
                        chosen = Some(index);
                    }
                    if ui
                        .small_button("Share\u{2026}")
                        .on_hover_text("Save it as a file to pass on")
                        .clicked()
                    {
                        share = Some(index);
                    }
                    if ui.small_button("Remove").clicked() {
                        self.removing = Some(index);
                    }
                });
            }
            ui.add_space(2.0 * unit);
        }
        if let Some(index) = chosen {
            let design = self.designs.kept[index].design.clone();
            let room = self.room_page;
            self.removing = None;
            self.try_design(design, Some(index), room);
        }
        if let Some(index) = share {
            let name = designs::file_name(&self.designs.kept[index].design);
            let to = rfd::FileDialog::new()
                .set_title("Save the design as a file")
                .set_file_name(&name)
                .add_filter("Formiga Home design", &[designs::EXTENSION])
                .save_file();
            if let Some(to) = to {
                self.share_design(index, &to);
            }
        }
        if let Some(index) = remove {
            self.removing = None;
            match self.designs.remove(index) {
                Ok(()) => self.say("Removed from the gallery. The house is as it was."),
                Err(error) => self.say(format!("It could not be removed: {error}.")),
            }
        }
        ui.add_space(4.0);
        if ui
            .button("Open a design file\u{2026}")
            .on_hover_text("A design someone passed on: you see it in the house first")
            .clicked()
        {
            let from = rfd::FileDialog::new()
                .set_title("Open a design")
                .add_filter("Formiga Home design", &[designs::EXTENSION])
                .pick_file();
            if let Some(from) = from {
                self.open_design(&from);
            }
        }
    }

    /// A design shown in the house: what it brings, what it leaves out and why, and what goes
    /// back in the drawer, before anything changes.
    fn trying_page(&mut self, ui: &mut egui::Ui, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let Some(trying) = &self.trying else {
            return;
        };
        kicker(
            ui,
            &format!("Trying \u{201c}{}\u{201d}", trying.design.name),
        );
        let mut into = None;
        if !trying.design.whole_house && self.house.rooms.len() > 1 {
            let mut room = trying.room;
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("In the").small());
                for (index, at) in self.house.rooms.iter().enumerate() {
                    let name = catalog::room_name(at.kind.as_ref(), index == 0);
                    ui.selectable_value(&mut room, index as u8, name);
                }
            });
            if room != trying.room {
                into = Some(room);
            }
        }
        let snapshot = &self.household.snapshot;
        let fitted = &trying.fitted;
        let (pieces, things) = fitted.rooms.iter().fold((0, 0), |(p, t), room| {
            let layout = &fitted.home.rooms[usize::from(*room)];
            (p + layout.pieces.len(), t + layout.displays.len())
        });
        lines::write(
            ui,
            format!(
                "It brings {pieces} piece{} and {things} thing{}.",
                if pieces == 1 { "" } else { "s" },
                if things == 1 { "" } else { "s" }
            ),
        );
        let report = report(fitted, snapshot);
        if report.is_empty() {
            lines::write(
                ui,
                egui::RichText::new("Everything in it comes.").color(ink::forest(dark)),
            );
        }
        for (heading, line) in report {
            lines::write(
                ui,
                egui::RichText::new(heading)
                    .small()
                    .color(ink::forest(dark)),
            );
            lines::write(ui, line);
        }
        lines::write(
            ui,
            egui::RichText::new(
                "Walls and doorways stay as they are, and so does everything the household \
                 remembers.",
            )
            .small()
            .italics()
            .color(ink::muted(dark)),
        );
        ui.add_space(2.0 * unit);
        let mut use_it = false;
        let mut keep = false;
        let mut cancel = false;
        let kept = trying.kept.is_some();
        ui.horizontal_wrapped(|ui| {
            use_it = ui.button("Use this design").clicked();
            if !kept {
                keep = ui
                    .button("Keep in the gallery")
                    .on_hover_text("Keep it to use later, without changing the house")
                    .clicked();
            }
            cancel = ui.button("Not now").clicked();
        });
        if let Some(room) = into {
            let (design, kept) = (trying.design.clone(), trying.kept);
            self.try_design(design, kept, room);
        } else if use_it {
            self.use_design();
        } else if keep {
            let design = trying.design.clone();
            match self.designs.keep(design, true) {
                Ok(index) => {
                    if let Some(trying) = &mut self.trying {
                        trying.kept = Some(index);
                    }
                    self.say("Kept in the gallery. The house is as it was.");
                }
                Err(error) => self.say(format!("It could not be kept: {error}.")),
            }
        } else if cancel {
            self.trying = None;
        }
    }
}

/// What the owner should know before using a fitted design, a heading and a line for each:
/// what does not come and why, and what goes back in the drawer or moves from another house.
fn report(fitted: &Fitted, snapshot: &HomeSnapshot) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    let named = |why: Why| -> Vec<String> {
        let mut names: Vec<(String, usize)> = Vec::new();
        for omitted in &fitted.omitted {
            let name = match omitted {
                Omitted::Piece(Some(piece), w) if *w == why => piece.name.to_owned(),
                Omitted::Floor(Some(finish), w) | Omitted::Wall(Some(finish), w) if *w == why => {
                    finish.name.to_owned()
                }
                Omitted::Thing(id, w) if *w == why => match snapshot.item(id) {
                    Some(item) => item.name.clone(),
                    None => continue,
                },
                _ => continue,
            };
            match names.iter_mut().find(|(had, _)| *had == name) {
                Some((_, count)) => *count += 1,
                None => names.push((name, 1)),
            }
        }
        names
            .into_iter()
            .map(|(name, count)| match count {
                1 => name,
                n => format!("{name} \u{d7}{n}"),
            })
            .collect()
    };
    let not_yet = named(Why::NotYet);
    if !not_yet.is_empty() {
        lines.push((
            "Not arrived here yet".to_owned(),
            format!("{}.", and_list(&not_yet)),
        ));
    }
    let no_space = named(Why::NoSpace);
    if !no_space.is_empty() {
        lines.push((
            "No room for".to_owned(),
            format!("{}.", and_list(&no_space)),
        ));
    }
    let full = named(Why::Full);
    if !full.is_empty() {
        lines.push((
            "More than a house holds".to_owned(),
            format!("{}.", and_list(&full)),
        ));
    }
    let count = |test: &dyn Fn(&Omitted) -> bool| fitted.omitted.iter().filter(|o| test(o)).count();
    let not_had = count(&|omitted| matches!(omitted, Omitted::Thing(_, Why::NotHad)));
    if not_had > 0 {
        lines.push((
            "Not in this colony".to_owned(),
            format!(
                "{not_had} found thing{} the colony does not have.",
                if not_had == 1 { "" } else { "s" }
            ),
        ));
    }
    let unknown = count(&|omitted| {
        matches!(
            omitted,
            Omitted::Piece(_, Why::Unknown)
                | Omitted::Floor(_, Why::Unknown)
                | Omitted::Wall(_, Why::Unknown)
        )
    });
    if unknown > 0 {
        lines.push((
            "From a newer Home".to_owned(),
            format!(
                "{unknown} thing{} this Home does not know yet.",
                if unknown == 1 { "" } else { "s" }
            ),
        ));
    }
    let rooms = count(&|omitted| matches!(omitted, Omitted::Room));
    if rooms > 0 {
        lines.push((
            "Rooms".to_owned(),
            format!(
                "This house has {} fewer room{} than the design: what was in {} stays out.",
                rooms,
                if rooms == 1 { "" } else { "s" },
                if rooms == 1 { "it" } else { "them" }
            ),
        ));
    }
    let name_of = |id: &DisplayId| {
        snapshot
            .item(id)
            .map_or_else(|| "something".to_owned(), |item| item.name.clone())
    };
    if !fitted.put_away.is_empty() {
        // A few by name, and the rest counted, so the note stays short.
        let mut names: Vec<String> = fitted.put_away.iter().take(3).map(name_of).collect();
        let more = fitted.put_away.len().saturating_sub(names.len());
        if more > 0 {
            names.push(format!("{more} more"));
        }
        lines.push((
            "Back in the drawer".to_owned(),
            format!("{}.", and_list(&names)),
        ));
    }
    for (item, keeper) in &fitted.moved {
        let house = snapshot
            .village
            .iter()
            .find(|neighbour| neighbour.keeper == *keeper)
            .map_or_else(
                || "another house".to_owned(),
                |neighbour| format!("{}'s house", neighbour.name),
            );
        lines.push((
            "Moves here".to_owned(),
            format!("{} from {house}.", name_of(item)),
        ));
    }
    lines
}
