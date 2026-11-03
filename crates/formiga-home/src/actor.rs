//! A resident in the room: where it stands on the floor, which way it faces, what its body is
//! doing, and the frames `formiga-art` draws for it, face and all.
//!
//! The room is isometric and the creatures are not: each is drawn upright, anchored by its feet,
//! and faces left or right by which way it is going across the screen. What it does is decided by
//! the household's simulation; the actor only walks where it is sent and holds the pose it is given.

use crate::art::cues::Cue;
use crate::body::Body;
use crate::household::{Id, Resident};
use crate::iso::View;
use crate::paint;
use formiga_art::{
    AccessoryArt, AnimationSpec, BodyClip, Canvas, CreatureRenderer, ExpressionKind, EyelidPose,
    FRAME_SIZE, FaceRenderState, GazeDirection, HeldThing, Holding, MealPose, PixelPoint,
};
use formiga_core::BubbleGrowth;
use formiga_core::meals::Meal;
use formiga_core::talk::Saying;
use formiga_core::{ActionKind, AppearanceGenome};
use std::collections::{HashMap, VecDeque};

/// What its body shows while it is not walking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub clip: BodyClip,
    pub expression: ExpressionKind,
    /// Holds its first frame rather than playing: a held look, or any pose with motion reduced.
    pub held: bool,
    pub cue: Option<Cue>,
    /// Turns this way and that, for a dance or a twirl.
    pub spin: bool,
    /// What is in its paws.
    pub hold: Hold,
    /// What it says, in a bubble over its head, if its sign says less than it means to, and how
    /// long after the pose begins: an answer waits for what it answers.
    pub say: Option<(Saying, f32)>,
}

/// What a resident has in its paws while it holds a pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hold {
    /// Whatever its clip holds of its own: its snack while it eats, its toy while it plays.
    Clip,
    /// Nothing of its own: the room's own thing is what it is using.
    Nothing,
    /// A meal, eaten through as the pose is held: what is eaten, and whether it is a picky eater
    /// or one that loves its food, for the face it pulls.
    Meal {
        meal: Meal,
        picky: bool,
        loves: bool,
    },
    /// Something of the room's, drawn in its paws by the scene: a find off the shelf, or a piece
    /// small enough to pick up, which is not drawn where it stood meanwhile.
    Room,
}

impl Pose {
    pub fn new(clip: impl Into<BodyClip>, expression: ExpressionKind) -> Self {
        Self {
            clip: clip.into(),
            expression,
            held: false,
            cue: None,
            spin: false,
            hold: Hold::Clip,
            say: None,
        }
    }

    pub fn idle(expression: ExpressionKind) -> Self {
        Self::new(ActionKind::Idle, expression)
    }

    pub fn with_cue(self, cue: Cue) -> Self {
        Self {
            cue: Some(cue),
            ..self
        }
    }

    pub fn held(self) -> Self {
        Self { held: true, ..self }
    }

    pub fn spinning(self) -> Self {
        Self { spin: true, ..self }
    }

    pub fn holding(self, hold: Hold) -> Self {
        Self { hold, ..self }
    }

    /// Says `saying` over its head as the pose begins.
    pub fn saying(self, saying: Saying) -> Self {
        Self {
            say: Some((saying, 0.0)),
            ..self
        }
    }

    /// Says `saying` a moment after the pose begins, in answer to someone.
    pub fn answering(self, saying: Saying) -> Self {
        Self {
            say: Some((saying, ANSWER_SECS)),
            ..self
        }
    }

    #[cfg(test)]
    pub fn asleep(&self) -> bool {
        self.clip == BodyClip::Action(ActionKind::Sleep)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FrameKey {
    clip: BodyClip,
    frame: u8,
    facing_right: bool,
    expression: ExpressionKind,
    eyelids: EyelidPose,
    gaze: GazeDirection,
    held: HeldKey,
}

/// What a frame puts in the paws, as the cache tells frames apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HeldKey {
    Clip,
    Nothing,
    Thing(HeldThing),
}

/// Everything about how a resident is drawn at a moment, and where: when it is the same, so is
/// its picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    id: Id,
    hidden: bool,
    pos: (f32, f32),
    lift: f32,
    on_piece: Option<u16>,
    dress: Option<AccessoryArt>,
    frame: Option<FrameKey>,
    origin: (i32, i32),
    cue: Option<CueLook>,
}

