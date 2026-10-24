//! The pose each act is done in, and for how long: by whoever leads it, and by whoever joins in.

use super::{Act, Dice, is_precious};
use crate::actor::Pose;
use crate::art::cues::Cue;
use crate::household::{Household, Id, Resident};
use formiga_art::ExpressionKind;
use formiga_core::{ActionKind, Gesture, TemperamentKind};
use formiga_home_contract::{DisplayId, HomeSnapshot};
use formiga_travel::{Band, Trait};

/// How someone joins in an act `leader` leads, which it keeps up as long as the leader does.
pub(super) fn joining_pose(
    household: &Household,
    resident: &Resident,
    leader: Id,
    act: &Act,
) -> Pose {
    let character = &resident.character;
    let bond = household.bond(resident.id, leader);
    let warm = bond.warmth >= Band::Medium || household.family(resident.id, leader);
    match act {
        Act::Tease(_) if character.axes.feistiness > 0.55 => {
            Pose::idle(ExpressionKind::Grumpy).with_cue(Cue::Grumble)
        }
        Act::Tease(_) => Pose::idle(ExpressionKind::Worried).with_cue(Cue::Bang),
        Act::Hug(_) | Act::Comfort(_) => {
            Pose::idle(ExpressionKind::Affectionate).with_cue(Cue::Heart)
        }
        Act::Greet(_) if warm => Pose::new(ActionKind::Greet, ExpressionKind::Joy),
        Act::Greet(_) => Pose::idle(character.idle_face()),
        Act::PlayTogether(_) | Act::PlayWith(..) => {
            Pose::new(ActionKind::SocialPlay, ExpressionKind::Joy).with_cue(Cue::Note)
        }
        Act::ShowTo(..) if character.kind == TemperamentKind::Grump => {
            Pose::new(Gesture::Watch, ExpressionKind::Bored)
        }
        Act::ShowTo(..) => Pose::new(Gesture::Watch, ExpressionKind::Curious),
        Act::Share(..) => Pose::new(ActionKind::Eat, ExpressionKind::Content),
        Act::PutOnShowFor(..) if character.kind == TemperamentKind::Grump => {
            Pose::new(Gesture::Watch, ExpressionKind::Bored)
        }
        Act::PutOnShowFor(..) => {
            Pose::new(Gesture::Watch, ExpressionKind::Joy).with_cue(Cue::Sparkle)
        }
        Act::SitTogether(_) => Pose::new(ActionKind::Perch, character.settled_face()),
        Act::InviteLittle(..) => {
            Pose::new(ActionKind::Sleep, ExpressionKind::Sleepy).with_cue(Cue::Sleep)
        }
        _ => Pose::idle(character.idle_face()),
    }
}

