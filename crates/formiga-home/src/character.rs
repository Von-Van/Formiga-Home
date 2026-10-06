//! Who a resident is, as far as Home asks: read from the snapshot's temperament, axes, pace,
//! habits and size, never written for a particular creature, so every household lives differently
//! and the same creature carries itself here as it does on the desktop.
//!
//! What a resident feels like doing is a handful of drives — rest, play, company, curiosity and
//! comfort — that rise while it is busy with something else and settle when it does what they ask.
//! They are reasons, not needs: nothing is shown as a bar, nothing runs down while Home is closed,
//! and nothing goes wrong if they are never met.

use formiga_art::ExpressionKind;
use formiga_core::{Axes, Habit, TemperamentKind};
use formiga_travel::{TravelRole, Traveler};

/// One of the things a resident can feel like.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Drive {
    Rest,
    Play,
    Company,
    Curiosity,
    Comfort,
}

impl Drive {
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Character {
    pub kind: TemperamentKind,
    pub axes: Axes,
    /// How lively it moves, from 0 to 1.
    pub pace: f32,
    pub habits: Vec<Habit>,
    pub little: bool,
}

impl Character {
    pub fn of(traveler: &Traveler) -> Self {
        Self {
            kind: traveler.character.temperament.into(),
            axes: traveler.character.axes.into(),
            pace: traveler.motion.activity.clamp(0.0, 1.0),
            habits: traveler
                .habits
                .iter()
                .map(|habit| Habit::from(*habit))
                .collect(),
            little: matches!(traveler.role, TravelRole::Mini { .. }),
        }
    }

    pub fn has_habit(&self, habit: Habit) -> bool {
        self.habits.contains(&habit)
    }

    /// How fast it walks, in tiles a second: its own pace, a little shorter in the stride for a
    /// little one.
    pub fn walk_speed(&self) -> f32 {
        let speed = 0.85 + self.pace * 0.9;
        if self.little { speed * 0.85 } else { speed }
    }

    /// How quickly each drive rises, per minute of doing something else.
    pub fn drive_rates(&self) -> [f32; 5] {
        let a = &self.axes;
        let mut rates = [
            0.25 + (1.0 - a.energy) * 0.5,
            0.15 + a.playfulness * 0.6,
            0.1 + a.social * 0.6 + a.affection * 0.2,
            0.12 + a.curiosity * 0.6,
            0.12 + (1.0 - a.boldness) * 0.3 + a.affection * 0.1,
        ];
        let boost = |rates: &mut [f32; 5], drive: Drive, by: f32| rates[drive.index()] *= by;
        match self.kind {
            TemperamentKind::Lazybones => boost(&mut rates, Drive::Rest, 1.8),
            TemperamentKind::Explorer => boost(&mut rates, Drive::Curiosity, 1.6),
            TemperamentKind::Scholar => boost(&mut rates, Drive::Curiosity, 1.5),
            TemperamentKind::Sweetheart => boost(&mut rates, Drive::Company, 1.5),
            TemperamentKind::Showoff => boost(&mut rates, Drive::Company, 1.3),
            TemperamentKind::Troublemaker => boost(&mut rates, Drive::Play, 1.4),
            TemperamentKind::Grump => {
                boost(&mut rates, Drive::Company, 0.5);
                boost(&mut rates, Drive::Comfort, 1.3);
            }
            TemperamentKind::Wallflower => {
                boost(&mut rates, Drive::Company, 0.6);
                boost(&mut rates, Drive::Comfort, 1.3);
            }
            TemperamentKind::Guardian => boost(&mut rates, Drive::Company, 1.2),
            TemperamentKind::Oddball => boost(&mut rates, Drive::Curiosity, 1.3),
        }
        if self.little {
            boost(&mut rates, Drive::Play, 1.3);
            boost(&mut rates, Drive::Rest, 1.2);
        }
        rates
    }

    /// How far its own whims stray from what its drives say: an oddball's a long way.
    pub fn whimsy(&self) -> f32 {
        let base = 0.15 + self.axes.impulsiveness * 0.3;
        if self.kind == TemperamentKind::Oddball {
            base * 2.0
        } else {
            base
        }
    }

