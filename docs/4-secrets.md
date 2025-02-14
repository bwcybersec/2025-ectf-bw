# Secrets

BW's implementation of the Encoder and Decoder make use of several secrets.

## Keys

- Deployment Key - this is a global key for the deployment. This key will never
be sent out to individual decoders, and is used as a master key for deriving
decoder keys
    - Decoder Key - this key is derived from the master key and the decoder ID,
    this key will be baked into the decoder at build time

- Channel Keys - These are global keys, one created for each channel. These will
be shared with the decoder, but ONLY when encrypted using a decoder key. This
ensures that a pirate subscription cannot be decrypted by a given decoder.
    - Channel 0 key - This is a special case. This key is baked into each
    decoder alongside the Decoder Key, so that decoders can always decode
    emergency communications.
