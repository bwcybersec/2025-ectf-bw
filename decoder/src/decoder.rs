use postcard::{from_bytes, to_extend};
use serde::{Deserialize, Serialize};

use crate::{crypto::Chacha20Key, flash::DecoderStorage, host_comms::DecoderError};

const MAX_SUBSCRIPTION_COUNT: usize = 8;

/// This struct represents the concept of the decoder. It will decode frames
/// that it has a valid subscription for, and can register more subscriptions.
#[derive(Debug)]
pub struct Decoder<'a> {
    subscriptions: [Option<Subscription>; MAX_SUBSCRIPTION_COUNT],
    storage: &'a mut DecoderStorage,
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
                Err(err) => return Err(DecoderError::SerializationFailed(err)),
            };
        }

        self.storage.flush_buffer()?;

        Ok(())
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
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct Subscription {
    pub channel_id: u32,
    pub start_time: u64,
    pub end_time: u64,
    pub channel_key: Chacha20Key,
}
