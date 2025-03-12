pub const MIN_SIZE_BYTES: usize = (80 + 16 * 16) / 8;

pub struct TagLookupEntry {
    tagno: u16,
    tle: Vec<u8>,
}

impl TagLookupEntry {
    pub fn from_bytes(tagno: u16, tle: Vec<u8>) -> Self {
        TagLookupEntry { tagno, tle }
    }

    pub fn new(
        tagno: u16,
        valid: bool,
        num_file_slots: u16,
        num_files: u16,
        filenos: Vec<u16>,
        offset: u64,
        offset_valid: bool,
    ) -> Self {
        let mut tle = Vec::with_capacity(MIN_SIZE_BYTES + (num_file_slots as usize) * 2);

        let mut filenos_as_u8: Vec<u8> = Vec::with_capacity(filenos.len() * 2);
        for fileno in filenos {
            filenos_as_u8.extend_from_slice(&fileno.to_be_bytes());
        }

        tle[0..2].copy_from_slice(&(tagno << 1 + (if valid { 1 } else { 0 })).to_be_bytes());
        tle[2..4].copy_from_slice(&num_file_slots.to_be_bytes());
        tle[4..6].copy_from_slice(&(num_files + (if offset_valid { 1 } else { 0 })).to_be_bytes());
        tle[6..11].copy_from_slice(&offset.to_be_bytes()[3..]);
        tle.extend(&filenos_as_u8);

        TagLookupEntry { tagno, tle }
    }

    pub fn tagno(&self) -> u16 {
        self.tagno
    }

    pub fn is_valid(&self) -> bool {
        self.tle[1] & 1 == 1
    }

    pub fn get_num_file_slots(&self) -> u16 {
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.tle[2..4]);
        u16::from_be_bytes(buf)
    }

    pub fn get_num_files(&self) -> u16 {
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.tle[4..6]);

        let nf = u16::from_be_bytes(buf);
        if (nf > self.get_num_file_slots()) {
            return nf - 1;
        }
        return nf;
    }

    pub fn is_offset_valid(&self) -> bool {
        self.get_num_files() > self.get_num_file_slots()
    }

    pub fn get_filenos(&self) -> Vec<u16> {
        let mut filenos = Vec::with_capacity(self.get_num_files() as usize);
        for i in 0..self.get_num_files() as usize {
            let mut buf = [0; 2];
            buf.copy_from_slice(&self.tle[11 + i * 2..13 + i * 2]);
            filenos.push(u16::from_be_bytes(buf));
        }
        filenos
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.tle.clone()
    }
}
