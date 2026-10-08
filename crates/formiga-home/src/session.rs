//! Home's side of a visit, as the household contract lays it out.
//!
//! Desktop starts Home with `--formiga-home <session directory>`. Home reads the snapshot and the
//! state there and answers them once with an acknowledgement. While the house is open it writes
//! its result whole whenever the owner finishes arranging, so a crash loses at most the last
//! arrangement; on leaving it writes the result once more and then one receipt. A recall, or the
//! session directory disappearing, ends the visit with nothing more written. Desktop never depends
//! on any of it: whatever goes wrong, the household goes back to the desktop as it was.

use crate::household::Household;
use anyhow::{Context, Result, bail};
use formiga_home_contract::{
    ACK_FILE, AckRefusal, HOME_FORMAT_VERSION, HomeAck, HomeCapability, HomeEffect, HomeError,
    HomeMoment, HomeReceipt, HomeResult, HomeSnapshot, HomeState, RECALL_FILE, RECEIPT_FILE,
    RESULT_FILE, SNAPSHOT_FILE, STATE_FILE, SessionId, SessionSeal, Together, TravelerId, decode,
    limits, read_bounded, sha256_hex, write_document,
};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub const HOME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the household did at home that Desktop may take in, if it offers to: how often each
/// pair spent time together, and how; the moments worth a line in its journal, most worth it
/// first; and the house the owner went over to, if they went next door.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lived {
    pub together: Vec<(TravelerId, TravelerId, Together, u8)>,
    pub moments: Vec<HomeMoment>,
    pub next_door: Option<TravelerId>,
    /// A friend asked to come and live here, and the house it would move into.
    pub move_in: Option<(TravelerId, TravelerId)>,
}

impl Lived {
    /// As effects, after `first`, for a Desktop that sent `snapshot`: each only if it offers to
    /// take it in, and all within the contract's bounds. Where there are more than a receipt
    /// holds, time together gives way to the rest.
    fn effects(&self, snapshot: &HomeSnapshot, first: Vec<HomeEffect>) -> Vec<HomeEffect> {
        let offers = |capability| snapshot.offers(capability);
        let mut effects = first;
        if offers(HomeCapability::NextDoor)
            && let Some(household) = self.next_door
            && household != snapshot.household.keeper
            && snapshot.neighbour(household).is_some()
        {
            effects.push(HomeEffect::NextDoor { household });
        }
        if offers(HomeCapability::Roommates)
            && let Some((resident, household)) = self.move_in
            && household == snapshot.household.keeper
            && snapshot.visitor(resident).is_some()
        {
            effects.push(HomeEffect::MoveIn {
                resident,
                household,
            });
        }
        if offers(HomeCapability::JournalMoments) {
            effects.extend(self.moments.iter().take(limits::MAX_MOMENTS).map(|moment| {
                HomeEffect::Moment {
                    moment: moment.clone(),
                }
            }));
        }
        if offers(HomeCapability::BondNudges) {
            let mut counted = std::collections::BTreeSet::new();
            for &(a, b, together, times) in &self.together {
                let pair = (a.min(b), a.max(b), together);
                if a == b || times == 0 || together == Together::Unknown || !counted.insert(pair) {
                    continue;
                }
                if effects.len() >= limits::MAX_EFFECTS {
                    break;
                }
                effects.push(HomeEffect::Together {
                    a: pair.0,
                    b: pair.1,
                    together,
                    times: times.min(limits::MAX_TOGETHER),
                });
            }
        }
        effects
    }
}

/// A visit from Desktop in progress.
pub struct Visit {
    dir: PathBuf,
    seal: SessionSeal,
    arrived_at_utc: OffsetDateTime,
    records_visits: bool,
    pub snapshot: HomeSnapshot,
    /// The homes Desktop sent, as they stand for this colony.
    pub state: HomeState,
    left: bool,
}

