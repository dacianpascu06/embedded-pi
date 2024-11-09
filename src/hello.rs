//! Print a "Hello, World!" message to the debugger and blink the LED on GPIO1.

#![no_std]
#![no_main]
// Delete the following line after you're done implementing
// the solution.
#![allow(unused)]

use cortex_m::asm;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_rp::{
    gpio::{Level, Output},
    usb::{Driver, InterruptHandler},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// TODO 2.1 : Write a task that blinks the LED connected to GPIO1.
#[embassy_executor::task]
async fn task_led(mut led: Output<'static>) {
    loop {
        led.set_high();
        Timer::after_millis(500).await;
        led.set_low();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // TODO 0 : Set the timer to (a)wait 5 seconds before printing
    //          the "Hello, World!" message.

    Timer::after_secs(2).await;
    info!("Hello World");

    // TODO 1 : Print the "Hello, World!" message to the debugger.

    println!("hello from println");

    let mut led = Output::new(p.PIN_5, Level::Low);
    led.set_low();
    Timer::after_secs(1).await;

    spawner.spawn(task_led(led)).expect("Could not spawn task");

    //loop {
    //    yield_now().await;
    //}

    // TODO 2.2 : Spawn the task that blinks the LED connected to GPIO1.
}
