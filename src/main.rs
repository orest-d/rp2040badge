//! # GPIO 'Blinky' Example
//!
//! This application demonstrates how to control a GPIO pin on the RP2040.
//!
//! It may need to be adapted to your particular board layout and/or pin assignment.
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]

use embedded_graphics::primitives::{Circle, PrimitiveStyleBuilder, Triangle};
// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// Alias for our HAL crate
use rp2040_hal as hal;

// Some traits we need
use embedded_hal::digital::v2::OutputPin;
use fugit::RateExtU32;
use rp2040_hal::clocks::Clock;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use display_interface::{DataFormat, WriteOnlyDataCommand};
use display_interface_spi::SPIInterface;
use hal::pac;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
/// Note: This boot block is not necessary when using a rp-hal based BSP
/// as the BSPs already perform this step.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
/// if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Entry point to our bare-metal application.
///
/// The `#[rp2040_hal::entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables and the spinlock are initialised.
///
/// The function configures the RP2040 peripherals, then toggles a GPIO pin in
/// an infinite loop. If there is an LED connected to that pin, it will blink.
/// The function configures the RP2040 peripherals, then performs some example
/// SPI transactions, then goes to sleep.

const HORIZONTAL_SCAN_DIR: bool = true;
const LCD_HEIGHT: u8 = 240;
const LCD_WIDTH: u8 = 240;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics_core::pixelcolor::raw::RawU16;
use embedded_graphics_core::{draw_target::DrawTarget, Pixel};

struct Random(u32);

impl Random {
    fn new() -> Random {
        Random(12345)
    }
    fn get_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }
    fn get_u16(&mut self) -> u16 {
        ((self.get_u32() / 23) & 0xFFFF) as u16
    }
    fn get_u8(&mut self) -> u8 {
        ((self.get_u32() / 23) & 0xFF) as u8
    }
}

struct Lcd<T: WriteOnlyDataCommand>(T);

fn wave(x: i32, period: i32, amplitude: i32) -> i32 {
    let x = (if x > 0 { x } else { period - x }) % (2 * period);
    let (x, sign) = if x < period { (x, 1) } else { (x - period, -1) };
    sign * x * (period - x) * amplitude / period / period / 4
}

fn wave2(x: i32, period: i32, amplitude: i32) -> i32 {
    let w = wave(x, period, 128);
    let ww = amplitude * w * w / 128;
    if x > 0 {
        ww / 16
    } else {
        -ww / 16
    }
}

