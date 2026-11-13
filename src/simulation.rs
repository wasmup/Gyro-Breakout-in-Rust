use gyro_breakout::game::{
    Game, BALL, BRICK_HEIGHT, BRICK_PITCH_X, BRICK_PITCH_Y, BRICK_TOP, COLS, PADDLE_HEIGHT,
    PADDLE_Y, ROWS, SOLID, WIDTH,
};
pub struct Controller {
    offset: i32,
    reached_top: bool,
    brick_hits: u32,
    paddle_hits: u32,
    missed: i32,
    shift: usize,
}
impl Controller {
    pub fn new(seed: u32, previous: i32) -> Self {
        let mut v = seed;
        v ^= v >> 16;
        v = v.wrapping_mul(0x7feb352d);
        v ^= v >> 15;
        v = v.wrapping_mul(0x846ca68b);
        v ^= v >> 16;
        let mut offset = (v % 7) as i32 - 3;
        if offset == previous {
            offset += 1;
            if offset > 3 {
                offset = -3;
            }
        }
        Self {
            offset,
            reached_top: false,
            brick_hits: 0,
            paddle_hits: 0,
            missed: 0,
            shift: 0,
        }
    }
    pub fn offset(&self) -> i32 {
        self.offset
    }
    pub fn input(&mut self, g: &Game) -> (i32, bool) {
        if g.brick_hits() != self.brick_hits {
            self.missed = 0;
            self.brick_hits = g.brick_hits();
        }
        if g.paddle_hits() != self.paddle_hits {
            self.paddle_hits = g.paddle_hits();
            self.missed += 1;
            if self.missed >= 2 {
                self.shift = (self.shift + 1) % 5;
                self.missed = 0;
            }
        }
        let (mut ball_x, mut ball_y, mut ball_vx, mut ball_vy) =
            (g.ball().x, g.ball().y, g.ball().vx, g.ball().vy);
        let collision_y = (PADDLE_Y - BALL) as f32;
        let mut descending = ball_vy > 0.0 && ball_y <= collision_y;
        for ball in g
            .extra_balls()
            .iter()
            .filter(|ball| ball.active && ball.vy > 0.0 && ball.y <= collision_y)
        {
            if !descending || ball.y > ball_y {
                (ball_x, ball_y, ball_vx, ball_vy) = (ball.x, ball.y, ball.vx, ball.vy);
                descending = true;
            }
        }
        if g.playing() && !descending && ball_vy > 0.0 && ball_y > collision_y {
            if let Some(ball) = g
                .extra_balls()
                .iter()
                .find(|ball| ball.active && ball.vy <= 0.0)
            {
                (ball_x, ball_y, ball_vx, ball_vy) = (ball.x, ball.y, ball.vx, ball.vy);
            } else {
                ball_x = (g.paddle() + g.paddle_width() / 2 - BALL / 2) as f32;
            }
        }
        let mut impact = ball_x;
        if g.playing() && descending {
            impact += ball_vx * ((PADDLE_Y - BALL) as f32 - ball_y) / ball_vy;
        }
        impact -=
            (impact / (2 * (WIDTH - BALL)) as f32) as i32 as f32 * (2 * (WIDTH - BALL)) as f32;
        if impact < 0.0 {
            impact += (2 * (WIDTH - BALL)) as f32;
        }
        if impact > (WIDTH - BALL) as f32 {
            impact = (2 * (WIDTH - BALL)) as f32 - impact;
        }
        let center = COLS / 2;
        if (0..ROWS).all(|r| g.bricks()[r][center] == 0 || g.bricks()[r][center] == SOLID)
            && g.ball().y + BALL as f32 <= BRICK_TOP as f32
        {
            self.reached_top = true;
        }
        let mut column = center;
        if self.reached_top {
            let mut best = ROWS + 1;
            for c in (0..COLS).rev() {
                let count = (0..ROWS)
                    .filter(|r| (1..SOLID).contains(&g.bricks()[*r][c]))
                    .count();
                if count > 0 && count < best {
                    best = count;
                    column = c;
                }
            }
        }
        let bx = column as i32 * BRICK_PITCH_X + BRICK_PITCH_X / 2 + self.offset;
        let mut by = BRICK_TOP - BALL;
        for r in (0..ROWS).rev() {
            if (1..SOLID).contains(&g.bricks()[r][column]) {
                by = BRICK_TOP + r as i32 * BRICK_PITCH_Y + BRICK_HEIGHT;
                break;
            }
        }
        let mut velocity = (bx as f32 - (impact + 5.0)) * 5.0 / (PADDLE_Y - BALL - by) as f32;
        if self.reached_top {
            if (0.0..4.0).contains(&velocity) {
                velocity = 4.0;
            } else if velocity < 0.0 && velocity > -4.0 {
                velocity = -4.0;
            }
        }
        velocity = velocity.clamp(-8.0, 8.0);
        let mut target = ((impact + 5.0 - (g.paddle_width() / 2) as f32 - 5.0 * velocity) as i32
            + [0, 6, -6, 12, -12][self.shift])
            .clamp(0, WIDTH - g.paddle_width());
        if g.playing() && g.power_up().active && g.power_up().y < PADDLE_Y + PADDLE_HEIGHT {
            let width = g.paddle_width();
            let mut left = (g.power_up().x - width + 1).max(0);
            let mut right = (g.power_up().x + BALL - 1).min(WIDTH - width);
            let pickup_frames = ((PADDLE_Y - BALL - g.power_up().y).max(0) + 1) / 2;
            let ball_frames = if descending {
                ((PADDLE_Y - BALL) as f32 - ball_y).max(0.0) / (ball_vy * g.speed() as f32 * 0.1)
            } else {
                f32::INFINITY
            };
            // If a ball arrives around the same time, only pursue positions that catch both.
            if ball_frames <= pickup_frames as f32 + 5.0 {
                left = left.max(impact as i32 - width + 1);
                right = right.min(impact as i32 + BALL - 1);
            }
            if left <= right {
                target = target.clamp(left, right);
            }
        }
        let mut gain = (target - g.paddle()) * 4 - g.paddle_fraction() - 2 * g.paddle_velocity();
        if gain > 20 {
            gain = (gain + 20) / 2;
        } else if gain < -20 {
            gain = (gain - 20) / 2;
        }
        (-gain.clamp(-100, 100), !g.playing() && !g.previous_button())
    }
}
