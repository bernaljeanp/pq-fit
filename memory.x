/* nRF51822 memory map (BBC micro:bit, a real Cortex-M0 board) — chosen because QEMU's
   "microbit" machine emulates it exactly. This handshake overflows its 16K of RAM (see
   README); the memory map stays accurate to the real chip rather than padded to pass. */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 256K
  RAM : ORIGIN = 0x20000000, LENGTH = 16K
}
