//! Platform-independent game rules. Time is expressed in 168 MHz CPU cycles.
mod physics;
mod power_up;
pub const WIDTH: i32 = 320;
pub const HEIGHT: i32 = 240;
pub const PADDLE_WIDTH: i32 = 96;
pub const PADDLE_MAX_SPEED: i32 = 32; // Quarter-pixels per frame.
const PADDLE_ACCELERATION: i32 = 4;
pub const WIDE_PADDLE_WIDTH: i32 = 144;
pub const WIDE_DURATION: u32 = 10 * CYCLES_PER_SECOND;
pub const PIERCE_DURATION: u32 = 10 * CYCLES_PER_SECOND;
pub const PADDLE_HEIGHT: i32 = 7;
pub const PADDLE_Y: i32 = HEIGHT - PADDLE_HEIGHT;
pub const BALL: i32 = 10;
pub const ROWS: usize = 5;
pub const COLS: usize = 8;
pub const SHOTS: usize = 16;
pub const SHATTER_FRAMES: u8 = 8;
pub const BRICK_TOP: i32 = 35;
pub const BRICK_PITCH_X: i32 = 40;
pub const BRICK_PITCH_Y: i32 = 15;
pub const BRICK_WIDTH: i32 = 38;
pub const BRICK_HEIGHT: i32 = 12;
const PLAY_TOP: i32 = 18;
pub const CYCLES_PER_SECOND: u32 = 168_000_000;
/// Special brick values; ordinary nonzero values are remaining hit points.
pub const BOMB: u8 = u8::MAX - 1;
pub const SOLID: u8 = u8::MAX;

