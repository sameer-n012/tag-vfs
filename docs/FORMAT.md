# Archive File Format

The archive file uses the extension `.dat`. The file holds all data for the file system in one flat binary file. The file has five contiguous sections.

This file describes the current format. The format supports large archives: 100,000 files, 100 GB of data, and 8.4 million tags.

All multi-byte integers are big-endian. Many fields do not align to a byte boundary. For example, some lengths use 39 bits and some offsets use 40 bits. The code packs these fields into byte arrays with bit-shifting.

Section index constants in `src/archive/archive.rs`:
```rust
const HEAD_S: u8 = 0;
const FLDR_S: u8 = 1;
const TGDR_S: u8 = 2;
const TGLK_S: u8 = 3;
const FLST_S: u8 = 4;
```

## Section 0 — Header (Section Pointers)

- First value: 16-bit magic number, `13579`.
- Value `k+1`: a 48-bit byte-granular offset from the start of this section to section `k`.
- Section length: 208 bits.

## Section 1 — File Directory

- First value: 32-bit unsigned integer. Number of file slots in the directory. (`DIR_SECTION_HEADER_BYTES = 8` for the two header fields together.)
- Second value: 32-bit unsigned integer. Number of file slots in use.
- The directory fills file slots in order, with no gaps.
- Each file slot is a 128-bit tuple `(l, v, p, n, o)`, `file_directory_entry::SIZE_BYTES = 16`:
  - `l` — 39-bit file length
  - `v` — valid bit. `1` means the slot holds a valid file.
  - `p` — 32-bit index of the parent in the file directory. (Planned removal — the design intends to use tags only. Keep the parent slot before the child slot in the directory to avoid an O(n²) scan.)
  - `n` — 16-bit hash of the file name
  - `o` — 40-bit offset from the start of section 4 to the start of the file

## Section 2 — Tag Directory

- First value: 32-bit unsigned integer. Number of tag slots in the directory. Max `(1 << 23) - 1`.
- Second value: 32-bit unsigned integer. Number of tag slots in use.
- The directory fills tag slots in order, with no gaps.
- Each remaining value is a tag slot: a 320-bit tuple `(i, v, t, o)`, `tag_directory_entry::SIZE_BYTES = 40`:
  - `v` — valid bit. `1` means the slot holds a valid tag.
  - `i` — 23-bit unsigned integer. The tag ID.
  - `t` — 32-byte string. The tag name.
  - `o` — 40-bit offset from the start of the tag lookup section to the first tag lookup tuple for this tag

## Section 3 — Tag Lookup

- First value: 32-bit unsigned integer. Size of the section, in bytes.
- Second value: 32-bit unsigned integer. Number of tuples in the section.
- A list of `(96 + 32k)`-bit tuples `(i, v, s, n, o, f1, f2, ...)`. The base (before the file pointers) is 96 bits, `tag_lookup_entry::BASE_SIZE_BYTES = 12`. Each file pointer `f` is 32 bits, `FILE_SLOT_SIZE = 4`.
  - `v` — valid bit. `1` means the tuple is valid.
  - `i` — 23-bit unsigned integer. The tag ID (matches section 2).
  - `s` — 16-bit number. The number of file slots in the tuple.
  - `n` — 16-bit unsigned short. The number of files with the tag. `n` counts only valid `f` entries in this tuple, plus one more if the next-tuple pointer is valid.
  - `f` — 32-bit unsigned integer. The index of a file with the tag. The first tuple for a tag holds 15 file pointers. The next tuple holds 31. The next holds 63, and so on. This growth pattern applies only to valid tuples; an invalid tuple can hold any number of pointers.
  - `o` — 40-bit offset from the start of this section to the next tuple for this tag

## Section 4 — File Storage

Each file is a block: file metadata, then the file data, then end-metadata.

- File metadata is a `k`-bit tuple `(l, v, f, p, y, nn, tn, (ti1, ti2, ...), n)`. The base (before the per-tag list and filename) is 136 bits, `file_metadata::BASE_SIZE_BYTES = 17`:
  - `l` — 39-bit unsigned integer. Length of the data.
  - `v` — valid bit. `1` means the file is valid.
  - `f` — 32-bit unsigned integer. The file index.
  - `p` — 32-bit unsigned integer. The index of the parent in the file directory.
  - `y` — 8-bit number. The file type.
  - `nn` — 8-bit number. Length of the file name, in bytes.
  - `tn` — 16-bit unsigned short. Number of tags on the file.
  - `ti` — 24-bit unsigned integer. A tag ID from the list of the file's tags. Each entry is `TAG_SLOT_SIZE = 3` bytes.
  - `n` — the file name, an `nn`-byte string.
- Data: arbitrary length.
- End-metadata: a 40-bit unsigned integer. Length of the data. (`FEM`, unaffected by any field-width change.)

When section 4 runs out of free space, `_resize_archive` grows it. The new size is `(used + space_needed) × RESIZE_FACTOR`, where `RESIZE_FACTOR = 2`. This leaves headroom for future writes after the resize.

## Format summary table

| # | Name | Key constants |
|---|------|----------------|
| 0 | Header | 16-bit magic (13579) + 4 × 48-bit section offsets |
| 1 | File Directory | 2 × u32 counts + N × FDE (128 bits / 16 bytes each) |
| 2 | Tag Directory | 2 × u32 counts + N × TDE (320 bits / 40 bytes each) |
| 3 | Tag Lookup | 2 × u32 meta + variable-width TLEs (96-bit base + 32-bit file slots) |
| 4 | File Storage | variable-width FileMetadata (136-bit base) + data + FileEndMeta |

Binary entry type names used in the code: `FDE` (file directory entry), `TDE` (tag directory entry), `TLE` (tag lookup entry), `FM` (file metadata), `FEM` (file end-metadata).

## Appendix: design notes

These notes come from the original design phase. Some notes describe open questions. Some describe decisions the implementation has already made.

- On file removal, move the file directory entry and the file entry. Removal is a rare operation, so this cost is acceptable.
- The file storage section may not need as many slots as the file directory has. Consider whether the design can afford large file metadata records (duplicate ID, path, tags).
- Open question: how to identify blank spots in the tag lookup list (where `i` is set but `n == 0`).
- Open question: how to defragment the tag list.
  - Set all bits to 0 when a tag is deleted.
  - When adding files to a tag, moving entries to the end of the list may not be needed.
- Open question: how to defragment file storage.
  - Coalesce unfilled sections right away.
  - Set the valid bit to 0 on removal.
  - Removal is a rare operation.
  - Merge a freed block using the next block's metadata and the previous block's end-metadata.
- Access to the file path is rare. The design can afford many random accesses for this case. Path access applies only when writing back to the hierarchical format (see `expand`).
