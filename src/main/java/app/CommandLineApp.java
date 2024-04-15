package app;

public class CommandLineApp extends App {

    public RunConfiguration config;

    public CommandLineApp(RunConfiguration config) {
        super();
        this.config = config;
        this.initializeApp();
    }

    public void run() {
        System.out.println("Running...");
    }




}
