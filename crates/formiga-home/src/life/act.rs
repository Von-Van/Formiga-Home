//! What someone in the house does, and what each act means: who else it takes, what it is done
//! with, the drive it settles, how Desktop counts it, and how it is put into words.

use crate::catalog;
use crate::character::Drive;
use crate::house::House;
use crate::household::{Household, Id};
use crate::projects::Count;
use formiga_home_contract::{DisplayId, HomeSnapshot, Together};

/// Something someone in the house does.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    GoTo(u8, u8),
    Sit(u16),
    Relax(u16),
    Nap(u16),
    Sleep(u16),
    CurlUp(u16),
    /// Lie down in the bed, and invite a little one to curl up too.
    InviteLittle(u16, Id),
    /// Sit on the floor at a table.
    SitAt(u16),
    /// A little one's: sit at a table and draw someone.
    Draw(u16, Id),
    /// Bounce on a bed, which is not what beds are for.
    Bounce(u16),
    /// Stretch out on a rug.
    Sprawl(u16),
    /// Switch a lamp on, or off.
    SwitchLamp(u16),
    /// Tend a plant.
    Tend(u16),
    /// Look over everything on a shelf or in a case.
    Browse(u16),
    Play(u16),
    PlayWith(u16, Id),
    ShowOffToy(u16),
    Snack(u16),
    Share(u16, Id),
    /// Sit by the fire a while.
    WarmUp(u16),
    /// Have supper at the family table.
    Supper(u16),
    /// Put on a puppet show, alone, or for someone who watches.
    PutOnShow(u16),
    PutOnShowFor(u16, Id),
    /// Wind a clock.
    Wind(u16),
    Inspect(DisplayId),
    Remember(DisplayId),
    ShowTo(DisplayId, Id),
    FussWith(DisplayId),
    PlayWithFind(DisplayId),
    /// Wear a find for a while, then put it back.
    TryOn(DisplayId),
    Greet(Id),
    SitTogether(Id),
    PlayTogether(Id),
    Tease(Id),
    Comfort(Id),
    Hug(Id),
    /// Its own idea only: a grump put out, for instance, by someone in its favourite seat.
    GrumbleAt(Id),
    /// Its own idea only: a look round somewhere else in the room.
    Wander(u8, u8),
}

impl Act {
    /// Who else it takes, if anyone.
    pub fn partner(&self) -> Option<Id> {
        match self {
            Self::InviteLittle(_, other)
            | Self::PlayWith(_, other)
            | Self::Share(_, other)
            | Self::PutOnShowFor(_, other)
            | Self::ShowTo(_, other)
            | Self::Greet(other)
            | Self::SitTogether(other)
            | Self::PlayTogether(other)
            | Self::Tease(other)
            | Self::Comfort(other)
            | Self::Hug(other) => Some(*other),
            _ => None,
        }
    }

    pub fn piece(&self) -> Option<u16> {
        match self {
            Self::Sit(uid)
            | Self::Relax(uid)
            | Self::Nap(uid)
            | Self::Sleep(uid)
            | Self::CurlUp(uid)
            | Self::InviteLittle(uid, _)
            | Self::SitAt(uid)
            | Self::Draw(uid, _)
            | Self::Bounce(uid)
            | Self::Sprawl(uid)
            | Self::SwitchLamp(uid)
            | Self::Tend(uid)
            | Self::Browse(uid)
            | Self::Play(uid)
            | Self::PlayWith(uid, _)
            | Self::ShowOffToy(uid)
            | Self::Snack(uid)
            | Self::Share(uid, _)
            | Self::WarmUp(uid)
            | Self::Supper(uid)
            | Self::PutOnShow(uid)
            | Self::PutOnShowFor(uid, _)
            | Self::Wind(uid) => Some(*uid),
            _ => None,
        }
    }

    pub fn item(&self) -> Option<&DisplayId> {
        match self {
            Self::Inspect(item)
            | Self::Remember(item)
            | Self::ShowTo(item, _)
            | Self::FussWith(item)
            | Self::PlayWithFind(item)
            | Self::TryOn(item) => Some(item),
            _ => None,
        }
    }

