use crate::app::app::App;
use crate::app::run_configuration::RunConfiguration;
use crate::util::style;
use std::collections::HashSet;
use std::io::{self, Write};

pub struct CommandLineApp {
    app: App,
}

// Parsed result of a command's argument list.
struct ParsedArgs {
    positionals: Vec<String>, // bare words not preceded by a flag
    filenames: Vec<String>,   // words after -f
    tags: Vec<String>,        // words after -t
    flags: HashSet<char>,     // single-char flags: -r, -a, -d, -p, -l
}

/**
 * Parses a slice of argument tokens into positionals, -f/-t lists, and flags.
 * Flag groups like "-rd" are split into individual characters.
 *
 * @param tokens the argument tokens (command name already removed).
 * @return a ParsedArgs struct.
 */
fn parse_args(tokens: &[&str]) -> ParsedArgs {
    let mut positionals = Vec::new();
    let mut filenames = Vec::new();
    let mut tags = Vec::new();
    let mut flags = HashSet::new();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "-f" => {
                i += 1;
                while i < tokens.len() && !tokens[i].starts_with('-') {
                    filenames.push(tokens[i].to_string());
                    i += 1;
                }
            }
            "-t" => {
                i += 1;
                while i < tokens.len() && !tokens[i].starts_with('-') {
                    tags.push(tokens[i].to_string());
                    i += 1;
                }
            }
            tok if tok.starts_with('-') => {
                for c in tok.chars().skip(1) {
                    flags.insert(c);
                }
                i += 1;
            }
            tok => {
                positionals.push(tok.to_string());
                i += 1;
            }
        }
    }
    ParsedArgs {
        positionals,
        filenames,
        tags,
        flags,
    }
}

impl CommandLineApp {
    pub fn new(config: RunConfiguration) -> Self {
        CommandLineApp {
            app: App::new(config),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut input = String::new();

        loop {
            print!("{} ", style::prompt());
            stdout.flush().unwrap();
            let bytes = stdin.read_line(&mut input).unwrap();
            if bytes == 0 {
                // EOF on stdin (e.g. piped input exhausted) — clean exit.
                self.app.clean();
                break;
            }
            let line = input.trim().to_string();
            if self.eval_command(&line) {
                break;
            }
            input.clear();
        }
    }

    /**
     * Splits a raw input line into command and arguments, dispatches to the
     * appropriate handler, and returns true if the application should quit.
     *
     * @param line the trimmed input line.
     * @return true if the REPL loop should exit.
     */
    fn eval_command(&mut self, line: &str) -> bool {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return false;
        }
        let cmd = tokens[0];
        let args = parse_args(&tokens[1..]);
        match cmd {
            "quit" => self.cli_quit(),
            "help" => self.cli_help(),
            "open" => self.cli_open(args),
            "import" => self.cli_import(args),
            "remove" => self.cli_remove(args),
            "tag" => self.cli_tag(args),
            "ls" => self.cli_list(args),
            "sz" => self.cli_size(args),
            "flush" => self.cli_flush(args),
            "destroy" => self.cli_destroy(args),
            "expand" => self.cli_expand(args),
            "reduce" => self.cli_reduce(args),
            "merge" => self.cli_merge(args),
            "config" => self.cli_config(args),
            "lt" => self.cli_list_tags(),
            "stat" => self.cli_stat(args),
            "disk" => self.cli_disk(),
            "apply" => self.cli_apply(args),
            "scrape" => self.cli_scrape(args),
            _ => {
                println!(
                    "{} '{}'. Type {} for available commands.",
                    style::bold_red("Unknown command:"),
                    cmd,
                    style::bold_cyan("help")
                );
                false
            }
        }
    }

    fn cli_quit(&self) -> bool {
        print!("Are you sure you want to quit? (y/n): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().eq_ignore_ascii_case("y") {
            self.app.clean();
            true
        } else {
            false
        }
    }

