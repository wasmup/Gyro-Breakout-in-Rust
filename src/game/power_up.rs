//! Falling power-ups and their effects.
use super::{
    Ball, Game, PowerUpKind, BALL, HEIGHT, PADDLE_HEIGHT, PADDLE_Y, PIERCE_DURATION, WIDE_DURATION,
};

impl Game {
    pub(super) fn update_power_up(&mut self) {
        if !self.power_up.active || !self.playing {
            return;
        }
        self.power_up.y += 2;
        if self.power_up.y + BALL >= PADDLE_Y
            && self.power_up.y < PADDLE_Y + PADDLE_HEIGHT
            && self.power_up.x + BALL > self.paddle
            && self.power_up.x < self.paddle + self.paddle_width()
        {
            match self.power_up_kind {
                PowerUpKind::Wide => {
                    let old_width = self.paddle_width();
                    self.wide_remaining = WIDE_DURATION;
                    self.resize_paddle(old_width);
                }
                PowerUpKind::Multiball => {
                    let primary = self.ball;
                    let playable = |ball: &Ball| {
                        ball.active && (ball.vy <= 0.0 || ball.y <= (PADDLE_Y - BALL) as f32)
                    };
                    let source = if playable(&primary) {
                        primary
                    } else if let Some(i) = self.extra_balls.iter().position(playable) {
                        let survivor = self.extra_balls[i];
                        self.extra_balls[i].active = false;
                        self.ball = survivor;
                        survivor
                    } else {
                        self.power_up.active = false;
                        return;
                    };
                    for ball in &mut self.extra_balls {
                        if !playable(ball) {
                            ball.active = false;
                        }
                    }
                    for i in 0..self.extra_balls.len() {
                        if !self.extra_balls[i].active {
                            let direction = if i == 0 { -1.0 } else { 1.0 };
                            let mut vx = source.vx + direction * 3.0;
                            while self.extra_balls.iter().any(|ball| {
                                ball.active
                                    && ball.x == source.x
                                    && ball.y == source.y
                                    && ball.vx == vx
                                    && ball.vy == source.vy
                            }) {
                                vx += direction;
                            }
                            self.extra_balls[i] = Ball {
                                active: true,
                                x: source.x,
                                y: source.y,
                                vx,
                                vy: source.vy,
                                contacts: source.contacts,
                            };
                        }
                    }
                }
                PowerUpKind::Pierce => self.pierce_remaining = PIERCE_DURATION,
            }
            self.power_up.active = false;
        } else if self.power_up.y >= HEIGHT {
            self.power_up.active = false;
        }
    }
}
