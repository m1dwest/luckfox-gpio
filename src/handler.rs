use anyhow::{Context, Result, anyhow};

use crate::gpio;
use std::collections::HashMap;

struct Signal {
    id: String,
    state: bool,
}

pub struct Handler<'a> {
    signals: HashMap<u8, Signal>,
    storage: &'a mut gpio::GpioStorage,
}

impl<'a> Handler<'a> {
    pub fn new(gpio_storage: &'a mut gpio::GpioStorage) -> Self {
        Self {
            signals: HashMap::new(),
            storage: gpio_storage,
        }
    }

    pub fn enable_gpio(&mut self, id: &str, signal: u8, state: bool) -> Result<()> {
        self.storage.get_or_create(id)?;
        self.set_value(id, state)?;
        self.signals.insert(
            signal,
            Signal {
                id: id.to_owned(),
                state,
            },
        );
        Ok(())
    }

    pub fn send(&mut self, signal: u8) -> Result<()> {
        let (id, state) = self
            .signals
            .get(&signal)
            .map(|s| (s.id.clone(), s.state))
            .ok_or_else(|| anyhow!("No GPIO for the signal {signal} exist"))?;
        self.set_value(id.as_str(), state)
    }

    pub fn status(&self, id: &str) -> Option<u8> {
        self.storage.get_value(id)
    }

    fn set_value(&mut self, id: &str, state: bool) -> Result<()> {
        let line_handler = self.storage.get_or_create(id)?;
        line_handler
            .set_value(state as u8)
            .with_context(|| format!("Unable to set {} state for GPIO {}", state, id))
    }
}
