# tag-vfs — Rust Implementation Guide

## Project Overview

`tag-vfs` (a.k.a. `file-vault`) is a tag-based virtual file system that stores files in a custom binary `.dat` archive format. Files are organized by tags rather than directories, and the archive is a single flat file with five contiguous binary sections. The `java/` folder is an old, incomplete reference implementation; ignore it.

## Architecture

```
src/
  main.rs                     - entry point
  lib.rs
  app/
    run_configuration.rs      - config map, path helpers, CLI arg parsing
    app.rs                    - top-level App struct; sets up directories and archive
    command_line_app.rs       - REPL loop; calls ArchiveManager methods per command
  archive/
    archive.rs                - Archive struct; all low-level binary I/O via mmap
    archive_manager.rs        - ArchiveManager; wraps Archive with higher-level ops
    file_directory_entry.rs   - FDE binary encoding/decoding (112 bits)
    tag_directory_entry.rs    - TDE binary encoding/decoding (184 bits)
    tag_lookup_entry.rs       - TLE binary encoding/decoding (variable width)
    file_metadata.rs          - FileMetadata binary encoding/decoding (variable width)
    file_end_metadata.rs      - FileEndMetadata binary encoding/decoding (40 bits)
  data/
    file_instance.rs          - FileInstance domain type (name, type, tags, path)
    file_type.rs              - FileType enum (DIR / FILE / UNK)
  util/
    named_file.rs             - NamedFile (File + path string)
  exceptions/
    config_parse_error.rs     - ConfigParseError
```

All file I/O in `Archive` goes through a `memmap2::Mmap` (read) / `MmapMut` (write) pair. Never use `self.file.read()` directly for data reads; use the mmap slices. Seeks on `self.file` are only used during archive copy in `_resize_archive` and `_backup_archive`.

## `.dat` Binary Format

Section layout (see TODO.md for full field-level spec):

| # | Name            | Key constants                                    |
|---|-----------------|--------------------------------------------------|
| 0 | Header          | 16-bit magic (13579) + 4×48-bit section offsets  |
| 1 | File Directory  | 2×u16 counts + N×FDE (112 bits each)             |
| 2 | Tag Directory   | 2×u16 counts + N×TDE (184 bits each)             |
| 3 | Tag Lookup      | 2×u16 meta + variable-width TLEs                 |
| 4 | File Storage    | variable-width FileMetadata + data + FileEndMeta |

All multi-byte integers are **big-endian**. Many fields use non-byte-aligned widths (e.g., 39-bit lengths, 40-bit offsets, 15-bit tag IDs) packed into byte arrays with manual bit-shifting. Always read/write these with the pattern `buf[3..8].copy_from_slice(...)` / `u64::from_be_bytes(buf) >> 1` already established in the code.

Section index constants in `archive.rs`:
```rust
const HEAD_S: u8 = 0;
const FLDR_S: u8 = 1;
const TGDR_S: u8 = 2;
const TGLK_S: u8 = 3;
const FLST_S: u8 = 4;
```

## Coding Style Rules

These rules reflect the style the author uses. Follow them exactly.

### Comment style
Use Javadoc-style block comments for all public and private methods:
```rust
/**
 * One-sentence description of what this does.
 *
 * @param foo description of foo.
 * @param bar description of bar.
 * @return description of return value.
 */
```
Keep comments at this exact length and phrasing. Do not use `///` doc comments or `//` inline comments except for very short notes inside method bodies. Never write multi-paragraph comments.

### Function skeletons
The codebase has stub methods (e.g., `remove_file`, `remove_tag`, `remove`, `import_files`, `add_tags`, etc.) that return `Ok(())` as placeholders. When implementing them, keep the existing signature, fill in the body, and keep any existing block comment above the function.

### Error handling
- Return `io::Result<T>` everywhere in `archive/`.
- Return descriptive `io::Error::new(io::ErrorKind::..., "message")` values.
- Never use `unwrap()` in public methods; use `?` propagation.
- Avoid `expect()` outside of initialization code.

### Explicit returns
Use `return Ok(...)` and `return Err(...)` at the end of functions, not bare expression returns. This is consistent with the existing style.

### Naming
- Private/internal archive methods: prefix with `_` (e.g., `_make_fde`, `_resize_archive`).
- Section-level locks: `head_l`, `fldr_l`, `tgdr_l`, `tglk_l`, `flst_l`.
- Binary entry types: `FDE` (file dir entry), `TDE` (tag dir entry), `TLE` (tag lookup entry), `FM` (file metadata), `FEM` (file end-metadata).
- Size constants per entry type: `SIZE_BYTES`, `BASE_SIZE_BYTES`, `MIN_SIZE_BYTES`.