impl<T: WriteOnlyDataCommand> Lcd<T> {
    fn init(&mut self, delay: &mut cortex_m::delay::Delay) {
        let iface = &mut self.0;
        /* Set the resolution and scanning method of the screen */

        let memory_access_reg = if HORIZONTAL_SCAN_DIR { 0xC8u8 } else { 0x68u8 };
        iface.send_commands(DataFormat::U8(&[0x36])).unwrap();
        iface
            .send_data(DataFormat::U8(&[memory_access_reg]))
            .unwrap();

        /* Initialize lcd registers */
        iface.send_commands(DataFormat::U8(&[0xEF, 0xEB])).unwrap();
        iface.send_data(DataFormat::U8(&[0x14])).unwrap();
        iface
            .send_commands(DataFormat::U8(&[0xFE, 0xEF, 0xEB]))
            .unwrap();
        iface.send_data(DataFormat::U8(&[0x14])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x84])).unwrap();
        iface.send_data(DataFormat::U8(&[0x40])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x85])).unwrap();
        iface.send_data(DataFormat::U8(&[0xFF])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x86])).unwrap();
        iface.send_data(DataFormat::U8(&[0xFF])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x87])).unwrap();
        iface.send_data(DataFormat::U8(&[0xFF])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x88])).unwrap();
        iface.send_data(DataFormat::U8(&[0x0A])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x89])).unwrap();
        iface.send_data(DataFormat::U8(&[0x21])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x8A])).unwrap();
        iface.send_data(DataFormat::U8(&[0x00])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x8B])).unwrap();
        iface.send_data(DataFormat::U8(&[0x80])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x8C])).unwrap();
        iface.send_data(DataFormat::U8(&[0x01])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x8D])).unwrap();
        iface.send_data(DataFormat::U8(&[0x01])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x8E])).unwrap();
        iface.send_data(DataFormat::U8(&[0xFF])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x8F])).unwrap();
        iface.send_data(DataFormat::U8(&[0xFF])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xB6])).unwrap();
        iface.send_data(DataFormat::U8(&[0x00, 0x20])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x36])).unwrap();
        iface.send_data(DataFormat::U8(&[0x08])).unwrap(); //Set as vertical screen

        iface.send_commands(DataFormat::U8(&[0x3A])).unwrap();
        iface.send_data(DataFormat::U8(&[0x05])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x90])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x08, 0x08, 0x08, 0x08]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xBD])).unwrap();
        iface.send_data(DataFormat::U8(&[0x06])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xBC])).unwrap();
        iface.send_data(DataFormat::U8(&[0x00])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xFF])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x60, 0x01, 0x04]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xC3])).unwrap();
        iface.send_data(DataFormat::U8(&[0x13])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xC4])).unwrap();
        iface.send_data(DataFormat::U8(&[0x13])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xC9])).unwrap();
        iface.send_data(DataFormat::U8(&[0x22])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xBE])).unwrap();
        iface.send_data(DataFormat::U8(&[0x11])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xE1])).unwrap();
        iface.send_data(DataFormat::U8(&[0x10, 0x0E])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xDF])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x21, 0x0C, 0x02]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xF0])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x45, 0x09, 0x08, 0x08, 0x26, 0x2A]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xF1])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x43, 0x70, 0x72, 0x36, 0x37, 0x6F]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xF2])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x45, 0x09, 0x08, 0x08, 0x26, 0x2A]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xF3])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x43, 0x70, 0x72, 0x36, 0x37, 0x6F]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xED])).unwrap();
        iface.send_data(DataFormat::U8(&[0x1B, 0x0B])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xAE])).unwrap();
        iface.send_data(DataFormat::U8(&[0x77])).unwrap();

        iface.send_commands(DataFormat::U8(&[0xCD])).unwrap();
        iface.send_data(DataFormat::U8(&[0x63])).unwrap();
        iface.send_commands(DataFormat::U8(&[0x70])).unwrap();
        iface
            .send_data(DataFormat::U8(&[
                0x07, 0x07, 0x04, 0x0E, 0x0F, 0x09, 0x07, 0x08, 0x03,
            ]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0xE8])).unwrap();
        iface.send_data(DataFormat::U8(&[0x34])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x62])).unwrap();
        iface
            .send_data(DataFormat::U8(&[
                0x18, 0x0D, 0x71, 0xED, 0x70, 0x70, 0x18, 0x0F, 0x71, 0xEF, 0x70, 0x70,
            ]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0x63])).unwrap();
        iface
            .send_data(DataFormat::U8(&[
                0x18, 0x11, 0x71, 0xF1, 0x70, 0x70, 0x18, 0x13, 0x71, 0xF3, 0x70, 0x70,
            ]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0x64])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x28, 0x29, 0xF1, 0x01, 0xF1, 0x00, 0x07]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0x66])).unwrap();
        iface
            .send_data(DataFormat::U8(&[
                0x3C, 0x00, 0xCD, 0x67, 0x45, 0x45, 0x10, 0x00, 0x00, 0x00,
            ]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0x67])).unwrap();
        iface
            .send_data(DataFormat::U8(&[
                0x00, 0x3C, 0x00, 0x00, 0x00, 0x01, 0x54, 0x10, 0x32, 0x98,
            ]))
            .unwrap();
        iface.send_commands(DataFormat::U8(&[0x74])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x10, 0x85, 0x80, 0x00, 0x00, 0x4E, 0x00]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0x98])).unwrap();
        iface.send_data(DataFormat::U8(&[0x3E, 0x07])).unwrap();

        iface.send_commands(DataFormat::U8(&[0x35])).unwrap();
        iface.send_data(DataFormat::U8(&[0x21])).unwrap();
        iface.send_commands(DataFormat::U8(&[0x11])).unwrap();
        delay.delay_ms(120);

        iface.send_commands(DataFormat::U8(&[0x29])).unwrap();
        delay.delay_ms(20);

        iface.send_commands(DataFormat::U8(&[0x21])).unwrap(); // Inversion in
    }
    fn set_windows(&mut self, x_start: u8, y_start: u8, x_end: u8, y_end: u8) {
        let iface = &mut self.0;

        //set the X coordinates
        iface.send_commands(DataFormat::U8(&[0x2A])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x00, x_start, 0x00, x_end - 1]))
            .unwrap();

        //set the Y coordinates
        iface.send_commands(DataFormat::U8(&[0x2B])).unwrap();
        iface
            .send_data(DataFormat::U8(&[0x00, y_start, 0x00, y_end - 1]))
            .unwrap();

        iface.send_commands(DataFormat::U8(&[0x2C])).unwrap();
    }

    fn raw_rectangle(&mut self, x_start: u8, y_start: u8, x_end: u8, y_end: u8, color: u16) {
        self.set_windows(x_start, y_start, x_end, y_end);
        let size = (x_end - x_start) as u16 * (y_end - y_start) as u16;
        let iface = &mut self.0;
        iface
            .send_data(DataFormat::U16BEIter(
                &mut (0..size).into_iter().map(|_| color),
            ))
            .unwrap();
    }

    fn show_image(&mut self, x: u8, y: u8, img: &impl MyImage) {
        self.set_windows(x, y, x + img.width(), y + img.height());
        let iface = &mut self.0;
        iface.send_data(DataFormat::U8(img.buffer())).unwrap();
    }

    fn full_image_noisy1(&mut self, img: &impl MyImage, random: &mut Random) {
        const together: usize = 40;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / together;

        for i in 0..400000 {
            let x = random.get_u8() % LCD_WIDTH;
            let y = random.get_u8() % LCD_HEIGHT;
            let offset = 2 * ((y as usize) * (LCD_WIDTH as usize) + (x as usize));
            self.set_windows(x, y, x + 1, y + 1);
            let iface = &mut self.0;
            iface
                .send_data(DataFormat::U8(&img.buffer()[offset..(offset + 2)]))
                .unwrap();
        }
        self.full_image(img);
    }

    fn full_image_noisy20(&mut self, img: &impl MyImage, random: &mut Random) {
        const together: u8 = 20;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);

        for i in 0..10000 {
            let x = random.get_u8() % (LCD_WIDTH - together);
            let y = random.get_u8() % LCD_HEIGHT;
            let offset = 2 * ((y as usize) * (LCD_WIDTH as usize) + (x as usize));
            self.set_windows(x, y, x + together, y + 1);
            let iface = &mut self.0;
            iface
                .send_data(DataFormat::U8(
                    &img.buffer()[offset..(offset + 2 * (together as usize))],
                ))
                .unwrap();
        }
        self.full_image(img);
    }

    fn full_image_tri(&mut self, img: &impl MyImage) {
        const together: u8 = 30;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);
        let mut buffer = [0u16; LCD_WIDTH as usize];
        //self.clear(LcdColor::BLACK);
        for t in 0..=together {
            let tt = together - t;
            for y in 0..LCD_HEIGHT - tt {
                for x in 0..(LCD_WIDTH - tt) {
                    buffer[x as usize] = img.get_pixel_u16(x, y)
                        | img.get_pixel_u16(x + tt, y) & img.get_pixel_u16(x, y + tt);
                }
                self.set_windows(0, y, LCD_WIDTH, y + 1);
                let iface = &mut self.0;
                iface.send_data(DataFormat::U16(&buffer)).unwrap();
            }
        }
    }
    fn full_image_wave1(&mut self, img: &impl MyImage) {
        const together: i32 = 150;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);
        let mut buffer = [0u16; LCD_WIDTH as usize];
        //self.clear(LcdColor::BLACK);
        for t in 0..=together {
            let tt = together - t;
            for y in 0..LCD_HEIGHT {
                for x in 0..(LCD_WIDTH) {
                    let r2 = ((x as i32 - 120) * (x as i32 - 120)
                        + (y as i32 - 120) * (y as i32 - 120))
                        / (10 + t);
                    let w1 = wave((x as i32) + 5 * (t as i32) + r2, 30 + t / 2, tt);
                    let w2 = wave((x as i32) + r2 / 2, 20 + t, 2 * tt);
                    let xx = (x as i32) + w1;
                    let yy = (y as i32) + w2;
                    let xx = if xx >= 0 && xx < 240 && yy >= 0 && yy < 240 {
                        buffer[x as usize] = img.get_pixel_u16(xx as u8, yy as u8);
                    } else {
                        buffer[x as usize] = 0;
                    };
                }
                self.set_windows(0, y, LCD_WIDTH, y + 1);
                let iface = &mut self.0;
                iface.send_data(DataFormat::U16(&buffer)).unwrap();
            }
        }
    }
    fn full_image_wave(&mut self, img: &impl MyImage) {
        const together: i32 = 150;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);
        let mut buffer = [0u16; LCD_WIDTH as usize];
        //self.clear(LcdColor::BLACK);
        for t in 0..=together {
            let tt = together - t;
            let ys = (0..LCD_HEIGHT/2).map(|i| 2*i).chain((0..LCD_HEIGHT/2).map(|i| 2*i+1));
            for y in ys {
                for x in 0..(LCD_WIDTH) {
                    let r2 = ((x as i32 - 120) * (x as i32 - 120)
                        + (y as i32 - 120) * (y as i32 - 120))
                        / (10 + t);
                    let w1 = wave((x as i32) + 5 * (t as i32) + r2, 30 + t / 2, tt);
                    let w2 = wave((x as i32) + r2 / 2, 20 + t, 2 * tt);
                    let xx = (x as i32) + w1;
                    let yy = (y as i32) + w2;
                    let xx = if xx >= 0 && xx < 240 && yy >= 0 && yy < 240 {
                        buffer[x as usize] = img.get_pixel_u16(xx as u8, yy as u8);
                    } else {
                        buffer[x as usize] = 0;
                    };
                }
                self.set_windows(0, y, LCD_WIDTH, y + 1);
                let iface = &mut self.0;
                iface.send_data(DataFormat::U16(&buffer)).unwrap();
            }
        }
    }

    fn full_image_rot(&mut self, img: &impl MyImage) {
        const together: i32 = 200;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);
        let mut buffer = [0u16; LCD_WIDTH as usize];
        //self.clear(LcdColor::BLACK);
        for t in 0..=together {
            let tt = together - t;
            let ys = (0..LCD_HEIGHT/2).map(|i| 2*i).chain((0..LCD_HEIGHT/2).map(|i| 2*i+1));
            for y in ys {
                for x in 0..(LCD_WIDTH) {
                    let r2 = ((x as i32 - 120) * (x as i32 - 120)
                        + (y as i32 - 120) * (y as i32 - 120))
                        / (1 + 5*t);
                    let r3 = ((x as i32 - 119) * (x as i32 - 119)
                        + (y as i32 - 120) * (y as i32 - 120))
                        / (1 + t);
                    let r4 = ((x as i32 - 122) * (x as i32 - 120)
                        + (y as i32 - 120) * (y as i32 - 120))
                        / (10 + 10*t);
                    let dx = 120-x as i32;
                    let dy = 120 -y as i32;
                    let xx = (x as i32) + dy*tt/50;
                    let yy = (y as i32) - dx*tt/50;
                    let xx = if xx >= 0 && xx < 240 && yy >= 0 && yy < 240 {
                        buffer[x as usize] = img.get_pixel_u16(xx as u8, yy as u8);
                    } else {
                        buffer[x as usize] = (r2 as u16)|(r3 as u16)|(r4 as u16);
                    };
                }
                self.set_windows(0, y, LCD_WIDTH, y + 1);
                let iface = &mut self.0;
                iface.send_data(DataFormat::U16(&buffer)).unwrap();
            }
        }
    }

    fn full_image_logic(&mut self, img: &impl MyImage) {
        const together: i32 = 50;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);
        let mut buffer = [0u16; LCD_WIDTH as usize];
        //self.clear(LcdColor::BLACK);
        for t in 0..=together {
            let tt = together - t;
            for y in 0..LCD_HEIGHT {
                for x in 0..(LCD_WIDTH) {
                    let x1 = x as i32 - tt;
                    let x2 = x as i32 + tt;
                    let y1 = y as i32 - tt;
                    let y2 = y as i32 + tt;
                    let xx = if x1 >= 0
                        && x1 < 240
                        && x2 >= 0
                        && x2 < 240
                        && y1 >= 0
                        && y1 < 240
                        && y2 >= 0
                        && y2 < 240
                    {
                        buffer[x as usize] = (img.get_pixel_u16(x1 as u8, y as u8)
                            | img.get_pixel_u16(x2 as u8, y as u8)
                            | img.get_pixel_u16(x as u8, y1 as u8)
                            | img.get_pixel_u16(x as u8, y2 as u8));
                    } else {
                        buffer[x as usize] = 0xFFFF;
                    };
                }
                self.set_windows(0, y, LCD_WIDTH, y + 1);
                let iface = &mut self.0;
                iface.send_data(DataFormat::U16(&buffer)).unwrap();
            }
        }
    }

    fn full_image_logictri(&mut self, img: &impl MyImage) {
        const together: i32 = 50;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);
        let mut buffer = [0u16; LCD_WIDTH as usize];
        //self.clear(LcdColor::BLACK);
        for t in 0..=together {
            let tt = together - t;
            for y in 0..LCD_HEIGHT {
                for x in 0..(LCD_WIDTH) {
                    let x1 = x as i32 - tt;
                    let x2 = x as i32 + tt;
                    let y1 = y as i32 - tt;
                    let y2 = y as i32 + tt;
                    let xx = if x1 >= 0
                        && x1 < 240
                        && x2 >= 0
                        && x2 < 240
                        && y1 >= 0
                        && y1 < 240
                        && y2 >= 0
                        && y2 < 240
                    {
                        buffer[x as usize] = img.get_pixel_u16(x1 as u8, y as u8)
                            & img.get_pixel_u16(x2 as u8, y as u8)
                            & img.get_pixel_u16(x as u8, y1 as u8);
                    } else {
                        buffer[x as usize] = 0;
                    };
                }
                self.set_windows(0, y, LCD_WIDTH, y + 1);
                let iface = &mut self.0;
                iface.send_data(DataFormat::U16(&buffer)).unwrap();
            }
        }
    }

    fn full_image_noisy(&mut self, img: &impl MyImage, random: &mut Random) {
        const together: u8 = 11;
        const length: usize = (LCD_WIDTH as usize) * (LCD_HEIGHT as usize) / (together as usize);

        let mut f = |d| {
            let x = random.get_u8() % (LCD_WIDTH - together - d);
            let y = random.get_u8() % (LCD_HEIGHT - d);
            let ox = random.get_u8() % d;
            let oy = random.get_u8() % d;
            let offset = 2 * ((y as usize) * (LCD_WIDTH as usize) + (x as usize));
            self.set_windows(x + ox, y + oy, x + ox + together, y + oy + 1);
            let iface = &mut self.0;
            iface
                .send_data(DataFormat::U8(
                    &img.buffer()[offset..(offset + 2 * (together as usize))],
                ))
                .unwrap();
        };
        for i in 0..20000 {
            f(10);
        }
        for i in 0..20000 {
            f(5);
        }
        for i in 0..20000 {
            f(2);
        }
        self.full_image(img);
    }

    fn full_image_interlaced(&mut self, img: &impl MyImage) {
        let mut f = |ox, oy| {
            for i in 0..LCD_HEIGHT / 4 - 4 {
                let x = 0;
                let y = oy + i * 4;
                let offset = 2 * ((y as usize) * (LCD_WIDTH as usize) + (x as usize));
                self.set_windows(x + ox, y, x + LCD_WIDTH - ox, y + 1);
                let iface = &mut self.0;
                iface
                    .send_data(DataFormat::U8(
                        &img.buffer()[offset..(offset + 2 * ((LCD_WIDTH - ox) as usize))],
                    ))
                    .unwrap();
                self.set_windows(x, y + 1, x + LCD_WIDTH, y + 2);
                let iface = &mut self.0;
                iface
                    .send_data(DataFormat::U8(
                        &img.buffer()[offset..(offset + 2 * ((LCD_WIDTH - ox) as usize))],
                    ))
                    .unwrap();
            }
        };

        for j in 0..16 {
            f(15 - j, 0);
        }
        f(0, 2);
        for j in 0..16 {
            f(j, 0);
            f(15 - j, 2);
        }
        f(0, 0);
        for j in 0..16 {
            f(15 - j, 0);
            f(j, 2);
        }
        f(0, 0);
        for j in 0..16 {
            f(15 - j, 0);
            f(15 - j, 2);
        }

        self.full_image(img);
    }

    fn show_image_clamped(&mut self, x: u8, y: u8, img: &impl MyImage, clamp: u8) {
        let h = clamp.min(img.height());
        self.set_windows(x, y, x + img.width(), y + h);
        let iface = &mut self.0;
        iface.send_data(DataFormat::U8(img.buffer())).unwrap();
    }

    fn full_image(&mut self, image_buffer: &impl MyImage) {
        let image = image_buffer.buffer();
        self.set_windows(0, 0, LCD_WIDTH, LCD_HEIGHT);
        let iface = &mut self.0;
        iface.send_data(DataFormat::U8(image)).unwrap();
    }
    fn full_image_horizontal_shift(&mut self, image_buffer: impl MyImage, offset: u8) {
        let image = image_buffer.buffer();
        for i in 0..LCD_HEIGHT {
            self.set_windows(0, i, LCD_WIDTH - offset, i + 1);

            self.0
                .send_data(DataFormat::U8(
                    &image[2 * ((i as usize) * (LCD_WIDTH as usize) + offset as usize)
                        ..2 * (i as usize + 1) * (LCD_WIDTH as usize)],
                ))
                .unwrap();
            self.set_windows(LCD_WIDTH - offset, i, LCD_WIDTH, i + 1);
            self.0
                .send_data(DataFormat::U8(
                    &image[2 * ((i as usize) * (LCD_WIDTH as usize))
                        ..2 * ((i as usize) * (LCD_WIDTH as usize) + offset as usize)],
                ))
                .unwrap();
        }
    }

    fn noise_rectangle(
        &mut self,
        x_start: u8,
        y_start: u8,
        x_end: u8,
        y_end: u8,
        rand: &mut Random,
    ) {
        self.set_windows(x_start, y_start, x_end, y_end);
        let size = (x_end - x_start) as u16 * (y_end - y_start) as u16;
        let iface = &mut self.0;
        iface
            .send_data(DataFormat::U16BEIter(
                &mut (0..size).into_iter().map(|_| rand.get_u16()),
            ))
            .unwrap();
    }
}