/// Reads what Desktop left in `dir` and answers it: accepted, with the household ready for the
/// room, or refused, with the reason written for Desktop and returned as an error. `busy` when
/// another Home window is open, which refuses the visit whatever it holds.
pub fn arrive(dir: &Path, busy: bool) -> Result<(Visit, Household)> {
    // The visit is named by its directory, and nothing else about the path is trusted.
    let session = dir
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(SessionId::parse)
        .with_context(|| format!("{} is not a visit's directory", dir.display()))?;
    let snapshot_bytes = read_bounded(&dir.join(SNAPSHOT_FILE), limits::MAX_SNAPSHOT_BYTES)
        .context("could not read the household's snapshot")?;
    let state_bytes = read_bounded(&dir.join(STATE_FILE), limits::MAX_STATE_BYTES)
        .context("could not read the homes Desktop keeps")?;
    let arrived_at_utc = OffsetDateTime::now_utc();
    let seal = |created_at_utc| SessionSeal {
        session_id: session.clone(),
        snapshot_sha256: sha256_hex(&snapshot_bytes),
        state_sha256: sha256_hex(&state_bytes),
        created_at_utc,
    };
    let refuse = |refusal: AckRefusal, why: String| -> Result<(Visit, Household)> {
        write_document(
            &dir.join(ACK_FILE),
            &HomeAck::refused(&seal(arrived_at_utc), HOME_VERSION, refusal),
        )?;
        bail!("Formiga Home could not open the house: {why}")
    };
    let refusal_for = |error: &HomeError| match error {
        HomeError::UnsupportedVersion { .. } => AckRefusal::UnsupportedVersion {
            reads: HOME_FORMAT_VERSION,
        },
        _ => AckRefusal::Invalid,
    };

    if busy {
        return refuse(AckRefusal::Busy, "another Home window is open".into());
    }
    let snapshot = match decode::<HomeSnapshot>(&snapshot_bytes) {
        Ok(snapshot) if snapshot.session_id == session => snapshot,
        Ok(_) => {
            return refuse(
                AckRefusal::Invalid,
                "the snapshot is for another visit".into(),
            );
        }
        Err(error) => return refuse(refusal_for(&error), error.to_string()),
    };
    if !snapshot.residents_readable() {
        return refuse(
            AckRefusal::UnsupportedVersion {
                reads: HOME_FORMAT_VERSION,
            },
            "the residents are drawn in a way this build cannot draw".into(),
        );
    }
    let state = match decode::<HomeState>(&state_bytes) {
        Ok(state) => state.settled_for(&snapshot),
        Err(error) => return refuse(refusal_for(&error), error.to_string()),
    };
    let household = match Household::new(snapshot.clone()) {
        Ok(household) => household,
        Err(error) => return refuse(AckRefusal::Invalid, error.to_string()),
    };
    let seal = seal(snapshot.created_at_utc);
    write_document(&dir.join(ACK_FILE), &HomeAck::accepted(&seal, HOME_VERSION))
        .context("could not answer Desktop")?;
    Ok((
        Visit {
            dir: dir.to_owned(),
            records_visits: snapshot.offers(HomeCapability::VisitRecord),
            seal,
            arrived_at_utc,
            snapshot,
            state,
            left: false,
        },
        household,
    ))
}

impl Visit {
    /// Whether Desktop has ended the visit: it left a recall, or the visit's files are gone.
    pub fn recalled(&self) -> bool {
        self.dir.join(RECALL_FILE).exists() || !self.dir.join(SNAPSHOT_FILE).exists()
    }

    /// Hand Desktop every home as it now stands, whole, to keep if the visit ends without
    /// another word. Nothing, once Desktop has ended it.
    pub fn keep(&self, state: &HomeState) -> Result<()> {
        if self.recalled() || self.left {
            return Ok(());
        }
        let result = HomeResult::new(
            &self.seal,
            OffsetDateTime::now_utc(),
            HOME_VERSION,
            state.clone(),
        );
        write_document(&self.dir.join(RESULT_FILE), &result)
            .context("could not hand back the homes")?;
        Ok(())
    }

