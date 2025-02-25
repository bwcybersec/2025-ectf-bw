use hal::{pac::Uart0, uart::BuiltUartPeripheral};

use crate::{
    crypto::{
        decrypt_decoder_encrypted_packet, CHACHA20_KEY_BYTES, ED25519_SIGNATURE_BYTES,
        ENCODER_CRYPTO_HEADER_LEN, XCHACHA20_NONCE_BYTES, XCHACHA20_TAG_BYTES,
    },
    decoder::{Decoder, Subscription},
};

/// The types of message that the decoder will receive.
#[derive(PartialEq, Eq)]
pub enum DecoderMessageType {
    List,
    Subscribe,
    Decode,
}

pub enum DecoderError {
    /// Decoder expected an ACK in the protocol, but got something else.
    ExpectedAckButGotOther,
    /// Decoder has run out of subscription space.
    NoMoreSubscriptionSpace,
    /// Decoder was sent a frame that claims to be more than 64 bytes
    FrameTooLarge,
    /// Decoder does not have a valid subscription for the given channel.
    NoSubscription,
    /// Given timestamp does fall within the subscription time window.
    SubscriptionTimeMismatch,
    /// Serialization failed while trying to write subscription update to flash.
    SerializationFailed,
    /// Saving the serialized data to flash failed
    SavingFailed,
    /// Failed to decrypt an encrypted payload.
    FailedDecryption,
    /// Recieved a frame from the past. We refuse to replay it.
    FrameOutOfOrder,
    /// Recieved a packet which should have a consistent size that had a different size
    PacketWrongSize,
    /// Recieved a packet with an invalid command byte.
    InvalidCommand,
}

impl DecoderError {
    /// Get the message to be sent to console when this error is received
    fn message(&self) -> &str {
        match self {
            Self::ExpectedAckButGotOther => "Expected ACK but got unexpected byte",
            Self::NoMoreSubscriptionSpace => "Attempted to add a subscription, but subscription space is full",
            Self::FrameTooLarge => "Was asked to decode a frame which is larger than 64 bytes",
            Self::NoSubscription => "Was asked to decode a frame for channel that we have no subscription for",
            Self::SubscriptionTimeMismatch => "Was asked to decode a frame with timestamp thats invalid for our subscription.",
            Self::SerializationFailed => "Failed to serialize subscription updates for flash",
            Self::SavingFailed=> "Failed to save subscriptions to flash",
            Self::FailedDecryption => "Failed to decrypt a encrypted payload. This can mean that you used a subscription for a different decoder, or that your message was corrupted or tampered with.",
            Self::FrameOutOfOrder => "Was asked to decode a frame with timestamp in the past",
            Self::PacketWrongSize => "Received a packet which has a constant expected size with an invalid size for the packet type",
            Self::InvalidCommand => "Received a command with a type byte that is not L, S, or D",
        }
    }

    /// Write this error to a given console
    pub fn write_to_console<RX, TX>(&self, console: &DecoderConsole<RX, TX>) {
        let message = self.message();
        let _ = console.print_debug(&message);
        let _ = console.print_error(&message);
    }
}

pub struct DecoderPacketHeader {
    pub msg_type: DecoderMessageType,
    pub size: u16,
}

pub struct DecoderConsole<RX, TX>(pub BuiltUartPeripheral<Uart0, RX, TX, (), ()>);

impl<RX, TX> DecoderConsole<RX, TX> {
    /// Returns the packet parsed information from the packet header.
    /// The Err on this Result
    pub fn read_command_header(&self) -> Result<DecoderPacketHeader, u8> {
        // Read until the magic %
        self.read_until_magic();

        // Turn the cmd into a DecoderPacketType, error if we shouldn't see this
        // yet.
        let cmd: u8 = self.read_byte();
        let msg_type = match cmd {
            b'D' => DecoderMessageType::Decode,
            b'S' => DecoderMessageType::Subscribe,
            b'L' => DecoderMessageType::List,
            _ => return Err(cmd),
        };

        let size = self.read_u16();
        self.write_ack();

        Ok(DecoderPacketHeader { msg_type, size })
    }

    // ACK

    /// Reads an ACK off the wire. Returns Ok if an ACK is found, otherwise
    /// Err containing the received byte
    pub fn read_ack(&self) -> Result<(), DecoderError> {
        self.read_until_magic();
        match self.read_byte() {
            b'A' => Ok(()),
            _ => Err(DecoderError::ExpectedAckButGotOther),
        }
    }

    pub fn write_ack(&self) {
        self.write_byte(b'%'); // magic byte
        self.write_byte(b'A'); // message type
        self.write_u16(0); // message length
    }

    // List

