//! The household's own keepsakes: what a friend leaves on going home, what its little ones draw,
//! and the photos the owner frames. Home makes them, from the household's home, and shows them
//! like anything the colony has, in this house only: none ever goes to another house, and Desktop
//! only keeps them.

use crate::art;
use crate::character::Character;
use crate::household::Resident;
use formiga_art::Canvas;
use formiga_core::TemperamentKind;
use formiga_home_contract::limits::MAX_MEMENTOS;
use formiga_home_contract::{
    DisplayId, DisplayItem, DisplayMode, DisplaySource, HomeMoment, HomeSnapshot, HouseholdHome,
    Ink, Memento, MementoKind, TravelerId,
};
use std::collections::HashMap;
use time::OffsetDateTime;

/// Whether a thing is one of the household's own keepsakes, not something the colony has.
pub fn is_keepsake(item: &DisplayItem) -> bool {
    matches!(item.source, DisplaySource::HomeMemento { .. })
}

/// The colony's own things, as Desktop sent them, without the household's keepsakes.
pub fn colony_things(snapshot: &HomeSnapshot) -> impl Iterator<Item = &DisplayItem> {
    snapshot.inventory.iter().filter(|item| !is_keepsake(item))
}

/// Someone's name, as far as the household knows them: one of its own, a friend visiting, or the
/// keeper of another house in the village.
pub fn name_of(snapshot: &HomeSnapshot, id: TravelerId) -> Option<String> {
    snapshot
        .resident(id)
        .or_else(|| snapshot.visitor(id))
        .map(|traveler| traveler.name.clone())
        .or_else(|| snapshot.neighbour(id).map(|house| house.name.clone()))
}

/// Names in a line: "Mochi", "Mochi and Pip", "Mochi, Pip and Biscuit", and no more than three.
fn names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        [a, b, c] => format!("{a}, {b} and {c}"),
        [a, b, ..] => format!("{a}, {b} and others"),
    }
}

/// What a keepsake is called: "Postcard from Biscuit", "Pip's drawing of Mochi", "Photo of Mochi
/// and Pip".
pub fn name(memento: &Memento, snapshot: &HomeSnapshot) -> String {
    let by = memento.by.and_then(|id| name_of(snapshot, id));
    let of: Vec<String> = memento
        .of
        .iter()
        .filter_map(|id| name_of(snapshot, *id))
        .collect();
    let from = |what: &str| match &by {
        Some(by) => format!("{what} from {by}"),
        None => format!("{what} from a friend"),
    };
    match memento.kind {
        MementoKind::Postcard => from("Postcard"),
        MementoKind::JamJar => from("Jar of jam"),
        MementoKind::PressedFlower => from("Pressed flower"),
        MementoKind::Rosette => from("Rosette"),
        MementoKind::Pebble => from("Painted pebble"),
        MementoKind::Drawing => match (&by, of.is_empty()) {
            (Some(by), false) => format!("{by}'s drawing of {}", names(&of)),
            (Some(by), true) => format!("{by}'s drawing"),
            (None, false) => format!("A drawing of {}", names(&of)),
            (None, true) => "A drawing".to_owned(),
        },
        MementoKind::Photo if of.is_empty() => "A photo of home".to_owned(),
        MementoKind::Photo => format!("Photo of {}", names(&of)),
        MementoKind::Unknown => "A keepsake".to_owned(),
    }
}

/// A keepsake of a kind, as a notice says it: "a postcard".
pub fn a(kind: MementoKind) -> &'static str {
    match kind {
        MementoKind::Postcard => "a postcard",
        MementoKind::JamJar => "a jar of jam",
        MementoKind::PressedFlower => "a pressed flower",
        MementoKind::Rosette => "a rosette",
        MementoKind::Pebble => "a painted pebble",
        MementoKind::Drawing => "a drawing",
        MementoKind::Photo => "a photo",
        MementoKind::Unknown => "a keepsake",
    }
}

/// How a keepsake may be shown, best first: a card goes on the wall or stands on a shelf, a jar
/// sits on one, and a photo hangs or stands in its frame.
fn modes(kind: MementoKind) -> Vec<DisplayMode> {
    use DisplayMode::*;
    let mut modes = match kind {
        MementoKind::Postcard | MementoKind::Drawing => vec![Wall, SurfaceSmall],
        MementoKind::PressedFlower => vec![Wall, Case],
        MementoKind::Rosette => vec![Wall, Textile],
        MementoKind::JamJar | MementoKind::Pebble => vec![SurfaceSmall, Case],
        MementoKind::Photo => vec![Wall, SurfaceSmall],
        MementoKind::Unknown => Vec::new(),
    };
    modes.push(FallbackCard);
    modes
}

