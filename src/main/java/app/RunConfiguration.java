package app;

import org.apache.commons.cli.*;

import java.util.Arrays;

public class RunConfiguration {

    // Run Settings
    public boolean gui;

    // App Settings
    public String appName;
    public String appNamePretty;
    public String appDirectory;

    // Visual Settings
    public String cliPrefix;
    public boolean darkMode;
    public int fontSizeSM;
    public int fontSizeMD;
    public int fontSizeLG;

    public RunConfiguration() {
        this.appName = "file-vault";
    }

    public void parseCommandLineArgs(String[] arguments) {
        Options opts = new Options();

        Option gui = new Option("g", "gui", false, "uses a GUI");
        gui.setRequired(false);
        opts.addOption(gui);

        Option help = new Option("h", "help", false, "print the usage text");
        gui.setRequired(false);
        opts.addOption(help);

        CommandLineParser parser = new DefaultParser();
        HelpFormatter formatter = new HelpFormatter();
        CommandLine cmd = null;

        try {
            cmd = parser.parse(opts, arguments);
        } catch (ParseException e) {
            formatter.printHelp(this.appName, opts);
            System.exit(1);
        }

        if(cmd.hasOption(help)) {
            formatter.printHelp(this.appName, opts);
            System.exit(0);
        }

        this.gui = cmd.hasOption(gui);
    }

    public void parseConfigFile() {}

    public void parseDefaultConfigFile() {}

}
