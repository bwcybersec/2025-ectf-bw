const CHACHA20_KEY_BYTES: usize = 32;
pub type Chacha20Key = [u8; CHACHA20_KEY_BYTES];

include!(concat!(env!("OUT_DIR"), "/gen_constants.rs"));