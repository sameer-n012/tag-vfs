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

1. ~~`FileMetadata::new` and `TagLookupEntry::new` — index into empty Vec~~ **Fixed.**
2. ~~`_read_section_pointers` — wrong byte offsets and wrong slice length~~ **Fixed.**
3. ~~`MAX_FILE_DIR_SLOTS` / `MAX_TAG_DIR_SLOTS` — operator precedence~~ **Fixed.**
4. ~~`TagDirectoryEntry::get_tagno()` — double right-shift~~ **Fixed.**
5. ~~`_make_tle` — inconsistent slot count logic, missing offset_valid bit~~ **Fixed.**
6. ~~`_coalesce_tglk` — integer underflow~~ **Fixed.**
7. ~~`_resize_archive` comment~~ **Fixed.**
8. ~~`bytes_read <= 0` on `usize`~~ **Fixed.**
9. ~~`get_fm` read `MIN_SIZE_BYTES` (14) instead of `BASE_SIZE_BYTES` (13)~~ **Fixed.** The extra byte polluted filenames (e.g. `hello.txth`) and made `size_bytes()` off by one, shifting all data-offset calculations on read paths.
10. ~~`_find_file_space` advanced `bytes_read` by data-length only~~ **Fixed.** Must advance by full block size (`full_FM_size + data_len + FEM_size`) to skip past valid files correctly.
11. ~~`_allocate_file_space` never split the free block~~ **Fixed.** After placing a file the remainder is now written as a new free FM+FEM so future scans find it.
12. ~~`_read_s1_meta` computed `file_storage_section_size_used` by reading FDE bytes with wrong strides~~ **Fixed.** Replaced with `_compute_storage_used()`, which scans section 4 directly and sums valid block sizes. Called at the end of `Archive::new()`.
13. ~~`get_filename()` in `FileMetadata` sliced to end of buffer~~ **Fixed.** Now slices to `start + filename_len` to avoid including trailing bytes.
14. ~~`_resize_archive` left two adjacent free blocks after growing S4~~ **Fixed.** The resize appended a new free block after copying the old section verbatim; the old section already ended with its own free block. `_coalesce_flst()` is now called at the end of `_resize_archive` to merge them, enabling imports of files larger than the original S4 capacity.
15. ~~`_coalesce_flst` used `BASE_SIZE_BYTES` instead of full FM size for invalid blocks~~ **Fixed.** When a valid file is invalidated (valid bit cleared but filename/tag fields left intact), its block is larger than `BASE_SIZE_BYTES + data + FEM`. The coalesce scan now computes the full FM size (`BASE_SIZE_BYTES + num_tags*TAG_SLOT_SIZE + filename_len`) for every block, valid or not. Both outer and inner loop conditions tightened to `offset + BASE_SIZE_BYTES <= section_end` to prevent reading past the mmap end.
16. ~~`_resize_archive` `min_required` not multiplied by `RESIZE_FACTOR`~~ **Fixed.** `min_required` was computed as `used + space_needed` without the `× RESIZE_FACTOR` multiplier. When the file being imported exceeded the current S4 size, the new S4 was sized to exactly `used + space_needed` — leaving no room for the actual allocation after the fixed-size header and existing content. Now `min_required = (used + space_needed) × RESIZE_FACTOR`, guaranteeing a free block large enough for `space_needed` after the resize.
17. ~~`_resize_archive` never called `_compute_storage_used()` after resize~~ **Fixed.** `_read_s1_meta` (called inside `_resize_archive`) resets `file_storage_section_size_used` to 0. Without a subsequent `_compute_storage_used()` call, the field stays at 0 for all future allocations, causing the `disk` command to show 0% S4 usage after a resize and making the early-exit check in `_find_file_space` too permissive (triggers unnecessary extra resizes). `_compute_storage_used()` is now called at the end of `_resize_archive`.
18. ~~`_remove_tagno_from_all_file_metadata` read only `BASE_SIZE_BYTES` then called `get_tags()`~~ **Fixed.** Creating a `FileMetadata` from a `BASE_SIZE_BYTES`-length buffer then calling `get_tags()` on it panics at runtime when `num_tags > 0` (the tag slots start at byte 17, beyond the buffer). The function also used `BASE_SIZE_BYTES + data_len + FEM_SIZE` as the stride, ignoring the variable-length tag and filename fields, desynchronising the scan for any file with a filename. Now reads the full FM buffer (`BASE_SIZE_BYTES + num_tags*TAG_SLOT_SIZE + filename_len`) before calling `get_tags()`, and uses `full_fm_size + data_len + FEM_SIZE` as the stride.

