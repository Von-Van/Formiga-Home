//! Rooms set up for looking at: the sample house as it first opens, and the same house lived in
//! for a while, with something shown every way a thing can be. For the review renders and the
//! tests, never for a real household.

use crate::actor::{Actor, Pose};
use crate::art::cues::Cue;
use crate::catalog;
use crate::house::House;
use crate::household::Household;
use crate::placement;
use crate::starter;
use formiga_art::ExpressionKind;
use formiga_core::ActionKind;
use formiga_home_contract::{CatalogId, DisplayId, HouseholdHome, MementoKind, Spot, WallSide};

/// The sample house a few weeks on: a sofa and a low table, a case for the precious things, a
/// lamp and a long rug, finds on the shelf, in the case, on the table, on both walls and on the
/// floor.
pub fn lived_in(household: &Household, floor: &str, wall: &str) -> HouseholdHome {
    let mut home = starter::home(&household.snapshot);
    let layout = &mut home.rooms[0];
    layout.floor = CatalogId::parse(floor).unwrap_or_else(|| CatalogId::known("floor.boards"));
    layout.wall = CatalogId::parse(wall).unwrap_or_else(|| CatalogId::known("wall.leafy"));
    layout
        .pieces
        .retain(|placed| placed.piece.as_str() != "round_rug");
    let add = |home: &mut HouseholdHome, id: &str, x, y, turn| {
        let piece = catalog::PIECES.iter().find(|piece| piece.id == id).unwrap();
        placement::add_piece(home, 0, piece, x, y, turn)
    };
    add(&mut home, "long_rug", 2, 3, 0);
    add(&mut home, "sofa", 3, 6, 2);
    let table = add(&mut home, "low_table", 3, 4, 0);
    let case = add(&mut home, "case", 7, 2, 3);
    add(&mut home, "lamp", 7, 0, 0);
    add(&mut home, "cushion", 6, 4, 0);
    add(&mut home, "snack_bowl", 1, 6, 0);
    let layout = &home.rooms[0];
    let shelf = layout
        .pieces
        .iter()
        .find(|placed| placed.piece.as_str() == "shelf")
        .unwrap()
        .uid;
    let side_table = layout
        .pieces
        .iter()
        .find(|placed| placed.piece.as_str() == "side_table")
        .unwrap()
        .uid;
    let shown = [
        (
            DisplayId::find(3),
            Spot::On {
                piece: shelf,
                slot: 0,
            },
        ),
        (
            DisplayId::find(16),
            Spot::On {
                piece: shelf,
                slot: 1,
            },
        ),
        (
            DisplayId::souvenir("chest_marble").unwrap(),
            Spot::On {
                piece: shelf,
                slot: 2,
            },
        ),
        (
            DisplayId::find(0),
            Spot::On {
                piece: case,
                slot: 0,
            },
        ),
        (
            DisplayId::find(135),
            Spot::On {
                piece: case,
                slot: 1,
            },
        ),
        (
            DisplayId::find(89),
            Spot::On {
                piece: table,
                slot: 0,
            },
        ),
        (
            DisplayId::find(18),
            Spot::On {
                piece: table,
                slot: 1,
            },
        ),
        (
            DisplayId::find(1),
            Spot::On {
                piece: side_table,
                slot: 0,
            },
        ),
        (
            DisplayId::find(76),
            Spot::Wall {
                side: WallSide::North,
                at: 2,
            },
        ),
        (
            DisplayId::souvenir("picnic_ribbon").unwrap(),
            Spot::Wall {
                side: WallSide::North,
                at: 3,
            },
        ),
        (
            DisplayId::find(98),
            Spot::Wall {
                side: WallSide::North,
                at: 5,
            },
        ),
        (
            DisplayId::find(26),
            Spot::Wall {
                side: WallSide::West,
                at: 1,
            },
        ),
        (
            DisplayId::find(159),
            Spot::Wall {
                side: WallSide::West,
                at: 4,
            },
        ),
        (DisplayId::find(132), Spot::Floor { x: 6, y: 7 }),
        (DisplayId::find(9), Spot::Floor { x: 0, y: 3 }),
    ];
    for (item, spot) in shown {
        placement::show(&mut home, 0, &item, spot);
    }
    keepsakes(&mut home, household);
    home
}

/// What a few weeks of visits leave: a postcard from the friend who drops by, the little one's
/// drawing of the keeper, and a photo of the two of them, each up where there is room.
fn keepsakes(home: &mut HouseholdHome, household: &Household) {
    let at = time::OffsetDateTime::now_utc() - time::Duration::days(3);
    let keeper = household.keeper();
    let likeness = |resident: &crate::household::Resident| {
        (
            formiga_home_contract::TravelerId(resident.id),
            crate::keepsakes::ink_of(resident),
        )
    };
    let mut made = Vec::new();
    if let Some(friend) = household.visitors.first() {
        made.extend(crate::keepsakes::make(
            home,
            MementoKind::Postcard,
            Some(formiga_home_contract::TravelerId(friend.id)),
            Vec::new(),
            at,
        ));
    }
    if let Some(little) = household.residents.iter().find(|r| r.is_little()) {
        made.extend(crate::keepsakes::make(
            home,
            MementoKind::Drawing,
            Some(formiga_home_contract::TravelerId(little.id)),
            vec![likeness(keeper)],
            at,
        ));
    }
    let everyone: Vec<_> = household.residents.iter().map(likeness).collect();
    made.extend(crate::keepsakes::make(
        home,
        MementoKind::Photo,
        None,
        everyone,
        at,
    ));
    let mut snapshot = household.snapshot.clone();
    crate::keepsakes::stock(&mut snapshot, home);
    let walls = [
        (WallSide::North, 6),
        (WallSide::West, 6),
        (WallSide::North, 0),
        (WallSide::West, 2),
    ];
    for id in made {
        let Some(item) = snapshot.item(&id) else {
            continue;
        };
        let free = walls.iter().find_map(|&(side, at)| {
            let spot = Spot::Wall { side, at };
            placement::can_show(&home.rooms[0], item, spot).map(|_| spot)
        });
        if let Some(spot) = free {
            placement::show(home, 0, &id, spot);
        }
    }
}

