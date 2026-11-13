#![no_std]
#![no_main]
mod board;
mod particle_demo;
mod simulation;
use core::arch::global_asm;
use core::panic::PanicInfo;
use gyro_breakout::{
    game::{Game, CYCLES_PER_SECOND},
    input::PressCycle,
    render::{self, Display},
    scores::Scores,
};
global_asm!(
    ".section .isr_vector,\"a\",%progbits",
    ".word _estack",
    ".word Reset_Handler",
    ".rept 14",
    ".word Default_Handler",
    ".endr"
);
extern "C" {
    static mut _sdata: u32;
    static mut _edata: u32;
    static _sidata: u32;
    static mut _sbss: u32;
    static mut _ebss: u32;
}
#[no_mangle]
unsafe extern "C" fn Reset_Handler() -> ! {
    let mut src = core::ptr::addr_of!(_sidata);
    let mut dst = core::ptr::addr_of_mut!(_sdata);
    while dst < core::ptr::addr_of_mut!(_edata) {
        dst.write(src.read());
        dst = dst.add(1);
        src = src.add(1);
    }
    dst = core::ptr::addr_of_mut!(_sbss);
    while dst < core::ptr::addr_of_mut!(_ebss) {
        dst.write(0);
        dst = dst.add(1);
    } // Enable the FPU before entering code compiled for the hard-float ABI.
    let cpacr = 0xe000_ed88 as *mut u32;
    cpacr.write_volatile(cpacr.read_volatile() | (0xf << 20));
    core::arch::asm!("dsb", "isb", options(nostack, preserves_flags));
    main()
}
#[no_mangle]
extern "C" fn Default_Handler() -> ! {
    board::fault()
}
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    board::fault()
}
#[derive(Default)]
struct Debouncer {
    stable: bool,
    candidate: bool,
    changed: u32,
}
impl Debouncer {
    fn update(&mut self, raw: bool, now: u32) -> bool {
        if raw != self.candidate {
            self.candidate = raw;
            self.changed = now;
        }
        if self.candidate != self.stable && now.wrapping_sub(self.changed) >= CYCLES_PER_SECOND / 50
        {
            self.stable = self.candidate;
        }
        self.stable
    }
}
fn explore_fractal(
    board: &mut board::Board,
    touch_available: &mut bool,
    mut zoom: impl FnMut(&mut board::Board, i32, i32),
) {
    let mut released_samples = 0u8;
    let mut cycle = PressCycle::default();
    let mut now = 0u32;
    loop {
        if cycle.update(board::button(), now) {
            break;
        }
        if *touch_available {
            match board.touch() {
                Ok((false, _)) => released_samples = released_samples.saturating_add(1).min(3),
                Ok((true, Some((x, y)))) if released_samples >= 3 => {
                    released_samples = 0;
                    zoom(board, x, y);
                }
                Ok((true, _)) if released_samples < 3 => released_samples = 0,
                Ok(_) => {}
                Err(()) => *touch_available = false,
            }
        }
        board::delay(10);
        now = now.wrapping_add(CYCLES_PER_SECOND / 100);
    }
}
fn main() -> ! {
    let Some(mut board) = board::Board::init() else {
        board::fault()
    };
    if board.startup_prepare() {
        particle_demo::run(&mut board);
        let mut touch_available = board.touch_init();
        particle_demo::interactive(&mut board, &mut touch_available);
        board.startup_fractal();
        let mut mandelbrot = gyro_breakout::fractal::Mandelbrot::default();
        explore_fractal(&mut board, &mut touch_available, |board, x, y| {
            if mandelbrot.zoom(x, y) {
                board.zoom_mandelbrot(&mandelbrot);
            }
        });
        let mut julia = gyro_breakout::fractal::Julia::default();
        board.startup_julia(&julia);
        explore_fractal(&mut board, &mut touch_available, |board, x, y| {
            if julia.zoom(x, y) {
                board.startup_julia(&julia);
            }
        });
        let mut snowflake = gyro_breakout::fractal::Viewport::default();
        board.startup_snowflake(&snowflake);
        explore_fractal(&mut board, &mut touch_available, |board, x, y| {
            if snowflake.zoom(x, y) {
                board.startup_snowflake(&snowflake);
            }
        });
        let mut fern = gyro_breakout::fractal::Viewport::default();
        board.startup_fern(&fern);
        explore_fractal(&mut board, &mut touch_available, |board, x, y| {
            if fern.zoom(x, y) {
                board.startup_fern(&fern);
            }
        });
        board.end_startup_fractal();
    }
    board.rect(0, 0, 320, 240, render::BLACK);
    render::text(&mut board, b"KEEP BOARD FLAT", 100, 2, render::WHITE);
    render::text(&mut board, b"CALIBRATING", 124, 2, render::WHITE);
    board::delay(1500);
    if !board.calibrate() {
        board::fault()
    }
    let simulation = board::button();
    let mut input = Debouncer::default();
    let mut previous = false;
    let mut previous_offset = 4;
    let mut scores = Scores::default();
    loop {
        let start = board::cycles();
        let mut game = Game::new(board.seed, start, previous);
        let mut controller = simulation::Controller::new(board.seed ^ start, previous_offset);
        previous_offset = controller.offset();
        if simulation {
            game.set_speed(10);
        }
        render::start(&mut board, &game);
        render::speed(&mut board, game.speed());
        let mut next = board::cycles();
        let mut last = next;
        let mut elapsed = 0u64;
        let mut loss_indicator: Option<(i32, i32, u32)> = None;
        while !game.finished()
            || loss_indicator.is_some()
            || game.shatters().iter().any(|effect| effect.frames > 0)
        {
            let old = game;
            let Some((mut paddle, speed)) = board.rates() else {
                board::fault()
            };
            let physical = input.update(board::button(), board::cycles());
            let pressed = if simulation {
                let (p, b) = controller.input(&game);
                paddle = p;
                if game.playing() {
                    physical
                } else {
                    b
                }
            } else {
                physical
            };
            let now = board::cycles();
            let was_playing_round = !game.finished();
            game.step(paddle, speed, pressed, now);
            if was_playing_round {
                elapsed += now.wrapping_sub(last) as u64;
                last = now;
            }
            if let Some((x, y, start)) = loss_indicator {
                board.rect(
                    x,
                    y,
                    gyro_breakout::game::BALL,
                    gyro_breakout::game::BALL,
                    render::BLACK,
                );
                if now.wrapping_sub(start) >= CYCLES_PER_SECOND / 5 {
                    loss_indicator = None;
                }
            }
            if let Some((x, y)) = game.lost_ball().filter(|_| !old.finished()) {
                loss_indicator = Some((x, y, now));
            }
            render::frame(&mut board, &old, &game);
            if let Some((x, y, _)) = loss_indicator {
                render::ball(&mut board, x, y, render::ORANGE);
            }
            next = next.wrapping_add(CYCLES_PER_SECOND / 100);
            let now = board::cycles();
            if (next.wrapping_sub(now) as i32) > 0 {
                while (next.wrapping_sub(board::cycles()) as i32) > 0 {
                    core::hint::spin_loop();
                }
            } else if now.wrapping_sub(next) > 3 * (CYCLES_PER_SECOND / 100) {
                next = now.wrapping_sub(3 * (CYCLES_PER_SECOND / 100));
            }
        }
        let win = game.remaining() == 0;
        let seconds = elapsed / CYCLES_PER_SECOND as u64;
        let rank = if win { scores.record(seconds) } else { None };
        render::result(&mut board, win, seconds, &scores, rank);
        board::delay(3000);
        if simulation {
            previous = false;
            continue;
        }
        loop {
            previous = input.update(board::button(), board::cycles());
            board::delay(20);
            if !previous {
                break;
            }
        }
        loop {
            previous = input.update(board::button(), board::cycles());
            board::delay(20);
            if previous {
                break;
            }
        }
    }
}
