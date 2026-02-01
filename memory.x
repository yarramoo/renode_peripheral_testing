/* Memory layout for STM32F407VG */
/* STM32F407 has 1MB Flash and 192KB RAM (128KB + 64KB CCM) */

MEMORY
{
  /* Main Flash memory - starts at 0x0800_0000 */
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K

  /* Main SRAM - starts at 0x2000_0000 */
  RAM : ORIGIN = 0x20000000, LENGTH = 128K

  /* CCM (Core Coupled Memory) - faster but not accessible by DMA */
  /* Starts at 0x1000_0000 */
  CCMRAM : ORIGIN = 0x10000000, LENGTH = 64K
}

/* The location of the stack can be overridden using the
   `_stack_start` symbol. Place the stack at the end of RAM */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
