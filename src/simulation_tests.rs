#[path = "simulation.rs"]
mod simulation;

use gyro_breakout::game::{
    Ball, Game, Shot, BALL, CYCLES_PER_SECOND, HEIGHT, PADDLE_Y, SOLID, WIDTH,
};
use simulation::Controller;

#[test]
fn simulation_preserves_game_invariants_across_rounds_and_timer_wrap() {
    for seed in 0..8 {
        let mut game = Game::new(seed, 0, false);
        game.speed = 10;
        let mut controller = Controller::new(seed, 4);
        for tick in 0u32..6000 {
            let (rate, launch) = controller.input(&game);
            let pressed = if game.playing { tick % 50 < 25 } else { launch };
            game.step(rate, 0, pressed, tick.wrapping_mul(CYCLES_PER_SECOND / 100));
            let remaining = game
                .bricks
                .iter()
                .flatten()
                .filter(|&&brick| brick != 0 && brick != SOLID)
                .count();
            assert_eq!(game.remaining, remaining, "seed {seed}, tick {tick}");
            assert!((0..=WIDTH - game.paddle_width()).contains(&game.paddle));
            assert!(
                game.ball.x.is_finite()
                    && game.ball.y.is_finite()
                    && game.ball.vx.is_finite()
                    && game.ball.vy.is_finite()
            );
            if game.playing {
                assert!(game.ball.y < HEIGHT as f32);
            }
            for ball in game.extra_balls.iter().filter(|ball| ball.active) {
                assert!(
                    ball.x.is_finite()
                        && ball.y.is_finite()
                        && ball.vx.is_finite()
                        && ball.vy.is_finite()
                );
                assert!(ball.y < HEIGHT as f32);
            }
            if game.finished {
                break;
            }
        }
    }
}

#[test]
fn ignores_a_missed_ball_when_another_can_be_caught() {
    let mut game = Game::new(1, 0, false);
    game.playing = true;
    game.ball.x = 0.0;
    game.ball.y = (PADDLE_Y - BALL + 1) as f32;
    game.ball.vx = 0.0;
    game.ball.vy = 5.0;
    game.extra_balls[0] = Ball {
        active: true,
        x: 300.0,
        y: 200.0,
        vx: 0.0,
        vy: 5.0,
        contacts: 0,
    };
    let mut controller = Controller::new(1, 4);
    assert!(controller.input(&game).0 < 0);

    game.ball.x = 300.0;
    game.ball.y = 200.0;
    game.extra_balls[0].x = 0.0;
    game.extra_balls[0].y = (PADDLE_Y - BALL + 1) as f32;
    let mut controller = Controller::new(1, 4);
    assert!(controller.input(&game).0 < 0);
}

#[test]
fn tracks_upward_survivor_when_primary_was_missed() {
    let mut game = Game::new(1, 0, false);
    game.playing = true;
    game.ball.x = 0.0;
    game.ball.y = (PADDLE_Y - BALL + 1) as f32;
    game.ball.vy = 5.0;
    game.extra_balls[0] = Ball {
        active: true,
        x: 300.0,
        y: 150.0,
        vx: 0.0,
        vy: -5.0,
        contacts: 0,
    };
    let mut controller = Controller::new(1, 4);
    assert!(controller.input(&game).0 < 0);
}

#[test]
fn moves_toward_power_up_when_ball_is_not_approaching() {
    let mut game = Game::new(1, 0, false);
    game.playing = true;
    game.ball.x = 155.0;
    game.ball.y = 130.0;
    game.ball.vy = -5.0;
    game.power_up = Shot {
        active: true,
        x: 0,
        y: 180,
    };
    let mut controller = Controller::new(1, 4);
    assert!((-3..=3).contains(&controller.offset()));
    let (rate, _) = controller.input(&game);
    assert!(rate > 0, "positive rate moves the paddle left");
}

#[test]
fn prioritizes_imminent_ball_if_pickup_is_out_of_reach() {
    let mut game = Game::new(1, 0, false);
    game.playing = true;
    game.ball.x = 300.0;
    game.ball.y = 200.0;
    game.ball.vx = 0.0;
    game.ball.vy = 5.0;
    game.power_up = Shot {
        active: true,
        x: 0,
        y: 200,
    };
    let mut controller = Controller::new(1, 4);
    let (rate, _) = controller.input(&game);
    assert!(rate < 0, "negative rate moves the paddle right");
}

#[test]
fn catches_a_reachable_power_up_with_momentum() {
    let mut game = Game::new(1, 0, false);
    game.playing = true;
    game.ball.x = 155.0;
    game.ball.y = 150.0;
    game.ball.vx = 0.0;
    game.ball.vy = 0.0;
    game.power_up = Shot {
        active: true,
        x: 0,
        y: 130,
    };
    let mut controller = Controller::new(1, 4);
    for tick in 0..60 {
        let (rate, _) = controller.input(&game);
        game.step(rate, 0, false, tick);
        if !game.power_up.active {
            break;
        }
    }
    assert!(game.wide_seconds() > 0);
}
