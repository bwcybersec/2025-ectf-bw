use core::cell::Cell;

use postcard::{from_bytes, to_extend};
use serde::{Deserialize, Serialize};

use crate::{
    crypto::{
        decrypt_encrypted_packet, Chacha20Key, Ed25519Signature, XChacha20Nonce, XChacha20Tag,
        CHANNEL_0_KEY,
    },
    flash::DecoderStorage,
    host_comms::DecoderError,
};

const MAX_SUBSCRIPTION_COUNT: usize = 8;

/// This struct represents the concept of the decoder. It will decode frames
/// that it has a valid subscription for, and can register more subscriptions.
pub struct Decoder<'a> {
    subscriptions: [Option<Subscription>; MAX_SUBSCRIPTION_COUNT],
    storage: &'a mut DecoderStorage,
    curr_time: Cell<Option<u64>>,
}

impl<'a> Decoder<'a> {
    pub fn new(storage: &'a mut DecoderStorage) -> Self {
        let decoder;

        {
            let buf = storage.get_buf_mut();
            let subscriptions: [Option<Subscription>; MAX_SUBSCRIPTION_COUNT] =
                match from_bytes(buf) {
                    Ok(res) => res,
                    Err(_) => Default::default(),
                };

            decoder = Self {
                subscriptions,
                storage,
                curr_time: Cell::new(None),
            };
        }

        decoder
    }

    pub fn get_subscriptions(&self) -> &[Option<Subscription>] {
        &self.subscriptions
    }

    pub fn register_subscription(&mut self, new_sub: Subscription) -> Result<(), DecoderError> {
        // If the subscription channel is already in the list, replace that.
        if let Some(old_sub) = self
            .subscriptions
            .iter_mut()
            .flat_map(|s| s.as_mut())
            .find(|s| s.channel_id == new_sub.channel_id)
        {
            *old_sub = new_sub;
            self.flush_subscriptions()?;
            return Ok(());
        }

        // Place the subscription into the next free space.
        if let Some(space) = self.subscriptions.iter_mut().find(|s| s.is_none()) {
            *space = Some(new_sub);
            self.flush_subscriptions()?;
            return Ok(());
        }

        Err(DecoderError::NoMoreSubscriptionSpace)
    }

    /// Get the subscription for a given channel_id, if there is any.
    pub fn get_subscription(&self, channel_id: u32) -> Option<&Subscription> {
        self.subscriptions
            .iter()
            .flatten()
            .filter(|s| s.channel_id == channel_id)
            .next()
    }

    fn flush_subscriptions(&mut self) -> Result<(), DecoderError> {
        let buf = self.storage.get_buf_mut();
        buf.clear();
        {
            let buf = ExtendableHeaplessVecMut { the_reference: buf };
            match to_extend(&self.subscriptions, buf) {
                Ok(_) => {}
                Err(_) => return Err(DecoderError::SerializationFailed),
            };
        }

        self.storage.flush_buffer()?;

        Ok(())
    }

    /// Decrypts and decodes a frame given the channel id and crypto parameters.
    /// payload will be reused for the frame contents.
    pub fn decode_frame(
        &self,
        channel_id: u32,
        nonce: &XChacha20Nonce,
        tag: &XChacha20Tag,
        signature: &Ed25519Signature,
        payload: &'a mut heapless::Vec<u8, 72>,
    ) -> Result<&'a [u8], DecoderError> {
        let start_time;
        let end_time;
        let channel_key;

        if channel_id == 0 {
            start_time = u64::MIN;
            end_time = u64::MAX;
            channel_key = &CHANNEL_0_KEY
        } else {
            match self.get_subscription(channel_id) {
                Some(sub) => {
                    start_time = sub.start_time;
                    end_time = sub.end_time;
                    channel_key = &sub.channel_key;
                }
                None => return Err(DecoderError::NoSubscription),
            };
        };

        // console.print_debug(&alloc::format!("decode_frame chan {channel_id} {nonce:?} {tag:?} {payload:?}"));
        decrypt_encrypted_packet(channel_key, nonce, tag, signature, payload)
            .or(Err(DecoderError::FailedDecryption))?;

        let timestamp = u64::from_le_bytes(payload[0..8].try_into().expect("8 == 8"));
        if timestamp < start_time || timestamp > end_time {
            return Err(DecoderError::SubscriptionTimeMismatch);
        }

        let curr_time = self.curr_time.get();
        if let Some(curr_time) = curr_time {
            if curr_time > timestamp {
                return Err(DecoderError::FrameOutOfOrder);
            }
        }

        self.curr_time.set(Some(timestamp));

        Ok(&payload[8..])
    }
}

/// This helper type exists solely so that we can have Extend on a &mut to
/// a heapless vec. Sigh.
struct ExtendableHeaplessVecMut<'why, T, const N: usize> {
    the_reference: &'why mut heapless::Vec<T, N>,
}

impl<T, const N: usize> Extend<T> for ExtendableHeaplessVecMut<'_, T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.the_reference.extend(iter)
    }
}

// Not Copy because it's potentially a bit big.
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Subscription {
    pub channel_id: u32,
    pub start_time: u64,
    pub end_time: u64,
    pub channel_key: Chacha20Key,
}
