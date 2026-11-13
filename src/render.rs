//! RGB565 framebuffer rendering in the LCD's rotated orientation.
use crate::game::{
    Game, PowerUpKind, Shatter, BALL, BOMB, BRICK_HEIGHT, BRICK_PITCH_X, BRICK_PITCH_Y, BRICK_TOP,
    BRICK_WIDTH, COLS, PADDLE_HEIGHT, PADDLE_Y, ROWS, SHATTER_FRAMES, SOLID,
};
use crate::scores::Scores;
pub const BLACK: u16 = 0;
pub const WHITE: u16 = 0xffff;
pub const RED: u16 = 0xf800;
pub const GREEN: u16 = 0x07e0;
pub const BLUE: u16 = 0x001f;
pub const YELLOW: u16 = 0xffe0;
pub const CYAN: u16 = 0x07ff;
pub const MAGENTA: u16 = 0xf81f;
pub const ORANGE: u16 = 0xfd20;
pub trait Display {
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u16);
}
/// Draw a filled, round ball within the 10-by-10 collision box.
pub fn ball(d: &mut impl Display, x: i32, y: i32, color: u16) {
    const INSETS: [i32; 10] = [3, 2, 1, 0, 0, 0, 0, 1, 2, 3];
    for (row, inset) in INSETS.iter().enumerate() {
        d.rect(x + inset, y + row as i32, BALL - 2 * inset, 1, color);
    }
}
fn color(g: &Game, r: usize, c: usize) -> u16 {
    if g.bricks()[r][c] == SOLID {
        WHITE
    } else if g.bricks()[r][c] == BOMB {
        YELLOW
    } else if g.bricks()[r][c] == 1 {
        RED
    } else if r * COLS + c == g.bonus() {
        BLUE
    } else {
        GREEN
    }
}
fn brick(d: &mut impl Display, g: &Game, r: usize, c: usize) {
    d.rect(
        c as i32 * BRICK_PITCH_X + 1,
        BRICK_TOP + r as i32 * BRICK_PITCH_Y,
        BRICK_WIDTH,
        BRICK_HEIGHT,
        if g.bricks()[r][c] > 0 {
            color(g, r, c)
        } else {
            BLACK
        },
    );
}
pub fn start(d: &mut impl Display, g: &Game) {
    d.rect(0, 0, 320, 240, BLACK);
    for r in 0..ROWS {
        for c in 0..COLS {
            brick(d, g, r, c);
        }
    }
}
fn restore(d: &mut impl Display, g: &Game, x: i32, y: i32) {
    for r in 0..ROWS {
        for c in 0..COLS {
            let bx = c as i32 * BRICK_PITCH_X + 1;
            let by = BRICK_TOP + r as i32 * BRICK_PITCH_Y;
            if g.bricks()[r][c] > 0
                && x + BALL > bx
                && x < bx + BRICK_WIDTH
                && y + BALL > by
                && y < by + BRICK_HEIGHT
            {
                brick(d, g, r, c);
            }
        }
    }
}
pub fn frame(d: &mut impl Display, old: &Game, g: &Game) {
    for r in 0..ROWS {
        for c in 0..COLS {
            if old.bricks()[r][c] != g.bricks()[r][c] {
                brick(d, g, r, c);
            }
        }
    }
    for effect in old.shatters().iter().filter(|effect| effect.frames > 0) {
        shatter(d, g, effect, false);
    }
    if old.power_up().active {
        d.rect(old.power_up().x, old.power_up().y, BALL, BALL, BLACK);
        restore(d, g, old.power_up().x, old.power_up().y);
    }
    for shot in old.shots().iter().filter(|s| s.active) {
        d.rect(shot.x, shot.y, BALL, BALL, BLACK);
        restore(d, g, shot.x, shot.y);
    }
    d.rect(old.ball().x as i32, old.ball().y as i32, BALL, BALL, BLACK);
    restore(d, g, old.ball().x as i32, old.ball().y as i32);
    for ball in old.extra_balls().iter().filter(|b| b.active) {
        d.rect(ball.x as i32, ball.y as i32, BALL, BALL, BLACK);
        restore(d, g, ball.x as i32, ball.y as i32);
    }
    d.rect(
        old.paddle(),
        PADDLE_Y,
        old.paddle_width(),
        PADDLE_HEIGHT,
        BLACK,
    );
    for i in 0..7 {
        d.rect(
            5 + i * 14,
            4,
            10,
            8,
            if i < g.lives() { BLUE } else { BLACK },
        );
    }
    d.rect(g.paddle(), PADDLE_Y, g.paddle_width(), PADDLE_HEIGHT, WHITE);
    let ball_color = if g.pierce_seconds() > 0 {
        MAGENTA
    } else {
        GREEN
    };
    ball(d, g.ball().x as i32, g.ball().y as i32, ball_color);
    for extra in g.extra_balls().iter().filter(|b| b.active) {
        ball(d, extra.x as i32, extra.y as i32, ball_color);
    }
    if g.power_up().active {
        d.rect(
            g.power_up().x,
            g.power_up().y,
            BALL,
            BALL,
            match g.power_up_kind() {
                PowerUpKind::Wide => YELLOW,
                PowerUpKind::Multiball => CYAN,
                PowerUpKind::Pierce => MAGENTA,
            },
        );
    }
    for shot in g.shots().iter().filter(|s| s.active) {
        d.rect(shot.x, shot.y, BALL, BALL, RED);
    }
    for effect in g.shatters().iter().filter(|effect| effect.frames > 0) {
        shatter(d, g, effect, true);
    }
    if old.speed() != g.speed() {
        speed(d, g.speed());
    }
    if old.wide_seconds() != g.wide_seconds() {
        power_timer(d, g.wide_seconds(), 246, LETTERS[22], YELLOW);
    }
    if old.pierce_seconds() != g.pierce_seconds() {
        power_timer(d, g.pierce_seconds(), 215, LETTERS[15], MAGENTA);
    }
}
fn shatter(d: &mut impl Display, g: &Game, effect: &Shatter, draw: bool) {
    let distance = 2 + i32::from(SHATTER_FRAMES - effect.frames) / 2;
    for (dx, dy) in [
        (-distance, -distance),
        (distance, -distance),
        (-distance, distance),
        (distance, distance),
    ] {
        let (x, y) = (effect.x + dx, effect.y + dy);
        // The HUD occupies the top of the display; keep particles in the playfield.
        if !(18..240).contains(&y) {
            continue;
        }
        d.rect(x, y, 2, 2, if draw { CYAN } else { BLACK });
        if !draw {
            restore(d, g, x, y);
        }
    }
}
fn power_timer(d: &mut impl Display, seconds: u32, left: i32, label: [u8; 7], color: u16) {
    d.rect(left, 3, 24, 10, BLACK);
    if seconds == 0 {
        return;
    }
    for (i, glyph) in [
        label,
        DIGITS[(seconds / 10) as usize],
        DIGITS[(seconds % 10) as usize],
    ]
    .iter()
    .enumerate()
    {
        for (y, row) in glyph.iter().enumerate() {
            for x in 0..5 {
                if row & (1 << (4 - x)) != 0 {
                    d.rect(left + i as i32 * 7 + x, 4 + y as i32, 1, 1, color);
                }
            }
        }
    }
}
pub fn speed(d: &mut impl Display, t: i32) {
    let glyphs = [
        0x7b6fu16, 0x2492, 0x73e7, 0x73cf, 0x5bc9, 0x79cf, 0x79ef, 0x7249, 0x7bef, 0x7bcf, 0x0002,
        0x0550,
    ];
    d.rect(284, 3, 34, 10, BLACK);
    for (i, s) in [t / 10, 10, t % 10, 11].iter().enumerate() {
        for y in 0..5 {
            for x in 0..3 {
                if glyphs[*s as usize] & (1 << (14 - y * 3 - x)) != 0 {
                    d.rect(284 + i as i32 * 8 + x * 2, 3 + y * 2, 2, 2, WHITE);
                }
            }
        }
    }
}
const LETTERS: [[u8; 7]; 26] = [
    [14, 17, 17, 31, 17, 17, 17],
    [30, 17, 17, 30, 17, 17, 30],
    [14, 17, 16, 16, 16, 17, 14],
    [30, 17, 17, 17, 17, 17, 30],
    [31, 16, 16, 30, 16, 16, 31],
    [31, 16, 16, 30, 16, 16, 16],
    [14, 17, 16, 23, 17, 17, 15],
    [0; 7],
    [14, 4, 4, 4, 4, 4, 14],
    [0; 7],
    [17, 18, 20, 24, 20, 18, 17],
    [16, 16, 16, 16, 16, 16, 31],
    [17, 27, 21, 21, 17, 17, 17],
    [17, 25, 25, 21, 19, 19, 17],
    [14, 17, 17, 17, 17, 17, 14],
    [30, 17, 17, 30, 16, 16, 16],
    [0; 7],
    [30, 17, 17, 30, 20, 18, 17],
    [15, 16, 16, 14, 1, 1, 30],
    [31, 4, 4, 4, 4, 4, 4],
    [17, 17, 17, 17, 17, 17, 14],
    [0; 7],
    [17, 17, 17, 21, 21, 21, 10],
    [17, 17, 10, 4, 10, 17, 17],
    [17, 17, 10, 4, 4, 4, 4],
    [0; 7],
];
const DIGITS: [[u8; 7]; 10] = [
    [14, 17, 19, 21, 25, 17, 14],
    [4, 12, 4, 4, 4, 4, 14],
    [14, 17, 1, 2, 4, 8, 31],
    [30, 1, 1, 14, 1, 1, 30],
    [2, 6, 10, 18, 31, 2, 2],
    [31, 16, 16, 30, 1, 1, 30],
    [14, 16, 16, 30, 17, 17, 14],
    [31, 1, 2, 4, 8, 8, 8],
    [14, 17, 17, 14, 17, 17, 14],
    [14, 17, 17, 15, 1, 1, 14],
];
pub fn text(d: &mut impl Display, s: &[u8], top: i32, scale: i32, color: u16) {
    let left = (320 - (s.len() as i32 * 6 - 1) * scale) / 2;
    text_at(d, s, left, top, scale, color);
}