    /// How soon a room palls and another calls, from 0 (a homebody, happy where it is) to 1:
    /// the curious, the lively and the bold get about the house, the idle and the shy less so.
    pub fn roams(&self) -> f32 {
        let a = &self.axes;
        let base = 0.15 + a.curiosity * 0.35 + a.energy * 0.3 + a.boldness * 0.2;
        let kind = match self.kind {
            TemperamentKind::Explorer => 1.4,
            TemperamentKind::Oddball | TemperamentKind::Troublemaker => 1.15,
            TemperamentKind::Lazybones => 0.6,
            TemperamentKind::Wallflower | TemperamentKind::Grump => 0.8,
            _ => 1.0,
        };
        (base * kind).clamp(0.0, 1.0)
    }

    /// Would rather sit apart from the others than among them.
    pub fn keeps_apart(&self) -> bool {
        matches!(
            self.kind,
            TemperamentKind::Wallflower | TemperamentKind::Grump
        ) || self.axes.social < 0.3
    }

    /// Picks on others for fun.
    pub fn teases(&self) -> bool {
        self.kind == TemperamentKind::Troublemaker
            || (self.axes.feistiness > 0.6 && self.axes.impulsiveness > 0.5)
    }

    /// Shows off what it has: anyone watching will do.
    pub fn shows_off(&self) -> bool {
        self.kind == TemperamentKind::Showoff || self.axes.boldness > 0.75
    }

    /// Likes a good look at the rare and curious.
    pub fn studies(&self) -> bool {
        matches!(
            self.kind,
            TemperamentKind::Scholar | TemperamentKind::Explorer
        ) || self.axes.curiosity > 0.7
    }

    /// Fusses with things rather than leaving them be.
    pub fn fusses(&self) -> bool {
        matches!(
            self.kind,
            TemperamentKind::Troublemaker | TemperamentKind::Oddball
        ) || self.axes.impulsiveness > 0.7
    }

    /// Affectionate enough to hug, given a bond to match.
    pub fn hugs(&self) -> bool {
        self.axes.affection > 0.45 || self.kind == TemperamentKind::Sweetheart
    }

    pub fn playful(&self) -> bool {
        self.axes.playfulness > 0.35 || self.little
    }

    /// The face it wears doing nothing in particular.
    pub fn idle_face(&self) -> ExpressionKind {
        match self.kind {
            TemperamentKind::Grump => ExpressionKind::Grumpy,
            TemperamentKind::Showoff | TemperamentKind::Troublemaker => ExpressionKind::Smug,
            TemperamentKind::Scholar => ExpressionKind::Focused,
            TemperamentKind::Explorer | TemperamentKind::Oddball => ExpressionKind::Curious,
            TemperamentKind::Lazybones => ExpressionKind::Sleepy,
            TemperamentKind::Guardian => ExpressionKind::Determined,
            TemperamentKind::Wallflower => ExpressionKind::Neutral,
            TemperamentKind::Sweetheart => ExpressionKind::Content,
        }
    }

    /// The face it walks with.
    pub fn walk_face(&self) -> ExpressionKind {
        if self.axes.playfulness > 0.65 {
            ExpressionKind::Joy
        } else if self.axes.curiosity > 0.6 {
            ExpressionKind::Curious
        } else {
            ExpressionKind::Content
        }
    }

    /// The face it settles into once it is comfortable.
    pub fn settled_face(&self) -> ExpressionKind {
        match self.kind {
            TemperamentKind::Grump => ExpressionKind::Smug,
            TemperamentKind::Lazybones => ExpressionKind::Sleepy,
            _ => ExpressionKind::Content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::sample;

    #[test]
    fn every_drive_rises_and_a_little_one_walks_a_shorter_stride() {
        let snapshot = sample::snapshot();
        let keeper = Character::of(&snapshot.residents[0]);
        let little = Character::of(&snapshot.residents[1]);
        assert!(little.little && !keeper.little);
        for rate in keeper.drive_rates().into_iter().chain(little.drive_rates()) {
            assert!(rate > 0.0 && rate.is_finite());
        }
        let mut same_pace = little.clone();
        same_pace.little = false;
        assert!(little.walk_speed() < same_pace.walk_speed());
    }

    #[test]
    fn a_lazybones_tires_sooner_than_an_explorer_and_wonders_less() {
        let snapshot = sample::snapshot();
        let mut lazy = Character::of(&snapshot.residents[0]);
        lazy.kind = TemperamentKind::Lazybones;
        let mut explorer = lazy.clone();
        explorer.kind = TemperamentKind::Explorer;
        let (lazy, explorer) = (lazy.drive_rates(), explorer.drive_rates());
        assert!(lazy[Drive::Rest.index()] > explorer[Drive::Rest.index()]);
        assert!(lazy[Drive::Curiosity.index()] < explorer[Drive::Curiosity.index()]);
    }
}
