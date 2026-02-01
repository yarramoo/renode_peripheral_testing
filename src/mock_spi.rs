use embedded_hal::spi::{SpiDevice, Operation};

#[repr(u8)]
enum Command {
    Echo = 1,
    WriteReg = 2,
    ReadReg = 3,
}

#[derive(Debug)]
pub enum Error {
    Spi,
}

pub struct MockSpiDriver<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> MockSpiDriver<SPI> {
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    pub fn into_inner(self) -> SPI {
        self.spi
    }

    pub fn echo(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        if buf.len() == 0 {
            return Ok(());
        }

        let len = buf.len();

        let mut wire = [0u8; 257];
        wire[0] = Command::Echo as u8;
        wire[1..=len].copy_from_slice(buf);

        self.spi
            .transfer_in_place(&mut wire[..len + 2])
            .map_err(|_| Error::Spi)?;

        buf.copy_from_slice(&wire[2..len + 2]);

        Ok(())
    }

    pub fn write_reg(&mut self, addr: u8, value: u8) -> Result<(), Error> {
        self.spi
            .transaction(&mut [Operation::Write(&[Command::WriteReg as u8, addr, value])])
            .map_err(|_| Error::Spi)
    }

    pub fn read_reg(&mut self, addr: u8) -> Result<u8, Error> {
        let mut rx = [0u8; 3];

        self.spi
            .transaction(&mut [Operation::Transfer(&mut rx, &[Command::ReadReg as u8, addr, 0x0])])
            .map_err(|_| Error::Spi)?;

        Ok(rx[0])
    }
}


