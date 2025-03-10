package app;

import org.apache.commons.cli.*;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.util.HashMap;

import org.json.*;



public class RunConfiguration {

    // static file locations in JAR
    public static final String DEFAULT_CONFIG_FILE_PATH = "/.conf.json";
    public static final String USER_CONFIG_FILE_PATH = "/user.conf.json";

    // static final locations not in JAR
    public static final String APP_DATA_DIR = "filevault";
    public static final String CACHE_DIR = "tmp_" + String.valueOf(System.currentTimeMillis());
    public static final String APP_ARCHIVE_FILE = "archive.dat";

    // Settings
    private HashMap<String, String> configMap;
    private String[] commandLineArgs;

    public long sessionID;

    public RunConfiguration() {
        this.sessionID = System.currentTimeMillis();
        this.setupConfig();
    }

    private void setupConfig() {
        this.configMap = new HashMap<>();
        this.configMap.put("appName", App.APP_NAME);
        this.configMap.put("appNamePretty", App.APP_NAME_PRETTY);
        this.configMap.put("javaVersion", System.getProperty("java.version"));
        this.configMap.put("javafxVersion", System.getProperty("javafx.version"));
        this.configMap.put("userHome", System.getProperty("user.home"));
        this.configMap.put("runPath", new File(".").getAbsolutePath());
    }

    public String getAppName() { return this.configMap.get("appName"); }

    public String getAppNamePretty() { return this.configMap.get("appNamePretty"); }

    public long getSessionID() { return this.sessionID; }

    public String updateConfig(String key, String value, boolean persist) {
        if(!RunConfigurationConstraints.checkConstraints(key, value)) {
            throw new IllegalArgumentException("Invalid config values");
        }
        String old = this.configMap.put(key, value);

        if(persist) {
            // TODO write to user settings file
        }

        return old;
    }

    public String getAppHomePathAbsolute() {
        return this.configMap.get("userHome") +
                File.separator + RunConfiguration.APP_DATA_DIR;
    }

    public String getCachePathAbsolute() {
         return this.getAppHomePathAbsolute() + File.separator + RunConfiguration.APP_ARCHIVE_FILE;
    }

    public String getArchivePathAbsolute() {
        return this.getAppHomePathAbsolute() + File.separator + RunConfiguration.APP_ARCHIVE_FILE;
    }

    // TODO Make sure to validate all user generated config
    public void parseCommandLineArgs(String[] arguments) throws ParseException {
        this.commandLineArgs = arguments;

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
            formatter.printHelp(this.getConfigString("appName"), opts);
            throw e;
        }

        if(cmd.hasOption(help)) {
            formatter.printHelp(this.getConfigString("appName"), opts);
            System.exit(0);
        }

        this.configMap.put("gui", String.valueOf(cmd.hasOption(gui)));
    }

    // TODO Make sure to validate all user generated config
    public void parseUserConfigFile() throws IOException {

        InputStream in = RunConfiguration.class.getResourceAsStream(RunConfiguration.USER_CONFIG_FILE_PATH);
        JSONObject json = new JSONObject(new String(in.readAllBytes()));

        for(String k : JSONObject.getNames(json)) {

            // do not add extra config keys
            if(!this.configMap.containsKey(k)) { continue; }

            String v = String.valueOf(json.get(k));
            if(k == null || v == null) { continue; }

            // config value constraint checking
            if(!RunConfigurationConstraints.checkConstraints(k, v)) { continue; }

            this.configMap.put(k, v);
        }

    }

    public void parseDefaultConfigFile() throws IOException {

        InputStream in = RunConfiguration.class.getResourceAsStream(RunConfiguration.DEFAULT_CONFIG_FILE_PATH);
        JSONObject json = new JSONObject(new String(in.readAllBytes()));

        for(String k : JSONObject.getNames(json)) {
            this.configMap.put(k, String.valueOf(json.get(k)));
        }

    }

    public boolean contains(String key) {
        return this.configMap.containsKey(key);
    }

    public void reloadConfig() throws ParseException, IOException {
        this.parseDefaultConfigFile();
        this.parseUserConfigFile();
        this.parseCommandLineArgs(this.commandLineArgs);
    }

    public void resetConfig() throws IOException {
        this.setupConfig();
    }

    public int getConfigInt(String key) {
        return Integer.parseInt(this.configMap.get(key));
    }

    public boolean getConfigBool(String key) {
        return Boolean.parseBoolean(this.configMap.get(key));
    }

    public String getConfigString(String key) {
        return this.configMap.get(key);
    }

    public double getConfigDouble(String key) {
        return Double.parseDouble(this.configMap.get(key));
    }

    public char getConfigChar(String key) {
        String v = this.configMap.get(key);
        if(v == null || v.length() == 0) {
            return (char) 0;
        } else {
            return this.configMap.get(key).charAt(0);
        }
    }

    public long getConfigLong(String key) {
        return Long.parseLong(this.configMap.get(key));
    }

    public String toString() {
        // TODO implement pretty string
        return "Run Configuration:";
    }

    private static class RunConfigurationConstraints {

        // configuration constraints
        private static final String INT_FIELDS = ",fontSizeLG,fontSizeMD,fontSizeSM,";
        private static final String POS_INT_FIELDS = ",fontSizeLG,fontSizeMD,fontSizeSM,";
        private static final String DOUBLE_FIELDS = "";
        private static final String STRING_FIELDS = ",cliPrefix,";
        private static final String CHAR_FIELDS = "";
        private static final String BOOL_FIELDS = ",gui,darkMode,";

        private static boolean checkConstraints(String key, String value) {
            String pKey = "," + key + ",";

            if(RunConfigurationConstraints.INT_FIELDS.contains(pKey)) {
                try { Integer.parseInt(value); }
                catch(NumberFormatException | NullPointerException e) { return false; }
            }

            if(RunConfigurationConstraints.POS_INT_FIELDS.contains(pKey)) {
                try {
                    int i = Integer.parseInt(value);
                    if(i <= 0) { return false; }
                }
                catch(NumberFormatException | NullPointerException e) { return false; }
            }

            if(RunConfigurationConstraints.DOUBLE_FIELDS.contains(pKey)) {
                try { Double.parseDouble(value); }
                catch(NumberFormatException | NullPointerException e) { return false; }
            }

            if(RunConfigurationConstraints.STRING_FIELDS.contains(pKey)) {
                return value != null;
            }

            if(RunConfigurationConstraints.CHAR_FIELDS.contains(pKey)) {
                return value != null && value.length() == 1;
            }

            if(RunConfigurationConstraints.BOOL_FIELDS.contains(pKey)) {
                return value.equalsIgnoreCase("true") || value.equalsIgnoreCase("false");
            }

            return true;
        }


    }

}
