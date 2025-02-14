# Protocol

All subscription updates and frames sent to the device are encrypted using the 
XChaCha20-Poly1305 algorithm. All encrypted messages have the following header, 
prefixed before the encrypted packet payload.

| Field           | Size (in bits) |
| --------------- | -------------- |
<!-- | Message Length  | 32             | -->
| Nonce           | 192            |
| MAC Tag         | 128            |

There is one "decoder" encryption key baked into the decoder. This key is used 
for encryption for all Update Subscription messages sent to the decoder.

On receiving a message that is unable to be decrypted successfully, the decoder
will pause until the total time since the command was sent hits 5 seconds.

## Update Subscription

The entire Update Subscription message is encrypted, the encrypted
payload is structured as follows:

| Field           | Size (in bits) |
| --------------- | -------------- |
| Channel ID      | 32             |
| Start Timestamp | 64             |
| End Timestamp   | 64             |
| Channel Key     | 256            |

The decoder will respond with an empty body on successfully registering a
subscription.


## Decode Frame

The encrypted payload for the Decode Frame packet is prefixed with the 32-bit
channel ID. This will be used to determine which channel key will be used to 
decrypt the rest of the message.

The Decoder will respond with the decrypted frame.

\newpage
