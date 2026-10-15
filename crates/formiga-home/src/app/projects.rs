//! The project board, on its own page of the notes while living in the house, and everything the
//! window does for the household's projects: counting what happens at home towards them, saying
//! quietly when one is ready or a souvenir from Formiga Hill counts, and finishing a stage when
//! the owner asks.
//!
//! The board is short until something on it is chosen: a line a project, saying where it
//! stands. Choosing one opens what it asks for, how far along each is, and what finishing it
//! gives. Below them, what the household has made, for good.

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
            ui.label(
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
        for project in shown {
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
                finish = Some(project);
            }
            ui.add_space(2.0 * unit);
        }
        let hidden = PROJECTS.len() - {
            let facts = self.facts();
            PROJECTS
                .iter()
                .filter(|project| project.standing(&facts) != Standing::Hidden)
                .count()
        };
        if hidden > 0 {
            ui.label(
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

    /// One project on the board: a picture of what it makes next, its name, and where it
    /// stands. The whole row chooses it.
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
        let picture = picture_of(project, done);
        let size = egui::vec2(24.0 * unit, 24.0 * unit);
        let response = ui
            .horizontal(|ui| {
                let (response, rect) =
                    tile(ui, egui::Id::new(("project", project.id)), size, open, unit);
                if let Some(piece) = picture {
                    let (texture, pixels) =
                        self.thumbnail(ctx, format!("piece:{}", piece.id), || {
                            cropped(crate::art::furniture::draw(piece, false).canvas)
                        });
                    let shown = crisp(ui, pixels, size - egui::vec2(5.0, 5.0) * unit, 2.0);
                    ui.painter().image(
                        texture,
                        egui::Rect::from_center_size(rect.center(), shown),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        if standing == Standing::Finished || done > 0 {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_white_alpha(110)
                        },
                    );
                }
                let text = ui
                    .vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(project.name).strong());
                            if new {
                                ui.label(label_job("New", LABEL - 1.5, ink::stamp(dark)));
                            }
                        });
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
                        ui.label(egui::RichText::new(line).size(12.0).color(colour));
                        pips(ui, project.stages.len(), done, standing, unit);
                    })
                    .response;
                response.union(ui.interact(
                    text.rect,
                    egui::Id::new(("project text", project.id)),
                    egui::Sense::click(),
                ))
            })
            .inner;
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, open, project.name)
        });
        response
    }

    /// A chosen project, opened under its row: what it is, what its next stage asks and how far
    /// along each is, what finishing it gives, and the stages already finished. Whether the owner
    /// asked to finish the stage.
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
                        label: need.label(),
                        met: need.met(&facts),
                        progress: need.counts().then(|| need.progress(&facts)),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut asked = false;
        ui.horizontal(|ui| {
            ui.add_space(4.0 * unit);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(label_job(project.kind.label(), LABEL, ink::forest(dark)));
                ui.label(
                    egui::RichText::new(project.about)
                        .size(12.0)
                        .italics()
                        .color(ink::muted(dark)),
                );
                if let Some(stage) = project.stages.get(done) {
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(stage.name).strong());
                    ui.label(egui::RichText::new(stage.about).size(12.0));
                    for NeedLine {
                        label,
                        met,
                        progress,
                    } in &lines
                    {
                        let text = match progress {
                            Some((have, want)) if !*met => format!("{label}: {have} of {want}"),
                            _ => label.clone(),
                        };
                        ui.horizontal(|ui| {
                            mark(ui, *met, unit);
                            ui.label(egui::RichText::new(text).size(12.0).color(if *met {
                                ink::forest(dark)
                            } else {
                                ink::muted(dark)
                            }));
                        });
                    }
                    if matches!(standing, Standing::Ready(_)) {
                        ui.add_space(2.0);
                        // Something new is built; something there already is finished.
                        let verb = match project.kind {
                            Kind::Construction | Kind::Legacy if done == 0 => "Build it",
                            _ => "Finish it",
                        };
                        asked = ui
                            .button(verb)
                            .on_hover_text("Whenever you like: nothing waits on it")
                            .clicked();
                    }
                }
                if !finished.is_empty() {
                    ui.add_space(2.0);
                    for (stage, at) in project.stages.iter().zip(&finished) {
                        let when = crate::journal::heading(
                            crate::journal::day_of(*at),
                            crate::journal::day_of(OffsetDateTime::now_utc()),
                        );
                        ui.horizontal(|ui| {
                            mark(ui, true, unit);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}, {}",
                                    stage.name,
                                    when.to_lowercase()
                                ))
                                .size(12.0)
                                .color(ink::muted(dark)),
                            );
                        });
                    }
                }
            });
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
        ui.add_space(6.0);
        kicker(ui, "Made at home");
        let today = crate::journal::day_of(OffsetDateTime::now_utc());
        let mut last = None;
        for (at, stage, project) in made {
            let day = crate::journal::day_of(at);
            if last != Some(day) {
                ui.label(
                    egui::RichText::new(crate::journal::heading(day, today))
                        .small()
                        .color(ink::forest(dark)),
                );
                last = Some(day);
            }
            ui.label(
                egui::RichText::new(format!("{stage}, for {}", project.the()))
                    .size(12.0)
                    .color(ink::muted(dark)),
            );
        }
        if all > LATEST {
            let words = if self.made_shown_whole {
                "Only the latest".to_owned()
            } else {
                format!("All {all}")
            };
            let link = ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(words)
                            .small()
                            .underline()
                            .color(ink::forest(dark)),
                    )
                    .sense(egui::Sense::click()),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if link.clicked() {
                self.made_shown_whole = !self.made_shown_whole;
            }
        }
    }
}

/// One thing a stage asks for, as the board writes it: what, whether the household has it, and
/// for what is counted, how far along it is.
struct NeedLine {
    label: String,
    met: bool,
    progress: Option<(u32, u32)>,
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

/// A mark before a line of what a stage asks: filled once the household has it, a ring until it
/// does. Painted, as the notebook's lettering has no tick.
fn mark(ui: &mut egui::Ui, met: bool, unit: f32) {
    let dark = ui.visuals().dark_mode;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(4.0 * unit, 4.0 * unit), egui::Sense::hover());
    let radius = 1.3 * unit;
    if met {
        ui.painter()
            .circle_filled(rect.center(), radius, ink::forest(dark));
    } else {
        ui.painter().circle_stroke(
            rect.center(),
            radius,
            egui::Stroke::new(unit * 0.6, ink::muted(dark)),
        );
    }
}

/// A dot for each stage: filled for each finished, ringed for one ready.
fn pips(ui: &mut egui::Ui, stages: usize, done: usize, standing: Standing, unit: f32) {
    let dark = ui.visuals().dark_mode;
    let gap = 5.0 * unit;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(gap * stages as f32, 4.0 * unit),
        egui::Sense::hover(),
    );
    for stage in 0..stages {
        let centre = rect.left_center() + egui::vec2(gap * (stage as f32 + 0.5), 0.0);
        let radius = 1.5 * unit;
        if stage < done {
            ui.painter()
                .circle_filled(centre, radius, ink::forest(dark));
        } else if standing == Standing::Ready(stage) {
            ui.painter()
                .circle_stroke(centre, radius, egui::Stroke::new(unit, ink::stamp(dark)));
        } else {
            ui.painter().circle_stroke(
                centre,
                radius,
                egui::Stroke::new(unit * 0.6, ink::line(dark)),
            );
        }
    }
}
