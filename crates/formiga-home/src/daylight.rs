//! The time of day and the time of year at home, as the owner's own clock and calendar have them.
//! By day the house is as it is drawn; as the light goes it warms, and at night it is blue, but
//! for the pools of its lamps. The garden through the front door turns with the year. Nothing
//! runs while Home is closed: this is only ever now.

use time::{Month, OffsetDateTime};

/// The hour and the month, at home.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Daylight {
    /// Hours since midnight, on the owner's clock.
    pub hour: f32,
    pub month: Month,
}

/// The time of year, as the garden shows it. The northern year: Desktop has no seasons of its
/// own to follow, and the owner's clock says nothing of where on the earth it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Default for Daylight {
    /// Midday in summer: the house exactly as it is drawn.
    fn default() -> Self {
        Self {
            hour: 12.0,
            month: Month::June,
        }
    }
}

/// How dark it is at an hour, and what colour the dark is: the multiplier each channel goes to
/// where no lamp reaches. Between these the light changes smoothly.
const LIGHT: [(f32, f32, [f32; 3]); 9] = [
    (0.0, 0.55, NIGHT),
    (4.5, 0.55, NIGHT),
    (6.0, 0.28, [0.92, 0.76, 0.8]),
    (7.5, 0.0, [1.0, 1.0, 1.0]),
    (17.0, 0.0, [1.0, 1.0, 1.0]),
    (18.5, 0.2, [0.98, 0.78, 0.58]),
    (20.0, 0.38, [0.78, 0.66, 0.68]),
    (21.5, 0.55, NIGHT),
    (24.0, 0.55, NIGHT),
];

const NIGHT: [f32; 3] = [0.4, 0.47, 0.74];

impl Daylight {
    /// Now, on the owner's clock.
    pub fn now() -> Self {
        let at = crate::journal::local(OffsetDateTime::now_utc());
        Self {
            hour: f32::from(at.hour()) + f32::from(at.minute()) / 60.0,
            month: at.month(),
        }
    }

    /// How much of the light has gone, from nothing by day to all of it at night, and the colour
    /// it leaves behind.
    pub fn dark(&self) -> (f32, [f32; 3]) {
        let hour = self.hour.rem_euclid(24.0);
        let after = LIGHT
            .iter()
            .position(|(at, ..)| *at > hour)
            .unwrap_or(LIGHT.len() - 1);
        let (from, to) = (LIGHT[after.saturating_sub(1)], LIGHT[after]);
        let along = if to.0 > from.0 {
            ((hour - from.0) / (to.0 - from.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let lerp = |a: f32, b: f32| a + (b - a) * along;
        (
            lerp(from.1, to.1),
            [
                lerp(from.2[0], to.2[0]),
                lerp(from.2[1], to.2[1]),
                lerp(from.2[2], to.2[2]),
            ],
        )
    }

    /// Dark enough that a lamp's light shows on the floor.
    pub fn lamplight(&self) -> bool {
        self.dark().0 > 0.1
    }

    pub fn season(&self) -> Season {
        match self.month {
            Month::March | Month::April | Month::May => Season::Spring,
            Month::June | Month::July | Month::August => Season::Summer,
            Month::September | Month::October | Month::November => Season::Autumn,
            Month::December | Month::January | Month::February => Season::Winter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_is_as_drawn_dusk_is_warm_and_night_is_blue_and_dark() {
        let at = |hour| Daylight {
            hour,
            month: Month::June,
        };
        assert_eq!(at(12.0).dark().0, 0.0);
        assert!(!at(12.0).lamplight());
        let (dusk, tint) = at(19.0).dark();
        assert!(dusk > 0.1 && dusk < 0.45);
        assert!(tint[0] > tint[2], "dusk is warm");
        let (night, tint) = at(23.5).dark();
        assert!(night >= 0.5 && at(2.0).dark().0 >= 0.5);
        assert!(tint[2] > tint[0], "night is blue");
        // No jump at midnight.
        assert!((at(23.99).dark().0 - at(0.0).dark().0).abs() < 0.01);
    }

    #[test]
    fn the_garden_turns_with_the_year() {
        let in_month = |month| Daylight { hour: 12.0, month }.season();
        assert_eq!(in_month(Month::April), Season::Spring);
        assert_eq!(in_month(Month::October), Season::Autumn);
        assert_eq!(in_month(Month::January), Season::Winter);
    }
}
