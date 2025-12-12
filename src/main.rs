use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

mod gpio;

#[derive(Clone, Copy, Debug)]
enum LedMode {
    Off,
    On,
    Blink { ms: i32 },
}

#[derive(Clone, Copy, Debug)]
struct LedState {
    mode: LedMode,
}

fn led_worker(state: Arc<Mutex<LedState>>) {
    let mut gpio_storage = gpio::GpioStorage::new();

    let output_handle = gpio_storage.get_or_create("GPIO1_C0").unwrap();
    let value_on = 1;
    let value_off = 0;

    loop {
        let mode = { state.lock().unwrap().mode };

        match mode {
            LedMode::Off => {
                output_handle.set_value(value_off).expect("value_off error");
            }
            LedMode::On => {
                output_handle.set_value(value_on).expect("value_off error");
            }
            LedMode::Blink { ms } => {
                output_handle.set_value(value_on).expect("value_on error");
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                output_handle.set_value(value_off).expect("value_off error");
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            }
        }
    }
}

fn handle_client(stream: TcpStream, state: Arc<Mutex<LedState>>) {
    let peer = stream.peer_addr().ok();
    let mut writer = stream;
    let reader = writer.try_clone().unwrap();
    let mut reader = BufReader::new(reader);

    let _ = writer.write_all(b"Server is ready\n\n");

    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 {
            break; // EOF
        }

        let cmd = line.trim();
        let mut parts = cmd.split_whitespace();
        let keyword = parts.next().unwrap_or("").to_uppercase();

        let mut response = String::new();

        match keyword.as_str() {
            "ON" => {
                let mut s = state.lock().unwrap();
                s.mode = LedMode::On;
                response = "OK: LED ON\n".into();
            }
            "OFF" => {
                let mut s = state.lock().unwrap();
                s.mode = LedMode::Off;
                response = "OK: LED OFF\n".into();
            }
            "BLINK" => {
                if let Some(ms_str) = parts.next() {
                    if let Ok(ms) = ms_str.parse::<i32>() {
                        let mut s = state.lock().unwrap();
                        s.mode = LedMode::Blink { ms };
                        response = "OK: LED BLINK\n".into();
                    } else {
                        response = "ERROR: LED BLINK\n".into();
                    }
                } else {
                    response = "ERROR: LED BLINK\n".into();
                }
            }
            _ => {
                response = "ERROR: unknown command\n".into();
            }
        }

        if writer.write_all(response.as_bytes()).is_err() {
            break;
        }

        line.clear();
    }

    if let Some(p) = peer {
        eprintln!("Client disconnected: {p}");
    }
}

fn main() -> Result<()> {
    let state = Arc::new(Mutex::new(LedState {
        mode: LedMode::Blink { ms: 100 },
    }));

    {
        let state_clone = state.clone();
        std::thread::spawn(move || led_worker(state_clone));
    }

    let address = "0.0.0.0:5000";
    let listener = TcpListener::bind(address)
        .context(format!("failed to bind listener to the address {address}"))?;
    println!("Listening on {address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state_clone = state.clone();
                std::thread::spawn(move || handle_client(stream, state_clone));
                println!("Success");
            }
            Err(error) => {
                eprintln!("Error accepting connection: {error}");
            }
        }
    }

    Ok(())
}
