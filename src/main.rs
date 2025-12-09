// use std::{thread, time::Duration}

use gpio_cdev::LineRequestFlags;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

fn get_offset(bank: u32, port: char, pin: u32) -> u32 {
    let port_val = port as u32 - 'a' as u32;
    bank * 32 + port_val * 8 + pin
}

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
    let chip_path = "/dev/gpiochip1";

    let mut chip = gpio_cdev::Chip::new(chip_path)
        .context(format!("failed to create the chip {chip_path}"))
        .unwrap();

    let line_offset = get_offset(1, 'c', 0);
    println!("line offset: {line_offset}");
    let line_offset = line_offset - 32;

    let outputs = chip
        .get_all_lines()
        .context("failed to get all lines")
        .unwrap();

    let total_lines = outputs.len();
    println!("total lines: {total_lines}");

    let output = chip
        .get_line(line_offset)
        .context(format!("failed to create the output {line_offset}"))
        .unwrap();

    let output_handle = output
        .request(LineRequestFlags::OUTPUT, 0, "act_led_blink")
        .context("failed to create the output handler: {error}")
        .unwrap();

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
                response = "OK: LED BLINK\n".into();
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
