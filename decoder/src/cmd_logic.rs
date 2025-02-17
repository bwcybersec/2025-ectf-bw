use alloc::format;

use crate::{
    crypto::{CHACHA20_KEY_BYTES, ENCODER_CRYPTO_HEADER_LEN},
    decoder::Decoder,
    host_comms::{DecoderConsole, DecoderError, DecoderMessageType}, led::LED,
};

// 4 for channel number
// 8 for start time
// 8 for end time
// CHACHA20_KEY_BYTES for channel key
// ENCODER_CRYPTO_HEADER_LEN for crypto header
const SUBSCRIPTION_MESSAGE_SIZE: u16 =
    4 + 8 + 8 + (CHACHA20_KEY_BYTES as u16) + (ENCODER_CRYPTO_HEADER_LEN as u16);

pub fn run_command<RX, TX>(
    console: &mut DecoderConsole<RX, TX>,
    decoder: &mut Decoder,
    led: &mut LED
) -> Result<(), DecoderError> {
    match console.read_command_header() {
        Ok(hdr) => {
            match hdr.msg_type {
                DecoderMessageType::List => {
                    led.cyan();

                    // List subscriptions
                    // No body to read, just ACK the header
                    if hdr.size != 0 {
                        // ERROR: List msg packet should not have a payload.
                        let _ = console.print_error(&format!("List message packet should not have a body, but had a body of {} bytes", hdr.size));
                        return Ok(());
                    }

                    console.write_ack();

                    let subscriptions = decoder.get_subscriptions().iter().flatten();
                    console.send_list(subscriptions)?;
                }
                DecoderMessageType::Subscribe => {
                    led.yellow();

                    if hdr.size != SUBSCRIPTION_MESSAGE_SIZE {
                        let _ = console.print_error(&format!(
                            "Subscription message should have a size of {}, was {}",
                            SUBSCRIPTION_MESSAGE_SIZE, hdr.size
                        ));
                        return Ok(());
                    }

                    console.write_ack();

                    let sub = console.read_subscription()?;

                    decoder.register_subscription(sub)?;

                    console.send_empty_payload(b'S')?;
                }
                DecoderMessageType::Decode => {
                    led.magenta();

                    console.write_ack();

                    console.decode_frame(&decoder, hdr.size)?;
                }
            }
        }
        Err(err) => {
            let _ =
                console.print_error(&format!("Expected a L, S, or D command, got byte {}", err));
        }
    };

    Ok(())
}
