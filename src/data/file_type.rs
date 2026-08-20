#[derive(Debug, PartialEq, Eq)]
pub enum FileType {
    DIR,
    FILE,
    UNK,
}

impl FileType {
    pub fn from_str(ftype: &str) -> Self {
        match ftype {
            "dir" => FileType::DIR,
            "file" => FileType::FILE,
            _ => FileType::UNK,
        }
    }

    pub fn from_u8(ftype: u8) -> Self {
        match ftype {
            1 => FileType::DIR,
            2 => FileType::FILE,
            _ => FileType::UNK,
        }
    }
}
