use core::mem;

use crate::host_comms::DecoderError;

const MAX_SUBSCRIPTION_COUNT: usize = 8;

/// This struct represents the concept of the decoder. It will decode frames
/// that it has a valid subscription for, and can register more subscriptions.
pub struct Decoder {
    subscriptions: [Option<Subscription>; MAX_SUBSCRIPTION_COUNT],
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            subscriptions: [None, None, None, None, None, None, None, None],
        }
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
            return Ok(());
        }

        // Place the subscription into the next free space.
        if let Some(space) = self.subscriptions.iter_mut().find(|s| s.is_none()) {
            *space = Some(new_sub);
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
}

// Not Copy because it's potentially a bit big.
#[derive(Clone)]
pub struct Subscription {
    pub channel_id: u32,
    pub start_time: u64,
    pub end_time: u64,
}