type LcdColor = Rgb565;
struct LoadedImage(&'static [u8]);

const HAL9000: LoadedImage = LoadedImage(include_bytes!("../assets/HAL9000.b"));
const NORDEA_PULSE: LoadedImage = LoadedImage(include_bytes!("../assets/Nordea-pulse-white.b"));
const IMG1: LoadedImage = LoadedImage(include_bytes!("../assets/pie-chart.b"));
const IMG2: LoadedImage = LoadedImage(include_bytes!("../assets/robots8.b"));
const IMG3: LoadedImage = LoadedImage(include_bytes!("../assets/sphere9.b"));
const IMG4: LoadedImage = LoadedImage(include_bytes!("../assets/sphere15.b"));
const IMG5: LoadedImage = LoadedImage(include_bytes!("../assets/spherebot3.b"));
const IMG6: LoadedImage = LoadedImage(include_bytes!("../assets/spherebot4.b"));
const IMG7: LoadedImage = LoadedImage(include_bytes!("../assets/robot1.b"));

struct ImageBuffer8k {
    w: u8,
    h: u8,
    buffer: [u8; 8192],
}
impl ImageBuffer8k {
    fn new(w: u8, h: u8) -> Self {
        let h = if (w as i16) * (h as i16) >= 8192 {
            (8192 / (w as i16)) as u8
        } else {
            h
        };
        ImageBuffer8k {
            w: w,
            h: h,
            buffer: [0u8; 8192],
        }
    }
    fn swap_xy(&mut self) -> &mut Self {
        let w = self.w;
        self.w = self.h;
        self.h = w;
        self
    }
}

struct ImageBuffer512 {
    w: u8,
    h: u8,
    buffer: [u8; 512],
}
impl ImageBuffer512 {
    fn new(w: u8, h: u8) -> Self {
        let h = if (w as i16) * (h as i16) >= 256 {
            255 / w
        } else {
            h
        };
        ImageBuffer512 {
            w: w,
            h: h,
            buffer: [0u8; 512],
        }
    }
    fn swap_xy(&mut self) -> &mut Self {
        let w = self.w;
        self.w = self.h;
        self.h = w;
        self
    }
    fn mirror_gradient(&self) -> Self {
        let mut g = Self::new(self.w, self.h);
        let count = (self.w as usize) * (self.h as usize);
        for i in 0..count {
            g.buffer[2 * (count - i)] = self.buffer[2 * i];
            g.buffer[2 * (count - i) + 1] = self.buffer[2 * i + 1];
        }
        g
    }
}

trait MyImage {
    fn width(&self) -> u8;
    fn height(&self) -> u8;
    fn buffer(&self) -> &[u8];
    fn buffer_mut(&mut self) -> &mut [u8];
    fn get_pixel_buff_mut(&mut self, x: u8, y: u8) -> &mut [u8] {
        let offset = 2 * ((x as usize) + (y as usize) * (self.width() as usize));
        &mut self.buffer_mut()[offset..]
    }
    fn get_pixel_buff(&self, x: u8, y: u8) -> &[u8] {
        let offset = 2 * ((x as usize) + (y as usize) * (self.width() as usize));
        &self.buffer()[offset..]
    }
    fn get_pixel_b(&self, x: u8, y: u8) -> [u8; 2] {
        let offset = 2 * ((x as usize) + (y as usize) * (self.width() as usize));
        let a = self.buffer()[offset];
        let b = self.buffer()[offset + 1];
        //(a as u16) + (b as u16)*256
        [a, b]
    }
    fn set_pixel_b(&mut self, x: u8, y: u8, c: &[u8]) {
        let offset = 2 * ((x as usize) + (y as usize) * (self.width() as usize));
        let b = self.buffer_mut();
        b[offset] = c[0];
        b[offset + 1] = c[1];
    }
    fn get_pixel_u16(&self, x: u8, y: u8) -> u16 {
        let offset = 2 * ((x as usize) + (y as usize) * (self.width() as usize));
        let a = self.buffer()[offset];
        let b = self.buffer()[offset + 1];
        (a as u16) + (b as u16) * 256
    }
    fn gradient(&self, x0: u8, y0: u8, x1: u8, y1: u8, count: u8) -> ImageBuffer512 {
        let mut img = ImageBuffer512::new(count, 1);
        let x0s = x0 as i16;
        let y0s = y0 as i16;
        let x1s = x1 as i16;
        let y1s = y1 as i16;
        let dx = x1s - x0s;
        let dy = y1s - y0s;
        for i in 0..count {
            let x = x0s + (dx * (i as i16)) / (count as i16);
            let y = y0s + (dy * (i as i16)) / (count as i16);
            img.set_pixel_b(i, 0, self.get_pixel_buff(x as u8, y as u8))
        }
        img
    }
}

impl MyImage for ImageBuffer8k {
    fn width(&self) -> u8 {
        self.w
    }
    fn height(&self) -> u8 {
        self.h
    }
    fn buffer(&self) -> &[u8] {
        &self.buffer[..(self.w as usize) * (self.h as usize) * 2]
    }
    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..(self.w as usize) * (self.h as usize) * 2]
    }
}

