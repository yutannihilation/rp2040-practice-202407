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

    use defmt::info;
    use defmt_rtt as _;
    use panic_probe as _;

    use pio_proc::pio_file;
    use rp_pico::{
        self as bsp,
        hal::{
            self,
            gpio::{FunctionPio0, Pin},
            pio::{PIOBuilder, PIOExt, Tx},
        },
    };

    use bsp::hal::{clocks::init_clocks_and_plls, sio::Sio, watchdog::Watchdog};

    #[derive(Debug, Clone, Copy)]
    struct PwmStep {
        length: u32,
        data: u32,
    }

    pub struct PwmData {
        pwm_levels: [u32; 8],
        pwm_steps: [PwmStep; 9],
    }

    impl PwmData {
        fn new() -> Self {
            let null_step = PwmStep {
                length: 255,
                data: 0,
            };

            Self {
                pwm_levels: [0; 8],
                pwm_steps: [null_step; 9],
            }
        }

        fn reflect(&mut self) {
            let mut indices: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
            indices.sort_unstable_by_key(|&i| self.pwm_levels[i]);

            let mut data = 255;
            let mut prev_level = 0;
            let mut cur_level = 0;

            for (i, &cur_index) in indices.iter().enumerate() {
                cur_level = self.pwm_levels[cur_index];

                self.pwm_steps[i] = PwmStep {
                    length: cur_level - prev_level,
                    data,
                };

                data &= !(1 << cur_index);
                // info!("{:b}", data);

                prev_level = cur_level;
            }

            // period after all pins are set low
            self.pwm_steps[8] = PwmStep {
                length: 255 - cur_level,
                data: 0,
            };
        }
    }

    #[shared]
    struct Shared {
        data: PwmData,
    }

    #[local]
    struct Local {
        // tx ix is used in only one task, so this can be Local
        tx: Tx<rp_pico::hal::pio::PIO0SM0>,

        // current position
        phase: f32,

        cur_index: usize,
        next_index: usize,
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
        info!("State machine started");

        // prepare data
        let mut data = PwmData::new();
        data.pwm_levels = [0; 8];
        data.reflect();

        update_data::spawn().ok();
        info!("update_data thread started");

        repeat_pwm::spawn().ok();
        info!("repeat_pwm thread started");

        (
            Shared { data },
            Local {
                tx,
                phase: 0.0,
                cur_index: 0,
                next_index: 0,
            },
        )
    }

    #[task(
        shared = [data],
        local = [phase, cur_index, next_index],
    )]
    async fn update_data(c: update_data::Context) {
        let mut data = c.shared.data;
        let mut phase = *c.local.phase;
        let mut cur_index = *c.local.cur_index;
        let mut next_index = *c.local.next_index;

        const IDX: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 4]; // 0 = nowhere

        loop {
            phase += 0.03;

            data.lock(|data: &mut PwmData| {
                if phase >= 1.0 {
                    // reset the current segment
                    data.pwm_levels[cur_index] = 0;

                    cur_index = next_index;
                    next_index = (next_index + 1) % 8;

                    phase -= 1.0;
                }

                data.pwm_levels[IDX[cur_index]] = (255. * (1.0 - phase)) as u32;
                data.pwm_levels[IDX[next_index]] = (255. * (phase - 0.4) * 1.667) as u32;

                data.reflect();
            });

            Mono::delay(5.millis()).await;
        }
    }

    #[task(
        shared = [data],
        local = [tx, step: u8 = 0],
    )]
    async fn repeat_pwm(c: repeat_pwm::Context) {
        let mut data = c.shared.data;
        let tx = c.local.tx;

        loop {
            let steps = data.lock(|data| data.pwm_steps);
            for step in steps {
                tx.write(step.data);

                let delay_ms = ((step.length * 10) as u64).micros();
                Mono::delay(delay_ms).await;
            }
            *c.local.step = (*c.local.step + 1) % 7;
        }
    }
}

// End of file
