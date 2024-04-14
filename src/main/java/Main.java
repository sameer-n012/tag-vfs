import org.apache.commons.cli.*;

public class Main {

    public static void Main(String[] args) {

        Options opts = new Options();

        Option input = new Option("i", "input", true, "input file path");
        input.setRequired(true);
        opts.addOption(input);

        Option output = new Option("o", "output", true, "output file");
        output.setRequired(true);
        opts.addOption(output);

        CommandLineParser parser = new DefaultParser();
        HelpFormatter formatter = new HelpFormatter();
        CommandLine cmd = null;

        try {
            cmd = parser.parse(opts, args);
        } catch (ParseException e) {
            System.out.println(e.getMessage());
            formatter.printHelp("utility-name", opts);

            System.exit(1);
        }

        String inputFilePath = cmd.getOptionValue("input");
        String outputFilePath = cmd.getOptionValue("output");

        System.out.println(inputFilePath);
        System.out.println(outputFilePath);


    }
}
