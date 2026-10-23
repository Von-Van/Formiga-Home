//! The project board, on its own page of the notes while living in the house, and everything the
//! window does for the household's projects: counting what happens at home towards them, saying
//! quietly when one is ready or a souvenir from Formiga Hill counts, and finishing a stage when
//! the owner asks.
//!
//! The board is short until something on it is chosen: a line a project, saying where it
//! stands. Choosing one opens what it asks for, how far along each is, and what finishing it
//! gives. Below them, what the household has made, for good.

use super::lines;
use super::notebook::{LABEL, ink, kicker, label_job};
use super::pages::{and_list, crisp, cropped, tile};
use super::*;
use crate::projects::{self, Facts, Gift, Given, Kind, PROJECTS, Project, Standing};
use formiga_home_contract::CatalogId;

impl HomeApp {
    /// Whether the household's projects are kept: only by a Desktop that reads household
    /// version 10. An older one would set them aside on every visit.
    pub(super) fn keeps_projects(&self) -> bool {
        self.host.keeps_projects()
    }

    fn facts(&self) -> Facts<'_> {
        Facts {
            snapshot: &self.household.snapshot,
            home: home_of(&self.state, self.keeper),
        }
    }

    /// The projects with a stage ready to finish.
    fn ready(&self) -> Vec<&'static str> {
        let facts = self.facts();
        PROJECTS
            .iter()
            .filter(|project| matches!(project.standing(&facts), Standing::Ready(_)))
            .map(|project| project.id)
            .collect()
    }

    /// Whether the board has something new on it, or something ready: what its tab marks.
    pub(super) fn board_wants_a_look(&self) -> bool {
        if !self.keeps_projects() {
            return false;
        }
        let facts = self.facts();
        PROJECTS.iter().any(|project| {
            project.is_new(&facts) || matches!(project.standing(&facts), Standing::Ready(_))
        })
    }

    /// Something that happened at home, counted for the household's projects. A stage that is
    /// ready because of it is said once, quietly.
    pub(super) fn count(&mut self, count: projects::Count) {
        if !self.keeps_projects() {
            return;
        }
        let before = self.ready();
        if let Some(home) = self.state.household_mut(self.keeper) {
            projects::count(home, count);
        }
        let now_ready = self
            .ready()
            .into_iter()
            .find(|id| !before.contains(id))
            .and_then(projects::project);
        if let Some(project) = now_ready {
            let stage = project.stages[project.done(home_of(&self.state, self.keeper))].name;
            self.say(format!(
                "{stage}, for {}, is ready on the project board.",
                project.the()
            ));
        }
    }

    /// On opening the house: any souvenir from Formiga Hill the household has not heard of, that
    /// counts towards one of its projects, is said once.
    pub(super) fn hear_of_souvenirs(&mut self) {
        if !self.keeps_projects() {
            return;
        }
        let facts = self.facts();
        let news = projects::fresh_souvenirs(facts.snapshot, facts.home)
            .into_iter()
            .find_map(|souvenir| {
                let counts = projects::counted_by(souvenir, &facts);
                (!counts.is_empty()).then(|| (souvenir.to_owned(), counts))
            });
        let snapshot = self.household.snapshot.clone();
        if let Some(home) = self.state.household_mut(self.keeper) {
            projects::heard_of_souvenirs(&snapshot, home);
        }
        if let Some((souvenir, counts)) = news {
            let names: Vec<String> = counts.iter().map(|project| project.the()).collect();
            self.say(format!(
                "The {} from Formiga Hill counts towards {}.",
                projects::souvenir_name(&souvenir).to_lowercase(),
                and_list(&names)
            ));
        }
    }

    /// Finish the stage of `project` the household is ready for, and give what it gives.
    pub(super) fn finish_project(&mut self, project: &'static Project) {
        let snapshot = self.household.snapshot.clone();
        let Some(home) = self.state.household_mut(self.keeper) else {
            return;
        };
        let Some(stage) = projects::finish(&snapshot, home, project, now_utc()) else {
            return;
        };
        let given = projects::give(home, stage);
        // What was arranged before is not what stands now: taking it back would take back
        // what the household made.
        self.arranging.forget();
        let rooms: Vec<String> = self
            .house
            .rooms
            .iter()
            .enumerate()
            .map(|(index, room)| catalog::room_name(room.kind.as_ref(), index == 0).to_lowercase())
            .collect();
        let mut said = vec![format!("{}: done.", stage.name)];
        // Improving what the house has, with none of it in the house just now: what it would
        // have been made into waits on the furniture page.
        let nothing_improved = !given.is_empty()
            && given
                .iter()
                .all(|given| matches!(given, Given::Upgraded(_, 0)));
        if nothing_improved {
            said.push(
                "There was nothing in the house to make over, so what it makes is on the \
                 furniture page."
                    .to_owned(),
            );
        }
        for given in given {
            said.push(match given {
                Given::Placed(piece, room) if rooms.len() > 1 => format!(
                    "The {} stands in the {}.",
                    piece.name.to_lowercase(),
                    rooms.get(room).map_or("house", String::as_str)
                ),
                Given::Placed(piece, _) => {
                    format!("The {} stands in the house.", piece.name.to_lowercase())
                }
                Given::Waiting(piece) => format!(
                    "The {} is on the furniture page, for when there is room.",
                    piece.name.to_lowercase()
                ),
                Given::Upgraded(_, 0) => continue,
                Given::Upgraded(piece, _) => {
                    format!("Made into the {}.", piece.name.to_lowercase())
                }
                Given::Finish(id) => format!(
                    "{} is on the rooms page.",
                    catalog::finish(id).map_or("A new finish", |finish| finish.name)
                ),
            });
        }
        self.changed(&said.join(" "));
    }

    /// The project board.
    pub(super) fn projects_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, unit: f32) {
        let dark = ui.visuals().dark_mode;
        kicker(ui, "Projects");
        if !self.keeps_projects() {
            lines::write(
                ui,
                egui::RichText::new(
                    "Projects are kept by Formiga Desktop 0.67.7 and newer. Once Desktop is up \
                     to date, the household can take them on here.",
                )
                .italics()
                .color(ink::muted(dark)),
            );
            return;
        }
        let shown: Vec<&'static Project> = {
            let facts = self.facts();
            PROJECTS
                .iter()
                .filter(|project| project.standing(&facts) != Standing::Hidden)
                .collect()
        };
        // Whatever the board shows is no longer new, once it has been seen.
        let new: Vec<&'static str> = {
            let facts = self.facts();
            shown
                .iter()
                .filter(|project| project.is_new(&facts))
                .map(|project| project.id)
                .collect()
        };
        if let Some(home) = self.state.household_mut(self.keeper) {
            for project in &shown {
                projects::seen(home, project);
            }
        }
        let mut finish = None;
        for project in &shown {
            let open = self.project_chosen == Some(project.id);
            let row = self.project_row(ui, ctx, project, open, new.contains(&project.id), unit);
            if row.clicked() {
                self.project_chosen = if open { None } else { Some(project.id) };
                if !open {
                    // What it opens onto is below it: bring it into view.
                    row.scroll_to_me(Some(egui::Align::TOP));
                }
            }
            if open && self.project_details(ui, project, unit) {
                finish = Some(*project);
            }
        }
        if shown.len() < PROJECTS.len() {
            lines::write(
                ui,
                egui::RichText::new("More appear as the household makes the house its own.")
                    .small()
                    .italics()
                    .color(ink::muted(dark)),
            );
        }
        self.made_at_home(ui);
        if let Some(project) = finish {
            self.finish_project(project);
        }
    }

    /// One project on the board, two lines of the page: a picture of what it makes next standing
    /// on the lower one, its name on the upper with a dot for each stage, and where it stands on
    /// the lower. The whole row chooses it.
    fn project_row(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        project: &'static Project,
        open: bool,
        new: bool,
        unit: f32,
    ) -> egui::Response {
        let dark = ui.visuals().dark_mode;
        let (standing, done) = {
            let facts = self.facts();
            (project.standing(&facts), project.done(facts.home))
        };
        let rect = lines::band(ui, 2.0, 24.0 * unit);
        let response = ui.interact(
            rect,
            egui::Id::new(("project", project.id)),
            egui::Sense::click(),
        );
        if open {
            ui.painter()
                .rect_filled(rect, 0.0, ink::mint(dark).gamma_multiply(0.55));
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, 0.0, ink::mint(dark).gamma_multiply(0.3));
        }
        let first_rule = rect.top() + rect.height() / 2.0;
        // The picture, on a slip of card standing on the lower line.
        let size = egui::vec2(rect.height(), rect.height());
        let card = egui::Rect::from_min_size(rect.min, size);
        let mut card_ui = ui.new_child(egui::UiBuilder::new().max_rect(card));
        let (_, at) = tile(
            &mut card_ui,
            egui::Id::new(("project tile", project.id)),
            size,
            open,
            unit,
        );
        if let Some(piece) = picture_of(project, done) {
            let (texture, pixels) = self.thumbnail(ctx, format!("piece:{}", piece.id), || {
                cropped(crate::art::furniture::draw(piece, false).canvas)
            });
            let shown = crisp(ui, pixels, size - egui::vec2(5.0, 5.0) * unit, 2.0);
            ui.painter().image(
                texture,
                egui::Rect::from_center_size(at.center(), shown),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                if done > 0 {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_white_alpha(110)
                },
            );
        }
        let left = rect.left() + size.x + 4.0 * unit;
        let name = ui.painter().layout_no_wrap(
            project.name.to_owned(),
            egui::FontId::proportional(14.0),
            ink::page(dark),
        );
        let name_width = name.size().x;
        let y = lines::on_rule(&name, first_rule);
        ui.painter()
            .galley(egui::pos2(left, y), name, ink::page(dark));
        let mut after = left + name_width + 3.0 * unit;
        after = pips(
            ui.painter(),
            egui::pos2(after, first_rule - 4.0 * unit),
            project.stages.len(),
            done,
            standing,
            unit,
            dark,
        ) + 2.0 * unit;
        if new {
            let galley = ui
                .painter()
                .layout_job(label_job("New", LABEL - 1.5, ink::stamp(dark)));
            let y = lines::on_rule(&galley, first_rule);
            ui.painter()
                .galley(egui::pos2(after, y), galley, ink::stamp(dark));
        }
        let (line, colour) = match standing {
            Standing::Ready(stage) => (
                format!("Ready: {}", project.stages[stage].name),
                ink::forest(dark),
            ),
            Standing::Underway(stage) => (
                format!(
                    "{} of {}: {}",
                    stage + 1,
                    project.stages.len(),
                    project.stages[stage].name
                ),
                ink::muted(dark),
            ),
            Standing::Finished => ("Finished".to_owned(), ink::muted(dark)),
            Standing::Hidden => (String::new(), ink::muted(dark)),
        };
        let status = ui.painter().layout(
            line,
            egui::FontId::proportional(12.0),
            colour,
            rect.right() - left,
        );
        let y = lines::on_rule(&status, rect.bottom());
        ui.painter().galley(egui::pos2(left, y), status, colour);
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, open, project.name)
        });
        response
    }

    /// A chosen project, opened under its row: what it is, what its next stage asks and how far
    /// along each is, what finishing it gives, and the stages already finished, each on a line
    /// of the page. Whether the owner asked to finish the stage.
    fn project_details(&mut self, ui: &mut egui::Ui, project: &'static Project, unit: f32) -> bool {
        let dark = ui.visuals().dark_mode;
        let facts = self.facts();
        let standing = project.standing(&facts);
        let done = project.done(facts.home);
        let finished: Vec<OffsetDateTime> = project
            .record(facts.home)
            .map(|record| record.stages.iter().map(|stage| stage.at_utc).collect())
            .unwrap_or_default();
        let lines: Vec<NeedLine> = project
            .stages
            .get(done)
            .map(|stage| {
                stage
                    .needs
                    .iter()
                    .map(|need| NeedLine {
                        text: need.line(&facts),
                        met: need.met(&facts),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut asked = false;
        // Set in under its row, with no rule down the side: the page's own margin is enough.
        ui.visuals_mut().indent_has_left_vline = false;
        ui.indent(("project details", project.id), |ui| {
            kicker(ui, project.kind.label());
            lines::write(
                ui,
                egui::RichText::new(project.about)
                    .italics()
                    .color(ink::muted(dark)),
            );
            if let Some(stage) = project.stages.get(done) {
                lines::write(ui, egui::RichText::new(stage.name).strong());
                lines::write(ui, stage.about);
                for NeedLine { text, met } in &lines {
                    let text = text.clone();
                    lines::ruled_row(ui, |ui| {
                        mark(ui, *met, unit);
                        lines::write(
                            ui,
                            egui::RichText::new(text).small().color(if *met {
                                ink::forest(dark)
                            } else {
                                ink::muted(dark)
                            }),
                        );
                    });
                }
                if matches!(standing, Standing::Ready(_)) {
                    // Something new is built; something there already is finished.
                    let verb = match project.kind {
                        Kind::Construction | Kind::Legacy if done == 0 => "Build it",
                        _ => "Finish it",
                    };
                    let band = lines::band(ui, 2.0, 24.0 * unit);
                    let pitch = band.height() / 2.0;
                    let galley = ui.painter().layout_no_wrap(
                        verb.to_owned(),
                        egui::FontId::proportional(13.5),
                        ink::page(dark),
                    );
                    let button = egui::Rect::from_min_size(
                        egui::pos2(band.left(), band.bottom() - pitch * 1.5),
                        egui::vec2(galley.size().x + 12.0 * unit, pitch * 1.3),
                    );
                    asked = ui
                        .put(button, egui::Button::new(verb))
                        .on_hover_text("Whenever you like: nothing waits on it")
                        .clicked();
                }
            }
            if !finished.is_empty() {
                let today = crate::journal::day_of(OffsetDateTime::now_utc());
                for (stage, at) in project.stages.iter().zip(&finished) {
                    let when = crate::journal::heading(crate::journal::day_of(*at), today);
                    lines::ruled_row(ui, |ui| {
                        mark(ui, true, unit);
                        lines::write(
                            ui,
                            egui::RichText::new(format!("{}, {}", stage.name, when.to_lowercase()))
                                .small()
                                .color(ink::muted(dark)),
                        );
                    });
                }
            }
        });
        asked
    }

    /// Every stage the household has finished, newest first under the day it was finished: what
    /// it has made of its home, kept for good. The latest few, until the rest are asked for.
    fn made_at_home(&mut self, ui: &mut egui::Ui) {
        const LATEST: usize = 3;
        let dark = ui.visuals().dark_mode;
        let home = home_of(&self.state, self.keeper);
        let mut made: Vec<(OffsetDateTime, &'static str, &'static Project)> = PROJECTS
            .iter()
            .filter_map(|project| project.record(home).map(|record| (project, record)))
            .flat_map(|(project, record)| {
                record
                    .stages
                    .iter()
                    .zip(project.stages)
                    .map(move |(done, stage)| (done.at_utc, stage.name, project))
            })
            .collect();
        if made.is_empty() {
            return;
        }
        made.sort_by_key(|(at, ..)| std::cmp::Reverse(*at));
        let all = made.len();
        if !self.made_shown_whole {
            made.truncate(LATEST);
        }
        kicker(ui, "Made at home");
        let today = crate::journal::day_of(OffsetDateTime::now_utc());
        let mut last = None;
        for (at, stage, project) in made {
            let day = crate::journal::day_of(at);
            if last != Some(day) {
                lines::write(
                    ui,
                    egui::RichText::new(crate::journal::heading(day, today))
                        .small()
                        .color(ink::forest(dark)),
                );
                last = Some(day);
            }
            lines::write(
                ui,
                egui::RichText::new(format!("{stage}, for {}", project.the()))
                    .small()
                    .color(ink::muted(dark)),
            );
        }
        if all > LATEST {
            let words = if self.made_shown_whole {
                "Only the latest".to_owned()
            } else {
                format!("All {all}")
            };
            let link = lines::write_with(
                ui,
                egui::RichText::new(words)
                    .small()
                    .underline()
                    .color(ink::forest(dark)),
                egui::Sense::click(),
            );
            if link.clicked() {
                self.made_shown_whole = !self.made_shown_whole;
            }
        }
    }
}

/// One thing a stage asks for, as the board writes it: what, with how far along it is if it is
/// counted, and whether the household has it.
struct NeedLine {
    text: String,
    met: bool,
}

/// What a project's picture on the board is: the piece its next stage makes, or the last one it
/// made, once it is finished.
fn picture_of(project: &Project, done: usize) -> Option<&'static catalog::Piece> {
    let at = done.min(project.stages.len() - 1);
    project.stages[..=at]
        .iter()
        .rev()
        .flat_map(|stage| stage.gifts.iter())
        .find_map(|gift| match gift {
            Gift::Piece(id) | Gift::Upgrade { to: id, .. } => catalog::piece(&CatalogId::known(id)),
            Gift::Finish(_) => None,
        })
}

/// A mark before a line of what a stage asks, on that line: filled once the household has it, a
/// ring until it does. Painted, as the notebook's lettering has no tick.
fn mark(ui: &mut egui::Ui, met: bool, unit: f32) {
    let dark = ui.visuals().dark_mode;
    let height = lines::RuleGrid::of(ui.ctx()).map_or(4.0 * unit, |grid| grid.pitch);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(5.0 * unit, height), egui::Sense::hover());
    let centre = egui::pos2(rect.center().x, rect.bottom() - 4.5 * unit);
    let radius = 1.3 * unit;
    if met {
        ui.painter()
            .circle_filled(centre, radius, ink::forest(dark));
    } else {
        ui.painter().circle_stroke(
            centre,
            radius,
            egui::Stroke::new(unit * 0.6, ink::muted(dark)),
        );
    }
}

/// A dot for each stage, from `at`: filled for each finished, ringed for one ready. Where they
/// end.
fn pips(
    painter: &egui::Painter,
    at: egui::Pos2,
    stages: usize,
    done: usize,
    standing: Standing,
    unit: f32,
    dark: bool,
) -> f32 {
    let gap = 4.5 * unit;
    let radius = 1.3 * unit;
    for stage in 0..stages {
        let centre = at + egui::vec2(gap * (stage as f32 + 0.5), 0.0);
        if stage < done {
            painter.circle_filled(centre, radius, ink::forest(dark));
        } else if standing == Standing::Ready(stage) {
            painter.circle_stroke(centre, radius, egui::Stroke::new(unit, ink::stamp(dark)));
        } else {
            painter.circle_stroke(
                centre,
                radius,
                egui::Stroke::new(unit * 0.6, ink::muted(dark)),
            );
        }
    }
    at.x + gap * stages as f32
}
