//! The household as Home knows it: each resident ready to draw, and the facts about who it is and
//! how it gets on with the others, read once from the snapshot Desktop sent.

use crate::character::Character;
use formiga_art::AccessoryArt;
use formiga_core::{AppearanceGenome, Creature};
use formiga_home_contract::{DisplayId, HomeSnapshot, HomeState};
use formiga_travel::{Band, TravelError, TravelRole, Traveler};

/// A resident's id, as the rest of Home passes it about.
pub type Id = u64;

pub struct Resident {
    pub id: Id,
    pub name: String,
    /// Desktop's stand-in for the companion: its look, temperament, pace, habits and what it
    /// wears, for the art crate.
    pub creature: Creature,
    pub dress: Option<AccessoryArt>,
    pub traveler: Traveler,
    pub character: Character,
}

impl Resident {
    pub fn genome(&self) -> &AppearanceGenome {
        &self.creature.appearance
    }

    pub fn parent(&self) -> Option<Id> {
        match self.traveler.role {
            TravelRole::Adult => None,
            TravelRole::Mini { parent_id } => Some(parent_id.0),
        }
    }

    pub fn is_little(&self) -> bool {
        self.parent().is_some()
    }
}

/// How one pair gets on, under Home's names for Desktop's bands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bond {
    pub warmth: Band,
    pub familiarity: Band,
    pub playfulness: Band,
    pub friction: Band,
}

impl Bond {
    /// What two companions with no bond on record are to each other.
    pub const STRANGERS: Self = Self {
        warmth: Band::None,
        familiarity: Band::None,
        playfulness: Band::None,
        friction: Band::None,
    };

    /// Close enough to seek each other out.
    pub fn close(&self) -> bool {
        self.warmth >= Band::High && self.friction <= Band::Low
    }
}

pub struct Household {
    pub snapshot: HomeSnapshot,
    pub residents: Vec<Resident>,
    /// Friends Desktop has lent for the visit, who live in houses of their own.
    pub visitors: Vec<Resident>,
}

impl Household {
    /// Everyone in the house, ready for the room. Fails only if Desktop sent a look this build
    /// cannot draw, which is a reason to refuse the household rather than show someone wrong.
    pub fn new(snapshot: HomeSnapshot) -> Result<Self, TravelError> {
        let ready = |travelers: &[formiga_travel::Traveler]| {
            travelers
                .iter()
                .map(|traveler| {
                    Ok(Resident {
                        id: traveler.id.0,
                        name: traveler.name.clone(),
                        creature: traveler.to_creature()?,
                        dress: traveler.accessory.map(|accessory| accessory.to_art()),
                        character: Character::of(traveler),
                        traveler: traveler.clone(),
                    })
                })
                .collect::<Result<Vec<_>, TravelError>>()
        };
        let residents = ready(&snapshot.residents)?;
        let visitors = ready(&snapshot.visitors)?;
        Ok(Self {
            snapshot,
            residents,
            visitors,
        })
    }

    /// Everyone who lives here, then everyone visiting.
    pub fn everyone(&self) -> impl Iterator<Item = &Resident> {
        self.residents.iter().chain(&self.visitors)
    }

    pub fn is_visitor(&self, id: Id) -> bool {
        self.visitors.iter().any(|visitor| visitor.id == id)
    }

    /// Whose house a visitor comes from: "Biscuit's house".
    /// Whom in the house a visitor came to see: the resident it is closest to.
    pub fn friend_of(&self, visitor: Id) -> Option<&Resident> {
        self.residents.iter().max_by_key(|resident| {
            let bond = self.bond(resident.id, visitor);
            (
                bond.warmth,
                bond.familiarity,
                std::cmp::Reverse(resident.id),
            )
        })
    }

    pub fn reduce_motion(&self) -> bool {
        self.snapshot.presentation.reduce_motion
    }

    /// Anyone in the house by id, whether they live here or are visiting.
    pub fn resident(&self, id: Id) -> Option<&Resident> {
        self.everyone().find(|resident| resident.id == id)
    }

    pub fn keeper(&self) -> &Resident {
        &self.residents[0]
    }

    /// "Mochi's house".
    pub fn house_name(&self) -> String {
        format!("{}'s house", self.keeper().name)
    }

    /// How `a` and `b` get on.
    pub fn bond(&self, a: Id, b: Id) -> Bond {
        let (low, high) = if a < b { (a, b) } else { (b, a) };
        self.snapshot
            .relationships
            .iter()
            .find(|pair| pair.a.0 == low && pair.b.0 == high)
            .map_or(Bond::STRANGERS, |pair| Bond {
                warmth: pair.affinity,
                familiarity: pair.familiarity,
                playfulness: pair.playfulness,
                friction: pair.avoidance,
            })
    }

    /// Whether one is the other's little one, or its adult: family, whatever the bond says.
    pub fn family(&self, a: Id, b: Id) -> bool {
        let parent = |id| self.resident(id).and_then(Resident::parent);
        parent(a) == Some(b) || parent(b) == Some(a)
    }

    /// Where a thing is shown in the village, as the drawer says it: "Here", "In Biscuit's house",
    /// or nowhere yet.
    pub fn whereabouts(&self, state: &HomeState, item: &DisplayId) -> Whereabouts {
        match state.shown_by(item) {
            None => Whereabouts::Nowhere,
            Some(keeper) if keeper == self.snapshot.household.keeper => Whereabouts::Here,
            Some(keeper) => Whereabouts::Elsewhere(self.snapshot.neighbour(keeper).map_or_else(
                || "another house".to_owned(),
                |house| format!("{}'s house", house.name),
            )),
        }
    }
}

/// Where in the village a thing is shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Whereabouts {
    Nowhere,
    Here,
    /// In another household's house, by name: "Biscuit's house".
    Elsewhere(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::sample;

    #[test]
    fn the_sample_household_draws_and_knows_its_own() {
        let household = Household::new(sample::snapshot()).unwrap();
        assert_eq!(household.residents.len(), 2);
        let keeper = household.keeper().id;
        let pip = household.residents[1].id;
        assert!(household.residents[1].is_little());
        assert!(household.family(keeper, pip));
        assert_eq!(household.bond(keeper, pip), household.bond(pip, keeper));
        assert_eq!(
            household.house_name(),
            format!("{}'s house", household.keeper().name)
        );
    }
}