/// The same house grown to `rooms` rooms: a reading nook behind it, then a gallery beside it, each
/// furnished as its kind first comes and put wherever it is first offered.
pub fn grown(
    mut home: HouseholdHome,
    rooms: usize,
    snapshot: &formiga_home_contract::HomeSnapshot,
) -> HouseholdHome {
    for id in ["room.nook", "room.gallery"]
        .into_iter()
        .take(rooms.saturating_sub(1))
    {
        let template = catalog::ROOMS
            .iter()
            .find(|template| template.id == id)
            .unwrap();
        if let Some(bigger) = crate::arrange::room_places(&home, template, snapshot)
            .into_iter()
            .next()
        {
            home = bigger;
        }
    }
    home
}

/// The household placed about the room for a picture: the keeper settled in the armchair, the
/// little ones up and about by the toys.
pub fn pose(household: &Household, house: &House, reduce_motion: bool) -> Vec<Actor> {
    let first = &house.rooms[0];
    let (ox, oy) = (f32::from(first.x), f32::from(first.y));
    let mut actors = Vec::new();
    for (index, resident) in household.residents.iter().enumerate() {
        let mut actor = Actor::new(resident, (ox + 4.5, oy + 3.5), reduce_motion);
        match index {
            0 => {
                if let Some(chair) = house
                    .pieces
                    .iter()
                    .find(|placed| placed.piece.as_str() == "armchair")
                {
                    let piece = catalog::piece(&chair.piece).unwrap();
                    let (cx, cy) = placement::footprint(chair).centre();
                    let facing = matches!(chair.turn % 4, 1 | 2);
                    actor.settle_on(chair.uid, (cx, cy), piece.lift as f32, facing);
                    actor.strike(
                        Pose::new(ActionKind::Perch, resident.character.settled_face()),
                        0.0,
                    );
                }
            }
            1 => {
                actor.pos = (ox + 4.6, oy + 5.4);
                actor.facing_right = true;
                actor.strike(
                    Pose::new(ActionKind::SoloPlay, ExpressionKind::Joy).with_cue(Cue::Note),
                    0.0,
                );
            }
            _ => {
                actor.pos = (ox + 2.5 + index as f32, oy + 2.5);
                actor.strike(Pose::idle(resident.character.idle_face()), 0.0);
            }
        }
        actors.push(actor);
    }
    actors
}

/// Every resident in every pose Home uses, a row each, for review: `--render-poses`.
pub fn poses(household: &Household) -> formiga_art::Canvas {
    use formiga_art::{BodyClip, Canvas};
    use formiga_core::Gesture;
    let clips: [BodyClip; 14] = [
        ActionKind::Idle.into(),
        ActionKind::Perch.into(),
        ActionKind::Sleep.into(),
        ActionKind::Homebound.into(),
        ActionKind::SoloPlay.into(),
        ActionKind::SocialPlay.into(),
        ActionKind::Eat.into(),
        ActionKind::Greet.into(),
        ActionKind::InspectScreen.into(),
        ActionKind::PresentDiscovery.into(),
        ActionKind::PetReaction.into(),
        Gesture::Watch.into(),
        Gesture::Cheer.into(),
        Gesture::Crouch.into(),
    ];
    let cell = (52, 56);
    let mut sheet = Canvas::new(
        (cell.0 * clips.len() as i32) as u32,
        (cell.1 * household.residents.len() as i32) as u32,
    );
    for (row, resident) in household.residents.iter().enumerate() {
        for (column, clip) in clips.iter().enumerate() {
            let (left, top) = (column as i32 * cell.0, row as i32 * cell.1);
            let tone = if (row + column) % 2 == 0 {
                0xeadfcf
            } else {
                0xdfd2bf
            };
            crate::paint::rect(
                &mut sheet,
                left,
                top,
                cell.0,
                cell.1,
                crate::paint::rgb(tone),
            );
            let frame = formiga_art::CreatureRenderer::render_dressed_composited_frame(
                resident.genome(),
                resident.dress,
                *clip,
                1,
                true,
                false,
                formiga_art::FaceRenderState {
                    expression: resident.character.idle_face(),
                    eyelids: formiga_art::EyelidPose::Open,
                    gaze: formiga_art::GazeDirection::default(),
                },
            );
            crate::paint::blit(&mut sheet, &frame, left + 2, top + 4);
        }
    }
    sheet
}
