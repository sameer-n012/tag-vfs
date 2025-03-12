pub const SIZE_BYTES: u8 = 40 / 8;

pub struct FileEndMetadata {
    fem: [u8; SIZE_BYTES as usize],
}

impl FileEndMetadata {
    pub fn from_bytes(fem: [u8; SIZE_BYTES as usize]) -> Self {
        FileEndMetadata { fem }
    }

    pub fn new(length: u64) -> Self {
        let mut fem = [0; SIZE_BYTES as usize];
        fem[0..5].copy_from_slice(&(length).to_be_bytes()[3..]);
        FileEndMetadata { fem }
    }

    pub fn get_length(&self) -> u64 {
        let mut length_bytes = [0; 8];
        length_bytes[3..].copy_from_slice(&self.fem[0..5]);
        u64::from_be_bytes(length_bytes)
    }

    pub fn as_bytes(&self) -> [u8; SIZE_BYTES as usize] {
        self.fem.clone()
    }
}