/// A cue as it is drawn: which, how far it bobs, and for a sleeper's, its second z.
type CueLook = (Cue, i32, Option<(i32, u8)>);

/// Something of the room's a resident can have in its paws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Carrying {
    Find(formiga_home_contract::DisplayId),
    Piece(u16),
}

/// Getting up onto a piece over a walk's last leg, or down off one over its first: the piece, how
/// high its seat is, and which way it sits on it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Climb {
    piece: u16,
    height: f32,
    facing_right: bool,
}

/// How much of a leg onto or off a piece is spent stepping up or down: about from the middle of
/// the tile beside it to its edge, so the rest is walked along the top.
const CLIMB_STEP: f32 = 0.5;

/// How long an answer waits after what it answers, in seconds.
const ANSWER_SECS: f32 = 0.9;
/// How long a bubble is up, in seconds, and how long between one and the next while a pose goes
/// on: a long nap says its Z now and then, not the whole time.
const SAYING_SECS: f32 = 2.4;
const SAYING_AGAIN_SECS: f32 = 9.0;
/// Each of a bubble's two growing steps on the way in, and its two shrinking ones on the way out.
const SAYING_STEP_SECS: f32 = 0.12;

/// Frames are drawn on first use and kept; this many is a long visit's worth.
const FRAME_CACHE_LIMIT: usize = 1200;
const SPIN_TURN: f32 = 0.22;

pub struct Actor {
    pub id: Id,
    appearance: AppearanceGenome,
    body: Body,
    dress: Option<AccessoryArt>,
    foot_row: i32,
    /// Where its feet are on the floor, in tiles.
    pub pos: (f32, f32),
    pub facing_right: bool,
    /// How far above the floor it is: sitting on a chair, lying in a bed, held in the air.
    pub lift: f32,
    /// The piece it is sitting or lying on, if any: it is drawn with that piece.
    pub on_piece: Option<u16>,
    /// Not in the house at all: a visitor yet to come, or gone home.
    pub hidden: bool,
    pub pose: Pose,
    pub gaze: GazeDirection,
    /// When the current pose began.
    since: f32,
    path: VecDeque<(f32, f32)>,
    /// Where the leg it is walking began, so a climb knows how far along it is.
    leg_from: (f32, f32),
    /// The piece it climbs down from over its walk's first leg, if it set off from one.
    down: Option<Climb>,
    /// The piece it climbs up onto over its walk's last leg, if it is going to sit or lie.
    up: Option<Climb>,
    speed: f32,
    walk_face: ExpressionKind,
    frames: HashMap<FrameKey, Canvas>,
    /// Where each frame it has drawn holds things, by clip, frame and facing.
    holds: HashMap<(BodyClip, u8, bool), Option<PixelPoint>>,
    /// What of the room's it has in its paws while its pose holds [`Hold::Room`]: a find, or a
    /// piece by its name in the house.
    pub carrying: Option<Carrying>,
    /// Blinks come round every `period` seconds, offset by `phase`, so no two are in step.
    blink: (f32, f32),
    reduce_motion: bool,
}

