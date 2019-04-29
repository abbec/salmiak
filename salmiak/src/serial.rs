use crate::gpu::mailbox::{self, MailboxPropertyBufferBuilder};

use crate::prelude::*;
use core::fmt::{Error, Write};

const UART_DR: u32 = 0x3F20_1000;

// The GPIO registers base address.
const GPIO_BASE: u32 = 0x3F20_0000;

// Controls actuation of pull up/down to ALL GPIO pins.
const GPPUD: *mut u32 = (GPIO_BASE + 0x94) as *mut u32;

// Controls actuation of pull up/down for specific GPIO pin.
const GPPUDCLK0: *mut u32 = (GPIO_BASE + 0x98) as *mut u32;

const GPFSEL1: *mut u32 = 0x3f20_0004 as *mut u32;
// const GPSET0: u32 = 0x3f20001C;
// const GPCLR0: u32 = 0x3f200028;

const UART0_DR: *mut u32 = UART_DR as *mut u32;
// const UART0_RSRECR: u32 = (UART_DR + 0x04);
const UART0_FR: *mut u32 = (UART_DR + 0x18) as *mut u32;
// const UART0_ILPR: u32 = (UART_DR + 0x20);
const UART0_IBRD: *mut u32 = (UART_DR + 0x24) as *mut u32;
const UART0_FBRD: *mut u32 = (UART_DR + 0x28) as *mut u32;
const UART0_LCRH: *mut u32 = (UART_DR + 0x2C) as *mut u32;
const UART0_CR: *mut u32 = (UART_DR + 0x30) as *mut u32;
// const UART0_IFLS: u32 = (UART_DR + 0x34);
// const UART0_IMSC: u32 = (UART_DR + 0x38);
// const UART0_RIS: u32 = (UART_DR + 0x3C);
// const UART0_MIS: u32 = (UART_DR + 0x40);
const UART0_ICR: *mut u32 = (UART_DR + 0x44) as *mut u32;
// const UART0_DMACR: u32 = (UART_DR + 0x48);
// const UART0_ITCR: u32 = (UART_DR + 0x80);
// const UART0_ITIP: u32 = (UART_DR + 0x84);
// const UART0_ITOP: u32 = (UART_DR + 0x88);
// const UART0_TDR: u32 = (UART_DR + 0x8C);

fn transmit_fifo_full() -> bool {
    unsafe { UART0_FR.read_volatile() & (1 << 5) != 0 }
}

pub fn readchar() -> Option<u8> {
    unsafe {
        if UART0_FR.read_volatile() & (1 << 4) == 0 {
            let c = UART0_DR.read_volatile() as u8;
            if c != 0 {
                return Some(c);
            }
        }
    }

    None
}

pub fn writechar(c: u8) {
    while transmit_fifo_full() {}
    unsafe {
        UART0_DR.write_volatile(u32::from(c));
    }
}

pub fn write(msg: &str) {
    for c in msg.chars() {
        writechar(c as u8)
    }
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        write(s);
        Ok(())
    }
}

pub fn delay(count: u32) {
    for _ in 0..count {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            asm!("nop" :::: "volatile");
        }
    }
}

pub fn init() -> Result<(), SalmiakError> {
    // Disable UART0
    unsafe {
        UART0_CR.write_volatile(0x0);
    }

    // we want consistent divisor values and
    // therefore set the clock rate of the UART
    let res = MailboxPropertyBufferBuilder::new()
        .set_clock_rate(
            mailbox::clock::UART,
            4_000_000, // 4 MHz
            0,         // skip turbo
            None,
        )
        .submit();

    if !res {
        return Err(
            SalmiakErrorKind::InitSerialError("Failed to set serial clockrate".to_owned()).into(),
        );
    }

    unsafe {
        let mut ra = GPFSEL1.read_volatile();
        ra &= !((7 << 12) | (7 << 15)); //gpio14, gpio15
        ra |= (4 << 12) | (4 << 15); //alt0
        GPFSEL1.write_volatile(ra);

        // Disable pull up/down for all GPIO pins and delay for 150 cycles
        GPPUD.write_volatile(0x0);
        delay(150);

        // Disable pull up/down for pin 14,15 & delay for 150 cycles.
        GPPUDCLK0.write_volatile((1 << 14) | (1 << 15));
        delay(150);

        // Write 0 to GPPUDCLK0 to make it take effect.
        GPPUDCLK0.write_volatile(0x0);

        // Clear pending interrupts.
        UART0_ICR.write_volatile(0x7ff);

        // Set integer & fractional part of baud rate.
        // Divider = UART_CLOCK/(16 * Baud)
        // Fraction part register = (Fractional part * 64) + 0.5
        // UART_CLOCK = 3000000; Baud = 115200.

        UART0_IBRD.write_volatile(2);
        UART0_FBRD.write_volatile(0xb);

        // Enable FIFO & 8 bit data transmissio (1 stop bit, no parity).
        UART0_LCRH.write_volatile(0b11 << 5);

        // Enable UART0, receive & transfer part of UART.
        UART0_CR.write_volatile((1) | (1 << 8) | (1 << 9));
        Ok(())
    }
}
