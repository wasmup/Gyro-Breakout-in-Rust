use gyro_breakout::game::{
    Ball, Game, PowerUpKind, Shot, BALL, BOMB, COLS, CYCLES_PER_SECOND, HEIGHT, PADDLE_WIDTH,
    PADDLE_Y, PIERCE_DURATION, ROWS, SOLID, WIDE_DURATION, WIDE_PADDLE_WIDTH, WIDTH,
};
use gyro_breakout::render::{self, Display, BLACK, CYAN, WHITE, YELLOW};
use std::vec::Vec;

#[test]
fn round_ball_fits_collision_box_and_has_clear_corners() {
    struct Pixels([[u16; 10]; 10]);
    impl Display for Pixels {
        fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u16) {
            for row in y..y + h {
                for col in x..x + w {
                    assert!((0..10).contains(&row) && (0..10).contains(&col));
                    self.0[row as usize][col as usize] = color;
                }
            }
        }
    }
    let mut pixels = Pixels([[BLACK; 10]; 10]);
    render::ball(&mut pixels, 0, 0, WHITE);
    assert_eq!(pixels.0[0][0], BLACK);
    assert_eq!(pixels.0[0][9], BLACK);
    assert_eq!(pixels.0[9][0], BLACK);
    assert_eq!(pixels.0[9][9], BLACK);
    assert_eq!(pixels.0[5][5], WHITE);
    for y in 0..10 {
        for x in 0..10 {
            assert_eq!(pixels.0[y][x], pixels.0[9 - y][x]);
            assert_eq!(pixels.0[y][x], pixels.0[y][9 - x]);
        }
    }
}

#[test]
fn brick_impact_shatters_briefly_and_clears_its_particles() {
    #[derive(Default)]
    struct Recorder(Vec<(i32, i32, i32, i32, u16)>);
    impl Display for Recorder {
        fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u16) {
            self.0.push((x, y, w, h, color));
        }
    }
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = 2;
    g.remaining = 1;
    g.playing = true;
    g.ball.x = 5.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    let before = g;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 1);
    assert_eq!(
        g.shatters()
            .iter()
            .filter(|effect| effect.frames > 0)
            .count(),
        1
    );
    let mut display = Recorder::default();
    render::frame(&mut display, &before, &g);
    assert!(display
        .0
        .iter()
        .any(|&(_, _, w, h, color)| w == 2 && h == 2 && color == CYAN));
    for tick in 1..=u32::from(super::SHATTER_FRAMES) {
        let old = g;
        g.step(0, 0, false, tick);
        display.0.clear();
        render::frame(&mut display, &old, &g);
        if tick == u32::from(super::SHATTER_FRAMES) {
            assert!(g.shatters().iter().all(|effect| effect.frames == 0));
            assert!(display
                .0
                .iter()
                .any(|&(_, _, w, h, color)| w == 2 && h == 2 && color == BLACK));
        }
    }
}

#[test]
fn final_brick_effect_can_finish_after_the_round_ends() {
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = 1;
    g.remaining = 1;
    g.playing = true;
    g.ball.x = 5.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 0);
    assert!(g.finished);
    assert!(g.shatters().iter().any(|effect| effect.frames > 0));
    let position = (g.ball.x, g.ball.y);
    for tick in 1..=u32::from(super::SHATTER_FRAMES) {
        g.step(0, 0, false, tick);
    }
    assert!(g.shatters().iter().all(|effect| effect.frames == 0));
    assert_eq!((g.ball.x, g.ball.y), position);
}

#[test]
fn ball_bounces_from_a_brick_corner_without_tunneling() {
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[1][1] = 2;
    g.remaining = 1;
    g.playing = true;
    g.ball.x = 30.0;
    g.ball.y = 39.0;
    g.ball.vx = 5.0;
    g.ball.vy = 5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[1][1], 1);
    assert!(g.ball.vx < 0.0 || g.ball.vy < 0.0);
}