impl Actor {
    pub fn new(resident: &Resident, pos: (f32, f32), reduce_motion: bool) -> Self {
        let baseline = CreatureRenderer::resting_baseline(resident.genome(), reduce_motion);
        let seed = (resident.id % 1009) as f32 / 1009.0;
        Self {
            id: resident.id,
            appearance: resident.genome().clone(),
            body: Body::of(resident.genome()),
            dress: resident.dress,
            foot_row: FRAME_SIZE as i32 - 1 - baseline as i32,
            pos,
            facing_right: resident.id.is_multiple_of(2),
            lift: 0.0,
            on_piece: None,
            hidden: false,
            pose: Pose::idle(resident.character.idle_face()),
            gaze: GazeDirection::default(),
            since: 0.0,
            path: VecDeque::new(),
            leg_from: pos,
            down: None,
            up: None,
            speed: resident.character.walk_speed(),
            walk_face: resident.character.walk_face(),
            frames: HashMap::new(),
            holds: HashMap::new(),
            carrying: None,
            blink: (3.2 + seed * 2.4, seed * 5.0),
            reduce_motion,
        }
    }

    /// The body it gets about the house with.
    pub fn body(&self) -> Body {
        self.body
    }

    /// Wear something else for a while, or what it came in again: every frame is drawn afresh.
    pub fn wear(&mut self, dress: Option<AccessoryArt>) {
        if dress != self.dress {
            self.dress = dress;
            self.frames.clear();
        }
    }

    /// Strike a pose from `now`.
    pub fn strike(&mut self, pose: Pose, now: f32) {
        if pose != self.pose {
            self.pose = pose;
            self.since = now;
        }
    }

    /// How long it has held its pose.
    pub fn held_for(&self, now: f32) -> f32 {
        (now - self.since).max(0.0)
    }

    /// Set off along `waypoints`. Off a piece, the first leg is the way down from it, walked and
    /// stepped down rather than dropped from. With motion reduced a walk is a cut: it is simply
    /// there, facing the way it went.
    pub fn walk(&mut self, waypoints: impl IntoIterator<Item = (f32, f32)>) {
        self.path.clear();
        self.path.extend(waypoints);
        self.leg_from = self.pos;
        self.up = None;
        self.down = self.on_piece.map(|piece| Climb {
            piece,
            height: self.lift,
            facing_right: self.facing_right,
        });
        if self.path.is_empty() {
            self.down = None;
            self.get_down();
        }
        if self.reduce_motion
            && let Some(last) = self.path.back().copied()
        {
            self.face_towards(last);
            self.pos = last;
            self.path.clear();
            self.down = None;
            self.get_down();
        }
    }

    /// Set off along `waypoints` to the floor beside a piece, then walk on up onto its seat at
    /// `seat` and sit there facing the way the piece does. Nobody is lifted onto a seat from
    /// across the room.
    pub fn walk_onto(
        &mut self,
        waypoints: impl IntoIterator<Item = (f32, f32)>,
        piece: u16,
        seat: (f32, f32),
        height: f32,
        facing_right: bool,
    ) {
        self.walk(waypoints.into_iter().chain([seat]));
        if self.walking() {
            self.up = Some(Climb {
                piece,
                height,
                facing_right,
            });
        } else {
            self.settle_on(piece, seat, height, facing_right);
        }
    }

    /// Stop where it is. Stopped up on top of a piece it stays up there, drawn with it, until it
    /// walks down; stopped mid-step, which is over the open floor beside it, it is put down there
    /// rather than left hanging at the piece's edge.
    pub fn stop(&mut self) {
        self.path.clear();
        let (up, down) = (self.up.take(), self.down.take());
        if let Some(climb) = up.or(down)
            && self.lift < climb.height
        {
            self.get_down();
        }
    }

    pub fn walking(&self) -> bool {
        !self.path.is_empty()
    }

    /// Where it is headed, or where it is.
    pub fn destination(&self) -> (f32, f32) {
        self.path.back().copied().unwrap_or(self.pos)
    }

    /// Sit or lie on a piece: on its seat, lifted to its height, facing the way the piece does.
    /// Placed there at once, so only for where it already is, or for a picture.
    pub fn settle_on(&mut self, piece: u16, seat: (f32, f32), lift: f32, facing_right: bool) {
        self.path.clear();
        self.up = None;
        self.down = None;
        self.pos = seat;
        self.lift = lift;
        self.on_piece = Some(piece);
        self.facing_right = facing_right;
    }

