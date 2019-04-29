pub mod alloc;
use crate::error::SalmiakError;
pub use core::{
    alloc::Layout,
    ops::{Deref, DerefMut},
    ptr::{read_volatile, write_volatile},
};

pub use self::alloc::Allocator;
use crate::gpu::mailbox::{ARMMemory, MailboxPropertyBufferBuilder};
use crate::memory::alloc::align_up;

use cortex_a::{barrier, regs::*};

pub const MB: usize = 0x100_000;

register_bitfields! {u64,
    // AArch64 Reference Manual page 2150
    STAGE1_DESCRIPTOR [
        /// Execute-never
        XN       OFFSET(54) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// Various address fields, depending on use case
        LVL1_OUTPUT_ADDR_4KiB    OFFSET(30) NUMBITS(18) [], // [47:30]
        LVL2_OUTPUT_ADDR_4KiB    OFFSET(21) NUMBITS(27) [], // [47:21]
        NEXT_LVL_TABLE_ADDR_4KiB OFFSET(12) NUMBITS(36) [], // [47:12]

        /// Access flag
        AF       OFFSET(10) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// Shareability field
        SH       OFFSET(8) NUMBITS(2) [
            OuterShareable = 0b10,
            InnerShareable = 0b11
        ],

        /// Access Permissions
        AP       OFFSET(6) NUMBITS(2) [
            RW_EL1 = 0b00,
            RW_EL1_EL0 = 0b01,
            RO_EL1 = 0b10,
            RO_EL1_EL0 = 0b11
        ],

        /// Memory attributes index into the MAIR_EL1 register
        AttrIndx OFFSET(2) NUMBITS(3) [],

        TYPE     OFFSET(1) NUMBITS(1) [
            Block = 0,
            Table = 1
        ],

        VALID    OFFSET(0) NUMBITS(1) [
            False = 0,
            True = 1
        ]
    ]
}

trait BaseAddr {
    fn base_addr(&self) -> u64;
}

impl BaseAddr for [u64; 512] {
    fn base_addr(&self) -> u64 {
        self as *const u64 as u64
    }
}

const NUM_ENTRIES_4KIB: usize = 512;

static mut LVL1_TABLE: PageTable = PageTable {
    tbl: [0; NUM_ENTRIES_4KIB],
};

static mut LVL2_TABLE: PageTable = PageTable {
    tbl: [0; NUM_ENTRIES_4KIB],
};

static mut SINGLE_LVL3_TABLE: PageTable = PageTable {
    tbl: [0; NUM_ENTRIES_4KIB],
};

// Used to force alignment on the page tables above
#[repr(C, align(4096))]
struct PageTable {
    tbl: [u64; NUM_ENTRIES_4KIB],
}

impl Deref for PageTable {
    type Target = [u64; NUM_ENTRIES_4KIB];
    fn deref(&self) -> &Self::Target {
        &self.tbl
    }
}

impl DerefMut for PageTable {
    fn deref_mut(&mut self) -> &mut [u64; NUM_ENTRIES_4KIB] {
        &mut self.tbl
    }
}

pub fn init(kernel_end: *const u8) -> Result<(), SalmiakError> {
    sprintln!("initializing memory...");
    let mut arm_memory: ARMMemory = Default::default();
    let res = MailboxPropertyBufferBuilder::new()
        .get_arm_memory(&mut arm_memory)
        .submit();

    if !res {
        panic!("Failed to get available ARM memory. Unable to create allocators.");
    }

    if kernel_end as usize <= arm_memory.base_address {
        panic!("\"This should never happen!\"")
    }

    let heap_start = align_up(kernel_end as usize, /*1*/ MB);

    sprintln!("* setting up allocators");
    sprintln!("    Kernel End: {:p}", kernel_end);
    sprintln!("    Heap Start: {:p}", heap_start as *const ());
    sprintln!(
        "    Heap End: {:p}",
        (arm_memory.base_address + arm_memory.size) as *const ()
    );
    sprintln!("    Heap Size: {} Mb", arm_memory.size / MB);

    #[cfg(target_arch = "aarch64")]
    unsafe {
        crate::ALLOCATOR.initialize(
            heap_start,
            arm_memory.base_address + arm_memory.size - heap_start,
        );

        sprintln!("* allocators initialized");

        // set up paging
        sprintln!("* setting up paging");
        init_mmu();
        sprintln!("* paging enabled");
    }

    sprintln!("done!");
    Ok(())
}

