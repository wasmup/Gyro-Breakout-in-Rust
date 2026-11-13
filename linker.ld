ENTRY(Reset_Handler)
MEMORY {
    FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 2048K
    RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 192K
    STACK (rw) : ORIGIN = 0x1000E000, LENGTH = 8K
}
_estack = ORIGIN(STACK) + LENGTH(STACK);
SECTIONS {
    .isr_vector : ALIGN(4) { KEEP(*(.isr_vector)) } > FLASH
    .text : ALIGN(4) { *(.text*) *(.rodata*) . = ALIGN(4); } > FLASH
    .ARM.exidx : ALIGN(4) { *(.ARM.exidx*) } > FLASH
    .data : ALIGN(4) { _sdata = .; *(.data*) . = ALIGN(4); _edata = .; } > RAM AT > FLASH
    _sidata = LOADADDR(.data);
    .bss (NOLOAD) : ALIGN(4) { _sbss = .; *(.bss*) *(COMMON) . = ALIGN(4); _ebss = .; } > RAM
}
