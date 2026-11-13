//! STM32F429I-DISCO register-level drivers. Only this module touches MMIO.
use core::ptr::{read_volatile, write_volatile};
use gyro_breakout::game::CYCLES_PER_SECOND;
use gyro_breakout::render::Display;
mod touch;
const RCC: usize = 0x4002_3800;
const SPI: usize = 0x4001_5000;
const PC: usize = 0x4002_0800;
const PF: usize = 0x4002_1400;
const LTDC: usize = 0x4001_6800;
const DMA2D: usize = 0x4002_b000;
const RCC_DMA2D: u32 = 1 << 23;
const LAYER: usize = LTDC + 0x84;
const DWT_CONTROL: usize = 0xe000_1000;
const DWT_CYCLE_COUNT: usize = 0xe000_1004;
const DEBUG_CONTROL: usize = 0xe000_edfc;
const SYSTICK_CONTROL: usize = 0xe000_e010;
const SYSTICK_RELOAD: usize = 0xe000_e014;
const SYSTICK_CURRENT: usize = 0xe000_e018;
const SYSTICK_COUNTFLAG: u32 = 1 << 16;
const SPI_STATUS: usize = SPI + 8;
const SPI_DATA: usize = SPI + 12;
const SPI_RX_READY: u32 = 1;
const SPI_TX_READY: u32 = 1 << 1;
const SPI_BUSY: u32 = 1 << 7;
const SDRAM_BASE: usize = 0xd000_0000;
const FMC_SDRAM: usize = 0xa000_0140;
const FMC_COMMAND: usize = FMC_SDRAM + 0x10;
const FMC_REFRESH: usize = FMC_SDRAM + 0x14;
const FMC_STATUS: usize = FMC_SDRAM + 0x18;
const SDRAM_FRAME_PIXELS: usize = 320 * 240;
static mut FRAMEBUFFER: [[u16; 240]; 320] = [[0; 240]; 320];

/// A hidden SDRAM framebuffer used by the startup benchmark.
pub struct ParticleFrame {
    address: usize,
}

impl ParticleFrame {
    pub fn index(&self) -> usize {
        usize::from(self.address != SDRAM_BASE)
    }

    /// Fill the same rotated RGB565 rectangle with either the CPU or DMA2D.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, color: u16, dma: bool) -> bool {
        let x0 = x.clamp(0, 320);
        let y0 = y.clamp(0, 240);
        let x1 = (x + w).clamp(0, 320);
        let y1 = (y + h).clamp(0, 240);
        if x0 >= x1 || y0 >= y1 {
            return true;
        }
        if !dma {
            for col in x0..x1 {
                for row in y0..y1 {
                    let offset = ((319 - col) * 240 + row) as usize;
                    unsafe { write_volatile((self.address as *mut u16).add(offset), color) };
                }
            }
            return true;
        }
        // Framebuffer columns are physical rows, with a stride of 240 pixels.
        let width = (y1 - y0) as u32;
        let height = (x1 - x0) as u32;
        let offset = ((320 - x1) * 240 + y0) as u32 * 2;
        write(DMA2D + 0x08, 0x3f); // Clear completion and error flags.
        write(DMA2D + 0x34, 2); // RGB565 output.
        write(DMA2D + 0x38, u32::from(color));
        write(DMA2D + 0x3c, self.address as u32 + offset);
        write(DMA2D + 0x40, 240 - width);
        write(DMA2D + 0x44, (width << 16) | height);
        write(DMA2D, (3 << 16) | 1); // Register-to-memory fill, then start.
        if wait(DMA2D, 1, false) && read(DMA2D + 0x04) & 0x23 == 0x02 {
            return true;
        }
        // Reset a failed or stalled engine before either framebuffer is reused.
        set(RCC + 0x10, RCC_DMA2D);
        clear(RCC + 0x10, RCC_DMA2D);
        false
    }
}