    /// This function takes a Iterator of subscriptions, and sends out the list
    /// response packet for them over UART
    pub fn send_list<'a, I>(&self, subscriptions: I) -> Result<(), DecoderError>
    where
        I: Iterator<Item = &'a Subscription> + Clone,
    {
        let sub_count = subscriptions.clone().count();
        let payload_len = (sub_count * (4 + 8 + 8)) as u16;

        self.write_byte(b'%'); // magic byte
        self.write_byte(b'L'); // message type
        self.write_u16(payload_len + 4); // message type

        self.read_ack()?;

        self.write_u32(sub_count as u32);

        let mut payload = DecoderPayloadWriter::new(&self);

        for sub in subscriptions {
            payload.write_u32(sub.channel_id)?;
            payload.write_u64(sub.start_time)?;
            payload.write_u64(sub.end_time)?;
        }

        payload.finish_payload()?;

        Ok(())
    }

    // Subscription
    /// Takes a subscription off the wire, and returns a subscription object,
    /// ready to be inserted into the subscription list by the Decoder
    pub fn read_subscription(&self) -> Result<Subscription, DecoderError> {
        const SUBSCRIPTION_SIZE: usize = 4 + 8 + 8 + CHACHA20_KEY_BYTES;

        let mut reader: DecoderPayloadReader<'_, RX, TX> = DecoderPayloadReader::new(&self);

        let mut nonce: [u8; XCHACHA20_NONCE_BYTES] = Default::default();
        let mut tag: [u8; XCHACHA20_TAG_BYTES] = Default::default();
        let mut signature: [u8; ED25519_SIGNATURE_BYTES] = [0; ED25519_SIGNATURE_BYTES];
        let mut body: [u8; SUBSCRIPTION_SIZE] = [0; SUBSCRIPTION_SIZE];

        reader.read_bytes(&mut nonce);
        reader.read_bytes(&mut tag);
        reader.read_bytes(&mut signature);
        reader.read_bytes(&mut body);
        reader.finish_payload();

        if let Err(_) = decrypt_decoder_encrypted_packet(&nonce, &tag, &signature, &mut body) {
            return Err(DecoderError::FailedDecryption);
        };

        let channel_id = u32::from_le_bytes(body[0..4].try_into().expect("4 == 4"));
        let start_time = u64::from_le_bytes(body[4..12].try_into().expect("8 == 8"));
        let end_time = u64::from_le_bytes(body[12..20].try_into().expect("8 == 8"));
        let channel_key: [u8; CHACHA20_KEY_BYTES] = body[20..]
            .try_into()
            .expect("subscription must be 4+8+8+CHACHA20_KEY_BYTES in length");

        Ok(Subscription {
            channel_id,
            start_time,
            end_time,
            channel_key,
        })
    }

    // Decode
    /// Reads a Decode Frame packet off the wire, extracting the fields for the
    /// crypto header, decrypts it, then writes the resulting frame back out.
    pub fn decode_frame(&self, decoder: &Decoder, packet_length: u16) -> Result<(), DecoderError> {
        let mut reader: DecoderPayloadReader<'_, RX, TX> = DecoderPayloadReader::new(&self);
        // 4 bytes for the channel ID, 8 bytes for the timestamp, a crypto header
        let frame_length = packet_length - 4 - 8 - (ENCODER_CRYPTO_HEADER_LEN) as u16;

        // The payload contains the timestamp as well as the frame
        let payload_length = frame_length + 8;

        if frame_length > 64 {
            return Err(DecoderError::FrameTooLarge);
        }

        let channel_id = reader.read_u32();
        let mut nonce: [u8; XCHACHA20_NONCE_BYTES] = Default::default();
        let mut tag: [u8; XCHACHA20_TAG_BYTES] = Default::default();
        let mut signature: [u8; ED25519_SIGNATURE_BYTES] = [0; ED25519_SIGNATURE_BYTES];

        reader.read_bytes(&mut nonce);
        reader.read_bytes(&mut tag);
        reader.read_bytes(&mut signature);

        // 72 because the frame could be 64, and the timestamp takes 8
        let mut payload: heapless::Vec<u8, 72> = heapless::Vec::new();
        reader.extend_with_n_bytes(&mut payload, payload_length as usize);
        reader.finish_payload();

        let frame = decoder.decode_frame(channel_id, &nonce, &tag, &signature, &mut payload)?;

        // Write out the frame.
        self.write_byte(b'%'); // magic byte
        self.write_byte(b'D'); // message type
        self.write_u16(frame_length); // message length

        self.read_ack()?;

        let mut writer: DecoderPayloadWriter<'_, RX, TX> = DecoderPayloadWriter::new(&self);
        writer.write_bytes(&frame)?;
        writer.finish_payload()?;

        Ok(())
    }

