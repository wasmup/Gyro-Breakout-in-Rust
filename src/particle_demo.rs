//! Startup particle benchmark. Both modes render the same scene into SDRAM.
use crate::board::{self, Board, ParticleFrame};
use gyro_breakout::{
    game::CYCLES_PER_SECOND,
    input::PressCycle,
    render::{self, Display},
};

const PARTICLES: u32 = 96;
const BAR_HEIGHT: i32 = 24;
const SAMPLE_CYCLES: u32 = CYCLES_PER_SECOND;
const SAMPLES_PER_MODE: u8 = 3;
const COLORS: [u16; 6] = [0xf81f, 0x07ff, 0xffe0, 0x07e0, 0xfd20, 0xffff];

fn fps_digits(mut fps: u32) -> [u8; 4] {
    let mut digits = *b"0000";
    fps = fps.min(9999);
    for digit in digits.iter_mut().rev() {
        *digit = b'0' + (fps % 10) as u8;
        fps /= 10;
    }
    digits
}

fn draw(frame: &mut ParticleFrame, dma: bool, tick: u32) -> bool {
    if !frame.fill(0, BAR_HEIGHT, 320, 240 - BAR_HEIGHT, 0x0003, dma) {
        return false;
    }
    for i in 0..PARTICLES {
        let size = 4 + (i % 5) as i32;
        let x = ((i * 79 + tick * (1 + i % 3)) % (321 - size as u32)) as i32;
        let y = 42 + ((i * 43 + tick * (1 + i % 2)) % (191 - size as u32)) as i32;
        if !frame.fill(x, y, size, size, COLORS[i as usize % COLORS.len()], dma) {
            return false;
        }
    }
    true
}

pub fn run(board: &mut Board) {
    let mut cycle = PressCycle::default();
    let mut dma = false;
    let mut dma_failed = false;
    let mut results = [None, None];
    let mut bars = [None, None];
    let mut samples = 0u8;
    let mut frames = 0u32;
    let mut started = board::cycles();
    loop {
        let now = board::cycles();
        if cycle.update(board::button(), now) {
            return;
        }
        let tick = now / (CYCLES_PER_SECOND / 60);
        let mut frame = board.particle_frame();
        if !draw(&mut frame, dma, tick) {
            // The driver resets DMA2D before returning; discard this partial frame.
            dma_failed = true;
            dma = false;
            frames = 0;
            samples = 0;
            started = board::cycles();
            continue;
        }
        let index = frame.index();
        let bar = (results, dma, dma_failed);
        if bars[index] != Some(bar) {
            frame.rect(0, 0, 320, BAR_HEIGHT, render::BLACK);
            if let Some(fps) = results[0] {
                render::text_at(
                    &mut frame,
                    &fps_digits(fps),
                    4,
                    5,
                    2,
                    if !dma { render::YELLOW } else { render::WHITE },
                );
            }
            if dma_failed {
                render::text_at(&mut frame, b"0000", 270, 5, 2, render::RED);
            } else if let Some(fps) = results[1] {
                render::text_at(
                    &mut frame,
                    &fps_digits(fps),
                    270,
                    5,
                    2,
                    if dma { render::YELLOW } else { render::WHITE },
                );
            }
            bars[index] = Some(bar);
        }
        board.show_particle_frame(&frame);
        frames += 1;
        let end = board::cycles();
        let elapsed = end.wrapping_sub(started);
        if elapsed >= SAMPLE_CYCLES {
            results[usize::from(dma)] = Some(
                ((u64::from(frames) * u64::from(CYCLES_PER_SECOND)) / u64::from(elapsed)) as u32,
            );
            frames = 0;
            started = end;
            samples += 1;
            if samples == SAMPLES_PER_MODE {
                samples = 0;
                if !dma_failed {
                    dma = !dma;
                }
            }
        }
    }
}

/// A persistent particle field over a cached Mandelbrot image.
pub fn interactive(board: &mut Board, touch_available: &mut bool) {
    #[derive(Clone, Copy)]
    struct Particle {
        x: i32,
        y: i32,
        vx: i32,
        vy: i32,
    }
    const COUNT: usize = 240;
    let mut particles = [Particle {
        x: 0,
        y: 0,
        vx: 0,
        vy: 0,
    }; COUNT];
    for (i, p) in particles.iter_mut().enumerate() {
        p.x = ((i * 79 % 320) as i32) * 256;
        p.y = ((i * 43 % 240) as i32) * 256;
        p.vx = (i as i32 * 37 % 513) - 256;
        p.vy = (i as i32 * 71 % 513) - 256;
    }
    board.prepare_particle_background();
    // The gyro measures rotation rate; calibrate before using it to steer.
    let gyro = board.calibrate();
    let mut cycle = PressCycle::default();
    let mut touching = false;
    loop {
        let start = board::cycles();
        if cycle.update(board::button(), start) {
            return;
        }
        let (gx, gy) = if gyro {
            board.rates().unwrap_or((0, 0))
        } else {
            (0, 0)
        };
        if *touch_available {
            match board.touch() {
                Ok((down, position)) => {
                    if down && !touching {
                        if let Some((x, y)) = position {
                            for (i, p) in particles.iter_mut().take(80).enumerate() {
                                p.x = x * 256;
                                p.y = y * 256;
                                p.vx = (i as i32 * 97 % 1537) - 768;
                                p.vy = (i as i32 * 173 % 1537) - 768;
                            }
                        }
                    }
                    touching = down;
                }
                Err(()) => *touch_available = false,
            }
        }
        let mut frame = board.particle_frame();
        board.restore_particle_background(&mut frame);
        for (i, p) in particles.iter_mut().enumerate() {
            p.vx = (p.vx + gy * 2).clamp(-768, 768);
            p.vy = (p.vy + gx * 2).clamp(-768, 768);
            p.x = (p.x + p.vx).rem_euclid(320 * 256);
            p.y = (p.y + p.vy).rem_euclid(240 * 256);
            frame.fill(p.x / 256, p.y / 256, 3, 3, COLORS[i % COLORS.len()], false);
        }
        board.show_particle_frame(&frame);
        while board::cycles().wrapping_sub(start) < CYCLES_PER_SECOND / 60 {
            core::hint::spin_loop();
        }
    }
}