impl Display for ParticleFrame {
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u16) {
        self.fill(x, y, w, h, color, false);
    }
}
#[inline]
fn read(a: usize) -> u32 {
    unsafe { read_volatile(a as *const u32) }
}
#[inline]
fn write(a: usize, v: u32) {
    unsafe { write_volatile(a as *mut u32, v) }
}
fn set(a: usize, v: u32) {
    write(a, read(a) | v)
}
fn clear(a: usize, v: u32) {
    write(a, read(a) & !v)
}
fn wait(a: usize, mask: u32, on: bool) -> bool {
    (0..1_000_000).any(|_| (read(a) & mask != 0) == on)
}
fn gpio(n: usize) -> usize {
    0x4002_0000 + n * 0x400
}
fn pin(port: usize, num: u32, mode: u32, af: u32) {
    let b = gpio(port);
    let s = num * 2;
    write(b, (read(b) & !(3 << s)) | (mode << s));
    set(b + 8, 3 << s);
    clear(b + 12, 3 << s);
    if mode == 2 {
        let a = b + 0x20 + (num as usize / 8) * 4;
        let s = (num % 8) * 4;
        write(a, (read(a) & !(15 << s)) | (af << s));
    }
}
pub fn cycles() -> u32 {
    read(DWT_CYCLE_COUNT)
}
fn start_cycle_counter() {
    set(DEBUG_CONTROL, 1 << 24);
    write(DWT_CYCLE_COUNT, 0);
    set(DWT_CONTROL, 1);
}
pub fn delay(ms: u32) {
    write(SYSTICK_RELOAD, CYCLES_PER_SECOND / 1000 - 1);
    write(SYSTICK_CURRENT, 0);
    write(SYSTICK_CONTROL, 5); // Enable SysTick using the processor clock.
    for _ in 0..ms {
        if !wait(SYSTICK_CONTROL, SYSTICK_COUNTFLAG, true) {
            fault();
        }
    }
    write(SYSTICK_CONTROL, 0);
}
pub fn button() -> bool {
    read(0x4002_0010) & 1 != 0
}
pub fn fault() -> ! {
    set(RCC + 0x30, 1 << 6);
    let _ = read(RCC + 0x30);
    clear(0x4002_1804, 1 << 14);
    write(0x4002_1800, (read(0x4002_1800) & !(3 << 28)) | (1 << 28));
    write(0x4002_1818, 1 << 14);
    loop {
        core::hint::spin_loop();
    }
}
fn clock() -> bool {
    set(RCC, 1);
    if !wait(RCC, 2, true) {
        return false;
    }
    clear(RCC + 8, 3);
    if !wait(RCC + 8, 12, false) {
        return false;
    }
    clear(RCC, (1 << 24) | (1 << 26) | (1 << 28));
    if !wait(RCC, (1 << 25) | (1 << 27) | (1 << 29), false) {
        return false;
    }
    clear(RCC, 1 << 16);
    if !wait(RCC, 1 << 17, false) {
        return false;
    }
    clear(RCC, 1 << 18);
    set(RCC, 1 << 16);
    if !wait(RCC, 1 << 17, true) {
        return false;
    }
    set(RCC + 0x40, 1 << 28);
    let _ = read(RCC + 0x40);
    set(0x4000_7000, 3 << 14);
    write(0x4002_3c00, (read(0x4002_3c00) & !7) | 5 | (7 << 8));
    write(
        RCC + 8,
        (read(RCC + 8) & !((15 << 4) | (7 << 10) | (7 << 13))) | (5 << 10) | (4 << 13),
    );
    // 8 MHz HSE / 8 * 336 / 2 = 168 MHz system clock; PLLQ = 7.
    write(RCC + 4, 8 | (336 << 6) | (1 << 22) | (7 << 24));
    set(RCC, 1 << 24);
    if !wait(RCC, 1 << 25, true) {
        return false;
    }
    write(RCC + 8, (read(RCC + 8) & !3) | 2);
    wait(RCC + 8, 1 << 3, true)
}
fn transfer(v: u8) -> Option<u8> {
    if !wait(SPI_STATUS, SPI_TX_READY, true) {
        return None;
    }
    unsafe { write_volatile(SPI_DATA as *mut u8, v) };
    if !wait(SPI_STATUS, SPI_RX_READY, true) {
        return None;
    }
    Some(unsafe { read_volatile(SPI_DATA as *const u8) })
}
fn transaction(cmd: u8, data: &mut [u8], reading: bool) -> bool {
    write(PC + 0x18, 1 << 17);
    let mut ok = transfer(cmd).is_some();
    for v in data {
        if !ok {
            break;
        }
        match transfer(if reading { 0xff } else { *v }) {
            Some(r) => {
                if reading {
                    *v = r
                }
            }
            None => ok = false,
        }
    }
    if ok {
        ok = wait(SPI_STATUS, SPI_BUSY, false)
    }
    write(PC + 0x18, 1 << 1);
    ok
}
fn axes() -> Option<[i16; 3]> {
    let mut d = [0; 6];
    if !transaction(0xe8, &mut d, true) {
        return None;
    }
    Some([
        i16::from_le_bytes([d[0], d[1]]),
        i16::from_le_bytes([d[2], d[3]]),
        i16::from_le_bytes([d[4], d[5]]),
    ])
}
pub struct Board {
    offsets: [i32; 3],
    history: [[i32; 2]; 3],
    totals: [i32; 3],
    index: usize,
    count: i32,
    pub seed: u32,
}
impl Board {
    pub fn init() -> Option<Self> {
        if !clock() {
            return None;
        }
        set(RCC + 0x30, 1 | (1 << 2));
        let _ = read(RCC + 0x30);
        clear(0x4002_0000, 3);
        clear(0x4002_000c, 3);
        write(PC + 0x18, 1 << 1);
        clear(PC + 4, 1 << 1);
        write(PC, (read(PC) & !(3 << 2)) | (1 << 2));
        if !lcd_init() {
            return None;
        }
        Some(Self {
            offsets: [0; 3],
            history: [[0; 2]; 3],
            totals: [0; 3],
            index: 0,
            count: 0,
            seed: 2166136261,
        })
    }
    /// Initialize the shared startup framebuffers and frame timing.
    pub fn startup_prepare(&mut self) -> bool {
        if !sdram_init() {
            return false;
        }
        start_cycle_counter();
        set(RCC + 0x30, RCC_DMA2D); // DMA2D clock.
        let _ = read(RCC + 0x30);
        true
    }
    pub fn startup_fractal(&mut self) {
        self.draw_mandelbrot(
            &gyro_breakout::fractal::Mandelbrot::default(),
            self.hidden_startup_buffer(),
        );
    }
    pub fn prepare_particle_background(&mut self) {
        let address = SDRAM_BASE + SDRAM_FRAME_PIXELS * 4;
        let view = gyro_breakout::fractal::Mandelbrot::default();
        for x in 0..320 {
            for y in 0..240 {
                let offset = ((319 - x) * 240 + y) as usize;
                unsafe { write_volatile((address as *mut u16).add(offset), view.pixel(x, y)) };
            }
        }
    }
    pub fn restore_particle_background(&mut self, frame: &mut ParticleFrame) {
        let source = SDRAM_BASE + SDRAM_FRAME_PIXELS * 4;
        write(DMA2D + 0x08, 0x3f);
        write(DMA2D + 0x0c, source as u32);
        write(DMA2D + 0x10, 0);
        write(DMA2D + 0x1c, 2);
        write(DMA2D + 0x34, 2);
        write(DMA2D + 0x3c, frame.address as u32);
        write(DMA2D + 0x40, 0);
        write(DMA2D + 0x44, (240 << 16) | 320);
        write(DMA2D, 1);
        if !wait(DMA2D, 1, false) || read(DMA2D + 0x04) & 0x23 != 0x02 {
            set(RCC + 0x10, RCC_DMA2D);
            clear(RCC + 0x10, RCC_DMA2D);
            for i in 0..SDRAM_FRAME_PIXELS {
                unsafe {
                    write_volatile(
                        (frame.address as *mut u16).add(i),
                        read_volatile((source as *const u16).add(i)),
                    )
                };
            }
        }
    }
    pub fn particle_frame(&mut self) -> ParticleFrame {
        ParticleFrame {
            address: self.hidden_startup_buffer(),
        }
    }
    pub fn show_particle_frame(&mut self, frame: &ParticleFrame) {
        write(LAYER + 0x28, frame.address as u32);
        write(LTDC + 0x24, 1);
    }
    fn draw_mandelbrot(&mut self, view: &gyro_breakout::fractal::Mandelbrot, address: usize) {
        self.draw_fractal_pixels(address, |x, y| view.pixel(x, y));
    }
    fn draw_fractal_pixels(&mut self, address: usize, pixel: impl Fn(i32, i32) -> u16) {
        let buffer = address as *mut u16;
        for x in 0..320 {
            for y in 0..240 {
                let offset = ((319 - x) * 240 + y) as usize;
                // SDRAM is initialized above; every offset is within the 150 KiB image.
                unsafe { write_volatile(buffer.add(offset), pixel(x, y)) };
            }
        }
        write(LAYER + 0x28, address as u32);
        write(LTDC + 0x24, 1); // Reload the new framebuffer address.
    }
    fn hidden_startup_buffer(&self) -> usize {
        if read(LAYER + 0x28) == SDRAM_BASE as u32 {
            SDRAM_BASE + SDRAM_FRAME_PIXELS * 2
        } else {
            SDRAM_BASE
        }
    }
    pub fn zoom_mandelbrot(&mut self, view: &gyro_breakout::fractal::Mandelbrot) {
        self.draw_mandelbrot(view, self.hidden_startup_buffer());
    }
    pub fn startup_julia(&mut self, view: &gyro_breakout::fractal::Julia) {
        self.draw_fractal_pixels(self.hidden_startup_buffer(), |x, y| view.pixel(x, y));
    }
    pub fn touch_init(&mut self) -> bool {
        touch::init()
    }
    pub fn touch(&mut self) -> Result<(bool, Option<(i32, i32)>), ()> {
        touch::sample()
    }
    /// Render into the SDRAM framebuffer that is not currently visible.
    pub fn startup_snowflake(&mut self, view: &gyro_breakout::fractal::Viewport) {
        let address = self.hidden_startup_buffer();
        let buffer = address as *mut u16;
        for offset in 0..SDRAM_FRAME_PIXELS {
            // The second 150 KiB framebuffer follows the first in initialized SDRAM.
            unsafe { write_volatile(buffer.add(offset), 0x0008) };
        }
        gyro_breakout::fractal::snowflake(view, |x, y| {
            for dx in 0..2 {
                for dy in 0..2 {
                    let (px, py) = (x + dx, y + dy);
                    if (0..320).contains(&px) && (0..240).contains(&py) {
                        let offset = ((319 - px) * 240 + py) as usize;
                        unsafe { write_volatile(buffer.add(offset), 0xafff) };
                    }
                }
            }
        });
        write(LAYER + 0x28, address as u32);
        write(LTDC + 0x24, 1);
    }
    /// Render fern views into the framebuffer that is not currently visible.
    pub fn startup_fern(&mut self, view: &gyro_breakout::fractal::Viewport) {
        let address = self.hidden_startup_buffer();
        let buffer = address as *mut u16;
        for offset in 0..SDRAM_FRAME_PIXELS {
            unsafe { write_volatile(buffer.add(offset), 0) };
        }
        gyro_breakout::fractal::fern(view, |x, y| {
            let offset = ((319 - x) * 240 + y) as usize;
            unsafe { write_volatile(buffer.add(offset), 0x07e0) };
        });
        write(LAYER + 0x28, address as u32);
        write(LTDC + 0x24, 1);
    }
    pub fn end_startup_fractal(&mut self) {
        write(LAYER + 0x28, core::ptr::addr_of!(FRAMEBUFFER) as u32);
        write(LTDC + 0x24, 1);
    }
    pub fn calibrate(&mut self) -> bool {
        write(PC + 0x18, (1 << 1) | (1 << 2));
        write(PC, (read(PC) & !(3 << 2)) | (1 << 2));
        clear(PC + 4, 1 << 1);
        write(PF, (read(PF) & !(3 << 16)) | (2 << 16));
        write(PF + 0x24, (read(PF + 0x24) & !15) | 5);
        let _ = read(SPI_DATA);
        let _ = read(SPI_STATUS);
        let mut id = [0];
        if !transaction(0x8f, &mut id, true) || !(id[0] == 0xd4 || id[0] == 0xd7) {
            return false;
        }
        if !transaction(0x20, &mut [0x0f], false) || !transaction(0x23, &mut [0x80], false) {
            return false;
        }
        delay(200);
        for _ in 0..32 {
            let Some(a) = axes() else { return false };
            for (i, v) in a.iter().enumerate() {
                self.offsets[i] += *v as i32;
                self.seed = (self.seed ^ (*v as u16 as u32)).wrapping_mul(16777619);
            }
            delay(12)
        }
        for v in &mut self.offsets {
            *v /= 32;
        }
        // Enable the Cortex-M DWT cycle counter for wrap-safe game timing.
        start_cycle_counter();
        true
    }
    pub fn rates(&mut self) -> Option<(i32, i32)> {
        let a = axes()?;
        self.count = (self.count + 1).min(2);
        let mut rate = [0; 3];
        for i in 0..3 {
            self.totals[i] -= self.history[i][self.index];
            self.history[i][self.index] = a[i] as i32 - self.offsets[i];
            self.totals[i] += self.history[i][self.index];
            rate[i] = (self.totals[i] / self.count * 875 / 100000).clamp(-250, 250);
        }
        self.index = (self.index + 1) % 2;
        Some((rate[0], rate[1]))
    }
}
fn sdram_command(command: u32) -> bool {
    write(FMC_COMMAND, command | (1 << 3)); // Target SDRAM bank 2.
    wait(FMC_STATUS, 1 << 5, false)
}

