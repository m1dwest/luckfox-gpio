use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::{Context, Result};
use log::{error, info};

mod gpio;
mod handler;

const LISTEN_IP: &str = "0.0.0.0";
const LISTEN_PORT: u32 = 5000;

const RESPONSE_EOT: u8 = 0x04;
const RESPONSE_ACK: u8 = 0x06;
const RESPONSE_NAK: u8 = 0x15;

const LED_OFF: u8 = 0x30;
const LED_ON: u8 = 0x31;
const HANDSHAKE: u8 = 0x05;

fn handle_byte(mut stream: TcpStream, handler: &mut handler::Handler) {
    let mut buf = [0u8; 1];
    loop {
        if stream.read_exact(&mut buf).is_err() || buf[0] == RESPONSE_EOT {
            info!("Disconnected");
            break;
        }

        let value = buf[0];
        info!("Received: {value}");

        match handler.send(value) {
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
    let mut handler = handler::Handler::new(&mut gpio_storage);
    handler.enable_gpio("GPIO1_C0", LED_ON, true);
    handler.enable_gpio("GPIO1_C0", LED_OFF, false);
    handler.enable_gpio("GPIO1_C0", HANDSHAKE, false);

    loop {
        info!("Listening on {address}");

        let (stream, address) = listener.accept()?;
        stream.set_nodelay(true).ok();

        info!("Connected: {address}");
        handle_byte(stream, &mut handler);
    }
}
