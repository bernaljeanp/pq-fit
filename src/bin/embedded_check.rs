//! Cross-compiled Cortex-M0 build of the same hybrid handshake — the "pqm0" track: pqm4
//! (https://github.com/mupq/pqm4) is the standard PQC cycle-count benchmark for Cortex-M4;
//! no equivalent exists yet for M0/M0+, the smallest and cheapest class of chip. This is a
//! first, QEMU-based attempt at one, not real silicon. See README for scope and limits.
//!
//! Reports two things: the linked flash/RAM footprint (via `cargo size`), and per-operation
//! cycle counts via SysTick — the one cycle-accurate timer ARMv6-M (M0) always has, unlike
//! pqm4's DWT cycle counter, which M0 lacks. QEMU's instruction timing is an approximation
//! of real silicon, not a replacement for it.
#![no_std]
#![no_main]

use cortex_m::peripheral::{syst::SystClkSource, Peripherals};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use ml_kem::{kem::Decapsulate, kem::Encapsulate, EncodedSizeUser, KemCore, MlKem768};
use rand_core::{CryptoRng, Error, RngCore};
use x25519_dalek::{EphemeralSecret, PublicKey};

use panic_halt as _;

/// Cycles elapsed running `f`, measured with SysTick (24-bit down-counter; fine for these
/// operations, which all finish in well under 2^24 cycles). Caller must have already
/// configured and started SysTick with a core-clock source and max reload.
fn cycles<T>(syst: &mut cortex_m::peripheral::SYST, f: impl FnOnce() -> T) -> (T, u32) {
    syst.clear_current();
    let out = f();
    let elapsed = cortex_m::peripheral::SYST::get_reload() - cortex_m::peripheral::SYST::get_current();
    (out, elapsed)
}

/// Deterministic, NOT cryptographically random. Exists only so the handshake code below
/// has *something* implementing RngCore to run against while we measure compiled size.
/// Never reuse this for anything that touches a real secret.
struct SizeOnlyRng(u64);

impl RngCore for SizeOnlyRng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
    fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(4) {
            chunk.copy_from_slice(&self.next_u32().to_le_bytes()[..chunk.len()]);
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}
impl CryptoRng for SizeOnlyRng {}

#[entry]
fn main() -> ! {
    let mut p = Peripherals::take().unwrap();
    p.SYST.set_clock_source(SystClkSource::Core);
    p.SYST.set_reload(0x00FF_FFFF);
    p.SYST.enable_counter();

    let mut rng = SizeOnlyRng(0x2545_F491_4F6C_DD1D);

    let (a_sk, c_a_keygen) = cycles(&mut p.SYST, || EphemeralSecret::random_from_rng(&mut rng));
    let (b_sk, c_b_keygen) = cycles(&mut p.SYST, || EphemeralSecret::random_from_rng(&mut rng));
    let b_pk = PublicKey::from(&b_sk);
    let (shared, c_dh) = cycles(&mut p.SYST, || a_sk.diffie_hellman(&b_pk));

    let ((dk, ek), c_mlkem_keygen) = cycles(&mut p.SYST, || MlKem768::generate(&mut rng));
    let ((ct, k_send), c_encap) = cycles(&mut p.SYST, || ek.encapsulate(&mut rng).unwrap());
    let (k_recv, c_decap) = cycles(&mut p.SYST, || dk.decapsulate(&ct).unwrap());

    // Touch every result so LTO can't prove the crypto calls are dead and strip them,
    // which would make these measurements meaningless.
    let checksum = shared.as_bytes()[0] ^ ek.as_bytes()[0] ^ ct[0] ^ k_send[0] ^ k_recv[0];
    core::hint::black_box(checksum);

    hprintln!("pqm0 (QEMU, not silicon) — cycles per operation:");
    hprintln!("x25519_keygen_a,{}", c_a_keygen);
    hprintln!("x25519_keygen_b,{}", c_b_keygen);
    hprintln!("x25519_diffie_hellman,{}", c_dh);
    hprintln!("mlkem768_keygen,{}", c_mlkem_keygen);
    hprintln!("mlkem768_encapsulate,{}", c_encap);
    hprintln!("mlkem768_decapsulate,{}", c_decap);
    hprintln!("checksum={}", checksum);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