/// Draw text at a specified horizontal position.
pub fn text_at(d: &mut impl Display, s: &[u8], left: i32, top: i32, scale: i32, color: u16) {
    for (i, ch) in s.iter().enumerate() {
        let glyph = match ch {
            b'A'..=b'Z' => LETTERS[(ch - b'A') as usize],
            b'0'..=b'9' => DIGITS[(ch - b'0') as usize],
            b':' => [0, 4, 4, 0, 4, 4, 0],
            b'-' => [0, 0, 0, 31, 0, 0, 0],
            b'+' => [0, 4, 4, 31, 4, 4, 0],
            _ => [0; 7],
        };
        for (y, row) in glyph.iter().enumerate() {
            for x in 0..5 {
                if row & (1 << (4 - x)) != 0 {
                    d.rect(
                        left + i as i32 * 6 * scale + x * scale,
                        top + y as i32 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }
}
pub fn result(d: &mut impl Display, win: bool, seconds: u64, scores: &Scores, rank: Option<usize>) {
    d.rect(0, 0, 320, 240, BLACK);
    if win {
        text(d, b"YOU WIN", 20, 3, GREEN);
    } else {
        text(d, b"MAYBE NEXT TIME", 20, 2, RED);
    }
    time_line(d, b"TIME ", seconds, 57, YELLOW);
    let mut position = *b"RANK - ";
    if let Some(i) = rank {
        position[5] = b'1' + i as u8;
    } else if win {
        position[5] = b'5';
        position[6] = b'+';
    }
    text(d, &position, 75, 2, YELLOW);
    text(d, b"TOP 5", 91, 2, WHITE);
    for (i, score) in scores.best.iter().enumerate() {
        let label = [b'1' + i as u8, b' '];
        if let Some(time) = score {
            time_line(
                d,
                &label,
                *time,
                120 + i as i32 * 24,
                if rank == Some(i) { YELLOW } else { WHITE },
            );
        } else {
            let empty = [label[0], b' ', b'-', b'-', b':', b'-', b'-'];
            text(d, &empty, 120 + i as i32 * 24, 2, WHITE);
        }
    }
}
fn time_line(d: &mut impl Display, label: &[u8], seconds: u64, top: i32, color: u16) {
    let mut buf = [b' '; 28];
    buf[..label.len()].copy_from_slice(label);
    let mut n = seconds / 60;
    let mut digits = [0u8; 20];
    let mut count = 0;
    loop {
        digits[count] = b'0' + (n % 10) as u8;
        count += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if count < 2 {
        digits[count] = b'0';
        count += 1;
    }
    for i in 0..count {
        buf[label.len() + i] = digits[count - 1 - i];
    }
    let p = label.len() + count;
    buf[p] = b':';
    buf[p + 1] = b'0' + ((seconds % 60) / 10) as u8;
    buf[p + 2] = b'0' + (seconds % 10) as u8;
    text(d, &buf[..p + 3], top, 2, color);
}