/// The pose someone leading an act does it in, and for how many seconds, with `dark` how dark it
/// is at home.
pub(super) fn leading_pose(
    snapshot: &HomeSnapshot,
    resident: &Resident,
    act: &Act,
    dark: f32,
    dice: &mut Dice,
) -> (Pose, f32) {
    let character = &resident.character;
    let settled = character.settled_face();
    match act {
        Act::GoTo(..) => (Pose::idle(character.idle_face()), 1.0),
        Act::Wander(..) => (
            Pose::new(Gesture::Watch, character.walk_face()),
            dice.between(1.5, 3.5),
        ),
        Act::Sit(_) => (
            Pose::new(ActionKind::Perch, settled),
            dice.between(6.0, 10.0),
        ),
        Act::Relax(_) => (
            Pose::new(ActionKind::Perch, ExpressionKind::Content),
            dice.between(12.0, 18.0),
        ),
        Act::SitAt(_) => (
            Pose::new(ActionKind::Perch, settled),
            dice.between(8.0, 12.0),
        ),
        Act::Draw(..) => (
            Pose::new(ActionKind::Perch, ExpressionKind::Focused).with_cue(Cue::Thought),
            dice.between(10.0, 14.0),
        ),
        Act::Nap(_) | Act::CurlUp(_) => {
            let long = if character.kind == TemperamentKind::Lazybones {
                1.5
            } else {
                1.0
            };
            (
                Pose::new(ActionKind::Sleep, ExpressionKind::Sleepy).with_cue(Cue::Sleep),
                dice.between(12.0, 18.0) * long,
            )
        }
        Act::Sprawl(_) => (
            Pose::new(ActionKind::Sleep, ExpressionKind::Content).with_cue(Cue::Sleep),
            dice.between(10.0, 16.0),
        ),
        Act::Sleep(_) | Act::InviteLittle(..) => (
            Pose::new(ActionKind::Sleep, ExpressionKind::Sleepy).with_cue(Cue::Sleep),
            dice.between(20.0, 28.0) * (1.0 + dark * 3.0),
        ),
        Act::Bounce(_) => (
            Pose::new(Gesture::Cheer, ExpressionKind::Joy)
                .spinning()
                .with_cue(Cue::Note),
            4.0,
        ),
        Act::SwitchLamp(_) => (Pose::new(Gesture::Reach, ExpressionKind::Curious), 1.4),
        Act::Wind(_) => (
            Pose::new(Gesture::Reach, ExpressionKind::Focused).with_cue(Cue::Note),
            3.0,
        ),
        Act::Tend(_) => (
            Pose::new(Gesture::Reach, ExpressionKind::Content).with_cue(Cue::Heart),
            4.0,
        ),
        Act::Browse(_) => (
            Pose::new(ActionKind::InspectScreen, ExpressionKind::Curious).with_cue(Cue::Question),
            dice.between(5.0, 7.0),
        ),
        Act::Play(_) | Act::PlayWithFind(_) => (
            Pose::new(ActionKind::SoloPlay, ExpressionKind::Joy).with_cue(Cue::Note),
            dice.between(6.0, 9.0),
        ),
        Act::PlayWith(..) | Act::PlayTogether(_) => (
            Pose::new(ActionKind::SocialPlay, ExpressionKind::Joy).with_cue(Cue::Note),
            dice.between(6.0, 8.0),
        ),
        Act::ShowOffToy(_) => (
            Pose::new(Gesture::Cheer, ExpressionKind::Smug).with_cue(Cue::Sparkle),
            3.5,
        ),
        Act::Snack(_) | Act::Share(..) => (Pose::new(ActionKind::Eat, snack_face(resident)), 5.0),
        Act::Supper(_) => (
            Pose::new(ActionKind::Eat, snack_face(resident)),
            dice.between(7.0, 10.0),
        ),
        Act::WarmUp(_) => (
            Pose::new(ActionKind::Perch, ExpressionKind::Content).with_cue(Cue::Heart),
            dice.between(10.0, 14.0) * (1.0 + dark),
        ),
        Act::PutOnShow(_) | Act::PutOnShowFor(..) => {
            let face = if character.shows_off() {
                ExpressionKind::Smug
            } else {
                ExpressionKind::Joy
            };
            (
                Pose::new(Gesture::Reach, face).with_cue(Cue::Note),
                dice.between(6.0, 9.0),
            )
        }
        Act::Inspect(item) => inspecting_pose(snapshot, resident, item, dice),
        Act::Remember(_) => (
            Pose::new(Gesture::Watch, ExpressionKind::Affectionate).with_cue(Cue::Thought),
            6.0,
        ),
        Act::ShowTo(..) => (
            Pose::new(ActionKind::PresentDiscovery, ExpressionKind::Smug).with_cue(Cue::Sparkle),
            5.0,
        ),
        Act::FussWith(_) => (Pose::new(Gesture::Reach, ExpressionKind::Smug), 3.5),
        Act::TryOn(_) => {
            let face = if character.shows_off() {
                ExpressionKind::Smug
            } else {
                ExpressionKind::Joy
            };
            (
                Pose::new(Gesture::Bop, face).with_cue(Cue::Sparkle),
                dice.between(7.0, 10.0),
            )
        }
        Act::Greet(_) => (Pose::new(ActionKind::Greet, ExpressionKind::Joy), 2.5),
        Act::SitTogether(_) => (
            Pose::new(ActionKind::Perch, ExpressionKind::Content),
            dice.between(10.0, 14.0),
        ),
        Act::Tease(_) => (
            Pose::new(Gesture::Crouch, ExpressionKind::Smug).with_cue(Cue::Bang),
            3.0,
        ),
        Act::Comfort(_) => (
            Pose::idle(ExpressionKind::Affectionate).with_cue(Cue::Heart),
            4.0,
        ),
        Act::Hug(_) => (
            Pose::idle(ExpressionKind::Affectionate).with_cue(Cue::Heart),
            3.5,
        ),
        Act::GrumbleAt(_) => (
            Pose::idle(ExpressionKind::Grumpy).with_cue(Cue::Grumble),
            2.5,
        ),
    }
}

/// How a snack goes down: a joy to one who loves its food.
fn snack_face(resident: &Resident) -> ExpressionKind {
    if resident
        .traveler
        .character
        .trait_ids
        .contains(&Trait::FoodMotivated)
    {
        ExpressionKind::Joy
    } else {
        ExpressionKind::Content
    }
}

/// A good look at a find: one who studies things looks hard, and longest at something precious.
fn inspecting_pose(
    snapshot: &HomeSnapshot,
    resident: &Resident,
    item: &DisplayId,
    dice: &mut Dice,
) -> (Pose, f32) {
    let character = &resident.character;
    let face = if character.studies() {
        ExpressionKind::Focused
    } else {
        ExpressionKind::Curious
    };
    let precious = snapshot.item(item).is_some_and(is_precious);
    let long = if character.studies() && precious {
        1.5
    } else {
        1.0
    };
    (
        Pose::new(ActionKind::InspectScreen, face).with_cue(Cue::Question),
        dice.between(4.0, 6.0) * long,
    )
}
