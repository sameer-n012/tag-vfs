# Command Reference

This file lists every command in the `tag-vfs` interactive shell. It lists the flags for each command and the current implementation status.

The project builds two binaries: `tag-vfs` (shell plus GUI) and `tag-vfs-cli` (shell only). Everything below applies to the interactive shell in both binaries, unless noted.

## Startup flags

Use these flags when you start a binary, before the interactive shell (or GUI) opens.

| Flag | Effect |
|---|---|
| `--home <DIR>` | Use `<DIR>` as the app home directory. The default is `~/tag_vfs`. The archive file is `<DIR>/archive.dat`. The session cache is `<DIR>/tmp_<session_id>/`. |
| `-g`, `--gui` | Open the GUI instead of the interactive shell. Only `tag-vfs` reads this flag; `tag-vfs-cli` does not have GUI code and always opens the shell. See [../README.md](../README.md) for what the GUI covers. |

## Commands

All commands below run inside the interactive shell.

| Command | Status |
|---|---|
| `open` | Implemented |
| `apply` | Stub — does nothing |
| `expand` | Implemented |
| `reduce` | Implemented |
| `import` | Implemented |
| `merge` | Implemented |
| `scrape` | Stub — does nothing |
| `destroy` | Implemented |
| `quit` | Implemented |
| `help` | Implemented |
| `config` | Implemented (except `-p`, see below) |
| `flush` | Implemented |
| `ls` | Implemented |
| `lt` | Implemented |
| `stat` | Implemented |
| `sz` | Implemented |
| `remove` | Implemented |
| `tag` | Implemented |
| `disk` | Implemented |

### `open`
Opens a file with the system viewer.
- `-t <tag1> ...` open only files with the given tags
- `-f <file1> ...` open only files with the given names

`open` copies the file to the session cache, then launches the system viewer on the cached copy.

### `apply <script>`
Applies a Python script to a set of files.
- `-t <tag1> ...` apply only to files with the given tags
- `-f <file1> ...` apply only to files with the given names

Status: stub. The command parses but does nothing.

### `expand <destination>`
Writes the archive contents to a directory tree, in hierarchical form.
- `-f <file>` expand a given `.dat` file instead of the working archive

### `reduce <file1> ...`
Compresses a set of files into the archive. This command is an alias for `import`.
- `-r` recurse into directories

### `import <file1> ...`
Imports a set of files into the archive.
- `-r` recurse into directories

### `merge <file>`
Merges a `.dat` file into the working archive. `merge` creates tags that do not yet exist in the working archive. `merge` keeps tags that already exist.

### `scrape`
Scrapes the web page for a link file and caches the result.
- `-t <tag1> ...` scrape only files with the given tags
- `-f <file1> ...` scrape only files with the given names

Status: stub. The command parses but does nothing.

### `destroy`
Removes files from the session cache. `destroy` deletes any unsaved changes to those files. `destroy` does not change the archive.
- `-t <tag1> ...` destroy only files with the given tags
- `-f <file1> ...` destroy only files with the given names
- `-a` destroy all cached files (overrides `-f` and `-t`)

### `quit`
Quits the application. `quit` asks for confirmation, then clears the session cache.

### `help`
Shows the command list.

### `config <key> <value>`
Sets a configuration key and value.
- `-p` save the key and value for future sessions (not yet implemented — the value applies to this session only)
- `-l` list all configuration keys and values

Known configuration keys: `appName`, `appNamePretty`, `runPath`, `gui`, `appHomePath`, `cliPrefix`, `darkMode`.

### `flush`
Writes cached file changes back to the archive.
- `-t <tag1> ...` flush only files with the given tags
- `-f <file1> ...` flush only files with the given names
- `-a` flush all cached files (overrides `-f` and `-t`)
- `-d` remove cached files after flush

`flush` skips a file if it has no unsaved changes.

### `ls <tag1> ...`
Lists all files that have every given tag. `ls` with no tags lists every file.

### `lt`
Lists every tag in the archive. `lt` shows the file count for each tag.

### `stat <file>`
Shows metadata for one named file: size, type, and tags.

### `sz <tag1> ...`
Shows the combined size of all files that have every given tag.

### `remove -f <file1> ... -t <tag1> ...`
Removes files that match the given names and tags from the archive.

### `tag <tags_to_add> -f <file1> ... -t <tag1> ...`
Adds tags to a set of files.
- `-d` remove the given tags from the matched files, instead of adding them

### `disk`
Shows disk usage for the archive file and each of its five sections.