    // Debug
    /// Sends a message to the host tools using the debug message type
    pub fn print_debug(&self, message: &str) {
        let message = message.as_bytes();
        self.write_byte(b'%'); // magic byte
        self.write_byte(b'G'); // message type
        self.write_u16(message.len() as u16); // message type

        // Debug doesn't need ACK logic
        self.0.write_bytes(message);
    }

    // Error
    /// Sends an error message to the host tools.
    ///
    /// THIS CLOSES THE HOST TOOL.
    pub fn print_error(&self, message: &str) -> Result<(), DecoderError> {
        let message = message.as_bytes();
        self.write_byte(b'%'); // magic byte
        self.write_byte(b'E'); // message type
        self.write_u16(message.len() as u16); // message type

        self.read_ack()?;

        let mut payload = DecoderPayloadWriter::new(&self);
        payload.write_bytes(message)?;
        payload.finish_payload()?;

        Ok(())
    }

    /// Send an empty payload with a particular type to the host tools.
    pub fn send_empty_payload(&self, msg_type: u8) -> Result<(), DecoderError> {
        self.write_byte(b'%');
        self.write_byte(msg_type);
        self.write_u16(0);
        self.read_ack()
    }

    // internal helpers

    // reads
    fn read_byte(&self) -> u8 {
        self.0.read_byte()
    }

    fn read_u16(&self) -> u16 {
        let mut u16_bytes: [u8; 2] = [0, 0];
        self.0.read_bytes(&mut u16_bytes);
        u16::from_le_bytes(u16_bytes)
    }

    /// Waits until UART receives the magic % byte, consuming bytes as it goes.
    fn read_until_magic(&self) {
        while self.0.read_byte() != b'%' {}
    }

    // writes
    fn write_byte(&self, val: u8) {
        self.0.write_byte(val)
    }

    fn write_u16(&self, val: u16) {
        self.0.write_bytes(&val.to_le_bytes())
    }

    fn write_u32(&self, val: u32) {
        self.0.write_bytes(&val.to_le_bytes())
    }
}

/// This struct represents a payload being written to the wire.
/// It handles expecting an ACK for every 256 bytes, as well as for the
/// last block.
struct DecoderPayloadWriter<'a, RX, TX> {
    bytes_written: usize,
    console: &'a DecoderConsole<RX, TX>,
}

impl<'a, RX, TX> DecoderPayloadWriter<'a, RX, TX> {
    fn new(console: &'a DecoderConsole<RX, TX>) -> Self {
        Self {
            bytes_written: 0,
            console,
        }
    }

    fn write_byte(&mut self, byte: u8) -> Result<(), DecoderError> {
        self.console.write_byte(byte);
        if self.bytes_written % 256 == 0 && self.bytes_written != 0 {
            self.console.read_ack()?;
        }

        self.bytes_written += 1;
        Ok(())
    }
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), DecoderError> {
        for byte in bytes {
            self.write_byte(*byte)?
        }
        Ok(())
    }

    fn write_u32(&mut self, val: u32) -> Result<(), DecoderError> {
        self.write_bytes(&val.to_le_bytes())
    }

    fn write_u64(&mut self, val: u64) -> Result<(), DecoderError> {
        self.write_bytes(&val.to_le_bytes())
    }

    fn finish_payload(self) -> Result<(), DecoderError> {
        self.console.read_ack()
    }
}

/// This struct represents a payload being read from the wire.
/// It handles writing an ACK for every 256 bytes, as well as for the
/// last block.
struct DecoderPayloadReader<'a, RX, TX> {
    bytes_read: usize,
    console: &'a DecoderConsole<RX, TX>,
}

impl<'a, RX, TX> DecoderPayloadReader<'a, RX, TX> {
    fn new(console: &'a DecoderConsole<RX, TX>) -> Self {
        Self {
            bytes_read: 0,
            console,
        }
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.console.read_byte();
        if self.bytes_read % 256 == 0 && self.bytes_read != 0 {
            self.console.write_ack();
        }
        self.bytes_read += 1;
        byte
    }

    fn read_bytes(&mut self, bytes: &mut [u8]) {
        for i in 0..bytes.len() {
            bytes[i] = self.read_byte()
        }
    }

    fn extend_with_n_bytes(&mut self, buf: &mut impl Extend<u8>, count: usize) {
        buf.extend((0..count).map(|_| self.read_byte()));
    }

    fn read_u32(&mut self) -> u32 {
        let mut bytes: [u8; 4] = Default::default();
        self.read_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    fn finish_payload(self) {
        self.console.write_ack();
    }
}
