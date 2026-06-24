# Commands to Implement

Sub-points indicate optional arguments

- `open` opens a file with system viewer
  - `-t <tag1> ...` open only files with the given tags
  - `-f <file1> ...` open only files with the given names
- `apply <script>` applies a python script to a set of files
  - `-t <tag1> ...` apply only to files with the given tags
  - `-f <file1> ...` apply only to files with the given names
- `expand <destination>` decompresses the `.dat` storage file into a hierarchical directory structure located at the destination directory
  - `-f <file>` decompresses a given `.dat` storage file instead of the application's working storage file
- `reduce <file1> ...` compresses a set of files into a `.dat` file
  - `-r` recursively compress, will not compress directories without
- `import <file1> ...` imports a set of files into the application's `.dat` storage file
  - `-r` recursively import, will not import directories without
- `merge <file>` merges a `.dat` file into the application's working `.dat` storage file
- `scrape` scrapes the webpage corresponding to a link file and caches the results
  - `-t <tag1> ...` scrape only files with the given tags
  - `-f <file1> ...` scrape only files with the given names
- `destroy` removes the set of files from the temporary application cache, deleting any updates
  - `-t <tag1> ...` apply only to files with the given tags
  - `-f <file1> ...` apply only to files with the given names
  - `-a` destroys all files that have been updated (overrides `-f`, `-t`)
- `quit` quits the application
- `help` shows help text
- `config <key> <value>` sets a key and value for the application configuration
  - `-p` persists the key and value across application sessions
  - `-l` lists all config key-value pairs
- `flush` writes updates from a set of currently open files to the `.dat` storage file
  - `-t <tag1> ...` apply only to files with the given tags
  - `-f <file1> ...` apply only to files with the given names
  - `-a` writes all files that have been updated (overrides `-f`, `-t`)
  - `-d` destroys the cached files after flushing all updates
- `ls <tag1> ...` lists all files with the given set of tags
- `sz <tag1> ...` lists the combined size of all files with given tags
- `remove -f <file1> ... -t <tag1> ...` removes the files with the given filenames and tags from the `.dat` storage file
- `tag -f <file1> ... -t <tag1> ...` adds a set of tags to a set of files
  - `-d` removes the set of tags from a set of files

# Format of Storage File

- `.dat` file extension

### Zeroth Section - Section Pointers

- first value is 16-bit magic number 13579
- (k+1)th value is a 48-bit byte-granular offset from the start of this section to section k
- section length is 208 bits

### First Section - File Directory

- first value is 16-bit unsigned short representing number of file slots in directory (max 65536)
- second value is 16-bit unsigned short representing number of file slots used in directory
- file slots in directory must be filled contiguously
- file slot is a 112-bit tuple `(l, v, p, n, o)`
  - `l` is the 39-bit length of the file
  - `v` is a valid bit, 1 if valid
  - `p` is the 16-bit index of the parent in the file directory (TODO remove - only use tags)
    - since could be n^2 time otherwise, try to always keep parent before child
  - `n` is the 16-bit hash of the file name
  - `o` is the 40-bit offset from the start of section 4 to the start of the file

### Second Section - Tag Directory

- first value is 16-bit unsigned short representing number of tag slots in directory (max 32k)
- second value is 16-bit unsigned short representing number of tag slots used in directory
- tag slots in directory must be filled contiguously
- rest of values are tag slot 184-bit tuple `(i, v, t, o)`
  - `v` is a valid bit, 1 if valid
  - `i` is a 15-bit unsigned short identifying the tag
  - `t` is a 16-byte string of the tag name
  - `o` is the 40-bit offset from the start of the tag lookup section to first tag lookup tuple

### Third Section - Tag Lookup

- first value is 16-bit unsigned integer representing the size of the section in bytes
- second value is 16-bit unsigned integer representing number of tuples in section
- list of (88+16k)-bit tuples `(i, v, s, n, o, f1, f2, ...)`
  - `v` is a valid bit, 1 if the tuple is valid
  - `i` is a 15-bit unsigned byte identifying the tag (corresponding to section 2)
  - `s` is an 16-bit number representing the number of file slots in the tuple
  - `n` is a 16-bit unsigned short representing the number of files with the given tag
    - `n` should represent valid `f` entries in this tuple alone, plus 1 if the next pointer is valid
  - `f` is a 16-bit unsigned short representing the index of a file with the tag
    - first tuples for tags have 15 file pointers, then 31, then 63, ... (only for valid tuples, invalid can have any number)
  - `o` is a 40-bit offset from the start of this section to the next tuple for this tag

### Fourth Section - File Storage

- each file is represented by a metadata file and then the data and then end-metadata
- metadata file is k-bit tuple `(l, v, f, p, y, nn, (tn, ti1, ti2, ...), n)`
  - `l` is a 39-bit unsigned integer representing the length of the data
  - `v` is a valid bit, 1 if this file is valid
  - `f` is a 16-bit unsigned short representing the index of a file
  - `p` is a 16-bit index of the parent in the file directory
  - `y` is a 8-bit number representing the file type
  - `nn` is the 8-bit number representing the length of the file name
  - `tn` is a 16-bit unsigned short representing the number of tags corresponding to the file
  - `ti` is the 16-bit unsigned short identifying the tag
  - `n` is the `nn` byte length string representing the name of the file
- data is arbitrary length
- end-metadata is 40-bit unsigned integer representing length of data

# Scale-Up Format Changes (100k files / 100 GB / 8.4M tags)