    fn cli_help(&self) -> bool {
        // Each entry: (name, args, description). Name is padded to 8 chars visually.
        let entries: &[(&str, &str, &str)] = &[
            (
                "open",
                "[-f <file> ...] [-t <tag> ...]            ",
                "open files with system viewer",
            ),
            (
                "import",
                "<file> ... [-r]                           ",
                "import files into the archive",
            ),
            (
                "remove",
                "-f <file> ... -t <tag> ...                ",
                "remove files from the archive",
            ),
            (
                "tag",
                "<tag> ... [-f <file> ...] [-t <tag>...] [-d]",
                "add (or -d: remove) tags to matching files",
            ),
            (
                "ls",
                "[<tag> ...]                                ",
                "list files with all given tags",
            ),
            (
                "lt",
                "                                           ",
                "list all tags with file counts",
            ),
            (
                "stat",
                "<file>                                     ",
                "show file metadata and tags",
            ),
            (
                "disk",
                "                                           ",
                "show archive space usage per section",
            ),
            (
                "sz",
                "[-f <file> ...] [-t <tag> ...]             ",
                "combined size of matching files",
            ),
            (
                "flush",
                "[-f <file> ...] [-t <tag> ...] [-a] [-d]  ",
                "write cached files to archive",
            ),
            (
                "destroy",
                "[-f <file> ...] [-t <tag> ...] [-a]       ",
                "discard cached files",
            ),
            (
                "expand",
                "<dest> [-f <src.dat>]                     ",
                "expand archive to a directory",
            ),
            (
                "reduce",
                "<file> ... [-r]                           ",
                "compress files into archive",
            ),
            (
                "merge",
                "<file.dat>                                ",
                "merge archive into this one",
            ),
            (
                "config",
                "<key> <value> [-p] [-l]                   ",
                "set or list config values",
            ),
            (
                "apply",
                "<script> [-f <file> ...] [-t <tag> ...]   ",
                "apply script to files",
            ),
            (
                "scrape",
                "[-f <file> ...] [-t <tag> ...]            ",
                "scrape link files",
            ),
            (
                "help",
                "                                           ",
                "show this help text",
            ),
            (
                "quit",
                "                                           ",
                "quit the application",
            ),
        ];
        println!("{}", style::bold("Commands:"));
        for (name, args, desc) in entries {
            // Pad visually: name.len() chars wide, then rest is literal string
            let pad = " ".repeat(9usize.saturating_sub(name.len()));
            println!(
                "  {}{}{}{}",
                style::bold_cyan(name),
                pad,
                args,
                style::dim(desc)
            );
        }
        false
    }