    /// How it is time spent together, as Desktop counts it, if it is.
    pub(super) fn together(&self) -> Option<Together> {
        match self {
            Self::PlayTogether(_) | Self::PlayWith(..) | Self::PutOnShowFor(..) => {
                Some(Together::Play)
            }
            Self::SitTogether(_) | Self::Share(..) | Self::Hug(_) | Self::InviteLittle(..) => {
                Some(Together::Cozy)
            }
            Self::Comfort(_) => Some(Together::Care),
            Self::Tease(_) => Some(Together::Squabble),
            _ => None,
        }
    }

    /// What it counts as for the household's projects, if anything.
    pub(super) fn counts(&self) -> Option<Count> {
        match self {
            Self::Snack(_) | Self::Share(..) => Some(Count::Snacks),
            Self::Nap(_) | Self::Sleep(_) | Self::CurlUp(_) | Self::InviteLittle(..) => {
                Some(Count::Rests)
            }
            Self::Play(_)
            | Self::PlayWith(..)
            | Self::ShowOffToy(_)
            | Self::PlayWithFind(_)
            | Self::PlayTogether(_) => Some(Count::Games),
            Self::Tend(_) => Some(Count::Tending),
            Self::WarmUp(_) => Some(Count::Hearthside),
            Self::Supper(_) => Some(Count::Suppers),
            Self::PutOnShow(_) | Self::PutOnShowFor(..) => Some(Count::Shows),
            Self::Inspect(_) | Self::Remember(_) | Self::ShowTo(..) | Self::Browse(_) => {
                Some(Count::Looks)
            }
            _ => None,
        }
    }

    /// Done sitting or lying on its piece.
    pub(super) fn settles(&self) -> bool {
        matches!(
            self,
            Self::Sit(_)
                | Self::Relax(_)
                | Self::Nap(_)
                | Self::Sleep(_)
                | Self::CurlUp(_)
                | Self::InviteLittle(..)
                | Self::Bounce(_)
        )
    }

    /// The drive doing it settles.
    pub(super) fn drive(&self) -> Drive {
        match self {
            Self::Sit(_)
            | Self::Relax(_)
            | Self::CurlUp(_)
            | Self::Snack(_)
            | Self::Remember(_)
            | Self::SitAt(_)
            | Self::Tend(_)
            | Self::WarmUp(_)
            | Self::Supper(_)
            | Self::GrumbleAt(_) => Drive::Comfort,
            Self::Nap(_) | Self::Sleep(_) | Self::InviteLittle(..) | Self::Sprawl(_) => Drive::Rest,
            Self::Play(_)
            | Self::PlayWith(..)
            | Self::ShowOffToy(_)
            | Self::PlayWithFind(_)
            | Self::PlayTogether(_)
            | Self::Tease(_)
            | Self::FussWith(_)
            | Self::TryOn(_)
            | Self::Draw(..)
            | Self::PutOnShow(_)
            | Self::PutOnShowFor(..)
            | Self::Bounce(_) => Drive::Play,
            Self::Inspect(_)
            | Self::Browse(_)
            | Self::SwitchLamp(_)
            | Self::Wind(_)
            | Self::Wander(..)
            | Self::GoTo(..) => Drive::Curiosity,
            Self::Share(..)
            | Self::ShowTo(..)
            | Self::Greet(_)
            | Self::SitTogether(_)
            | Self::Comfort(_)
            | Self::Hug(_) => Drive::Company,
        }
    }

