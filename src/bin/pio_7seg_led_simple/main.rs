//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use pio_proc::pio_file;
// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico::{
    self as bsp,
    hal::{
        self,
        gpio::{FunctionPio0, Pin},
        pio::{PIOBuilder, PIOExt},
    },
};
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

#[entry]
fn main() -> ! {
    info!("Program start");
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

    let out_pin: Pin<_, FunctionPio0, _> = pins.gpio2.into_function();
    let _clock_pin = pins.gpio3.into_function::<hal::gpio::FunctionPio0>();
    let _ratch_pin = pins.gpio4.into_function::<hal::gpio::FunctionPio0>();

    let out_pin_id = out_pin.id().num;

    // Create a pio program
    let program = pio_file!("pio/shift_register.pio", select_program("shift_register"),);

    let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio0.install(&program.program).unwrap();

    let (mut sm, _, mut tx) = PIOBuilder::from_installed_program(installed)
        .out_pins(out_pin_id, 1)
        .side_set_pin_base(out_pin_id + 1)
        .build(sm0);

    #[rustfmt::skip]
    sm.set_pindirs([
        (out_pin_id,     hal::pio::PinDir::Output),
        (out_pin_id + 1, hal::pio::PinDir::Output),
        (out_pin_id + 2, hal::pio::PinDir::Output),
    ]);

    sm.start();

    let mut shift: u8 = 0;

    loop {
        info!("on! (shift {})", shift);

        tx.write(1 << shift);

        delay.delay_ms(300);

        // tx.write(u32::MAX);
        // info!("off!");

        // delay.delay_ms(10);

        if shift >= 6 {
            shift = 0;
        } else {
            shift += 1
        }
    }
}

// End of file