## Architectural Issues — All Fixed

1. ~~Dual `Mmap` + `MmapMut` on the same file — unsound~~ **Fixed.**
2. ~~Contiguity requirement not enforced on delete~~ **Fixed.**
3. ~~`_make_tle` always allocates new instead of filling existing slots~~ **Fixed.**

## Implemented Commands

All commands except `apply` and `scrape` are fully implemented end-to-end:

| Command  | Notes |
|----------|-------|
| `import` | single files and `-r` recursive directories |
| `remove` | by `-f` filename and/or `-t` tag (AND semantics) |
| `tag`    | add or `-d` remove tags from matching files |
| `ls`     | list files; optional positional tag filters (AND) |
| `lt`     | list all tags in the archive with file counts per tag |
| `stat`   | show metadata for a named file: size, type, all tags |
| `sz`     | combined size of matching files |
| `open`   | extract to session cache, launch system viewer |
| `flush`  | write cached files back to archive; change-detection skips unchanged |
| `destroy`| discard cached files (does not touch archive) |
| `expand` | write all archive files to a directory; `-f` selects an alternate source `.dat` |
| `reduce` | import files/directories into archive (alias for `import`) |
| `merge`  | ingest another `.dat`; creates missing tags, preserves existing ones |
| `config` | set key/value; `-l` lists all; `-p` persists (file write not yet implemented) |
| `apply`  | **stub** — silently does nothing |
| `scrape` | **stub** — silently does nothing |
| `quit`   | confirmation prompt; `clean()` removes session cache |

## CLI Flags

`--home <DIR>` — override the app home directory (default `~/filevault`). The archive is stored at `<DIR>/archive.dat` and the session cache at `<DIR>/tmp_<session_id>/`. Used by integration tests for isolation.

## Running and Testing

```bash
cd rs

# Build
cargo build

# Rust unit tests (7 tests, runs in ~0.04 s)
cargo test --lib -- --test-threads=1

# Bash integration tests (requires built binary)
bash tests/run_all.sh           # build + run all suites
bash tests/run_all.sh --no-build  # skip cargo build step
bash tests/test_<name>.sh       # run a single suite
```

### Rust unit tests (`archive_manager.rs`)

| Test | What it covers |
|------|----------------|
| `cache_creates_cached_file` | `cache()` copies file to cache dir, adds to `open_files` |
| `flush_removes_cached_file_by_name` | `flush()` writes new file to archive, removes from `open_files` |
| `destroy_filters_cached_files` | `destroy()` removes matching file, leaves others |
| `destroy_all_clears_cache` | `destroy_all()` empties `open_files` |
| `expand_writes_files_to_directory` | import → expand → verify file content round-trips |
| `reduce_adds_files_to_archive` | `reduce()` → expand → verify content |
| `merge_combines_two_archives` | two archives merged → both files + correct content |

Use `tempfile::TempDir` for isolation. Do **not** use `ScopedHome` for tests that create archives — pass an explicit temp path to `create_archive_file` instead (macOS `dirs::home_dir()` ignores `$HOME`).

### Bash integration tests (`tests/`)

Each script sources `tests/common.sh` for `setup()`/`teardown()`, `run_vfs()`, and `assert_*` helpers. All tests use `--home <tmpdir>` for isolation.

| Script | Commands tested |
|--------|----------------|
| `test_import.sh` | `import` single, multi, recursive, data integrity |
| `test_remove.sh` | `remove` by name, by tag, nonexistent file |
| `test_tag.sh` | `tag` add, remove, multi-tag AND filter |
| `test_ls.sh` | `ls` all files, tag filter, empty archive |
| `test_sz.sh` | `sz` known size, multiple files, tag filter |
| `test_flush.sh` | `flush` no-cache error, no-change path, modify+flush via FIFO, `flush -a` |
| `test_destroy.sh` | `destroy` removes cached file, archive intact, `destroy -a` |
| `test_expand.sh` | `expand` single, multiple, `expand -f` alternate archive |
| `test_reduce.sh` | `reduce` single, recursive, accumulation |
| `test_merge.sh` | `merge` presence, content, tag preservation |
| `test_config.sh` | `config -l`, set prefix, unknown key, `-p` flag |
| `test_e2e.sh` | Full 9-phase workflow: import→tag→sz→flush→remove→expand→reduce→merge→config |

Tests that need to interleave external file modification with a live REPL session (flush, destroy) use a named FIFO as stdin and run the binary in the background.
