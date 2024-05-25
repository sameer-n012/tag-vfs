package app;

import archive.Archive;
import archive.ArchiveManager;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;

public abstract class App {

    // project information
    public static final String GITHUB_WEB_PATH = "https://github.com/sameer-n012/filevault";
    public static final String PROJECT_WEB_PATH = null;
    public static final String APP_NAME = "file-vault";
    public static final String APP_NAME_PRETTY = "File Vault";

    protected RunConfiguration config;
    protected ArchiveManager am;

    public App(RunConfiguration config) {
        this.config = config;
        this.initializeApp();
    }

    public void initializeApp() {

        System.out.println("Initializing App...");

        this.setupAppDirectory();


    }

    public abstract void run();

    public void setupAppDirectory() {

        try {
            File f = new File(this.config.getAppHomePathAbsolute());
            if(f.exists() && !f.isDirectory()) {
                throw new FileNotFoundException("No suitable location to persist app");
            }
            if (!f.exists()) {
                if (!f.mkdir()) {
                    throw new FileNotFoundException("No suitable location to persist app");
                }
            }

            f = new File(this.config.getCachePathAbsolute());
            if(f.exists() && !f.isDirectory()) {
                throw new FileNotFoundException("No suitable location to persist app");
            }
            if (!f.exists()) {
                if (!f.mkdir()) {
                    throw new FileNotFoundException("No suitable location to persist app");
                }
            }

            f = new File(this.config.getArchivePathAbsolute());
            if(f.exists() && f.isDirectory()) {
                throw new FileNotFoundException("No suitable location to persist app");
            }

            this.am = new ArchiveManager(this.config);

            if (!f.exists()) {
                this.am.createArchiveFile();
            } else {
                // TODO uncomment when done testing
//                this.am.readArchiveFile(appHomePath +
//                        File.separator + RunConfiguration.APP_ARCHIVE_FILE);
                this.am.createArchiveFile();
            }

        } catch(SecurityException | IOException e) {
            System.out.println(e);
            this.clean();
            System.exit(1);
        }
    }

    public void clean() throws SecurityException {
        File f = new File(this.config.getCachePathAbsolute());
        if(f.exists() && f.isDirectory()) {
            File[] flist = f.listFiles();
            if(flist == null) {
                f.delete();
                return;
            }
            for(File f2 : flist) {
                f2.delete();
            }
        }
        f.delete();
    }

}