    /// Off whatever it was on, and back on the floor.
    pub fn get_down(&mut self) {
        self.on_piece = None;
        self.lift = 0.0;
    }

    /// Turn to look towards a point on the floor.
    pub fn face_towards(&mut self, point: (f32, f32)) {
        let across = (point.0 - self.pos.0) - (point.1 - self.pos.1);
        if across.abs() > 0.05 {
            self.facing_right = across > 0.0;
        }
    }

    /// Walk on for `dt` seconds. Whether it has arrived where it was going.
    pub fn advance(&mut self, dt: f32) -> bool {
        let mut budget = dt;
        while let Some(&next) = self.path.front() {
            let (dx, dy) = (next.0 - self.pos.0, next.1 - self.pos.1);
            let distance = (dx * dx + dy * dy).sqrt();
            self.face_towards(next);
            let stride = self.speed * budget;
            if stride >= distance {
                self.pos = next;
                budget -= distance / self.speed;
                self.path.pop_front();
                self.leg_from = next;
                // Down off the piece it set off from, and on the floor.
                if self.down.take().is_some() {
                    self.get_down();
                }
                if self.path.is_empty()
                    && let Some(up) = self.up.take()
                {
                    self.on_piece = Some(up.piece);
                    self.lift = up.height;
                    self.facing_right = up.facing_right;
                }
            } else {
                self.pos.0 += dx / distance * stride;
                self.pos.1 += dy / distance * stride;
                self.climb(next);
                return false;
            }
        }
        true
    }