unsafe fn init_mmu() {
    // First, define the two memory types that we will map. Normal DRAM and
    // device.
    //
    MAIR_EL1.write(
        // Attribute 1
        MAIR_EL1::Attr1_HIGH::Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc
        + MAIR_EL1::Attr1_LOW_MEMORY::InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc

        // Attribute 0
        + MAIR_EL1::Attr0_HIGH::Device
        + MAIR_EL1::Attr0_LOW_DEVICE::Device_nGnRE,
    );

    mod mair {

        pub const DEVICE: u64 = 0;
        pub const NORMAL: u64 = 1;
    }

    // Set up the first LVL2 entry, pointing to a 4KiB table base address.
    let lvl3_base: u64 = SINGLE_LVL3_TABLE.base_addr() >> 12;
    LVL2_TABLE[0] = (STAGE1_DESCRIPTOR::VALID::True
        + STAGE1_DESCRIPTOR::TYPE::Table
        + STAGE1_DESCRIPTOR::NEXT_LVL_TABLE_ADDR_4KiB.val(lvl3_base))
    .value;

    // identity-map all memory, but set the mmio blocks to type "device"
    let pb: u32 = 0x3f00_0000;
    let mmio_base: u64 = (pb >> 21).into();
    let common = STAGE1_DESCRIPTOR::VALID::True
        + STAGE1_DESCRIPTOR::TYPE::Block
        + STAGE1_DESCRIPTOR::AP::RW_EL1
        + STAGE1_DESCRIPTOR::AF::True
        + STAGE1_DESCRIPTOR::XN::True;

    // set up all entries but skip the first one
    for (i, entry) in LVL2_TABLE.iter_mut().enumerate().skip(1) {
        let j: u64 = i as u64;

        let mem_attr = if j >= mmio_base {
            STAGE1_DESCRIPTOR::SH::OuterShareable + STAGE1_DESCRIPTOR::AttrIndx.val(mair::DEVICE)
        } else {
            STAGE1_DESCRIPTOR::SH::InnerShareable + STAGE1_DESCRIPTOR::AttrIndx.val(mair::NORMAL)
        };

        *entry = (common + mem_attr + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KiB.val(j)).value;
    }

    // Set up level 1 table
    // first entry points to level 2 table
    let lvl2_base: u64 = LVL2_TABLE.base_addr() >> 12;
    LVL1_TABLE[0] = (STAGE1_DESCRIPTOR::VALID::True
        + STAGE1_DESCRIPTOR::TYPE::Table
        + STAGE1_DESCRIPTOR::NEXT_LVL_TABLE_ADDR_4KiB.val(lvl2_base))
    .value;

    // second entry identity maps 1-2 Gb
    LVL1_TABLE[1] = (common
        + STAGE1_DESCRIPTOR::SH::OuterShareable
        + STAGE1_DESCRIPTOR::AttrIndx.val(mair::DEVICE)
        + STAGE1_DESCRIPTOR::LVL1_OUTPUT_ADDR_4KiB.val(1))
    .value;

    // Using the linker script, we ensure that the RO sections are 4KiB aligned,
    // and we export their boundaries via symbols.
    extern "C" {
        static mut __ro_start: u64;
        static mut __ro_end: u64;
    }

    const PAGESIZE: u64 = 4096;
    let ro_start: u64 = &__ro_start as *const _ as u64 / PAGESIZE;
    let ro_end: u64 = &__ro_end as *const _ as u64 / PAGESIZE;
    let common = STAGE1_DESCRIPTOR::VALID::True
        + STAGE1_DESCRIPTOR::TYPE::Table
        + STAGE1_DESCRIPTOR::AttrIndx.val(mair::NORMAL)
        + STAGE1_DESCRIPTOR::SH::InnerShareable
        + STAGE1_DESCRIPTOR::AF::True;

    for (i, entry) in SINGLE_LVL3_TABLE.iter_mut().enumerate() {
        let j: u64 = i as u64;

        let mem_attr = if j < ro_start || j >= ro_end {
            STAGE1_DESCRIPTOR::AP::RW_EL1 + STAGE1_DESCRIPTOR::XN::True
        } else {
            STAGE1_DESCRIPTOR::AP::RO_EL1 + STAGE1_DESCRIPTOR::XN::False
        };

        *entry = (common + mem_attr + STAGE1_DESCRIPTOR::NEXT_LVL_TABLE_ADDR_4KiB.val(j)).value;
    }

    // Point to the LVL2 table base address in TTBR0.
    TTBR0_EL1.set_baddr(LVL1_TABLE.base_addr());

    // Configure various settings of stage 1 of the EL1 translation regime.
    let ips = ID_AA64MMFR0_EL1.read(ID_AA64MMFR0_EL1::PARange);
    TCR_EL1.write(
        TCR_EL1::TBI0::Ignored
            + TCR_EL1::IPS.val(ips)
            + TCR_EL1::TG0::KiB_4 // 4 KiB granule
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::T0SZ.val(25), // Start walks at level 1
    );

    // Switch the MMU on.
    //
    // First, force all previous changes to be seen before the MMU is enabled.
    barrier::isb(barrier::SY);

    // actually enable the MMU
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

    // Force MMU init to complete before next instruction
    barrier::isb(barrier::SY);
}
