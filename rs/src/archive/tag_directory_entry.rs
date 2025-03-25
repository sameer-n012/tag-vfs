pub const SIZE_BYTES: u8 = 184 / 8;
pub const MAX_TAG_NAME_LENGTH: usize = 16;

pub struct TagDirectoryEntry {
    tagno: u16,
    tde: [u8; SIZE_BYTES as usize],
}

impl TagDirectoryEntry {
    pub fn from_bytes(tagno: u16, tde: [u8; SIZE_BYTES as usize]) -> Self {
        TagDirectoryEntry { tagno, tde }
    }

    pub fn new(tagno: u16, valid: bool, name: &str, offset: u64) -> Self {
        let mut tde = [0; SIZE_BYTES as usize];

        let mut name_bytes: [u8; 16] = [0; 16];
        name_bytes[0..name.len()].copy_from_slice(name.as_bytes());

        tde[0..2].copy_from_slice(&(tagno << 1 + (if valid { 1 } else { 0 })).to_be_bytes());
        tde[2..18].copy_from_slice(&name_bytes);
        tde[18..23].copy_from_slice(&offset.to_be_bytes()[3..]);
        TagDirectoryEntry { tagno, tde }
    }

    pub fn get_tagno(&self) -> u16 {
        self.tagno >> 1
    }

    pub fn is_valid(&self) -> bool {
        self.tde[1] & 1 == 1
    }

    pub fn get_name(&self) -> String {
        String::from_utf8_lossy(&self.tde[2..18]).to_string()
    }

    pub fn get_offset(&self) -> u64 {
        let mut buf = [0; 8];
        buf[3..].copy_from_slice(&self.tde[18..23]);
        u64::from_be_bytes(buf)
    }

    pub fn as_bytes(&self) -> [u8; SIZE_BYTES as usize] {
        self.tde.clone()
    }
}
