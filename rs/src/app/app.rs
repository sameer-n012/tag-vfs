use crate::app::run_configuration::RunConfiguration;
use crate::archive::archive_manager::ArchiveManager;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub struct App {
    pub config: Arc<RunConfiguration>,
    am: ArchiveManager,
}

impl App {
    pub fn new(config: RunConfiguration) -> Self {
        let config = Arc::new(config);
        let mut app = App {
            config: Arc::clone(&config),
            am: ArchiveManager::new(Arc::clone(&config)),
        };
        app.initialize_app();
        app
    }

    fn initialize_app(&mut self) {
        println!("Initializing App...");
        self.setup_app_directory();
    }

    fn setup_app_directory(&mut self) {
        let app_home_path = self.config.get_app_home_path_absolute();
        if !Path::new(&app_home_path).exists() {
            fs::create_dir_all(&app_home_path).expect("No suitable location to persist data");
        }

        let cache_path = self.config.get_cache_path_absolute();
        if !Path::new(&cache_path).exists() {
            fs::create_dir_all(&cache_path).expect("No suitable location to persist data");
        }

        let archive_path = self.config.get_archive_path_absolute();
        if !Path::new(&archive_path).exists() {
            self.am
                .create_archive_file(archive_path)
                .expect("Failed to create archive file");
        } else {
            self.am
                .read_archive_file(archive_path)
                .expect("Failed to read archive file");
        }
    }

    pub fn clean(&self) {
        let cache_path = self.config.get_cache_path_absolute();
        if Path::new(&cache_path).exists() {
            fs::remove_dir_all(&cache_path).expect("Failed to clean cache directory");
        }
    }
}
