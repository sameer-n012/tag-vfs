# tag-vfs

`tag-vfs` (also called `file-vault`) is a tag-based virtual file system.

The system stores files by tags. It does not use folders. You add one or more tags to each file. You find files by their tags, not by a folder path.

The system stores all files in one archive file. The archive file has the extension `.dat`. The archive uses a custom binary format. See [docs/FORMAT.md](docs/FORMAT.md) for the full format.

## Install and build

The project uses Rust and Cargo.

```bash
# Build the project
cargo build

# Build the release binary
cargo build --release
```

The build produces a binary named `tag-vfs`.

## Quick start

Run the binary to start the interactive command shell:

```bash
cargo run
```

The shell creates an archive at `~/filevault/archive.dat` on first run. Use `--home <DIR>` to pick a different location:

```bash
cargo run -- --home /path/to/data
```

Example session:

```
> import notes.txt
> tag work -f notes.txt
> ls work
> sz work
> stat notes.txt
```

See [docs/COMMANDS.md](docs/COMMANDS.md) for the full command reference.

## Documentation

| File | Content |
|---|---|
| [docs/COMMANDS.md](docs/COMMANDS.md) | Full CLI command reference |
| [docs/FORMAT.md](docs/FORMAT.md) | The `.dat` binary archive format |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Open work items |
| [CHANGELOG.md](CHANGELOG.md) | Fixed bugs and completed milestones |
| [CLAUDE.md](CLAUDE.md) | Guide for developers and AI coding agents |

## Testing

```bash
# Rust unit tests
cargo test --lib -- --test-threads=1

# Bash integration tests (needs a built binary)
bash tests/run_all.sh
```

See [CLAUDE.md](CLAUDE.md) for full test details.
