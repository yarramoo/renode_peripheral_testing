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

#[entry]
fn main() -> ! {

    // ---------------------------------------------------------------
    // 2. Configure USART2 (base 0x4000_4400)
    //    On STM32F4 the register layout is:
    //      +0x00  SR   (status)
    //      +0x04  DR   (data)
    //      +0x08  BRR  (baud-rate)
    //      +0x0C  CR1  (control 1)
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