    /**
     * Opens files matching the given filters with the system viewer.
     * Tag-based filtering is not yet supported.
     *
     * @param args -f for filenames, -t for tags (tag filter not yet implemented).
     */
    fn cli_open(&mut self, args: ParsedArgs) -> bool {
        let mut names = args.positionals;
        names.extend(args.filenames);
        if let Err(e) = self.app.am().open_files(names, args.tags) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Imports files from the filesystem into the archive.
     *
     * @param args positional paths to import, -r to recurse into directories.
     */
    fn cli_import(&mut self, args: ParsedArgs) -> bool {
        let recursive = args.flags.contains(&'r');
        if args.positionals.is_empty() {
            println!("Usage: import <file> ... [-r]");
            return false;
        }
        if let Err(e) = self.app.am().import_files(args.positionals, recursive) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Removes files from the archive matching filename and tag filters.
     *
     * @param args -f for filenames, -t for tags (files must match all tags).
     */
    fn cli_remove(&mut self, args: ParsedArgs) -> bool {
        if args.filenames.is_empty() && args.tags.is_empty() {
            println!("Usage: remove -f <file> ... -t <tag> ...");
            return false;
        }
        if let Err(e) = self.app.am().remove(args.filenames, args.tags) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Adds or removes tags from files matching the given filters.
     * Positionals are the tags to add/remove; -f and -t select which files to modify.
     *
     * @param args positionals: tags to add/remove; -f file filter; -t tag filter; -d to remove.
     */
    fn cli_tag(&mut self, args: ParsedArgs) -> bool {
        if args.positionals.is_empty() {
            println!("Usage: tag <tag> ... [-f <file> ...] [-t <filter-tag> ...] [-d]");
            return false;
        }
        let tags_to_modify = args.positionals;
        let result = if args.flags.contains(&'d') {
            self.app
                .am()
                .remove_tags(tags_to_modify, args.filenames, args.tags)
        } else {
            self.app
                .am()
                .add_tags(tags_to_modify, args.filenames, args.tags)
        };
        if let Err(e) = result {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Lists all files in the archive that have all of the given tags.
     * With no tags, lists every file.
     *
     * @param args positional tag names (no -t flag needed).
     */
    fn cli_list(&mut self, args: ParsedArgs) -> bool {
        let tags = if !args.positionals.is_empty() {
            args.positionals
        } else {
            args.tags
        };
        if let Err(e) = self.app.am().list_files(tags) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Prints the combined size of files matching the given filters.
     * With no filters, prints the total size of every file.
     *
     * @param args -f for filenames, -t for tags (positionals also accepted as tag shorthand).
     */
    fn cli_size(&mut self, args: ParsedArgs) -> bool {
        let mut tags = args.tags;
        tags.extend(args.positionals);
        if let Err(e) = self.app.am().size_of(args.filenames, tags) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Writes cached file updates back to the archive.
     * -a flushes all, -d destroys the cache copies afterward.
     *
     * @param args -f for filenames, -t for tags, -a for all, -d to destroy after.
     */
    fn cli_flush(&mut self, args: ParsedArgs) -> bool {
        let destroy_after = args.flags.contains(&'d');
        let flush_all = args.flags.contains(&'a');

        let result = if flush_all {
            if destroy_after {
                self.app.am().flush_all_and_destroy()
            } else {
                self.app.am().flush_all()
            }
        } else if destroy_after {
            self.app.am().flush_and_destroy(args.filenames, args.tags)
        } else {
            self.app.am().flush(args.filenames, args.tags)
        };
        if let Err(e) = result {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Removes files from the temporary cache, discarding any unsaved edits.
     * -a destroys all cached files.
     *
     * @param args -f for filenames, -t for tags, -a for all.
     */
    fn cli_destroy(&mut self, args: ParsedArgs) -> bool {
        let result = if args.flags.contains(&'a') {
            self.app.am().destroy_all()
        } else {
            if args.filenames.is_empty() && args.tags.is_empty() {
                println!("Usage: destroy -f <file> ... | -t <tag> ... | -a");
                return false;
            }
            self.app.am().destroy(args.filenames, args.tags)
        };
        if let Err(e) = result {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Decompresses the archive into a hierarchical directory at the given path.
     * -f specifies an alternative source .dat file.
     *
     * @param args positional destination path, -f for optional source archive.
     */
    fn cli_expand(&mut self, args: ParsedArgs) -> bool {
        let dest = match args.positionals.into_iter().next() {
            Some(d) => d,
            None => {
                println!("Usage: expand <destination> [-f <source.dat>]");
                return false;
            }
        };
        let result = match args.filenames.into_iter().next() {
            Some(src) => self.app.am().expand_from(dest, src),
            None => self.app.am().expand(dest),
        };
        if let Err(e) = result {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Compresses files into the archive's .dat storage file.
     *
     * @param args positional paths to compress, -r to recurse into directories.
     */
    fn cli_reduce(&mut self, args: ParsedArgs) -> bool {
        let recursive = args.flags.contains(&'r');
        if args.positionals.is_empty() {
            println!("Usage: reduce <file> ... [-r]");
            return false;
        }
        if let Err(e) = self.app.am().reduce(args.positionals, recursive) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Merges another .dat archive file into the working archive.
     *
     * @param args positional path to the .dat file to merge.
     */
    fn cli_merge(&mut self, args: ParsedArgs) -> bool {
        let path = match args.positionals.into_iter().next() {
            Some(p) => p,
            None => {
                println!("Usage: merge <file.dat>");
                return false;
            }
        };
        if let Err(e) = self.app.am().merge(path) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Sets a configuration key-value pair. -p persists it to disk. -l lists all values.
     *
     * @param args positional key and value, -p to persist, -l to list.
     */
    fn cli_config(&mut self, args: ParsedArgs) -> bool {
        if args.flags.contains(&'l') {
            println!("{}", self.app.config);
            return false;
        }
        if args.positionals.len() < 2 {
            println!("Usage: config <key> <value> [-p] [-l]");
            return false;
        }
        let persist = args.flags.contains(&'p');
        self.app.update_config(
            args.positionals[0].clone(),
            args.positionals[1].clone(),
            persist,
        );
        false
    }

    /**
     * Lists all tags in the archive with their file counts.
     */
    fn cli_list_tags(&mut self) -> bool {
        if let Err(e) = self.app.am().list_tags() {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Prints archive disk usage: file/tag counts and per-section space usage.
     */
    fn cli_disk(&mut self) -> bool {
        if let Err(e) = self.app.am().disk_info() {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Prints metadata for a named file in the archive.
     *
     * @param args positional filename (or -f <filename>).
     */
    fn cli_stat(&mut self, args: ParsedArgs) -> bool {
        let filename = match args
            .positionals
            .into_iter()
            .next()
            .or_else(|| args.filenames.into_iter().next())
        {
            Some(f) => f,
            None => {
                println!("Usage: stat <filename>");
                return false;
            }
        };
        if let Err(e) = self.app.am().stat_file(filename) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Applies a Python script to files matching the given filters.
     * Not yet implemented.
     *
     * @param args positional script path, -f for filenames, -t for tags.
     */
    fn cli_apply(&mut self, args: ParsedArgs) -> bool {
        let script = match args.positionals.into_iter().next() {
            Some(s) => s,
            None => {
                println!("Usage: apply <script> [-f <file> ...] [-t <tag> ...]");
                return false;
            }
        };
        if let Err(e) = self.app.am().apply(args.filenames, args.tags, script) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }

    /**
     * Scrapes the webpages referenced by link files matching the given filters.
     * Not yet implemented.
     *
     * @param args -f for filenames, -t for tags.
     */
    fn cli_scrape(&mut self, args: ParsedArgs) -> bool {
        if let Err(e) = self.app.am().scrape(args.filenames, args.tags) {
            println!("{} {}", style::bold_red("Error:"), e);
        }
        false
    }
}
