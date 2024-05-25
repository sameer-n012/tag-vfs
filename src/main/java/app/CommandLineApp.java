package app;

import org.apache.commons.cli.*;
import org.apache.commons.lang3.StringUtils;
import org.apache.commons.text.similarity.LevenshteinDistance;

import java.util.*;

public class CommandLineApp extends App {

    public Scanner scn;

    private static final String[] VALID_COMMANDS = {"help", "quit", "config", "open", "apply", "expand", "reduce",
            "import", "remove", "merge", "scrape", "flush", "ls", "sz", "tag"};
    private static final int MAX_COMMAND_SUGGESTION_DIST = 3;

    private final CommandLineParser parser;
    private final HelpFormatter formatter;


    public CommandLineApp(RunConfiguration config) {
        super(config);
        this.scn = new Scanner(System.in);
        this.parser = new DefaultParser();
        this.formatter = new HelpFormatter();
    }

    public void run() {

        String input;
        boolean quit = false;

        while(!quit) {
            System.out.print(this.config.getConfigString("cliPrefix") + " ");
            input = scn.nextLine().trim();

            quit = this.evalCommand(input);
        }
    }

    public boolean evalCommand(String command) {

        if(command == null) { return false; }

        String[] cmd = command.split(" ");

        if (cmd.length == 0) {
            return false;
        }

        switch (cmd[0]) {
            case "quit" -> { return cliQuit(); }
            case "help" -> cliHelp();
            case "open" -> cliOpen(cmd);
            case "apply" -> cliApply(cmd);
            case "expand" -> cliExpand(cmd);
            case "reduce" -> cliReduce(cmd);
            case "import" -> cliImport(cmd);
            case "remove" -> cliRemove(cmd);
            case "destroy" -> cliDestroy(cmd);
            case "merge" -> cliMerge(cmd);
            case "scrape" -> cliScrape(cmd);
            case "config" -> cliConfig(cmd);
            case "flush" -> cliFlush(cmd);
            case "ls" -> cliList(cmd);
            case "sz" -> cliSize(cmd);
            case "tag" -> cliTag(cmd);
            default -> cliUnknownCommand(cmd[0]);
        }

        return false;

    }

    private boolean cliQuit() {
        System.out.print("Are you sure you want to quit? (y/n): ");
        String input = scn.nextLine().trim();
        if(input.equalsIgnoreCase("y")) {
            this.clean();
            return true;
        }
        return false;
    }

    private void cliHelp() {
        StringBuilder sb = new StringBuilder();
        sb.append("help menu:");
        sb.append("thing");
        System.out.println(sb.toString());
    }

    private void cliUnknownCommand(String command) {
        LevenshteinDistance ld = new LevenshteinDistance(CommandLineApp.MAX_COMMAND_SUGGESTION_DIST);

        String mins = null;
        int mind = Integer.MAX_VALUE;
        for(String cmd : CommandLineApp.VALID_COMMANDS) {
            int d = ld.apply(cmd, command);
            if(d < mind) { mins = cmd; mind = d; }
        }

        if(mins != null && mind <= CommandLineApp.MAX_COMMAND_SUGGESTION_DIST) {
            System.out.println("\"" + command + "\" is not a command, did you mean \"" + mins + "\"? " +
                    "See \"help\" for valid commands.");
        } else {
            System.out.println("\"" + command + "\" is not a command. See \"help\" for valid commands.");
        }

    }

    private void cliConfig(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("p", "persist", false, "persists the new configuration option across sessions"));
        opts.getOption("p").setRequired(false);

