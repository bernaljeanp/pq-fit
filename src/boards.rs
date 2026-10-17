//! Public datasheet specs for a handful of common, cheap microcontroller boards, used to
//! turn the one footprint we actually measured (see embedded_check + README) into a
//! quick "where would this fit" table, instead of making the reader cross-reference
//! datasheets themselves. Flash/RAM figures are the chip's total, not application budget.

pub struct Board {
    pub name: &'static str,
    pub flash_bytes: u32,
    pub ram_bytes: u32,
}

pub const BOARDS: &[Board] = &[
    Board { name: "STM32F030F4 (Cortex-M0, ~$1)", flash_bytes: 16 * 1024, ram_bytes: 4 * 1024 },
    Board { name: "BBC micro:bit / nRF51822 (Cortex-M0)", flash_bytes: 256 * 1024, ram_bytes: 16 * 1024 },
    Board { name: "nRF52832 (Cortex-M4)", flash_bytes: 512 * 1024, ram_bytes: 64 * 1024 },
    Board { name: "Raspberry Pi Pico / RP2040 (Cortex-M0+)", flash_bytes: 2 * 1024 * 1024, ram_bytes: 264 * 1024 },
    Board { name: "ESP32 (Xtensa LX6)", flash_bytes: 4 * 1024 * 1024, ram_bytes: 320 * 1024 },
    Board { name: "STM32L4R5 (Cortex-M4)", flash_bytes: 2 * 1024 * 1024, ram_bytes: 640 * 1024 },
];

/// Prints a fit/no-fit table for a measured (flash, stack) footprint against `BOARDS`.
/// Flash and stack numbers aren't measured by this binary — they come from running
/// embedded_check under cargo-size/QEMU on the actual target (see README). This just
/// applies that one measurement across known chips instead of checking datasheets by hand.
pub fn print_fit_table(flash_bytes: u32, stack_bytes: u32, source_note: &str) {
    println!("\n== Does it fit on a real board? ({source_note})");
    println!(
        "  handshake needs: {} bytes flash, ~{} bytes of stack",
        flash_bytes, stack_bytes
    );
    for b in BOARDS {
        let flash_ok = flash_bytes <= b.flash_bytes;
        let ram_ok = stack_bytes <= b.ram_bytes;
        let verdict = if flash_ok && ram_ok { "FITS" } else { "DOES NOT FIT" };
        println!(
            "  {:<42} flash {}K/{}K  ram {}K/{}K  -> {}",
            b.name,
            flash_bytes / 1024,
            b.flash_bytes / 1024,
            stack_bytes / 1024,
            b.ram_bytes / 1024,
            verdict
        );
    }
}
