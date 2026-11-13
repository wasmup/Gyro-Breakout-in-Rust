//! STMPE811 on the Discovery's PA8/PC9 touch bus, polled during startup.
//! Software I²C keeps this optional peripheral independent of the game drivers.
use super::{delay, gpio, pin, read, set, write, PC};

const PA: usize = 0x4002_0000;
const SCL: u32 = 1 << 8;
const SDA: u32 = 1 << 9;
const ADDRESS: u8 = 0x82;

fn pause() {
    // At 168 MHz this exceeds the standard-mode I²C half-period.
    for _ in 0..500 {
        unsafe { core::arch::asm!("nop", options(nomem, nostack, preserves_flags)) }
    }
}
fn data(high: bool) {
    write(PC + 0x18, if high { SDA } else { SDA << 16 });
}
fn clock(high: bool) -> bool {
    write(PA + 0x18, if high { SCL } else { SCL << 16 });
    if high && !(0..10_000).any(|_| read(PA + 0x10) & SCL != 0) {
        return false;
    }
    pause();
    true
}
fn start() -> bool {
    data(true);
    if !clock(true) || read(PC + 0x10) & SDA == 0 {
        return false;
    }
    data(false);
    pause();
    clock(false)
}
fn stop() {
    data(false);
    let _ = clock(true);
    data(true);
    pause();
}
fn send(byte: u8) -> bool {
    for bit in (0..8).rev() {
        data(byte & (1 << bit) != 0);
        if !clock(true) {
            return false;
        }
        clock(false);
    }
    data(true);
    if !clock(true) {
        return false;
    }
    let ack = read(PC + 0x10) & SDA == 0;
    clock(false);
    ack
}
fn receive(ack: bool) -> Option<u8> {
    let mut byte = 0;
    data(true);
    for _ in 0..8 {
        if !clock(true) {
            return None;
        }
        byte = (byte << 1) | u8::from(read(PC + 0x10) & SDA != 0);
        clock(false);
    }
    data(!ack);
    if !clock(true) {
        return None;
    }
    clock(false);
    data(true);
    Some(byte)
}
fn write_reg(reg: u8, value: u8) -> bool {
    let ok = start() && send(ADDRESS) && send(reg) && send(value);
    stop();
    ok
}
fn read_regs(reg: u8, output: &mut [u8]) -> bool {
    let mut ok = start() && send(ADDRESS) && send(reg) && start() && send(ADDRESS | 1);
    let len = output.len();
    for (i, byte) in output.iter_mut().enumerate() {
        if !ok {
            break;
        }
        if let Some(value) = receive(i + 1 < len) {
            *byte = value;
        } else {
            ok = false;
        }
    }
    stop();
    ok
}
fn read_reg(reg: u8) -> Option<u8> {
    let mut byte = [0];
    read_regs(reg, &mut byte).then_some(byte[0])
}

pub fn init() -> bool {
    // Both GPIO clocks are already enabled; release the open-drain lines first.
    data(true);
    write(PA + 0x18, SCL);
    set(gpio(0) + 4, SCL);
    set(gpio(2) + 4, SDA);
    pin(0, 8, 1, 0);
    pin(2, 9, 1, 0);
    if read_reg(0) != Some(0x08) || read_reg(1) != Some(0x11) {
        return false;
    }
    if !write_reg(0x03, 0x02) {
        return false;
    }
    delay(10);
    if !write_reg(0x03, 0) {
        return false;
    }
    delay(2);
    // Enable ADC, touch and GPIO clocks, route pins to touch, configure the FIFO.
    for (reg, value) in [
        (0x04, 0x00),
        (0x17, 0x00),
        (0x20, 0x48),
        (0x21, 0x01),
        (0x41, 0x9a),
        (0x4a, 0x01),
        (0x4b, 0x01),
        (0x4b, 0x00),
        (0x56, 0x01),
        (0x58, 0x01),
        (0x40, 0x03),
    ] {
        if !write_reg(reg, value) {
            return false;
        }
    }
    true
}

pub fn sample() -> Result<(bool, Option<(i32, i32)>), ()> {
    let pressed = read_reg(0x40).ok_or(())? & 0x80 != 0;
    if !pressed {
        // Drop old samples so the next press cannot reuse a stale position.
        if !write_reg(0x4b, 1) || !write_reg(0x4b, 0) {
            return Err(());
        }
        return Ok((false, None));
    }
    if read_reg(0x4c).ok_or(())? == 0 {
        return Ok((true, None));
    }
    let mut bytes = [0; 3];
    if !read_regs(0xd7, &mut bytes) || !write_reg(0x4b, 1) || !write_reg(0x4b, 0) {
        return Err(());
    }
    let raw_x = (i32::from(bytes[0]) << 4) | (i32::from(bytes[1]) >> 4);
    let raw_y = ((i32::from(bytes[1]) & 15) << 8) | i32::from(bytes[2]);
    // ST's default DISCO calibration is in portrait coordinates. The project's
    // framebuffer rotates portrait counterclockwise into 320 × 240 landscape.
    let portrait_x = ((3870 - raw_x) / 15).clamp(0, 239);
    let portrait_y = ((raw_y - 360) / 11).clamp(0, 319);
    Ok((true, Some((319 - portrait_y, portrait_x))))
}
