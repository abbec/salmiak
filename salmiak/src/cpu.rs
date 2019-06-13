use crate::prelude::*;
use crate::timer;

#[no_mangle]
pub extern "C" fn print_unhandled_exception(tp: u32, esr: u32, elr: u32, far: u32) {
    let type_ = match tp {
        0 => "exception",
        1 => "irq",
        2 => "fast irq",
        _ => "error",
    };

    let cause = match esr >> 26 {
        0b00_0000 => "Unknown",
        0b00_0001 => "Trapped WFI/WFE",
        0b00_1110 => "Illegal execution",
        0b01_0101 => "System call",
        0b10_0000 => "Instruction abort, lower EL",
        0b10_0001 => "Instruction abort, same EL",
        0b10_0010 => "Instruction alignment fault",
        0b10_0100 => "Data abort, lower EL",
        0b10_0101 => "Data abort, same EL",
        0b10_0110 => "Stack alignment fault",
        0b10_1100 => "Floating point",
        _ => "Unknown",
    };

    panic!(
        "Unhandled {}, esr: 0x{:x} ({}), elr (address): 0x{:x}, far (address): 0x{:x}, goodnight...",
        type_, esr, cause, elr, far
    );
}

#[no_mangle]
pub unsafe extern "C" fn handle_irq() {
    disable_irq();

    let pending_irq = (0x4000_0060 as *mut u32).read_volatile(); // TODO: What does this address point to
    match pending_irq {
        0x01..=0x8 => timer::handle_timer_interrupt(),
        _ => sprintln!("unknown IRQ type: {}", pending_irq),
    }
    enable_irq();
}

extern "C" {
    fn enable_irq();
    fn disable_irq();
}

const INTERRUPT_CONTROLLER: *mut u32 = 0x4000_0040 as *mut u32;

pub fn init() -> Result<(), SalmiakError> {
    sprintln!("initializing cpu...");

    sprintln!("* setting up timer irq");
    timer::setup_timer_interrupt();

    sprintln!("* enabling interrupts");
    unsafe {
        INTERRUPT_CONTROLLER.write_volatile(0x2); // TODO: this should be nicer should have abstraction for interrupt controller
        enable_irq();
    }

    sprintln!("done!");
    Ok(())
}
