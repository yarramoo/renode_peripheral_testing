//! Bare-metal `SpiDevice<u8>` backed by STM32F4 SPI1 hardware registers.
//!
//! This is the missing HAL layer: it lets `mock_spi::MockSpiDriver` (which
//! speaks `embedded_hal::spi::SpiDevice`) actually toggle SPI1's CR1/DR
//! registers and a GPIO CS pin.
//!
//! Register map used:
//!   SPI1 base         = 0x4000_5000
//!     +0x00  CR1      – control 1  (SPE, MSTR, BR, SSM, SSI, …)
//!     +0x04  CR2      – control 2  (FRXTH)
//!     +0x08  SR       – status     (TXE bit 1, RXNE bit 0, BSY bit 7)
//!     +0x0C  DR       – data       (byte-wide access for 8-bit frames)
//!
//!   GPIOA base        = 0x4000_8000
//!     +0x18  BSRR     – bit set/reset  (CS toggle)
//!
//! CS pin = PA4 (bit 4) – matches the STM32F4 Discovery kit's default
//! SPI1 NSS mapping.  The .repl file attaches the mock to spi1, so CS
//! transitions are what trigger FinishTransmission() in the C# mock.

#![allow(dead_code)]

use embedded_hal::spi::{ErrorKind, Operation, SpiDevice};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SPI1_BASE: u32 = 0x4001_3000;
const SPI1_CR1:  u32 = SPI1_BASE + 0x00;
const SPI1_CR2:  u32 = SPI1_BASE + 0x04;
const SPI1_SR:   u32 = SPI1_BASE + 0x08;
const SPI1_DR:   u32 = SPI1_BASE + 0x0C;

const GPIOA_BASE: u32 = 0x4000_8000;
const GPIOA_BSRR: u32 = GPIOA_BASE + 0x18;

/// CS pin index within GPIOA.  PA4 = bit 4.
const CS_PIN: u32 = 4;

// CR1 bits
const CR1_MSTR:  u32 = 1 << 2;
const CR1_SPE:   u32 = 1 << 6;
const CR1_SSM:   u32 = 1 << 9;   // software slave management
const CR1_SSI:   u32 = 1 << 8;   // internal slave select (must be 1 when SSM=1 in master)
// BR[2:0] at bits 5..3 – we use 0b111 (/256) to keep it slow and safe in sim
const CR1_BR_SLOWEST: u32 = 0b111 << 3;

// CR2 bits
const CR2_FRXTH: u32 = 1 << 6;   // FIFO threshold = 1 byte (needed for 8-bit reads on F4)

// SR bits
const SR_RXNE: u32 = 1 << 0;
const SR_TXE:  u32 = 1 << 1;
const SR_BSY:  u32 = 1 << 7;

// ---------------------------------------------------------------------------
// Volatile helpers
// ---------------------------------------------------------------------------

#[inline(always)]
unsafe fn rd(addr: u32) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}

#[inline(always)]
unsafe fn wr(addr: u32, val: u32) {
    core::ptr::write_volatile(addr as *mut u32, val);
}

/// Byte-sized volatile write to DR (important: on F4 with FRXTH=1 you must
/// write only the low byte, not the full 32-bit word, to keep the 8-bit
/// frame size in effect).
#[inline(always)]
unsafe fn wr_byte(addr: u32, val: u8) {
    core::ptr::write_volatile(addr as *mut u8, val);
}

/// Byte-sized volatile read from DR (clears RXNE on F4 when FRXTH=1).
#[inline(always)]
unsafe fn rd_byte(addr: u32) -> u8 {
    core::ptr::read_volatile(addr as *const u8)
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Copy, Clone)]
pub struct Stm32SpiError;

impl embedded_hal::spi::Error for Stm32SpiError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl core::fmt::Display for Stm32SpiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "STM32 SPI error")
    }
}

// ---------------------------------------------------------------------------
// Stm32Spi1Device – implements SpiDevice<u8>
// ---------------------------------------------------------------------------

/// A zero-sized handle.  All state lives in the hardware registers.
pub struct Stm32Spi1Device;

impl Stm32Spi1Device {
    /// Configure SPI1 for Mode 0, 8-bit, master, software NSS.
    ///
    /// Call this once after RCC has clocked SPI1 and GPIOA.  It does NOT
    /// configure GPIO pin modes / alternate functions – Renode's STM32
    /// model routes SPI1 signals without explicit GPIO AF setup, so we
    /// skip that step in simulation.
    pub fn init() {
        unsafe {
            // Write CR1 with SPE=0 first (many F4 errata require config
            // while peripheral is disabled)
            let cr1 = CR1_MSTR | CR1_SSM | CR1_SSI | CR1_BR_SLOWEST;
            wr(SPI1_CR1, cr1);

            // CR2: FRXTH=1 so 8-bit reads work
            wr(SPI1_CR2, CR2_FRXTH);

            // Now enable
            wr(SPI1_CR1, cr1 | CR1_SPE);

            // Pull CS high (inactive) to start clean
            Self::cs_high();
        }
    }

    // -- CS control via GPIOA BSRR -------------------------------------------

    /// CS low = active (assert).  BSRR bits [31:16] are reset bits.
    #[inline(always)]
    unsafe fn cs_low() {
        wr(GPIOA_BSRR, 1 << (16 + CS_PIN));
    }

    /// CS high = inactive (deassert).  BSRR bits [15:0] are set bits.
    #[inline(always)]
    unsafe fn cs_high() {
        wr(GPIOA_BSRR, 1 << CS_PIN);
    }

    // -- Core transfer -------------------------------------------------------

    /// Full-duplex single-byte exchange: wait TXE, write, wait RXNE, read.
    #[inline(always)]
    unsafe fn transfer_byte(tx: u8) -> u8 {
        // Wait for transmit buffer empty
        while rd(SPI1_SR) & SR_TXE == 0 {}
        // Byte-write to DR
        wr_byte(SPI1_DR, tx);
        // Wait for receive buffer not empty
        while rd(SPI1_SR) & SR_RXNE == 0 {}
        // Byte-read from DR
        rd_byte(SPI1_DR)
    }
}

// ---------------------------------------------------------------------------
// SpiDevice impl
// ---------------------------------------------------------------------------

impl embedded_hal::spi::ErrorType for Stm32Spi1Device {
    type Error = Stm32SpiError;
}

impl SpiDevice<u8> for Stm32Spi1Device {
    fn transaction(
        &mut self,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), Stm32SpiError> {
        unsafe {
            Self::cs_low();

            for op in operations.iter_mut() {
                match op {
                    Operation::Write(buf) => {
                        for &b in buf.iter() {
                            Self::transfer_byte(b); // discard RX
                        }
                    }
                    Operation::Read(buf) => {
                        for slot in buf.iter_mut() {
                            *slot = Self::transfer_byte(0x00); // dummy TX
                        }
                    }
                    Operation::Transfer(rx, tx) => {
                        // True simultaneous full-duplex
                        for (r, &t) in rx.iter_mut().zip(tx.iter()) {
                            *r = Self::transfer_byte(t);
                        }
                    }
                    Operation::TransferInPlace(buf) => {
                        for slot in buf.iter_mut() {
                            *slot = Self::transfer_byte(*slot);
                        }
                    }
                    Operation::DelayNs(_) => {
                        // No-op in simulation – Renode's SPI model is
                        // cycle-accurate, no real timing gaps needed.
                    }
                }
            }

            Self::cs_high();
        }
        Ok(())
    }
}