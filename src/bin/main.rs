#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::Level;
use esp_hal::main;
use esp_hal::rmt::{PulseCode, Rmt, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// WS2812 bit timings in RMT ticks at 80 MHz (12.5 ns per tick).
const T0H: u16 = 32; // 0.40 µs
const T0L: u16 = 68; // 0.85 µs
const T1H: u16 = 64; // 0.80 µs
const T1L: u16 = 36; // 0.45 µs

/// Encodes one GRB color as 24 WS2812 bits plus an RMT end marker.
fn encode(r: u8, g: u8, b: u8) -> [PulseCode; 25] {
    let grb = (u32::from(g) << 16) | (u32::from(r) << 8) | u32::from(b);
    let mut codes = [PulseCode::end_marker(); 25];
    for (i, code) in codes.iter_mut().take(24).enumerate() {
        *code = if grb & (1 << (23 - i)) != 0 {
            PulseCode::new(Level::High, T1H, Level::Low, T1L)
        } else {
            PulseCode::new(Level::High, T0H, Level::Low, T0L)
        };
    }
    codes
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 2.0.0
    // generator parameters: -o esp32s3

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The onboard RGB LED on most ESP32-S3-N16R8 boards is a WS2812 on GPIO48.
    // Some boards (e.g. DevKitC-1 v1.1) use GPIO38 instead.
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let mut channel = rmt
        .channel0
        .configure_tx(&TxChannelConfig::default().with_clk_divider(1))
        .unwrap()
        .with_pin(peripherals.GPIO48);

    let on = encode(0, 0, 32); // dim blue
    let off = encode(0, 0, 0);
    let delay = Delay::new();

    loop {
        for frame in [&on, &off] {
            channel = match channel.transmit(frame) {
                Ok(tx) => tx.wait().unwrap_or_else(|(_, ch)| ch),
                Err((_, ch)) => ch,
            };
            delay.delay_millis(500);
        }
    }
}
