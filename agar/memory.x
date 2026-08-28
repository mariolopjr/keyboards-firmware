/* The first 16K of flash holds the UF2 bootloader. The 104K here stops at
   0x0801E000, where the vendor settings begin */
MEMORY
{
  FLASH : ORIGIN = 0x08004000, LENGTH = 104K
  RAM   : ORIGIN = 0x20000000, LENGTH = 20K
}
