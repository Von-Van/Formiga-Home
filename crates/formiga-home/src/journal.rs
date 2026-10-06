//! The household's journal, as its page in the notebook words it: a line for each moment, under
//! the day it happened, as the owner's own clock has it. The journal itself is structured, and
//! Desktop words the few moments it is sent in its own way.

use crate::catalog;
use crate::keepsakes::{self, name_of};
use formiga_home_contract::{FavouriteKind, HomeMoment, HomeSnapshot, MementoKind, TravelerId};
use std::sync::OnceLock;
use time::{Date, OffsetDateTime, UtcOffset};

static LOCAL: OnceLock<UtcOffset> = OnceLock::new();

/// Read the owner's own clock's offset, while nothing else is running that could change it
/// under the reading. Called first thing; until it is, days are as UTC has them.
pub fn read_local_offset() {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let _ = LOCAL.set(offset);
}

/// The day a moment happened, on the owner's clock.
pub fn day_of(at: OffsetDateTime) -> Date {
    at.to_offset(*LOCAL.get().unwrap_or(&UtcOffset::UTC)).date()
}

/// A day's heading: "Today", "Yesterday", or "5 October".
pub fn heading(day: Date, today: Date) -> String {
    if day == today {
        "Today".to_owned()
    } else if today.previous_day() == Some(day) {
        "Yesterday".to_owned()
    } else {
        format!("{} {}", day.day(), day.month())
    }
}

/// A moment as one short line: "Biscuit came over.", "Pip drew Mochi.".
pub fn line(moment: &HomeMoment, snapshot: &HomeSnapshot) -> String {
    let who = |id: &TravelerId| name_of(snapshot, *id).unwrap_or_else(|| "A friend".to_owned());
    match moment {
        HomeMoment::Visit { visitor } => format!("{} came over.", who(visitor)),
        HomeMoment::Memento {
            memento: MementoKind::Drawing,
            by,
            of,
        } => {
            let by = by.as_ref().map_or_else(|| "A little one".to_owned(), who);
            match of.first() {
                Some(of) => format!("{by} drew {}.", who(of)),
                None => format!("{by} drew a picture."),
            }
        }
        HomeMoment::Memento {
            memento: MementoKind::Photo,
            of,
            ..
        } => {
            let names: Vec<String> = of.iter().map(who).collect();
            match names.as_slice() {
                [] => "A photo of home, framed.".to_owned(),
                [one] => format!("A photo of {one}, framed."),
                [rest @ .., last] => format!("A photo of {} and {last}, framed.", rest.join(", ")),
            }
        }
        HomeMoment::Memento { memento, by, .. } => {
            let by = by.as_ref().map_or_else(|| "A friend".to_owned(), who);
            format!("{by} left {}.", keepsakes::a(*memento))
        }
        HomeMoment::Favourite { resident, thing } => {
            let thing = match thing {
                FavouriteKind::Seat => "seat",
                FavouriteKind::Bed => "bed",
                FavouriteKind::Toy => "toy",
                FavouriteKind::Find => "find",
                FavouriteKind::Unknown => "thing",
            };
            format!("{} found a favourite {thing}.", who(resident))
        }
        HomeMoment::StayedOver { visitor } => format!("{} stayed over.", who(visitor)),
        HomeMoment::Room { room } => {
            let name = catalog::room_name(Some(room), false).to_lowercase();
            format!("The house grew a {name}.")
        }
        HomeMoment::Unknown => "Something happened.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::{CatalogId, sample};
    use time::macros::date;

    #[test]
    fn a_day_is_today_yesterday_or_its_date() {
        let today = date!(2026 - 10 - 05);
        assert_eq!(heading(today, today), "Today");
        assert_eq!(heading(date!(2026 - 10 - 04), today), "Yesterday");
        assert_eq!(heading(date!(2026 - 09 - 28), today), "28 September");
    }

    #[test]
    fn every_moment_is_a_short_line_in_the_household_s_names() {
        let snapshot = sample::snapshot();
        let (keeper, little) = (snapshot.residents[0].id, snapshot.residents[1].id);
        let friend = snapshot.visitors[0].id;
        let (keeper_name, little_name, friend_name) = (
            &snapshot.residents[0].name,
            &snapshot.residents[1].name,
            &snapshot.visitors[0].name,
        );
        let cases = [
            (
                HomeMoment::Visit { visitor: friend },
                format!("{friend_name} came over."),
            ),
            (
                HomeMoment::Memento {
                    memento: MementoKind::Drawing,
                    by: Some(little),
                    of: vec![keeper],
                },
                format!("{little_name} drew {keeper_name}."),
            ),
            (
                HomeMoment::Memento {
                    memento: MementoKind::JamJar,
                    by: Some(friend),
                    of: Vec::new(),
                },
                format!("{friend_name} left a jar of jam."),
            ),
            (
                HomeMoment::Memento {
                    memento: MementoKind::Photo,
                    by: None,
                    of: vec![keeper, little],
                },
                format!("A photo of {keeper_name} and {little_name}, framed."),
            ),
            (
                HomeMoment::Favourite {
                    resident: little,
                    thing: FavouriteKind::Toy,
                },
                format!("{little_name} found a favourite toy."),
            ),
            (
                HomeMoment::Room {
                    room: CatalogId::known("room.nook"),
                },
                "The house grew a reading nook.".to_owned(),
            ),
        ];
        for (moment, said) in cases {
            assert_eq!(line(&moment, &snapshot), said);
        }
    }
}