impl MyImage for ImageBuffer512 {
    fn width(&self) -> u8 {
        self.w
    }
    fn height(&self) -> u8 {
        self.h
    }
    fn buffer(&self) -> &[u8] {
        &self.buffer[..(self.w as usize) * (self.h as usize) * 2]
    }
    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..(self.w as usize) * (self.h as usize) * 2]
    }
}

impl MyImage for LoadedImage {
    fn width(&self) -> u8 {
        self.0[0]
    }
    fn height(&self) -> u8 {
        self.0[1]
    }
    fn buffer(&self) -> &[u8] {
        &self.0[2..]
    }
    fn buffer_mut(&mut self) -> &mut [u8] {
        panic!("No mutation for loaded image");
    }
}

impl<T: WriteOnlyDataCommand> OriginDimensions for Lcd<T> {
    fn size(&self) -> Size {
        Size::new(LCD_WIDTH as u32, LCD_HEIGHT as u32)
    }
}

/**	Sets the start position and size of the display area **/
fn set_windows(
    iface: &mut impl WriteOnlyDataCommand,
    x_start: u8,
    y_start: u8,
    x_end: u8,
    y_end: u8,
) {
    //set the X coordinates
    iface.send_commands(DataFormat::U8(&[0x2A])).unwrap();
    iface
        .send_data(DataFormat::U8(&[0x00, x_start, 0x00, x_end - 1]))
        .unwrap();

    //set the Y coordinates
    iface.send_commands(DataFormat::U8(&[0x2B])).unwrap();
    iface
        .send_data(DataFormat::U8(&[0x00, y_start, 0x00, y_end - 1]))
        .unwrap();

    iface.send_commands(DataFormat::U8(&[0x2C])).unwrap();
}