#[derive(Clone, Copy, Default)]
pub struct Shot {
    pub active: bool,
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Default)]
pub struct Ball {
    pub active: bool,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub contacts: u64,
}
/// A short-lived visual effect at a ball/brick impact.
#[derive(Clone, Copy, Default)]
pub struct Shatter {
    pub x: i32,
    pub y: i32,
    pub frames: u8,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum PowerUpKind {
    #[default]
    Wide,
    Multiball,
    Pierce,
}
/// Game state can be inspected through accessors and updated through [`Game::step`].
///
/// ```compile_fail
/// use gyro_breakout::game::Game;
/// let mut game = Game::new(1, 0, false);
/// game.bricks[0][0] = 0;
/// ```
#[derive(Clone, Copy)]
pub struct Game {
    bricks: [[u8; COLS]; ROWS],
    remaining: usize,
    bonus: usize,
    lives: i32,
    paddle: i32,
    power_up: Shot,
    power_up_kind: PowerUpKind,
    ball: Ball,
    extra_balls: [Ball; 2],
    shatters: [Shatter; 3],
    lost_ball: Option<(i32, i32)>,
    wide_remaining: u32,
    pierce_remaining: u32,
    last_update: u32,
    drop_seed: u32,
    fraction: i32,
    paddle_velocity: i32,
    playing: bool,
    finished: bool,
    speed: i32,
    brick_hits: u32,
    paddle_hits: u32,
    ready: bool,
    previous_button: bool,
    last_gesture: u32,
    last_fire: u32,
    shots: [Shot; SHOTS],
}
fn mix(mut v: u32) -> u32 {
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846ca68b);
    v ^ (v >> 16)
}
impl Game {
    pub fn bricks(&self) -> &[[u8; COLS]; ROWS] {
        &self.bricks
    }
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    pub fn bonus(&self) -> usize {
        self.bonus
    }
    pub fn lives(&self) -> i32 {
        self.lives
    }
    pub fn paddle(&self) -> i32 {
        self.paddle
    }
    pub fn power_up(&self) -> &Shot {
        &self.power_up
    }
    pub fn power_up_kind(&self) -> PowerUpKind {
        self.power_up_kind
    }
    pub fn ball(&self) -> &Ball {
        &self.ball
    }
    pub fn extra_balls(&self) -> &[Ball; 2] {
        &self.extra_balls
    }
    pub fn shatters(&self) -> &[Shatter; 3] {
        &self.shatters
    }
    pub fn lost_ball(&self) -> Option<(i32, i32)> {
        self.lost_ball
    }
    pub fn playing(&self) -> bool {
        self.playing
    }
    pub fn finished(&self) -> bool {
        self.finished
    }
    pub fn speed(&self) -> i32 {
        self.speed
    }
    pub fn brick_hits(&self) -> u32 {
        self.brick_hits
    }
    pub fn paddle_hits(&self) -> u32 {
        self.paddle_hits
    }
    pub fn shots(&self) -> &[Shot; SHOTS] {
        &self.shots
    }
    /// Set the initial simulation speed within the same limits as player gestures.
    pub fn set_speed(&mut self, speed: i32) {
        self.speed = speed.clamp(5, 13);
    }
    pub fn new(seed: u32, now: u32, pressed: bool) -> Self {
        let random = mix(seed ^ now);
        let bonus = (random % (ROWS * COLS) as u32) as usize;
        let anchor = mix(random ^ 0xa5a5_5a5a) as usize % COLS;
        let mut bricks = [[2; COLS]; ROWS];
        let mut remaining = ROWS * COLS;
        let mut previous_solid = false;
        // Placement is column-oriented, but the brick grid is stored by row.
        #[allow(clippy::needless_range_loop)]
        for col in 0..COLS {
            let choice = mix(random.wrapping_add(col as u32 + 1));
            let solid =
                col == anchor || (!previous_solid && col.abs_diff(anchor) > 1 && choice & 1 == 0);
            if solid {
                let mut row = (choice as usize >> 1) % (ROWS - 1);
                if col != bonus % COLS {
                    row = (choice as usize >> 1) % ROWS;
                } else if row >= bonus / COLS {
                    row += 1;
                }
                bricks[row][col] = SOLID;
                remaining -= 1;
            }
            previous_solid = solid;
        }
        let mut bomb_choice = mix(random ^ 0x671a_4b9d) as usize % (remaining - 1);
        'place_bomb: for (row, bricks_row) in bricks.iter_mut().enumerate() {
            for (col, brick) in bricks_row.iter_mut().enumerate() {
                if *brick == SOLID || row * COLS + col == bonus {
                    continue;
                }
                if bomb_choice == 0 {
                    *brick = BOMB;
                    break 'place_bomb;
                }
                bomb_choice -= 1;
            }
        }
        Self {
            bricks,
            remaining,
            bonus,
            lives: 5,
            paddle: (WIDTH - PADDLE_WIDTH) / 2,
            power_up: Shot::default(),
            power_up_kind: PowerUpKind::Wide,
            ball: Ball {
                active: true,
                x: 155.0,
                y: (PADDLE_Y - BALL) as f32,
                vx: 3.0,
                vy: -5.0,
                contacts: 0,
            },
            extra_balls: [Ball::default(); 2],
            shatters: [Shatter::default(); 3],
            lost_ball: None,
            wide_remaining: 0,
            pierce_remaining: 0,
            last_update: now,
            drop_seed: mix(random ^ 0x7a91_b42d),
            fraction: 0,
            paddle_velocity: 0,
            playing: false,
            finished: false,
            speed: 5,
            brick_hits: 0,
            paddle_hits: 0,
            ready: true,
            previous_button: pressed,
            last_gesture: now.wrapping_sub(CYCLES_PER_SECOND),
            last_fire: now,
            shots: [Shot::default(); SHOTS],
        }
    }
    pub fn paddle_fraction(&self) -> i32 {
        self.fraction
    }
    pub fn paddle_velocity(&self) -> i32 {
        self.paddle_velocity
    }
    pub fn paddle_width(&self) -> i32 {
        if self.wide_remaining > 0 {
            WIDE_PADDLE_WIDTH
        } else {
            PADDLE_WIDTH
        }
    }
    pub fn wide_seconds(&self) -> u32 {
        self.wide_remaining.div_ceil(CYCLES_PER_SECOND)
    }
    pub fn pierce_seconds(&self) -> u32 {
        self.pierce_remaining.div_ceil(CYCLES_PER_SECOND)
    }
    fn resize_paddle(&mut self, old_width: i32) {
        self.paddle = (self.paddle + (old_width - self.paddle_width()) / 2)
            .clamp(0, WIDTH - self.paddle_width());
    }
    pub fn previous_button(&self) -> bool {
        self.previous_button
    }
    fn hit(&mut self, row: usize, col: usize, ball: bool) {
        if self.bricks[row][col] == BOMB {
            self.bricks[row][col] = 1;
            self.hit(row, col, ball);
            let mut destroyed = 0;
            for r in row.saturating_sub(1)..=(row + 1).min(ROWS - 1) {
                for c in col.saturating_sub(1)..=(col + 1).min(COLS - 1) {
                    if self.bricks[r][c] != 0 && self.bricks[r][c] != SOLID {
                        self.bricks[r][c] = 1;
                        self.hit(r, c, false);
                        destroyed += 1;
                        if destroyed == 2 {
                            return;
                        }
                    }
                }
            }
            return;
        }
        if self.bricks[row][col] == SOLID {
            return;
        }
        self.brick_hits += 1;
        self.bricks[row][col] -= 1;
        if self.bricks[row][col] == 0 {
            self.remaining -= 1;
            if !self.power_up.active
                && mix(self.drop_seed ^ (row * COLS + col) as u32).is_multiple_of(5)
            {
                self.power_up = Shot {
                    active: true,
                    x: col as i32 * BRICK_PITCH_X + 15,
                    y: BRICK_TOP + row as i32 * BRICK_PITCH_Y,
                };
                self.power_up_kind =
                    match mix(self.drop_seed ^ 0x4295_1023 ^ (row * COLS + col) as u32) % 3 {
                        0 => PowerUpKind::Wide,
                        1 => PowerUpKind::Multiball,
                        _ => PowerUpKind::Pierce,
                    };
            }
            if ball && row * COLS + col == self.bonus {
                self.lives += 2;
            }
        }
    }
    fn shatter(&mut self, x: f32, y: f32) {
        if let Some(effect) = self.shatters.iter_mut().find(|effect| effect.frames == 0) {
            *effect = Shatter {
                x: x as i32 + BALL / 2,
                y: y as i32 + BALL / 2,
                frames: SHATTER_FRAMES,
            };
        }
    }
    pub fn step(&mut self, rate: i32, speed_rate: i32, pressed: bool, now: u32) {
        for effect in &mut self.shatters {
            effect.frames = effect.frames.saturating_sub(1);
        }
        if self.finished {
            return;
        }
        self.lost_ball = None;
        let old_width = self.paddle_width();
        let elapsed = now.wrapping_sub(self.last_update);
        self.last_update = now;
        self.wide_remaining = self.wide_remaining.saturating_sub(elapsed);
        self.pierce_remaining = self.pierce_remaining.saturating_sub(elapsed);
        if self.paddle_width() != old_width {
            self.resize_paddle(old_width);
        }
        let mut gain = if (-2..=2).contains(&rate) {
            0
        } else {
            rate.clamp(-100, 100)
        };
        if gain > 20 {
            gain = 2 * gain - 20;
        } else if gain < -20 {
            gain = 2 * gain + 20;
        }
        let desired_velocity = (-gain).clamp(-PADDLE_MAX_SPEED, PADDLE_MAX_SPEED);
        self.paddle_velocity += (desired_velocity - self.paddle_velocity)
            .clamp(-PADDLE_ACCELERATION, PADDLE_ACCELERATION);
        self.fraction += self.paddle_velocity;
        self.paddle = (self.paddle + self.fraction / 4).clamp(0, WIDTH - self.paddle_width());
        self.fraction %= 4;
        if (self.paddle == 0 && self.paddle_velocity < 0)
            || (self.paddle == WIDTH - self.paddle_width() && self.paddle_velocity > 0)
        {
            self.fraction = 0;
            self.paddle_velocity = 0;
        }
        if (-10..=10).contains(&speed_rate) {
            self.ready = true;
        }
        if self.ready && speed_rate.abs() >= 40 {
            self.ready = false;
            if now.wrapping_sub(self.last_gesture) >= CYCLES_PER_SECOND {
                self.speed = (self.speed + if speed_rate < 0 { 1 } else { -1 }).clamp(5, 13);
                self.last_gesture = now;
            }
        }
        if pressed && !self.previous_button && !self.playing {
            self.playing = true;
            self.last_fire = now;
        } else if pressed
            && self.playing
            && (!self.previous_button || now.wrapping_sub(self.last_fire) >= CYCLES_PER_SECOND / 5)
        {
            let paddle_width = self.paddle_width();
            for pair in self.shots.as_chunks_mut::<2>().0 {
                if !pair[0].active && !pair[1].active {
                    pair[0] = Shot {
                        active: true,
                        x: self.paddle,
                        y: PADDLE_Y - BALL,
                    };
                    pair[1] = Shot {
                        active: true,
                        x: self.paddle + paddle_width - BALL,
                        y: PADDLE_Y - BALL,
                    };
                    self.last_fire = now;
                    break;
                }
            }
        }
        self.previous_button = pressed;
        self.update_balls();
        self.update_shots();
        self.update_power_up();
        if self.remaining == 0 {
            self.finished = true;
        }
    }
    fn update_shots(&mut self) {
        for i in 0..SHOTS {
            if !self.shots[i].active {
                continue;
            }
            let old_y = self.shots[i].y;
            self.shots[i].y -= 8;
            let shot = self.shots[i];
            if !self.playing || shot.y < PLAY_TOP {
                self.shots[i].active = false;
                continue;
            }
            'grid: for r in (0..ROWS).rev() {
                for c in 0..COLS {
                    let (x, y) = (
                        c as i32 * BRICK_PITCH_X + 1,
                        BRICK_TOP + r as i32 * BRICK_PITCH_Y,
                    );
                    if self.bricks[r][c] > 0
                        && shot.x + BALL > x
                        && shot.x < x + BRICK_WIDTH
                        && shot.y < y + BRICK_HEIGHT
                        && old_y + BALL > y
                    {
                        self.hit(r, c, false);
                        self.shots[i].active = false;
                        break 'grid;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "gameplay_tests.rs"]
mod gameplay_tests;
#[cfg(test)]
#[path = "simulation_tests.rs"]
mod simulation_tests;
