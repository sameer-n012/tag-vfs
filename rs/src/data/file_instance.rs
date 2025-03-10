use crate::data::file_type::FileType;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

struct FileInstance {
    name: String,
    file_type: FileType,
    size: usize,
    path: PathBuf,
    parent: Option<PathBuf>,
    tags: HashSet<String>,
}

impl FileInstance {
    pub fn new(path: &str, file_type: Option<FileType>, size: Option<usize>) -> Self {
        let path = Path::new(path);
        FileInstance {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            file_type: file_type.unwrap_or(FileType::UNK),
            size: size.unwrap_or(0),
            path: PathBuf::from(path),
            parent: path.parent().and_then(|p| Some(PathBuf::from(p))),
            tags: HashSet::new(),
        }
    }

    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::DIR
    }

    pub fn get_formatted_size(&self) -> String {
        if self.size == 0 {
            return "0 B".to_string();
        }

        let units = ["B", "kB", "MB", "GB", "TB"];
        let digit_groups = ((self.size as f64).log10() / (1024_f64).log10()) as i32;
        format!(
            "{:.1} {}",
            self.size as f64 / 1024_f64.powi(digit_groups),
            units[digit_groups as usize]
        )
    }

    fn view(&self) -> io::Result<Vec<u8>> {
        let mut file = File::open(&self.path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    fn open(&self) -> io::Result<()> {
        if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(&["/C", "start", &self.path.to_string_lossy()])
                .output()?;
        } else if cfg!(target_os = "macos") {
            std::process::Command::new("open")
                .arg(&self.path)
                .output()?;
        } else if cfg!(target_os = "linux") {
            std::process::Command::new("xdg-open")
                .arg(&self.path)
                .output()?;
        }
        Ok(())
    }

    fn open_new(&self) -> io::Result<()> {
        let path = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        let mut filename = self.name.clone();
        if let Some(extension) = self.path.extension() {
            filename = format!(
                "{}-temp.{}",
                self.path.file_stem().unwrap().to_string_lossy(),
                extension.to_string_lossy()
            );
        } else {
            filename.push_str("-temp");
        }

        let new_path = path.join(filename);
        fs::write(&new_path, self.view()?)?;

        // Open the file
        return self.open();
    }

    fn delete(&self) -> io::Result<()> {
        fs::remove_file(&self.path)
    }
}

impl ToString for FileInstance {
    fn to_string(&self) -> String {
        format!(
            "{}{} ({})",
            self.name,
            if self.is_directory() { "/" } else { "" },
            self.get_formatted_size()
        )
    }
}
