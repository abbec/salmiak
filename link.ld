ENTRY(_start)

SECTIONS
{
    . = 0x80000;

    __ro_start = .;
    .text :
    {
        KEEP(*(.text.boot)) *(.text .text.*)
    }

    .vectors ALIGN(2048):
    {
        *(.vectors)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4096); /* Fill up to 4KiB */
    __ro_end = .;

    .data :
    {
        *(.data .data.*)
    }


    .bss ALIGN(8):
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    __end = .;

   /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}

__bss_size = (__bss_end - __bss_start)>>3;