fn sdram_init() -> bool {
    // FMC signals for the Discovery board's 16-bit SDRAM, all AF12 push-pull.
    set(RCC + 0x30, 0x7e);
    set(RCC + 0x38, 1); // FMC clock.
    let _ = read(RCC + 0x38);
    let pins = [
        (1, 5),
        (1, 6),
        (2, 0),
        (3, 0),
        (3, 1),
        (3, 8),
        (3, 9),
        (3, 10),
        (3, 14),
        (3, 15),
        (4, 0),
        (4, 1),
        (4, 7),
        (4, 8),
        (4, 9),
        (4, 10),
        (4, 11),
        (4, 12),
        (4, 13),
        (4, 14),
        (4, 15),
        (5, 0),
        (5, 1),
        (5, 2),
        (5, 3),
        (5, 4),
        (5, 5),
        (5, 11),
        (5, 12),
        (5, 13),
        (5, 14),
        (5, 15),
        (6, 0),
        (6, 1),
        (6, 4),
        (6, 5),
        (6, 8),
        (6, 15),
    ];
    for (port, number) in pins {
        clear(gpio(port) + 4, 1 << number);
        pin(port, number, 2, 12);
    }

    // Shared clock/read pipe, then bank 2: 8 column bits, 12 row bits,
    // 16-bit data, four internal banks, CAS latency 3; SDRAM clock = 84 MHz.
    write(FMC_SDRAM, (1 << 11) | (1 << 13));
    write(FMC_SDRAM + 4, (1 << 2) | (1 << 4) | (1 << 6) | (3 << 7));
    // Timing fields are in clock cycles minus one: TMRD=2, TXSR=7,
    // TRAS=4, TRC=7, TWR=2, TRP=2, TRCD=2.
    write(FMC_SDRAM + 8, (6 << 12) | (1 << 20));
    write(
        FMC_SDRAM + 12,
        1 | (6 << 4) | (3 << 8) | (1 << 16) | (1 << 24),
    );
    if !sdram_command(1) {
        return false;
    } // Clock enable.
    delay(1);
    if !sdram_command(2) || !sdram_command(3 | (3 << 5)) {
        return false;
    }
    if !sdram_command(4 | (0x230 << 9)) {
        return false;
    } // CAS 3, burst length 1.
    write(FMC_REFRESH, 1292 << 1); // 15.625 µs at 84 MHz, less 20 clocks.

    // Verify both ends of the two image buffers before the display controller uses them.
    let first = SDRAM_BASE as *mut u16;
    let last = (SDRAM_BASE + (2 * SDRAM_FRAME_PIXELS - 1) * 2) as *mut u16;
    unsafe {
        write_volatile(first, 0x55aa);
        write_volatile(last, 0xaa55);
        read_volatile(first) == 0x55aa && read_volatile(last) == 0xaa55
    }
}
impl Display for Board {
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        let x0 = x.clamp(0, 320);
        let y0 = y.clamp(0, 240);
        let x1 = (x + w).clamp(0, 320);
        let y1 = (y + h).clamp(0, 240);
        let base = core::ptr::addr_of_mut!(FRAMEBUFFER) as *mut u16;
        for col in x0..x1 {
            for row in y0..y1 {
                unsafe { write_volatile(base.add(((319 - col) * 240 + row) as usize), c) }
            }
        }
    }
}
fn lcd_byte(v: u8, data: bool) -> bool {
    write(gpio(3) + 0x18, 1 << if data { 13 } else { 29 });
    write(PC + 0x18, 1 << 18);
    let ok = transfer(v).is_some() && wait(SPI_STATUS, SPI_BUSY, false);
    write(PC + 0x18, 1 << 2);
    ok
}
fn command(cmd: u8, data: &[u8]) -> bool {
    if !lcd_byte(cmd, false) {
        return false;
    }
    for v in data {
        if !lcd_byte(*v, true) {
            return false;
        }
    }
    true
}
fn lcd_init() -> bool {
    // Enable GPIO ports A, B, C, D, F, and G, plus SPI5 and LTDC clocks.
    set(RCC + 0x30, 0x6f);
    set(RCC + 0x44, (1 << 20) | (1 << 26));
    let _ = read(RCC + 0x44);
    // PC2 selects the LCD and PD13 selects command/data; PF7/PF9 are SPI5.
    pin(2, 2, 1, 0);
    pin(3, 13, 1, 0);
    write(PC + 0x18, 1 << 2);
    pin(5, 7, 2, 5);
    pin(5, 9, 2, 5);
    write(SPI, (1 << 2) | (1 << 8) | (1 << 9) | (4 << 3) | (1 << 6));
    delay(50);
    // Panel power, timing, pixel format, orientation, address window, and gamma setup.
    let commands: &[(u8, &[u8])] = &[
        (0xca, &[0xc3, 8, 0x50]),
        (0xcf, &[0, 0xc1, 0x30]),
        (0xed, &[0x64, 3, 0x12, 0x81]),
        (0xe8, &[0x85, 0, 0x78]),
        (0xcb, &[0x39, 0x2c, 0, 0x34, 2]),
        (0xf7, &[0x20]),
        (0xea, &[0, 0]),
        (0xb1, &[0, 0x1b]),
        (0xb6, &[0x0a, 0xa2]),
        (0xc0, &[0x10]),
        (0xc1, &[0x10]),
        (0xc5, &[0x45, 0x15]),
        (0xc7, &[0x90]),
        (0x36, &[0xc8]),
        (0xf2, &[0]),
        (0xb0, &[0xc2]),
        (0xb6, &[0x0a, 0xa7, 0x27, 4]),
        (0x2a, &[0, 0, 0, 0xef]),
        (0x2b, &[0, 0, 1, 0x3f]),
        (0xf6, &[1, 0, 6]),
        (0x26, &[1]),
        (
            0xe0,
            &[
                0x0f, 0x29, 0x24, 0x0c, 0x0e, 9, 0x4e, 0x78, 0x3c, 9, 0x13, 5, 0x17, 0x11, 0,
            ],
        ),
        (
            0xe1,
            &[
                0, 0x16, 0x1b, 4, 0x11, 7, 0x31, 0x33, 0x42, 5, 0x0c, 0x0a, 0x28, 0x2f, 0x0f,
            ],
        ),
        (0x11, &[]), // Sleep out; allow the panel to stabilize before display on.
    ];
    for (c, d) in commands {
        if !command(*c, d) {
            return false;
        }
    }
    delay(200);
    if !command(0x29, &[]) {
        // Display on.
        return false;
    }
    // GPIO port index, pin, and alternate-function number for the LTDC RGB interface.
    let pins = [
        (0, 3, 14),
        (0, 4, 14),
        (0, 6, 14),
        (0, 11, 14),
        (0, 12, 14),
        (1, 0, 9),
        (1, 1, 9),
        (1, 8, 14),
        (1, 9, 14),
        (1, 10, 14),
        (1, 11, 14),
        (2, 6, 14),
        (2, 7, 14),
        (2, 10, 14),
        (3, 3, 14),
        (3, 6, 14),
        (5, 10, 14),
        (6, 6, 14),
        (6, 7, 14),
        (6, 10, 9),
        (6, 11, 14),
        (6, 12, 9),
    ];
    for (p, n, a) in pins {
        pin(p, n, 2, a)
    }
    // Configure and start PLLSAI, which supplies the LTDC pixel clock.
    write(RCC + 0x88, (192 << 6) | (7 << 24) | (4 << 28));
    write(RCC + 0x8c, (read(RCC + 0x8c) & !(3 << 16)) | (2 << 16));
    set(RCC, 1 << 28);
    if !wait(RCC, 1 << 29, true) {
        return false;
    }
    // Program sync, porch, active window, and total dimensions for the 240x320 panel.
    write(LTDC + 8, (9 << 16) | 1);
    write(LTDC + 0xc, (29 << 16) | 3);
    write(LTDC + 0x10, (269 << 16) | 323);
    write(LTDC + 0x14, (279 << 16) | 327);
    write(LAYER + 4, (269 << 16) | 30);
    write(LAYER + 8, (323 << 16) | 4);
    write(LAYER + 0x10, 2); // RGB565 framebuffer.
    write(LAYER + 0x14, 255);
    write(LAYER + 0x1c, (4 << 8) | 5);
    write(LAYER + 0x28, core::ptr::addr_of!(FRAMEBUFFER) as u32);
    write(LAYER + 0x2c, (480 << 16) | 483);
    write(LAYER + 0x30, 320);
    write(LAYER, 1);
    write(LTDC + 0x24, 1);
    set(LTDC + 0x18, 1);
    true
}
