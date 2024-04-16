package app;

import java.util.Scanner;

public class CommandLineApp extends App {

    public Scanner scn;


    public CommandLineApp(RunConfiguration config) {
        super(config);
        this.scn = new Scanner(System.in);
    }

    public void run() {

        String input;

        while(true) {
            System.out.print(this.config.getConfigString("cliPrefix") + " ");
            input = scn.nextLine().trim();

            if(input.equals("quit")) {
                System.out.print("Are you sure you want to quit? (y/n): ");
                input = scn.nextLine().trim();
                if(input.equalsIgnoreCase("y")) {
                    this.clean();
                    System.exit(0);
                }
            } else if(input.equals("help")) {
                printHelp();
            } else {
                printUnknownCommand(input);
            }

        }
    }

    public void printHelp() {
        StringBuilder sb = new StringBuilder();
        sb.append("help menu:");
        sb.append("thing");
        System.out.println(sb.toString());
    }

    public void printUnknownCommand(String command) {
        System.out.println("\"" + command + "\" is not a command. See help for valid commands.");
    }




}
