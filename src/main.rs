// use std::{thread, time::Duration}

use gpio_cdev::LineRequestFlags;

use std::net::{TcpListener, TcpStream};

use anyhow::{Context, Result};

fn get_offset(bank: u32, port: char, pin: u32) -> u32 {
    let port_val = port as u32 - 'a' as u32;
    bank * 32 + port_val * 8 + pin
}

fn main() -> Result<()> {
    let chip_path = "/dev/gpiochip1";

    let mut chip = gpio_cdev::Chip::new(chip_path)
        .context(format!("failed to create the chip {chip_path}"))?;

    let line_offset = get_offset(1, 'c', 0);
    println!("line offset: {line_offset}");
    let line_offset = line_offset - 32;

    let outputs = chip.get_all_lines().context("failed to get all lines")?;

    let total_lines = outputs.len();
    println!("total lines: {total_lines}");

    let output = chip
        .get_line(line_offset)
        .context(format!("failed to create the output {line_offset}"))?;

    let output_handle = output
        .request(LineRequestFlags::OUTPUT, 0, "act_led_blink")
        .context("failed to create the output handler: {error}")?;

    let value_on = 0;
    let value_off = 1;

    // loop {
    //     output_handle.set_value(value_on).expect("value_on error");
    //     std::thread::sleep(std::time::Duration::from_millis(100));
    //     output_handle.set_value(value_off).expect("value_off error");
    //     std::thread::sleep(std::time::Duration::from_millis(100));
    // }
    let listener = TcpListener::bind("0.0.0.0:5000");
    Ok(())
}
