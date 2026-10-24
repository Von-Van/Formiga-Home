//! What the owner can ask of someone, for the menu, and where in the room things are done from:
//! the places on a piece to sit or lie, where to stand to look at something shown, and whether
//! the house still has what an act is about.

use super::{Act, somewhere_to_sit, subject, suits, toy_like, wearable};
use crate::body::Body;
use crate::catalog::{self, Family, Use};
use crate::house::{At, House};
use crate::household::{Household, Id, Resident};
use crate::path::Floor;
use crate::placement;
use formiga_home_contract::{DisplayId, HomeSnapshot};
use formiga_travel::Band;

/// Every place on a piece someone can sit or lie: along its width for a seat, head and foot for
/// a bed.
pub fn seat_points(
    placed: &formiga_home_contract::PlacedPiece,
    piece: &catalog::Piece,
) -> Vec<(f32, f32)> {
    let (w, d) = (f32::from(piece.size.0), f32::from(piece.size.1));
    let points: Vec<(f32, f32)> = if piece.has(Use::Sit { seats: 1 }) {
        let seats = piece.seats().max(1);
        (0..seats)
            .map(|seat| ((f32::from(seat) + 0.5) * w / f32::from(seats), d * 0.6))
            .collect()
    } else if piece.has(Use::Sleep) && piece.size.1 >= 2 {
        vec![(w / 2.0, 0.75), (w / 2.0, d - 0.55)]
    } else if piece.has(Use::Sleep) {
        vec![(w / 2.0, d / 2.0)]
    } else {
        Vec::new()
    };
    points
        .into_iter()
        .map(|point| {
            let (x, y) = catalog::turned(point, piece.size, placed.turn);
            (f32::from(placed.x) + x, f32::from(placed.y) + y)
        })
        .collect()
}

/// Tiles to stand on, nearest first, and the point to look at.
type Viewpoint = (Vec<(i32, i32)>, (f32, f32));

/// Where someone stands to look at something shown, and the point they look at.
pub(super) fn item_spot(house: &House, floor: &Floor, item: &DisplayId) -> Option<Viewpoint> {
    let shown = house.shown.iter().find(|shown| &shown.item == item)?;
    match shown.at {
        At::On { piece, slot } => {
            let placed = house.piece(piece)?;
            let footprint = placement::footprint(placed);
            let at = placement::surfaces(placed).get(usize::from(slot))?.at;
            Some((
                floor.beside(
                    placed.x,
                    placed.y,
                    footprint.w,
                    footprint.d,
                    catalog::front(placed.turn),
                ),
                at,
            ))
        }
        At::Wall { room, side, at } => {
            let wall = house.wall(room, side, at)?;
            let (tile, look) = ((i32::from(wall.x), i32::from(wall.y)), wall.middle());
            let near = floor.nearest_open_in(tile, room)?;
            let mut around = vec![near];
            around.extend(
                floor
                    .beside(near.0 as u8, near.1 as u8, 1, 1, (0, 1))
                    .into_iter()
                    .take(2),
            );
            Some((around, look))
        }
        At::Floor { x, y } => Some((
            floor.beside(x, y, 1, 1, (0, 1)),
            (f32::from(x) + 0.5, f32::from(y) + 0.5),
        )),
    }
}

/// The room something shown is in.
pub(super) fn item_room(house: &House, item: &DisplayId) -> Option<u8> {
    let shown = house.shown.iter().find(|shown| &shown.item == item)?;
    match shown.at {
        At::On { piece, .. } => house
            .piece(piece)
            .and_then(|placed| house.room_at(i32::from(placed.x), i32::from(placed.y))),
        At::Wall { room, .. } => Some(room),
        At::Floor { x, y } => house.room_at(i32::from(x), i32::from(y)),
    }
}

/// Whether the house still has what an act is about.
pub(super) fn still_there(act: &Act, house: &House, snapshot: &HomeSnapshot) -> bool {
    if let Some(uid) = act.piece()
        && house.piece(uid).is_none()
    {
        return false;
    }
    if let Some(item) = act.item() {
        return snapshot.item(item).is_some()
            && house.shown.iter().any(|shown| &shown.item == item);
    }
    true
}

