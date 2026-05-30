use memmap2::MmapMut;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::archive::{
    file_directory_entry, file_end_metadata, file_metadata, tag_directory_entry, tag_lookup_entry,
};
use crate::data::file_instance::FileInstance;
use crate::data::file_type::FileType;
use crate::util::named_file::NamedFile;

use super::tag_lookup_entry::TagLookupEntry;

// Constants
const MAGIC_NUMBER: i32 = 13579;
const ARCHIVE_COPY_TMP_FILENAME: &str = "_archive_copy_tmp.dat";
const ARCHIVE_BACKUP_FILENAME: &str = "_archive_copy.dat.bak";

const NUMBER_SECTIONS: u8 = 5;
const HEAD_S: u8 = 0; // header section
const FLDR_S: u8 = 1; // file directory section
const TGDR_S: u8 = 2; // tag directory section
const TGLK_S: u8 = 3; // tag lookup section
const FLST_S: u8 = 4; // file storage section

const MAX_FILE_DIR_SLOTS: u16 = u16::MAX;
const MAX_TAG_DIR_SLOTS: u16 = u16::MAX;

const RESIZE_FILL_FACTOR_THRESHOLD: f32 = 0.5;
const RESIZE_FACTOR: u8 = 2;

pub struct Archive {
    fpath: String,
    file: File,
    section_offset: Vec<usize>, // should be an array of length 5, with first value set to 0
    head_l: RwLock<()>,
    fldr_l: RwLock<()>,
    tgdr_l: RwLock<()>,
    tglk_l: RwLock<()>,
    flst_l: RwLock<()>,

    mmap_mut: MmapMut,

    num_file_dir_slots: u16,
    num_file_dir_slots_used: u16,
    // fldr_mbb: Option<MappedByteBuffer>,
    num_tag_dir_slots: u16,
    num_tag_dir_slots_used: u16,
    // tgdr_mbb: Option<MappedByteBuffer>,
    tag_lookup_section_size: u16,      // includes metadata
    tag_lookup_section_size_used: u16, // includes metadata
    num_tag_lookup_tuples: u16,
    // tglk_mbb: Option<MappedByteBuffer>,
    file_storage_section_size: u64,      // includes metadata
    file_storage_section_size_used: u64, // includes metadata
}

