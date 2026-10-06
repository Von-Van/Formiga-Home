//! Who the house is open for. A real visit answers Desktop through the session's files; a
//! rehearsal — the sample household, or a house in a colony file opened for development — stands
//! in for Desktop itself, keeping each arrangement only as Desktop would keep it, through the
//! contract's own [`accept_result`], so a rehearsal proves the same loop a visit does.

use crate::session::{HOME_VERSION, Lived, Visit};
use crate::store::RehearsalHomes;
use anyhow::{Context, Result};
use formiga_home_contract::{
    HomeCapability, HomeResult, HomeSnapshot, HomeState, SessionId, SessionSeal, TravelerId,
    accept_result, encode, likely_visitors, project_household, sample,
};
use std::path::PathBuf;
use time::OffsetDateTime;

pub enum Host {
    Visit(Visit),
    Rehearsal(Rehearsal),
}

/// Where a rehearsal's houses come from: Desktop's sample colony, or a colony file read and never
/// written. Any house in it can be opened as Desktop would open it, so a rehearsal can go next
/// door too.
#[derive(Clone, Debug, PartialEq)]
pub enum Colony {
    Sample,
    Save(PathBuf),
}

/// Which of a colony's houses to open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Which {
    /// By its place in the village, the colony house first.
    Nth(usize),
    /// The one its keeper keeps.
    Kept(TravelerId),
}

impl Colony {
    /// A house of the colony, projected exactly as Desktop would open it, with its closest
    /// friends lent as Desktop would lend them if they were free.
    pub fn open(&self, which: Which) -> Result<HomeSnapshot> {
        let (save, made, label) = match self {
            Self::Sample if which == Which::Nth(0) => return Ok(sample::snapshot()),
            Self::Sample => (
                sample::colony(),
                sample::MADE + time::Duration::days(sample::DAYS_LIVED),
                "sample colony",
            ),
            Self::Save(path) => (
                formiga_core::SaveStore::read_snapshot(path)
                    .with_context(|| format!("could not read the colony file {}", path.display()))?
                    .into_inner(),
                OffsetDateTime::now_utc(),
                "a colony file",
            ),
        };
        let owners = formiga_core::house_owners(&save.creatures, &save.home.cottage_order);
        let keeper = match which {
            Which::Nth(house) => *owners
                .as_slice()
                .get(house)
                .with_context(|| format!("the colony has {} houses", owners.as_slice().len()))?,
            Which::Kept(keeper) => *owners
                .as_slice()
                .iter()
                .find(|owner| **owner == keeper.0)
                .context("nobody keeps that house")?,
        };
        let visitors = likely_visitors(&save, keeper);
        Ok(project_household(
            &save,
            keeper,
            &visitors,
            SessionId::generate().context("no randomness for a session id")?,
            made,
            label,
        )?)
    }
}

pub struct Rehearsal {
    snapshot: HomeSnapshot,
    /// The homes as the rehearsal started: what Desktop would have sent.
    sent: HomeState,
    seal: SessionSeal,
    homes: RehearsalHomes,
    pub label: String,
    pub colony: Colony,
}

impl Rehearsal {
    pub fn new(
        snapshot: HomeSnapshot,
        homes: RehearsalHomes,
        label: String,
        colony: Colony,
    ) -> Self {
        let sent = homes.load(&snapshot.colony_key).settled_for(&snapshot);
        let seal = SessionSeal::of(
            &snapshot,
            &encode(&snapshot).unwrap_or_default(),
            &encode(&sent).unwrap_or_default(),
        );
        Self {
            snapshot,
            sent,
            seal,
            homes,
            label,
            colony,
        }
    }
}

impl Host {
    /// The homes as they stood when the house was opened.
    pub fn state(&self) -> &HomeState {
        match self {
            Self::Visit(visit) => &visit.state,
            Self::Rehearsal(rehearsal) => &rehearsal.sent,
        }
    }

    /// Hand back the homes as they now stand.
    pub fn keep(&mut self, state: &HomeState) -> Result<()> {
        match self {
            Self::Visit(visit) => visit.keep(state),
            Self::Rehearsal(rehearsal) => {
                let result = HomeResult::new(
                    &rehearsal.seal,
                    OffsetDateTime::now_utc(),
                    HOME_VERSION,
                    state.clone(),
                );
                let accepted = accept_result(
                    &rehearsal.seal,
                    &rehearsal.snapshot,
                    &rehearsal.sent,
                    &result,
                );
                if !accepted.set_aside.is_empty() {
                    eprintln!(
                        "formiga-home: Desktop would set aside {:?}",
                        accepted.set_aside
                    );
                }
                rehearsal.homes.save(&accepted.state)
            }
        }
    }

    /// The owner is leaving the house, and this is what was lived there. A rehearsal keeps the
    /// homes; what was lived is for Desktop alone.
    pub fn leave(&mut self, state: &HomeState, lived: &Lived) -> Result<()> {
        match self {
            Self::Visit(visit) => visit.leave(state, lived),
            Self::Rehearsal(_) => self.keep(state),
        }
    }

    /// Whether the owner can go over to another house from this one: Desktop opens it next if it
    /// offers to; a rehearsal opens it itself.
    pub fn goes_next_door(&self) -> bool {
        match self {
            Self::Visit(visit) => visit.snapshot.offers(HomeCapability::NextDoor),
            Self::Rehearsal(_) => true,
        }
    }

    /// Whether a friend can be asked to move in: Desktop considers it if it offers to; a
    /// rehearsal hears it, and nobody moves.
    pub fn hears_move_ins(&self) -> bool {
        match self {
            Self::Visit(visit) => visit.snapshot.offers(HomeCapability::Roommates),
            Self::Rehearsal(_) => true,
        }
    }

    /// Whether Desktop has taken the household back already.
    pub fn recalled(&self) -> bool {
        match self {
            Self::Visit(visit) => visit.recalled(),
            Self::Rehearsal(_) => false,
        }
    }

    /// What the window says about where the household came from, for a rehearsal.
    pub fn rehearsal_label(&self) -> Option<&str> {
        match self {
            Self::Visit(_) => None,
            Self::Rehearsal(rehearsal) => Some(&rehearsal.label),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::household::Household;
    use formiga_home_contract::sample;

    #[test]
    fn a_rehearsal_keeps_an_arrangement_across_closing_and_opening_again() {
        let data =
            std::env::temp_dir().join(format!("formiga-home-rehearsal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let snapshot = sample::snapshot();
        let household = Household::new(snapshot.clone()).unwrap();
        let open = || {
            Host::Rehearsal(Rehearsal::new(
                snapshot.clone(),
                RehearsalHomes::new(Some(&data), &snapshot.colony_key),
                "test".to_owned(),
                Colony::Sample,
            ))
        };
        let mut first = open();
        assert!(first.state().households.is_empty());
        let mut arranged = first.state().clone();
        let home = crate::staging::lived_in(&household, "floor.rose", "wall.plaster");
        arranged.set_household(home.clone());
        first.leave(&arranged, &Lived::default()).unwrap();
        let second = open();
        assert_eq!(second.state().household(home.keeper), Some(&home));
        let _ = std::fs::remove_dir_all(&data);
    }
}
