use crate::prelude::*;

const RSTC: *mut u32 = (mem_constants::MMIO_BASE + 0x100_01C) as *mut u32;
const W_DOG: *mut u32 = (mem_constants::MMIO_BASE + 0x100_024) as *mut u32;
const W_PASSWORD: u32 = 0x5a_000_000;
const W_CLR: u32 = 0xffff_ffcf;
const W_FULL_RESET: u32 = 0x0000_0020;

pub fn reset() -> ! {
    unsafe {
        // use a timeout of 10 ticks (~150us)
        W_DOG.write_volatile(W_PASSWORD | 10);
        let mut val = RSTC.read_volatile();
        val &= W_CLR;
        val |= W_PASSWORD | W_FULL_RESET;
        RSTC.write_volatile(val);
    }

    #[allow(clippy::empty_loop)]
    loop {}
}
