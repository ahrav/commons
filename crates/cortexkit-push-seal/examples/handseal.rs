// Run: cargo run -p cortexkit-push-seal --example handseal -- <recipient_key> '<json>'
//
// The key argument accepts either bare hex or a labelled block pasted whole:
//
//     push_seal_pubkey_hex=63e0...
//     apns_device_token_hex=9f21...
//
// Either `=` or `:` separates a label from its value.
//
// Prefer labelled input because the sealing key and device token share the same
// 64-hex-character shape. X25519 accepts essentially any 32 bytes as a public
// key, so using a token can produce an undecryptable blob without a sealing
// error. Label selection prevents that swap.
//
// Reject non-hex input rather than repairing it because repair could silently
// select a key the operator did not provide.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: handseal <recipient_key_hex_or_labelled_block> '<json>'");
        std::process::exit(2);
    }

    let key_hex = match select_key(&args[1]) {
        Ok(hex) => hex,
        Err(why) => {
            eprintln!("{why}");
            std::process::exit(2);
        }
    };

    let pk: Vec<u8> = (0..key_hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&key_hex[i..i + 2], 16).expect("checked above"))
        .collect();

    let sealed = cortexkit_push_seal::seal(&pk, args[2].as_bytes()).expect("seal");
    println!(
        "{}",
        sealed
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    eprintln!(
        "plaintext {} bytes -> sealed {} bytes",
        args[2].len(),
        sealed.len()
    );
}

fn select_key(raw: &str) -> Result<String, String> {
    const LABEL: &str = "push_seal_pubkey_hex";

    if raw.contains(LABEL) {
        let value = raw
            .lines()
            .find(|line| line.contains(LABEL))
            .and_then(|line| line.split([':', '=']).nth(1))
            .map(str::trim)
            .ok_or_else(|| format!("found `{LABEL}` but no value after it"))?;
        return validate(value);
    }

    // A 32-byte device token can pass public-key validation but yields ciphertext
    // that no recipient can decrypt.
    if raw.contains("apns_device_token_hex") {
        return Err(format!(
            "this block carries apns_device_token_hex but no {LABEL}. The device \
             token is not a sealing key; sealing to it would succeed and produce \
             a blob nobody can open."
        ));
    }

    validate(raw.trim())
}

// Reject empty, non-hex, or non-64-character candidate keys; do not normalize input.
fn validate(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("empty key".into());
    }
    if let Some(bad) = value.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(format!(
            "not a key: contains {bad:?}. A value carrying words, spaces or a 0x \
             prefix is a failure message written where a key belongs -- the fault \
             it names happened before the paste, so re-running this will not help."
        ));
    }
    if value.len() != 64 {
        return Err(format!(
            "expected 64 hex characters, got {}. A 66-character value is usually \
             the `SK ` label taken with the secret key.",
            value.len()
        ));
    }
    Ok(value.to_string())
}
