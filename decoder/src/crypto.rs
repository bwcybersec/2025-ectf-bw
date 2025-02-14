include!(concat!(env!("OUT_DIR"), "/const_gen.rs"));

const CHACHA20_KEY_BYTES: usize = 32;
pub type Chacha20Key = [u8; CHACHA20_KEY_BYTES];
