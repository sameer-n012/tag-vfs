use memmap2::{Mmap, MmapMut};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::archive::{file_directory_entry, file_end_metadata, file_metadata, tag_directory_entry};
use crate::util::named_file::NamedFile;

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
    section_offset: Vec<u64>, // should be an array of length 5, with first value set to 0
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
    tag_lookup_section_size: u64, // includes metadata
    tag_lookup_section_size_used: u64,
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

        if !a.validate_file_type()? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is not a valid archive file",
            ));
        }

        a.read_section_pointers()?;
        a.read_s1_meta()?;
        a.read_s2_meta()?;
        a.read_s3_meta()?;
        a.read_s4_meta()?;

        return Ok(a);
    }

    /**
     * Copies the archive file to a backup with name given by Archive.ARCHIVE_BACKUP_FILENAME preceded
     * by a number.
     *
     * @return the backup file.
     */
    fn backup_archive(&mut self) -> io::Result<()> {
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
    fn resize_archive(&mut self) -> io::Result<()> {
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
                new_tag_lookup_section_size = self.tag_lookup_section_size * RESIZE_FACTOR as u64;
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
            offset += 32 + 16 + new_tag_lookup_section_size;
            new_file.write(&offset.to_be_bytes()[2..8])?;

            const BUF_SIZE: usize = 1024 * 1024;
            let mut byte_buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
            let mut bytes_read: usize;

            // write section 1
            new_file.write(&new_num_file_dir_slots.to_be_bytes())?;
            self.file.seek(SeekFrom::Current(2));
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
            Archive::write_empty(
                &mut new_file,
                (new_num_file_dir_slots - self.num_file_dir_slots) as u64
                    * file_directory_entry::SIZE_BYTES as u64,
            )?;

            // write section 2
            new_file.write(&new_num_tag_dir_slots.to_be_bytes())?;
            self.file.seek(SeekFrom::Current(2));
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
            Archive::write_empty(
                &mut new_file,
                (new_num_tag_dir_slots - self.num_tag_dir_slots) as u64
                    * tag_directory_entry::SIZE_BYTES as u64,
            )?;

            // write section 3
            new_file.write(&new_tag_lookup_section_size.to_be_bytes())?;
            self.file.seek(SeekFrom::Current(8));
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
                bytes_left -= bytes_read as u64;
            }
            Archive::write_empty(
                &mut new_file,
                new_tag_lookup_section_size - self.tag_lookup_section_size,
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
            Archive::write_empty(&mut new_file, file_length);
            // FileEndMetadata fme = new FileEndMetadata(fileLength);
            new_file.write(&[0; 5])?;

            new_file.flush()?;
            self.file.rewind()?;

            fs::remove_file(&self.fpath)?;
            fs::rename(new_path, &self.fpath)?;
            self.file = File::open(&self.fpath)?;
        }

        self.read_section_pointers()?;
        self.read_s1_meta()?;
        self.read_s2_meta()?;
        self.read_s3_meta()?;
        self.read_s4_meta()?;

        Ok(())
    }

    /**
     * Validates that the file given is an archive file for this application using the magic number.
     *
     * @return true if the file is valid and false otherwise.
     */
    fn validate_file_type(&self) -> io::Result<bool> {
        self.head_l.read().unwrap();

        if (u16::from_be_bytes(self.mmap[0..2].try_into().unwrap()) != MAGIC_NUMBER as u16) {
            return Ok(false);
        }
        Ok(true)
    }

    /**
     * Reads the pointers to each section found in the archive header.
     *
     */
    fn read_section_pointers(&mut self) -> io::Result<()> {
        // Read section pointers

        self.head_l.write().unwrap();

        for i in 1..NUMBER_SECTIONS {
            self.section_offset[i as usize] = u64::from_be_bytes(
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
    fn read_s1_meta(&mut self) -> io::Result<()> {
        self.fldr_l.write().unwrap();

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
    fn read_s2_meta(&mut self) -> io::Result<()> {
        Ok(())
    }

    /**
     * Reads the metadata found in the tag lookup section including
     * total section size, space used, and number of lookup tuples.
     *
     */
    fn read_s3_meta(&mut self) -> io::Result<()> {
        Ok(())
    }

    /**
     * Reads the metadata found in the file storage section including
     * total section size.
     *
     */
    fn read_s4_meta(&mut self) -> io::Result<()> {
        self.flst_l.write().unwrap();
        self.file_storage_section_size =
            self.mmap.len() as u64 - self.section_offset[FLST_S as usize];

        Ok(())
    }

    /**
     * Writes a given number of bytes to a file output stream. Writes in block sizes of 1 MB.
     *
     * @param file the file output stream.
     * @param num_bytes the number of bytes to write.
     */
    fn write_empty(file: &mut File, num_bytes: u64) -> io::Result<()> {
        let byte_buf: [u8; 1024 * 1024] = [0; 1024 * 1024];
        let mut bytes_written: usize;
        let mut bytes_left = num_bytes;
        while bytes_left > 0 {
            bytes_written = file.write(&byte_buf[0..(1024 * 1024).min(bytes_left as usize)])?;
            bytes_left -= bytes_written as u64;
        }
        Ok(())
    }
}
