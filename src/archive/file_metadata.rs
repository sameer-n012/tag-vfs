// FM on-disk layout (base = 17 bytes):
//   [0..5]   l(39)+v(1): file length (upper 39 bits) and valid flag (bit 0)
//   [5..9]   f(32):      file-directory index (u32)
//   [9..13]  p(32):      parent file-directory index (u32)
//   [13]     y(8):       file type
//   [14]     nn(8):      filename length in bytes
//   [15..17] tn(16):     number of tag IDs that follow
//   [17..]   ti(24)*tn:  tag IDs, 3 bytes each (upper 23 bits used; same encoding as TDE/TLE)
//   [...]    n(nn):      filename bytes
pub const BASE_SIZE_BYTES: usize = 136 / 8;
pub const TAG_SLOT_SIZE: usize = 3; // bytes per tag entry
pub const MIN_NAME_SIZE: usize = 1;
pub const MIN_SIZE_BYTES: usize = BASE_SIZE_BYTES + MIN_NAME_SIZE;

const FILENO_OFFSET: usize = 5;
const PARENT_OFFSET: usize = 9;
const FILE_TYPE_OFFSET: usize = 13;
pub const FILENAME_LEN_OFFSET: usize = 14;
pub const NUM_TAGS_OFFSET: usize = 15;
const TAGS_OFFSET: usize = 17;

pub struct FileMetadata {
    num_tags: u16,
    fm: Vec<u8>,
}

impl FileMetadata {
    pub fn from_bytes(fm: Vec<u8>) -> Self {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(&fm[NUM_TAGS_OFFSET..NUM_TAGS_OFFSET + 2]);
        let num_tags = u16::from_be_bytes(buf);
        FileMetadata { num_tags, fm }
    }

    pub fn new(
        fileno: u32,
        length: u64,
        valid: bool,
        parent: u32,
        file_type: u8,
        filename: &str,
        tags: Vec<u32>,
    ) -> Self {
        let filename_len = filename.len() as u8;
        let tags_len = tags.len() as u16;

        if filename.len() > u8::MAX as usize {
            panic!(
                "Filename length exceeds the maximum allowed value of {}",
                u8::MAX
            );
        }

        let mut tag_vec_u8 = Vec::with_capacity(tags_len as usize * TAG_SLOT_SIZE);
        for tag in &tags {
            // Store each tag ID as 3 bytes (big-endian, dropping the leading zero byte of u32).
            tag_vec_u8.extend_from_slice(&tag.to_be_bytes()[1..4]);
        }

        let mut fm =
            Vec::with_capacity(BASE_SIZE_BYTES + tag_vec_u8.len() + filename_len as usize);
        fm.extend_from_slice(&((length << 1) | (if valid { 1 } else { 0 })).to_be_bytes()[3..]); // 5 bytes
        fm.extend_from_slice(&fileno.to_be_bytes());   // 4 bytes
        fm.extend_from_slice(&parent.to_be_bytes());   // 4 bytes
        fm.push(file_type);                             // 1 byte
        fm.push(filename_len);                          // 1 byte
        fm.extend_from_slice(&tags_len.to_be_bytes()); // 2 bytes
        fm.extend_from_slice(&tag_vec_u8);
        fm.extend_from_slice(filename.as_bytes());

        FileMetadata { num_tags: tags_len, fm }
    }

    pub fn get_fileno(&self) -> u32 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.fm[FILENO_OFFSET..FILENO_OFFSET + 4]);
        u32::from_be_bytes(buf)
    }

    pub fn get_length(&self) -> u64 {
        let mut buf = [0u8; 8];
        buf[3..].copy_from_slice(&self.fm[0..5]);
        u64::from_be_bytes(buf) >> 1
    }

    pub fn is_valid(&self) -> bool {
        self.fm[4] & 1 == 1
    }

    pub fn get_parent(&self) -> u32 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.fm[PARENT_OFFSET..PARENT_OFFSET + 4]);
        u32::from_be_bytes(buf)
    }

    pub fn get_file_type(&self) -> u8 {
        self.fm[FILE_TYPE_OFFSET]
    }

    pub fn get_filename_len(&self) -> u8 {
        self.fm[FILENAME_LEN_OFFSET]
    }

    pub fn get_num_tags_count(&self) -> u16 {
        self.num_tags
    }

    pub fn get_filename(&self) -> String {
        let filename_len = self.fm[FILENAME_LEN_OFFSET] as usize;
        let start = TAGS_OFFSET + self.num_tags as usize * TAG_SLOT_SIZE;
        String::from_utf8_lossy(&self.fm[start..start + filename_len]).to_string()
    }

    pub fn get_tags(&self) -> Vec<u32> {
        let mut tags = Vec::with_capacity(self.num_tags as usize);
        for i in 0..self.num_tags as usize {
            let start = TAGS_OFFSET + i * TAG_SLOT_SIZE;
            let mut buf = [0u8; 4];
            buf[1..4].copy_from_slice(&self.fm[start..start + TAG_SLOT_SIZE]);
            tags.push(u32::from_be_bytes(buf));
        }
        tags
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.fm.clone()
    }

    pub fn size_bytes(&self) -> usize {
        self.fm.len()
    }

    pub fn calculate_needed_size(num_tags: u16, name_length: u8) -> usize {
        BASE_SIZE_BYTES + num_tags as usize * TAG_SLOT_SIZE + name_length as usize
    }
}
