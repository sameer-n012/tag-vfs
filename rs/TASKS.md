# Archive Manager Implementation Tasks

Break down `src/archive/archive_manager.rs` method implementations into single-function work units (~5 minutes each).

1. [DONE] `cache`: copy the named file into the cache directory, creating parents, and store the `NamedFile` in `open_files`; log actions and respect `open` flag.
2. [DONE] `flush`: send one or more cached files/tags into the archive (using planned `Archive` helpers), verify persistence, and surface errors.
3. `flush_all`: iterate over `open_files`, call `flush` on every cached entry, and clean obsolete cache files afterwards.
4. `destroy`: remove matching cached files/tags from the cache without persisting, updating `open_files`.
5. `destroy_all`: purge every cached file and reset `open_files` while leaving the archive untouched.
6. `remove`: delete files matching filters from the archive metadata (file directory, tag lookup) and report success/failure.
7. `import_files`: import filesystem paths (with optional recursion) into the archive, updating directory/tag records.
8. `add_tags`: append tags to specified files, ensuring tag directory/lookup entries are updated consistently.
9. `remove_tags`: drop provided tags from files and clean up orphaned tag lookup entries.
10. `list_files`: read archive metadata to return filenames filtered by tags, formatting the response for CLI consumption.
11. `size_of`: compute total size for file sets matched by the tag filters from the archive metadata.
12. `apply`: run an external command (script/executable) against matching cached files (using `command`), capturing output/errors.
13. `scrape`: fetch remote content for link-type files in the cache, store results, and populate relevant metadata.
14. `merge`: ingest another `.dat` archive file into the current archive, reconciling tag/file indices.
15. `expand_from`: expand either the current archive or a provided `.dat` file into a destination directory hierarchy.
16. `expand`: expand only the current working archive into `destination` using `expand_from` helpers.
17. `reduce`: compress files/directories into a `.dat` file, optionally walking directories recursively.
18. `open` helper (already calls `cache` but verify final state) – ensure it delegates correctly and handles missing files.
19. CLI wiring: update `src/app/command_line_app.rs` (help text & command dispatch) to call each implemented `ArchiveManager` method once operational.

Each task should return meaningful `io::Result` errors, log progress for diagnostics, and be called from the CLI once done.

## Archive Internal Tasks
20. `get_fde`: implement lookup of a file directory entry by fileno against section 1, validating slot bounds.
21. `get_fde_by_filename`: search the file directory for the slot whose filename hash matches the requested path.
22. `get_tde`: read a tag directory entry and interpret its metadata (valid bit, name, lookup offset).
23. `get_tde_from_tagname`: map a tag name to the tag number using the tag directory table.
24. `get_fm`: deserialize file metadata from the storage section at the given offset and expose length/tags/name.
25. `_coalesce_tglk`: rebuild the entire tag lookup section when it becomes fragmented, ensuring tuples are contiguous.
26. `_coalesce_tglk_around`: merge neighboring invalid tag lookup tuples surrounding a given offset so freespace grows.
27. `_coalesce_flst`: scan the file storage section, merge adjacent free slots, and write consolidated metadata.
28. `_coalesce_flst_around`: merge a specific freed slot with preceding/following free blocks using end metadata.
29. `add_file`: allocate entries across sections 1/4 for a new `FileInstance`, update tag lookup and directory tables.
30. `add_tag`: reserve a slot in the tag directory and append/extend tag lookup tuples for a new tag name.
31. `remove_file`: mark a file slot invalid, remove it from tag lookups, and coalesce freed storage.
32. `remove_tag`: invalidate a tag directory slot, wipe associated lookup tuples, and update files that referenced it.
33. `read_file`: stream a persisted file’s bytes from storage and reconstruct its `FileInstance` metadata.
34. `create`: write a brand-new archive header plus initial empty sections (section pointers, directories, lookups, storage).
