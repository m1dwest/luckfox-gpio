use anyhow::{Context, Result, anyhow};

use crate::gpio;
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub enum Action {
    On,
    Off,
    Toggle,
    Status,
    Null,
}

type Reply = Option<u8>;

impl Action {
    pub fn apply(&self, value: &mut u8) -> Reply {
        match self {
            Action::On => {
                *value = 1;
                None
            }
            Action::Off => {
                *value = 0;
                None
            }
            Action::Toggle => {
                *value = if *value == 1 { 0 } else { 1 };
                None
            }
            Action::Status => Some(*value),
            Action::Null => None,
        }
    }
}

struct Signal {
    id: String,
    action: Action,
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

    pub fn add_gpio_handler(&mut self, signal: u8, id: &str, action: Action) -> Result<()> {
        self.storage.get_or_create(id)?;
        self.apply_action(id, &action)?;
        self.signals.insert(
            signal,
            Signal {
                id: id.to_owned(),
                action,
            },
        );
        Ok(())
    }

    pub fn send(&mut self, signal: u8) -> Result<Reply> {
        let (id, action) = self
            .signals
            .get(&signal)
            .map(|s| (s.id.clone(), s.action))
            .ok_or_else(|| anyhow!("No GPIO for the signal {signal} exist"))?;
        self.apply_action(id.as_str(), &action)
    }

    pub fn _status(&self, id: &str) -> Option<u8> {
        self.storage.get_value(id)
    }

    fn set_value(&mut self, id: &str, state: bool) -> Result<()> {
        let line_handler = self.storage.get_or_create(id)?;
        line_handler
            .set_value(state as u8)
            .with_context(|| format!("Unable to set {} state for GPIO {}", state, id))
    }

    fn apply_action(&mut self, id: &str, action: &Action) -> Result<Reply> {
        let line_handler = self.storage.get_or_create(id)?;
        let mut value = line_handler.get_value()?;
        let reply = action.apply(&mut value);
        line_handler
            .set_value(value)
            .with_context(|| format!("Unable to set {} value for GPIO {}", value, id))?;
        Ok(reply)
    }
}
