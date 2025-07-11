use chacha20poly1305::{aead::AeadMutInPlace, KeyInit, XChaCha20Poly1305};
use ed25519_dalek::{Signature, VerifyingKey};
use hal::trng::Trng;
use once_cell::sync::OnceCell;
use rand::RngCore;

// Encryption
pub const CHACHA20_KEY_BYTES: usize = 32;
pub const XCHACHA20_NONCE_BYTES: usize = 24;
pub const XCHACHA20_TAG_BYTES: usize = 16;
pub type Chacha20Key = [u8; CHACHA20_KEY_BYTES];
pub type XChacha20Nonce = [u8; XCHACHA20_NONCE_BYTES];
pub type XChacha20Tag = [u8; XCHACHA20_TAG_BYTES];

// Signing
pub const ED25519_SIGNATURE_BYTES: usize = ed25519_dalek::SIGNATURE_LENGTH;
pub type Ed25519PubKey = [u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
pub type Ed25519Signature = [u8; ED25519_SIGNATURE_BYTES];

// Crypto Header
pub const ENCODER_CRYPTO_HEADER_LEN: usize =
    XCHACHA20_NONCE_BYTES + XCHACHA20_TAG_BYTES + ED25519_SIGNATURE_BYTES;

include!(concat!(env!("OUT_DIR"), "/gen_constants.rs"));

// Initializing the VerifyingKey object from a compressed byte array is
// non-trivial, so I'd like to avoid doing it on every frame.
fn get_verifying_key() -> &'static VerifyingKey {
    static VERIFYING_KEY: OnceCell<VerifyingKey> = OnceCell::new();

    VERIFYING_KEY.get_or_init(|| {
        VerifyingKey::from_bytes(&VERIFYING_KEY_COMPRESSED)
            .expect("VERIFYING_KEY_COMPRESSED is always a valid Ed25519 public key")
    })
}

/// Allows main to bootstrap the OnceCell in crypto without needing to let the
/// implementation details of it leaking.
pub fn bootstrap_crypto() {
    let _ = get_verifying_key();
}

/// Decrypts an encrypted packet in place given the key, nonce, and tag.
pub fn decrypt_encrypted_packet(
    key: &Chacha20Key,
    nonce: &XChacha20Nonce,
    tag: &XChacha20Tag,
    signature: &Ed25519Signature,
    body: &mut [u8],
) -> Result<(), ()> {
    let mut cipher = XChaCha20Poly1305::new(key.into());
    if cipher
        .decrypt_in_place_detached(nonce.into(), &[], body, tag.into())
        .is_err()
    {
        // Failed to decrypt
        return Err(());
    }

    get_verifying_key()
        .verify_strict(body, &Signature::from_bytes(signature))
        .or(Err(()))
}

/// Decrypts an encrypted decoder packet in place given the nonce, and tag.
pub fn decrypt_decoder_encrypted_packet(
    nonce: &XChacha20Nonce,
    tag: &XChacha20Tag,
    signature: &Ed25519Signature,
    body: &mut [u8],
) -> Result<(), ()> {
    decrypt_encrypted_packet(&DECODER_KEY, nonce, tag, signature, body)
}

/// Encrypts the flash buffer.
///
/// Returns a tuple of the nonce and the tag
pub fn encrypt_flash_buffer(
    buffer: &mut [u8],
    trng: &mut Trng,
) -> Result<(XChacha20Nonce, XChacha20Tag), ()> {
    let mut cipher = XChaCha20Poly1305::new((&FLASH_KEY).into());
    let mut nonce: XChacha20Nonce = Default::default();
    trng.fill_bytes(&mut nonce);

    match cipher.encrypt_in_place_detached(&nonce.into(), &[], buffer) {
        Ok(tag) => Ok((nonce, tag.into())),
        Err(_) => Err(()),
    }
}

// Decrypts the flash buffer
pub fn decrypt_flash_buffer(
    buffer: &mut [u8],
    nonce: &XChacha20Nonce,
    tag: &XChacha20Tag,
) -> Result<(), ()> {
    let mut cipher = XChaCha20Poly1305::new((&FLASH_KEY).into());

    cipher
        .decrypt_in_place_detached(nonce.into(), &[], buffer, tag.into())
        .or(Err(()))
}
