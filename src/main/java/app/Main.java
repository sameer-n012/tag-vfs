package app;

public class Main {

    public static void main(String[] args) {

        RunConfiguration config = new RunConfiguration();
        config.parseDefaultConfigFile();
        config.parseConfigFile();
        config.parseCommandLineArgs(args);

        if(config.gui) {
            System.out.println("GUI not yet supported");
        } else {
            new CommandLineApp(config).run();
        }


    }
}
