use hal::flc::{FlashError, Flc};

use crate::host_comms::DecoderError;

use core::fmt::Debug;

pub const STORAGE_MAX: usize = 1024;
pub const STORAGE_MAX_U32: u32 = STORAGE_MAX as u32;

const PERSIST_BASE_ADDR: u32 = 0x10044000;
const DATA_LEN_ADDR: u32 = PERSIST_BASE_ADDR + 4;
const DATA_BASE_ADDR: u32 = DATA_LEN_ADDR + 4;

const FLASH_INITIALIZED_MAGIC: u32 = 0x4d696b75;

#[derive(Debug)]
pub enum DecoderStorageReadError {
    /// The length value in flash is invalid,
    FlashLengthTooLarge,
    /// Got an error from the flash library.
    /// This is probably a logic bug.
    FlashError,
}

impl From<FlashError> for DecoderStorageReadError {
    fn from(_: FlashError) -> Self {
        Self::FlashError
    }
}

#[derive(Debug)]
pub enum DecoderStorageWriteError {
    /// Got an error from the flash library.
    /// This is probably a logic bug.
    FlashError,
}

impl From<FlashError> for DecoderStorageWriteError {
    fn from(_: FlashError) -> Self {
        Self::FlashError
    }
}

impl From<DecoderStorageWriteError> for DecoderError {
    fn from(_: DecoderStorageWriteError) -> Self {
        Self::SavingFailed
    }
}
pub struct DecoderStorage {
    flc: Flc,
    buf: heapless::Vec<u8, STORAGE_MAX>,
}

/// When debugging, we don't want the entire formatted 1024 byte buffer to be
/// sent over the (probably slow/memory constrained) protocol that we're using.
impl Debug for DecoderStorage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DecoderStorage").finish_non_exhaustive()
    }
}

impl DecoderStorage {
    pub fn init(flc: Flc) -> Result<DecoderStorage, DecoderStorageReadError> {
        let mut storage = Self {
            flc,
            buf: heapless::Vec::new(),
        };

        let read_magic = match storage.flc.read_32(PERSIST_BASE_ADDR) {
            Ok(x) => x,
            Err(_) => return Err(DecoderStorageReadError::FlashError),
        };

        if read_magic != FLASH_INITIALIZED_MAGIC {
            // unwrap is okay here, we know the address is fine.
            // heprintln!("Storage is not initialized, resetting.");
            storage.reset_storage().unwrap();
        } else {
            // heprintln!("Storage is initialized, reading into buffer.");
            storage.fill_buffer()?;
        }

        Ok(storage)
    }

    /// Reset the flash so that next time that we read state in, we get an empty
    /// buffer.
    pub fn reset_storage(&mut self) -> Result<(), DecoderStorageWriteError> {
        self.erase_page();
        self.flc.write_128(
            PERSIST_BASE_ADDR,
            &[FLASH_INITIALIZED_MAGIC, 0, 0xFFFFFFFF, 0xFFFFFFFF],
        )?;
        self.buf.clear();
        Ok(())
    }

    /// Fill the buffer in RAM using the contents of the flash.
    pub fn fill_buffer(&mut self) -> Result<(), DecoderStorageReadError> {
        let length = self.flc.read_32(DATA_LEN_ADDR).unwrap();
        if length > STORAGE_MAX_U32 {
            return Err(DecoderStorageReadError::FlashLengthTooLarge);
        }

        // heprintln!("clearing buffer");
        self.buf.clear();

        let mut cursor = DATA_BASE_ADDR;
        // dbg!(cursor);
        loop {
            let bytes_left = (length - (cursor - DATA_BASE_ADDR)) as usize;
            // dbg!(bytes_left);
            if bytes_left >= 4 {
                let read = self
                    .flc
                    .read_32(cursor)
                    .expect("STORAGE_MAX is less than the page size");
                self.buf.extend(read.to_ne_bytes());
                cursor += 4;
            } else if bytes_left == 0 {
                break; // This skips a flash read.
            } else {
                let read = self
                    .flc
                    .read_32(cursor)
                    .expect("STORAGE_MAX is less than the page size");
                let read_bytes = &read.to_ne_bytes()[0..bytes_left];
                match self.buf.extend_from_slice(read_bytes) {
                    Ok(_) => {}
                    Err(_) => {}
                };
                break;
            }
        }

        Ok(())
    }

    /// Write the buffer out to flash, in the expected format.
    pub fn flush_buffer(&self) -> Result<(), DecoderStorageWriteError> {
        self.erase_page();

        let mut cursor = PERSIST_BASE_ADDR;
        // Don't write the initialized magic here. This avoids a race condition
        // where the power could be pulled mid-write, which hypothetically could
        // lead to a channel key being set to all FF.
        let mut u32s_to_write = [0xFFFFFFFF, self.buf.len() as u32, 0xDEADBEEF, 0xDEADBEEF];

        let mut i: usize = 2;

        let chunks = self.buf.array_chunks::<4>();
        let remainder = chunks.remainder();
        for chunk in chunks {
            u32s_to_write[i] = u32::from_ne_bytes(*chunk);
            i += 1;

            if i == u32s_to_write.len() {
                self.flc.write_128(cursor, &u32s_to_write)?;

                // move the cursor by 4 u32s.
                cursor += 4 * 4;
                i = 0;
            }
        }

        let mut final_u32: [u8; 4] = [0x41; 4];

        for (i, b) in final_u32.iter_mut().zip(remainder) {
            *i = *b;
        }

        u32s_to_write[i] = u32::from_ne_bytes(final_u32);
        self.flc.write_128(cursor, &u32s_to_write)?;

        // we finished writing the flash, now write the flash initialized magic :)
        self.flc
            .write_32(PERSIST_BASE_ADDR, FLASH_INITIALIZED_MAGIC)?;
        Ok(())
    }

    fn erase_page(&self) {
        // Safety: this page is reserved in memory.x, and thus cannot be the
        // page that we are running code from.
        unsafe {
            self.flc.erase_page(PERSIST_BASE_ADDR).unwrap();
        }
    }

    pub fn get_buf_mut(&mut self) -> &mut heapless::Vec<u8, STORAGE_MAX> {
        &mut self.buf
    }
}