/// A keepsake of the household of `keeper`, as a thing its house can show.
pub fn item(keeper: TravelerId, memento: &Memento, snapshot: &HomeSnapshot) -> DisplayItem {
    DisplayItem {
        id: memento.id(keeper),
        source: DisplaySource::HomeMemento {
            memento: memento.kind,
        },
        name: name(memento, snapshot),
        ink: None,
        modes: modes(memento.kind),
        found_at_utc: memento.made_at_utc,
        finder_name: memento.by.and_then(|id| name_of(snapshot, id)),
    }
}

/// The household's things with its keepsakes among them, after everything the colony has, so
/// whatever finds a thing by its id finds a keepsake too.
pub fn stock(snapshot: &mut HomeSnapshot, home: &HouseholdHome) {
    snapshot.inventory.retain(|item| !is_keepsake(item));
    let keepsakes: Vec<DisplayItem> = home
        .mementos
        .iter()
        .map(|memento| item(home.keeper, memento, snapshot))
        .collect();
    snapshot.inventory.extend(keepsakes);
}

/// Every keepsake's picture, by id, with whoever is in it.
pub fn pictures(home: &HouseholdHome) -> HashMap<DisplayId, Canvas> {
    home.mementos
        .iter()
        .map(|memento| (memento.id(home.keeper), art::keepsakes::picture(memento)))
        .collect()
}

/// The colours someone is drawn in, for a keepsake that has them in it.
pub fn ink_of(resident: &Resident) -> Ink {
    Ink::of(trinket_ink(formiga_art::palette_for(resident.genome())))
}

fn trinket_ink(palette: formiga_art::Palette) -> formiga_art::TrinketInk {
    formiga_art::TrinketInk {
        outline: palette.outline,
        deep: palette.shadow,
        body: palette.coat,
        light: palette.highlight,
        accent: palette.accent,
    }
}

/// What a friend leaves on going home, as suits it: a postcard from one who gets about, a
/// pressed flower from a quiet or studious one, a rosette from a show-off, jam from the warm
/// and the homely, and a painted pebble from anyone up to mischief.
pub fn gift(character: &Character) -> MementoKind {
    match character.kind {
        TemperamentKind::Explorer | TemperamentKind::Oddball => MementoKind::Postcard,
        TemperamentKind::Scholar | TemperamentKind::Wallflower => MementoKind::PressedFlower,
        TemperamentKind::Showoff => MementoKind::Rosette,
        TemperamentKind::Sweetheart | TemperamentKind::Guardian | TemperamentKind::Grump => {
            MementoKind::JamJar
        }
        TemperamentKind::Troublemaker | TemperamentKind::Lazybones => MementoKind::Pebble,
    }
}

/// How likely a friend is to leave something on going home: the warmer the friendship, and the
/// warmer the friend, the likelier.
pub fn gives(character: &Character, warmth: f32) -> f32 {
    let warm = match character.kind {
        TemperamentKind::Sweetheart => 0.25,
        TemperamentKind::Showoff | TemperamentKind::Guardian => 0.15,
        TemperamentKind::Grump | TemperamentKind::Wallflower => -0.1,
        _ => 0.0,
    };
    (0.2 + warmth * 0.35 + character.axes.affection * 0.2 + warm).clamp(0.05, 0.85)
}

/// Whether the household has room for another keepsake.
pub fn has_room(home: &HouseholdHome) -> bool {
    home.mementos.len() < MAX_MEMENTOS
}

/// A new keepsake for the house, numbered and noted in its journal. Its id, if there was room
/// for it.
pub fn make(
    home: &mut HouseholdHome,
    kind: MementoKind,
    by: Option<TravelerId>,
    of: Vec<(TravelerId, Ink)>,
    at: OffsetDateTime,
) -> Option<DisplayId> {
    if !has_room(home) {
        return None;
    }
    let at = at.replace_nanosecond(0).unwrap_or(at);
    let of: Vec<_> = of
        .into_iter()
        .take(formiga_home_contract::limits::MAX_IN_A_MEMENTO)
        .collect();
    let memento = Memento {
        serial: home.next_memento(),
        kind,
        by,
        of: of.iter().map(|(id, _)| *id).collect(),
        inks: of.iter().map(|(_, ink)| *ink).collect(),
        made_at_utc: at,
    };
    let id = memento.id(home.keeper);
    home.note(
        at,
        HomeMoment::Memento {
            memento: kind,
            by,
            of: memento.of.clone(),
        },
    );
    home.mementos.push(memento);
    Some(id)
}

