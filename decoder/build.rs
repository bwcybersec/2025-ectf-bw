//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.
//!
//! The build script also sets the linker flags to tell it which link script to use.

use std::env::{self, var};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use serde::Deserialize;
use sha2::Sha256;

#[derive(Deserialize)]
struct Secrets {
    deployment_key: String,
    salt: String,
    channel_0_key: String,
    signing_sk: String,
}

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo::rustc-link-search={}", out.display());

    // I'm (ab)using build.rs for bringing secrets and stuff in. So I'm just
    // forcing a rerun of build.rs everytime.

    // Thanks to this random forum post for the bad idea
    // https://users.rust-lang.org/t/how-can-i-make-build-rs-rerun-every-time-that-cargo-run-or-cargo-build-is-run/51852/5

    std::env::set_var("REBUILD", format!("{:?}", std::time::Instant::now()));
    println!("cargo::rerun-if-env-changed=REBUILD");
    println!("cargo::warning=ran build.rs");

    // Specify linker arguments.

    // `--nmagic` is required if memory section addresses are not aligned to 0x10000,
    // for example the FLASH and RAM sections in your `memory.x`.
    // See https://github.com/rust-embedded/cortex-m-quickstart/pull/95
    println!("cargo::rustc-link-arg=--nmagic");

    // Set the linker script to the one provided by cortex-m-rt.
    println!("cargo::rustc-link-arg=-Tlink.x");

    // Just to get rust-analyzer to be kinda useful, generate some all zero
    // constants if /global.secrets doesn't exist.

    let secrets_path = Path::new("/global.secrets");
    if !secrets_path.exists() {
        println!("cargo::warning=secrets file does not exist, writing mock secrets.");
        fs::write(
            out.join("gen_constants.rs"),
            "const DECODER_KEY: Chacha20Key = [0; 32];\npub(crate) const CHANNEL_0_KEY: Chacha20Key = [0; 32];\nconst VERIFYING_KEY_COMPRESSED: Ed25519PubKey = [0; 32];",
        )
        .expect("Failed to write constants");

        return;
    }

    // Import secrets
    let secrets_file = File::open(secrets_path).expect("couldn't open secrets");
    let secrets: Secrets = serde_json::from_reader(secrets_file).expect("couldn't parse secrets");

    // ChaCha20 (symmetric/encrypting) secrets
    let deployment_key =
        hex::decode(secrets.deployment_key).expect("couldn't unhex deployment_key");
    let salt = hex::decode(secrets.salt).expect("couldn't unhex salt");
    let decoder_id = var("DECODER_ID").expect("DECODER_ID env var was not present");
    let info = hex::decode(&decoder_id[2..]).expect("couldn't unhex the decoder id");
    let channel_0_key = hex::decode(secrets.channel_0_key).expect("couldn't unhex channel_0_key");

    // Derive the decoder key
    let hk: Hkdf<_, _> = Hkdf::<Sha256>::new(Some(&salt[..]), &deployment_key);
    let mut decoder_key: [u8; 32] = [0; 32];

    hk.expand(&info, &mut decoder_key)
        .expect("32 is a valid length for SHA256");

    // Ed25519 (asymmetric/signing) secrets
    // Note for the reader:
    // sk -> secret key (this does not go to the decoder)
    // vk -> verifying key (this goes to the decoder)
    let signing_sk_bytes = hex::decode(secrets.signing_sk).expect("couldn't unhex signing_sk");
    let signing_sk = SigningKey::from_bytes(
        &signing_sk_bytes
            .try_into()
            .expect("signing_sk wasn't the right length"),
    );
    let signing_vk = signing_sk.verifying_key();

    assert!(!signing_vk.is_weak(), "How is our signing key weak?");

    let signing_vk_bytes = signing_vk.as_bytes();

    fs::write(
        out.join("gen_constants.rs"),
        format!(
            "const DECODER_KEY: Chacha20Key = {:#?};\npub(crate) const CHANNEL_0_KEY: Chacha20Key = {:#?};\nconst VERIFYING_KEY_COMPRESSED: Ed25519PubKey = {:#?};",
            decoder_key, channel_0_key, signing_vk_bytes
        ),
    )
    .expect("Failed to write constants");
}
