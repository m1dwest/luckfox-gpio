use anyhow::{Context, Result};

use super::Handler;
use crate::gpio;

const LED_OFF: u8 = 0x30;
const LED_ON: u8 = 0x31;
const HANDSHAKE: u8 = 0x05;

#[derive(Debug)]
pub enum LedState {
    Off,
    On,
    Blink { ms: i32 },
}

#[derive(Debug)]
pub struct Led<'a> {
    state: LedState,
    output_handle: &'a gpio_cdev::LineHandle,
    gpio_id: String,

    value_on: u8,
    value_off: u8,
}

impl<'a> Led<'a> {
    pub fn new(gpio_storage: &'a mut gpio::GpioStorage, gpio_id: &'static str) -> Result<Self> {
        let output_handle = gpio_storage
            .get_or_create(gpio_id)
            .context("Failed to get output handle for {gpio_id}")?;

        Ok(Self {
            state: LedState::Off,
            output_handle,
            gpio_id: gpio_id.to_string(),
            value_on: 1,
            value_off: 0,
        })
    }

    fn set_gpio_state(&mut self, state: LedState) -> Result<()> {
        let value = match state {
            LedState::On => self.value_on,
            LedState::Off => self.value_off,
            LedState::Blink { ms: _ } => panic!("Not implemented"),
        };

        self.output_handle.set_value(value).context(format!(
            "Unable to set {} state to value {value}",
            self.gpio_id
        ))?;

        self.state = state;

        Ok(())
    }
}

impl<'a> Handler for Led<'a> {
    fn handle(&mut self, byte: u8) -> Result<()> {
        match byte {
            LED_ON => self.set_gpio_state(LedState::On),
            LED_OFF => self.set_gpio_state(LedState::Off),
            HANDSHAKE => Ok(()),
            _ => {
                anyhow::bail!("Unable to parse the value: {byte}")
            }
        }
    }

    fn init_default(&mut self) -> Result<()> {
        self.set_gpio_state(LedState::Off)
    }
}
