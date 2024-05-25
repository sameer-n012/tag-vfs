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
- rest of values are tag slot 184-bit tuple `(v, i, t, o)`
    - `v` is a valid bit, 1 if valid
    - `i` is a 15-bit unsigned short identifying the tag
    - `t` is a 16-byte string of the tag name
    - `o` is the 40-bit offset from the start of the tag lookup section to first tag lookup tuple

### Third Section - Tag Lookup
- first value is 16-bit unsigned integer representing the size of the section in bytes
- second value is 16-bit unsigned integer representing number of tuples in section
- list of (80+16k)-bit tuples `(v, i, n, f1, f2, ...)`
    - `v` is a valid bit, 1 if the tuple is valid
    - `i` is a 15-bit unsigned byte identifying the tag (corresponding to section 2)
    - `s` is an 8-bit number representing the number of file slots in the tuple
    - `n` is a 16-bit unsigned short representing the number of files with the given tag
        - `n` should represent valid `f` entries in this tuple alone, plus 1 if the next pointer is valid
    - `f` is a 16-bit unsigned short representing the index of a file with the tag
        - first tuples for tags have 16 file pointers, then 32, ...
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
    - `ti` is the 15-bit unsigned short identifying the tag
    - `n` is the `nn` byte length string representing the name of the file
- data is arbitrary length
- end-metadata is 40-bit unsigned integer representing length of data

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