/// What someone can be asked to do with something in the room, for the owner's menu. `present`
/// is everyone in the house just now, and `guests` whoever of them is staying over.
pub fn choices(
    household: &Household,
    house: &House,
    snapshot: &HomeSnapshot,
    present: &[Id],
    guests: &[Id],
    id: Id,
    target: &crate::scene::Target,
) -> Vec<Act> {
    use crate::scene::Target;
    let Some(resident) = household.resident(id) else {
        return Vec::new();
    };
    let character = &resident.character;
    let visiting = household.is_visitor(id);
    let others: Vec<Id> = present
        .iter()
        .copied()
        .filter(|other| *other != id)
        .collect();
    let mut acts = Vec::new();
    match target {
        Target::Floor(x, y) => acts.push(Act::GoTo(*x, *y)),
        Target::Piece(uid) => {
            let Some(placed) = house.piece(*uid) else {
                return acts;
            };
            let Some(piece) = catalog::piece(&placed.piece) else {
                return acts;
            };
            if piece.has(Use::Sit { seats: 1 }) {
                acts.extend([Act::Sit(*uid), Act::Relax(*uid), Act::Nap(*uid)]);
            }
            if piece.has(Use::Sleep) && !visiting {
                acts.extend([Act::Sleep(*uid), Act::CurlUp(*uid), Act::Bounce(*uid)]);
                if !resident.is_little() && piece.size.1 >= 2 {
                    // A little one that can get up into the bed too.
                    for other in &others {
                        if household.resident(*other).is_some_and(|r| {
                            r.parent() == Some(id) && Body::of(r.genome()).settles_on(piece)
                        }) {
                            acts.push(Act::InviteLittle(*uid, *other));
                        }
                    }
                }
            } else if piece.has(Use::Sleep) && guests.contains(&id) {
                // A friend staying over may be shown to a bed.
                acts.extend([Act::Sleep(*uid), Act::CurlUp(*uid)]);
            }
            if piece.family == Family::Tables {
                acts.push(Act::SitAt(*uid));
                if resident.is_little() && !visiting {
                    acts.push(Act::Draw(*uid, subject(household, id, &others)));
                }
            }
            if piece.has(Use::Warm) {
                acts.push(Act::WarmUp(*uid));
            } else if piece.family == Family::Lights {
                acts.push(Act::SwitchLamp(*uid));
            }
            if piece.has(Use::Supper) {
                acts.push(Act::Supper(*uid));
            }
            if piece.has(Use::Wind) {
                acts.push(Act::Wind(*uid));
            }
            if piece.has(Use::Show) {
                acts.push(Act::PutOnShow(*uid));
                acts.extend(others.iter().map(|other| Act::PutOnShowFor(*uid, *other)));
            }
            if piece.family == Family::Plants {
                acts.push(Act::Tend(*uid));
            }
            if piece.family == Family::Shelves {
                acts.push(Act::Browse(*uid));
            }
            if piece.flat {
                acts.push(Act::Sprawl(*uid));
            }
            if piece.has(Use::Play) {
                acts.push(Act::Play(*uid));
                acts.extend(others.iter().map(|other| Act::PlayWith(*uid, *other)));
                acts.push(Act::ShowOffToy(*uid));
            }
            if piece.has(Use::Snack) {
                acts.push(Act::Snack(*uid));
                acts.extend(others.iter().map(|other| Act::Share(*uid, *other)));
            }
            let body = Body::of(resident.genome());
            acts.retain(|act| suits(body, house, act));
            if acts.is_empty() || piece.flat {
                let footprint = placement::footprint(placed);
                let (cx, cy) = footprint.centre();
                let floor = Floor::of(house);
                let beside = floor
                    .beside(
                        placed.x,
                        placed.y,
                        footprint.w,
                        footprint.d,
                        catalog::front(placed.turn),
                    )
                    .first()
                    .copied();
                if piece.flat {
                    acts.insert(0, Act::GoTo(cx as u8, cy as u8));
                } else if let Some((x, y)) = beside {
                    acts.push(Act::GoTo(x as u8, y as u8));
                }
            }
        }
        Target::Shown(item) => {
            acts.push(Act::Inspect(item.clone()));
            acts.push(Act::Remember(item.clone()));
            acts.extend(others.iter().map(|other| Act::ShowTo(item.clone(), *other)));
            let thing = snapshot.item(item);
            if thing.is_some_and(toy_like) {
                acts.push(Act::PlayWithFind(item.clone()));
            }
            if !visiting && thing.and_then(wearable).is_some() {
                acts.push(Act::TryOn(item.clone()));
            }
            if character.fusses() {
                acts.push(Act::FussWith(item.clone()));
            }
        }
        Target::Resident(other) if *other != id => {
            let other = *other;
            let bond = household.bond(id, other);
            let family = household.family(id, other);
            acts.push(Act::Greet(other));
            // Somewhere to sit for each of them.
            let sits = |who: &Resident| somewhere_to_sit(Body::of(who.genome()), house);
            if sits(resident) && household.resident(other).is_some_and(sits) {
                acts.push(Act::SitTogether(other));
            }
            if character.playful() {
                acts.push(Act::PlayTogether(other));
            }
            if character.teases() {
                acts.push(Act::Tease(other));
            }
            let little = household.resident(other).is_some_and(|r| r.is_little());
            if character.hugs() || family || little {
                acts.push(Act::Comfort(other));
            }
            if character.hugs() && (bond.warmth >= Band::Medium || family) {
                acts.push(Act::Hug(other));
            }
        }
        Target::Resident(_) => {}
    }
    acts
}