The following are breaking binary format changes that must all be made together in a single coordinated migration.

### Section 1 — File Directory

- [x] Widen `num_file_dir_slots` and `num_file_dir_slots_used` in the section header from `u16` (2 bytes each) to `u32` (4 bytes each). Section header grows from 4 → 8 bytes. (`DIR_SECTION_HEADER_BYTES = 8`)
- [x] Widen the `p` (parent index) field in FDE from 16 bits → 32 bits. FDE grows from 112 bits (14 bytes) → 128 bits (16 bytes).
- [x] Update `MAX_FILE_DIR_SLOTS` constant from `u16::MAX` to a `u32` value ≥ 100,000.
- [x] Update all code that reads/writes the Section 1 header to use `u32::from_be_bytes` / `to_be_bytes`.
- [x] Update `file_directory_entry::SIZE_BYTES` from `u8` (14) to `usize` (16); update all slice arithmetic using this constant.
- [x] In `_resize_archive`, update the stride calculation for copying FDE data and the padding fill for new slots.

### Section 2 — Tag Directory

- [x] Widen `num_tag_dir_slots` and `num_tag_dir_slots_used` in the section header from `u16` → `u32`. Section header grows from 4 → 8 bytes.
- [x] Widen tag name field `t` in TDE from 16 bytes → 32 bytes (128 → 256 bits).
- [x] Widen tag ID field `i` in TDE from 15 bits → 23 bits. TDE grows from 184 bits (23 bytes) → 320 bits (40 bytes).
- [x] Update `MAX_TAG_DIR_SLOTS` from `u16::MAX` to `(1 << 23) - 1` as `u32`.
- [x] Update `tag_directory_entry::SIZE_BYTES` from 23 → 40 and fix all slice arithmetic.
- [x] Update `TagDirectoryEntry::get_tagno()` bit-shift logic for 23-bit field.
- [x] Update tag name read/write in `TagDirectoryEntry` to use a 32-byte buffer.
- [x] In `_resize_archive`, update the TDE stride and padding fill.

### Section 3 — Tag Lookup

- [x] Widen `tag_lookup_section_size` and `tag_lookup_section_size_used` in the section header from `u16` → `u32`. Widen `num_tag_lookup_tuples` from `u16` → `u32`. Section header grows from 4 → 8 bytes.
- [x] Update all in-memory fields on `Archive` struct from `u16` to `u32`.
- [x] Widen tag ID field `i` in TLE base from 15 bits → 23 bits. TLE base grows from 88 bits (11 bytes) → 96 bits (12 bytes). `BASE_SIZE_BYTES = 12`.
- [x] Widen each file pointer `fi` in TLE payload from 16 bits (2 bytes) → 32 bits (4 bytes). `FILE_SLOT_SIZE = 4`.
- [x] Update `tag_lookup_entry::MIN_SIZE_BYTES` to reflect new base and per-slot sizes.
- [x] Update `_make_tle`, `_coalesce_tglk`, and all TLE scan loops for new base size and 4-byte file pointer stride.
- [x] In `_resize_archive`, update the tag lookup section copy and arithmetic.

### Section 4 — File Storage

- [x] Widen `f` (file index) and `p` (parent index) fields in FM base from 16 bits → 32 bits each. FM base grows from 104 bits (13 bytes) → 136 bits (17 bytes). `BASE_SIZE_BYTES = 17`.
- [x] Widen each per-file tag ID entry `ti` in FM from 16 bits (2 bytes) → 24 bits (3 bytes). `TAG_SLOT_SIZE = 3`.
- [x] Update `get_fm`, `_find_file_space`, `_allocate_file_space`, and all Section 4 scan loops for the new FM base size.
- [x] `FEM` (40-bit end-metadata) is unchanged.

### Archive struct / archive_manager

- [x] Change `num_file_dir_slots` and `num_file_dir_slots_used` fields on `Archive` from `u16` → `u32`.
- [x] Change `num_tag_dir_slots` and `num_tag_dir_slots_used` fields on `Archive` from `u16` → `u32`.
- [x] Widen `fileno`/`tagno` parameters from `u16` → `u32` in all public and private methods.
- [x] Update `_read_section_pointers` and `_read_s1_meta`/`_read_s2_meta`/`_read_s3_meta` for widened headers.
- [x] Update `create_archive_file` initial slot counts to use `u32` writes.
- [x] Fixed `_resize_archive`: uses `SeekFrom::Start` for correct seek positions, remaps mmap after rename, uses `OpenOptions` read+write, fixed all u32 arithmetic.

### Migration

- [ ] Write a one-shot migration tool (or `migrate` command) that reads a v1 `.dat` file (old field widths) and rewrites it as a v2 `.dat` file with the new layout.
- [ ] Bump the magic number (currently `13579`) or add a format version field to the Section 0 header so old and new archives are distinguishable.

# Thoughts

- on file remove, move file directory entry and file entry
  - removals are rare
- file storage does not need as much space as there are slots in file directory?
  - can we afford to have large file metadata (duplicate id, path, tags)?
- how do we identify blank spots in the tag lookup list (i, n == 0)
- how do we defragment tag list
  - make sure to set all bits to 0 on deleting a tag
  - when adding files to tag, might have to move to end? no
- how do we defragment file storage
  - coalesce unfilled sections immediately
    - set valid bit to 0 on removal
    - removal is rare
    - merge using next's metadata and previous' end-metadata
- accessing path is rare - we can afford to do lots of random accesses
  - only used for writing back to hierarchical format
- use java BitSet