### Locking pattern
Acquire the section's `RwLock` at the start of every method that touches that section. Read operations take `.read().unwrap()`, write operations take `.write().unwrap()`. Do not hold multiple locks across an internal method call that also locks — resolve by splitting into a read phase and a write phase (see `_make_fde` as the reference).

### Vec initialization with known layout
When constructing binary entry types, build a `Vec<u8>` with `Vec::with_capacity(n)` then push/extend fields in order — do **not** index into it before extending (the vec is zero-length after `with_capacity`). Use `extend_from_slice` for each field region. Example pattern from `TagLookupEntry::new` (current code has a bug here — see Known Bugs below).

## Known Bugs — All Fixed

1. ~~`FileMetadata::new` and `TagLookupEntry::new` — index into empty Vec~~ **Fixed.** Both now use sequential `extend_from_slice`/`push` to build the byte layout. `calculate_needed_size` in `TagLookupEntry` corrected to use `BASE_SIZE_BYTES` (not `MIN_SIZE_BYTES`). `is_offset_valid` corrected to compare the raw stored value against `num_file_slots` instead of calling `get_num_files()` which already subtracts the overflow bit.
2. ~~`_read_section_pointers` — wrong byte offsets and wrong slice length~~ **Fixed.** Now pads a 6-byte slice into an 8-byte buffer before calling `usize::from_be_bytes`, and starts the offset scan at byte 2 (after the magic number).
3. ~~`MAX_FILE_DIR_SLOTS` / `MAX_TAG_DIR_SLOTS` — operator precedence~~ **Fixed.** Both replaced with `u16::MAX`.
4. ~~`TagDirectoryEntry::get_tagno()` — double right-shift~~ **Fixed.** `get_tagno()` now returns `self.tagno` directly.
5. ~~`_make_tle` — inconsistent slot count logic, missing offset_valid bit, always creates new TLE~~ **Fixed.** Rewrote as a two-pass function: pass 1 finds the last TLE for the tag; if it has free slots the file is inserted directly. If the last TLE is full, pass 2 allocates a new TLE with the correct doubling slot count, and updates the previous TLE's next-offset bytes plus increments its raw `num_files` field to set the offset_valid bit.
6. ~~`_coalesce_tglk` — integer underflow~~ **Fixed.** Uses `saturating_sub` when decrementing the remaining file count.
7. ~~`_resize_archive` comment~~ **Fixed.** Section 4 write block is now labelled correctly.
8. ~~`bytes_read <= 0` on `usize`~~ **Fixed.** All changed to `== 0`.

## Architectural Issues — All Fixed

1. ~~Dual `Mmap` + `MmapMut` on the same file — unsound~~ **Fixed.** Dropped `mmap: Mmap`; all reads go through `mmap_mut` via `Deref<Target=[u8]>`.
2. ~~Contiguity requirement not enforced on delete~~ **Fixed.** Directory scans iterate the full slot range. Slot counts are written back to the mmap in all four create/delete methods.
3. ~~`_make_tle` always allocates new instead of filling existing slots~~ **Fixed** as part of item 5 above.

## Implementing Stub Methods

Priority order for completing the implementation:

1. ~~Fix the bugs above first~~ All bugs fixed.
2. `remove_file` — mark FM invalid, call `_remove_fileno_from_all_tag_lookups`, call `_delete_fde`, call `_coalesce_flst_around`.
3. `remove_tag` — call `_remove_tagno_from_all_file_metadata`, call `_delete_all_tle_for_tag`, call `_delete_tde`, call `_coalesce_tglk`.
4. `ArchiveManager::remove` / `import_files` / `add_tags` / `remove_tags` / `list_files` / `size_of`.
5. `CommandLineApp::eval_command` — wire up the commented-out command branches.
6. `expand` / `reduce` — decompression/compression of the archive to/from a directory tree.

## Commands to Implement

See TODO.md for the full command spec. Commands are parsed in `CommandLineApp::eval_command` and dispatched to `ArchiveManager` methods. Each CLI command maps one-to-one to an `ArchiveManager` method with the same name.

## Running and Testing

```bash
cd rs
cargo build
cargo test
cargo run
```

Tests live in `archive_manager.rs` under `#[cfg(test)]`. Use `tempfile::TempDir` for isolation and a `ScopedHome` guard to redirect `$HOME`.
