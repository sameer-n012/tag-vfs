// TLE on-disk layout (base = 12 bytes):
//   [0..3]   v(1)+i(23): valid flag (bit 0) and tag ID (upper 23 bits), packed as (tagno<<1)|valid
//   [3..5]   s(16):      number of file-pointer slots in this tuple
//   [5..7]   n(16):      number of valid file entries (plus 1 if next-pointer is valid)
//   [7..12]  o(40):      40-bit offset from start of section 3 to next TLE for this tag
//   [12..]   fi(32)*s:   file-directory indices, 4 bytes each
//
// First TLE for a tag has MIN_NUM_FILE_SLOTS slots; each additional TLE doubles minus one.
pub const BASE_SIZE_BYTES: usize = 96 / 8;
pub const FILE_SLOT_SIZE: usize = 4; // bytes per file pointer (u32)
pub const MIN_NUM_FILE_SLOTS: usize = 15;
pub const MIN_SIZE_BYTES: usize = BASE_SIZE_BYTES + MIN_NUM_FILE_SLOTS * FILE_SLOT_SIZE;

const TAG_SLOTS_OFFSET: usize = 3;
const NUM_FILES_OFFSET: usize = 5;
const NEXT_OFFSET_FIELD: usize = 7;
const FILE_LIST_OFFSET: usize = 12;

pub struct TagLookupEntry {
    tagno: u32,
    tle: Vec<u8>,
}

impl TagLookupEntry {
    pub fn from_bytes(tle: Vec<u8>) -> Self {
        // Decode 3-byte big-endian packed field: upper 23 bits = tagno, bit 0 = valid.
        let mut buf = [0u8; 4];
        buf[1..4].copy_from_slice(&tle[0..3]);
        let tagno: u32 = u32::from_be_bytes(buf) >> 1;
        TagLookupEntry { tagno, tle }
    }

    pub fn new(
        tagno: u32,
        valid: bool,
        num_file_slots: u16,
        num_files: u16,
        filenos: Vec<u32>,
        offset: u64,
        offset_valid: bool,
    ) -> Self {
        let mut tle = Vec::with_capacity(BASE_SIZE_BYTES + num_file_slots as usize * FILE_SLOT_SIZE);

        let mut filenos_as_u8: Vec<u8> = Vec::with_capacity(filenos.len() * FILE_SLOT_SIZE);
        for fileno in filenos {
            filenos_as_u8.extend_from_slice(&fileno.to_be_bytes());
        }

        let packed: u32 = (tagno << 1) | (if valid { 1 } else { 0 });
        tle.extend_from_slice(&packed.to_be_bytes()[1..4]); // 3 bytes
        tle.extend_from_slice(&num_file_slots.to_be_bytes()); // 2 bytes
        tle.extend_from_slice(
            &(num_files + (if offset_valid { 1 } else { 0 })).to_be_bytes(),
        ); // 2 bytes
        tle.extend_from_slice(&offset.to_be_bytes()[3..]); // 5 bytes
        tle.extend_from_slice(&filenos_as_u8);
        // Pad remaining unused slots with zeros.
        let filled = filenos_as_u8.len();
        let total = num_file_slots as usize * FILE_SLOT_SIZE;
        if filled < total {
            tle.extend(std::iter::repeat(0u8).take(total - filled));
        }

        TagLookupEntry { tagno, tle }
    }

    pub fn tagno(&self) -> u32 {
        self.tagno
    }

    pub fn is_valid(&self) -> bool {
        // Valid bit is the LSB of the last byte of the 3-byte packed field.
        self.tle[2] & 1 == 1
    }

    pub fn get_num_file_slots(&self) -> u16 {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(&self.tle[TAG_SLOTS_OFFSET..TAG_SLOTS_OFFSET + 2]);
        u16::from_be_bytes(buf)
    }

    pub fn get_num_files(&self) -> u16 {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(&self.tle[NUM_FILES_OFFSET..NUM_FILES_OFFSET + 2]);
        let nf = u16::from_be_bytes(buf);
        if nf > self.get_num_file_slots() {
            return nf - 1;
        }
        return nf;
    }

    pub fn is_offset_valid(&self) -> bool {
        let raw = u16::from_be_bytes(self.tle[NUM_FILES_OFFSET..NUM_FILES_OFFSET + 2].try_into().unwrap());
        raw > self.get_num_file_slots()
    }

    pub fn get_next_offset(&self) -> u64 {
        let mut buf = [0u8; 8];
        buf[3..8].copy_from_slice(&self.tle[NEXT_OFFSET_FIELD..NEXT_OFFSET_FIELD + 5]);
        u64::from_be_bytes(buf)
    }

    pub fn get_filenos(&self) -> Vec<u32> {
        let count = self.get_num_files() as usize;
        let mut filenos = Vec::with_capacity(count);
        for i in 0..count {
            let start = FILE_LIST_OFFSET + i * FILE_SLOT_SIZE;
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&self.tle[start..start + FILE_SLOT_SIZE]);
            filenos.push(u32::from_be_bytes(buf));
        }
        filenos
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.tle.clone()
    }

    pub fn size_bytes(&self) -> usize {
        self.tle.len()
    }

    pub fn calculate_needed_size(num_file_slots: u16) -> usize {
        BASE_SIZE_BYTES + num_file_slots as usize * FILE_SLOT_SIZE
    }
}
