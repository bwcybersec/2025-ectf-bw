use crate::{
    crypto::{CHACHA20_KEY_BYTES, ENCODER_CRYPTO_HEADER_LEN},
    decoder::Decoder,
    host_comms::{DecoderConsole, DecoderError, DecoderMessageType},
    led::LED,
    timer::DecoderClock,
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
    led: &mut LED,
    clock: &mut DecoderClock,
) -> Result<(), DecoderError> {
    let hdr = console.read_command_header();
    // We read the header, transaction time starts now.
    clock.start_transaction_timer();
    match hdr {
        Ok(hdr) => {
            match hdr.msg_type {
                DecoderMessageType::List => {
                    led.cyan();

                    // List subscriptions
                    // No body to read, just ACK the header
                    if hdr.size != 0 {
                        // ERROR: List msg packet should not have a payload.
                        return Err(DecoderError::PacketWrongSize);
                    }

                    let subscriptions = decoder.get_subscriptions().iter().flatten();
                    console.send_list(subscriptions)?;
                }
                DecoderMessageType::Subscribe => {
                    led.yellow();

                    if hdr.size != SUBSCRIPTION_MESSAGE_SIZE {
                        // ERROR: Subscriptions should have a consistent size.
                        return Err(DecoderError::PacketWrongSize);
                    }

                    let sub = console.read_subscription()?;

                    decoder.register_subscription(sub)?;

                    console.send_empty_payload(b'S')?;
                }
                DecoderMessageType::Decode => {
                    led.magenta();

                    console.decode_frame(&decoder, hdr.size)?;
                }
            }
        }
        Err(_) => return Err(DecoderError::InvalidCommand),
    };

    Ok(())
}
