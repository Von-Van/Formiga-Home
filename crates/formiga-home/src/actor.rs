//! A resident in the room: where it stands on the floor, which way it faces, what its body is
//! doing, and the frames `formiga-art` draws for it, face and all.
//!
//! The room is isometric and the creatures are not: each is drawn upright, anchored by its feet,
//! and faces left or right by which way it is going across the screen. What it does is decided by
//! the household's simulation; the actor only walks where it is sent and holds the pose it is given.

use crate::art::cues::Cue;
use crate::household::{Id, Resident};
use crate::iso::View;
use crate::paint;
use formiga_art::{
    AccessoryArt, AnimationSpec, BodyClip, Canvas, CreatureRenderer, ExpressionKind, EyelidPose,
    FRAME_SIZE, FaceRenderState, GazeDirection,
};
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
}

impl Pose {
    pub fn new(clip: impl Into<BodyClip>, expression: ExpressionKind) -> Self {
        Self {
            clip: clip.into(),
            expression,
            held: false,
            cue: None,
            spin: false,
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

    #[cfg(test)]
    pub fn asleep(&self) -> bool {
        self.clip == BodyClip::Action(ActionKind::Sleep)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FrameKey {
    clip: BodyClip,
    frame: u8,
    facing_right: bool,
    expression: ExpressionKind,
    eyelids: EyelidPose,
    gaze: GazeDirection,
}

/// Frames are drawn on first use and kept; this many is a long visit's worth.
const FRAME_CACHE_LIMIT: usize = 1200;
const SPIN_TURN: f32 = 0.22;

pub struct Actor {
    pub id: Id,
    appearance: AppearanceGenome,
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
    speed: f32,
    walk_face: ExpressionKind,
    frames: HashMap<FrameKey, Canvas>,
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
            speed: resident.character.walk_speed(),
            walk_face: resident.character.walk_face(),
            frames: HashMap::new(),
            blink: (3.2 + seed * 2.4, seed * 5.0),
            reduce_motion,
        }
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

    /// Set off along `waypoints`, getting down from wherever it was first. With motion reduced a
    /// walk is a cut: it is simply there, facing the way it went.
    pub fn walk(&mut self, waypoints: impl IntoIterator<Item = (f32, f32)>) {
        self.get_down();
        self.path.clear();
        self.path.extend(waypoints);
        if self.reduce_motion
            && let Some(last) = self.path.back().copied()
        {
            self.face_towards(last);
            self.pos = last;
            self.path.clear();
        }
    }

    pub fn stop(&mut self) {
        self.path.clear();
    }

    pub fn walking(&self) -> bool {
        !self.path.is_empty()
    }

    /// Where it is headed, or where it is.
    pub fn destination(&self) -> (f32, f32) {
        self.path.back().copied().unwrap_or(self.pos)
    }

    /// Sit or lie on a piece: on its seat, lifted to its height, facing the way the piece does.
    pub fn settle_on(&mut self, piece: u16, seat: (f32, f32), lift: f32, facing_right: bool) {
        self.path.clear();
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
            } else {
                self.pos.0 += dx / distance * stride;
                self.pos.1 += dy / distance * stride;
                return false;
            }
        }
        true
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
        let eyelids = if asleep {
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
        }
    }

    fn frame(&mut self, key: FrameKey) -> &Canvas {
        if self.frames.len() >= FRAME_CACHE_LIMIT {
            self.frames.clear();
        }
        let (appearance, dress, reduce_motion) = (&self.appearance, self.dress, self.reduce_motion);
        self.frames.entry(key).or_insert_with(|| {
            CreatureRenderer::render_dressed_composited_frame(
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
            )
        })
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
        let frame = self.frame(key);
        paint::blit_faded(canvas, &frame.clone(), x, y, opacity);
    }

    /// The cue over its head, if its pose has one.
    pub fn draw_cue(&mut self, canvas: &mut Canvas, view: &View, now: f32) {
        let Some(cue) = self.pose.cue.filter(|_| !self.walking()) else {
            return;
        };
        let (l, t, r, _) = self.bounds(view, now);
        crate::art::cues::draw(
            canvas,
            cue,
            (l + r) / 2,
            t - 2,
            self.held_for(now),
            self.reduce_motion,
        );
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
}
