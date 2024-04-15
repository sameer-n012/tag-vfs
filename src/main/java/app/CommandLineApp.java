package app;

import jdk.swing.interop.SwingInterOpUtils;

import java.util.Scanner;

public class CommandLineApp extends App {

    public Scanner scn;


    public CommandLineApp(RunConfiguration config) {
        super(config);
        this.scn = new Scanner(System.in);
    }

    public void run() {

        String input = null;

        while(true) {
            System.out.print(this.config.getConfigString("cliPrefix") + " ");
            input = scn.nextLine();

            if(input.equals("quit")) {
                System.out.print("Are you sure you want to quit? (y/n): ");
                input = scn.nextLine();
                if(input.equalsIgnoreCase("y")) {
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
        System.out.println(this.getHelp());
    }

    public void printUnknownCommand(String command) {
        System.out.println("\"" + command + "\" is not a command. See help for valid commands.");
    }




}