impl<T: WriteOnlyDataCommand> DrawTarget for Lcd<T> {
    type Color = LcdColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(Point { x, y }, color) in pixels.into_iter() {
            let x = x as u8;
            let y = y as u8;
            if x < 239 && y < 239 {
                set_windows(&mut self.0, x, y, x + 1, y + 1);
                self.0
                    .send_data(DataFormat::U16BE(&mut [RawU16::from(color).into_inner()]))
                    .unwrap();
            }
        }

        Ok(())
    }
}
fn draw1<T: WriteOnlyDataCommand>(lcd: &mut Lcd<T>, delay: &mut cortex_m::delay::Delay) {
    let style = PrimitiveStyleBuilder::new()
        .stroke_color(LcdColor::WHITE)
        .stroke_width(1)
        .fill_color(LcdColor::BLACK)
        .build();

    lcd.bounding_box().into_styled(style).draw(lcd).unwrap();

    let style = PrimitiveStyleBuilder::new()
        .stroke_color(LcdColor::GREEN)
        .stroke_width(5)
        .fill_color(LcdColor::BLUE)
        .build();

    Triangle::new(
        Point::new(20, 20),
        Point::new(220, 20),
        Point::new(120, 220),
    )
    .into_styled(style)
    .draw(lcd)
    .unwrap();

    let style = PrimitiveStyleBuilder::new()
        .stroke_color(LcdColor::RED)
        .stroke_width(5)
        .fill_color(LcdColor::GREEN)
        .build();

    Circle::new(Point::new(88, 30), 27)
        .into_styled(style)
        .draw(lcd)
        .unwrap();

    lcd.full_image(&HAL9000);
    delay.delay_ms(100);
    lcd.full_image(&NORDEA_PULSE);
    delay.delay_ms(100);
    lcd.clear(LcdColor::BLACK);
}

