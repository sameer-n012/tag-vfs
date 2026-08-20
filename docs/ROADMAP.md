# Roadmap

This file lists open work items only. For completed work, see [CHANGELOG.md](../CHANGELOG.md).

## Stub commands

These commands parse their arguments but do not act on them:

- [ ] `apply <script>` — apply a Python script to a set of files
- [ ] `scrape` — scrape the web page for a link file and cache the result

## Config persistence

- [ ] `config -p` does not yet write the persisted key/value to a file. The value applies to the current session only.

## Missing test coverage

`tests/` has no bash integration test script for these commands: `lt`, `stat`, `disk`. See [CLAUDE.md](../CLAUDE.md) for the list of existing test scripts.

- [ ] Add `tests/test_lt.sh`
- [ ] Add `tests/test_stat.sh`
- [ ] Add `tests/test_disk.sh`

## Empty data-type modules

The following modules exist and are declared in `src/data/mod.rs`, but hold no code:

- `src/data/directory.rs`
- `src/data/error_file.rs`
- `src/data/image_file.rs`
- `src/data/link_file.rs`
- `src/data/plain_text.rs`
- `src/data/rich_text.rs`

`src/loader/file_importer.rs` and `src/util/conversion.rs` are also empty. These modules likely support `apply`, `scrape`, and richer file-type handling once built out.
