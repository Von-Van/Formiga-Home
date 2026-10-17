//! How a resident's body gets about the house. A companion, and most forms sculpted in Formiga
//! Farm, can get up onto anything in the house and do whatever it is for. A few body plans cannot:
//! a turtle's short legs do not reach an armchair, a whale has no legs to bounce on a bed or
//! stretch out on a rug, and a giraffe or an elephant does not fit in a little one's basket. The
//! household's life leaves those to the others, so nobody is shown doing what its body could not.

use crate::catalog::{self, Piece};
use crate::house::House;
use formiga_core::AppearanceGenome;
use formiga_core::forms::Plan;

/// How high a crawler gets up onto a piece, in pixels: a bedroll or a basket, not a bed or a
/// chair.
const CRAWLER_REACH: i32 = 3;

/// The body a resident is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// A companion, drawn from its recipe or its genes, or as the Echo Fox.
    Companion,
    /// A form sculpted in Formiga Farm on one of the newer plans.
    Sculpted(Plan),
}

impl Body {
    pub fn of(genome: &AppearanceGenome) -> Self {
        match &genome.sculpt {
            // The Echo Fox is drawn over any form, and gets about as a fox.
            Some(sculpt) if !genome.echo_fox => Self::Sculpted(sculpt.plan),
            _ => Self::Companion,
        }
    }

    /// Whether it gets up onto a piece whose seat is `lift` pixels off the floor.
    pub fn reaches(self, lift: i32) -> bool {
        self != Self::Sculpted(Plan::Crawler) || lift <= CRAWLER_REACH
    }

    /// Whether it has legs or wings to bounce on a bed with.
    pub fn bounces(self) -> bool {
        !matches!(self, Self::Sculpted(Plan::Floater | Plan::Crawler))
    }

    /// Whether it can stretch out on a rug.
    pub fn sprawls(self) -> bool {
        self != Self::Sculpted(Plan::Floater)
    }

    /// Whether it rises onto a piece and sinks off it over a whole stretch of floor, as a floater
    /// drifts, rather than stepping up at the piece's edge.
    pub fn floats(self) -> bool {
        self == Self::Sculpted(Plan::Floater)
    }

    /// Whether it can sit or lie on `piece`: one it can get up onto, and that it fits.
    pub fn settles_on(self, piece: &Piece) -> bool {
        let too_big = matches!(
            self,
            Self::Sculpted(Plan::LargeQuadruped | Plan::TallQuadruped)
        );
        self.reaches(piece.lift) && !(too_big && piece.id == "basket")
    }

    /// Whether it can sit or lie on the piece placed as `uid`. A piece that is not there, or not
    /// in this catalogue, is nothing to settle on.
    pub fn settles_on_placed(self, house: &House, uid: u16) -> bool {
        house
            .piece(uid)
            .and_then(|placed| catalog::piece(&placed.piece))
            .is_some_and(|piece| self.settles_on(piece))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_core::forms::{Design, Form, Sculpt};

    fn sculpted(plan: Plan) -> AppearanceGenome {
        let base = formiga_travel::sample::colony(3).creatures[0]
            .appearance
            .clone();
        Design {
            form: Form::Sculpted {
                sculpt: Sculpt::starter(plan),
            },
            face: base.face,
        }
        .genome(&base)
    }

    #[test]
    fn a_companion_and_the_echo_fox_get_up_onto_everything() {
        let companion = formiga_travel::sample::colony(3).creatures[0]
            .appearance
            .clone();
        assert_eq!(Body::of(&companion), Body::Companion);
        let mut fox = sculpted(Plan::Crawler);
        fox.echo_fox = true;
        assert_eq!(Body::of(&fox), Body::Companion);
        for piece in catalog::PIECES.iter() {
            assert!(Body::Companion.settles_on(piece), "{}", piece.id);
        }
        assert!(Body::Companion.bounces() && Body::Companion.sprawls());
        assert!(!Body::Companion.floats());
    }

    #[test]
    fn each_plan_keeps_to_what_its_body_can_do() {
        let piece = |id: &str| {
            catalog::PIECES
                .iter()
                .find(|piece| piece.id == id)
                .unwrap_or_else(|| panic!("{id} is in the catalogue"))
        };
        let body = |plan| Body::of(&sculpted(plan));
        let turtle = body(Plan::Crawler);
        assert!(turtle.settles_on(piece("bedroll")) && turtle.settles_on(piece("basket")));
        assert!(!turtle.settles_on(piece("armchair")) && !turtle.settles_on(piece("bed")));
        assert!(!turtle.bounces());
        let whale = body(Plan::Floater);
        assert!(whale.settles_on(piece("armchair")) && whale.floats());
        assert!(!whale.bounces() && !whale.sprawls());
        for plan in [Plan::LargeQuadruped, Plan::TallQuadruped] {
            assert!(!body(plan).settles_on(piece("basket")), "{plan:?}");
            assert!(body(plan).settles_on(piece("bed")), "{plan:?}");
        }
        for plan in [Plan::CompactQuadruped, Plan::Upright, Plan::Percher] {
            assert!(body(plan).settles_on(piece("basket")), "{plan:?}");
            assert!(body(plan).bounces() && body(plan).sprawls(), "{plan:?}");
        }
    }
}
