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
use std::path::PathBuf;

use hkdf::Hkdf;
use serde::Deserialize;
use sha2::Sha256;

#[derive(Deserialize)]
struct Secrets {
    deployment_key: String,
    salt: String,
    channel_0_key: String,
}

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    // println!("cargo:rerun-if-changed=memory.x");
    // BW - Rerun every time, thinking about this is too hard

    // Specify linker arguments.

    // `--nmagic` is required if memory section addresses are not aligned to 0x10000,
    // for example the FLASH and RAM sections in your `memory.x`.
    // See https://github.com/rust-embedded/cortex-m-quickstart/pull/95
    println!("cargo:rustc-link-arg=--nmagic");

    // Set the linker script to the one provided by cortex-m-rt.
    println!("cargo:rustc-link-arg=-Tlink.x");

    // Import secrets
    let secrets_file = File::open("/secrets/secrets.json").expect("couldn't read secrets");
    let secrets: Secrets = serde_json::from_reader(secrets_file).expect("couldn't parse secrets");
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

    fs::write(
        out.join("gen_constants.rs"),
        format!(
            "const DECODER_KEY: [u8; 32] = {:#?};\nconst CHANNEL_0_KEY: [u8; 32] = {:#?};",
            decoder_key, channel_0_key
        ),
    )
    .expect("Failed to write constants");
}