#[test]
fn ball_at_adjacent_solid_and_destructible_bricks_keeps_invariants() {
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = SOLID;
    g.bricks[0][1] = 2;
    g.remaining = 1;
    g.playing = true;
    g.ball.x = 35.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], SOLID);
    assert!(g.bricks[0][1] == 1 || g.bricks[0][1] == 2);
    assert_eq!(g.remaining, 1);
    assert!(g.ball.vy > 0.0);
}

#[test]
fn two_balls_can_damage_the_same_brick_in_one_step() {
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = 2;
    g.remaining = 1;
    g.bonus = 1;
    g.playing = true;
    g.ball.x = 5.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.extra_balls[0] = Ball {
        active: true,
        x: 20.0,
        y: 47.0,
        vx: 0.0,
        vy: -5.0,
        contacts: 0,
    };
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 0);
    assert_eq!(g.remaining, 0);
    assert!(g.finished);
    assert!(g.ball.vy > 0.0 && g.extra_balls[0].vy > 0.0);
}

#[test]
fn ball_bounces_from_first_of_two_simultaneous_brick_contacts() {
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = 2;
    g.bricks[0][1] = 2;
    g.remaining = 2;
    g.playing = true;
    g.ball.x = 35.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 1);
    assert_eq!(g.bricks[0][1], 2);
    assert_eq!(g.remaining, 2);
    assert!(g.ball.vy > 0.0);
}

#[test]
fn multiball_pickup_replaces_missed_balls_with_three_playable_balls() {
    let mut g = Game::new(1, 0, false);
    g.playing = true;
    g.ball.x = 0.0;
    g.ball.y = (PADDLE_Y - BALL + 1) as f32;
    g.ball.vy = 5.0;
    g.extra_balls[0] = Ball {
        active: true,
        x: 200.0,
        y: 150.0,
        vx: 2.0,
        vy: -5.0,
        contacts: 0,
    };
    g.power_up_kind = PowerUpKind::Multiball;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 0);
    assert!(g.ball.y <= (PADDLE_Y - BALL) as f32 || g.ball.vy <= 0.0);
    assert_eq!(g.extra_balls.iter().filter(|ball| ball.active).count(), 2);
    assert!(g
        .extra_balls
        .iter()
        .all(|ball| ball.y <= (PADDLE_Y - BALL) as f32 || ball.vy <= 0.0));
    assert_eq!(g.lives, 5);
}

#[test]
fn multiball_pickup_replaces_a_missed_extra_ball() {
    let mut g = Game::new(1, 0, false);
    g.playing = true;
    g.ball.x = 150.0;
    g.ball.y = 150.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.extra_balls[0] = Ball {
        active: true,
        x: 0.0,
        y: (PADDLE_Y - BALL + 1) as f32,
        vx: 0.0,
        vy: 5.0,
        contacts: 0,
    };
    g.power_up_kind = PowerUpKind::Multiball;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 0);
    assert!(g
        .extra_balls
        .iter()
        .all(|ball| ball.active && ball.y == g.ball.y));
    assert_ne!(g.extra_balls[0].vx, g.extra_balls[1].vx);
    assert_eq!(g.lives, 5);
}

#[test]
fn multiball_splits_a_survivor_when_primary_has_been_missed() {
    let mut g = Game::new(1, 0, false);
    g.playing = true;
    g.ball.x = 0.0;
    g.ball.y = (PADDLE_Y - BALL + 1) as f32;
    g.ball.vx = 0.0;
    g.ball.vy = 5.0;
    g.extra_balls[0] = Ball {
        active: true,
        x: 200.0,
        y: 150.0,
        vx: 2.0,
        vy: -5.0,
        contacts: 0,
    };
    g.power_up_kind = PowerUpKind::Multiball;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 0);
    let survivor = g.extra_balls[0];
    let split = g.extra_balls[1];
    assert!(split.active);
    assert_eq!(
        (split.x, split.y, split.vy),
        (survivor.x, survivor.y, survivor.vy)
    );
    assert_ne!(split.vx, survivor.vx);
    g.step(0, 0, false, 1);
    assert!(g.extra_balls.iter().all(|ball| ball.active));
}

