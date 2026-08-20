// TDE on-disk layout (40 bytes = 320 bits):
//   [0..3]   v(1)+i(23): valid flag (bit 0) and tag ID (upper 23 bits), packed as (tagno<<1)|valid
//   [3..35]  t(256):     32-byte null-padded tag name
//   [35..40] o(40):      40-bit byte offset into section 3 for first tag-lookup tuple
pub const SIZE_BYTES: usize = 320 / 8;
pub const MAX_TAG_NAME_LENGTH: usize = 32;

const TAG_NAME_OFFSET: usize = 3;
const TAG_OFFSET_FIELD: usize = 35;

pub struct TagDirectoryEntry {
    tagno: u32,
    tde: [u8; SIZE_BYTES],
}

impl TagDirectoryEntry {
    pub fn from_bytes(tagno: u32, tde: [u8; SIZE_BYTES]) -> Self {
        TagDirectoryEntry { tagno, tde }
    }

    pub fn new(tagno: u32, valid: bool, name: &str, offset: u64) -> Self {
        let mut tde = [0u8; SIZE_BYTES];

        let mut name_bytes = [0u8; MAX_TAG_NAME_LENGTH];
        name_bytes[0..name.len()].copy_from_slice(name.as_bytes());

        // Pack (tagno << 1) | valid into 3 bytes big-endian.
        let packed: u32 = (tagno << 1) | (if valid { 1 } else { 0 });
        tde[0..3].copy_from_slice(&packed.to_be_bytes()[1..4]);
        tde[TAG_NAME_OFFSET..TAG_NAME_OFFSET + MAX_TAG_NAME_LENGTH].copy_from_slice(&name_bytes);
        tde[TAG_OFFSET_FIELD..TAG_OFFSET_FIELD + 5].copy_from_slice(&offset.to_be_bytes()[3..]);
        TagDirectoryEntry { tagno, tde }
    }

    pub fn get_tagno(&self) -> u32 {
        self.tagno
    }

    pub fn is_valid(&self) -> bool {
        // Valid bit is the LSB of the last byte of the packed field.
        self.tde[2] & 1 == 1
    }

    pub fn get_name(&self) -> String {
        String::from_utf8_lossy(&self.tde[TAG_NAME_OFFSET..TAG_NAME_OFFSET + MAX_TAG_NAME_LENGTH])
            .trim_end_matches('\0')
            .to_string()
    }

    pub fn get_offset(&self) -> u64 {
        let mut buf = [0u8; 8];
        buf[3..].copy_from_slice(&self.tde[TAG_OFFSET_FIELD..TAG_OFFSET_FIELD + 5]);
        u64::from_be_bytes(buf)
    }

    pub fn as_bytes(&self) -> [u8; SIZE_BYTES] {
        self.tde
    }
}
