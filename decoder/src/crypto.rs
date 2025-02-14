use chacha20poly1305::{aead::AeadMutInPlace, KeyInit, XChaCha20Poly1305};

pub const CHACHA20_KEY_BYTES: usize = 32;
pub const XCHACHA20_NONCE_BYTES: usize = 24;
pub const XCHACHA20_TAG_BYTES: usize = 16;
pub const ENCODER_CRYPTO_HEADER_LEN: usize = XCHACHA20_NONCE_BYTES + XCHACHA20_TAG_BYTES;
pub type Chacha20Key = [u8; CHACHA20_KEY_BYTES];
pub type XChacha20Nonce = [u8; XCHACHA20_NONCE_BYTES];
pub type XChacha20Tag = [u8; XCHACHA20_TAG_BYTES];

include!(concat!(env!("OUT_DIR"), "/gen_constants.rs"));


/// decrypts an encrypted packet in place given the key, nonce, and tag.
pub fn decrypt_encrypted_packet(key: &Chacha20Key, nonce: &XChacha20Nonce, tag: &XChacha20Tag, body: &mut [u8]) -> Result<(),()> {
    let mut cipher = XChaCha20Poly1305::new(key.into());
    cipher.decrypt_in_place_detached(nonce.into(), &[], body, tag.into()).or(Err(()))
}

/// decrypts an encrypted decoder packet in place given the nonce, and tag.
pub fn decrypt_decoder_encrypted_packet(nonce: &XChacha20Nonce, tag: &XChacha20Tag, body: &mut [u8]) -> Result<(),()> {
    decrypt_encrypted_packet(&DECODER_KEY, nonce, tag, body)
}