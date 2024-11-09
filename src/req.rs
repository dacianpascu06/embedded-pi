//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to Wifi network and makes a web request to get the current time.

#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

use core::fmt::Write;
use core::str::from_utf8;
use core::write;
use embedded_nov_2024::bmp280::*;
use heapless::String;
use heapless::*;

use cyw43::JoinOptions;
use cyw43_pio::PioSpi;
use defmt::*;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::dns::*;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::Duration;
use embassy_time::Timer;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::iso_8859_1::FONT_7X13_BOLD;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::{Rgb565, RgbColor};
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_nov_2024::display::SPIDeviceInterface;
use rand_core::RngCore;
use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::request::Method;
use {defmt_rtt as _, panic_probe as _};
const DISPLAY_FREQ: u32 = 64_000_000;

use serde::Deserialize;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _, serde_json_core};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = "Wyliodrin"; // change to your network SSID
const WIFI_PASSWORD: &str = "g3E2PjWy"; // change to your network password

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    // let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    // let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner
        .spawn(cyw43_task(runner))
        .expect("failed to spawn network driver");

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner
        .spawn(net_task(runner))
        .expect("failed to start new network stack");

    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    info!("waiting for link up...");
    while !stack.is_link_up() {
        Timer::after_millis(500).await;
    }
    info!("Link is up!");

    info!("waiting for stack to be up...");
    stack.wait_config_up().await;
    info!("Stack is up!");

    // And now we can use it!

    loop {
        let mut rx_buffer = [0; 8192];

        let client_state = TcpClientState::<1, 1024, 1024>::new();
        let tcp_client = TcpClient::new(stack, &client_state);
        let dns_client = DnsSocket::new(stack);

        //let mut http_client = HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);
        //let url = "https://worldtimeapi.org/api/timezone/Europe/Berlin";
        // for non-TLS requests, use this instead:
        let mut http_client = HttpClient::new(&tcp_client, &dns_client);
        let url = "http://192.168.1.199:5000/time";

        info!("connecting to {}", &url);

        let mut request = match http_client.request(Method::GET, &url).await {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to make HTTP request: {:?}", e);
                return; // handle the error
            }
        };

        let response = match request.send(&mut rx_buffer).await {
            Ok(resp) => resp,
            Err(_e) => {
                error!("Failed to send HTTP request");
                return; // handle the error;
            }
        };

        let body = match from_utf8(response.body().read_to_end().await.unwrap()) {
            Ok(b) => b,
            Err(_e) => {
                error!("Failed to read response body");
                return; // handle the error
            }
        };
        info!("Response body: {:?}", &body);

        // parse the response body and update the RTC
        #[derive(Deserialize, Debug, Format)]
        struct ApiResponse {
            date: Date,
            time: Time,
        }

        #[derive(Deserialize, Debug, Format)]
        struct Date {
            day: u32,
            month: u32,
            year: u32,
        }

        #[derive(Deserialize, Debug, Format)]
        struct Time {
            hour: u32,
            minute: u32,
            second: u32,
        }

        let bytes = body.as_bytes();

        info!("Initializing display...");
        // ************** Display initialization - DO NOT MODIFY! *****************
        let miso = p.PIN_4;
        let display_cs = p.PIN_17;
        let mosi = p.PIN_19;
        let clk = p.PIN_18;
        let rst = p.PIN_0;
        let dc = p.PIN_16;
        let mut display_config = embassy_rp::spi::Config::default();
        display_config.frequency = DISPLAY_FREQ;
        display_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
        display_config.polarity = embassy_rp::spi::Polarity::IdleHigh;

        // Init SPI
        let spi: embassy_rp::spi::Spi<'_, _, embassy_rp::spi::Blocking> =
            embassy_rp::spi::Spi::new_blocking(p.SPI0, clk, mosi, miso, display_config.clone());
        let spi_bus: embassy_sync::blocking_mutex::Mutex<
            embassy_sync::blocking_mutex::raw::NoopRawMutex,
            _,
        > = embassy_sync::blocking_mutex::Mutex::new(core::cell::RefCell::new(spi));

        let display_spi = embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig::new(
            &spi_bus,
            embassy_rp::gpio::Output::new(display_cs, embassy_rp::gpio::Level::High),
            display_config,
        );

        let dc = embassy_rp::gpio::Output::new(dc, embassy_rp::gpio::Level::Low);
        let rst = embassy_rp::gpio::Output::new(rst, embassy_rp::gpio::Level::Low);
        let di = SPIDeviceInterface::new(display_spi, dc);

        // Init ST7789 LCD
        let mut display = st7789::ST7789::new(di, rst, 240, 240);
        display.init(&mut embassy_time::Delay).unwrap();
        display
            .set_orientation(st7789::Orientation::Portrait)
            .unwrap();
        display.clear(<embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::BLACK).unwrap();
        // ************************************************************************

        info!("Display initialization finished!");

        // Write welcome message
        let style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::CYAN);
        Text::new("hello!", Point::new(36, 190), style)
            .draw(&mut display)
            .unwrap();
        // Wait a bit
        Timer::after_secs(1).await;

        loop {
            match serde_json_core::de::from_slice::<ApiResponse>(bytes) {
                Ok((output, _used)) => {
                    let mut string: String<300> = String::new();
                    write!(string, "{:?}", output).unwrap();
                    Text::new(&string, Point::new(36, 50), style)
                        .draw(&mut display)
                        .unwrap();
                }
                Err(_e) => {
                    error!("Failed to parse response body");
                    return; // handle the error
                }
            }
            Timer::after(Duration::from_secs(5)).await;
        }
    }
}
