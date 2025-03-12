pub const SIZE_BYTES: u8 = 112 / 8;

pub struct FileDirectoryEntry {
    fileno: u16,
    fde: [u8; SIZE_BYTES as usize],
}

impl FileDirectoryEntry {
    pub fn from_bytes(fileno: u16, fde: [u8; SIZE_BYTES as usize]) -> Self {
        FileDirectoryEntry { fileno, fde }
    }

    pub fn new(
        fileno: u16,
        length: u64,
        valid: bool,
        parent: u16,
        filename_hash: u16,
        offset: u64,
    ) -> Self {
        let mut fde = [0; SIZE_BYTES as usize];
        fde[0..5].copy_from_slice(&(length << 1 + (if valid { 1 } else { 0 })).to_be_bytes()[3..]);
        fde[5..7].copy_from_slice(&parent.to_be_bytes());
        fde[7..9].copy_from_slice(&filename_hash.to_be_bytes());
        fde[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
        FileDirectoryEntry { fileno, fde }
    }

    pub fn get_fileno(&self) -> u16 {
        self.fileno
    }

    pub fn get_length(&self) -> u64 {
        let mut buf = [0; 8];
        buf[3..].copy_from_slice(&self.fde[0..5]);
        u64::from_be_bytes(buf) >> 1
    }

    pub fn is_valid(&self) -> bool {
        self.fde[4] & 1 == 1
    }

    pub fn get_parent(&self) -> u16 {
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.fde[5..7]);
        u16::from_be_bytes(buf)
    }

    pub fn get_filename_hash(&self) -> u16 {
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.fde[7..9]);
        u16::from_be_bytes(buf)
    }

    pub fn get_offset(&self) -> u64 {
        let mut buf = [0; 8];
        buf[3..].copy_from_slice(&self.fde[9..14]);
        u64::from_be_bytes(buf)
    }

    pub fn as_bytes(&self) -> [u8; SIZE_BYTES as usize] {
        self.fde.clone()
    }
}