    /// What the owner is offered, and what the queue shows: "Nap in the armchair", "Hug Pip".
    pub fn label(&self, household: &Household, house: &House, snapshot: &HomeSnapshot) -> String {
        let piece = |uid: &u16| {
            house
                .piece(*uid)
                .and_then(|placed| catalog::piece(&placed.piece))
                .map_or("something", |piece| piece.name)
                .to_lowercase()
        };
        let item = |id: &DisplayId| {
            snapshot
                .item(id)
                .map_or("something".to_owned(), |item| item.name.to_lowercase())
        };
        let who = |id: &Id| {
            household
                .resident(*id)
                .map_or("someone".to_owned(), |r| r.name.clone())
        };
        match self {
            Self::GoTo(..) => "Go here".to_owned(),
            Self::Sit(uid) => format!("Sit on the {}", piece(uid)),
            Self::Relax(uid) => format!("Relax on the {}", piece(uid)),
            Self::Nap(uid) => format!("Nap on the {}", piece(uid)),
            Self::Sleep(uid) => format!("Sleep in the {}", piece(uid)),
            Self::CurlUp(uid) => format!("Curl up in the {}", piece(uid)),
            Self::InviteLittle(_, little) => format!("Turn in with {}", who(little)),
            Self::SitAt(uid) => format!("Sit at the {}", piece(uid)),
            Self::Draw(uid, of) => format!("Draw {} at the {}", who(of), piece(uid)),
            Self::Bounce(uid) => format!("Bounce on the {}", piece(uid)),
            Self::Sprawl(uid) => format!("Stretch out on the {}", piece(uid)),
            Self::SwitchLamp(uid) => format!("Switch the {}", piece(uid)),
            Self::Tend(uid) => format!("Tend the {}", piece(uid)),
            Self::Browse(uid) => format!("Look over the {}", piece(uid)),
            Self::Play(uid) => format!("Play with the {}", piece(uid)),
            Self::PlayWith(uid, other) => {
                format!("Play with the {} with {}", piece(uid), who(other))
            }
            Self::ShowOffToy(uid) => format!("Show off with the {}", piece(uid)),
            Self::Snack(uid) => format!("Have a snack from the {}", piece(uid)),
            Self::Share(_, other) => format!("Share a snack with {}", who(other)),
            Self::WarmUp(uid) => format!("Warm up by the {}", piece(uid)),
            Self::Supper(uid) => format!("Have supper at the {}", piece(uid)),
            Self::PutOnShow(uid) => format!("Put on a show at the {}", piece(uid)),
            Self::PutOnShowFor(_, other) => format!("Put on a puppet show for {}", who(other)),
            Self::Wind(uid) => format!("Wind the {}", piece(uid)),
            Self::Inspect(id) => format!("Look at the {}", item(id)),
            Self::Remember(id) => format!("Remember the {}", item(id)),
            Self::ShowTo(id, other) => format!("Show {} the {}", who(other), item(id)),
            Self::FussWith(id) => format!("Fuss with the {}", item(id)),
            Self::PlayWithFind(id) => format!("Play with the {}", item(id)),
            Self::TryOn(id) => format!("Try on the {}", item(id)),
            Self::Greet(other) => format!("Say hello to {}", who(other)),
            Self::SitTogether(other) => format!("Sit with {}", who(other)),
            Self::PlayTogether(other) => format!("Play with {}", who(other)),
            Self::Tease(other) => format!("Tease {}", who(other)),
            Self::Comfort(other) => format!("Comfort {}", who(other)),
            Self::Hug(other) => format!("Hug {}", who(other)),
            Self::GrumbleAt(other) => format!("Grumble at {}", who(other)),
            Self::Wander(..) => "Have a look round".to_owned(),
        }
    }

    /// What it is doing, as the room's status line says it: "napping on the armchair".
    pub(super) fn doing(
        &self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
    ) -> String {
        let label = self.label(household, house, snapshot);
        let mut words = label.splitn(2, ' ');
        let verb = words.next().unwrap_or_default();
        let rest = words.next().unwrap_or_default();
        let ing = match verb {
            "Go" => return "on the way somewhere".to_owned(),
            "Sit" => "sitting",
            "Relax" => "relaxing",
            "Nap" => "napping",
            "Sleep" => "sleeping",
            "Curl" => "curling",
            "Turn" => "turning",
            "Bounce" => "bouncing",
            "Draw" => "drawing",
            "Stretch" => "stretching",
            "Switch" => "switching",
            "Tend" => "tending",
            "Play" => "playing",
            "Show" => "showing",
            "Have" => "having",
            "Warm" => "warming",
            "Wind" => "winding",
            "Put" => "putting",
            "Share" => "sharing",
            "Look" => "looking",
            "Remember" => "remembering",
            "Fuss" => "fussing",
            "Try" => "trying",
            "Say" => "saying",
            "Tease" => "teasing",
            "Comfort" => "comforting",
            "Hug" => "hugging",
            "Grumble" => "grumbling",
            other => return other.to_lowercase(),
        };
        format!("{ing} {rest}")
    }
}
