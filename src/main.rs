//! Bare-metal test harness for MockSpiDriver on STM32F4.
//!
//! This file is deliberately kept thin.  It contains only the entry point,
//! a minimal UART writer (so you can see results in Renode's analyzer), and
//! the test sequence.  The actual SPI peripheral setup is gated behind a
//! feature flag so the same source works with different HAL crates.
//!
//! ## Building
//!
//! See README.md for the full flow.  The short version:
//!
//!   cargo +nightly build --target thumbv7em-none-eabihf --release \
//!       --features stm32f4xx-hal
//!
//! ## What you'll see in Renode
//!
//! The UART2 analyzer will print lines like:
//!
//!   [PASS] write_reg / read_reg: wrote 0xAB, read back 0xAB
//!   [PASS] echo: sent [11 22 33], got back [11 22 33]
//!   All tests passed.

#![no_std]
#![no_main]

mod mock_spi;
mod stm32_spi;

use mock_spi::MockSpiDriver;

use cortex_m_rt::entry;

// ---------------------------------------------------------------------------
// Tiny UART2 writer – enough to print ASCII to the Renode analyzer.
// USART2 base on STM32F4 = 0x4000_4400.  STM32F4 USART register map:
//   +0x00  SR   – status register   (TXE is bit 7)
//   +0x04  DR   – data register
//   +0x08  BRR  – baud-rate register
//   +0x0C  CR1  – control register 1
// ---------------------------------------------------------------------------

const USART2_BASE: u32 = 0x4000_4400;
const USART2_SR: *const u32 = (USART2_BASE + 0x00) as *const u32;
const USART2_DR: *mut u32   = (USART2_BASE + 0x04) as *mut u32;

fn uart_write_byte(b: u8) {
    unsafe {
        // Wait for TXE (bit 7)
        while (*USART2_SR & (1 << 7)) == 0 {}
        *USART2_DR = b as u32;
    }
}

fn uart_print(s: &str) {
    for b in s.bytes() {
        uart_write_byte(b);
    }
}

fn uart_println(s: &str) {
    uart_print(s);
    uart_write_byte(b'\r');
    uart_write_byte(b'\n');
}

/// Print a u8 as two hex chars.
fn uart_print_hex(v: u8) {
    const HEX: &[u8] = b"0123456789ABCDEF";
    uart_write_byte(HEX[(v >> 4) as usize]);
    uart_write_byte(HEX[(v & 0x0F) as usize]);
}

fn uart_print_hex_slice(slice: &[u8]) {
    uart_print("[");
    for (i, &b) in slice.iter().enumerate() {
        if i > 0 {
            uart_print(" ");
        }
        uart_print_hex(b);
    }
    uart_print("]");
}

// ---------------------------------------------------------------------------
// Stub SPI implementation – used ONLY so that `cargo check` works on a
// host machine (x86_64).  When you actually cross-compile with a real HAL,
// you replace `get_spi_device()` with the HAL's type.
//
// For a quick smoke-test on host:
//   cargo check                  # compiles the logic, no cross-compile needed
// ---------------------------------------------------------------------------

/// A dead-simple in-memory mock that implements SpiDevice<u8>.
/// It just loops back TX → RX (no protocol logic).  Good enough to let the
/// compiler verify types; real behaviour comes from Renode + the C# mock.
mod host_stub {
    use embedded_hal::spi::{ErrorKind, Operation, SpiDevice};

    #[derive(Debug)]
    pub struct StubError;

