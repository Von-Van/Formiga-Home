//! Friends who come over, and going over to theirs: asking a visitor to stay over or to move in,
//! and going next door.

use super::*;

impl HomeApp {
    /// Asking a visitor to stay over, while it is visiting and not staying already.
    pub(super) fn stay_over_entry(&self, id: Id) -> Option<(Entry, String)> {
        (self.household.is_visitor(id)
            && self.life.present().contains(&id)
            && !self.life.staying(id))
        .then(|| {
            (
                Entry::StayOver(id),
                format!("Ask {} to stay over", self.name(id)),
            )
        })
    }

    /// Asking a visitor to move in, once a visit, where somebody will hear of it.
    pub(super) fn move_in_entry(&self, id: Id) -> Option<(Entry, String)> {
        (self.household.is_visitor(id)
            && self.life.present().contains(&id)
            && self.move_in.is_none()
            && self.host.hears_move_ins())
        .then(|| {
            (
                Entry::MoveIn(id),
                format!("Ask {} to move in", self.name(id)),
            )
        })
    }

    /// A friend asked to come and live here: it says whether it would like to, and if so, Desktop
    /// hears of it on leaving and decides.
    pub(super) fn ask_to_move_in(&mut self, visitor: Id) {
        let name = self.name(visitor);
        let warmth = self.household.friend_of(visitor).map_or(0.0, |friend| {
            life::band(self.household.bond(friend.id, visitor).warmth)
        });
        let Some(character) = self
            .household
            .resident(visitor)
            .map(|visitor| visitor.character.clone())
        else {
            return;
        };
        if !character.would_move_in(warmth) {
            self.say(format!("{name} is happy in their own house."));
            return;
        }
        self.move_in = Some(visitor);
        self.note(HomeMoment::AskedToMoveIn {
            visitor: TravelerId(visitor),
        });
        match &self.host {
            Host::Visit(_) => self.say(format!(
                "{name} would like that. Whether they move in is settled at home in the village."
            )),
            Host::Rehearsal(_) => self.say(format!(
                "{name} would like that, but nobody moves house in a rehearsal."
            )),
        }
    }

    /// Go over to the house `keeper` keeps. On a visit, the house is left and Desktop opens that
    /// one next, if it will; a rehearsal opens it itself, in the same window.
    pub(super) fn go_next_door(&mut self, ctx: &egui::Context, keeper: TravelerId) {
        let name = self.household.snapshot.neighbour(keeper).map_or_else(
            || "the house next door".to_owned(),
            |house| format!("{}'s house", house.name),
        );
        self.next_door = Some(keeper);
        let (colony, label) = match &self.host {
            Host::Visit(_) => {
                self.say(format!("Off to {name}…"));
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            Host::Rehearsal(rehearsal) => (rehearsal.colony.clone(), rehearsal.label.clone()),
        };
        self.leave();
        let opened = colony
            .open(crate::host::Which::Kept(keeper))
            .and_then(|snapshot| {
                let household = Household::new(snapshot.clone())
                    .map_err(|error| anyhow::anyhow!("could not draw the household: {error}"))?;
                Ok((snapshot, household))
            });
        match opened {
            Ok((snapshot, household)) => {
                let homes = store::RehearsalHomes::new(self.data.as_deref(), &snapshot.colony_key);
                let host =
                    Host::Rehearsal(crate::host::Rehearsal::new(snapshot, homes, label, colony));
                let open = self._open.take();
                let data = self.data.clone();
                *self = HomeApp::new(ctx, household, host, data, open);
                self.say(format!("Over at {name}."));
            }
            Err(error) => {
                eprintln!("formiga-home: {error:#}");
                self.next_door = None;
                self.left = false;
                self.say(format!("{name} could not be opened."));
            }
        }
    }
}
