/* The first 16K of flash holds the UF2 bootloader. The 104K here stops at
   0x0801E000, where the vendor settings begin

   RAM ends at 0x20004000, the address the bootloader reads a request from.

   Measured at LENGTH = 20K: flip-link puts statics at 0x200046C0..0x20005000
   and _stack_start at 0x200046C0. Statics stay 1728 bytes clear of the request
   word. The stack does not. It grows down from _stack_start and reaches the
   word after 1728 bytes of call depth. A frame parked on the word can hold the
   magic through a reset, and writing a request would corrupt that frame.

   Ending RAM here costs 4K of stack, not of statics. _stack_start drops to
   0x200036C0 and statics still grow up to 0x20004000. link.x puts .data, .bss
   and the stack in one region, and that region has to stop below the request
   word, 4K below the top of a 20K part. To get the 4092 bytes back, add a
   second region and place a section in it */
MEMORY
{
  FLASH : ORIGIN = 0x08004000, LENGTH = 104K
  RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}

/* bootloader.rs hardcodes 0x20004000. Its safety rests on the LENGTH above,
   in another file. Tie the two together */
ASSERT(ORIGIN(RAM) + LENGTH(RAM) <= 0x20004000,
  "RAM must end at or below 0x20004000, the UF2 bootloader's request word");
