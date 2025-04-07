use memmap2::{Mmap, MmapMut};
use std::fs::{self, File};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::archive::{
    file_directory_entry, file_end_metadata, file_metadata, tag_directory_entry, tag_lookup_entry,
};
use crate::util::named_file::NamedFile;

use super::file_metadata::FileMetadata;
use super::tag_lookup_entry::BASE_SIZE_BYTES;

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

const MAX_FILE_DIR_SLOTS: u16 = 1 << 16 - 1;
const MAX_TAG_DIR_SLOTS: u16 = 1 << 16 - 1;

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

    mmap: Mmap,
    mmap_mut: MmapMut,

    num_file_dir_slots: u16,
    num_file_dir_slots_used: u16,
    // fldr_mbb: Option<MappedByteBuffer>,
    num_tag_dir_slots: u16,
    num_tag_dir_slots_used: u16,
    // tgdr_mbb: Option<MappedByteBuffer>,
    tag_lookup_section_size: u16, // includes metadata
    tag_lookup_section_size_used: u16,
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

            mmap: unsafe { Mmap::map(&file.file.try_clone().unwrap())? },
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

            // write section 0
            new_file.write(&(MAGIC_NUMBER as u16).to_be_bytes())?;
            let mut offset: u64 = 48 * 4 + 16;
            new_file.write(&offset.to_be_bytes()[2..8])?;
            offset +=
                16 * 2 + file_directory_entry::SIZE_BYTES as u64 * new_num_file_dir_slots as u64;
            new_file.write(&offset.to_be_bytes()[2..8])?;
            offset +=
                16 * 2 + tag_directory_entry::SIZE_BYTES as u64 * new_num_tag_dir_slots as u64;
            new_file.write(&offset.to_be_bytes()[2..8])?;
            offset += 32 + 16 + new_tag_lookup_section_size as u64;
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
                if bytes_read <= 0 {
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
                if bytes_read <= 0 {
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
            self.file.seek(SeekFrom::Current(8))?;
            let mut bytes_left = self.tag_lookup_section_size + 8;
            loop {
                bytes_read = self
                    .file
                    .read(&mut byte_buf[0..BUF_SIZE.min(bytes_left as usize)])
                    .unwrap();
                if bytes_read <= 0 {
                    break;
                }
                new_file.write(&byte_buf[0..bytes_read])?;
                bytes_left -= bytes_read as u16;
            }
            Archive::_write_empty(
                &mut new_file,
                (new_tag_lookup_section_size - self.tag_lookup_section_size) as u64,
            )?;

            // write section 3
            let mut bytes_left = self.file_storage_section_size;
            loop {
                bytes_read = self
                    .file
                    .read(&mut byte_buf[0..BUF_SIZE.min(bytes_left as usize)])
                    .unwrap();
                if bytes_read <= 0 {
                    break;
                }
                new_file.write(&byte_buf[0..bytes_read])?;
                bytes_left -= bytes_read as u64;
            }
            let file_length: u64 = new_file_storage_section_size
                - self.file_storage_section_size
                - file_metadata::MIN_SIZE_BYTES as u64
                - file_end_metadata::SIZE_BYTES as u64;

            // FileMetadata fm = new FileMetadata(fileLength,
            //         false, (short) -1, (short) -1, (byte) 0, null, null);
            new_file.write(&[0; 1024])?;
            Archive::_write_empty(&mut new_file, file_length)?;
            // FileEndMetadata fme = new FileEndMetadata(fileLength);
            new_file.write(&[0; 5])?;

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

        if (u16::from_be_bytes(self.mmap[0..2].try_into().unwrap()) != MAGIC_NUMBER as u16) {
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
            self.section_offset[i as usize] = usize::from_be_bytes(
                self.mmap[((i as usize - 1) * 6)..(i as usize * 6)]
                    .try_into()
                    .unwrap(),
            );
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
            self.mmap[self.section_offset[FLDR_S as usize] as usize
                ..(self.section_offset[FLDR_S as usize] as usize + 2)]
                .try_into()
                .unwrap(),
        );

        self.num_file_dir_slots_used = u16::from_be_bytes(
            self.mmap[self.section_offset[FLDR_S as usize] as usize + 2
                ..(self.section_offset[FLDR_S as usize] as usize + 4)]
                .try_into()
                .unwrap(),
        );

        let mut bytes_read: usize = 0;
        let mut files_found: u16 = 0;
        let mut space_used: u64 = 0;
        let mut buffer: u64;
        while (bytes_read
            < self.num_file_dir_slots as usize * file_directory_entry::SIZE_BYTES as usize
            && files_found < self.num_file_dir_slots_used)
        {
            buffer = u64::from_be_bytes(
                self.mmap[self.section_offset[FLDR_S as usize] as usize + 4 + bytes_read
                    ..self.section_offset[FLDR_S as usize] as usize + 4 + bytes_read + 8]
                    .try_into()
                    .unwrap(),
            );
            if buffer % 2 == 1 {
                space_used += buffer >> 1;
                files_found += 1;
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
            self.mmap
                [self.section_offset[TGDR_S as usize]..(self.section_offset[TGDR_S as usize] + 2)]
                .try_into()
                .unwrap(),
        );

        self.num_tag_dir_slots_used = u16::from_be_bytes(
            self.mmap[self.section_offset[TGDR_S as usize] + 2
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
            self.mmap
                [self.section_offset[TGLK_S as usize]..(self.section_offset[TGLK_S as usize] + 2)]
                .try_into()
                .unwrap(),
        );

        self.num_tag_lookup_tuples = u16::from_be_bytes(
            self.mmap[self.section_offset[TGLK_S as usize] + 2
                ..(self.section_offset[TGLK_S as usize] + 4)]
                .try_into()
                .unwrap(),
        );

        let mut bytes_read: usize = 0;
        let mut tuples_found: u16 = 0;
        let mut num_file_slots: u16;
        let mut space_used: usize = 0;

        while bytes_read < self.tag_lookup_section_size as usize
            && tuples_found < self.num_tag_lookup_tuples
        {
            if (self.mmap[self.section_offset[TGLK_S as usize] + 4 + bytes_read] & 0x80 != 0) {
                num_file_slots = u16::from_be_bytes(
                    self.mmap[self.section_offset[TGLK_S as usize] + 4 + bytes_read + 1
                        ..self.section_offset[TGLK_S as usize] + 4 + bytes_read + 3]
                        .try_into()
                        .unwrap(),
                );
                space_used += (2 + 1 + 2 + 2 * num_file_slots + 5) as usize;
                bytes_read += (2 + 1 + 2 + 2 * num_file_slots + 5) as usize;
                tuples_found += 1;
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
            (self.mmap.len() - self.section_offset[FLST_S as usize]) as u64;

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

        let buf: [u8; file_directory_entry::SIZE_BYTES as usize] = self.mmap[self.section_offset
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
            buf = self.mmap[self.section_offset[FLDR_S as usize]
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

        let buf: [u8; tag_directory_entry::SIZE_BYTES as usize] = self.mmap[self.section_offset
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
            buf = self.mmap[self.section_offset[FLDR_S as usize]
                + 4
                + i * file_directory_entry::SIZE_BYTES as usize
                ..self.section_offset[FLDR_S as usize]
                    + 4
                    + (i + 1) * file_directory_entry::SIZE_BYTES as usize]
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
        let mut buf: Vec<u8> = self.mmap[self.section_offset[FLST_S as usize] + offset as usize
            ..self.section_offset[FLST_S as usize]
                + offset as usize
                + file_metadata::MIN_SIZE_BYTES as usize]
            .to_vec();

        let name_len = buf[10] as usize;
        let num_tags = u16::from_be_bytes(buf[11..13].try_into().unwrap()) as usize;

        buf.extend_from_slice(
            &self.mmap[self.section_offset[FLST_S as usize]
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
            buf = self.mmap[self.section_offset[FLDR_S as usize]
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
            buf = self.mmap[self.section_offset[TGDR_S as usize]
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
    fn _make_tle(
        &mut self,
        tagno: u16,
        filenos: Vec<u16>,
    ) -> io::Result<tag_lookup_entry::TagLookupEntry> {
        let num_file_slots = (filenos.len() + 1).next_power_of_two() as u16;

        let mut need_resize: bool = true;
        let mut offset: u64 = 0;
        let mut prev_exists: bool = false;
        let mut prev_offset: u64 = 0;
        let mut prev_num_file_slots: u16 = 7;
        {
            let lock = self.tglk_l.read().unwrap();

            // loop through total section based on size of tag lookup section
            // to find space
            let mut bytes_read: usize = 0;
            while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES
                < self.tag_lookup_section_size as usize
            {
                let buf = self.mmap[self.section_offset[TGLK_S as usize] + 4 + bytes_read
                    ..self.section_offset[TGLK_S as usize]
                        + 4
                        + bytes_read
                        + tag_lookup_entry::BASE_SIZE_BYTES]
                    .try_into()
                    .unwrap();

                let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf);

                if tle.is_valid() && tle.tagno() == tagno {
                    prev_exists = true;
                    prev_offset = bytes_read as u64;
                    prev_num_file_slots = tle.get_num_file_slots();
                }

                if !tle.is_valid() && tle.get_num_file_slots() >= num_file_slots {
                    offset = bytes_read as u64;
                    need_resize = false;
                    break;
                }

                bytes_read += BASE_SIZE_BYTES + tle.get_num_file_slots() as usize * 2;
            }
        }
        if need_resize {
            self._resize_archive()?;
            {
                let lock = self.tglk_l.read().unwrap();

                // loop through total section based on size of tag lookup section
                // to find space
                let mut bytes_read: usize = 0;
                while bytes_read + tag_lookup_entry::BASE_SIZE_BYTES
                    < self.tag_lookup_section_size as usize
                {
                    let buf = self.mmap[self.section_offset[TGLK_S as usize] + 4 + bytes_read
                        ..self.section_offset[TGLK_S as usize]
                            + 4
                            + bytes_read
                            + tag_lookup_entry::BASE_SIZE_BYTES]
                        .try_into()
                        .unwrap();

                    let tle = tag_lookup_entry::TagLookupEntry::from_bytes(buf);

                    if tle.is_valid() && tle.tagno() == tagno && !tle.is_offset_valid() {
                        prev_exists = true;
                        prev_offset = bytes_read as u64;
                        prev_num_file_slots = tle.get_num_file_slots();
                    }

                    if !tle.is_valid() && tle.get_num_file_slots() >= num_file_slots {
                        offset = bytes_read as u64;
                        need_resize = false;
                        break;
                    }

                    bytes_read += BASE_SIZE_BYTES + tle.get_num_file_slots() as usize * 2;
                }

                if need_resize {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "No tag lookup entry space found",
                    ));
                }
            }
        }

        let lock = self.tglk_l.write().unwrap();

        let buf: Vec<u8> = Vec::with_capacity(
            tag_lookup_entry::TagLookupEntry::calculate_needed_size(num_file_slots),
        );

        let new_num_file_slots = prev_num_file_slots * 2 + 1;
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

        // update previous tle's next offset
        if prev_exists {
            let mut buf: [u8; 5] = [0; 5];
            buf.copy_from_slice(&offset.to_be_bytes()[3..]);
            self.mmap_mut[self.section_offset[TGLK_S as usize] + 4 + prev_offset as usize + 6
                ..self.section_offset[TGLK_S as usize] + 4 + prev_offset as usize + 11]
                .copy_from_slice(&buf);
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

        let mut buf: [u8; 2] = self.mmap[self.section_offset[TGLK_S as usize] + 4 + offset as usize
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

        let mut bytes_read: usize = 0;
        let mut buf: [u8; 8] = [0; 8];
        while (bytes_read + 8 < self.file_storage_section_size as usize) {
            buf[3..8].copy_from_slice(
                self.mmap[self.section_offset[FLST_S as usize] + bytes_read
                    ..self.section_offset[FLST_S as usize] + bytes_read + 5]
                    .try_into()
                    .unwrap(),
            );

            let val = u64::from_be_bytes(buf);
            if val % 2 == 0 && (val >> 1) >= space_needed {
                return Ok(bytes_read as u64);
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

    pub fn _coalesce_tglk(&mut self) -> io::Result<()> {
        return Ok(());
    }

    pub fn _coalesce_tglk_around(&mut self, offset: u64) -> io::Result<()> {

        let l1 = self.tglk_l.write().unwrap();
        let l2 = self.tgdr_l.write().unwrap();



        return Ok(());
    }

    pub fn _coalesce_flst(&mut self) -> io::Result<()> {
        return Ok(());
    }

    pub fn _coalesce_flst_around(&mut self, offset: u64) -> io::Result<()> {
        return Ok(());
    }

    pub fn add_file(&mut self) -> io::Result<()> {
        return Ok(());
    }

    pub fn add_tag(&mut self) -> io::Result<()> {
        return Ok(());
    }

    pub fn remove_file(&mut self) -> io::Result<()> {
        return Ok(());
    }

    pub fn remove_tag(&mut self) -> io::Result<()> {
        return Ok(());
    }

    pub fn read_file(&mut self) -> io::Result<()> {
        return Ok(());
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
        file.write(&MAGIC_NUMBER.to_be_bytes()[2..4])?;
        let mut offset: u64 = 48 * 4 + 16;
        file.write(&offset.to_be_bytes()[2..8])?;
        offset += 16 * 2 + file_directory_entry::SIZE_BYTES as u64 * file_dir_slots as u64;
        file.write(&offset.to_be_bytes()[2..8])?;
        offset += 16 * 2 + tag_directory_entry::SIZE_BYTES as u64 * tag_dir_slots as u64;
        file.write(&offset.to_be_bytes()[2..8])?;
        offset += 32 + 16 + tag_lookup_size as u64;
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
        file.write(&tag_lookup_size.to_be_bytes())?;
        file.write(&0u16.to_be_bytes())?;
        let mut bytes_left = tag_lookup_size as usize;
        while bytes_left > 0 {
            let bytes_written = file.write(&byte_buf[0..bytes_left.min(BUF_SIZE)])?;
            bytes_left -= bytes_written;
        }

        // Write section 4
        let file_length = file_storage_space
            - file_metadata::MIN_SIZE_BYTES as usize
            - file_end_metadata::SIZE_BYTES as usize;
        file.write(&(file_length as u64).to_be_bytes())?;
        let mut space_left = file_length as usize;
        while space_left > 0 {
            let bytes_written = file.write(&byte_buf[0..space_left.min(BUF_SIZE)])?;
            space_left -= bytes_written;
        }
        file.write(&(file_length as u64).to_be_bytes())?;

        file.flush()?;
        return Ok(NamedFile::new(File::open(path.clone())?, path));
    }
}
