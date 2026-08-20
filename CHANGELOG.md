# Changelog

This file lists completed work: fixed bugs, fixed design issues, and finished milestones. For open work, see [docs/ROADMAP.md](docs/ROADMAP.md).

## Bug fixes

1. `FileMetadata::new` and `TagLookupEntry::new` indexed into an empty `Vec`. Fixed.
2. `_read_section_pointers` used the wrong byte offsets and the wrong slice length. Fixed.
3. `MAX_FILE_DIR_SLOTS` and `MAX_TAG_DIR_SLOTS` had an operator precedence error. Fixed.
4. `TagDirectoryEntry::get_tagno()` applied a double right-shift. Fixed.
5. `_make_tle` had inconsistent slot count logic and a missing offset-valid bit. Fixed.
6. `_coalesce_tglk` had an integer underflow. Fixed.
7. `_resize_archive` had an incorrect comment. Fixed.
8. A comparison `bytes_read <= 0` ran on a `usize`, which is always false for a negative bound. Fixed.
9. `get_fm` read `MIN_SIZE_BYTES` (14) instead of `BASE_SIZE_BYTES` (13). The extra byte polluted filenames (for example, `hello.txt` became `hello.txth`) and made `size_bytes()` off by one, which shifted all data-offset calculations on read paths. Fixed.
10. `_find_file_space` advanced `bytes_read` by the data length only. The fix advances by the full block size (`full_FM_size + data_len + FEM_size`), so the scan correctly skips past valid files. Fixed.
11. `_allocate_file_space` never split the free block after placing a file. The fix writes the remainder as a new free FM+FEM block, so future scans can find it. Fixed.
12. `_read_s1_meta` computed `file_storage_section_size_used` from FDE bytes with the wrong strides. The fix replaces this with `_compute_storage_used()`, which scans section 4 directly and sums valid block sizes. `Archive::new()` calls this function at the end of setup. Fixed.
13. `get_filename()` in `FileMetadata` sliced to the end of the buffer, which could include trailing bytes. The fix slices to `start + filename_len`. Fixed.
14. `_resize_archive` left two adjacent free blocks after growing section 4. The resize appended a new free block after copying the old section, but the old section already ended with its own free block. The fix calls `_coalesce_flst()` at the end of `_resize_archive`, which merges the two blocks and allows imports of files larger than the original section 4 capacity. Fixed.
15. `_coalesce_flst` used `BASE_SIZE_BYTES` instead of the full FM size for invalid blocks. When a valid file is invalidated, its valid bit clears but its filename and tag fields stay in place, so its block is larger than `BASE_SIZE_BYTES + data + FEM`. The fix computes the full FM size (`BASE_SIZE_BYTES + num_tags * TAG_SLOT_SIZE + filename_len`) for every block, valid or not, and tightens both loop conditions to `offset + BASE_SIZE_BYTES <= section_end` to avoid reading past the end of the mmap. Fixed.
16. `_resize_archive` computed `min_required` without the `RESIZE_FACTOR` multiplier. When an imported file exceeded the current section 4 size, the resize sized the new section to exactly `used + space_needed`, which left no room for the allocation after the fixed-size header and existing content. The fix computes `min_required = (used + space_needed) × RESIZE_FACTOR`, which guarantees a free block large enough for `space_needed` after the resize. Fixed.
17. `_resize_archive` never called `_compute_storage_used()` after a resize. `_read_s1_meta`, called inside `_resize_archive`, resets `file_storage_section_size_used` to 0. Without a follow-up call to `_compute_storage_used()`, this field stayed at 0 for all future allocations. This made the `disk` command show 0% section 4 usage after a resize, and made the early-exit check in `_find_file_space` too permissive, which triggered unneeded extra resizes. The fix calls `_compute_storage_used()` at the end of `_resize_archive`. Fixed.
18. `_remove_tagno_from_all_file_metadata` read only `BASE_SIZE_BYTES`, then called `get_tags()`. Building a `FileMetadata` from a `BASE_SIZE_BYTES`-length buffer and then calling `get_tags()` on it panicked at runtime whenever `num_tags > 0`, because the tag slots start at byte 17, past the end of the buffer. The function also used `BASE_SIZE_BYTES + data_len + FEM_SIZE` as its scan stride, which ignored the variable-length tag and filename fields and threw off the scan for any file with a filename. The fix reads the full FM buffer (`BASE_SIZE_BYTES + num_tags * TAG_SLOT_SIZE + filename_len`) before calling `get_tags()`, and uses `full_fm_size + data_len + FEM_SIZE` as the stride. Fixed.

## Design fixes

1. The code held a `Mmap` (read) and a `MmapMut` (write) open on the same file at once. This is unsound. Fixed.
2. The file directory did not enforce the contiguity requirement on delete. Fixed.
3. `_make_tle` always allocated a new tuple, instead of filling an existing slot first. Fixed.

## Scale-up format

The archive format field sizes were widened to support 100,000 files, 100 GB of data, and 8.4 million tags. All field-width changes are complete across sections 1 through 4 and the `Archive` struct. This is the only archive format; see [docs/FORMAT.md](docs/FORMAT.md) for the full layout.

## Command implementation

All 34 method-level work units for `archive_manager.rs` and `archive.rs` are complete. This includes `cache`, `flush`, `flush_all`, `destroy`, `destroy_all`, `remove`, `import_files`, `add_tags`, `remove_tags`, `list_files`, `size_of`, `merge`, `expand_from`, `expand`, `reduce`, and the matching internal `Archive` methods (`get_fde`, `get_tde`, `get_fm`, `_coalesce_tglk`, `_coalesce_flst`, `add_file`, `add_tag`, `remove_file`, `remove_tag`, `read_file`, `create`, and related helpers).

`apply` and `scrape` remain stubs. See [docs/ROADMAP.md](docs/ROADMAP.md).