#[test]
fn paddle_sits_at_bottom_and_ball_rests_and_bounces_above_it() {
    assert_eq!(PADDLE_Y + 7, HEIGHT);
    let mut g = Game::new(1, 0, false);
    assert_eq!(g.ball.y, (PADDLE_Y - BALL) as f32);
    g.playing = true;
    g.ball.x = (g.paddle + 10) as f32;
    g.ball.y = (PADDLE_Y - BALL - 1) as f32;
    g.ball.vx = 0.0;
    g.ball.vy = 5.0;
    g.step(0, 0, false, 0);
    assert!(g.ball.vy < 0.0);
    assert_eq!(g.lives, 5);
}

#[test]
fn multiball_splits_into_three_distinct_trajectories() {
    for initial_vx in [-3.0, 0.0, 3.0] {
        let mut g = Game::new(1, 0, false);
        g.playing = true;
        g.ball.x = 155.0;
        g.ball.y = 150.0;
        g.ball.vx = initial_vx;
        g.ball.vy = -5.0;
        g.power_up_kind = PowerUpKind::Multiball;
        g.power_up = Shot {
            active: true,
            x: g.paddle + 5,
            y: PADDLE_Y - 11,
        };
        g.step(0, 0, false, 0);
        let [a, b] = g.extra_balls;
        assert!(a.active && b.active);
        assert_ne!(a.vx, g.ball.vx);
        assert_ne!(b.vx, g.ball.vx);
        assert_ne!(a.vx, b.vx);
        assert_eq!(a.vy, g.ball.vy);
        assert_eq!(b.vy, g.ball.vy);
    }
}

#[test]
fn bomb_placement_and_rendering_are_valid() {
    struct Recorder(Vec<(i32, i32, u16)>);
    impl Display for Recorder {
        fn rect(&mut self, x: i32, y: i32, _: i32, _: i32, color: u16) {
            self.0.push((x, y, color));
        }
    }
    let mut positions = Vec::new();
    for seed in 0..100 {
        let g = Game::new(seed, 0, false);
        let bombs: Vec<_> = g
            .bricks
            .iter()
            .flatten()
            .enumerate()
            .filter(|(_, b)| **b == BOMB)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(bombs.len(), 1);
        assert_ne!(bombs[0], g.bonus);
        positions.push(bombs[0]);
    }
    assert!(positions.iter().any(|&p| p != positions[0]));
    let mut g = Game::new(0, 0, false);
    g.bricks[0][0] = BOMB;
    let mut display = Recorder(Vec::new());
    render::start(&mut display, &g);
    assert!(display.0.contains(&(1, 35, YELLOW)));
}

#[test]
fn bomb_destroys_neighboring_bricks_and_preserves_solid() {
    let mut g = Game::new(1, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = BOMB;
    g.bricks[0][1] = 2;
    g.bricks[1][0] = 1;
    g.bricks[1][1] = SOLID;
    g.bricks[2][0] = 2;
    g.bonus = COLS;
    g.remaining = 4;
    g.playing = true;
    g.ball.x = 5.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 0);
    assert_eq!(g.bricks[0][1], 0);
    assert_eq!(g.bricks[1][0], 0);
    assert_eq!(g.bricks[1][1], SOLID);
    assert_eq!(g.bricks[2][0], 2);
    assert_eq!(g.remaining, 1);
    assert_eq!(g.lives, 5);
}

#[test]
fn bomb_destroys_at_most_two_neighbors() {
    let mut g = Game::new(1, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = BOMB;
    g.bricks[0][1] = 2;
    g.bricks[1][0] = 2;
    g.bricks[1][1] = 2;
    g.remaining = 4;
    g.playing = true;
    g.ball.x = 5.0;
    g.ball.y = 26.0;
    g.ball.vx = 0.0;
    g.ball.vy = 5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 0);
    assert_eq!(g.bricks[0][1], 0);
    assert_eq!(g.bricks[1][0], 0);
    assert_eq!(g.bricks[1][1], 2);
    assert_eq!(g.remaining, 1);
}

