use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::{Context, Result};
use log::{error, info};

use crate::handlers::Handler;

mod gpio;
mod handlers;

const LISTEN_IP: &str = "0.0.0.0";
const LISTEN_PORT: u32 = 5000;

const RESPONSE_EOT: u8 = 0x04;
const RESPONSE_ACK: u8 = 0x06;
const RESPONSE_NAK: u8 = 0x15;

fn handle_byte<T: handlers::Handler>(mut stream: TcpStream, handler: &mut T) {
    let mut buf = [0u8; 1];
    loop {
        if stream.read_exact(&mut buf).is_err() || buf[0] == RESPONSE_EOT {
            info!("Disconnected");
            break;
        }

        let value = buf[0];
        info!("Received: {value}");

        match handler.handle(value) {
            Ok(()) => {
                stream.write_all(&[RESPONSE_ACK]).unwrap();
            }
            Err(e) => {
                error!("{e}");
                stream.write_all(&[RESPONSE_NAK]).unwrap();
            }
        }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stdout)
        .init();

    let address = format!("{LISTEN_IP}:{LISTEN_PORT}");
    let listener = TcpListener::bind(address.clone())
        .context(format!("Failed to bind listener to the address {address}"))?;

    let mut gpio_storage = gpio::GpioStorage::new();
    let mut led_handler = handlers::Led::new(&mut gpio_storage, "GPIO1_C0")?;
    led_handler
        .init_default()
        .context("Failed to init default state for handler")?;

    loop {
        info!("Listening on {address}");

        let (stream, address) = listener.accept()?;
        stream.set_nodelay(true).ok();

        info!("Connected: {address}");
        handle_byte(stream, &mut led_handler);
    }
}