#[rp2040_hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Configure GPIO25 as an output
    let mut led_pin = pins.gpio25.into_push_pull_output();
    // These are implicitly used by the spi driver if they are in the correct mode
    let _spi_sclk = pins.gpio10.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio11.into_mode::<hal::gpio::FunctionSpi>();
    //let _spi_miso = pins.gpio12.into_mode::<hal::gpio::FunctionSpi>();
    let spi = hal::Spi::<_, _, 8>::new(pac.SPI1);

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        288.MHz(),
        &embedded_hal::spi::MODE_0,
    );
    let dc = pins.gpio8.into_push_pull_output();
    let mut cs = pins.gpio9.into_push_pull_output();
    let mut rst = pins.gpio12.into_push_pull_output();
    /* Reset */
    rst.set_high().unwrap();
    delay.delay_ms(100);
    rst.set_low().unwrap();
    rst.set_high().unwrap();
    cs.set_low().unwrap();
    delay.delay_ms(100);

    let mut iface = SPIInterface::new(spi, dc, cs);

    led_pin.set_high().unwrap();
    /*
    for i in 0..0 {
        set_windows(&mut iface, 0, 0, 240, 240);
        iface
            .send_data(DataFormat::U16BEIter(&mut (0..240 * 240).into_iter()))
            .unwrap();
    }
    */

    let mut lcd = Lcd(iface);
    lcd.init(&mut delay);

    let mut rand = Random::new();

    //    draw1(&mut lcd, &mut delay);

    //    for i in 0..120 {
    //        lcd.noise_rectangle(120 - i, 120 - i, 120 + i, 120 + i, &mut rand);
    //    }
    //lcd.full_image(NORDEA_PULSE);

    /*
    for i in 0..120 {
        lcd.full_image_horizontal_shift(NORDEA_PULSE, i*2);
        let style = MonoTextStyle::new(&FONT_6X10, LcdColor::BLUE);
        // Create a text at position (20, 30) and draw it using the previously defined style
        }
    */
    /*
    for i in 0..30 {
        lcd.full_image_horizontal_shift(NORDEA_PULSE, 240-(i*8));
    }
    let mut gradient = HAL9000.gradient(120,120, 120, 20, 80);
    gradient.swap_xy();
    let mut g2 = gradient.mirror_gradient();
    lcd.full_image(HAL9000);


    let mut d=8;
    for k in 0..100{
        d=8;
        for i in 0..240{
            d += rand.get_u8()/64-2;
            lcd.show_image_clamped(i, d, &g2, 80-d);
            lcd.show_image_clamped(i, 120-d, &gradient, 80-d);
        }
    }
    lcd.full_image(HAL9000);
    */
    
    loop {
        led_pin.set_high().unwrap();
        
        lcd.clear(LcdColor::WHITE);
        lcd.full_image_interlaced(&NORDEA_PULSE);
        for i in 0..60 {
            lcd.full_image_horizontal_shift(NORDEA_PULSE, 240 - (i * 4));
        }
        lcd.full_image_logic(&IMG2);
        delay.delay_ms(2000);
        //        lcd.full_image_noisy(&HAL9000, &mut rand);
        for i in 0..120 {
            lcd.noise_rectangle(120 - i, 120 - i, 120 + i, 120 + i, &mut rand);
        }
//        lcd.full_image(&IMG1);
        delay.delay_ms(2000);
        lcd.full_image_logictri(&HAL9000);
        //        lcd.full_image_noisy1(&HAL9000, &mut rand);
        led_pin.set_low().unwrap();
        delay.delay_ms(100);
        led_pin.set_high().unwrap();
        delay.delay_ms(100);
        led_pin.set_low().unwrap();
        delay.delay_ms(100);
        led_pin.set_high().unwrap();

        delay.delay_ms(1000);

        //        lcd.full_image_noisy20(&IMG2, &mut rand);
        lcd.full_image_wave(&IMG3);
        lcd.full_image(&IMG3);
        delay.delay_ms(1000);
        lcd.full_image_noisy1(&IMG5, &mut rand);
        lcd.full_image(&IMG5);
        delay.delay_ms(100);

        lcd.full_image_wave(&IMG6);
        lcd.full_image(&IMG6);
        delay.delay_ms(100);

        lcd.full_image_noisy20(&IMG7, &mut rand);
        lcd.full_image(&IMG7);
        delay.delay_ms(3000);

        lcd.full_image_rot(&IMG4);
        delay.delay_ms(3000);

        /*
        lcd.full_image(&IMG3);
        delay.delay_ms(3000);
        lcd.full_image(&IMG4);
        delay.delay_ms(3000);
        lcd.full_image(&IMG5);
        delay.delay_ms(3000);
        lcd.full_image(&IMG6);
        delay.delay_ms(3000);
        lcd.full_image(&IMG7);
        delay.delay_ms(3000);
        */
    }
}

// End of file
