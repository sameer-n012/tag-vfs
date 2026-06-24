use crate::app::run_configuration::RunConfiguration;
use crate::archive::archive::Archive;
use crate::archive::tag_lookup_entry;
use crate::data::file_instance::FileInstance;
use crate::util::named_file::NamedFile;
use crate::util::style;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::Path;
use std::sync::Arc;

const INITIAL_FILE_DIR_SLOTS: u32 = 1024;
const INITIAL_TAG_DIR_SLOTS: u32 = 256;
const INITIAL_TAG_LOOKUP_SLOTS: u32 = 1024;
const INITIAL_TAG_LOOKUP_SPACE_BYTES: usize =
    INITIAL_TAG_LOOKUP_SLOTS as usize * tag_lookup_entry::MIN_SIZE_BYTES;
const INITIAL_FILE_STORAGE_SPACE_BYTES: usize = 1024 * 1024 * 1024; // 1 GB

// Collects all filenos that satisfy both the filename and tag filters (AND semantics).
// Files must have ALL listed tags and match ANY listed filename.
// An empty filter list means "no constraint on that dimension".
fn collect_matching_filenos(
    archive: &mut Archive,
    filenames: &[String],
    tags: &[String],
) -> io::Result<Vec<u32>> {
    let mut candidates: Option<HashSet<u32>> = None;

    for tagname in tags {
        let set: HashSet<u32> = match archive.get_tde_from_tagname(tagname.clone())? {
            None => return Ok(Vec::new()), // tag doesn't exist → no matches
            Some(tde) => archive
                ._get_all_filenos_for_tag(tde.get_tagno())?
                .into_iter()
                .collect(),
        };
        candidates = Some(match candidates {
            None => set,
            Some(s) => s.intersection(&set).cloned().collect(),
        });
    }

    if !filenames.is_empty() {
        let mut name_set: HashSet<u32> = HashSet::new();
        for filename in filenames {
            for fde in archive.get_fde_by_filename(filename.clone())? {
                if fde.is_valid() {
                    name_set.insert(fde.get_fileno());
                }
            }
        }
        candidates = Some(match candidates {
            None => name_set,
            Some(s) => s.intersection(&name_set).cloned().collect(),
        });
    }

    Ok(candidates.unwrap_or_default().into_iter().collect())
}

// Recursively collects all file paths under a directory.
fn collect_paths_recursive(dir: &Path, out: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_paths_recursive(&p, out)?;
        } else {
            out.push(p.to_string_lossy().to_string());
        }
    }
    Ok(())
}

// Writes all valid files from an archive into a destination directory.
fn expand_archive(archive: &mut Archive, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let num_slots = archive.num_file_dir_slots();
    let mut count = 0;
    for fileno in 0..num_slots {
        match archive.get_fde(fileno) {
            Ok(fde) if fde.is_valid() => {}
            _ => continue,
        }
        let data = archive.read_file_data(fileno)?;
        let fi = match archive.read_file(fileno)? {
            Some(fi) => fi,
            None => continue,
        };
        let out_path = dest.join(&fi.name);
        fs::write(&out_path, &data)?;
        println!("  {} {} {}", style::bold(&fi.name), style::dim("→"), style::dim(&out_path.display().to_string()));
        count += 1;
    }
    println!("{} {} file(s) to {}", style::green("Expanded"), count, style::bold(&dest.display().to_string()));
    return Ok(());
}