    impl embedded_hal::spi::Error for StubError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    impl core::fmt::Display for StubError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "stub")
        }
    }

    pub struct StubSpiDevice;

    impl embedded_hal::spi::ErrorType for StubSpiDevice {
        type Error = StubError;
    }

    impl SpiDevice<u8> for StubSpiDevice {
        fn transaction(
            &mut self,
            operations: &mut [Operation<'_, u8>],
        ) -> Result<(), StubError> {
            for op in operations.iter_mut() {
                match op {
                    Operation::Read(buf) => buf.iter_mut().for_each(|b| *b = 0xAB),
                    Operation::Write(_) => {}
                    Operation::Transfer(read, write) => {
                        for (r, w) in read.iter_mut().zip(write.iter()) {
                            *r = *w;
                        }
                    }
                    Operation::TransferInPlace(buf) => {} // no-op
                    Operation::DelayNs(_) => {}
                }
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Cortex-M vector table
//
// The CPU does not use ENTRY() or any symbol lookup at reset.  It literally
// reads two 32-bit words from address 0x0800_0000:
//   [0]  initial value of SP
//   [1]  address of the reset handler (with bit 0 set → Thumb mode)
//
// We place this array in the `.vector_table` input section so the linker
// script puts it at the very start of flash.
// ---------------------------------------------------------------------------

/// Reset handler – the CPU jumps here after reading word 1 of the vector
/// table.  All it does is call _start(); Renode's STM32 model already
/// initialises SP from word 0 for us.



// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[entry]
fn main() -> ! {
    // ---------------------------------------------------------------
    // 1. Clock-enable peripherals via RCC
    //    RCC base = 0x4002_1000 (STM32F407)
    //    APB1ENR @ +0x40  – bit 17 = USART2
    //    APB2ENR @ +0x44  – bit 0  = GPIOA, bit 12 = SPI1
    // ---------------------------------------------------------------
    // unsafe {
    //     let rcc_apb1enr = 0x4002_1040u32 as *mut u32;
    //     let rcc_apb2enr = 0x4002_1044u32 as *mut u32;

    //     // Enable USART2 clock (APB1, bit 17)
        // core::ptr::write_volatile(rcc_apb1enr,
        //     core::ptr::read_volatile(rcc_apb1enr) | (1 << 17));

    //     // Enable GPIOA + SPI1 clocks (APB2, bits 0 and 12)
    //     core::ptr::write_volatile(rcc_apb2enr,
    //         core::ptr::read_volatile(rcc_apb2enr) | (1 << 0) | (1 << 12));
    // }

    // ---------------------------------------------------------------
    // 2. Configure USART2 (base 0x4000_4400)
    //    On STM32F4 the register layout is:
    //      +0x00  SR   (status)
    //      +0x04  DR   (data)
    //      +0x08  BRR  (baud-rate)
    //      +0x0C  CR1  (control 1)
    //
    //    Renode's STM32 USART model sets TXE as soon as TE is enabled,
    //    regardless of baud-rate divider, so we just need a non-zero BRR
    //    and TE + UE set.  BRR = 0x36 gives ~115 200 at 16 MHz APB1
    //    (doesn't matter in simulation, but keeps it realistic).
    // ---------------------------------------------------------------
    unsafe {
        let usart2_brr = 0x4000_4408u32 as *mut u32;
        let usart2_cr1 = 0x4000_440Cu32 as *mut u32;

        // BRR: non-zero so the peripheral considers itself configured
        core::ptr::write_volatile(usart2_brr, 0x36);

        // CR1: TE (bit 3) | UE (bit 13) – transmit-enable + USART-enable
        core::ptr::write_volatile(usart2_cr1, (1 << 3) | (1 << 13));
    }

    // ---------------------------------------------------------------
    // Now UART is live — everything below can print.
    // ---------------------------------------------------------------
    uart_println("USART2 initialised.");

    stm32_spi::Stm32Spi1Device::init();
    uart_println("SPI1 initialised.");

    let mut dev = MockSpiDriver::new(stm32_spi::Stm32Spi1Device);

    // --- Test 1: write_reg / read_reg -----------------------------------
    let write_val: u8 = 0xAB;
    let reg_addr: u8 = 0x03;

    match dev.write_reg(reg_addr, write_val) {
        Ok(()) => {}
        Err(_) => uart_println("[FAIL] write_reg returned an error"),
    }

    match dev.read_reg(reg_addr) {
        Ok(v) if v == write_val => {
            uart_print("[PASS] write_reg / read_reg: wrote 0x");
            uart_print_hex(write_val);
            uart_print(", read back 0x");
            uart_print_hex(v);
            uart_write_byte(b'\r');
            uart_write_byte(b'\n');
        }
        Ok(v) => {
            uart_print("[FAIL] read_reg: expected 0x");
            uart_print_hex(write_val);
            uart_print(", got 0x");
            uart_print_hex(v);
            uart_write_byte(b'\r');
            uart_write_byte(b'\n');
        }
        Err(_) => uart_println("[FAIL] read_reg returned an error"),
    }

    // --- Test 2: echo --------------------------------------------------
    let mut echo_buf: [u8; 3] = [0x11, 0x22, 0x33];
    let expected = echo_buf;

    uart_print("echo sent ");
    uart_print_hex_slice(&echo_buf);
    uart_print(", ");

    match dev.echo(&mut echo_buf) {
        Ok(()) => {
            uart_print("got back ");
            uart_print_hex_slice(&echo_buf);
            if echo_buf == expected {
                uart_println(" [PASS]");
            } else {
                uart_println(" [FAIL]");
            }
        }
        Err(_) => uart_println("[FAIL] echo returned an error"),
    }

    uart_println("All tests finished.");

    // Halt – spin forever so Renode doesn't fly off into unmapped memory.
    loop {}
}

// ---------------------------------------------------------------------------
// Panic handler (required by #![no_std])
// ---------------------------------------------------------------------------

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    uart_println("[PANIC]");
    loop {}
}