/// Let a keepsake go: off show, no one's favourite any more, and out of the house's keeping.
pub fn let_go(home: &mut HouseholdHome, id: &DisplayId) {
    let Some(serial) = home.memento(id).map(|memento| memento.serial) else {
        return;
    };
    home.take_down(id);
    home.mementos.retain(|memento| memento.serial != serial);
    home.forget_what_is_gone();
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::sample;

    #[test]
    fn a_keepsake_is_named_for_who_it_came_from_and_who_is_in_it() {
        let snapshot = sample::snapshot();
        let (keeper, little) = (snapshot.residents[0].id, snapshot.residents[1].id);
        let friend = snapshot.visitors[0].id;
        let at = snapshot.created_at_utc;
        let memento = |kind, by, of: Vec<TravelerId>| Memento {
            serial: 1,
            kind,
            by,
            of,
            inks: Vec::new(),
            made_at_utc: at,
        };
        let (keeper_name, little_name) = (&snapshot.residents[0].name, &snapshot.residents[1].name);
        assert_eq!(
            name(
                &memento(MementoKind::Postcard, Some(friend), vec![]),
                &snapshot
            ),
            format!("Postcard from {}", snapshot.visitors[0].name)
        );
        assert_eq!(
            name(
                &memento(MementoKind::Drawing, Some(little), vec![keeper]),
                &snapshot
            ),
            format!("{little_name}'s drawing of {keeper_name}")
        );
        assert_eq!(
            name(
                &memento(MementoKind::Photo, None, vec![keeper, little]),
                &snapshot
            ),
            format!("Photo of {keeper_name} and {little_name}")
        );
        // Someone the household no longer knows is still a friend.
        assert_eq!(
            name(
                &memento(MementoKind::JamJar, Some(TravelerId(4040)), vec![]),
                &snapshot
            ),
            "Jar of jam from a friend"
        );
    }

    #[test]
    fn keepsakes_are_stocked_after_the_colony_s_things_and_let_go_whole() {
        let snapshot = sample::snapshot();
        let mut stocked = snapshot.clone();
        let mut home = crate::starter::home(&snapshot);
        let colony = snapshot.inventory.len();
        let friend = snapshot.visitors[0].id;
        let id = make(
            &mut home,
            MementoKind::Rosette,
            Some(friend),
            Vec::new(),
            snapshot.created_at_utc,
        )
        .unwrap();
        stock(&mut stocked, &home);
        stock(&mut stocked, &home);
        assert_eq!(stocked.inventory.len(), colony + 1);
        assert_eq!(colony_things(&stocked).count(), colony);
        let shown = stocked.item(&id).unwrap();
        assert!(shown.allows(DisplayMode::Wall));
        assert!(matches!(
            home.journal.last().map(|entry| &entry.moment),
            Some(HomeMoment::Memento {
                memento: MementoKind::Rosette,
                ..
            })
        ));
        crate::placement::show(
            &mut home,
            0,
            &id,
            formiga_home_contract::Spot::Wall {
                side: formiga_home_contract::WallSide::North,
                at: 1,
            },
        );
        let_go(&mut home, &id);
        assert!(home.mementos.is_empty());
        assert!(home.shown().all(|shown| shown != &id));
        stock(&mut stocked, &home);
        assert_eq!(stocked.inventory.len(), colony);
    }

    #[test]
    fn a_full_house_makes_no_more_keepsakes() {
        let snapshot = sample::snapshot();
        let mut home = crate::starter::home(&snapshot);
        for _ in 0..MAX_MEMENTOS {
            assert!(
                make(
                    &mut home,
                    MementoKind::Pebble,
                    None,
                    Vec::new(),
                    snapshot.created_at_utc
                )
                .is_some()
            );
        }
        assert!(
            make(
                &mut home,
                MementoKind::Pebble,
                None,
                Vec::new(),
                snapshot.created_at_utc
            )
            .is_none()
        );
        assert_eq!(home.mementos.len(), MAX_MEMENTOS);
    }
}
