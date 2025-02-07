MEMORY {
    ROM         (rx) : ORIGIN = 0x00000000, LENGTH = 0x00010000 /* 64kB ROM */
    BOOTLOADER  (rx) : ORIGIN = 0x10000000, LENGTH = 0x0000E000 /* Bootloader flash */
    FLASH       (rx) : ORIGIN = 0x1000E000, LENGTH = 0x00036000 /* Location of team firmware */
    PERSIST     (rw) : ORIGIN = 0x10044000, LENGTH = 0x00002000 /* Page of flash that we use for persistence */
    RESERVED    (rw) : ORIGIN = 0x10046000, LENGTH = 0x00038000 /* Reserved */
    ROM_BL_PAGE (rw) : ORIGIN = 0x1007E000, LENGTH = 0x00002000 /* Reserved */
    RAM         (rwx): ORIGIN = 0x20000000, LENGTH = 0x00020000 /* 64kB RAM + another secret 64k */
}

_stext = ORIGIN(FLASH) + 0x200; /* Jump point for bootloader */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS {
    .flash_code :
    {
        . = ALIGN(4);
        *(.flashprog*)
        . = ALIGN(4);
    } > RAM AT>FLASH
}

INSERT AFTER .data;