#[test]
fn shot_detonates_bomb_and_can_finish_round() {
    let mut g = Game::new(1, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = BOMB;
    g.bricks[0][1] = 1;
    g.remaining = 2;
    g.playing = true;
    g.shots[0] = Shot {
        active: true,
        x: 5,
        y: 48,
    };
    g.step(0, 0, false, 0);
    assert!(g.finished);
    assert_eq!(g.remaining, 0);
    assert!(!g.shots[0].active);
}

#[test]
fn paddle_accelerates_slows_and_stops_at_screen_edges() {
    let mut g = Game::new(1, 0, false);
    let start = g.paddle;
    g.step(-100, 0, false, 0);
    assert_eq!(g.paddle - start, 1);
    for _ in 0..7 {
        g.step(-100, 0, false, 0);
    }
    assert!(g.paddle_velocity() <= 32);
    let before_release = g.paddle;
    g.step(0, 0, false, 0);
    assert!(g.paddle > before_release);
    for _ in 0..20 {
        g.step(0, 0, false, 0);
    }
    assert_eq!(g.paddle_velocity(), 0);
    for _ in 0..100 {
        g.step(-100, 0, false, 0);
    }
    assert_eq!(g.paddle, WIDTH - PADDLE_WIDTH);
    assert_eq!(g.paddle_velocity(), 0);
    for _ in 0..4 {
        g.step(3, 0, false, 0);
    }
    assert!(g.paddle < WIDTH - PADDLE_WIDTH);
}

#[test]
fn piercing_hits_once_per_pass_and_still_bounces_off_solid_bricks() {
    let mut g = Game::new(2, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[1][0] = 2;
    g.bricks[0][0] = SOLID;
    g.remaining = 1;
    g.playing = true;
    g.power_up_kind = PowerUpKind::Pierce;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 0);
    assert_eq!(g.pierce_seconds(), 10);
    g.ball.x = 5.0;
    g.ball.y = 62.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 1);
    assert_eq!(g.bricks[1][0], 1);
    assert!(g.ball.vy < 0.0);
    for tick in 2..6 {
        g.step(0, 0, false, tick);
    }
    assert_eq!(g.bricks[1][0], 1);
    g.ball.x = 5.0;
    g.ball.y = 47.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 6);
    assert_eq!(g.bricks[0][0], SOLID);
    assert!(g.ball.vy > 0.0);
}

#[test]
fn piercing_refreshes_expires_across_wrap_and_ends_on_life_loss() {
    let now = u32::MAX - 50;
    let mut g = Game::new(1, now, false);
    g.playing = true;
    g.power_up_kind = PowerUpKind::Pierce;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, now);
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    let refresh = now.wrapping_add(5 * CYCLES_PER_SECOND);
    g.step(0, 0, false, refresh);
    assert_eq!(g.pierce_seconds(), 10);
    g.step(0, 0, false, refresh.wrapping_add(PIERCE_DURATION));
    assert_eq!(g.pierce_seconds(), 0);
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, refresh.wrapping_add(PIERCE_DURATION + 1));
    g.ball.y = 239.0;
    g.ball.vy = 5.0;
    g.step(0, 0, false, refresh.wrapping_add(PIERCE_DURATION + 2));
    assert_eq!(g.pierce_seconds(), 0);
}

#[test]
fn extra_ball_damages_bricks_and_can_win() {
    let mut g = Game::new(3, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = 1;
    g.remaining = 1;
    g.bonus = 0;
    g.playing = true;
    g.extra_balls[0] = Ball {
        active: true,
        x: 5.0,
        y: 47.0,
        vx: 0.0,
        vy: -5.0,
        contacts: 0,
    };
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 0);
    assert_eq!(g.lives, 7);
    assert!(g.finished);
}

