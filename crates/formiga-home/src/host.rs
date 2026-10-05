//! Who the house is open for. A real visit answers Desktop through the session's files; a
//! rehearsal — the sample household, or a house in a colony file opened for development — stands
//! in for Desktop itself, keeping each arrangement only as Desktop would keep it, through the
//! contract's own [`accept_result`], so a rehearsal proves the same loop a visit does.

use crate::session::{HOME_VERSION, Visit};
use crate::store::RehearsalHomes;
use anyhow::Result;
use formiga_home_contract::{
    HomeResult, HomeSnapshot, HomeState, SessionSeal, accept_result, encode,
};
use time::OffsetDateTime;

pub enum Host {
    Visit(Visit),
    Rehearsal(Rehearsal),
}

pub struct Rehearsal {
    snapshot: HomeSnapshot,
    /// The homes as the rehearsal started: what Desktop would have sent.
    sent: HomeState,
    seal: SessionSeal,
    homes: RehearsalHomes,
    pub label: String,
}

impl Rehearsal {
    pub fn new(snapshot: HomeSnapshot, homes: RehearsalHomes, label: String) -> Self {
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

    /// The owner is leaving the house.
    pub fn leave(&mut self, state: &HomeState) -> Result<()> {
        match self {
            Self::Visit(visit) => visit.leave(state),
            Self::Rehearsal(_) => self.keep(state),
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
            ))
        };
        let mut first = open();
        assert!(first.state().households.is_empty());
        let mut arranged = first.state().clone();
        let home = crate::staging::lived_in(&household, "floor.rose", "wall.plaster");
        arranged.set_household(home.clone());
        first.leave(&arranged).unwrap();
        let second = open();
        assert_eq!(second.state().household(home.keeper), Some(&home));
        let _ = std::fs::remove_dir_all(&data);
    }
}