    /// Partway along a leg towards `next`: stepping down off a piece near the end of the first,
    /// or up onto one near the start of the last, and drawn with that piece meanwhile. A floater
    /// has no step to take: it rises or sinks over the whole leg.
    fn climb(&mut self, next: (f32, f32)) {
        let between =
            |a: (f32, f32), b: (f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let length = between(self.leg_from, next);
        let step = if self.body.floats() {
            length
        } else {
            CLIMB_STEP.min(length)
        }
        .max(f32::EPSILON);
        if let Some(down) = self.down {
            self.on_piece = Some(down.piece);
            self.lift = down.height * (between(self.pos, next) / step).min(1.0);
        } else if let Some(up) = self.up.filter(|_| self.path.len() == 1) {
            self.on_piece = Some(up.piece);
            self.lift = up.height * (between(self.leg_from, self.pos) / step).min(1.0);
        }
    }

    /// What its body is shown doing at `now`.
    #[cfg(test)]
    pub fn shows(&self, now: f32) -> BodyClip {
        self.key(now).clip
    }

    fn key(&self, now: f32) -> FrameKey {
        let still = self.reduce_motion;
        let elapsed = self.held_for(now);
        let (clip, frame, expression, mut facing_right) = if self.walking() {
            let clip = BodyClip::Action(ActionKind::Traverse);
            (
                clip,
                frame_of(clip, now, still),
                self.walk_face,
                self.facing_right,
            )
        } else if let Hold::Meal {
            mut meal,
            picky,
            loves,
        } = self.pose.hold
        {
            // Each step of the meal in its own pose, the food going down as it goes.
            meal.elapsed = elapsed.min(meal.length - 0.01);
            let (clip, frame) = match formiga_art::meal_pose(&meal) {
                MealPose::Playing(clip, into) => (clip, frame_of(clip, into, still)),
                MealPose::Held(clip, frame) => (clip, if still { 0 } else { frame }),
            };
            (
                clip,
                frame,
                formiga_art::meal_face(&meal, picky, loves),
                self.facing_right,
            )
        } else {
            let pose = self.pose;
            let breathing = now + self.blink.1;
            let frame = if pose.held {
                0
            } else if pose.clip == BodyClip::Action(ActionKind::Idle) {
                frame_of(pose.clip, breathing, still)
            } else {
                frame_of(pose.clip, elapsed, still)
            };
            let turned = pose.spin && !still && (elapsed / SPIN_TURN) as i32 % 2 == 1;
            (
                pose.clip,
                frame,
                pose.expression,
                self.facing_right ^ turned,
            )
        };
        let asleep = clip == BodyClip::Action(ActionKind::Sleep);
        let walking = self.walking();
        let meal = match self.pose.hold {
            Hold::Meal { mut meal, .. } if !walking => {
                meal.elapsed = elapsed.min(meal.length - 0.01);
                Some(meal)
            }
            _ => None,
        };
        let held = match self.pose.hold {
            _ if walking => HeldKey::Clip,
            Hold::Clip => HeldKey::Clip,
            Hold::Nothing | Hold::Room => HeldKey::Nothing,
            Hold::Meal { .. } => meal
                .as_ref()
                .and_then(formiga_art::meal_held)
                .map_or(HeldKey::Nothing, HeldKey::Thing),
        };
        let eyelids = if let Some(eyelids) = meal.as_ref().and_then(formiga_art::meal_eyelids) {
            eyelids
        } else if asleep {
            EyelidPose::Closed
        } else if expression == ExpressionKind::Sleepy {
            EyelidPose::Half
        } else if !still && (now + self.blink.1) % self.blink.0 < 0.12 {
            EyelidPose::Closed
        } else {
            EyelidPose::Open
        };
        let gaze = if self.walking() || asleep {
            GazeDirection::default()
        } else {
            self.gaze
        };
        if still {
            facing_right = self.facing_right;
        }
        FrameKey {
            clip,
            frame,
            facing_right,
            expression,
            eyelids,
            gaze,
            held,
        }
    }

    fn frame(&mut self, key: FrameKey) -> &Canvas {
        if self.frames.len() >= FRAME_CACHE_LIMIT {
            self.frames.clear();
        }
        let (appearance, dress, reduce_motion) = (&self.appearance, self.dress, self.reduce_motion);
        self.frames.entry(key).or_insert_with(|| {
            CreatureRenderer::render_composited_frame_holding(
                appearance,
                dress,
                key.clip,
                key.frame,
                key.facing_right,
                reduce_motion,
                FaceRenderState {
                    expression: key.expression,
                    eyelids: key.eyelids,
                    gaze: key.gaze,
                },
                match key.held {
                    HeldKey::Clip => Holding::AsClip,
                    HeldKey::Nothing => Holding::Nothing,
                    HeldKey::Thing(thing) => Holding::Thing(thing),
                },
            )
        })
    }

    /// Where, in the scene, the middle of what of the room's it has in its paws goes at `now`:
    /// `None` unless its pose holds something of the room's and its frame holds anything.
    pub fn room_hold(&mut self, view: &View, now: f32) -> Option<(i32, i32)> {
        if self.pose.hold != Hold::Room || self.carrying.is_none() || self.walking() {
            return None;
        }
        let key = self.key(now);
        let (appearance, reduce_motion) = (&self.appearance, self.reduce_motion);
        let point = *self
            .holds
            .entry((key.clip, key.frame, key.facing_right))
            .or_insert_with(|| {
                CreatureRenderer::held_point(
                    appearance,
                    key.clip,
                    key.frame,
                    key.facing_right,
                    reduce_motion,
                )
            });
        let point = point?;
        let (x, y) = self.origin(view);
        Some((x + point.x, y + point.y))
    }

    /// What of the room's it has in its paws just now, if anything.
    pub fn carried(&self) -> Option<&Carrying> {
        (self.pose.hold == Hold::Room && !self.walking())
            .then_some(self.carrying.as_ref())
            .flatten()
    }

    /// Where its frame's top-left goes in the scene.
    fn origin(&self, view: &View) -> (i32, i32) {
        let (sx, sy) = view.screen(self.pos.0, self.pos.1);
        (
            sx.round() as i32 - FRAME_SIZE as i32 / 2,
            (sy - self.lift).round() as i32 - self.foot_row,
        )
    }

    /// Its feet on screen, before any lift.
    pub fn feet(&self, view: &View) -> (i32, i32) {
        view.pixel(self.pos.0, self.pos.1)
    }

    /// The opaque part of what it shows at `now`, in scene pixels, inclusive.
    pub fn bounds(&mut self, view: &View, now: f32) -> (i32, i32, i32, i32) {
        let key = self.key(now);
        let (x, y) = self.origin(view);
        let size = FRAME_SIZE as i32;
        let (l, t, r, b) = self
            .frame(key)
            .alpha_bounds()
            .map_or((0, 0, size - 1, size - 1), |(a, b, c, d)| {
                (a as i32, b as i32, c as i32, d as i32)
            });
        (x + l, y + t, x + r, y + b)
    }

    /// Whether a point of the scene is on it.
    pub fn covers(&mut self, view: &View, now: f32, point: (i32, i32)) -> bool {
        let key = self.key(now);
        let (x, y) = self.origin(view);
        self.frame(key).get(point.0 - x, point.1 - y).a > 40
    }

    /// Its shadow on the floor, under its feet. Nothing when it is up on something.
    pub fn draw_shadow(&self, canvas: &mut Canvas, view: &View) {
        if self.lift > 0.5 {
            return;
        }
        let (fx, fy) = self.feet(view);
        paint::ellipse(canvas, fx, fy, 8, 3, paint::rgba(0x2a1a14, 60));
    }

    pub fn draw(&mut self, canvas: &mut Canvas, view: &View, now: f32, opacity: u8) {
        let key = self.key(now);
        let (x, y) = self.origin(view);
        paint::blit_faded(canvas, self.frame(key), x, y, opacity);
    }

    /// How it is drawn at `now`, and where. Out of the house it is not drawn at all, unless it
    /// is `shown` all the same, as pointed at.
    pub fn look(&self, view: &View, now: f32, shown: bool) -> Look {
        let drawn = !self.hidden || shown;
        Look {
            id: self.id,
            hidden: self.hidden,
            pos: self.pos,
            lift: self.lift,
            on_piece: self.on_piece,
            dress: self.dress,
            frame: drawn.then(|| self.key(now)),
            origin: if drawn { self.origin(view) } else { (0, 0) },
            cue: self
                .pose
                .cue
                .filter(|_| drawn && !self.walking())
                .map(|cue| {
                    let (bob, drift) =
                        crate::art::cues::motion(cue, self.held_for(now), self.reduce_motion);
                    (cue, bob, drift)
                }),
        }
    }

    /// What it is saying just now, and how far its bubble has grown: what its pose says, or its
    /// sign said as a bubble, for a moment as the pose begins and again now and then while it
    /// lasts. Nothing while it walks.
    pub fn saying(&self, now: f32) -> Option<(Saying, BubbleGrowth)> {
        if self.walking() || self.hidden {
            return None;
        }
        let (saying, after) = self
            .pose
            .say
            .or_else(|| self.pose.cue.map(|cue| (cue.saying(), 0.0)))?;
        let into = self.held_for(now) - after;
        if into < 0.0 {
            return None;
        }
        let age = into % SAYING_AGAIN_SECS;
        if age >= SAYING_SECS {
            return None;
        }
        if self.reduce_motion {
            return Some((saying, BubbleGrowth::Full));
        }
        let edge = age.min(SAYING_SECS - age);
        let growth = if edge < SAYING_STEP_SECS {
            BubbleGrowth::Small
        } else if edge < 2.0 * SAYING_STEP_SECS {
            BubbleGrowth::Medium
        } else {
            BubbleGrowth::Full
        };
        Some((saying, growth))
    }
}

fn frame_of(clip: BodyClip, elapsed: f32, still: bool) -> u8 {
    if still {
        0
    } else {
        AnimationSpec::for_clip(clip).frame_at(elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::household::Household;
    use formiga_home_contract::sample;

    fn actor(reduce_motion: bool) -> Actor {
        let household = Household::new(sample::snapshot()).unwrap();
        Actor::new(&household.residents[0], (1.5, 1.5), reduce_motion)
    }

    #[test]
    fn it_walks_at_its_own_pace_and_faces_the_way_it_goes_across_the_screen() {
        let mut actor = actor(false);
        actor.walk([(4.5, 1.5)]);
        let arrived = actor.advance(0.5);
        assert!(!arrived && actor.walking());
        assert!(
            actor.facing_right,
            "down the room's width is to the right on screen"
        );
        while !actor.advance(0.1) {}
        assert_eq!(actor.pos, (4.5, 1.5));
        actor.walk([(4.5, 5.5)]);
        actor.advance(0.1);
        assert!(
            !actor.facing_right,
            "down the room's depth is to the left on screen"
        );
    }

    #[test]
    fn stopped_on_top_of_a_piece_it_stays_up_there_and_stopped_mid_step_it_is_on_the_floor() {
        let mut actor = actor(false);
        actor.walk_onto([(2.5, 1.5)], 7, (4.5, 1.5), 10.0, true);
        while actor.lift < 10.0 {
            actor.advance(0.02);
        }
        assert!(
            actor.walking() && actor.pos.0 < 4.5,
            "still walking along the top"
        );
        actor.stop();
        assert_eq!((actor.on_piece, actor.lift), (Some(7), 10.0));

        let mut actor = self::actor(false);
        actor.walk_onto([(2.5, 1.5)], 7, (4.5, 1.5), 10.0, true);
        while actor.lift == 0.0 {
            actor.advance(0.02);
        }
        assert!(actor.lift < 10.0, "partway up the step");
        actor.stop();
        assert_eq!((actor.on_piece, actor.lift), (None, 0.0));
    }

    #[test]
    fn with_motion_reduced_a_walk_is_a_cut() {
        let mut actor = actor(true);
        actor.walk([(2.5, 1.5), (6.5, 6.5)]);
        assert!(!actor.walking());
        assert_eq!(actor.pos, (6.5, 6.5));
    }

    #[test]
    fn it_is_drawn_where_its_feet_are_and_can_be_pointed_at() {
        let mut actor = actor(false);
        let view = View::new(8, 8);
        let (l, t, r, b) = actor.bounds(&view, 0.0);
        let (fx, fy) = actor.feet(&view);
        assert!(l < fx && fx < r && t < fy && (b - fy).abs() <= 2);
        assert!(actor.covers(&view, 0.0, ((l + r) / 2, (t + b) / 2)));
    }

    /// A sign is said as a bubble for a moment as the pose begins, and again now and then while
    /// it lasts; an answer waits a moment first; nothing is said on the move.
    #[test]
    fn a_resident_speaks_up_for_a_moment_and_an_answer_waits() {
        use formiga_core::talk::{TalkStyle, TalkTopic};
        let mut actor = actor(false);
        actor.strike(Pose::idle(ExpressionKind::Joy).with_cue(Cue::Heart), 0.0);
        let (saying, _) = actor
            .saying(0.05)
            .expect("a heart is said as the pose begins");
        assert_eq!(saying.style, TalkStyle::Tender);
        assert_eq!(saying.first, TalkTopic::Heart);
        assert_eq!(
            actor.saying(1.0).map(|(_, growth)| growth),
            Some(BubbleGrowth::Full)
        );
        assert!(actor.saying(5.0).is_none(), "and then it is quiet");
        assert!(
            actor.saying(SAYING_AGAIN_SECS + 1.0).is_some(),
            "and says it again"
        );
        actor.strike(
            Pose::idle(ExpressionKind::Joy)
                .answering(Saying::one(TalkStyle::Chat, TalkTopic::Wave)),
            20.0,
        );
        assert!(actor.saying(20.3).is_none(), "an answer waits");
        assert!(actor.saying(21.5).is_some(), "and then comes");
        actor.walk([(4.5, 1.5)]);
        assert!(actor.saying(21.6).is_none(), "nothing is said on the move");
    }
}
