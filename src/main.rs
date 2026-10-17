//! pq-fit: does hybrid post-quantum key exchange (X25519 + ML-KEM-768) fit YOUR device's time, memory and binary-size budget?
//!
//! This is a FEASIBILITY tool, not a security product: it answers "can my hardware afford this?" before anyone wires it
//! into a real protocol. It does not protect any secret. Both crypto crates it uses say so themselves:
//!   - `ml-kem` (RustCrypto): "has never been independently audited! USE AT YOUR OWN RISK!"
//!   - `x25519-dalek`: no independent-audit claim found either.
//! Benchmarks below run on THIS machine, as a desktop stand-in. A real embedded target (Cortex-M0, ESP32, ...) will be
//! slower and tighter on RAM; `size` command, `cargo bloat`, and a real board are still the only ground truth.
use std::{mem::size_of, time::Instant};

use ml_kem::{kem::Decapsulate, kem::Encapsulate, EncodedSizeUser, KemCore, MlKem768};
use rand_core::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey};

/// One measured step: name, how long it took, and how many bytes it moved over the wire (0 if none).
struct Step {
    name: &'static str,
    micros: u128,
    wire_bytes: usize,
}

fn timed<T>(name: &'static str, wire_bytes: usize, f: impl FnOnce() -> T) -> (T, Step) {
    let start = Instant::now();
    let out = f();
    (out, Step { name, micros: start.elapsed().as_micros(), wire_bytes })
}

/// Classical-only handshake (X25519), for comparison.
fn classical_handshake() -> Vec<Step> {
    let (a_sk, step_a) = timed("x25519 keygen (side A)", 32, || EphemeralSecret::random_from_rng(OsRng));
    let a_pk = PublicKey::from(&a_sk);
    let (b_sk, step_b) = timed("x25519 keygen (side B)", 32, || EphemeralSecret::random_from_rng(OsRng));
    let b_pk = PublicKey::from(&b_sk);
    let (shared_a, step_dh_a) = timed("x25519 diffie-hellman (side A)", 0, || a_sk.diffie_hellman(&b_pk));
    let (shared_b, step_dh_b) = timed("x25519 diffie-hellman (side B)", 0, || b_sk.diffie_hellman(&a_pk));
    assert_eq!(shared_a.as_bytes(), shared_b.as_bytes(), "x25519 handshake did not agree on a shared secret");
    vec![step_a, step_b, step_dh_a, step_dh_b]
}

/// ML-KEM-768 handshake alone (the post-quantum half, unaudited reference implementation).
fn mlkem_handshake() -> Vec<Step> {
    let ((dk, ek), step_keygen) = timed("ML-KEM-768 keygen", 0, || MlKem768::generate(&mut OsRng));
    let ek_wire = ek.as_bytes().len();
    let ((ct, k_send), step_encap) = timed("ML-KEM-768 encapsulate", 0, || ek.encapsulate(&mut OsRng).expect("encapsulate"));
    let ct_wire = ct.len();
    let (k_recv, step_decap) = timed("ML-KEM-768 decapsulate", 0, || dk.decapsulate(&ct).expect("decapsulate"));
    assert_eq!(k_send, k_recv, "ML-KEM handshake did not agree on a shared secret");
    vec![
        Step { wire_bytes: ek_wire, ..step_keygen },
        Step { wire_bytes: ct_wire, ..step_encap },
        step_decap,
    ]
}

fn report(title: &str, steps: &[Step]) -> (u128, usize) {
    println!("\n== {title}");
    let mut total_us = 0u128;
    let mut total_wire = 0usize;
    for s in steps {
        println!("  {:<34} {:>8} us   {:>6} bytes on the wire", s.name, s.micros, s.wire_bytes);
        total_us += s.micros;
        total_wire += s.wire_bytes;
    }
    println!("  {:<34} {:>8} us   {:>6} bytes total", "TOTAL", total_us, total_wire);
    (total_us, total_wire)
}

fn main() {
    println!("pq-fit 0.1.0 - does hybrid post-quantum key exchange fit your budget?");
    println!("WARNING: ml-kem and x25519-dalek here are not confirmed independently audited. This measures FIT, not safety.");
    println!("These numbers are from THIS machine, a desktop stand-in. Re-run on your real target before deciding anything.\n");

    let (classical_us, classical_wire) = report("Classical only: X25519", &classical_handshake());
    let (pq_us, pq_wire) = report("Post-quantum only: ML-KEM-768", &mlkem_handshake());

    println!("\n== Hybrid (both, as a real migration would run them side by side)");
    println!("  combined time:  {} us ({:.1}x classical-only)", classical_us + pq_us, (classical_us + pq_us) as f64 / classical_us.max(1) as f64);
    println!("  combined wire:  {} bytes ({:.1}x classical-only)", classical_wire + pq_wire, (classical_wire + pq_wire) as f64 / classical_wire.max(1) as f64);
    println!("  static sizes:   X25519 key/shared-secret = {} bytes each (stack)", size_of::<[u8; 32]>());
    println!("\nFor flash/RAM on a real embedded target: build with --release, then run `size` or `cargo bloat --release` on the binary.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classical_handshake_agrees_and_reports_real_wire_sizes() {
        let steps = classical_handshake();
        // two keygens (32 bytes each, public key on the wire) + two DH computations (0 wire bytes: already exchanged)
        assert_eq!(steps.iter().filter(|s| s.name.contains("keygen")).count(), 2);
        assert!(steps.iter().filter(|s| s.name.contains("keygen")).all(|s| s.wire_bytes == 32));
        assert!(steps.iter().filter(|s| s.name.contains("diffie-hellman")).all(|s| s.wire_bytes == 0));
    }

    #[test]
    fn mlkem_handshake_agrees_and_reports_the_standard_ml_kem_768_sizes() {
        // FIPS 203 ML-KEM-768 fixed sizes: encapsulation key 1184 B, ciphertext 1088 B.
        let steps = mlkem_handshake();
        let ek = steps.iter().find(|s| s.name.contains("keygen")).unwrap();
        let ct = steps.iter().find(|s| s.name.contains("encapsulate")).unwrap();
        assert_eq!(ek.wire_bytes, 1184, "ML-KEM-768 encapsulation key must be 1184 bytes per FIPS 203");
        assert_eq!(ct.wire_bytes, 1088, "ML-KEM-768 ciphertext must be 1088 bytes per FIPS 203");
    }

    #[test]
    fn report_sums_time_and_wire_bytes_correctly() {
        let steps = vec![Step { name: "a", micros: 10, wire_bytes: 100 }, Step { name: "b", micros: 5, wire_bytes: 0 }];
        let (us, wire) = report("test", &steps);
        assert_eq!(us, 15);
        assert_eq!(wire, 100);
    }
}
