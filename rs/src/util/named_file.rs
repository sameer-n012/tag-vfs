use std::fs::File;
use std::io::{self, Read, Write};

pub struct NamedFile {
    pub path: String,
    pub file: File,
}

impl NamedFile {
    pub fn new(file: File, path: String) -> Self {
        NamedFile { path, file }
    }
}
