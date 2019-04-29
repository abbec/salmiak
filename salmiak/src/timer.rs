use cortex_a::regs::*;

const INTERVAL: u64 = 20_000_000;

pub fn get_ticks() -> u64 {
    CNTPCT_EL0.get()
}

pub fn get_ms() -> Option<u64> {
    let frq = u64::from(CNTFRQ_EL0.get());
    (CNTPCT_EL0.get() * 1000).checked_div(frq)
}

pub fn setup_timer_interrupt() {
    // set the next interrupt
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("msr CNTP_TVAL_EL0, $0"
         :
         : "r"(INTERVAL)
         :
         : "volatile");
    }
    // TODO: should be CNTP_CVAL_EL0.set(cur);

    CNTP_CTL_EL0.set(0x1);
}

pub fn handle_timer_interrupt() {
    // set the next interrupt
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("msr CNTP_TVAL_EL0, $0"
             :
             : "r"(INTERVAL)
             :
             : "volatile");
    }
    // TODO: should be CNTP_CVAL_EL0.set(cur);
}
