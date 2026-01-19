use anyhow::Result;

pub trait Handler {
    fn init_default(&mut self) -> Result<()>;
    fn handle(&mut self, byte: u8) -> Result<()>;
}
