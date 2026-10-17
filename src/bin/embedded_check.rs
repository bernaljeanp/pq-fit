//! Cross-compiled Cortex-M0 build of the same hybrid handshake, to get a REAL flash/RAM
//! footprint number without owning a board yet. This does NOT run on silicon — it only
//! measures what `cargo size --release --target thumbv6m-none-eabi --bin embedded_check`
//! reports for the linked binary. See README for how to read that number, and its limits.
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use ml_kem::{kem::Decapsulate, kem::Encapsulate, EncodedSizeUser, KemCore, MlKem768};
use rand_core::{CryptoRng, Error, RngCore};
use x25519_dalek::{EphemeralSecret, PublicKey};

use panic_halt as _;

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
    let mut rng = SizeOnlyRng(0x2545_F491_4F6C_DD1D);

    let a_sk = EphemeralSecret::random_from_rng(&mut rng);
    let b_sk = EphemeralSecret::random_from_rng(&mut rng);
    let b_pk = PublicKey::from(&b_sk);
    let shared = a_sk.diffie_hellman(&b_pk);

    let (dk, ek) = MlKem768::generate(&mut rng);
    let (ct, k_send) = ek.encapsulate(&mut rng).unwrap();
    let k_recv = dk.decapsulate(&ct).unwrap();

    // Touch every result so LTO can't prove the crypto calls are dead and strip them,
    // which would make the flash-size number meaningless.
    let checksum = shared.as_bytes()[0] ^ ek.as_bytes()[0] ^ ct[0] ^ k_send[0] ^ k_recv[0];
    core::hint::black_box(checksum);

    hprintln!("hybrid handshake completed on target, checksum={}", checksum);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
