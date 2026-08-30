// PK is the recipient key for `handseal`; SK is the matching secret for
// `handopen`. Both have the same hex length, so swapping them seals to a keypair
// nobody holds and fails silently on the device rather than here.
fn main() {
    use hpke::{Kem, Serializable};
    let (sk, pk) = hpke::kem::X25519HkdfSha256::gen_keypair();
    let h = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    println!("SK {}", h(&sk.to_bytes()));
    println!("PK {}", h(&pk.to_bytes()));
}
