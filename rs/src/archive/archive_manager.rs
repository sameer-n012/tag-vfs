use crate::app::run_configuration::RunConfiguration;
use crate::archive::archive::Archive;
use crate::util::named_file::NamedFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};

pub struct ArchiveManager {
    run_config: RunConfiguration,
    archive: Option<Archive>,
    open_files: HashMap<u16, String>,
    cache_file_names: HashMap<u16, String>,
    cache_file_loader: FileImporter,
}

impl ArchiveManager {
    pub fn new(rc: RunConfiguration) -> Self {
        ArchiveManager {
            run_config: rc,
            archive: None,
            open_files: HashMap::new(),
            cache_file_names: HashMap::new(),
            cache_file_loader: FileImporter::new(rc.get_cache_path_absolute()),
        }
    }

    pub fn create_archive_file(&mut self, path: String) -> io::Result<()> {
        self.archive = Archive::new(NamedFile::new(File::create(path.clone())?, path)).ok();
        return Ok(());
    }

    pub fn read_archive_file(&mut self, path: String) -> io::Result<()> {
        self.archive = Archive::new(NamedFile::new(File::open(path.clone())?, path)).ok();
        return Ok(());
    }

    // Implement other methods here
}
