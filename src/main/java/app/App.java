package app;

public abstract class App {

    public RunConfiguration config;

    public App(RunConfiguration config) {
        this.config = config;
        this.initializeApp();
    }

    public void initializeApp() {
        System.out.println("Initializing App...");
    }

    public abstract void run();

    public String getHelp() {
        return "help menu:";
    }

}
