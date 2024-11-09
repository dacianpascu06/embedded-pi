//! Generate two colors and get the RGB encodings for them. These are the colors
//! Go to [Random Color Generator](https://randomwordgenerator.com/color.php)
//! you will need to display on the RGB LED.
//!
//! Your application should smoothly transition from one color to another. The colors will
//! be displayed sequentially for 3 seconds each, with a gradual transition period of 1 second.
//!
//! Keep in mind that the RGB LED is common anode.
//!
//! For displaying the color on the LED, PWM (Pulse Width Modulation) will need to be set up
//! on the pin. Connect them to pins: GPIO0 (Red), GPIO1 (Green), and
//! GPIO2 (Blue). (Hint: Pin 0 and 1 will share the same channel).

#![no_std]
#![no_main]
// Delete the following line after you're done implementing
// the solution.
#![allow(unused)]

use core::{default, u16};

use cortex_m::asm;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::Config;
use embassy_rp::peripherals::{PIN_0, PIN_1, PIN_2, PWM_SLICE0, PWM_SLICE1};
use embassy_rp::pwm::{Config as PwmConfig, Pwm, SetDutyCycle};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // TODO 0 : Create the Config for the PWM that will drive the RGB LED.

    let mut config = embassy_rp::pwm::Config::default();

    config.top = 255;

    config.invert_a = true;
    config.invert_b = true;

    config.compare_a = 0; // red
    config.compare_b = 255; // green

    // duty = compare / top

    let mut config2 = embassy_rp::pwm::Config::default();

    config2.invert_a = true;
    config2.compare_a = 0; // blue
    config2.top = 255;

    let mut pwm = Pwm::new_output_ab(p.PWM_SLICE0, p.PIN_0, p.PIN_1, config.clone());
    let mut pwm2 = Pwm::new_output_a(p.PWM_SLICE1, p.PIN_2, config2.clone());

    Timer::after_secs(2).await;
    loop {
        Timer::after_millis(200).await;
        config.compare_a += 1;
        config.compare_b -= 1;
        pwm.set_config(&config);
        asm::nop();
    }

    // TODO 1 : Modify the RGB values and loop through the configs to create a transition.
}
