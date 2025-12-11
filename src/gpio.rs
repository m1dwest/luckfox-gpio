use std::collections::HashMap;

use anyhow::{Context, Result};

const GPIO_CHIP_BASE_PATH: &str = "/dev/gpiochip";
const GPIO_CHIPS_N: usize = 4;

pub struct GpioId {
    string_id: &'static str,
    chip_n: u32,
    line_offset: u32,
}

impl GpioId {
    fn calculate_abs_offset(bank: u32, port: char, pin: u32) -> u32 {
        let port_val = port.to_ascii_lowercase() as u32 - 'a' as u32;
        bank * 32 + port_val * 8 + pin
    }

    pub fn parse(id: &'static str) -> Option<Self> {
        let id = id.strip_prefix("GPIO")?;
        let (bank, rest) = id.split_once('_')?;
        let bank: u32 = bank.parse().ok()?;
        let mut rest_chars = rest.chars();
        let port = rest_chars.next()?;
        let pin: u32 = rest_chars.as_str().parse().ok()?;

        let offset_abs = GpioId::calculate_abs_offset(bank, port, pin);
        let chip_n = offset_abs / 32;
        let offset_rel = offset_abs - chip_n * 32;

        Some(Self {
            string_id: id,
            chip_n,
            line_offset: offset_rel,
        })
    }

    pub fn from_literal(id: &'static str) -> Self {
        Self::parse(id).unwrap_or_else(|| panic!("Invalid GPIO id: {id}"))
    }

    pub fn get_chip_number(&self) -> u32 {
        self.chip_n
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

    fn ensure_chip<'a>(
        chips: &'a mut [Option<gpio_cdev::Chip>],
        gpio_id: &GpioId,
    ) -> Result<&'a mut gpio_cdev::Chip> {
        let chip = chips.get_mut(gpio_id.chip_n as usize).ok_or_else(|| {
            let string_id = &gpio_id.string_id;
            let chip_n = gpio_id.chip_n;

            anyhow::anyhow!(
                "Id {string_id} is not supported for the current board. \
                {string_id} should be located on {GPIO_CHIP_BASE_PATH}{chip_n} \
                but the last gpiochip is {GPIO_CHIP_BASE_PATH}{GPIO_CHIPS_N}"
            )
        })?;

        if chip.is_none() {
            let chip_path = format!("{GPIO_CHIP_BASE_PATH}{}", gpio_id.chip_n);
            let new_chip =
                gpio_cdev::Chip::new(chip_path).context("Failed to create the chip {chip_path}")?;
            *chip = Some(new_chip);
        }

        Ok(chip.as_mut().expect("Chip Option is None"))
    }

    pub fn get_or_create(&mut self, id: &'static str) -> Result<&gpio_cdev::LineHandle> {
        use std::collections::hash_map::Entry;

        let gpio_id = GpioId::from_literal(id);
        let chip = GpioStorage::ensure_chip(&mut self.chips, &gpio_id)?;

        let handle: &gpio_cdev::LineHandle = match self.pins.entry(id.to_string()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let output_handle = get_output_handle(chip, gpio_id.line_offset)?;

                e.insert(output_handle)
            }
        };

        Ok(handle)
    }
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