        opts.addOption(new Option("l", "list", false, "lists the current key-value configuration pairs"));
        opts.getOption("l").setRequired(false);


        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
        } else if (cl.hasOption('l')) {
            System.out.println(this.config);
        } else {
            if(cl.getArgs().length != 2) { this.formatter.printHelp(cmd[0], opts); return; }
            this.config.updateConfig(cl.getArgs()[0], cl.getArgs()[1], cl.hasOption('p'));
        }


    }

    private void cliOpen(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("f", "file", true, "opens only files with the given filenames"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(Option.UNLIMITED_VALUES);

        opts.addOption(new Option("t", "tag", true, "opens only files with the given tags"));
        opts.getOption("t").setRequired(false);
        opts.getOption("t").setArgs(Option.UNLIMITED_VALUES);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }

        this.am.open(fnames, tags);


    }

    private void cliApply(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("f", "file", true, "applies to only files with the given filenames"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(Option.UNLIMITED_VALUES);

        opts.addOption(new Option("t", "tag", true, "applies to only files with the given tags"));
        opts.getOption("t").setRequired(false);
        opts.getOption("t").setArgs(Option.UNLIMITED_VALUES);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }

        this.am.apply(fnames, tags);


    }

    private void cliScrape(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("f", "file", true, "scrapes only files with the given filenames"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(Option.UNLIMITED_VALUES);

        opts.addOption(new Option("t", "tag", true, "scrapes only files with the given tags"));
        opts.getOption("t").setRequired(false);
        opts.getOption("t").setArgs(Option.UNLIMITED_VALUES);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }

        this.am.scrape(fnames, tags);


    }

    private void cliRemove(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("f", "file", true, "removes only files with the given filenames"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(Option.UNLIMITED_VALUES);

        opts.addOption(new Option("t", "tag", true, "removes only files with the given tags"));
        opts.getOption("t").setRequired(false);
        opts.getOption("t").setArgs(Option.UNLIMITED_VALUES);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }

        this.am.remove(fnames, tags);


    }

    private void cliDestroy(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("a", "all", false, "destroys all open files (overrides -f, -t)"));
        opts.getOption("a").setRequired(false);

        opts.addOption(new Option("f", "file", true, "applies to only files with the given filenames"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(Option.UNLIMITED_VALUES);

        opts.addOption(new Option("t", "tag", true, "applies to only files with the given tags"));
        opts.getOption("t").setRequired(false);
        opts.getOption("t").setArgs(Option.UNLIMITED_VALUES);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        } else if (cl.hasOption('a')) {
            this.am.destroyAll();
            return;
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }

        this.am.destroy(fnames, tags);


    }

    private void cliTag(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("h", "help", false, "removes the set of tags from the files"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("f", "file", true, "applies tags to only files with the given filenames"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(Option.UNLIMITED_VALUES);

        opts.addOption(new Option("t", "tag", true, "applies tags to only files with the given tags"));
        opts.getOption("t").setRequired(false);
        opts.getOption("t").setArgs(Option.UNLIMITED_VALUES);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }
        ArrayList<String> toAdd = new ArrayList<>(cl.getArgList());

        if (cl.hasOption('d')) {
            this.am.removeTags(fnames, tags, toAdd);
        } else {
            this.am.addTags(fnames, tags, toAdd);
        }



    }

    private void cliList(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        this.am.listFiles(new ArrayList<>(cl.getArgList()));


    }

    private void cliSize(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        this.am.sizeOf(new ArrayList<>(cl.getArgList()));


    }

    private void cliFlush(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("a", "all", false, "destroys all open files (overrides -f, -t)"));
        opts.getOption("a").setRequired(false);

        opts.addOption(new Option("d", "destroy", false, "destroys files after flushing"));
        opts.getOption("d").setRequired(false);

        opts.addOption(new Option("f", "file", true, "applies to only files with the given filenames"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(Option.UNLIMITED_VALUES);

        opts.addOption(new Option("t", "tag", true, "applies to only files with the given tags"));
        opts.getOption("t").setRequired(false);
        opts.getOption("t").setArgs(Option.UNLIMITED_VALUES);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        } else if (cl.hasOption('a')) {
            this.am.flushAll();
            if(cl.hasOption('d')) { this.am.destroyAll(); }
            return;
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }

        this.am.flush(fnames, tags);
        if(cl.hasOption('d')) { this.am.destroy(fnames, tags); }


    }

    private void cliExpand(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("f", "file", true, "expands the specified archive file"));
        opts.getOption("f").setRequired(false);
        opts.getOption("f").setArgs(1);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        } else if (cl.hasOption('f')) {
            if(cl.getArgs().length != 1) { this.formatter.printHelp(cmd[0], opts); return; }
            this.am.expand(cl.getArgs()[0], cl.getOptionValue("f"));
        } else {
            if(cl.getArgs().length != 1) { this.formatter.printHelp(cmd[0], opts); return; }
            this.am.expand(cl.getArgs()[0]);
        }

        ArrayList<String> fnames = null;
        ArrayList<String> tags = null;

        if (cl.hasOption('f')) {
            fnames = new ArrayList<>(Arrays.asList(cl.getOptionValues("f")));
        }
        if (cl.hasOption('t')){
            tags = new ArrayList<>(Arrays.asList(cl.getOptionValues("t")));
        }

        this.am.flush(fnames, tags);
        if(cl.hasOption('d')) { this.am.destroy(fnames, tags); }


    }

    private void cliReduce(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("r", "recursive", false, "recursively reduces directories"));
        opts.getOption("r").setRequired(false);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
        } else {
            if(cl.getArgs().length == 0) { this.formatter.printHelp(cmd[0], opts); return; }
            this.am.reduce(new ArrayList<>(cl.getArgList()), cl.hasOption('r'));
        }

    }

    private void cliImport(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        opts.addOption(new Option("r", "recursive", false, "recursively imports directories"));
        opts.getOption("r").setRequired(false);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
        } else {
            if(cl.getArgs().length == 0) { this.formatter.printHelp(cmd[0], opts); return; }
            this.am.importFiles(new ArrayList<>(cl.getArgList()), cl.hasOption('r'));
        }

    }

    private void cliMerge(String[] cmd) {

        String[] args = Arrays.copyOfRange(cmd, 1, cmd.length);

        Options opts = new Options();

        opts.addOption(new Option("h", "help", false, "prints command usage text"));
        opts.getOption("h").setRequired(false);

        CommandLine cl = null;

        try {
            cl = this.parser.parse(opts, args);
        } catch (ParseException e) {
            this.formatter.printHelp(cmd[0], opts);
            return;
        }

        if (cl.hasOption('h')) {
            this.formatter.printHelp(cmd[0], opts);
        } else {
            if(cl.getArgs().length != 1) { this.formatter.printHelp(cmd[0], opts); return; }
            this.am.merge(cl.getArgs()[0]);
        }


    }


}
