use alloc::format;

use crate::{
    decoder::Decoder,
    host_comms::{DecoderConsole, DecoderError, DecoderMessageType},
};

const SUBSCRIPTION_MESSAGE_SIZE: u16 = 24;

pub fn run_command<RX, TX>(
    console: &mut DecoderConsole<RX, TX>,
    decoder: &mut Decoder,
) -> Result<(), DecoderError> {
    match console.read_command_header() {
        Ok(hdr) => {
            match hdr.msg_type {
                DecoderMessageType::List => {
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
                }
                DecoderMessageType::Decode => {
                    // This logic is done inside of DecoderConsole
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
