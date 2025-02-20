# Storage
Storing the subscription updates to flash is done using the Postcard 
serialization format. Storage uses the last page of available flash. In this
page, the first word of the flash is set to a known value (0x4d696b75) to 
determine whether or not this is the first boot, or if the saving process was 
previously interrupted. The storage can store up to 8 subscriptions, each for a 
unique channel.

If the decoder determines that the saving process was interrupted and the 
storage is corrupted, it will reset and wipe the storage.
\newpage