//! Opening a house: who comes through the door, what the colony has to show, and how the snapshot
//! is held to the contract either way.

mod common;

use common::session;
use formiga_core::{CreatureRole, house_owners};
use formiga_home_contract::*;
use std::collections::BTreeSet;
use time::Duration;

#[test]
fn the_sample_house_is_the_founder_and_its_little_one() {
    let snapshot = sample::snapshot();
    let keeper = &snapshot.residents[0];
    assert_eq!(keeper.id, snapshot.household.keeper);
    assert_eq!(
        snapshot.household.slot, 0,
        "the founder keeps the colony house"
    );
    assert_eq!(snapshot.residents.len(), 2);
    assert_eq!(snapshot.residents[1].name, "Pip");
    assert!(matches!(
        snapshot.residents[1].role,
        formiga_travel::TravelRole::Mini { parent_id } if parent_id == keeper.id
    ));
    assert_eq!(
        snapshot.relationships.len(),
        1,
        "the household's own bond, and nobody else's"
    );
    let adults = sample::colony()
        .creatures
        .iter()
        .filter(|creature| creature.role.is_adult())
        .count();
    assert_eq!(snapshot.village.len(), adults, "every adult keeps a house");
    assert_eq!(snapshot.days_lived, sample::DAYS_LIVED as u32);
}

#[test]
fn every_other_house_opens_to_its_own_keeper_alone() {
    let save = sample::colony();
    let owners = house_owners(&save.creatures, &save.home.cottage_order);
    for (slot, keeper) in owners.as_slice().iter().enumerate().skip(1) {
        let snapshot = project_household(
            &save,
            *keeper,
            session(),
            sample::MADE + Duration::days(2),
            "test",
        )
        .unwrap();
        assert_eq!(usize::from(snapshot.household.slot), slot);
        let ids: Vec<_> = snapshot.residents.iter().map(|r| r.id.0).collect();
        assert_eq!(
            ids,
            vec![*keeper],
            "nobody lives in house {slot} but its keeper"
        );
        assert!(snapshot.relationships.is_empty());
    }
}

#[test]
fn a_little_one_is_never_a_house_of_its_own() {
    let save = sample::colony();
    let mini = save
        .creatures
        .iter()
        .find(|creature| matches!(creature.role, CreatureRole::Mini { .. }))
        .unwrap();
    assert!(matches!(
        project_household(&save, mini.id, session(), sample::MADE, "test"),
        Err(ProjectionError::NoSuchHouse)
    ));
}

#[test]
fn everything_the_colony_has_can_be_shown_and_nothing_else() {
    let snapshot = sample::snapshot();
    let finds: BTreeSet<_> = sample::FINDS
        .iter()
        .map(|(variant, _)| DisplayId::find(*variant))
        .collect();
    let souvenirs: BTreeSet<_> = sample::SOUVENIRS
        .iter()
        .map(|souvenir| DisplayId::souvenir(souvenir.id()).unwrap())
        .collect();
    let listed: BTreeSet<_> = snapshot
        .inventory
        .iter()
        .map(|item| item.id.clone())
        .collect();
    assert_eq!(listed, finds.union(&souvenirs).cloned().collect());
    for item in &snapshot.inventory {
        assert!(
            item.allows(DisplayMode::FallbackCard),
            "{} has no card",
            item.id
        );
        match &item.source {
            DisplaySource::DesktopFind { variant } => {
                assert!(item.ink.is_some());
                assert_eq!(
                    item.name,
                    formiga_core::trinket_info(*variant).unwrap().name
                );
                assert!(item.finder_name.is_some(), "a find remembers who found it");
            }
            DisplaySource::HillSouvenir { id } => {
                assert!(item.ink.is_none(), "a souvenir has its own colours");
                assert!(formiga_core::Souvenir::from_id(id).is_some());
            }
            DisplaySource::Unknown => panic!("Desktop wrote a source it does not know"),
        }
    }
}

#[test]
fn a_souvenir_desktop_has_kept_reaches_every_house_without_hill() {
    let mut save = sample::colony();
    save.trips.souvenirs.clear();
    let keeper = sample::keeper(&save);
    let before = project_household(&save, keeper, session(), sample::MADE, "test").unwrap();
    assert!(
        before
            .item(&DisplayId::souvenir("well_penny").unwrap())
            .is_none()
    );
    save.trips.souvenirs.push(formiga_core::SouvenirRecord {
        souvenir: formiga_core::Souvenir::WellPenny,
        brought_home_at_utc: sample::MADE,
    });
    let after = project_household(&save, keeper, session(), sample::MADE, "test").unwrap();
    let penny = after
        .item(&DisplayId::souvenir("well_penny").unwrap())
        .unwrap();
    assert_eq!(penny.name, "Well penny");
}

#[test]
fn the_same_house_opened_twice_is_the_same_snapshot_byte_for_byte() {
    let a = encode(&sample::snapshot()).unwrap();
    let b = encode(&sample::snapshot()).unwrap();
    assert_eq!(a, b);
    let read: HomeSnapshot = decode(&a).unwrap();
    assert_eq!(read, sample::snapshot());
}

#[test]
fn a_snapshot_that_does_not_add_up_is_refused() {
    let base = sample::snapshot();
    let mut keeperless = base.clone();
    keeperless.residents.swap(0, 1);
    let mut lonely = base.clone();
    lonely.residents.clear();
    let mut twice = base.clone();
    twice.inventory.push(base.inventory[0].clone());
    let mut cardless = base.clone();
    cardless.inventory[0]
        .modes
        .retain(|mode| *mode != DisplayMode::FallbackCard);
    let mut impostor = base.clone();
    impostor.inventory[0].id = DisplayId::find(150);
    let mut homeless = base.clone();
    homeless
        .village
        .retain(|house| house.keeper != base.household.keeper);
    let mut shouting = base.clone();
    shouting.residents[0].name = "Pip\u{202E}".to_owned();
    let mut newer_looks = base;
    newer_looks.travel_min_reader_version = newer_looks.travel_version + 1;
    for snapshot in [
        keeperless,
        lonely,
        twice,
        cardless,
        impostor,
        homeless,
        shouting,
        newer_looks,
    ] {
        assert!(
            encode(&snapshot).is_err(),
            "{:?} was accepted",
            snapshot.household
        );
    }
}
