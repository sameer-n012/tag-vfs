use crate::app::run_configuration::RunConfiguration;
use crate::archive::archive::Archive;
use crate::archive::tag_lookup_entry;
use crate::util::named_file::NamedFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self};
use std::sync::Arc;

const INITIAL_FILE_DIR_SLOTS: u16 = 1024;
const INITIAL_TAG_DIR_SLOTS: u16 = 256;
const INITIAL_TAG_LOOKUP_SLOTS: u16 = 1024;
const INITIAL_TAG_LOOKUP_SPACE_BYTES: usize =
    INITIAL_TAG_LOOKUP_SLOTS as usize * tag_lookup_entry::MIN_SIZE_BYTES;
const INITIAL_FILE_STORAGE_SPACE_BYTES: usize = 1024 * 1024 * 1024; // 1 GB

pub struct ArchiveManager {
    run_config: Arc<RunConfiguration>,
    archive: Option<Archive>,
    open_files: HashMap<u16, NamedFile>, // maps fileno to file instance object
                                         // cache_file_names: HashMap<u16, String>, // maps fileno to cache file name
                                         // cache_file_loader: FileImporter,
}

impl ArchiveManager {
    pub fn new(rc: Arc<RunConfiguration>) -> Self {
        ArchiveManager {
            run_config: rc,
            archive: None,
            open_files: HashMap::new(),
            // cache_file_names: HashMap::new(),
            // cache_file_loader: FileImporter::new(rc.get_cache_path_absolute()),
        }
    }

    pub fn create_archive_file(&mut self, path: String) -> io::Result<()> {
        self.archive = Archive::new(
            Archive::create(
                path,
                INITIAL_FILE_DIR_SLOTS,
                INITIAL_TAG_DIR_SLOTS,
                INITIAL_TAG_LOOKUP_SPACE_BYTES,
                INITIAL_FILE_STORAGE_SPACE_BYTES,
            )
            .unwrap(),
        )
        .ok();
        return Ok(());
    }

    pub fn read_archive_file(&mut self, path: String) -> io::Result<()> {
        self.archive = Archive::new(NamedFile::new(File::open(path.clone())?, path)).ok();
        return Ok(());
    }

    pub fn open(&mut self, filename: String) -> io::Result<()> {
        self.cache(filename, true)?;
        return Ok(());
    }

    pub fn cache(&mut self, filename: String, open: bool) -> io::Result<()> {
        Ok(())
    }

    pub fn flush(&self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn flush_all(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn destroy(&self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn destroy_all(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn remove(&self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn import_files(&self, paths: Vec<String>, recursive: bool) -> io::Result<()> {
        Ok(())
    }

    pub fn add_tags(&self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn remove_tags(&self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn list_files(&self, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn size_of(&self, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn apply(
        &self,
        filenames: Vec<String>,
        tags: Vec<String>,
        command: String,
    ) -> io::Result<()> {
        Ok(())
    }

    pub fn scrape(&self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    pub fn merge(&self, path: String) -> io::Result<()> {
        Ok(())
    }

    pub fn expand_from(&self, destination: String, path: String) -> io::Result<()> {
        Ok(())
    }

    pub fn expand(&self, destination: String) -> io::Result<()> {
        Ok(())
    }

    pub fn reduce(&self, paths: Vec<String>, recursive: bool) -> io::Result<()> {
        Ok(())
    }

    // Implement other methods here
}
