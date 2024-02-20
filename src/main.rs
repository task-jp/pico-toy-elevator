//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use bsp::entry;
use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    uart::{DataBits, StopBits, UartConfig},
    watchdog::Watchdog,
};
use embedded_alloc::Heap;
use embedded_graphics::{
    mono_font::{ascii::FONT_5X8, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use embedded_hal::digital::v2::PinState;
use fugit::RateExtU32;
use rp_pico as bsp;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

const HEAP_SIZE: usize = 200 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

mod button;
mod elevator;

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let (mut pac, _core) = unsafe { (pac::Peripherals::steal(), pac::CorePeripherals::steal()) };
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let scl = pins.gpio17.into_function::<bsp::hal::gpio::FunctionI2C>();
    let sda = pins.gpio16.into_function::<bsp::hal::gpio::FunctionI2C>();
    let i2c = bsp::hal::I2C::i2c0(
        pac.I2C0,
        sda,
        scl,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_5X8)
        .text_color(BinaryColor::On)
        .build();

    display.clear(BinaryColor::Off).unwrap();
    let mut s = format!("{:?}", info);
    let mut x = 0;
    let mut y = 0;
    let width = 128 / 6 - 1;
    while !s.is_empty() {
        let end_of_line = s
            .find(|c| {
                if c == '\n' || x > width {
                    x = 0;
                    y += 1;
                    true
                } else {
                    x += 1;
                    false
                }
            })
            .unwrap_or(s.len());
        let (line, rest) = s.split_at(end_of_line);
        let sz = text_style.font.character_size;
        Text::new(
            line,
            Point::new(x * sz.width as i32, y * sz.height as i32),
            text_style,
        )
        .draw(&mut display)
        .unwrap();
        s = rest.strip_prefix('\n').unwrap_or(rest).to_string();
    }
    display.flush().unwrap();

    loop {}
}

#[entry]
fn main() -> ! {
    unsafe {
        ALLOCATOR.init(
            &mut HEAP as *const u8 as usize,
            core::mem::size_of_val(&HEAP),
        )
    }
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
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

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // ボタンの管理
    let mut buttons: heapless::Vec<Box<dyn button::LedButtonTrait>, 2> = heapless::Vec::new();

    // LedButton インスタンスを作成して Vec に追加するマクロ
    macro_rules! push_led_button {
        ($led:expr, $button:expr) => {
            let _ = buttons.push(Box::new(button::LedButton::new(
                $led.into_push_pull_output_in_state(PinState::High),
                $button.into_pull_up_input(),
            )));
        };
    }
    push_led_button!(pins.gpio4, pins.gpio2); // A
    push_led_button!(pins.gpio5, pins.gpio3); // B

    // ディスプレイ
    // https://docs.rs/crate/rp-pico/latest/source/examples/pico_i2c_oled_display_ssd1306.rs
    let scl = pins.gpio17.into_function::<bsp::hal::gpio::FunctionI2C>();
    let sda = pins.gpio16.into_function::<bsp::hal::gpio::FunctionI2C>();
    let i2c = bsp::hal::I2C::i2c0(
        pac.I2C0,
        sda,
        scl,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    // Empty the display:
    display.clear(BinaryColor::Off).unwrap();
    display.flush().unwrap();

    // ATP3012xx の初期化
    let uart_pins = (pins.gpio0.into_function(), pins.gpio1.into_function());
    let uart = bsp::hal::uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    macro_rules! led_button_new {
        ($led:expr, $button:expr) => {
            Box::new(button::LedButton::new(
                $led.into_push_pull_output_in_state(PinState::High),
                $button.into_pull_up_input(),
            ))
        };
    }

    let mut elevator = elevator::Elevator::new([
        (-2, "B2", led_button_new!(pins.gpio22, pins.gpio27)),
        (-1, "B1", led_button_new!(pins.gpio26, pins.gpio28)),
        (1, "1", led_button_new!(pins.gpio19, pins.gpio21)),
        (2, "2", led_button_new!(pins.gpio18, pins.gpio20)),
        (3, "3", led_button_new!(pins.gpio10, pins.gpio8)),
        (4, "4", led_button_new!(pins.gpio11, pins.gpio9)),
        (5, "5", led_button_new!(pins.gpio13, pins.gpio14)),
        (6, "6", led_button_new!(pins.gpio12, pins.gpio15)),
    ]);

    elevator.on_announce(move |message: &[u8]| {
        uart.write_full_blocking(message);
    });

    delay.delay_ms(100);
    loop {
        let mut i = 0;
        display.clear(BinaryColor::Off).unwrap();
        for button in buttons.iter_mut() {
            if button.is_pressed().unwrap() {
                match i {
                    0 => {
                        if elevator.set_door_open(false) {
                            button.turn_on().unwrap();
                        } else {
                            button.turn_off().unwrap();
                        }
                    }
                    1 => {
                        if elevator.set_door_open(true) {
                            button.turn_on().unwrap();
                        } else {
                            button.turn_off().unwrap();
                        }
                    }
                    _ => {}
                }
            } else {
                button.turn_off().unwrap();
            }
            i += 1;
        }
        elevator.advance();
        elevator.draw(&mut display).unwrap();
        display.flush().unwrap();
        delay.delay_ms(100);
    }
}
