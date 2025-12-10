use std::collections::HashMap;

use anyhow::{Context, Result};

const GPIO_CHIPS_N: usize = 4;

pub struct GpioId {
    gpio_chip_n: u32,
    line_offset: u32,
}

impl GpioId {
    fn calculate_abs_offset(bank: u32, port: char, pin: u32) -> u32 {
        let port_val = port.to_ascii_lowercase() as u32 - 'a' as u32;
        println!("bb {bank} {port_val} {pin}");
        bank * 32 + port_val * 8 + pin
    }

    pub fn from(id: &str) -> Option<Self> {
        let id = id.strip_prefix("GPIO")?;
        let (bank, rest) = id.split_once('_')?;
        let bank: u32 = bank.parse().ok()?;
        let mut rest_chars = rest.chars();
        let port = rest_chars.next()?;
        let pin: u32 = rest_chars.as_str().parse().ok()?;
        println!("{bank} {port} {pin}");

        let offset_abs = GpioId::calculate_abs_offset(bank, port, pin);
        let gpio_chip_n = offset_abs / 32;
        let offset_rel = offset_abs - gpio_chip_n * 32;
        println!("{offset_abs} {gpio_chip_n}");

        Some(Self {
            gpio_chip_n,
            line_offset: offset_rel,
        })
    }

    pub fn get_chip_number(&self) -> u32 {
        self.gpio_chip_n
    }

    pub fn get_line_offset(&self) -> u32 {
        self.line_offset
    }
}

pub struct GpioStorage {
    pins: HashMap<String, gpio_cdev::LineHandle>,
    chips: [Option<gpio_cdev::Chip>; GPIO_CHIPS_N],
}

impl GpioStorage {
    pub fn new() -> Self {
        Self {
            pins: HashMap::new(),
            chips: std::array::from_fn(|_| None),
        }
    }

    pub fn get_or_create(&mut self, id: &str) -> Option<&gpio_cdev::LineHandle> {
        use std::collections::hash_map::Entry;

        let handle: &gpio_cdev::LineHandle = match self.pins.entry(id.to_string()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let gpio_id = match GpioId::from(id) {
                    Some(v) => v,
                    None => {
                        eprintln!("Unable to parse gpio id: {id}");
                        return None;
                    }
                };

                let chip = match self.chips.get_mut(gpio_id.gpio_chip_n as usize) {
                    Some(v) => v,
                    None => {
                        // TODO
                        let chip_path = "/dev/gpiochip";
                        eprintln!(
                            "Id {id} is not supported for the current board. {id} should be located on {chip_path}{} but the last gpiochip is {chip_path}{}",
                            gpio_id.gpio_chip_n, GPIO_CHIPS_N
                        );
                        return None;
                    }
                };

                if chip.is_none() {
                    let chip_path = "/dev/gpiochip1";
                    let xchip = match gpio_cdev::Chip::new(chip_path) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Failed to create the chip {chip_path}: {e}");
                            return None;
                        }
                    };
                    *chip = Some(xchip);
                    // self.chips[gpio_id.gpio_chip_n as usize] = Some(xchip);
                }

                let output_handle =
                    match get_output_handle(chip.as_mut().unwrap(), gpio_id.line_offset) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("{e}");
                            return None;
                        }
                    };

                e.insert(output_handle)
            }
        };

        Some(handle)
    }

    //     self.pins.entry(id.to_string()).or_insert_with(|| {
    //
    //     })
    // }
}

fn get_output_handle(chip: &mut gpio_cdev::Chip, offset: u32) -> Result<gpio_cdev::LineHandle> {
    let output = chip
        .get_line(offset)
        .context(format!("failed to create the output {}", offset))?;

    let output_handle = output
        .request(gpio_cdev::LineRequestFlags::OUTPUT, 0, "act_led_blink")
        .context("failed to create the output handler: {error}")?;

    Ok(output_handle)
}
