// FDE on-disk layout (16 bytes = 128 bits):
//   [0..5]  l(39)+v(1): file length (upper 39 bits) and valid flag (bit 0)
//   [5..9]  p(32):      parent file-directory index
//   [9..11] n(16):      16-bit hash of filename
//   [11..16] o(40):     40-bit byte offset into section 4
pub const SIZE_BYTES: usize = 128 / 8;

pub struct FileDirectoryEntry {
    fileno: u32,
    fde: [u8; SIZE_BYTES],
}

impl FileDirectoryEntry {
    pub fn from_bytes(fileno: u32, fde: [u8; SIZE_BYTES]) -> Self {
        FileDirectoryEntry { fileno, fde }
    }

    pub fn new(
        fileno: u32,
        length: u64,
        valid: bool,
        parent: u32,
        filename_hash: u16,
        offset: u64,
    ) -> Self {
        let mut fde = [0u8; SIZE_BYTES];
        fde[0..5]
            .copy_from_slice(&((length << 1) + (if valid { 1 } else { 0 })).to_be_bytes()[3..]);
        fde[5..9].copy_from_slice(&parent.to_be_bytes());
        fde[9..11].copy_from_slice(&filename_hash.to_be_bytes());
        fde[11..16].copy_from_slice(&offset.to_be_bytes()[3..]);
        FileDirectoryEntry { fileno, fde }
    }

    pub fn get_fileno(&self) -> u32 {
        self.fileno
    }

    pub fn get_length(&self) -> u64 {
        let mut buf = [0u8; 8];
        buf[3..].copy_from_slice(&self.fde[0..5]);
        u64::from_be_bytes(buf) >> 1
    }

    pub fn is_valid(&self) -> bool {
        self.fde[4] & 1 == 1
    }

    pub fn get_parent(&self) -> u32 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.fde[5..9]);
        u32::from_be_bytes(buf)
    }

    pub fn get_filename_hash(&self) -> u16 {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(&self.fde[9..11]);
        u16::from_be_bytes(buf)
    }

    pub fn get_offset(&self) -> u64 {
        let mut buf = [0u8; 8];
        buf[3..].copy_from_slice(&self.fde[11..16]);
        u64::from_be_bytes(buf)
    }

    pub fn as_bytes(&self) -> [u8; SIZE_BYTES] {
        self.fde
    }
}