impl Archive {
    /**
     * Constructor to create the archive object from a file. Sets up locks for the archive file,
     * validates that it is an archive file, and reads the metadata for each section.
     *
     * @param file where read the archive from.
     */
    pub fn new(file: NamedFile) -> io::Result<Self> {
        // Initialize an Archive instance
        let mut a = Self {
            fpath: file.path,
            file: file.file.try_clone().unwrap(),

            section_offset: vec![0; 5],
            head_l: RwLock::new(()),
            fldr_l: RwLock::new(()),
            tgdr_l: RwLock::new(()),
            tglk_l: RwLock::new(()),
            flst_l: RwLock::new(()),

            mmap_mut: unsafe { MmapMut::map_mut(&file.file.try_clone().unwrap())? },

            num_file_dir_slots: 0,
            num_file_dir_slots_used: 0,
            // fldr_mbb: None,
            num_tag_dir_slots: 0,
            num_tag_dir_slots_used: 0,
            // tgdr_mbb: None,
            tag_lookup_section_size: 0,
            tag_lookup_section_size_used: 0,
            num_tag_lookup_tuples: 0,
            // tglk_mbb: None,
            file_storage_section_size: 0,
            file_storage_section_size_used: 0,
        };

        if !a._validate_file_type()? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is not a valid archive file",
            ));
        }

        a._read_section_pointers()?;
        a._read_s1_meta()?;
        a._read_s2_meta()?;
        a._read_s3_meta()?;
        a._read_s4_meta()?;

        return Ok(a);
    }

    /**
     * Copies the archive file to a backup with name given by Archive.ARCHIVE_BACKUP_FILENAME preceded
     * by a number.
     *
     * @return the backup file.
     */
    fn _backup_archive(&mut self) -> io::Result<()> {
        let l1 = self.head_l.read().unwrap();
        let l2 = self.fldr_l.read().unwrap();
        let l3 = self.tgdr_l.read().unwrap();
        let l4 = self.tglk_l.read().unwrap();
        let l5 = self.flst_l.read().unwrap();

        let mut fp: PathBuf = PathBuf::from(self.fpath.clone());
        fp.pop();
        let mut c = 0;
        while PathBuf::from(fp.as_os_str())
            .join(c.to_string() + ARCHIVE_BACKUP_FILENAME)
            .exists()
        {
            c += 1;
        }

        let mut new_file: File = File::create(
            PathBuf::from(fp.as_os_str()).join(c.to_string() + ARCHIVE_BACKUP_FILENAME),
        )?;

        let mut byte_buf: [u8; 1024 * 1024] = [0; 1024 * 1024];
        let mut bytes_read: usize;
        loop {
            bytes_read = self.file.read(&mut byte_buf).unwrap();
            if bytes_read <= 0 {
                break;
            }
            new_file.write(&byte_buf[0..bytes_read])?;
        }

        new_file.flush()?;
        self.file.rewind()?;

        return Ok(());
    }

    /**
     * Resizes the archive based on the current fill factor. For each section, if the space used
     * is greater than Archive.RESIZE_FILL_FACTOR_THRESHOLD the archive section size is multiplied by
     * Archive.RESIZE_FACTOR. The data is copied to a temporary file given by Archive.ARCHIVE_COPY_TMP_FILENAME
     * before the temporary file is renamed to the original file.
     *
     */
    fn _resize_archive(&mut self) -> io::Result<()> {
        {
            let l1 = self.head_l.write().unwrap();
            let l2 = self.fldr_l.write().unwrap();
            let l3 = self.tgdr_l.write().unwrap();
            let l4 = self.tglk_l.write().unwrap();
            let l5 = self.flst_l.write().unwrap();

            let mut fp: PathBuf = PathBuf::from(self.fpath.clone());
            fp.pop();
            let mut c = 0;
            while PathBuf::from(fp.as_os_str())
                .join(c.to_string() + ARCHIVE_BACKUP_FILENAME)
                .exists()
            {
                c += 1;
            }

            let suffix: String = c.to_string() + ARCHIVE_BACKUP_FILENAME;

            let full_path: PathBuf = PathBuf::from(fp.as_os_str()).join(suffix);
            let new_path = full_path.to_str().unwrap();
            let mut new_file: File = File::create(Path::new(new_path))?;

            let mut new_num_file_dir_slots = self.num_file_dir_slots;
            if (self.num_file_dir_slots_used as f32
                > self.num_file_dir_slots as f32 * RESIZE_FILL_FACTOR_THRESHOLD)
            {
                new_num_file_dir_slots =
                    MAX_FILE_DIR_SLOTS.max(self.num_file_dir_slots * RESIZE_FACTOR as u16)
            }
            let mut new_num_tag_dir_slots = self.num_tag_dir_slots;
            if (self.num_tag_dir_slots_used as f32
                > self.num_tag_dir_slots as f32 * RESIZE_FILL_FACTOR_THRESHOLD)
            {
                new_num_tag_dir_slots =
                    MAX_TAG_DIR_SLOTS.max(self.num_tag_dir_slots * RESIZE_FACTOR as u16);
            }
            let mut new_tag_lookup_section_size = self.tag_lookup_section_size;
            if (self.tag_lookup_section_size_used as f32
                > self.tag_lookup_section_size as f32 * RESIZE_FILL_FACTOR_THRESHOLD)
            {
                new_tag_lookup_section_size = self.tag_lookup_section_size * RESIZE_FACTOR as u16;
            }
            let mut new_file_storage_section_size = self.file_storage_section_size;
            if (self.file_storage_section_size_used as f32
                > self.file_storage_section_size as f32 * RESIZE_FILL_FACTOR_THRESHOLD)
            {
                new_file_storage_section_size =
                    self.file_storage_section_size * RESIZE_FACTOR as u64;
            }

            // write section 0 (same bit→byte fix as create)
            new_file.write(&(MAGIC_NUMBER as u16).to_be_bytes())?;
            let mut offset: u64 = (48 * 4 + 16) / 8; // 26 bytes
            new_file.write(&offset.to_be_bytes()[2..8])?;
            offset += 4 + file_directory_entry::SIZE_BYTES as u64 * new_num_file_dir_slots as u64;
            new_file.write(&offset.to_be_bytes()[2..8])?;
            offset += 4 + tag_directory_entry::SIZE_BYTES as u64 * new_num_tag_dir_slots as u64;
            new_file.write(&offset.to_be_bytes()[2..8])?;
            offset += 4 + new_tag_lookup_section_size as u64;
            new_file.write(&offset.to_be_bytes()[2..8])?;

            const BUF_SIZE: usize = 1024 * 1024;
            let mut byte_buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
            let mut bytes_read: usize;

            // write section 1
            new_file.write(&new_num_file_dir_slots.to_be_bytes())?;
            self.file.seek(SeekFrom::Current(2))?;
            let mut bytes_left =
                self.num_file_dir_slots * file_directory_entry::SIZE_BYTES as u16 + 2;
            loop {
                bytes_read = self
                    .file
                    .read(&mut byte_buf[0..BUF_SIZE.min(bytes_left as usize)])
                    .unwrap();
                if bytes_read == 0 {
                    break;
                }
                new_file.write(&byte_buf[0..bytes_read])?;
                bytes_left -= bytes_read as u16;
            }
            Archive::_write_empty(
                &mut new_file,
                (new_num_file_dir_slots - self.num_file_dir_slots) as u64
                    * file_directory_entry::SIZE_BYTES as u64,
            )?;

            // write section 2
            new_file.write(&new_num_tag_dir_slots.to_be_bytes())?;
            self.file.seek(SeekFrom::Current(2))?;
            let mut bytes_left =
                self.num_tag_dir_slots * tag_directory_entry::SIZE_BYTES as u16 + 2;
            loop {
                bytes_read = self
                    .file
                    .read(&mut byte_buf[0..BUF_SIZE.min(bytes_left as usize)])
                    .unwrap();
                if bytes_read == 0 {
                    break;
                }
                new_file.write(&byte_buf[0..bytes_read])?;
                bytes_left -= bytes_read as u16;
            }
            Archive::_write_empty(
                &mut new_file,
                (new_num_tag_dir_slots - self.num_tag_dir_slots) as u64
                    * tag_directory_entry::SIZE_BYTES as u64,
            )?;

            // write section 3
            new_file.write(&new_tag_lookup_section_size.to_be_bytes())?;
            new_file.write(&self.num_tag_lookup_tuples.to_be_bytes())?;
            self.file.seek(SeekFrom::Current(4))?; // skip 2×u16 header in source
            let mut bytes_left = self.tag_lookup_section_size + 4;
            loop {
                bytes_read = self
                    .file
                    .read(&mut byte_buf[0..BUF_SIZE.min(bytes_left as usize)])
                    .unwrap();
                if bytes_read == 0 {
                    break;
                }
                new_file.write(&byte_buf[0..bytes_read])?;
                bytes_left -= bytes_read as u16;
            }
            Archive::_write_empty(
                &mut new_file,
                (new_tag_lookup_section_size - self.tag_lookup_section_size) as u64,
            )?;

            // write section 4
            let mut bytes_left = self.file_storage_section_size;
            loop {
                bytes_read = self
                    .file
                    .read(&mut byte_buf[0..BUF_SIZE.min(bytes_left as usize)])
                    .unwrap();
                if bytes_read == 0 {
                    break;
                }
                new_file.write(&byte_buf[0..bytes_read])?;
                bytes_left -= bytes_read as u64;
            }
            let file_length: u64 = new_file_storage_section_size
                - self.file_storage_section_size
                - file_metadata::MIN_SIZE_BYTES as u64
                - file_end_metadata::SIZE_BYTES as u64;

            let new_fm = file_metadata::FileMetadata::new(0, file_length, false, 0, 0, "", vec![]);
            let new_fem = file_end_metadata::FileEndMetadata::new(file_length);
            new_file.write(&new_fm.as_bytes())?;
            Archive::_write_empty(&mut new_file, file_length)?;
            new_file.write(&new_fem.as_bytes())?;

            new_file.flush()?;
            self.file.rewind()?;

            fs::remove_file(&self.fpath)?;
            fs::rename(new_path, &self.fpath)?;
            self.file = File::open(&self.fpath)?;
        }

        self._read_section_pointers()?;
        self._read_s1_meta()?;
        self._read_s2_meta()?;
        self._read_s3_meta()?;
        self._read_s4_meta()?;

        Ok(())
    }

    /**
     * Validates that the file given is an archive file for this application using the magic number.
     *
     * @return true if the file is valid and false otherwise.
     */
    fn _validate_file_type(&self) -> io::Result<bool> {
        let lock = self.head_l.read().unwrap();

        if (u16::from_be_bytes(self.mmap_mut[0..2].try_into().unwrap()) != MAGIC_NUMBER as u16) {
            return Ok(false);
        }
        Ok(true)
    }

    /**
     * Reads the pointers to each section found in the archive header.
     *
     */
    fn _read_section_pointers(&mut self) -> io::Result<()> {
        // Read section pointers

        let lock = self.head_l.write().unwrap();

        for i in 1..NUMBER_SECTIONS {
            let mut buf = [0u8; 8];
            buf[2..8].copy_from_slice(
                &self.mmap_mut[2 + (i as usize - 1) * 6..2 + i as usize * 6],
            );
            self.section_offset[i as usize] = usize::from_be_bytes(buf);
        }

        Ok(())
    }

    /**
     * Reads the metadata found in the file directory section including
     * current storage section fill, total slots, and slots used.
     *
     */
    fn _read_s1_meta(&mut self) -> io::Result<()> {
        let lock = self.fldr_l.write().unwrap();

        self.num_file_dir_slots = u16::from_be_bytes(
            self.mmap_mut[self.section_offset[FLDR_S as usize] as usize
                ..(self.section_offset[FLDR_S as usize] as usize + 2)]
                .try_into()
                .unwrap(),
        );

        self.num_file_dir_slots_used = u16::from_be_bytes(
            self.mmap_mut[self.section_offset[FLDR_S as usize] as usize + 2
                ..(self.section_offset[FLDR_S as usize] as usize + 4)]
                .try_into()
                .unwrap(),
        );

        let mut bytes_read: usize = 0;
        let mut space_used: u64 = 0;
        let mut buffer: u64;
        while bytes_read
            < self.num_file_dir_slots as usize * file_directory_entry::SIZE_BYTES as usize
        {
            buffer = u64::from_be_bytes(
                self.mmap_mut[self.section_offset[FLDR_S as usize] as usize + 4 + bytes_read
                    ..self.section_offset[FLDR_S as usize] as usize + 4 + bytes_read + 8]
                    .try_into()
                    .unwrap(),
            );
            if buffer % 2 == 1 {
                space_used += buffer >> 1;
            }
            bytes_read += file_directory_entry::SIZE_BYTES as usize - 5;
        }
        self.file_storage_section_size_used = space_used + 2;

        Ok(())
    }

    /**
     * Reads the metadata found in the tag directory section including
     * total slots and slots used.
     *
     */
    fn _read_s2_meta(&mut self) -> io::Result<()> {
        let lock = self.tgdr_l.write().unwrap();

        self.num_tag_dir_slots = u16::from_be_bytes(
            self.mmap_mut
                [self.section_offset[TGDR_S as usize]..(self.section_offset[TGDR_S as usize] + 2)]
                .try_into()
                .unwrap(),
        );

        self.num_tag_dir_slots_used = u16::from_be_bytes(
            self.mmap_mut[self.section_offset[TGDR_S as usize] + 2
                ..(self.section_offset[TGDR_S as usize] + 4)]
                .try_into()
                .unwrap(),
        );

        return Ok(());
    }

    /**
     * Reads the metadata found in the tag lookup section including
     * total section size, space used, and number of lookup tuples.
     *
     */
    fn _read_s3_meta(&mut self) -> io::Result<()> {
        let lock = self.tglk_l.write().unwrap();

        self.tag_lookup_section_size = u16::from_be_bytes(
            self.mmap_mut
                [self.section_offset[TGLK_S as usize]..(self.section_offset[TGLK_S as usize] + 2)]
                .try_into()
                .unwrap(),
        );

        self.num_tag_lookup_tuples = u16::from_be_bytes(
            self.mmap_mut[self.section_offset[TGLK_S as usize] + 2
                ..(self.section_offset[TGLK_S as usize] + 4)]
                .try_into()
                .unwrap(),
        );

        let mut bytes_read: usize = 0;
        let mut num_file_slots: u16;

        while bytes_read < self.tag_lookup_section_size as usize {
            if self.mmap_mut[self.section_offset[TGLK_S as usize] + 4 + bytes_read] & 0x80 != 0 {
                num_file_slots = u16::from_be_bytes(
                    self.mmap_mut[self.section_offset[TGLK_S as usize] + 4 + bytes_read + 1
                        ..self.section_offset[TGLK_S as usize] + 4 + bytes_read + 3]
                        .try_into()
                        .unwrap(),
                );
                bytes_read += (2 + 1 + 2 + 2 * num_file_slots + 5) as usize;
            } else {
                bytes_read += 2 + 1 + 2 + 2 * 0 + 5;
            }
        }

        Ok(())
    }

    /**
     * Reads the metadata found in the file storage section including
     * total section size.
     *
     */
    fn _read_s4_meta(&mut self) -> io::Result<()> {
        let lock = self.flst_l.write().unwrap();
        self.file_storage_section_size =
            (self.mmap_mut.len() - self.section_offset[FLST_S as usize]) as u64;

        Ok(())
    }

    /**
     * Gets the corresponding file directory entry that matches
     * the given file number.
     *
     * @param fileno the file number to search for.
     * @return a file directory entry.
     */
    pub fn get_fde(&mut self, fileno: u16) -> io::Result<file_directory_entry::FileDirectoryEntry> {
        let lock = self.fldr_l.read().unwrap();

        let buf: [u8; file_directory_entry::SIZE_BYTES as usize] = self.mmap_mut[self.section_offset
            [FLDR_S as usize]
            + 4
            + fileno as usize * file_directory_entry::SIZE_BYTES as usize
            ..self.section_offset[FLDR_S as usize]
                + 4
                + (fileno + 1) as usize * file_directory_entry::SIZE_BYTES as usize]
            .try_into()
            .unwrap();

        return Ok(file_directory_entry::FileDirectoryEntry::from_bytes(
            fileno, buf,
        ));
    }

    /**
     * Gets the corresponding list of file directory entries that match
     * the given filename. Note that multiple files can have the same filename.
     * Uses the filename hash to quickly match filenames before checking the file metadata.
     *
     * @param filename the filename to search for.
     * @return an vector of file directory entries.
     */
    pub fn get_fde_by_filename(
        &mut self,
        filename: String,
    ) -> io::Result<Vec<file_directory_entry::FileDirectoryEntry>> {
        let lock = self.fldr_l.read().unwrap();

        let filename_hash: u16 = Archive::_hash_filename(filename);

        let mut fdes: Vec<file_directory_entry::FileDirectoryEntry> = Vec::new();

        let mut buf: [u8; file_directory_entry::SIZE_BYTES as usize];
        for i in 0..self.num_file_dir_slots as usize {
            buf = self.mmap_mut[self.section_offset[FLDR_S as usize]
                + 4
                + i * file_directory_entry::SIZE_BYTES as usize
                ..self.section_offset[FLDR_S as usize]
                    + 4
                    + (i + 1) * file_directory_entry::SIZE_BYTES as usize]
                .try_into()
                .unwrap();
            let fde = file_directory_entry::FileDirectoryEntry::from_bytes(i as u16, buf);
            if (fde.is_valid() && fde.get_filename_hash() == filename_hash) {
                fdes.push(file_directory_entry::FileDirectoryEntry::from_bytes(
                    i as u16, buf,
                ));
            }
        }

        return Ok(fdes);
    }

    /**
     * Searches for a valid tag directory entry with a tag that matches the tag
     * number given.
     *
     * @param tag the name of the tag to search for.
     * @return the tag directory entry found.
     */
    pub fn get_tde(&self, tagno: u16) -> io::Result<tag_directory_entry::TagDirectoryEntry> {
        let lock = self.tgdr_l.read().unwrap();

        let buf: [u8; tag_directory_entry::SIZE_BYTES as usize] = self.mmap_mut[self.section_offset
            [TGDR_S as usize]
            + 4
            + tagno as usize * tag_directory_entry::SIZE_BYTES as usize
            ..self.section_offset[TGDR_S as usize]
                + 4
                + (tagno + 1) as usize * tag_directory_entry::SIZE_BYTES as usize]
            .try_into()
            .unwrap();

        return Ok(tag_directory_entry::TagDirectoryEntry::from_bytes(
            tagno, buf,
        ));
    }

    /**
     * Searches for a valid tag directory entry with a tag that matches the name
     * given. Note that tag names are unique.
     *
     * @param tag the name of the tag to search for.
     * @return the tag directory entry found, or none if none are found
     */
    pub fn get_tde_from_tagname(
        &self,
        tagname: String,
    ) -> io::Result<Option<tag_directory_entry::TagDirectoryEntry>> {
        let lock = self.tgdr_l.read().unwrap();

        let mut buf: [u8; tag_directory_entry::SIZE_BYTES as usize];
        for i in 0..self.num_tag_dir_slots as usize {
            buf = self.mmap_mut[self.section_offset[TGDR_S as usize]
                + 4
                + i * tag_directory_entry::SIZE_BYTES as usize
                ..self.section_offset[TGDR_S as usize]
                    + 4
                    + (i + 1) * tag_directory_entry::SIZE_BYTES as usize]
                .try_into()
                .unwrap();

            let tde = tag_directory_entry::TagDirectoryEntry::from_bytes(i as u16, buf);

            if (tde.is_valid() && tde.get_name() == tagname) {
                return Ok(Some(tde));
            }
        }

        return Ok(None);
    }

    /**
     * Gets the file metadata located at a specific offset.
     *
     * @param offset the offset into the file storage section where the metadata is located.
     * @return the file metadata.
     */
    pub fn get_fm(&self, offset: u64) -> io::Result<file_metadata::FileMetadata> {
        let mut buf: Vec<u8> = self.mmap_mut[self.section_offset[FLST_S as usize] + offset as usize
            ..self.section_offset[FLST_S as usize]
                + offset as usize
                + file_metadata::MIN_SIZE_BYTES as usize]
            .to_vec();

        let name_len = buf[10] as usize;
        let num_tags = u16::from_be_bytes(buf[11..13].try_into().unwrap()) as usize;

        buf.extend_from_slice(
            &self.mmap_mut[self.section_offset[FLST_S as usize]
                + offset as usize
                + file_metadata::MIN_SIZE_BYTES as usize
                ..self.section_offset[FLST_S as usize]
                    + offset as usize
                    + file_metadata::MIN_SIZE_BYTES as usize
                    + name_len
                    + num_tags * 2]
                .to_vec(),
        );

        return Ok(file_metadata::FileMetadata::from_bytes(buf));
    }

    /**
     * Creates the file directory entry in the file directory entry section. Will attempt to resize
     * the archive if there is no space, and if unable to do so, will not create the entry.
     *
     * @param length the length of the file.
     * @param parent the file number of the parent of the file (-1 if parent is root).
     * @param filename the name of the file.
     * @param offset the offset into the file storage section at which the file (and its metadata) is
     *               located in the file storage section.
     * @return the new file directory entry, or null if none was able to be created.
     */
    fn _make_fde(
        &mut self,
        length: u64,
        parent: u16,
        filename: String,
        offset: u64,
    ) -> io::Result<file_directory_entry::FileDirectoryEntry> {
        let mut need_resize: bool = false;
        {
            let lock = self.fldr_l.read().unwrap();

            // all slots are currently filled
            if (self.num_file_dir_slots_used == self.num_file_dir_slots) {
                if (self.num_file_dir_slots == u16::MAX) {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Maximum number of file directory slots reached",
                    ));
                } else {
                    need_resize = true;
                }
            }
        }
        if need_resize {
            self._resize_archive()?;
        }

        let lock = self.fldr_l.write().unwrap();

        let filename_hash: u16 = Archive::_hash_filename(filename);

        let mut buf: [u8; file_directory_entry::SIZE_BYTES as usize];
        for i in 0..self.num_file_dir_slots {
            buf = self.mmap_mut[self.section_offset[FLDR_S as usize]
                + 4
                + i as usize * file_directory_entry::SIZE_BYTES as usize
                ..self.section_offset[FLDR_S as usize]
                    + 4
                    + (i + 1) as usize * file_directory_entry::SIZE_BYTES as usize]
                .try_into()
                .unwrap();

            if (!file_directory_entry::FileDirectoryEntry::from_bytes(i, buf).is_valid()) {
                let fde = file_directory_entry::FileDirectoryEntry::new(
                    i,
                    length,
                    true,
                    parent,
                    filename_hash,
                    offset,
                );

                self.mmap_mut[self.section_offset[FLDR_S as usize]
                    + 4
                    + i as usize * file_directory_entry::SIZE_BYTES as usize
                    ..self.section_offset[FLDR_S as usize]
                        + 4
                        + (i + 1) as usize * file_directory_entry::SIZE_BYTES as usize]
                    .copy_from_slice(&fde.as_bytes());

                self.num_file_dir_slots_used += 1;
                self.mmap_mut[self.section_offset[FLDR_S as usize] + 2
                    ..self.section_offset[FLDR_S as usize] + 4]
                    .copy_from_slice(&self.num_file_dir_slots_used.to_be_bytes());

                return Ok(fde);
            }
        }

        return Err(io::Error::new(
            io::ErrorKind::Other,
            "No empty file directory slots found",
        ));
    }

    /**
     * Deletes the file directory entry in the file directory entry section by zeroing all bits.
     * Does not delete the file metadata/data, tag data, or move around the file directory entries.
     * The file data and tag data should be fixed before running this, otherwise there will be
     * inconsistencies in the archive.
     *
     * @param fileno the file number to delete.
     */
    fn _delete_fde(&mut self, fileno: u16) -> io::Result<()> {
        let l = self.fldr_l.write().unwrap();

        let buf: [u8; file_directory_entry::SIZE_BYTES as usize] =
            [0; file_directory_entry::SIZE_BYTES as usize];

        self.mmap_mut[self.section_offset[FLDR_S as usize]
            + 4
            + fileno as usize * file_directory_entry::SIZE_BYTES as usize
            ..self.section_offset[FLDR_S as usize]
                + 4
                + (fileno + 1) as usize * file_directory_entry::SIZE_BYTES as usize]
            .copy_from_slice(&buf);

        self.num_file_dir_slots_used -= 1;
        self.mmap_mut[self.section_offset[FLDR_S as usize] + 2
            ..self.section_offset[FLDR_S as usize] + 4]
            .copy_from_slice(&self.num_file_dir_slots_used.to_be_bytes());

        return Ok(());
    }

    /**
     * Creates the tag directory entry in the tag directory entry section. Will attempt to resize
     * the archive if there is no space, and if unable to do so, will not create the entry.
     *
     * @param tagname the name of the tag.
     * @param offset the offset into the tag lookup storage section at which the first tag lookup
     *               tuple is located.
     * @return the new file directory entry, or null if none was able to be created.
     */
    fn _make_tde(
        &mut self,
        tagname: String,
        offset: u64,
    ) -> io::Result<tag_directory_entry::TagDirectoryEntry> {
        if tagname.len() > tag_directory_entry::MAX_TAG_NAME_LENGTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Tag name is too long",
            ));
        }

        let mut need_resize: bool = false;
        {
            let lock = self.tgdr_l.read().unwrap();

            // all slots are currently filled
            if (self.num_tag_dir_slots_used == self.num_tag_dir_slots) {
                if (self.num_tag_dir_slots == u16::MAX) {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Maximum number of tag directory slots reached",
                    ));
                } else {
                    need_resize = true;
                }
            }
        }
        if need_resize {
            self._resize_archive()?;
        }

        let lock = self.tgdr_l.write().unwrap();

        let mut buf: [u8; tag_directory_entry::SIZE_BYTES as usize];
        for i in 0..self.num_tag_dir_slots {
            buf = self.mmap_mut[self.section_offset[TGDR_S as usize]
                + 4
                + i as usize * tag_directory_entry::SIZE_BYTES as usize
                ..self.section_offset[TGDR_S as usize]
                    + 4
                    + (i + 1) as usize * tag_directory_entry::SIZE_BYTES as usize]
                .try_into()
                .unwrap();

            if (!tag_directory_entry::TagDirectoryEntry::from_bytes(i, buf).is_valid()) {
                let tde =
                    tag_directory_entry::TagDirectoryEntry::new(i, true, tagname.as_str(), offset);

                self.mmap_mut[self.section_offset[TGDR_S as usize]
                    + 4
                    + i as usize * tag_directory_entry::SIZE_BYTES as usize
                    ..self.section_offset[TGDR_S as usize]
                        + 4
                        + (i + 1) as usize * tag_directory_entry::SIZE_BYTES as usize]
                    .copy_from_slice(&tde.as_bytes());

                self.num_tag_dir_slots_used += 1;
                self.mmap_mut[self.section_offset[TGDR_S as usize] + 2
                    ..self.section_offset[TGDR_S as usize] + 4]
                    .copy_from_slice(&self.num_tag_dir_slots_used.to_be_bytes());

                return Ok(tde);
            }
        }

        return Err(io::Error::new(
            io::ErrorKind::Other,
            "No empty tag directory slots found",
        ));
    }

    /**
     * Deletes the tag directory entry in the tag directory entry section by zeroing all bits.
     * Does not delete the file metadata/data, tag lookup data, or move around the tag directory entries.
     * The tag lookup data should be fixed before running this, otherwise there will be
     * inconsistencies in the archive.
     *
     * @param tagno the tag number to delete.
     */
    fn _delete_tde(&mut self, tagno: u16) -> io::Result<()> {
        let l = self.tgdr_l.write().unwrap();

        let buf: [u8; tag_directory_entry::SIZE_BYTES as usize] =
            [0; tag_directory_entry::SIZE_BYTES as usize];

        self.mmap_mut[self.section_offset[TGDR_S as usize]
            + 4
            + tagno as usize * tag_directory_entry::SIZE_BYTES as usize
            ..self.section_offset[TGDR_S as usize]
                + 4
                + (tagno + 1) as usize * tag_directory_entry::SIZE_BYTES as usize]
            .copy_from_slice(&buf);

        self.num_tag_dir_slots_used -= 1;
        self.mmap_mut[self.section_offset[TGDR_S as usize] + 2
            ..self.section_offset[TGDR_S as usize] + 4]
            .copy_from_slice(&self.num_tag_dir_slots_used.to_be_bytes());

        return Ok(());
    }

    /**
     * Creates the tag lookup entry in the tag lookup entry section. Assumes that this
     * is the last tag lookup tuple for the given tag, so no next offset is needed.
     * The lookup tuple should have 15, 31, 63, ... file slots. Updates any
     * previous tag lookup tuple to point to this one.
     *
     * @param tagno the tag number of the tag corresponding to the tag directory entry.
     * @param filenos the file numbers of the files with the tag.
     * @return the new tag lookup entry, or null if none was able to be created.
     */
    pub fn _make_tle(
        &mut self,
        tagno: u16,
        filenos: Vec<u16>,
    ) -> io::Result<tag_lookup_entry::TagLookupEntry> {
        // Pass 1: find the last TLE for this tag (the one with no valid next pointer).
        // Track its offset, slot count, and current file count.
        let mut last_offset: Option<u64> = None;
        let mut last_num_file_slots: u16 = 7; // initial value yields 15 on first creation
        let mut last_num_files: u16 = 0;
        {
            let lock = self.tglk_l.read().unwrap();
            let mut bytes_read: usize = 0;
            while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES
                < self.tag_lookup_section_size as usize
            {
                let buf: Vec<u8> = self.mmap_mut[self.section_offset[TGLK_S as usize] + 4 + bytes_read
                    ..self.section_offset[TGLK_S as usize]
                        + 4
                        + bytes_read
                        + tag_lookup_entry::BASE_SIZE_BYTES]
                    .try_into()
                    .unwrap();
                let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf);

                if tle.is_valid() && tle.tagno() == tagno && !tle.is_offset_valid() {
                    last_offset = Some(bytes_read as u64);
                    last_num_file_slots = tle.get_num_file_slots();
                    last_num_files = tle.get_num_files();
                }

                bytes_read +=
                    tag_lookup_entry::BASE_SIZE_BYTES + tle.get_num_file_slots() as usize * 2;
            }
        }

        // If the last TLE has enough free slots, insert directly without creating a new TLE.
        if let Some(prev_off) = last_offset {
            if last_num_file_slots - last_num_files >= filenos.len() as u16 {
                let lock = self.tglk_l.write().unwrap();
                let base = self.section_offset[TGLK_S as usize] + 4 + prev_off as usize;
                for (i, &fileno) in filenos.iter().enumerate() {
                    let slot = last_num_files as usize + i;
                    self.mmap_mut[base + tag_lookup_entry::BASE_SIZE_BYTES + slot * 2
                        ..base + tag_lookup_entry::BASE_SIZE_BYTES + slot * 2 + 2]
                        .copy_from_slice(&fileno.to_be_bytes());
                }
                let new_count = last_num_files + filenos.len() as u16;
                self.mmap_mut[base + 4..base + 6]
                    .copy_from_slice(&new_count.to_be_bytes());

                let full_buf: Vec<u8> = self.mmap_mut[base
                    ..base
                        + tag_lookup_entry::BASE_SIZE_BYTES
                        + last_num_file_slots as usize * 2]
                    .try_into()
                    .unwrap();
                return Ok(tag_lookup_entry::TagLookupEntry::from_bytes(full_buf));
            }
        }

        // Need a new TLE. Slot count follows the doubling sequence 15, 31, 63, ...
        let new_num_file_slots = last_num_file_slots * 2 + 1;

        // Pass 2: find a free slot large enough for the new TLE, resizing once if needed.
        let find_free_slot = |mmap_mut: &MmapMut, section_offset: usize, section_size: u16| {
            let mut bytes_read: usize = 0;
            while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES < section_size as usize {
                let buf: Vec<u8> = mmap_mut[section_offset + 4 + bytes_read
                    ..section_offset + 4 + bytes_read + tag_lookup_entry::BASE_SIZE_BYTES]
                    .try_into()
                    .unwrap();
                let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf);
                if !tle.is_valid() && tle.get_num_file_slots() >= new_num_file_slots {
                    return Some(bytes_read as u64);
                }
                bytes_read +=
                    tag_lookup_entry::BASE_SIZE_BYTES + tle.get_num_file_slots() as usize * 2;
            }
            None
        };

        let mut new_offset = {
            let lock = self.tglk_l.read().unwrap();
            find_free_slot(
                &self.mmap_mut,
                self.section_offset[TGLK_S as usize],
                self.tag_lookup_section_size,
            )
        };

        if new_offset.is_none() {
            self._resize_archive()?;
            let lock = self.tglk_l.read().unwrap();
            new_offset = find_free_slot(
                &self.mmap_mut,
                self.section_offset[TGLK_S as usize],
                self.tag_lookup_section_size,
            );
            if new_offset.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "No tag lookup entry space found",
                ));
            }
        }

        let offset = new_offset.unwrap();

        let lock = self.tglk_l.write().unwrap();

        let tle = tag_lookup_entry::TagLookupEntry::new(
            tagno,
            true,
            new_num_file_slots,
            filenos.len() as u16,
            filenos,
            0,
            false,
        );

        // write new tle
        self.mmap_mut[self.section_offset[TGLK_S as usize] + 4 + offset as usize
            ..self.section_offset[TGLK_S as usize] + 4 + offset as usize + tle.size_bytes()]
            .copy_from_slice(&tle.as_bytes());

        // update previous tle: write next offset and set the offset_valid bit
        if let Some(prev_off) = last_offset {
            let base = self.section_offset[TGLK_S as usize] + 4 + prev_off as usize;
            self.mmap_mut[base + 6..base + 11]
                .copy_from_slice(&offset.to_be_bytes()[3..]);
            let raw_n =
                u16::from_be_bytes(self.mmap_mut[base + 4..base + 6].try_into().unwrap());
            let num_slots =
                u16::from_be_bytes(self.mmap_mut[base + 2..base + 4].try_into().unwrap());
            if raw_n <= num_slots {
                self.mmap_mut[base + 4..base + 6]
                    .copy_from_slice(&(raw_n + 1).to_be_bytes());
            }
        }

        return Ok(tle);
    }

    /**
     * Deletes the tag lookup entry in the tag lookup entry section by zeroing the valid bit.
     * Does not delete or modify any of the previous tag lookup tuples. Does not coalesce
     * the tag lookup section.
     *
     * @param offset num_file_slots the number of file slots in the lookup tuple.
     * @return the new file directory entry, or null if none was able to be created.
     */
    fn _delete_tle(&mut self, offset: u16) -> io::Result<()> {
        let lock = self.tglk_l.write().unwrap();

        let mut buf: [u8; 2] = self.mmap_mut[self.section_offset[TGLK_S as usize] + 4 + offset as usize
            ..self.section_offset[TGLK_S as usize] + 4 + offset as usize + 2]
            .try_into()
            .unwrap();
        buf[1] = buf[1] & 0x6;

        // write new empty section
        self.mmap_mut[self.section_offset[TGLK_S as usize] + 4 + offset as usize
            ..self.section_offset[TGLK_S as usize] + 4 + offset as usize + 2]
            .copy_from_slice(&buf);

        return Ok(());
    }

    /**
     * Finds an appropriate space for the file and its metadata in the file storage section.
     *
     * @param length the length of the file.
     * @param metadata_length the length of the beginning file metadata.
     * @return the offset into the file storage section where the file metadata should start
     */
    fn _find_file_space(&mut self, length: u64, metadata_length: u64) -> io::Result<u64> {
        let l = self.flst_l.read().unwrap();

        let space_needed = length + metadata_length + file_end_metadata::SIZE_BYTES as u64;
        if self.file_storage_section_size_used + space_needed > self.file_storage_section_size {
            return Err(io::Error::new(io::ErrorKind::Other, "No space found"));
        }

        let mut bytes_read: usize = 4; // skip 4-byte section header
        let mut buf: [u8; 8] = [0; 8];
        while (bytes_read + 8 < self.file_storage_section_size as usize) {
            buf[3..8].copy_from_slice(
                self.mmap_mut[self.section_offset[FLST_S as usize] + bytes_read
                    ..self.section_offset[FLST_S as usize] + bytes_read + 5]
                    .try_into()
                    .unwrap(),
            );

            let val = u64::from_be_bytes(buf);
            if val % 2 == 0 && (val >> 1) >= space_needed {
                return Ok(bytes_read as u64);
            }

            if (val >> 1) == 0 {
                return Err(io::Error::new(io::ErrorKind::Other, "Zero-length block in file storage"));
            }
            bytes_read += (val >> 1) as usize;
        }

        return Err(io::Error::new(io::ErrorKind::Other, "No space found"));
    }

    /**
     * Creates space for a file at a given offset by writing the file metadata and file end-metadata.
     *
     * @param offset the offset into the file storage section indicating the beginning of the file metadata.
     * @param length the length of the file.
     * @param fileno the file number.
     * @param parent the parent of the file (-1 if the parent is root).
     * @param type the type of file.
     * @param filename the name of the file.
     * @param tags a list of tag IDs for the file
     * @return the file metadata created.
     */
    fn _allocate_file_space(
        &mut self,
        offset: u64,
        length: u64,
        fileno: u16,
        parent: u16,
        filename: String,
        filetype: u8,
        tags: Vec<u16>,
    ) -> io::Result<file_metadata::FileMetadata> {
        // Check for available space or resize
        let mut offset: u64 = 0;
        let mut need_resize: bool;
        match self._find_file_space(
            length,
            file_metadata::FileMetadata::calculate_needed_size(
                tags.len() as u16,
                filename.len() as u8,
            ) as u64,
        ) {
            Ok(x) => {
                offset = x;
                need_resize = false;
            }
            Err(_) => need_resize = true,
        }

        // If space not found, attempt to resize
        if need_resize {
            self._resize_archive()?;
            match self._find_file_space(
                length,
                file_metadata::FileMetadata::calculate_needed_size(
                    tags.len() as u16,
                    filename.len() as u8,
                ) as u64,
            ) {
                Ok(x) => {
                    offset = x;
                    need_resize = false;
                }
                Err(_) => need_resize = true,
            }

            if need_resize {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "No space found in file storage",
                ));
            }
        }

        // Write the file metadata and end-metadata at the selected offset
        let fm = file_metadata::FileMetadata::new(
            fileno,
            length,
            true,
            parent,
            filetype,
            filename.as_str(),
            tags,
        );

        let fem = file_end_metadata::FileEndMetadata::new(length);

        self.mmap_mut[self.section_offset[FLST_S as usize] + offset as usize
            ..self.section_offset[FLST_S as usize] + offset as usize + fm.size_bytes()]
            .copy_from_slice(&fm.as_bytes());

        self.mmap_mut[self.section_offset[FLST_S as usize]
            + offset as usize
            + fm.size_bytes()
            + length as usize
            ..self.section_offset[FLST_S as usize]
                + offset as usize
                + fm.size_bytes()
                + length as usize
                + fem.size_bytes()]
            .copy_from_slice(&fem.as_bytes());

        self.file_storage_section_size_used +=
            fm.size_bytes() as u64 + length + fem.size_bytes() as u64;

        return Ok(fm);
    }

    /**
     * Coalesces the tag lookup section by merging unused adjacent tag lookup
     * tuples.
     *
     * @return io::Result<()> indicating success or failure.
     */
    pub fn _coalesce_tglk(&mut self) -> io::Result<()> {
        // create new hash map mapping tag id -> vec of tuple offsets
        let mut tag_map: HashMap<u16, Vec<TagLookupEntry>> = HashMap::new();

        let mut bytes_read: usize = 4;
        while (bytes_read + tag_lookup_entry::BASE_SIZE_BYTES
            < self.tag_lookup_section_size as usize)
        {
            let buf = self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                ..self.section_offset[TGLK_S as usize]
                    + bytes_read
                    + tag_lookup_entry::BASE_SIZE_BYTES]
                .try_into()
                .unwrap();

            let tle = TagLookupEntry::from_bytes(buf);

            if tle.is_valid() {
                let full_tle = TagLookupEntry::from_bytes(
                    self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                        ..self.section_offset[TGLK_S as usize]
                            + bytes_read
                            + TagLookupEntry::calculate_needed_size(tle.get_num_file_slots())]
                        .try_into()
                        .unwrap(),
                );
                let tagno = full_tle.tagno();
                if !tag_map.contains_key(&tagno) {
                    tag_map.insert(tagno, Vec::new());
                }
                tag_map.get_mut(&tagno).unwrap().push(full_tle);
            }

            bytes_read += TagLookupEntry::calculate_needed_size(tle.get_num_file_slots());
        }

        // iterate through hash map and get sizes of future tag tuples
        let mut cur_offset = 4;
        for (tagno, vec) in tag_map.iter() {
            let mut total_files_in_tag: u32 = 0;
            let mut file_ids: Vec<u16> = Vec::new();
            for tle in vec.iter() {
                total_files_in_tag += tle.get_num_files() as u32;
                file_ids.extend_from_slice(&tle.get_filenos()[0..tle.get_num_files() as usize]);
            }

            let mut files_per_tle: Vec<u32> = Vec::new();
            let mut tle_size_counter: u32 = 15;
            while total_files_in_tag > 0 {
                files_per_tle.push(tle_size_counter);
                total_files_in_tag =
                    total_files_in_tag.saturating_sub(tle_size_counter);
                tle_size_counter = tle_size_counter * 2 + 1;
            }

            // write new tag lookup entries
            // TODO
            file_ids.extend(vec![0; *(files_per_tle.last().unwrap()) as usize]);
            for i in 0..files_per_tle.len() {
                let n = *(files_per_tle.get(i).unwrap()) as usize;
                let mut tle_file_ids: Vec<u16> = file_ids.drain(0..n.min(file_ids.len())).collect();
                let valid_len = tle_file_ids.len();
                if valid_len < n {
                    tle_file_ids.extend(vec![0; n - valid_len])
                }

                let new_tle_size = TagLookupEntry::calculate_needed_size(n as u16) as u64;
                let new_tle = TagLookupEntry::new(
                    *tagno,
                    true,
                    n as u16,
                    valid_len as u16,
                    tle_file_ids,
                    cur_offset + new_tle_size,
                    !file_ids.is_empty(),
                );

                self.mmap_mut[self.section_offset[TGLK_S as usize] + cur_offset as usize
                    ..self.section_offset[TGLK_S as usize]
                        + cur_offset as usize
                        + new_tle_size as usize]
                    .copy_from_slice(&new_tle.as_bytes());

                cur_offset += new_tle_size;

                if file_ids.is_empty() {
                    break;
                }
            }
        }

        // zero out the rest of the tag lookup section
        let zeros = vec![0; self.tag_lookup_section_size as usize - cur_offset as usize];
        self.mmap_mut[self.section_offset[TGLK_S as usize] + cur_offset as usize
            ..self.section_offset[TGLK_S as usize] + cur_offset as usize + zeros.len()]
            .copy_from_slice(zeros.as_slice());

        return Ok(());
    }

    /**
     * Coalesces the tag lookup section around a given offset. This verifies
     * that the given offset is invalid and attempts to merge it with the
     * preceding and following tag lookup tuples. If not invalid, does nothing.
     * Is comparably slow, should use _coalesce_tglk() if possible
     *
     * @param offset the offset into the tag lookup section to coalesce around.
     */
    pub fn _coalesce_tglk_around(&mut self, offset: u64) -> io::Result<()> {
        if offset == 0 {
            return Ok(());
        }
        println!("Coalescing tag lookup around offset {}", offset);
        self._coalesce_tglk()
    }

    /**
     * Coalesces the file storage section by merging all adjacent free spaces.
     *
     * @return io::Result<()> indicating success or failure.
     */
    pub fn _coalesce_flst(&mut self) -> io::Result<()> {
        let l = self.flst_l.write().unwrap();
        let mut offset = 4; // skip section header
        let section_end = self.file_storage_section_size as usize;

        while offset < section_end {
            // Read metadata
            let meta_start = self.section_offset[FLST_S as usize] + offset;
            let mut meta_buf = vec![0u8; file_metadata::BASE_SIZE_BYTES];
            meta_buf.copy_from_slice(
                &self.mmap_mut[meta_start..meta_start + file_metadata::BASE_SIZE_BYTES],
            );
            let fm = file_metadata::FileMetadata::from_bytes(meta_buf.clone());

            let length = fm.get_length() as usize;
            let valid = fm.is_valid();

            // Find contiguous invalid regions
            if !valid {
                let mut free_start = offset;
                let mut free_len = file_metadata::BASE_SIZE_BYTES
                    + length
                    + file_end_metadata::SIZE_BYTES as usize;
                let mut next_offset = offset + free_len;

                // Merge with subsequent invalid regions
                while next_offset < section_end {
                    let next_meta_start = self.section_offset[FLST_S as usize] + next_offset;
                    let mut next_meta_buf = vec![0u8; file_metadata::BASE_SIZE_BYTES];
                    next_meta_buf.copy_from_slice(
                        &self.mmap_mut
                            [next_meta_start..next_meta_start + file_metadata::BASE_SIZE_BYTES],
                    );
                    let next_fm = file_metadata::FileMetadata::from_bytes(next_meta_buf.clone());
                    let next_length = next_fm.get_length() as usize;
                    let next_valid = next_fm.is_valid();

                    if next_valid {
                        break;
                    }
                    // Merge this region
                    free_len += file_metadata::BASE_SIZE_BYTES
                        + next_length
                        + file_end_metadata::SIZE_BYTES as usize;
                    next_offset += file_metadata::BASE_SIZE_BYTES
                        + next_length
                        + file_end_metadata::SIZE_BYTES as usize;
                }

                // Write a single free block (metadata with valid=0, length=free_len - metadata - endmeta)
                let free_block_len = free_len
                    - file_metadata::BASE_SIZE_BYTES
                    - file_end_metadata::SIZE_BYTES as usize;
                let free_fm = file_metadata::FileMetadata::new(
                    0, // fileno
                    free_block_len as u64,
                    false,  // valid
                    0,      // parent
                    0,      // filetype
                    "",     // filename
                    vec![], // tags
                );
                let free_fem = file_end_metadata::FileEndMetadata::new(free_block_len as u64);

                self.mmap_mut[self.section_offset[FLST_S as usize] + free_start
                    ..self.section_offset[FLST_S as usize]
                        + free_start
                        + file_metadata::BASE_SIZE_BYTES]
                    .copy_from_slice(&free_fm.as_bytes()[..file_metadata::BASE_SIZE_BYTES]);
                self.mmap_mut[self.section_offset[FLST_S as usize]
                    + free_start
                    + file_metadata::BASE_SIZE_BYTES
                    + free_block_len
                    ..self.section_offset[FLST_S as usize]
                        + free_start
                        + file_metadata::BASE_SIZE_BYTES
                        + free_block_len
                        + file_end_metadata::SIZE_BYTES as usize]
                    .copy_from_slice(&free_fem.as_bytes());

                // Zero out the rest (optional, for security)
                // let zero_start = self.section_offset[FLST_S as usize] + free_start + file_metadata::BASE_SIZE_BYTES;
                // let zero_end = zero_start + free_block_len;
                // for b in &mut self.mmap_mut[zero_start..zero_end] { *b = 0; }

                offset = next_offset;
            } else {
                // Move to next file
                offset += file_metadata::BASE_SIZE_BYTES
                    + length
                    + file_end_metadata::SIZE_BYTES as usize;
            }
        }
        Ok(())
    }

    /**
     * Coalesces the file storage section around a given offset. This verifies
     * that the given offset is invalid and attempts to merge it with the
     * preceding and following free spaces. If not invalid, does nothing.
     * Is reasonably fast due to file end metadata.
     *
     * @param offset the offset into the file storage section to coalesce
     * around.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn _coalesce_flst_around(&mut self, offset: u64) -> io::Result<()> {
        let l = self.flst_l.write().unwrap();
        let section_start = self.section_offset[FLST_S as usize];
        let section_end = self.file_storage_section_size as usize;

        // Find previous region
        let mut prev_offset = offset as usize;
        while prev_offset > 4 {
            // Try to find the start of the previous file
            let endmeta_start =
                section_start + prev_offset - file_end_metadata::SIZE_BYTES as usize;
            let mut endmeta_buf = [0u8; file_end_metadata::SIZE_BYTES as usize];
            endmeta_buf.copy_from_slice(
                &self.mmap_mut[endmeta_start..endmeta_start + file_end_metadata::SIZE_BYTES as usize],
            );
            let prev_length =
                file_end_metadata::FileEndMetadata::from_bytes(endmeta_buf).get_length() as usize;
            let prev_meta_start = prev_offset
                - file_metadata::BASE_SIZE_BYTES
                - prev_length
                - file_end_metadata::SIZE_BYTES as usize;
            if prev_meta_start < 4 {
                break;
            }
            let mut prev_meta_buf = vec![0u8; file_metadata::BASE_SIZE_BYTES];
            prev_meta_buf.copy_from_slice(
                &self.mmap_mut[section_start + prev_meta_start
                    ..section_start + prev_meta_start + file_metadata::BASE_SIZE_BYTES],
            );
            let prev_fm = file_metadata::FileMetadata::from_bytes(prev_meta_buf.clone());
            if !prev_fm.is_valid() {
                // Merge with current
                let mut merge_start = prev_meta_start;
                let mut merge_len = file_metadata::BASE_SIZE_BYTES
                    + prev_length
                    + file_end_metadata::SIZE_BYTES as usize;

                // Check if current is also invalid
                let mut curr_meta_buf = vec![0u8; file_metadata::BASE_SIZE_BYTES];
                curr_meta_buf.copy_from_slice(
                    &self.mmap_mut[section_start + offset as usize
                        ..section_start + offset as usize + file_metadata::BASE_SIZE_BYTES],
                );
                let curr_fm = file_metadata::FileMetadata::from_bytes(curr_meta_buf.clone());
                let curr_length = curr_fm.get_length() as usize;
                if !curr_fm.is_valid() {
                    merge_len += file_metadata::BASE_SIZE_BYTES
                        + curr_length
                        + file_end_metadata::SIZE_BYTES as usize;

                    // Write merged free block
                    let free_block_len = merge_len
                        - file_metadata::BASE_SIZE_BYTES
                        - file_end_metadata::SIZE_BYTES as usize;
                    let free_fm = file_metadata::FileMetadata::new(
                        0,
                        free_block_len as u64,
                        false,
                        0,
                        0,
                        "",
                        vec![],
                    );
                    let free_fem = file_end_metadata::FileEndMetadata::new(free_block_len as u64);

                    self.mmap_mut[section_start + merge_start
                        ..section_start + merge_start + file_metadata::BASE_SIZE_BYTES]
                        .copy_from_slice(&free_fm.as_bytes()[..file_metadata::BASE_SIZE_BYTES]);
                    self.mmap_mut[section_start
                        + merge_start
                        + file_metadata::BASE_SIZE_BYTES
                        + free_block_len
                        ..section_start
                            + merge_start
                            + file_metadata::BASE_SIZE_BYTES
                            + free_block_len
                            + file_end_metadata::SIZE_BYTES as usize]
                        .copy_from_slice(&free_fem.as_bytes());
                }
            }
            break;
        }
        Ok(())
    }

    /// Returns the total number of file directory slots.
    pub fn num_file_dir_slots(&self) -> u16 {
        self.num_file_dir_slots
    }

    /// Returns all file numbers for a given tag number.
    pub fn _get_all_filenos_for_tag(&self, tagno: u16) -> io::Result<Vec<u16>> {
        let mut filenos = Vec::new();
        let mut bytes_read: usize = 4;
        while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES < self.tag_lookup_section_size as usize
        {
            let buf = self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                ..self.section_offset[TGLK_S as usize]
                    + bytes_read
                    + tag_lookup_entry::BASE_SIZE_BYTES]
                .to_vec();
            let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf.clone());
            if tle.is_valid() && tle.tagno() == tagno {
                let full_tle = tag_lookup_entry::TagLookupEntry::from_bytes(
                    self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                        ..self.section_offset[TGLK_S as usize]
                            + bytes_read
                            + tag_lookup_entry::TagLookupEntry::calculate_needed_size(
                                tle.get_num_file_slots(),
                            )]
                        .to_vec(),
                );
                filenos.extend(full_tle.get_filenos());
            }
            bytes_read +=
                tag_lookup_entry::TagLookupEntry::calculate_needed_size(tle.get_num_file_slots());
        }
        Ok(filenos)
    }

    /// Removes a tag number from a single file's metadata entry.
    fn _remove_tagno_from_file_metadata(&mut self, fileno: u16, tagno: u16) -> io::Result<()> {
        let fde = self.get_fde(fileno)?;
        let offset = fde.get_offset();
        let fm = self.get_fm(offset)?;
        let mut tags = fm.get_tags();
        let orig_len = tags.len();
        tags.retain(|&t| t != tagno);
        if tags.len() != orig_len {
            let new_fm = file_metadata::FileMetadata::new(
                fm.get_fileno(),
                fm.get_length(),
                true,
                fm.get_parent(),
                fm.get_file_type(),
                &fm.get_filename(),
                tags,
            );
            self.mmap_mut[self.section_offset[FLST_S as usize] + offset as usize
                ..self.section_offset[FLST_S as usize] + offset as usize + new_fm.size_bytes()]
                .copy_from_slice(&new_fm.as_bytes());
        }
        Ok(())
    }

    /// Deletes all tag lookup entries for a given tag number.
    fn _delete_all_tle_for_tag(&mut self, tagno: u16) -> io::Result<()> {
        let mut bytes_read: usize = 4;
        while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES < self.tag_lookup_section_size as usize
        {
            let buf = self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                ..self.section_offset[TGLK_S as usize]
                    + bytes_read
                    + tag_lookup_entry::BASE_SIZE_BYTES]
                .to_vec();
            let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf.clone());
            if tle.is_valid() && tle.tagno() == tagno {
                // Mark as invalid
                let mut new_bytes = tle.as_bytes();
                new_bytes[1] &= !1; // Clear valid bit
                self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                    ..self.section_offset[TGLK_S as usize] + bytes_read + new_bytes.len()]
                    .copy_from_slice(&new_bytes);
            }
            bytes_read +=
                tag_lookup_entry::TagLookupEntry::calculate_needed_size(tle.get_num_file_slots());
        }
        Ok(())
    }

    /// Removes a tag number from all file metadata entries.
    fn _remove_tagno_from_all_file_metadata(&mut self, tagno: u16) -> io::Result<()> {
        let mut offset = 4;
        while offset < self.file_storage_section_size as usize {
            let meta_start = self.section_offset[FLST_S as usize] + offset;
            let mut meta_buf = vec![0u8; file_metadata::BASE_SIZE_BYTES];
            meta_buf.copy_from_slice(
                &self.mmap_mut[meta_start..meta_start + file_metadata::BASE_SIZE_BYTES],
            );
            let fm = file_metadata::FileMetadata::from_bytes(meta_buf.clone());
            if fm.is_valid() {
                let mut tags = fm.get_tags();
                let orig_len = tags.len();
                tags.retain(|&t| t != tagno);
                if tags.len() != orig_len {
                    // Write back updated metadata
                    let new_fm = file_metadata::FileMetadata::new(
                        fm.get_fileno(),
                        fm.get_length(),
                        true,
                        fm.get_parent(),
                        fm.get_file_type(),
                        &fm.get_filename(),
                        tags,
                    );
                    self.mmap_mut[meta_start..meta_start + new_fm.size_bytes()]
                        .copy_from_slice(&new_fm.as_bytes());
                }
            }
            let length = fm.get_length() as usize;
            offset +=
                file_metadata::BASE_SIZE_BYTES + length + file_end_metadata::SIZE_BYTES as usize;
        }
        Ok(())
    }

    /// Removes a file number from all tag lookup entries.
    fn _remove_fileno_from_all_tag_lookups(&mut self, fileno: u16) -> io::Result<()> {
        let mut bytes_read: usize = 4;
        while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES < self.tag_lookup_section_size as usize
        {
            let buf = self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                ..self.section_offset[TGLK_S as usize]
                    + bytes_read
                    + tag_lookup_entry::BASE_SIZE_BYTES]
                .to_vec();
            let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf.clone());
            if tle.is_valid() {
                let mut full_tle = tag_lookup_entry::TagLookupEntry::from_bytes(
                    self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                        ..self.section_offset[TGLK_S as usize]
                            + bytes_read
                            + tag_lookup_entry::TagLookupEntry::calculate_needed_size(
                                tle.get_num_file_slots(),
                            )]
                        .to_vec(),
                );
                let mut filenos = full_tle.get_filenos();
                let orig_len = filenos.len();
                filenos.retain(|&f| f != fileno);
                if filenos.len() != orig_len {
                    // Write back updated TLE, preserving next-pointer
                    let new_tle = tag_lookup_entry::TagLookupEntry::new(
                        full_tle.tagno(),
                        true,
                        full_tle.get_num_file_slots(),
                        filenos.len() as u16,
                        filenos,
                        full_tle.get_next_offset(),
                        full_tle.is_offset_valid(),
                    );
                    self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                        ..self.section_offset[TGLK_S as usize] + bytes_read + new_tle.size_bytes()]
                        .copy_from_slice(&new_tle.as_bytes());
                }
            }
            bytes_read +=
                tag_lookup_entry::TagLookupEntry::calculate_needed_size(tle.get_num_file_slots());
        }
        Ok(())
    }

    /**
     * Removes a file number from the tag lookup entries for a specific tag only.
     *
     * @param fileno the file number to remove.
     * @param tagno the tag whose lookup entries to search.
     */
    pub fn _remove_fileno_from_tag_lookup(&mut self, fileno: u16, tagno: u16) -> io::Result<()> {
        let mut bytes_read: usize = 4;
        while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES < self.tag_lookup_section_size as usize
        {
            let buf = self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                ..self.section_offset[TGLK_S as usize]
                    + bytes_read
                    + tag_lookup_entry::BASE_SIZE_BYTES]
                .to_vec();
            let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf);
            if tle.is_valid() && tle.tagno() == tagno {
                let full_tle = tag_lookup_entry::TagLookupEntry::from_bytes(
                    self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                        ..self.section_offset[TGLK_S as usize]
                            + bytes_read
                            + tag_lookup_entry::TagLookupEntry::calculate_needed_size(
                                tle.get_num_file_slots(),
                            )]
                        .to_vec(),
                );
                let mut filenos = full_tle.get_filenos();
                let orig_len = filenos.len();
                filenos.retain(|&f| f != fileno);
                if filenos.len() != orig_len {
                    let new_tle = tag_lookup_entry::TagLookupEntry::new(
                        full_tle.tagno(),
                        true,
                        full_tle.get_num_file_slots(),
                        filenos.len() as u16,
                        filenos,
                        full_tle.get_next_offset(),
                        full_tle.is_offset_valid(),
                    );
                    self.mmap_mut[self.section_offset[TGLK_S as usize] + bytes_read
                        ..self.section_offset[TGLK_S as usize]
                            + bytes_read
                            + new_tle.size_bytes()]
                        .copy_from_slice(&new_tle.as_bytes());
                }
            }
            bytes_read += tag_lookup_entry::TagLookupEntry::calculate_needed_size(
                tle.get_num_file_slots(),
            );
        }
        Ok(())
    }

    /**
     * Relocates a file's storage block with an updated tag list. Reads the current
     * file data, invalidates the old block, finds new space, writes the new FM and
     * data, then updates the FDE to point to the new location.
     *
     * @param fileno the file number to update.
     * @param new_tags the new list of tag IDs for the file.
     */
    pub fn _update_file_tags(&mut self, fileno: u16, new_tags: Vec<u16>) -> io::Result<()> {
        let fde = self.get_fde(fileno)?;
        let old_offset = fde.get_offset();
        let fm = self.get_fm(old_offset)?;
        let length = fm.get_length();
        let filename = fm.get_filename();
        let parent = fm.get_parent();
        let filetype = fm.get_file_type();

        // Read data before invalidating the old block
        let old_data_start =
            self.section_offset[FLST_S as usize] + old_offset as usize + fm.size_bytes();
        let data: Vec<u8> =
            self.mmap_mut[old_data_start..old_data_start + length as usize].to_vec();

        // Invalidate old FM (clear valid bit at byte 4) and update used counter
        self.mmap_mut[self.section_offset[FLST_S as usize] + old_offset as usize + 4] &= !1;
        self.file_storage_section_size_used -=
            fm.size_bytes() as u64 + length + file_end_metadata::SIZE_BYTES as u64;

        // Scan file storage to find a free block large enough for the new FM + data + end-meta
        let new_fm_size = file_metadata::FileMetadata::calculate_needed_size(
            new_tags.len() as u16,
            filename.len() as u8,
        );
        let needed = new_fm_size + length as usize + file_end_metadata::SIZE_BYTES as usize;
        let mut new_offset: Option<usize> = None;
        let mut scan = 4usize; // skip 4-byte section header
        while scan + file_metadata::BASE_SIZE_BYTES < self.file_storage_section_size as usize {
            let scan_start = self.section_offset[FLST_S as usize] + scan;
            let base: Vec<u8> = self.mmap_mut
                [scan_start..scan_start + file_metadata::BASE_SIZE_BYTES]
                .to_vec();
            let scan_fm = file_metadata::FileMetadata::from_bytes(base);
            let block_size = file_metadata::BASE_SIZE_BYTES
                + scan_fm.get_length() as usize
                + file_end_metadata::SIZE_BYTES as usize;
            if !scan_fm.is_valid() && block_size >= needed {
                new_offset = Some(scan);
                break;
            }
            scan += block_size;
        }

        if new_offset.is_none() {
            self._resize_archive()?;
            let mut scan = 4usize; // skip 4-byte section header
            while scan + file_metadata::BASE_SIZE_BYTES < self.file_storage_section_size as usize {
                let scan_start = self.section_offset[FLST_S as usize] + scan;
                let base: Vec<u8> = self.mmap_mut
                    [scan_start..scan_start + file_metadata::BASE_SIZE_BYTES]
                    .to_vec();
                let scan_fm = file_metadata::FileMetadata::from_bytes(base);
                let block_size = file_metadata::BASE_SIZE_BYTES
                    + scan_fm.get_length() as usize
                    + file_end_metadata::SIZE_BYTES as usize;
                if !scan_fm.is_valid() && block_size >= needed {
                    new_offset = Some(scan);
                    break;
                }
                scan += block_size;
            }
            if new_offset.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "No file storage space found",
                ));
            }
        }

        let new_off = new_offset.unwrap();

        // Write new FM, data, and end-metadata at the new location
        let new_fm = file_metadata::FileMetadata::new(
            fileno,
            length,
            true,
            parent,
            filetype,
            &filename,
            new_tags,
        );
        let fem = file_end_metadata::FileEndMetadata::new(length);
        let fm_start = self.section_offset[FLST_S as usize] + new_off;
        self.mmap_mut[fm_start..fm_start + new_fm.size_bytes()]
            .copy_from_slice(&new_fm.as_bytes());
        let data_start = fm_start + new_fm.size_bytes();
        self.mmap_mut[data_start..data_start + length as usize].copy_from_slice(&data);
        let fem_start = data_start + length as usize;
        self.mmap_mut[fem_start..fem_start + file_end_metadata::SIZE_BYTES as usize]
            .copy_from_slice(&fem.as_bytes());
        self.file_storage_section_size_used +=
            new_fm.size_bytes() as u64 + length + file_end_metadata::SIZE_BYTES as u64;

        // Update FDE offset field (bytes 9..14 of the FDE entry)
        let fde_base = self.section_offset[FLDR_S as usize]
            + 4
            + fileno as usize * file_directory_entry::SIZE_BYTES as usize;
        self.mmap_mut[fde_base + 9..fde_base + 14]
            .copy_from_slice(&(new_off as u64).to_be_bytes()[3..]);

        Ok(())
    }

    /*
     * Adds a file to the archive.
     * - Adds the file directory entry at the next available slot
     * - Adds the file to the appropriate tag lookup entries
     * - Adds the file metadata, data, and end-metadata to the file storage
     *   section
     *
     * Returns an error if there is no space available or any issue occurs.
     * If an issue occurs, clean up should be performed to remove partial adds.
     * For this reason, tag lookup entries should be modified last.
     *
     * @param file the FileInstance to add.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn add_file(&mut self, file: FileInstance) -> io::Result<()> {
        // 1. Add file directory entry
        let parent = 0; // Or resolve parent if needed
        let filename = file.name.clone();
        let tags: Vec<String> = file.tags.iter().cloned().collect();
        let tagnos: Vec<u16> = tags
            .iter()
            .filter_map(|tag| {
                self.get_tde_from_tagname(tag.clone())
                    .ok()
                    .flatten()
                    .map(|tde| tde.get_tagno())
            })
            .collect();

        // Read file data
        let data = std::fs::read(&file.path)?;
        let length = data.len() as u64;
        let filetype = file.file_type as u8;

        // 1.1 Allocate file storage (metadata, data, end-metadata)
        let fileno = self.num_file_dir_slots_used;
        let metadata_offset = self._find_file_space(
            length,
            file_metadata::FileMetadata::calculate_needed_size(
                tagnos.len() as u16,
                filename.len() as u8,
            ) as u64,
        )?;
        let fm = self._allocate_file_space(
            metadata_offset,
            length,
            fileno,
            parent,
            filename.clone(),
            filetype,
            tagnos.clone(),
        )?;

        // Write file data
        let data_offset =
            self.section_offset[FLST_S as usize] + metadata_offset as usize + fm.size_bytes();
        self.mmap_mut[data_offset..data_offset + data.len()].copy_from_slice(&data);

        // 2. Add file directory entry
        let fde = match self._make_fde(length, parent, filename.clone(), metadata_offset) {
            Ok(fde) => fde,
            Err(e) => {
                // Clean up file storage
                // (Mark region as invalid, coalesce)
                self._coalesce_flst_around(metadata_offset).ok();
                return Err(e);
            }
        };

        // 3. Add file to tag lookup entries (last, for cleanup safety)
        for &tagno in &tagnos {
            // Find or create tag lookup entry for this tag
            if let Err(e) = self._make_tle(tagno, vec![fileno]) {
                // Clean up previous steps
                self._delete_fde(fileno).ok();
                self._coalesce_flst_around(metadata_offset).ok();
                return Err(e);
            }
        }

        Ok(())
    }

    /*
     * Adds a tag to the archive.
     * - Adds the tag directory entry at the next available slot
     * - Adds a single tag lookup entry of the smallest size with no files
     *
     * Returns an error if there is no space available or any issue occurs.
     * If an issue occurs, clean up should be performed to remove partial adds.
     *
     * @param tagname the name of the tag to add.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn add_tag(&mut self, tagname: String) -> io::Result<()> {
        // 1. Add tag directory entry
        let tde = match self._make_tde(tagname.clone(), 0) {
            Ok(tde) => tde,
            Err(e) => return Err(e),
        };

        // 2. Add empty tag lookup entry
        let tle = self._make_tle(tde.get_tagno(), vec![]);
        if let Err(e) = tle {
            // Clean up tag directory entry
            self._delete_tde(tde.get_tagno()).ok();
            return Err(e);
        }

        Ok(())
    }

    /*
     * Removes a file from the archive.
     * - Deletes the file directory entry
     * - Deletes the file metadata, data, and end-metadata from the file
     *   storage section
     * - Removes the file from any tag lookup entries
     * - Coalesces the file storage section around the removed file
     *
     * Returns an error if the file does not exist or any issue occurs.
     * If an issue occurs, attempt to clean up as much as possible.
     * To minimize dangling references, the file data should be deleted first,
     * followed by the tag lookup entries, and finally the file directory entry.
     *
     * @param fileno the file number of the file to remove.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn remove_file(&mut self, fileno: u16) -> io::Result<()> {
        // 1. Mark file metadata as invalid
        let fde = self.get_fde(fileno)?;
        let offset = fde.get_offset();
        let fm = self.get_fm(offset)?;

        let mut fm_bytes = fm.as_bytes();
        fm_bytes[4] &= !1; // clear valid bit
        self.mmap_mut[self.section_offset[FLST_S as usize] + offset as usize
            ..self.section_offset[FLST_S as usize] + offset as usize + fm.size_bytes()]
            .copy_from_slice(&fm_bytes);

        self.file_storage_section_size_used -= fm.size_bytes() as u64
            + fm.get_length()
            + file_end_metadata::SIZE_BYTES as u64;

        // 2. Remove file from all tag lookup entries
        self._remove_fileno_from_all_tag_lookups(fileno)?;

        // 3. Delete file directory entry
        self._delete_fde(fileno)?;

        // 4. Coalesce file storage section around the removed file
        self._coalesce_flst_around(offset)?;

        Ok(())
    }

    /*
     * Removes a tag from the archive.
     * - Deletes the tag directory entry
     * - Deletes all tag lookup entries for the tag
     * - Removes the tag from any file metadata entries
     *
     * Returns an error if the tag does not exist or any issue occurs.
     * If an issue occurs, attempt to clean up as much as possible.
     * To minimize dangling references, the file metadata entries should be
     * deleted first, followed by the tag lookup entries, and then the tag
     * directory entry. The file metadata entries are found by iterating through
     * all tag lookup entries for the tag.
     *
     * @param tagno the tag number of the tag to remove.
     * @return io::Result<()> indicating success or failure.
     */
    pub fn remove_tag(&mut self, tagno: u16) -> io::Result<()> {
        // 1. Remove tag from all file metadata entries
        self._remove_tagno_from_all_file_metadata(tagno)?;

        // 2. Delete all tag lookup entries for this tag and coalesce
        self._delete_all_tle_for_tag(tagno)?;
        self._coalesce_tglk()?;

        // 3. Delete tag directory entry
        self._delete_tde(tagno)?;

        Ok(())
    }

    /*
     * Reads a file from the archive.
     * - Finds the corresponding file directory entry
     * - Reads the file metadata, data, and end-metadata from the file storage
     *   section
     * - Reads the tag names from the tag directory entries
     *
     * Returns an error if any issue occurs.
     *
     * @param fileno the file number of the file to read.
     * @return io::Result<Option<FileInstance>> containing the FileInstance
     * if found, or None if not found.
     */
    pub fn read_file(&mut self, fileno: u16) -> io::Result<Option<FileInstance>> {
        // 1. Find file directory entry
        let fde = match self.get_fde(fileno) {
            Ok(fde) if fde.is_valid() => fde,
            _ => return Ok(None),
        };

        // 2. Read file metadata
        let offset = fde.get_offset();
        let fm = self.get_fm(offset)?;

        // 3. Read file data
        let data_offset = self.section_offset[FLST_S as usize] + offset as usize + fm.size_bytes();
        let data_len = fm.get_length() as usize;
        let data = self.mmap_mut[data_offset..data_offset + data_len].to_vec();

        // 4. Read tag names
        let tags: HashSet<String> = fm
            .get_tags()
            .iter()
            .filter_map(|&tagno| self.get_tde(tagno).ok().map(|tde| tde.get_name()))
            .collect();

        // 5. Build FileInstance
        let file_instance = FileInstance {
            name: fm.get_filename(),
            file_type: FileType::from_u8(fm.get_file_type()),
            size: data_len,
            path: PathBuf::from(fm.get_filename()),
            parent: None, // Could be resolved if needed
            tags,
        };

        Ok(Some(file_instance))
    }

    /**
     * Hashes a filename to a 16-bit integer using the djb2 algorithm.
     *
     * @param filename the filename to hash
     * @return the hash value
     */
    fn _hash_filename(filename: String) -> u16 {
        let mut hasher = DefaultHasher::new();
        filename.hash(&mut hasher);
        (hasher.finish() & 0xffff) as u16
    }

    /**
     * Returns the raw data bytes for a file stored in the archive.
     *
     * @param fileno the file number to read.
     * @return the file data bytes.
     */
    pub fn read_file_data(&mut self, fileno: u16) -> io::Result<Vec<u8>> {
        let fde = self.get_fde(fileno)?;
        if !fde.is_valid() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File {} not found or invalid", fileno),
            ));
        }
        let offset = fde.get_offset();
        let fm = self.get_fm(offset)?;
        let data_start =
            self.section_offset[FLST_S as usize] + offset as usize + fm.size_bytes();
        let data_len = fm.get_length() as usize;
        return Ok(self.mmap_mut[data_start..data_start + data_len].to_vec());
    }

    /**
     * Replaces a file's data in the archive with new bytes, relocating the storage
     * block if the new length differs. Compares old and new bytes first; returns
     * false without writing if the content is unchanged.
     * Updates both the file storage block and the FDE length and offset fields.
     *
     * @param fileno the file number to update.
     * @param new_data the replacement data bytes.
     * @return Ok(true) if data was written, Ok(false) if content was unchanged.
     */
    pub fn update_file_data(&mut self, fileno: u16, new_data: Vec<u8>) -> io::Result<bool> {
        let fde = self.get_fde(fileno)?;
        let old_offset = fde.get_offset();
        let fm = self.get_fm(old_offset)?;
        let old_length = fm.get_length() as usize;
        let filename = fm.get_filename();
        let parent = fm.get_parent();
        let filetype = fm.get_file_type();
        let tags = fm.get_tags();

        // Compare old and new bytes; bail early if unchanged
        let old_data_start =
            self.section_offset[FLST_S as usize] + old_offset as usize + fm.size_bytes();
        let old_data: Vec<u8> =
            self.mmap_mut[old_data_start..old_data_start + old_length].to_vec();
        if old_data == new_data {
            return Ok(false);
        }

        let new_length = new_data.len() as u64;
        let new_fm_size = file_metadata::FileMetadata::calculate_needed_size(
            tags.len() as u16,
            filename.len() as u8,
        );
        let needed = new_fm_size + new_data.len() + file_end_metadata::SIZE_BYTES as usize;

        // Invalidate old block before scanning so it can be reused as free space
        self.mmap_mut[self.section_offset[FLST_S as usize] + old_offset as usize + 4] &= !1;
        self.file_storage_section_size_used -=
            fm.size_bytes() as u64 + old_length as u64 + file_end_metadata::SIZE_BYTES as u64;

        // Scan for a free block large enough for new FM + data + end-metadata
        let mut new_off: Option<usize> = None;
        let mut scan = 4usize; // skip 4-byte section header
        while scan + file_metadata::BASE_SIZE_BYTES < self.file_storage_section_size as usize {
            let scan_start = self.section_offset[FLST_S as usize] + scan;
            let base: Vec<u8> = self.mmap_mut
                [scan_start..scan_start + file_metadata::BASE_SIZE_BYTES]
                .to_vec();
            let scan_fm = file_metadata::FileMetadata::from_bytes(base);
            let block_size = file_metadata::BASE_SIZE_BYTES
                + scan_fm.get_length() as usize
                + file_end_metadata::SIZE_BYTES as usize;
            if !scan_fm.is_valid() && block_size >= needed {
                new_off = Some(scan);
                break;
            }
            scan += block_size;
        }

        if new_off.is_none() {
            self._resize_archive()?;
            let mut scan = 4usize; // skip 4-byte section header
            while scan + file_metadata::BASE_SIZE_BYTES < self.file_storage_section_size as usize {
                let scan_start = self.section_offset[FLST_S as usize] + scan;
                let base: Vec<u8> = self.mmap_mut
                    [scan_start..scan_start + file_metadata::BASE_SIZE_BYTES]
                    .to_vec();
                let scan_fm = file_metadata::FileMetadata::from_bytes(base);
                let block_size = file_metadata::BASE_SIZE_BYTES
                    + scan_fm.get_length() as usize
                    + file_end_metadata::SIZE_BYTES as usize;
                if !scan_fm.is_valid() && block_size >= needed {
                    new_off = Some(scan);
                    break;
                }
                scan += block_size;
            }
            if new_off.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "No file storage space found after resize",
                ));
            }
        }

        let dest = new_off.unwrap();

        // Write new FM, data, and end-metadata
        let new_fm = file_metadata::FileMetadata::new(
            fileno, new_length, true, parent, filetype, &filename, tags,
        );
        let fem = file_end_metadata::FileEndMetadata::new(new_length);
        let fm_start = self.section_offset[FLST_S as usize] + dest;
        self.mmap_mut[fm_start..fm_start + new_fm.size_bytes()]
            .copy_from_slice(&new_fm.as_bytes());
        let data_start = fm_start + new_fm.size_bytes();
        self.mmap_mut[data_start..data_start + new_data.len()].copy_from_slice(&new_data);
        let fem_start = data_start + new_data.len();
        self.mmap_mut[fem_start..fem_start + file_end_metadata::SIZE_BYTES as usize]
            .copy_from_slice(&fem.as_bytes());
        self.file_storage_section_size_used +=
            new_fm.size_bytes() as u64 + new_length + file_end_metadata::SIZE_BYTES as u64;

        // Update FDE: length+valid (bytes 0..5) and offset (bytes 9..14)
        let fde_base = self.section_offset[FLDR_S as usize]
            + 4
            + fileno as usize * file_directory_entry::SIZE_BYTES as usize;
        self.mmap_mut[fde_base..fde_base + 5]
            .copy_from_slice(&((new_length << 1) | 1).to_be_bytes()[3..]);
        self.mmap_mut[fde_base + 9..fde_base + 14]
            .copy_from_slice(&(dest as u64).to_be_bytes()[3..]);

        return Ok(true);
    }

    /**
     * Writes a given number of bytes to a file output stream. Writes in block sizes of 1 MB.
     *
     * @param file the file output stream.
     * @param num_bytes the number of bytes to write.
     */
    fn _write_empty(file: &mut File, num_bytes: u64) -> io::Result<()> {
        let byte_buf: [u8; 1024 * 1024] = [0; 1024 * 1024];
        let mut bytes_written: usize;
        let mut bytes_left = num_bytes;
        while bytes_left > 0 {
            bytes_written = file.write(&byte_buf[0..(1024 * 1024).min(bytes_left as usize)])?;
            bytes_left -= bytes_written as u64;
        }
        Ok(())
    }

    /**
     * Creates an archive file by writing archive metadata to the file. The newly created archive
     * will be empty except for the metadata.
     *
     * @param path the filepath to write to.
     * @param file_dir_slots the number of file directory slots to create.
     * @param tag_dir_slots the number of tag directory slots to create.
     * @param tag_lookup_size the size of the tag lookup section created.
     * @param file_storage_space the size of the file storage section created.
     */
    pub fn create(
        path: String,
        file_dir_slots: u16,
        tag_dir_slots: u16,
        tag_lookup_size: usize,
        file_storage_space: usize,
    ) -> io::Result<NamedFile> {
        let mut file = File::create(&path).unwrap();

        const BUF_SIZE: usize = 1024 * 1024;
        let byte_buf: [u8; BUF_SIZE] = [0; BUF_SIZE];

        // Write section 0
        // Section 0 = 2-byte magic + 4 × 6-byte pointers = 26 bytes.
        // Each header below is 2×u16 = 4 bytes; bit-width values from spec are divided by 8.
        file.write(&MAGIC_NUMBER.to_be_bytes()[2..4])?;
        let mut offset: u64 = (48 * 4 + 16) / 8; // 26 bytes
        file.write(&offset.to_be_bytes()[2..8])?;
        offset += 4 + file_directory_entry::SIZE_BYTES as u64 * file_dir_slots as u64;
        file.write(&offset.to_be_bytes()[2..8])?;
        offset += 4 + tag_directory_entry::SIZE_BYTES as u64 * tag_dir_slots as u64;
        file.write(&offset.to_be_bytes()[2..8])?;
        offset += 4 + tag_lookup_size as u64; // section 3 header = 2×u16 = 4 bytes
        file.write(&offset.to_be_bytes()[2..8])?;

        // Write section 1
        file.write(&file_dir_slots.to_be_bytes())?;
        file.write(&0u16.to_be_bytes())?;
        for _ in 0..file_dir_slots {
            file.write(&byte_buf[0..file_directory_entry::SIZE_BYTES as usize])?;
        }

        // Write section 2
        file.write(&tag_dir_slots.to_be_bytes())?;
        file.write(&0u16.to_be_bytes())?;
        for _ in 0..tag_dir_slots {
            file.write(&byte_buf[0..tag_directory_entry::SIZE_BYTES as usize])?;
        }

        // Write section 3
        file.write(&(tag_lookup_size as u16).to_be_bytes())?;
        file.write(&0u16.to_be_bytes())?;
        let mut bytes_left = tag_lookup_size as usize;
        while bytes_left > 0 {
            let bytes_written = file.write(&byte_buf[0..bytes_left.min(BUF_SIZE)])?;
            bytes_left -= bytes_written;
        }

        // Write section 4 (4-byte header + initial free FM block + data region + FEM)
        file.write(&[0u8; 4])?;
        let file_length = (file_storage_space
            - 4
            - file_metadata::FileMetadata::calculate_needed_size(0, 0)
            - file_end_metadata::SIZE_BYTES as usize) as u64;
        let init_fm = file_metadata::FileMetadata::new(0, file_length, false, 0, 0, "", vec![]);
        let init_fem = file_end_metadata::FileEndMetadata::new(file_length);
        file.write(&init_fm.as_bytes())?;
        let mut space_left = file_length as usize;
        while space_left > 0 {
            let bytes_written = file.write(&byte_buf[0..space_left.min(BUF_SIZE)])?;
            space_left -= bytes_written;
        }
        file.write(&init_fem.as_bytes())?;

        file.flush()?;
        return Ok(NamedFile::new(
            OpenOptions::new().read(true).write(true).open(path.clone())?,
            path,
        ));
    }
}