    /// The owner is leaving: the homes once more, then the receipt, with the visit noted if
    /// Desktop records visits and what was lived there if it takes that in. Only once.
    pub fn leave(&mut self, state: &HomeState, lived: &Lived) -> Result<()> {
        if self.left || self.recalled() {
            self.left = true;
            return Ok(());
        }
        self.keep(state)?;
        self.left = true;
        let now = OffsetDateTime::now_utc();
        let visited = if self.records_visits {
            vec![HomeEffect::HomeVisit {
                household: self.snapshot.household.keeper,
                arrived_at_utc: self
                    .arrived_at_utc
                    .replace_nanosecond(0)
                    .unwrap_or(self.arrived_at_utc),
                left_at_utc: now.replace_nanosecond(0).unwrap_or(now),
            }]
        } else {
            Vec::new()
        };
        let effects = lived.effects(&self.snapshot, visited);
        let receipt = HomeReceipt::new(&self.seal, now, HOME_VERSION, effects);
        write_document(&self.dir.join(RECEIPT_FILE), &receipt)
            .context("could not write the receipt")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::{
        HomeDocument, HomeRecall, RecallReason, accept_result, encode, read_document, sample,
    };

    /// A visit directory as Desktop would write it, under a scratch folder of its own.
    fn visit_dir(name: &str, snapshot: &HomeSnapshot, state: &HomeState) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("formiga-home-visit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(snapshot.session_id.as_str());
        std::fs::create_dir_all(&dir).unwrap();
        write_document(&dir.join(SNAPSHOT_FILE), snapshot).unwrap();
        write_document(&dir.join(STATE_FILE), state).unwrap();
        dir
    }

    fn ack(dir: &Path) -> HomeAck {
        read_document::<HomeAck>(&dir.join(ACK_FILE)).unwrap().0
    }

    #[test]
    fn a_visit_is_answered_kept_and_left_and_desktop_keeps_what_it_may() {
        let snapshot = sample::snapshot();
        let state = sample::state();
        let dir = visit_dir("whole", &snapshot, &state);
        let (mut visit, household) = arrive(&dir, false).unwrap();
        assert!(ack(&dir).accepted);
        assert_eq!(household.residents.len(), 2);
        let mut arranged = visit.state.clone();
        arranged.set_household(crate::show_house::lived_in(
            &household,
            "floor.checks",
            "wall.stripes",
        ));
        visit.keep(&arranged).unwrap();
        visit.leave(&arranged, &Lived::default()).unwrap();
        let seal = SessionSeal::of(
            &snapshot,
            &encode(&snapshot).unwrap(),
            &encode(&state).unwrap(),
        );
        let result = read_document::<HomeResult>(&dir.join(RESULT_FILE))
            .unwrap()
            .0;
        let accepted = accept_result(&seal, &snapshot, &state, &result);
        assert!(accepted.set_aside.is_empty(), "{:?}", accepted.set_aside);
        assert_eq!(
            accepted.state.household(snapshot.household.keeper),
            arranged.household(snapshot.household.keeper)
        );
        let receipt = read_document::<HomeReceipt>(&dir.join(RECEIPT_FILE))
            .unwrap()
            .0;
        assert!(receipt.answers(&seal));
        assert!(matches!(
            receipt.effects.as_slice(),
            [HomeEffect::HomeVisit { household, .. }] if *household == snapshot.household.keeper
        ));
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    /// Desktop's side of closing a visit, as the contract has it: keep only what checks out.
    fn desktop_closes(dir: &Path, snapshot: &HomeSnapshot, sent: &HomeState) -> HomeState {
        let seal = SessionSeal::of(snapshot, &encode(snapshot).unwrap(), &encode(sent).unwrap());
        match read_document::<HomeResult>(&dir.join(RESULT_FILE)) {
            Ok((result, _)) => accept_result(&seal, snapshot, sent, &result).state,
            Err(_) => sent.clone(),
        }
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn a_find_put_on_a_shelf_is_still_there_when_the_house_is_opened_again() {
        use crate::arrange::{Arranging, Carry, Landing};
        let snapshot = sample::snapshot();
        let sent = sample::state();
        let dir = visit_dir("again", &snapshot, &sent);
        let (mut visit, _) = arrive(&dir, false).unwrap();
        let mut state = visit.state.clone();
        crate::arrange::ensure_home(&mut state, &snapshot);
        let keeper = snapshot.household.keeper;
        let shelf = state.household(keeper).unwrap().rooms[0]
            .pieces
            .iter()
            .find(|placed| placed.piece.as_str() == "shelf")
            .unwrap()
            .uid;
        let mut arranging = Arranging::default();
        arranging.carrying = Some(Carry::Thing(formiga_home_contract::DisplayId::find(3)));
        let landing = Landing::Spot {
            room: 0,
            spot: formiga_home_contract::Spot::On {
                piece: shelf,
                slot: 0,
            },
        };
        let house = crate::house::House::of(&state.household(keeper).unwrap().rooms);
        assert!(arranging.put(&mut state, keeper, &snapshot, &house, landing));
        visit.leave(&state, &Lived::default()).unwrap();
        let kept = desktop_closes(&dir, &snapshot, &sent);
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());

        // The next visit, with the homes Desktop kept.
        let mut next = snapshot.clone();
        next.session_id = SessionId::parse("1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e").unwrap();
        let dir = visit_dir("again-2", &next, &kept);
        let (visit, _) = arrive(&dir, false).unwrap();
        assert_eq!(visit.state.household(keeper), state.household(keeper));
        assert_eq!(
            visit
                .state
                .shown_by(&formiga_home_contract::DisplayId::find(3)),
            Some(keeper)
        );
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_home_that_crashes_after_arranging_loses_nothing_it_had_handed_back() {
        let snapshot = sample::snapshot();
        let sent = sample::state();
        let dir = visit_dir("crash", &snapshot, &sent);
        let (visit, household) = arrive(&dir, false).unwrap();
        let mut arranged = visit.state.clone();
        arranged.set_household(crate::show_house::lived_in(
            &household,
            "floor.straw",
            "wall.timber",
        ));
        visit.keep(&arranged).unwrap();
        // The window dies here: no second result, no receipt.
        drop(visit);
        assert!(!dir.join(RECEIPT_FILE).exists());
        let kept = desktop_closes(&dir, &snapshot, &sent);
        assert_eq!(
            kept.household(snapshot.household.keeper),
            arranged.household(snapshot.household.keeper)
        );
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_home_that_crashes_before_answering_leaves_desktop_as_it_was() {
        let snapshot = sample::snapshot();
        let sent = sample::state();
        let dir = visit_dir("early", &snapshot, &sent);
        // Started, and gone before it read anything.
        assert_eq!(desktop_closes(&dir, &snapshot, &sent), sent);
        // A result that is half written is no result at all.
        std::fs::write(
            dir.join(RESULT_FILE),
            b"{\"format\": \"formiga.home.result\", \"vers",
        )
        .unwrap();
        assert_eq!(desktop_closes(&dir, &snapshot, &sent), sent);
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn what_was_lived_goes_to_desktop_only_as_far_as_it_offers_and_within_bounds() {
        let snapshot = sample::snapshot();
        let (keeper, friend) = (snapshot.residents[0].id, snapshot.visitors[0].id);
        let lived = Lived {
            together: vec![
                (friend, keeper, Together::Play, 9),
                (keeper, friend, Together::Play, 1),
                (keeper, keeper, Together::Cozy, 2),
                (keeper, friend, Together::Care, 1),
            ],
            moments: vec![HomeMoment::Visit { visitor: friend }; 5],
            next_door: Some(friend),
            move_in: Some((friend, keeper)),
        };
        let effects_for = |capabilities: Vec<HomeCapability>, name: &str| {
            let mut offered = snapshot.clone();
            offered.capabilities = capabilities;
            let dir = visit_dir(name, &offered, &sample::state());
            let (mut visit, _) = arrive(&dir, false).unwrap();
            visit.leave(&visit.state.clone(), &lived).unwrap();
            let receipt = read_document::<HomeReceipt>(&dir.join(RECEIPT_FILE))
                .unwrap()
                .0;
            let _ = std::fs::remove_dir_all(dir.parent().unwrap());
            receipt.effects
        };
        let all = effects_for(
            vec![
                HomeCapability::VisitRecord,
                HomeCapability::BondNudges,
                HomeCapability::JournalMoments,
                HomeCapability::NextDoor,
                HomeCapability::Roommates,
            ],
            "lived-all",
        );
        let kinds: Vec<&str> = all.iter().map(HomeEffect::kind).collect();
        assert_eq!(
            kinds,
            [
                "home_visit",
                "next_door",
                "move_in",
                "moment",
                "moment",
                "moment",
                "together",
                "together"
            ]
        );
        assert!(all.contains(&HomeEffect::Together {
            a: keeper.min(friend),
            b: keeper.max(friend),
            together: Together::Play,
            times: limits::MAX_TOGETHER,
        }));
        let visits_only = effects_for(vec![HomeCapability::VisitRecord], "lived-visits");
        assert!(matches!(
            visits_only.as_slice(),
            [HomeEffect::HomeVisit { .. }]
        ));
    }

    #[test]
    fn a_busy_home_refuses_and_says_so() {
        let snapshot = sample::snapshot();
        let dir = visit_dir("busy", &snapshot, &sample::state());
        assert!(arrive(&dir, true).is_err());
        assert_eq!(ack(&dir).refusal, Some(AckRefusal::Busy));
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_damaged_or_newer_snapshot_is_refused_with_the_reason() {
        let snapshot = sample::snapshot();
        let dir = visit_dir("damaged", &snapshot, &sample::state());
        std::fs::write(
            dir.join(SNAPSHOT_FILE),
            b"{\"format\": \"formiga.home.snapshot\"",
        )
        .unwrap();
        assert!(arrive(&dir, false).is_err());
        assert_eq!(ack(&dir).refusal, Some(AckRefusal::Invalid));
        let mut newer = serde_json::to_value(&snapshot).unwrap();
        newer["version"] = (HOME_FORMAT_VERSION + 1).into();
        newer["min_reader_version"] = (HOME_FORMAT_VERSION + 1).into();
        std::fs::write(dir.join(SNAPSHOT_FILE), serde_json::to_vec(&newer).unwrap()).unwrap();
        assert!(arrive(&dir, false).is_err());
        assert_eq!(
            ack(&dir).refusal,
            Some(AckRefusal::UnsupportedVersion {
                reads: HOME_FORMAT_VERSION
            })
        );
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_snapshot_in_another_visits_directory_is_refused() {
        let snapshot = sample::snapshot();
        let dir = visit_dir("misplaced", &snapshot, &sample::state());
        let elsewhere = dir.with_file_name("ffffffffffffffffffffffffffffffff");
        std::fs::rename(&dir, &elsewhere).unwrap();
        assert!(arrive(&elsewhere, false).is_err());
        assert_eq!(ack(&elsewhere).refusal, Some(AckRefusal::Invalid));
        assert!(
            arrive(&elsewhere.join("nested"), false).is_err(),
            "not a visit's directory"
        );
        let _ = std::fs::remove_dir_all(elsewhere.parent().unwrap());
    }

    #[test]
    fn after_a_recall_nothing_more_is_written() {
        let snapshot = sample::snapshot();
        let dir = visit_dir("recalled", &snapshot, &sample::state());
        let (mut visit, _) = arrive(&dir, false).unwrap();
        let recall = HomeRecall::new(
            snapshot.session_id,
            OffsetDateTime::now_utc(),
            RecallReason::OwnerAsked,
        );
        recall.validate().unwrap();
        write_document(&dir.join(RECALL_FILE), &recall).unwrap();
        assert!(visit.recalled());
        visit
            .leave(&visit.state.clone(), &Lived::default())
            .unwrap();
        assert!(!dir.join(RESULT_FILE).exists() && !dir.join(RECEIPT_FILE).exists());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
