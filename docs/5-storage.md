# Storage
Storing the subscription updates to flash is done using the Postcard 
serialization format. Storage uses the last page of available flash. In this
page, the first word of the flash is set to 0x4D494B55 to determine whether or 
not this is the first boot. The storage can store up to 8 subscriptions, each
for a unique channel.
\newpage