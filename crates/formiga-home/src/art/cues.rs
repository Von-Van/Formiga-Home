//! The signs over a resident's head that say what it is about: a heart, a note, a Z, a question.
//! Each is said as one of Desktop's bubbles (see [`Cue::saying`]), which the window paints over
//! the room.

use formiga_core::talk::Saying;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    Heart,
    Note,
    Sleep,
    Question,
    Bang,
    Sparkle,
    /// Thinking of something: a little cloud.
    Thought,
    Grumble,
}

impl Cue {
    /// How the sign is said as one of Desktop's bubbles: a heart tenderly, a note in song, a Z
    /// as a thought, a grumble in a dark little cloud.
    pub fn saying(self) -> Saying {
        use formiga_core::talk::{TalkStyle as S, TalkTopic as T};
        match self {
            Cue::Heart => Saying::one(S::Tender, T::Heart),
            Cue::Note => Saying::one(S::Song, T::Note),
            Cue::Sleep => Saying::one(S::Thought, T::Sleep),
            Cue::Question => Saying::one(S::Thought, T::Question),
            Cue::Bang => Saying::one(S::Shout, T::Exclaim),
            Cue::Sparkle => Saying::one(S::Excited, T::Sparkle),
            Cue::Thought => Saying::one(S::Thought, T::Dots),
            Cue::Grumble => Saying::one(S::Grumble, T::Storm),
        }
    }
}

/// How a cue `seconds` into its pose is moving: how far it bobs, and for a sleeper's, how far up
/// its second z has drifted and how clear it is. Whatever is the same, it is drawn the same.
pub fn motion(cue: Cue, seconds: f32, still: bool) -> (i32, Option<(i32, u8)>) {
    if still {
        return (0, None);
    }
    let bob = ((seconds * 3.0).sin() * 1.2).round() as i32;
    let drift = (cue == Cue::Sleep).then(|| {
        let drift = (seconds * 0.8).fract();
        ((drift * 4.0) as i32, (255.0 * (1.0 - drift)) as u8)
    });
    (bob, drift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sign_is_said_its_own_way() {
        let cues = [
            Cue::Heart,
            Cue::Note,
            Cue::Sleep,
            Cue::Question,
            Cue::Bang,
            Cue::Sparkle,
            Cue::Thought,
            Cue::Grumble,
        ];
        let sayings: std::collections::BTreeSet<String> = cues
            .iter()
            .map(|cue| format!("{:?}", cue.saying()))
            .collect();
        assert_eq!(sayings.len(), cues.len());
    }
}
