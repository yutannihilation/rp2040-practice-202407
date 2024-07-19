//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use rtic_monotonics::rp2040::prelude::*;
rp2040_timer_monotonic!(Mono);

#[rtic::app(device = rp_pico::hal::pac, peripherals = true)]
mod app {

    use super::*;

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
            pio::{PIOBuilder, PIOExt, Tx},
        },
    };
    // use sparkfun_pro_micro_rp2040 as bsp;

    use bsp::hal::{clocks::init_clocks_and_plls, sio::Sio, watchdog::Watchdog};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        // tx ix is used in only one task, so this can be Local
        tx: Tx<rp_pico::hal::pio::PIO0SM0>,
    }

    #[init]
    fn init(c: init::Context) -> (Shared, Local) {
        let mut pac = c.device;

        Mono::start(pac.TIMER, &pac.RESETS);

        let mut watchdog = Watchdog::new(pac.WATCHDOG);
        let sio = Sio::new(pac.SIO);

        // While this doesn't use the `clock` object, it seems this code is
        // needed to initialize the clock.
        let external_xtal_freq_hz = 12_000_000u32;
        let _clocks = init_clocks_and_plls(
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

        let out_pin: Pin<_, FunctionPio0, _> = pins.gpio2.into_function();
        let _clock_pin = pins.gpio3.into_function::<hal::gpio::FunctionPio0>();
        let _ratch_pin = pins.gpio4.into_function::<hal::gpio::FunctionPio0>();

        let out_pin_id = out_pin.id().num;

        // Create a pio program
        let program = pio_file!("pio/shift_register.pio", select_program("shift_register"),);

        let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
        let installed = pio0.install(&program.program).unwrap();

        let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
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

        update_data::spawn().ok();

        (Shared {}, Local { tx })
    }

    #[task(
        shared = [],
        local = [tx, shift: u8 = 0]
    )]
    async fn update_data(c: update_data::Context) {
        loop {
            c.local.tx.write(1 << *c.local.shift);

            if *c.local.shift >= 6 {
                *c.local.shift = 0;
            } else {
                *c.local.shift += 1
            }

            Mono::delay(100.millis()).await;
        }
    }
}

// End of file
