//! Ball motion and collision handling.
use super::{
    Ball, Game, BALL, BRICK_HEIGHT, BRICK_PITCH_X, BRICK_PITCH_Y, BRICK_TOP, BRICK_WIDTH, COLS,
    HEIGHT, PADDLE_Y, PLAY_TOP, ROWS, SOLID, WIDTH,
};

impl Game {
    pub(super) fn update_balls(&mut self) {
        if !self.playing {
            self.ball.x = (self.paddle + (self.paddle_width() - BALL) / 2) as f32;
            self.ball.y = (PADDLE_Y - BALL) as f32;
            return;
        }
        let mut primary = self.ball;
        let primary_lost = self.advance_ball(&mut primary);
        self.ball = primary;
        let mut last_lost = (primary.x, primary.y);
        for i in 0..self.extra_balls.len() {
            let mut ball = self.extra_balls[i];
            if !ball.active {
                continue;
            }
            let lost = self.advance_ball(&mut ball);
            if lost {
                last_lost = (ball.x, ball.y);
            }
            ball.active = !lost;
            self.extra_balls[i] = ball;
        }
        if primary_lost {
            if let Some(ball) = self.extra_balls.iter_mut().find(|ball| ball.active) {
                // Field copies avoid linking a large memmove routine on the target.
                self.ball.x = ball.x;
                self.ball.y = ball.y;
                self.ball.vx = ball.vx;
                self.ball.vy = ball.vy;
                self.ball.contacts = ball.contacts;
                ball.active = false;
            } else {
                self.lost_ball = Some((
                    (last_lost.0 as i32).clamp(0, WIDTH - BALL),
                    (last_lost.1 as i32).clamp(0, HEIGHT - BALL),
                ));
                self.lives -= 1;
                self.playing = false;
                let old_width = self.paddle_width();
                self.wide_remaining = 0;
                self.pierce_remaining = 0;
                self.ball.contacts = 0;
                self.resize_paddle(old_width);
                self.power_up.active = false;
                self.ball.x = (self.paddle + 43) as f32;
                self.ball.y = (PADDLE_Y - BALL) as f32;
                self.ball.vx = 3.0;
                self.ball.vy = -5.0;
                if self.lives == 0 {
                    self.finished = true;
                }
            }
        }
    }
    fn advance_ball(&mut self, ball: &mut Ball) -> bool {
        let scale = self.speed as f32 * 0.1;
        let steps = (ball.vx.abs().max(ball.vy.abs()) * scale / 2.0) as i32 + 1;
        let mut hits = 0;
        for _ in 0..steps {
            if hits >= 4 {
                break;
            }
            let (px, py) = (ball.x, ball.y);
            if ball.contacts != 0 {
                for r in 0..ROWS {
                    for c in 0..COLS {
                        let x = (c as i32 * BRICK_PITCH_X + 1) as f32;
                        let y = (BRICK_TOP + r as i32 * BRICK_PITCH_Y) as f32;
                        if px + BALL as f32 <= x
                            || px >= x + BRICK_WIDTH as f32
                            || py + BALL as f32 <= y
                            || py >= y + BRICK_HEIGHT as f32
                        {
                            ball.contacts &= !(1u64 << (r * COLS + c));
                        }
                    }
                }
            }
            ball.x += ball.vx * scale / steps as f32;
            ball.y += ball.vy * scale / steps as f32;
            Self::walls(ball);
            self.resolve_paddle_collision(ball, px, py, scale, steps);
            self.resolve_brick_collisions(ball, px, py, &mut hits);
        }
        ball.y >= HEIGHT as f32
    }
    fn resolve_paddle_collision(
        &mut self,
        ball: &mut Ball,
        px: f32,
        py: f32,
        scale: f32,
        steps: i32,
    ) {
        if ball.vy > 0.0
            && py + BALL as f32 <= PADDLE_Y as f32
            && ball.y + BALL as f32 >= PADDLE_Y as f32
            && ball.y > py
        {
            let f = (PADDLE_Y as f32 - BALL as f32 - py) / (ball.y - py);
            let ix = px + (ball.x - px) * f;
            if ix + BALL as f32 > self.paddle as f32
                && ix < (self.paddle + self.paddle_width()) as f32
            {
                self.paddle_hits += 1;
                ball.vx = (ix + 5.0 - (self.paddle + self.paddle_width() / 2) as f32) / 5.0;
                ball.vy = -5.0;
                ball.x = ix + ball.vx * scale / steps as f32 * (1.0 - f);
                ball.y = (PADDLE_Y - BALL) as f32 + ball.vy * scale / steps as f32 * (1.0 - f);
                Self::walls(ball);
            }
        }
    }
    fn resolve_brick_collisions(
        &mut self,
        ball: &mut Ball,
        mut px: f32,
        mut py: f32,
        hits: &mut i32,
    ) {
        while *hits < 4 {
            let (dx, dy) = (ball.x - px, ball.y - py);
            let mut first = 2.0f32;
            let mut found = None;
            for r in 0..ROWS {
                for c in 0..COLS {
                    let (x, y) = (
                        (c as i32 * BRICK_PITCH_X + 1) as f32,
                        (BRICK_TOP + r as i32 * BRICK_PITCH_Y) as f32,
                    );
                    if self.bricks[r][c] == 0
                        || (self.pierce_remaining > 0
                            && ball.contacts & (1u64 << (r * COLS + c)) != 0)
                        || ball.x + BALL as f32 <= x
                        || ball.x >= x + BRICK_WIDTH as f32
                        || ball.y + BALL as f32 <= y
                        || ball.y >= y + BRICK_HEIGHT as f32
                    {
                        continue;
                    }
                    let ex = if dx > 0.0 && px + BALL as f32 <= x {
                        (x - px - BALL as f32) / dx
                    } else if dx < 0.0 && px >= x + BRICK_WIDTH as f32 {
                        (x + BRICK_WIDTH as f32 - px) / dx
                    } else {
                        -1.0
                    };
                    let ey = if dy > 0.0 && py + BALL as f32 <= y {
                        (y - py - BALL as f32) / dy
                    } else if dy < 0.0 && py >= y + BRICK_HEIGHT as f32 {
                        (y + BRICK_HEIGHT as f32 - py) / dy
                    } else {
                        -1.0
                    };
                    let entry = ex.max(ey).max(0.0);
                    if entry < first {
                        first = entry;
                        found = Some((r, c, ex, ey));
                    }
                }
            }
            let Some((r, c, ex, ey)) = found else { break };
            let (ix, iy) = (px + dx * first, py + dy * first);
            let pierce = self.pierce_remaining > 0 && self.bricks[r][c] != SOLID;
            self.shatter(ix, iy);
            self.hit(r, c, true);
            if pierce {
                ball.contacts |= 1u64 << (r * COLS + c);
                continue;
            }
            if ex >= 0.0 && ex > ey {
                ball.x = px + dx * (2.0 * ex - 1.0);
                ball.vx = -ball.vx;
            } else if ey >= 0.0 {
                ball.y = py + dy * (2.0 * ey - 1.0);
                ball.vy = -ball.vy;
            } else {
                ball.y = py;
                ball.vy = -ball.vy;
            }
            *hits += 1;
            px = ix;
            py = iy;
            Self::walls(ball);
        }
    }
    fn walls(ball: &mut Ball) {
        if ball.x < 0.0 {
            ball.x = -ball.x;
            ball.vx = -ball.vx;
        }
        if ball.x > (WIDTH - BALL) as f32 {
            ball.x = (2 * (WIDTH - BALL)) as f32 - ball.x;
            ball.vx = -ball.vx;
        }
        if ball.y < PLAY_TOP as f32 {
            ball.y = (2 * PLAY_TOP) as f32 - ball.y;
            ball.vy = -ball.vy;
        }
    }
}
