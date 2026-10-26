//! What happens in the house, as the household's life tells the window, and what is kept of it:
//! keepsakes made, favourites come to, lines in the journal, and what was lived there, for Desktop.

use super::*;
use crate::life::{Event, Used};
use crate::session::Lived;
use formiga_home_contract::FavouriteKind;

/// What is said when a keepsake has nowhere to go, after a "but".
pub(super) const DRAWER_FULL: &str =
    "the drawer of keepsakes is full. Let one go, from the finds page, to make room.";

impl HomeApp {
    /// What has happened in the house since the last frame.
    pub(super) fn events(&mut self) {
        for event in self.life.take_events() {
            match event {
                Event::Did(_, count) => self.count(count),
                Event::Arrived(id) => {
                    let notice = match self.household.friend_of(id) {
                        Some(friend) => {
                            format!("{} has come over to see {}.", self.name(id), friend.name)
                        }
                        None => format!("{} has come over.", self.name(id)),
                    };
                    self.say(notice);
                    // Counted after the arrival is said, so that a stage it readies is what the
                    // window says, rather than lost under the arrival.
                    self.count(crate::projects::Count::Visits);
                    self.note(HomeMoment::Visit {
                        visitor: TravelerId(id),
                    });
                }
                Event::Left(id, gift) => {
                    let left = gift.map(|kind| {
                        let kept = self.make_keepsake(kind, Some(id), Vec::new()).is_some();
                        (kind, kept)
                    });
                    match left {
                        Some((kind, true)) => self.say(format!(
                            "{} has gone home, and left {} for the drawer.",
                            self.name(id),
                            keepsakes::a(kind)
                        )),
                        Some((kind, false)) => {
                            self.drawer_full_said = true;
                            self.say(format!(
                                "{} has gone home. There was {} for the drawer, but {}",
                                self.name(id),
                                keepsakes::a(kind),
                                DRAWER_FULL
                            ));
                        }
                        None => self.say(format!("{} has gone home.", self.name(id))),
                    }
                    if self.selected == Some(id) {
                        self.selected = None;
                    }
                }
                Event::StayingOver(id) => {
                    self.say(format!("{} is staying over.", self.name(id)));
                    self.count(crate::projects::Count::Sleepovers);
                    self.note(HomeMoment::StayedOver {
                        visitor: TravelerId(id),
                    });
                }
                Event::Drew(by, of) => {
                    let whom = if of == by {
                        "itself".to_owned()
                    } else {
                        self.name(of)
                    };
                    if self
                        .make_keepsake(MementoKind::Drawing, Some(by), vec![of])
                        .is_some()
                    {
                        self.say(format!(
                            "{} has drawn {whom}. The drawing is in the drawer.",
                            self.name(by)
                        ));
                    } else if !self.drawer_full_said {
                        // Said once a visit: little ones draw often.
                        self.drawer_full_said = true;
                        self.say(format!(
                            "{} has drawn {whom}, but {}",
                            self.name(by),
                            DRAWER_FULL
                        ));
                    }
                }
                Event::Used(id, used) => {
                    let liked = match used {
                        Used::Piece(name) => self.house.liked(name),
                        Used::Shown(item) => Some(Liked::Shown { item }),
                    };
                    let Some(liked) = liked else { continue };
                    let was = self.favourites(id).iter().any(|(_, thing)| *thing == liked);
                    if let Some(home) = self.state.household_mut(self.keeper) {
                        home.note_use(TravelerId(id), liked.clone());
                    }
                    // A favourite just now come to is worth a line in the journal.
                    let now = self
                        .favourites(id)
                        .into_iter()
                        .find(|(_, thing)| *thing == liked)
                        .map(|(kind, _)| kind);
                    if let (Some(kind), false) = (now, was) {
                        self.note(HomeMoment::Favourite {
                            resident: TravelerId(id),
                            thing: match kind {
                                life::Kind::Seat => FavouriteKind::Seat,
                                life::Kind::Bed => FavouriteKind::Bed,
                                life::Kind::Toy => FavouriteKind::Toy,
                                life::Kind::Find => FavouriteKind::Find,
                            },
                        });
                    }
                }
            }
        }
    }

