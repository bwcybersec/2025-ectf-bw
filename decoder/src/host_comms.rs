use core::fmt::Display;

use alloc::format;
use hal::{pac::Uart0, uart::BuiltUartPeripheral};

use crate::{
    decoder::{Decoder, Subscription},
    flash::DecoderStorageWriteError,
};

#[derive(PartialEq, Eq)]
pub enum DecoderMessageType {
    List,
    Subscribe,
    Decode,
}

#[derive(Debug)]
pub enum DecoderError {
    /// Decoder expected an ACK in the protocol, but got something else.
    ExpectedAckButGot(u8),
    /// Decoder has run out of subscription space.
    NoMoreSubscriptionSpace,
    /// Decoder was sent a frame that claims to be more than 64 bytes
    FrameTooLarge(u16),
    /// Decoder does not have a valid subscription for the given frame.
    NoSubscription(u32),
    /// Given timestamp does fall within the subscription time window.
    SubscriptionTimeMismatch(u32, u64),
    /// Serialization failed while trying to write subscription update to flash.
    SerializationFailed(postcard::Error),
    /// Saving the serialized data to flash failed
    SavingFailed(DecoderStorageWriteError),
}

impl Display for DecoderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ExpectedAckButGot(byte) => {
                write!(f, "Expected ACK but got unexpected byte {}", byte)
            }
            Self::NoMoreSubscriptionSpace => {
                write!(f, "Attempted to add a subscription, but subscription space is full")
            }
            Self::FrameTooLarge(frame_size) => write!(f,
                "Was asked to decode a frame of {}, which is larger than 64",
                frame_size
            ),
            Self::NoSubscription(channel_id) => write!(f,
                "Was asked to decode a frame for channel {}, but we have no subscription for that channel",
                 channel_id
            ),
            Self::SubscriptionTimeMismatch(channel_id, timestamp) => write!(f,
                "Was asked to decode a frame for channel {} with timestamp {}, but that timestamp is invalid for our subscription.", channel_id, timestamp
            ),
            Self::SerializationFailed(err) => write!(f, "Attempted to serialize subscription updates for flash, and failed with error {err}"),
            Self::SavingFailed(err) => write!(f, "Attempted to save subscription updates to flash, and failed with error {err:?}")
         }
    }
}

impl DecoderError {
    pub fn write_to_console<RX, TX>(&self, console: &DecoderConsole<RX, TX>) {
        let message = format!("{self}");
        // heprintln!("{message}");
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

        Ok(DecoderPacketHeader {
            msg_type,
            size: self.read_u16(),
        })
    }

    // ACK

    /// Reads an ACK off the wire. Returns Ok if an ACK is found, otherwise
    /// Err containing the received byte
    pub fn read_ack(&self) -> Result<(), DecoderError> {
        self.read_until_magic();
        match self.read_byte() {
            b'A' => Ok(()),
            byte => Err(DecoderError::ExpectedAckButGot(byte)),
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
    /// This function takes a subscription off the wire, and returns a
    /// subscription object, ready to be inserted into the subscription list by
    /// the logical Decoder
    pub fn read_subscription(&self) -> Result<Subscription, DecoderError> {
        let mut reader: DecoderPayloadReader<'_, RX, TX> = DecoderPayloadReader::new(&self);

        let _decoder_id = reader.read_u32();
        // TODO: Replace this with a secure implementation
        let channel_id = reader.read_u32();
        let start_time = reader.read_u64();
        let end_time = reader.read_u64();

        Ok(Subscription {
            channel_id,
            start_time,
            end_time,
        })
    }

    /// Decode
    pub fn decode_frame(&self, decoder: &Decoder, packet_length: u16) -> Result<(), DecoderError> {
        let mut reader: DecoderPayloadReader<'_, RX, TX> = DecoderPayloadReader::new(&self);
        // 4 bytes for the channel ID, 8 bytes for the timestamp
        let frame_length = packet_length - 4 - 8;

        if frame_length > 64 {
            return Err(DecoderError::FrameTooLarge(frame_length));
        }

        let channel_id = reader.read_u32();
        let timestamp = reader.read_u64();

        let sub = decoder.get_subscription(channel_id);

        match sub {
            Some(sub) => {
                if timestamp < sub.start_time || sub.end_time > timestamp {
                    return Err(DecoderError::SubscriptionTimeMismatch(
                        channel_id, timestamp,
                    ));
                }

                let mut frame_buf: heapless::Vec<u8, 64> = heapless::Vec::new();
                let frame = &mut frame_buf[0..frame_length as usize];
                reader.read_bytes(frame);
                reader.finish_payload();

                // Write the frame back out
                // Packet header
                self.write_byte(b'%'); // magic byte
                self.write_byte(b'D'); // message type
                self.write_u16(frame_length); // message length

                let mut writer: DecoderPayloadWriter<'_, RX, TX> = DecoderPayloadWriter::new(&self);
                writer.write_bytes(frame)?;
                writer.finish_payload()?;

                Ok(())
            }
            None => Err(DecoderError::NoSubscription(channel_id)),
        }
    }

    // Error
    pub fn print_debug(&self, message: &str) {
        let message = message.as_bytes();
        self.write_byte(b'%'); // magic byte
        self.write_byte(b'G'); // message type
        self.write_u16(message.len() as u16); // message type

        // Debug doesn't need ACK logic
        self.0.write_bytes(message);
    }

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

    fn read_until_magic(&self) {
        loop {
            if self.0.read_byte() == b'%' {
                break;
            }
        }
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

    fn write_u64(&self, val: u64) {
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

    fn write_u16(&mut self, val: u16) -> Result<(), DecoderError> {
        self.write_bytes(&val.to_le_bytes())
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
/// It handles expecting an ACK for every 256 bytes, as well as for the
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

    fn read_u32(&mut self) -> u32 {
        let mut bytes: [u8; 4] = Default::default();
        self.read_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    fn read_u64(&mut self) -> u64 {
        let mut bytes: [u8; 8] = Default::default();
        self.read_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    fn finish_payload(self) {
        self.console.write_ack();
    }
}
