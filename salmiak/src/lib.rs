#![cfg_attr(target_arch = "aarch64", no_std)]
#![cfg_attr(target_arch = "aarch64", deny(warnings))]
#![cfg_attr(target_arch = "aarch64", feature(global_asm, asm, alloc_error_handler))]
#![cfg_attr(not(target_arch = "aarch64"), allow(dead_code, unused_imports))]

#[cfg(target_arch = "aarch64")]
extern crate alloc;

#[macro_use]
extern crate register;

#[cfg(not(test))]
#[macro_export]
macro_rules! sprintln {
    () => {
        $crate::serial::write("\n")
    };
    ($($arg:tt)*) => {{
        let mut writer = $crate::serial::SerialWriter;
        core::fmt::write(&mut writer, format_args!($($arg)*)).unwrap();
        $crate::serial::write("\n");
    }};
}

#[cfg(test)]
#[macro_export]
macro_rules! sprintln {
    () => {
        println!("");
    };
    ($($arg:tt)*) => {{
        println!($($arg)*);
    }};
}

#[macro_export]
macro_rules! entry {
    ($path:path) => {
        #[export_name = "main"]
        pub unsafe fn __main() -> ! {
            // type check the given path
            let f: fn() -> ! = $path;

            f()
        }
    };
}

pub mod cpu;
pub mod error;
pub mod gpu;
pub mod memory;
pub mod power;
pub mod serial;
pub mod timer;

#[cfg(target_arch = "aarch64")]
pub mod prelude {
    pub use crate::error::{SalmiakError, SalmiakErrorKind};
    pub use alloc::{
        borrow::ToOwned,
        boxed::Box,
        format,
        string::{String, ToString},
        vec::Vec,
    };

    pub mod mem_constants {
        pub const MMIO_BASE_PTR: *const u32 = 0x3f00_0000 as *const u32;
        pub const MMIO_BASE: u32 = 0x3f00_0000;
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub mod prelude {
    pub use crate::error::{SalmiakError, SalmiakErrorKind};
    pub mod mem_constants {
        pub const MMIO_BASE_PTR: *const u32 = 0x0 as *const u32;
        pub const MMIO_BASE: u32 = 0x0;
    }
}

#[cfg(target_arch = "aarch64")]
#[global_allocator]
pub(crate) static mut ALLOCATOR: memory::alloc::OriginAllocator =
    memory::alloc::OriginAllocator::new();

#[cfg(target_arch = "aarch64")]
mod main {
    use core::panic::PanicInfo;
    use cortex_a::{asm, barrier, regs::*};
    unsafe fn reset() -> ! {
        // Enable fpu
        asm!("msr cpacr_el1, $0"
             :
             : "r"(0x0030_0000) :
        );

        extern "C" {
            // Boundaries of the .bss section, provided by the linker script
            static mut __bss_start: u64;
            static mut __bss_end: u64;
            static _vectors: u64;
            static mut __end: u8;
        }

        // Zeroes the .bss section
        r0::zero_bss(&mut __bss_start, &mut __bss_end);

        extern "Rust" {
            fn main() -> !;
        }

        if let Err(e) = super::serial::init() {
            // May be hard to print but at least we tried.
            panic!("Failed to init serial: {}", e);
        }

        let exception_vectors_start: u64 = &_vectors as *const _ as u64;
        if exception_vectors_start.trailing_zeros() < 11 {
            panic!("Failed to set up exceptions.");
        } else {
            cortex_a::regs::VBAR_EL1.set(exception_vectors_start);

            // Force VBAR update to complete before next instruction.
            barrier::isb(barrier::SY);
        }

        if let Err(e) = super::memory::init(&__end) {
            panic!("Failed to init memory: {}", e);
        }

        if let Err(e) = super::cpu::init() {
            panic!("Failed to init CPU: {}", e);
        }
        main();
    }

    /// Prepare and execute transition from EL2 to EL1.
    #[inline]
    fn setup_and_enter_el1_from_el2() -> ! {
        let stack_start: u64 = _start as *const () as u64;

        // Enable timer counter registers for EL1
        CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);

        // No offset for reading the counters
        CNTVOFF_EL2.set(0);

        // Set EL1 execution state to AArch64
        HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

        // Set up a simulated exception return.
        //
        // First, fake a saved program status, where all interrupts were
        // masked and SP_EL1 was used as a stack pointer.
        SPSR_EL2.write(
            SPSR_EL2::D::Masked
                + SPSR_EL2::A::Masked
                + SPSR_EL2::I::Masked
                + SPSR_EL2::F::Masked
                + SPSR_EL2::M::EL1h,
        );

        // Second, let the link register point to reset().
        ELR_EL2.set(reset as *const () as u64);

        // Set up SP_EL1 (stack pointer), which will be used by EL1 once
        // we "return" to it.
        SP_EL1.set(stack_start);

        // Use `eret` to "return" to EL1. This will result in execution of
        // `reset()` in EL1.
        asm::eret()
    }

    /// Entrypoint of the processor.
    ///
    /// Parks all cores except core0 and checks if we started in EL2. If
    /// so, proceeds with setting up EL1.
    #[link_section = ".text.boot"]
    #[no_mangle]
    pub unsafe extern "C" fn _start() -> ! {
        const CORE_0: u64 = 0;
        const CORE_MASK: u64 = 0x3;
        const EL2: u32 = CurrentEL::EL::EL2.value;

        if (CORE_0 == MPIDR_EL1.get() & CORE_MASK) && (EL2 == CurrentEL.get()) {
            setup_and_enter_el1_from_el2()
        }

        // if not core0 or EL != 2, infinitely wait for events
        loop {
            asm::wfe();
        }
    }

    #[panic_handler]
    pub fn panic(info: &PanicInfo) -> ! {
        sprintln!("{}", info);
        loop {
            asm::wfe();
        }
    }

    #[alloc_error_handler]
    fn foo(layout: core::alloc::Layout) -> ! {
        sprintln!(
            "Memory allocation for {} bytes with alignment {} failed.",
            layout.size(),
            layout.align()
        );
        panic!();
    }
}

#[cfg(target_arch = "aarch64")]
global_asm!(include_str!("exceptions.s"));