#[test]
fn multiball_loses_a_life_only_after_the_last_ball() {
    let mut g = Game::new(1, 0, false);
    g.playing = true;
    g.power_up_kind = PowerUpKind::Multiball;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 0);
    assert_eq!(g.extra_balls.iter().filter(|b| b.active).count(), 2);
    assert_ne!(g.extra_balls[0].vx, g.extra_balls[1].vx);
    g.ball.y = 239.0;
    g.ball.vy = 5.0;
    g.step(0, 0, false, 1);
    assert_eq!(g.lives, 5);
    assert!(g.playing);
    assert_eq!(g.lost_ball, None);
    assert_eq!(g.extra_balls.iter().filter(|b| b.active).count(), 1);
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 2);
    assert_eq!(g.extra_balls.iter().filter(|b| b.active).count(), 2);
    g.ball.y = 239.0;
    g.ball.vy = 5.0;
    for ball in &mut g.extra_balls {
        ball.y = 239.0;
        ball.vy = 5.0;
    }
    g.step(0, 0, false, 3);
    assert_eq!(g.lives, 4);
    assert!(!g.playing);
    assert_eq!(g.lost_ball.map(|(_, y)| y), Some(HEIGHT - BALL));
    assert!(g.extra_balls.iter().all(|b| !b.active));
}

#[test]
fn wide_power_up_expires_across_counter_wrap_and_keeps_paddle_in_bounds() {
    let now = u32::MAX - 100;
    let mut g = Game::new(2, now, false);
    g.playing = true;
    g.paddle = WIDTH - PADDLE_WIDTH;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, now);
    assert!(!g.power_up.active);
    assert_eq!(g.paddle_width(), WIDE_PADDLE_WIDTH);
    assert_eq!(g.paddle, WIDTH - WIDE_PADDLE_WIDTH);
    assert_eq!(g.wide_seconds(), 10);
    g.step(0, 0, false, now.wrapping_add(WIDE_DURATION - 1));
    assert_eq!(g.wide_seconds(), 1);
    g.step(0, 0, false, now.wrapping_add(WIDE_DURATION));
    assert_eq!(g.paddle_width(), PADDLE_WIDTH);
    assert_eq!(g.wide_seconds(), 0);
}

#[test]
fn wide_power_up_refreshes_and_ends_on_life_loss() {
    let mut g = Game::new(1, 0, false);
    g.playing = true;
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 0);
    g.power_up = Shot {
        active: true,
        x: g.paddle + 5,
        y: PADDLE_Y - 11,
    };
    g.step(0, 0, false, 5 * CYCLES_PER_SECOND);
    assert_eq!(g.wide_seconds(), 10);
    g.ball.y = 239.0;
    g.ball.vy = 5.0;
    g.step(0, 0, false, 5 * CYCLES_PER_SECOND + 1);
    assert_eq!(g.paddle_width(), PADDLE_WIDTH);
    assert!(!g.power_up.active);
}

#[test]
fn destroyed_bricks_can_drop_power_ups() {
    let mut found_drop = false;
    let mut found_no_drop = false;
    let mut found_wide = false;
    let mut found_multi = false;
    let mut found_pierce = false;
    for seed in 0..100 {
        let mut g = Game::new(seed, 0, false);
        g.bricks = [[0; COLS]; ROWS];
        g.bricks[0][0] = 1;
        g.remaining = 2;
        g.playing = true;
        g.ball.x = 5.0;
        g.ball.y = 47.0;
        g.ball.vx = 0.0;
        g.ball.vy = -5.0;
        g.step(0, 0, false, 0);
        found_drop |= g.power_up.active;
        found_no_drop |= !g.power_up.active;
        found_wide |= g.power_up.active && g.power_up_kind == PowerUpKind::Wide;
        found_multi |= g.power_up.active && g.power_up_kind == PowerUpKind::Multiball;
        found_pierce |= g.power_up.active && g.power_up_kind == PowerUpKind::Pierce;
    }
    assert!(found_drop && found_no_drop);
    assert!(found_wide && found_multi && found_pierce);
}

#[test]
fn solid_bricks_are_random_and_never_replace_the_bonus() {
    let mut layouts = Vec::new();
    let mut bonus_column_without_solid = false;
    for seed in 0..100 {
        let g = Game::new(seed, 0, false);
        let mut solids = 0;
        let mut previous_solid = false;
        for c in 0..COLS {
            let count = g.bricks.iter().filter(|row| row[c] == SOLID).count();
            assert!(count <= 1);
            assert!(!(previous_solid && count > 0));
            previous_solid = count > 0;
            solids += count;
        }
        assert!(solids > 0);
        assert_eq!(g.remaining, ROWS * COLS - solids);
        assert_eq!(g.bricks[g.bonus / COLS][g.bonus % COLS], 2);
        bonus_column_without_solid |= (0..ROWS).all(|r| g.bricks[r][g.bonus % COLS] != SOLID);
        layouts.push(g.bricks);
    }
    assert!(layouts.iter().any(|layout| *layout != layouts[0]));
    assert!(bonus_column_without_solid);
}

