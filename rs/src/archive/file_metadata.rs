pub const MIN_SIZE_BYTES: u8 = 104 / 8;

pub struct FileMetadata {
    filename_len: u8,
    num_tags: u16,
    fm: Vec<u8>,
}

impl FileMetadata {
    pub fn from_bytes(fm: Vec<u8>) -> Self {
        let filename_len = fm[10];
        let mut buf = [0; 2];
        buf.copy_from_slice(&fm[11..13]);
        let num_tags = u16::from_be_bytes(buf);

        FileMetadata {
            filename_len,
            num_tags,
            fm,
        }
    }

    pub fn new(
        fileno: u16,
        length: u64,
        valid: bool,
        parent: u16,
        file_type: u8,
        filename: &str,
        tags: Vec<u16>,
    ) -> Self {
        let filename_len = filename.len() as u8;
        let tags_len = tags.len() as u16;
        if (filename_len > u8::MAX) {
            panic!(
                "Filename length exceeds the maximum allowed value of {}",
                u8::MAX
            );
        }

        let mut tag_vec_u8 = Vec::with_capacity(tags_len as usize * 2);
        for tag in tags {
            tag_vec_u8.extend_from_slice(&tag.to_be_bytes());
        }

        let mut fm =
            Vec::with_capacity(MIN_SIZE_BYTES as usize + filename_len as usize + tag_vec_u8.len());
        fm[0..5].copy_from_slice(&(length << 1 + (if valid { 1 } else { 0 })).to_be_bytes()[3..]);
        fm[5..7].copy_from_slice(&fileno.to_be_bytes());
        fm[7..9].copy_from_slice(&parent.to_be_bytes());
        fm[9] = file_type;
        fm[10] = filename.len() as u8;
        fm[11..13].copy_from_slice(&(tags_len as u16).to_be_bytes());
        fm.extend_from_slice(&tag_vec_u8);
        fm.extend_from_slice(filename.as_bytes());

        FileMetadata {
            filename_len,
            num_tags: tags_len,
            fm,
        }
    }

    pub fn get_fileno(&self) -> u16 {
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.fm[5..7]);
        u16::from_be_bytes(buf)
    }

    pub fn get_length(&self) -> u64 {
        let mut buf = [0; 8];
        buf[3..].copy_from_slice(&self.fm[0..5]);
        u64::from_be_bytes(buf) >> 1
    }

    pub fn is_valid(&self) -> bool {
        self.fm[4] & 1 == 1
    }

    pub fn get_parent(&self) -> u16 {
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.fm[7..9]);
        u16::from_be_bytes(buf)
    }

    pub fn get_file_type(&self) -> u8 {
        self.fm[9]
    }

    pub fn get_filename(&self) -> String {
        let filename_len = self.fm[10] as usize;
        let filename = String::from_utf8_lossy(&self.fm[13 + self.num_tags as usize * 2..]);
        filename.to_string()
    }

    pub fn get_tags(&self) -> Vec<u16> {
        let mut tags = Vec::with_capacity(self.num_tags as usize);
        for i in 0..self.num_tags {
            let mut buf = [0; 2];
            buf.copy_from_slice(&self.fm[13 + i as usize * 2..]);
            tags.push(u16::from_be_bytes(buf));
        }
        tags
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.fm.clone()
    }

    pub fn size_bytes(&self) -> usize {
        self.fm.len() * 2
    }
}
