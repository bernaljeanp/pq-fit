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

`cargo run --release` also prints this same result applied across a few common, cheap boards (public datasheet specs, not individually measured):

| Board | Flash | RAM | Fits? |
|---|---|---|---|
| STM32F030F4 (Cortex-M0, ~$1) | 16 KB | 4 KB | ❌ |
| BBC micro:bit / nRF51822 (Cortex-M0) | 256 KB | 16 KB | ❌ |
| nRF52832 (Cortex-M4) | 512 KB | 64 KB | ✅ |
| Raspberry Pi Pico / RP2040 (Cortex-M0+) | 2 MB | 264 KB | ✅ |
| ESP32 (Xtensa LX6) | 4 MB | 320 KB | ✅ |
| STM32L4R5 (Cortex-M4) | 2 MB | 640 KB | ✅ |

Against our ~26.2 KB flash / ~51 KB stack: fine on anything with 64 KB+ of RAM, a real problem on the smallest, cheapest Cortex-M0 chips — which is exactly where this question matters most.

## The "pqm0" track: real cycle counts on an M0-class core

[pqm4](https://github.com/mupq/pqm4) is the standard cycle-count benchmark for PQC on Cortex-M4. Nothing equivalent exists yet for **Cortex-M0/M0+** — the smallest, cheapest chip class, and the one where "does it fit" matters most. `embedded_check` is a first attempt at that, using **SysTick** (the one cycle-accurate timer ARMv6-M always has — M0 has no DWT cycle counter, unlike M4) instead of wall-clock time:

```
pqm0 (QEMU, not silicon) — cycles per operation:
x25519_keygen_a,3433
x25519_keygen_b,450
x25519_diffie_hellman,117566
mlkem768_keygen,85805
mlkem768_encapsulate,49431
mlkem768_decapsulate,57238
```

Measured on QEMU's `lm3s6965evb` (Cortex-M3, since the real micro:bit's 16 KB RAM crashes before any cycle count can print — see above). Two honest caveats: QEMU's instruction timing approximates real silicon, it isn't cycle-accurate the way actual hardware is; and `diffie_hellman` costs ~35x more than the second `x25519_keygen` because `x25519-dalek`'s `precomputed-tables` feature is disabled here (it needs `alloc`, which doesn't fit this no-heap build) — a real M0 port would want to resolve that trade-off, not just inherit it. This is v0.1 of one scheme on one implementation; a real "pqm0" needs more schemes, more implementations, and eventually a real board. PRs and issues welcome.

## ⚠️ Read this before you rely on anything here

- **This is a feasibility/benchmark tool, not a security product.** It does not claim to protect any secret, and it should not be wired into a real protocol as-is.
- **Neither underlying crypto crate has a confirmed independent audit.** `ml-kem` (RustCrypto) states in its own documentation that it "has never been independently audited" and says to use it at your own risk. No independent-audit claim was found for `x25519-dalek` either. `pq-fit` prints this warning every time it runs — it is not buried in this README.
- **Desktop numbers are not embedded numbers.** A real microcontroller (Cortex-M0, ESP32, etc.) will be slower and far tighter on RAM than the machine you ran this on. Use `size`/`cargo bloat --release` and a real board for ground truth before deciding anything.

## Got a real device to migrate?

If you maintain a constrained or embedded Linux device and need to know whether it can afford post-quantum key exchange — or you want a written fit report for a specific device — [request a fit report](https://github.com/bernaljeanp/pq-fit/issues/new?template=fit-report-request.yml) or email bernalmirandajean@gmail.com. Async only, no calls needed.

## License

MIT.