#[test]
fn solid_bricks_bounce_balls_absorb_shots_and_do_not_prevent_winning() {
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; COLS]; ROWS];
    g.bricks[0][0] = SOLID;
    g.bricks[0][1] = 1;
    g.remaining = 1;
    g.playing = true;
    g.ball.x = 5.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], SOLID);
    assert!(g.ball.vy > 0.0);
    assert_eq!(g.remaining, 1);
    g.shots[0] = Shot {
        active: true,
        x: 5,
        y: 48,
    };
    g.step(0, 0, false, 1);
    assert!(!g.shots[0].active);
    assert_eq!(g.bricks[0][0], SOLID);
    g.shots[0] = Shot {
        active: true,
        x: 45,
        y: 48,
    };
    g.step(0, 0, false, 2);
    assert_eq!(g.bricks[0][1], 0);
    assert!(g.finished);
}

#[test]
fn solid_bricks_render_white() {
    struct Recorder(Vec<(i32, i32, u16)>);
    impl Display for Recorder {
        fn rect(&mut self, x: i32, y: i32, _: i32, _: i32, color: u16) {
            self.0.push((x, y, color));
        }
    }
    let mut g = Game::new(0, 0, false);
    g.bricks[0][0] = SOLID;
    let mut display = Recorder(Vec::new());
    render::start(&mut display, &g);
    assert!(display.0.contains(&(1, 35, WHITE)));
}

#[test]
fn launch_requires_a_fresh_button_edge_and_shots_repeat() {
    let mut g = Game::new(1, 0, true);
    g.step(0, 0, true, 1);
    assert!(!g.playing);
    g.step(0, 0, false, 2);
    g.step(0, 0, true, 3);
    assert!(g.playing);
    assert_eq!(g.shots.iter().filter(|s| s.active).count(), 0);
    g.step(0, 0, true, CYCLES_PER_SECOND / 5 + 3);
    assert_eq!(g.shots.iter().filter(|s| s.active).count(), 2);
}

#[test]
fn speed_gestures_require_rest_and_work_across_counter_wrap() {
    let mut g = Game::new(3, u32::MAX - 20, false);
    g.step(0, -50, false, u32::MAX - 10);
    assert_eq!(g.speed, 6);
    g.step(0, -50, false, CYCLES_PER_SECOND + 1);
    assert_eq!(g.speed, 6);
    g.step(0, 0, false, CYCLES_PER_SECOND + 2);
    g.step(0, -50, false, CYCLES_PER_SECOND + 3);
    assert_eq!(g.speed, 7);
}

#[test]
fn paddle_stays_in_bounds_and_loss_ends_game() {
    let mut g = Game::new(0, 0, false);
    for _ in 0..100 {
        g.step(-100, 0, false, 0);
    }
    assert_eq!(g.paddle, WIDTH - PADDLE_WIDTH);
    g.lives = 1;
    g.playing = true;
    g.ball.y = 239.0;
    g.ball.vy = 5.0;
    g.step(0, 0, false, 0);
    assert!(g.finished);
    assert_eq!(g.lives, 0);
}

#[test]
fn ball_damages_then_destroys_bonus_brick() {
    let mut g = Game::new(0, 0, false);
    g.bricks = [[0; 8]; 5];
    g.bricks[0][0] = 2;
    g.remaining = 1;
    g.bonus = 0;
    g.playing = true;
    g.ball.x = 5.0;
    g.ball.y = 47.0;
    g.ball.vx = 0.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 1);
    g.ball.y = 47.0;
    g.ball.vy = -5.0;
    g.step(0, 0, false, 0);
    assert_eq!(g.bricks[0][0], 0);
    assert_eq!(g.lives, 7);
    assert_eq!(g.remaining, 0);
}
