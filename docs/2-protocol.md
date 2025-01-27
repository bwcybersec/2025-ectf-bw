# Protocol

All messages sent to the device are encrypted using Chacha20-Poly1305 
encryption.

There is one "decoder" encryption key baked into the decoder. This key is used 
for encryption for all Update Subscription messages sent to the decoder. 
This key is also used for decoding frames sent to the emergency channel 0.

On receiving a message that is unable to be decrypted successfully, the decoder
will pause until the total time since the command was sent hits 5 seconds.


## Update Subscription


The entire Update Subscription packet payload is encrypted, the encrypted
payload is structured as follows:

| Field           | Size (in bits) |
| --------------- | -------------- |
| Channel ID      | 32             |
| Start Timestamp | 64             |
| End Timestamp   | 64             |
| Channel Key     | 256            |

The decoder will respond with an empty body on successfully registering a
subscription.

## List Subscriptions

The List Subscriptions command has no body. The Decoder will respond with all
subscriptions, excluding the channel keys.

## Decode Frame

The encrypted payload for the Decode Frame packet is prefixed with the 32-bit
channel ID. This will be used to determine which channel key (or the decoder 
key, in the case of channel 0) will be used to decrypt the rest of the message.

The Decoder will respond with the decrypted frame.

\newpage