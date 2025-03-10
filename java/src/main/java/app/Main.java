package app;

import org.apache.commons.cli.ParseException;
import javax.xml.parsers.ParserConfigurationException;
import java.io.IOException;

public class Main {

    public static void main(String[] args) {

        RunConfiguration config = new RunConfiguration();
        try {
            config.parseDefaultConfigFile();
            config.parseUserConfigFile();
            config.parseCommandLineArgs(args);
        } catch (IOException | ParseException e) {
            System.exit(1);
        }

        if(config.getConfigBool("gui")) {
            System.out.println("GUI not yet supported");
            System.exit(0);
        } else {
            new CommandLineApp(config).run();
        }


    }
}
