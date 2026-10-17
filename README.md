# pq-fit

**Does post-quantum key exchange fit your device's budget?**

Every lock protecting data today — passwords, bank transfers, messages — is a very good lock. In a few years there may exist a master key (a large enough quantum computer) that opens those locks easily. Big companies are already switching to new locks ("post-quantum cryptography"). But millions of small devices — industrial sensors, routers, embedded Linux boxes — still have the old lock, and the new ones are often too big or too slow to fit: more CPU time, more RAM, more bytes sent over the wire per handshake.

`pq-fit` answers one question before you commit to a migration: **does the new lock fit on this device?**

It runs a real hybrid post-quantum key exchange — classical **X25519** combined with **ML-KEM-768** (FIPS 203), the way a real migration would run them side by side — and reports, on your actual machine:

- time per step (keygen, handshake, encapsulate/decapsulate) and total
- bytes moved over the wire per step and total
- the hybrid cost as a multiple of classical-only (e.g. "4.2x the time, 36.5x the wire bytes")

That's it. It doesn't implement a protocol, store a key, or protect anything.

## Why this matters

No one knows exactly when a quantum computer capable of breaking RSA/ECC will exist ("Q-Day") — estimates from security researchers range from the early 2030s to as soon as 2027-2028. Either way, the data a device encrypts *today* can be harvested now and decrypted later, once such a machine exists ("harvest now, decrypt later"). Large organizations have multi-year migration budgets. A constrained embedded device often doesn't, and the people maintaining it usually have no easy way to know whether a post-quantum handshake will even run acceptably on the hardware they have. `pq-fit` is a first, honest step toward answering that, cheaply, before any larger migration investment.

## Running it

```bash
cargo run --release
```

```bash
cargo test --release
```

Run it **on the real target device**, not just your laptop, if you want numbers that mean anything for that device. The numbers above are a desktop stand-in and the tool says so every time it runs.

## Embedded footprint (cross-compiled + QEMU, still no physical board)

`src/bin/embedded_check.rs` is the same handshake, built `no_std` for a real Cortex-M0 target (`thumbv6m-none-eabi`), with a toy deterministic RNG standing in for hardware randomness:

```bash
rustup target add thumbv6m-none-eabi
cargo size --release --bin embedded_check --target thumbv6m-none-eabi --features embedded -- -A
```

**Flash: ~26.2 KB** (`.vector_table` + `.text` + `.rodata`, `opt-level = "z"` + LTO). **Static RAM: 8 bytes** (`.bss`/`.data`) — but that number alone is misleading, because it doesn't count the stack, and the stack is where this actually breaks:

- `memory.x` models the **BBC micro:bit** (a real Cortex-M0 board, nRF51822, 16 KB RAM), and QEMU's `microbit` machine emulates it. Booting `embedded_check` on it crashes with `HardFault`/`Lockup` — the call stack blows straight through all 16 KB of RAM.
- Re-linked against a board with more RAM (QEMU's `lm3s6965evb`, a Cortex-M3 — thumbv6m binaries run on it fine, since M0's instruction set is a strict subset) and bisecting the RAM size: it **needs roughly 48–50 KB of stack** to complete without crashing. At 48 KB it hangs; at 50 KB it finishes and prints its checksum correctly.

That's the actual result `pq-fit` exists to produce: on a real, common, cheap Cortex-M0 board, **this hybrid handshake does not fit** — not because of flash (26 KB is nothing) but because of stack depth, 3x what the chip has. A device with more RAM, or a non-reference implementation with a smaller stack footprint, might fit fine; this one, on this board, doesn't. We still don't own physical hardware, so this is QEMU, not silicon — but QEMU did execute real ARMv6-M/ARMv7-M instructions, not x86_64 ones, so the crash and the correct checksum are both real signal, not a desktop guess.

## ⚠️ Read this before you rely on anything here

- **This is a feasibility/benchmark tool, not a security product.** It does not claim to protect any secret, and it should not be wired into a real protocol as-is.
- **Neither underlying crypto crate has a confirmed independent audit.** `ml-kem` (RustCrypto) states in its own documentation that it "has never been independently audited" and says to use it at your own risk. No independent-audit claim was found for `x25519-dalek` either. `pq-fit` prints this warning every time it runs — it is not buried in this README.
- **Desktop numbers are not embedded numbers.** A real microcontroller (Cortex-M0, ESP32, etc.) will be slower and far tighter on RAM than the machine you ran this on. Use `size`/`cargo bloat --release` and a real board for ground truth before deciding anything.

## Got a real device to migrate?

If you maintain a constrained or embedded Linux device and need to know whether it can afford post-quantum key exchange — or you want a written fit report for a specific device — [open an issue](https://github.com/bernaljeanp/pq-fit/issues/new) or email bernalmirandajean@gmail.com. Async only, no calls needed.

## License

MIT.