    /// A resident's favourites in the house, as they now stand.
    fn favourites(&self, id: Id) -> Vec<(life::Kind, Liked)> {
        life::favourites(&home_of(&self.state, self.keeper).likings, &self.house, id)
    }

    /// A line in the household's journal, as of now.
    pub(super) fn note(&mut self, moment: HomeMoment) {
        if let Some(home) = self.state.household_mut(self.keeper) {
            home.note(now_utc(), moment);
        }
    }

    /// What was lived in the house while it was open, for Desktop: who spent time together and
    /// how, and the few moments most worth a line in its journal — a keepsake first, then a new
    /// room, a new favourite, a friend come over — in the order they happened.
    pub(super) fn lived(&self) -> Lived {
        let home = home_of(&self.state, self.keeper);
        let worth = |moment: &HomeMoment| match moment {
            HomeMoment::Project { .. } => 0,
            HomeMoment::Memento { .. } => 1,
            HomeMoment::AskedToMoveIn { .. } => 2,
            HomeMoment::StayedOver { .. } => 3,
            HomeMoment::Room { .. } => 4,
            HomeMoment::Favourite { .. } => 5,
            HomeMoment::Visit { .. } => 6,
            HomeMoment::Unknown => 7,
        };
        let mut moments: Vec<_> = written_since(&home.journal, &self.journal_at_open)
            .iter()
            .filter(|entry| match &entry.moment {
                // A room built and taken back again is no news.
                HomeMoment::Room { room } => home
                    .rooms
                    .iter()
                    .any(|layout| layout.kind.as_ref() == Some(room)),
                HomeMoment::Unknown => false,
                _ => true,
            })
            .collect();
        moments.sort_by_key(|entry| (worth(&entry.moment), entry.at_utc));
        moments.truncate(formiga_home_contract::limits::MAX_MOMENTS);
        moments.sort_by_key(|entry| entry.at_utc);
        Lived {
            together: self
                .life
                .together()
                .into_iter()
                .map(|(a, b, how, times)| (TravelerId(a), TravelerId(b), how, times))
                .collect(),
            moments: moments
                .into_iter()
                .map(|entry| entry.moment.clone())
                .collect(),
            next_door: self.next_door,
            move_in: self.move_in.map(|friend| (TravelerId(friend), self.keeper)),
        }
    }

    /// A new keepsake for the house, from `by` and of `of`, each of them as they look now,
    /// handed back to Desktop at once. Its id, if the house had room for it.
    pub(super) fn make_keepsake(
        &mut self,
        kind: MementoKind,
        by: Option<Id>,
        of: Vec<Id>,
    ) -> Option<DisplayId> {
        let of: Vec<_> = of
            .iter()
            .filter_map(|id| self.household.resident(*id))
            .map(|resident| (TravelerId(resident.id), keepsakes::ink_of(resident)))
            .collect();
        let home = self.state.household_mut(self.keeper)?;
        let id = keepsakes::make(home, kind, by.map(TravelerId), of, now_utc())?;
        if let Some(memento) = home.memento(&id) {
            self.arranging.came(home, memento);
        }
        self.keepsakes_changed();
        self.keep();
        Some(id)
    }
}

/// The lines of `journal` written since it was `then`. The journal only grows while the house is
/// open, its oldest lines let go once it is full, so what is left of `then` begins it and the rest
/// is new. Told apart by the lines themselves rather than by when each says it was written, so a
/// clock put back during the visit loses none of its moments, and lines written on an earlier
/// visit while the clock was ahead are not sent again.
fn written_since<'a>(journal: &'a [JournalEntry], then: &[JournalEntry]) -> &'a [JournalEntry] {
    (0..=journal.len().min(then.len()))
        .rev()
        .find(|&kept| journal[..kept] == then[then.len() - kept..])
        .map_or(journal, |kept| &journal[kept..])
}