// Formats a byte count as a human-readable string.
fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let units = ["B", "kB", "MB", "GB", "TB"];
    let exp = ((bytes as f64).log10() / (1024_f64).log10()) as usize;
    let exp = exp.min(units.len() - 1);
    format!(
        "{:.1} {}",
        bytes as f64 / 1024_f64.powi(exp as i32),
        units[exp]
    )
}

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
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.clone())?;
        self.archive = Archive::new(NamedFile::new(file, path)).ok();
        return Ok(());
    }

    pub fn open(&mut self, filename: String) -> io::Result<()> {
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        let fdes = archive.get_fde_by_filename(filename.clone())?;
        let fde = fdes.into_iter().find(|f| f.is_valid()).ok_or_else(|| {
            io::Error::new(
                ErrorKind::NotFound,
                format!("File not found in archive: {}", filename),
            )
        })?;

        let fileno = fde.get_fileno();
        let mut file_instance = archive
            .read_file(fileno)?
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "File not found in archive"))?;
        let data = archive.read_file_data(fileno)?;

        let cache_dir_path = self.run_config.get_cache_path_absolute();
        let cache_dir = Path::new(&cache_dir_path);
        fs::create_dir_all(cache_dir)?;
        let dest_path = cache_dir.join(&file_instance.name);
        fs::write(&dest_path, &data)?;

        let file = File::open(&dest_path)?;
        let dest_str = dest_path.to_string_lossy().to_string();
        let named = NamedFile::new(file, dest_str.clone());
        let next_key = self
            .open_files
            .keys()
            .max()
            .map(|k| k.saturating_add(1))
            .unwrap_or(1);
        self.open_files.insert(next_key, named);

        println!("{} {} ...", style::dim("Opening"), style::bold(&dest_str));
        file_instance.path = dest_path;
        return file_instance.open();
    }

    pub fn open_files(&mut self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        if !tags.is_empty() {
            // Tag filter: find matching filenos, collect names, then open each
            let archive = self
                .archive
                .as_mut()
                .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;
            let filenos = collect_matching_filenos(archive, &filenames, &tags)?;
            let mut names: Vec<String> = Vec::new();
            for fileno in filenos {
                if let Some(fi) = archive.read_file(fileno)? {
                    names.push(fi.name.clone());
                }
            }
            for name in names {
                self.open(name)?;
            }
        } else {
            for filename in filenames {
                self.open(filename)?;
            }
        }
        return Ok(());
    }

    pub fn cache(&mut self, filename: String, open: bool) -> io::Result<()> {
        let src_path = Path::new(&filename);
        if !src_path.exists() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("Source file does not exist: {}", filename),
            ));
        }

        let cache_dir_path = self.run_config.get_cache_path_absolute();
        let cache_dir = Path::new(&cache_dir_path);
        fs::create_dir_all(cache_dir)?;

        let file_name = src_path
            .file_name()
            .ok_or_else(|| {
                io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid filename provided: {}", filename),
                )
            })?
            .to_os_string();

        let dest_path = cache_dir.join(file_name);
        println!("{} {} ...", style::dim("Caching"), style::bold(&filename));
        fs::copy(src_path, &dest_path)?;
        let file = File::open(&dest_path)?;
        let named = NamedFile::new(file, dest_path.to_string_lossy().to_string());

        let next_fileno = self
            .open_files
            .keys()
            .max()
            .map(|k| k.saturating_add(1))
            .unwrap_or(1);
        self.open_files.insert(next_fileno, named);

        if open {
            println!("{} open flag set for {}; not yet implemented", style::dim("Note:"), filename);
        }

        Ok(())
    }

    pub fn flush(&mut self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        if self.open_files.is_empty() {
            return Err(io::Error::new(ErrorKind::NotFound, "No cached files"));
        }

        let name_filter: Option<HashSet<String>> = if filenames.is_empty() {
            None
        } else {
            Some(
                filenames
                    .into_iter()
                    .map(|n| n.to_ascii_lowercase())
                    .collect(),
            )
        };

        // Collect candidates by filename filter (no archive needed yet)
        let candidates: Vec<(u16, String, String)> = self
            .open_files
            .iter()
            .filter_map(|(&key, named)| {
                let file_name = Path::new(&named.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(ref filter) = name_filter {
                    if !filter.contains(&file_name.to_ascii_lowercase()) {
                        return None;
                    }
                }
                Some((key, named.path.clone(), file_name))
            })
            .collect();

        // Apply tag filter: keep only cached files whose archive entry has all given tags
        let work: Vec<(u16, String, String)> = if tags.is_empty() {
            candidates
        } else {
            let archive = self
                .archive
                .as_mut()
                .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;
            let mut tag_nos: Vec<u32> = Vec::new();
            for tagname in &tags {
                match archive.get_tde_from_tagname(tagname.clone())? {
                    Some(tde) => tag_nos.push(tde.get_tagno()),
                    None => return Ok(()), // unknown tag → nothing to flush
                }
            }
            let mut filtered = Vec::new();
            for item in candidates {
                let fdes = archive
                    .get_fde_by_filename(item.2.clone())
                    .unwrap_or_default();
                if let Some(fde) = fdes.into_iter().find(|f| f.is_valid()) {
                    if let Ok(fm) = archive.get_fm(fde.get_offset()) {
                        let file_tags: Vec<u32> = fm.get_tags();
                        if tag_nos.iter().all(|t| file_tags.contains(t)) {
                            filtered.push(item);
                        }
                    }
                }
            }
            filtered
        };

        if work.is_empty() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "No cached files matched the filters",
            ));
        }

        for (_, cached_path, file_name) in &work {
            let new_data = fs::read(cached_path)?;

            let archive = self
                .archive
                .as_mut()
                .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

            let fdes = archive.get_fde_by_filename(file_name.clone())?;
            if let Some(fde) = fdes.into_iter().find(|f| f.is_valid()) {
                let changed = archive.update_file_data(fde.get_fileno(), new_data)?;
                if changed {
                    println!("{} {}", style::green("Flushed"), style::bold(file_name));
                } else {
                    println!("{}", style::dim(&format!("No changes in {}", file_name)));
                }
            } else {
                archive.add_file(FileInstance::new(cached_path, None, None))?;
                println!("{} {} to archive", style::green("Added"), style::bold(file_name));
            }
        }

        for (cache_key, ..) in &work {
            self.open_files.remove(cache_key);
        }

        return Ok(());
    }

    pub fn flush_all(&mut self) -> io::Result<()> {
        self.flush(Vec::new(), Vec::new())
    }

    pub fn destroy(&mut self, filenames: Vec<String>, _tags: Vec<String>) -> io::Result<()> {
        if self.open_files.is_empty() {
            return Err(io::Error::new(ErrorKind::NotFound, "No cached files"));
        }

        if filenames.is_empty() {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "Must specify at least one filename to destroy",
            ));
        }

        let filter: HashSet<String> = filenames
            .into_iter()
            .map(|name| name.to_ascii_lowercase())
            .collect();

        let mut removed = Vec::new();
        for (&fileno, named) in self.open_files.iter() {
            if let Some(name) = Path::new(&named.path).file_name().and_then(|n| n.to_str()) {
                if filter.contains(&name.to_ascii_lowercase()) {
                    removed.push((fileno, named.path.clone()));
                }
            }
        }

        if removed.is_empty() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "No cached files matched the destroy filters",
            ));
        }

        for (fileno, path) in removed {
            self.open_files.remove(&fileno);
            let _ = fs::remove_file(&path);
        }

        Ok(())
    }

    pub fn destroy_all(&mut self) -> io::Result<()> {
        if self.open_files.is_empty() {
            return Err(io::Error::new(ErrorKind::NotFound, "No cached files"));
        }

        let paths: Vec<String> = self
            .open_files
            .iter()
            .map(|(_, named)| named.path.clone())
            .collect();

        self.open_files.clear();

        for path in paths {
            let _ = fs::remove_file(&path);
        }

        Ok(())
    }

    /*
     * Removes files from the archive matching the given filenames and tags.
     * Files must satisfy all provided filters: matching any given filename
     * AND having all given tags. If only filenames are given, removes those
     * files. If only tags are given, removes all files with all of those tags.
     *
     * @param filenames the filenames to match (empty = no filename filter).
     * @param tags the tag names to match (empty = no tag filter).
     * @return io::Result<()> indicating success or failure.
     */
    pub fn remove(&mut self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        if filenames.is_empty() && tags.is_empty() {
            return Ok(());
        }
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        let filenos = collect_matching_filenos(archive, &filenames, &tags)?;
        for fileno in filenos {
            archive.remove_file(fileno)?;
        }
        Ok(())
    }

    /*
     * Imports files from the filesystem into the archive. Directories are
     * only traversed when recursive is true; otherwise they are silently skipped.
     *
     * @param paths the file or directory paths to import.
     * @param recursive whether to recurse into subdirectories.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn import_files(&mut self, paths: Vec<String>, recursive: bool) -> io::Result<()> {
        let mut all_paths: Vec<String> = Vec::new();
        for path_str in &paths {
            let p = Path::new(path_str);
            if p.is_dir() {
                if recursive {
                    collect_paths_recursive(p, &mut all_paths)?;
                }
            } else {
                all_paths.push(path_str.clone());
            }
        }

        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;
        for path_str in all_paths {
            archive.add_file(FileInstance::new(&path_str, None, None))?;
        }
        Ok(())
    }

    /*
     * Adds the given tags to every file matching the given filenames. Tags
     * that do not yet exist in the archive are created automatically.
     *
     * @param filenames the filenames to match.
     * @param tags the tag names to add.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn add_tags(&mut self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        if filenames.is_empty() || tags.is_empty() {
            return Ok(());
        }
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        // Resolve tag names to tagnos, creating missing tags
        let mut tagnos: Vec<u32> = Vec::new();
        for tagname in &tags {
            let tagno = match archive.get_tde_from_tagname(tagname.clone())? {
                Some(tde) => tde.get_tagno(),
                None => {
                    archive.add_tag(tagname.clone())?;
                    archive
                        .get_tde_from_tagname(tagname.clone())?
                        .ok_or_else(|| io::Error::new(ErrorKind::Other, "Failed to create tag"))?
                        .get_tagno()
                }
            };
            tagnos.push(tagno);
        }

        let filenos = collect_matching_filenos(archive, &filenames, &[])?;
        for fileno in filenos {
            let fde = archive.get_fde(fileno)?;
            let fm = archive.get_fm(fde.get_offset())?;
            let mut current_tags: Vec<u32> = fm.get_tags();
            let mut added: Vec<u32> = Vec::new();
            for &tagno in &tagnos {
                if !current_tags.contains(&tagno) {
                    current_tags.push(tagno);
                    added.push(tagno);
                }
            }
            if !added.is_empty() {
                archive._update_file_tags(fileno, current_tags)?;
                for tagno in added {
                    archive._make_tle(tagno, vec![fileno])?;
                }
            }
        }
        Ok(())
    }

    /*
     * Removes the given tags from every file matching the given filenames.
     * Tags or files that do not exist are silently ignored.
     *
     * @param filenames the filenames to match.
     * @param tags the tag names to remove.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn remove_tags(&mut self, filenames: Vec<String>, tags: Vec<String>) -> io::Result<()> {
        if filenames.is_empty() || tags.is_empty() {
            return Ok(());
        }
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        // Resolve tag names, skipping unknown tags
        let mut tagnos: Vec<u32> = Vec::new();
        for tagname in &tags {
            if let Some(tde) = archive.get_tde_from_tagname(tagname.clone())? {
                tagnos.push(tde.get_tagno());
            }
        }
        if tagnos.is_empty() {
            return Ok(());
        }

        let filenos = collect_matching_filenos(archive, &filenames, &[])?;
        for fileno in filenos {
            let fde = archive.get_fde(fileno)?;
            let fm = archive.get_fm(fde.get_offset())?;
            let mut current_tags: Vec<u32> = fm.get_tags();
            let mut removed: Vec<u32> = Vec::new();
            current_tags.retain(|t| {
                if tagnos.contains(t) {
                    removed.push(*t);
                    false
                } else {
                    true
                }
            });
            if !removed.is_empty() {
                archive._update_file_tags(fileno, current_tags)?;
                for tagno in removed {
                    archive._remove_fileno_from_tag_lookup(fileno, tagno)?;
                }
            }
        }
        Ok(())
    }

    /*
     * Lists all files in the archive that have all of the given tags. If no
     * tags are provided, lists every valid file in the archive.
     *
     * @param tags the tag names to filter by (all must be present).
     * @return io::Result<()> indicating success or failure.
     */
    pub fn list_files(&mut self, tags: Vec<String>) -> io::Result<()> {
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        let filenos: Vec<u32> = if tags.is_empty() {
            (0..archive.num_file_dir_slots())
                .filter_map(|i| archive.get_fde(i).ok())
                .filter(|fde| fde.is_valid())
                .map(|fde| fde.get_fileno())
                .collect()
        } else {
            let empty: Vec<String> = Vec::new();
            collect_matching_filenos(archive, &empty, &tags)?
        };

        for fileno in filenos {
            if let Ok(Some(file)) = archive.read_file(fileno) {
                println!("{}", file.to_string());
            }
        }
        Ok(())
    }

    /*
     * Prints the combined size of all files in the archive that have all of
     * the given tags. If no tags are provided, prints the combined size of
     * every valid file.
     *
     * @param tags the tag names to filter by (all must be present).
     * @return io::Result<()> indicating success or failure.
     */
    pub fn size_of(&mut self, tags: Vec<String>) -> io::Result<()> {
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        let filenos: Vec<u32> = if tags.is_empty() {
            (0..archive.num_file_dir_slots())
                .filter_map(|i| archive.get_fde(i).ok())
                .filter(|fde| fde.is_valid())
                .map(|fde| fde.get_fileno())
                .collect()
        } else {
            let empty: Vec<String> = Vec::new();
            collect_matching_filenos(archive, &empty, &tags)?
        };

        let mut total: u64 = 0;
        for fileno in filenos {
            if let Ok(fde) = archive.get_fde(fileno) {
                total += fde.get_length();
            }
        }
        println!("{}", format_size(total));
        Ok(())
    }

    /*
     * Lists all tags in the archive, printing each tag name along with the
     * number of files that have that tag.
     *
     * @return io::Result<()> indicating success or failure.
     */
    pub fn list_tags(&mut self) -> io::Result<()> {
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        let mut count = 0u32;
        for tagno in 0..archive.num_tag_dir_slots() {
            let tde = archive.get_tde(tagno)?;
            if !tde.is_valid() {
                continue;
            }
            let filenos = archive._get_all_filenos_for_tag(tagno)?;
            let file_count = filenos.len();
            println!("{} {}",
                style::bold_cyan(&tde.get_name()),
                style::dim(&format!("({} file{})", file_count, if file_count == 1 { "" } else { "s" })));
            count += 1;
        }
        if count == 0 {
            println!("{}", style::dim("No tags in archive."));
        }
        return Ok(());
    }

    /*
     * Prints metadata for every file in the archive that matches the given
     * filename. Shows size, file type, parent, and all associated tags.
     *
     * @param filename the filename to look up.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn stat_file(&mut self, filename: String) -> io::Result<()> {
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        let fdes = archive.get_fde_by_filename(filename.clone())?;
        let valid_fdes: Vec<_> = fdes.into_iter().filter(|f| f.is_valid()).collect();
        if valid_fdes.is_empty() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("File not found in archive: {}", filename),
            ));
        }

        for fde in valid_fdes {
            let fileno = fde.get_fileno();
            let fm = archive.get_fm(fde.get_offset())?;

            let tag_names: Vec<String> = fm
                .get_tags()
                .iter()
                .filter_map(|&tagno| archive.get_tde(tagno).ok())
                .filter(|tde| tde.is_valid())
                .map(|tde| tde.get_name())
                .collect();

            let file_type_str = match fm.get_file_type() {
                1 => "directory",
                2 => "file",
                _ => "unknown",
            };

            println!("{}", style::bold(&fm.get_filename()));
            println!("  {}  {}", style::dim("fileno:"), style::bold(&fileno.to_string()));
            println!("  {}    {}", style::dim("size:"), style::bold(&format_size(fm.get_length())));
            println!("  {}    {}", style::dim("type:"), file_type_str);
            if !tag_names.is_empty() {
                println!("  {}    {}", style::dim("tags:"), tag_names.join(", "));
            } else {
                println!("  {}    {}", style::dim("tags:"), style::dim("(none)"));
            }
        }
        return Ok(());
    }

    pub fn apply(
        &self,
        _filenames: Vec<String>,
        _tags: Vec<String>,
        _command: String,
    ) -> io::Result<()> {
        Ok(())
    }

    pub fn scrape(&self, _filenames: Vec<String>, _tags: Vec<String>) -> io::Result<()> {
        Ok(())
    }

    /*
     * Merges all files and tags from another archive file into the working
     * archive. Tags that do not yet exist are created automatically. Files
     * are written to the session cache temporarily to satisfy add_file's
     * path requirement and cleaned up afterward.
     *
     * @param path the path to the source .dat archive file to merge.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn merge(&mut self, path: String) -> io::Result<()> {
        // Phase 1: open source archive and collect all valid files.
        let src_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.clone())?;
        let mut src = Archive::new(NamedFile::new(src_file, path))?;

        let num_slots = src.num_file_dir_slots();
        let mut collected: Vec<(String, Vec<u8>, Vec<String>)> = Vec::new();
        for fileno in 0..num_slots {
            match src.get_fde(fileno) {
                Ok(fde) if fde.is_valid() => {}
                _ => continue,
            }
            let data = src.read_file_data(fileno)?;
            let fi = match src.read_file(fileno)? {
                Some(fi) => fi,
                None => continue,
            };
            let tags: Vec<String> = fi.tags.into_iter().collect();
            collected.push((fi.name, data, tags));
        }
        drop(src);

        // Phase 2: add collected files to the current archive.
        let cache_dir_path = self.run_config.get_cache_path_absolute();
        fs::create_dir_all(&cache_dir_path)?;

        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;

        for (name, data, tags) in &collected {
            // Ensure all source tags exist in the current archive.
            for tagname in tags {
                if archive.get_tde_from_tagname(tagname.clone())?.is_none() {
                    archive.add_tag(tagname.clone())?;
                }
            }

            // Write to a temp file in the cache dir so add_file can read it.
            let temp_path = Path::new(&cache_dir_path).join(name);
            fs::write(&temp_path, data)?;

            let mut fi = FileInstance::new(&temp_path.to_string_lossy(), None, None);
            fi.tags = tags.iter().cloned().collect();
            archive.add_file(fi)?;

            let _ = fs::remove_file(&temp_path);
            println!("{} {}", style::green("Merged"), style::bold(&name));
        }

        return Ok(());
    }

    /*
     * Expands the files stored in a given archive file to a directory tree.
     * Opens the archive at path without replacing the active working archive.
     *
     * @param destination the directory path to write files into.
     * @param path the path to the source .dat archive file to expand.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn expand_from(&mut self, destination: String, path: String) -> io::Result<()> {
        let src_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.clone())?;
        let mut src = Archive::new(NamedFile::new(src_file, path))?;
        return expand_archive(&mut src, Path::new(&destination));
    }

    /*
     * Expands all files in the working archive to a directory tree.
     *
     * @param destination the directory path to write files into.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn expand(&mut self, destination: String) -> io::Result<()> {
        let archive = self
            .archive
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No archive loaded"))?;
        return expand_archive(archive, Path::new(&destination));
    }

    /*
     * Compresses files from the filesystem into the working archive.
     * Directories are only traversed when recursive is true.
     *
     * @param paths the file or directory paths to compress.
     * @param recursive whether to recurse into subdirectories.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn reduce(&mut self, paths: Vec<String>, recursive: bool) -> io::Result<()> {
        return self.import_files(paths, recursive);
    }

    // Implement other methods here
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::io;
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct ScopedHome {
        prev: Option<OsString>,
    }

    impl ScopedHome {
        fn set(path: &Path) -> Self {
            let prev = env::var_os("HOME");
            env::set_var("HOME", path);
            ScopedHome { prev }
        }
    }

    impl Drop for ScopedHome {
        fn drop(&mut self) {
            match self.prev.take() {
                Some(val) => env::set_var("HOME", val),
                None => env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn cache_creates_cached_file() -> io::Result<()> {
        let home = TempDir::new()?;
        let _guard = ScopedHome::set(home.path());
        let config = RunConfiguration::new(std::env::args());
        let rc = Arc::new(config);
        let mut manager = ArchiveManager::new(Arc::clone(&rc));

        let source = TempDir::new()?;
        let source_file = source.path().join("cache_test.txt");
        fs::write(&source_file, b"content")?;

        manager.cache(source_file.to_string_lossy().to_string(), false)?;

        let cached_path = Path::new(&rc.get_cache_path_absolute()).join("cache_test.txt");
        assert!(cached_path.exists());
        assert_eq!(manager.open_files.len(), 1);
        assert_eq!(fs::read(&cached_path)?, b"content");
        Ok(())
    }

    #[test]
    fn flush_removes_cached_file_by_name() -> io::Result<()> {
        let home = TempDir::new()?;
        let _guard = ScopedHome::set(home.path());
        let config = RunConfiguration::new(std::env::args());
        let rc = Arc::new(config);
        let mut manager = ArchiveManager::new(Arc::clone(&rc));

        // Create archive in a separate temp dir to avoid dirs::home_dir() ambiguity
        let archive_dir = TempDir::new()?;
        let archive_path = archive_dir
            .path()
            .join("archive.dat")
            .to_string_lossy()
            .to_string();
        manager.create_archive_file(archive_path)?;

        let source = TempDir::new()?;
        let source_file = source.path().join("cache_flush.txt");
        fs::write(&source_file, b"data")?;

        manager.cache(source_file.to_string_lossy().to_string(), false)?;
        assert_eq!(manager.open_files.len(), 1);

        // flush adds the cached file to the archive as a new entry
        manager.flush(vec!["cache_flush.txt".to_string()], vec![])?;

        assert!(manager.open_files.is_empty());
        let cached_path = Path::new(&rc.get_cache_path_absolute()).join("cache_flush.txt");
        assert!(cached_path.exists());
        Ok(())
    }

    #[test]
    fn destroy_filters_cached_files() -> io::Result<()> {
        let home = TempDir::new()?;
        let _guard = ScopedHome::set(home.path());
        let config = RunConfiguration::new(std::env::args());
        let rc = Arc::new(config);
        let mut manager = ArchiveManager::new(Arc::clone(&rc));

        let source = TempDir::new()?;
        let first = source.path().join("keep.txt");
        let second = source.path().join("remove.txt");
        fs::write(&first, b"keep")?;
        fs::write(&second, b"remove")?;

        manager.cache(first.to_string_lossy().to_string(), false)?;
        manager.cache(second.to_string_lossy().to_string(), false)?;

        assert_eq!(manager.open_files.len(), 2);
        manager.destroy(vec!["remove.txt".to_string()], vec![])?;
        assert_eq!(manager.open_files.len(), 1);

        assert!(manager
            .open_files
            .values()
            .any(|named| named.path.contains("keep.txt")));
        Ok(())
    }

    #[test]
    fn expand_writes_files_to_directory() -> io::Result<()> {
        let home = TempDir::new()?;
        let _guard = ScopedHome::set(home.path());
        let config = RunConfiguration::new(std::env::args());
        let rc = Arc::new(config);
        let mut manager = ArchiveManager::new(Arc::clone(&rc));

        let archive_dir = TempDir::new()?;
        let archive_path = archive_dir
            .path()
            .join("archive.dat")
            .to_string_lossy()
            .to_string();
        manager.create_archive_file(archive_path)?;

        // Import a file into the archive
        let source = TempDir::new()?;
        let src_file = source.path().join("hello.txt");
        fs::write(&src_file, b"hello world")?;
        manager.import_files(vec![src_file.to_string_lossy().to_string()], false)?;

        // Expand to a destination directory
        let dest = TempDir::new()?;
        manager.expand(dest.path().to_string_lossy().to_string())?;

        let expanded = dest.path().join("hello.txt");
        assert!(expanded.exists(), "expanded file should exist");
        assert_eq!(fs::read(&expanded)?, b"hello world");
        Ok(())
    }

    #[test]
    fn reduce_adds_files_to_archive() -> io::Result<()> {
        let home = TempDir::new()?;
        let _guard = ScopedHome::set(home.path());
        let config = RunConfiguration::new(std::env::args());
        let rc = Arc::new(config);
        let mut manager = ArchiveManager::new(Arc::clone(&rc));

        let archive_dir = TempDir::new()?;
        let archive_path = archive_dir
            .path()
            .join("archive.dat")
            .to_string_lossy()
            .to_string();
        manager.create_archive_file(archive_path)?;

        let source = TempDir::new()?;
        let src_file = source.path().join("reduce_test.txt");
        fs::write(&src_file, b"reduced content")?;

        manager.reduce(vec![src_file.to_string_lossy().to_string()], false)?;

        // Verify file is in the archive by expanding it
        let dest = TempDir::new()?;
        manager.expand(dest.path().to_string_lossy().to_string())?;
        let out = dest.path().join("reduce_test.txt");
        assert!(out.exists(), "reduced file should appear after expand");
        assert_eq!(fs::read(&out)?, b"reduced content");
        Ok(())
    }

    #[test]
    fn merge_combines_two_archives() -> io::Result<()> {
        let home = TempDir::new()?;
        let _guard = ScopedHome::set(home.path());
        let config = RunConfiguration::new(std::env::args());
        let rc = Arc::new(config);

        // Create archive A with file_a.txt
        let dir_a = TempDir::new()?;
        let path_a = dir_a.path().join("a.dat").to_string_lossy().to_string();
        let mut mgr_a = ArchiveManager::new(Arc::clone(&rc));
        mgr_a.create_archive_file(path_a.clone())?;
        let source = TempDir::new()?;
        let file_a = source.path().join("file_a.txt");
        fs::write(&file_a, b"from archive a")?;
        mgr_a.import_files(vec![file_a.to_string_lossy().to_string()], false)?;

        // Create archive B with file_b.txt
        let dir_b = TempDir::new()?;
        let path_b = dir_b.path().join("b.dat").to_string_lossy().to_string();
        let mut mgr_b = ArchiveManager::new(Arc::clone(&rc));
        mgr_b.create_archive_file(path_b.clone())?;
        let file_b = source.path().join("file_b.txt");
        fs::write(&file_b, b"from archive b")?;
        mgr_b.import_files(vec![file_b.to_string_lossy().to_string()], false)?;

        // Merge B into A
        mgr_a.merge(path_b)?;

        // Expand A and verify both files are present
        let dest = TempDir::new()?;
        mgr_a.expand(dest.path().to_string_lossy().to_string())?;
        let out_a = dest.path().join("file_a.txt");
        let out_b = dest.path().join("file_b.txt");
        assert!(out_a.exists(), "file_a.txt should be in merged archive");
        assert!(out_b.exists(), "file_b.txt should be in merged archive");
        assert_eq!(fs::read(&out_a)?, b"from archive a");
        assert_eq!(fs::read(&out_b)?, b"from archive b");
        Ok(())
    }

    #[test]
    fn destroy_all_clears_cache() -> io::Result<()> {
        let home = TempDir::new()?;
        let _guard = ScopedHome::set(home.path());
        let config = RunConfiguration::new(std::env::args());
        let rc = Arc::new(config);
        let mut manager = ArchiveManager::new(Arc::clone(&rc));

        let source = TempDir::new()?;
        let file_a = source.path().join("a.txt");
        fs::write(&file_a, b"a")?;

        manager.cache(file_a.to_string_lossy().to_string(), false)?;
        assert_eq!(manager.open_files.len(), 1);

        manager.destroy_all()?;
        assert_eq!(manager.open_files.len(), 0);
        Ok(())
    }
}
