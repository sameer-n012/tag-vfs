package app;

import archive.Archive;
import archive.ArchiveFactory;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;

public abstract class App {

    // project information
    public static final String GITHUB_WEB_PATH = "https://github.com/sameer-n012/filevault";
    public static final String PROJECT_WEB_PATH = null;
    public static final String APP_NAME = "file-vault";
    public static final String APP_NAME_PRETTY = "File Vault";

    public RunConfiguration config;
    public Archive archive;

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
        String appHomePath = this.config.getConfigString("userHome") +
                File.separator + RunConfiguration.APP_DATA_DIR;

        try {
            File f = new File(appHomePath);
            if(f.exists() && !f.isDirectory()) {
                throw new FileNotFoundException("No suitable location to persist app");
            }
            if (!f.exists()) {
                if (!f.mkdir()) {
                    throw new FileNotFoundException("No suitable location to persist app");
                }
            }

            f = new File(appHomePath +
                    File.separator + RunConfiguration.CACHE_DIR);
            if(f.exists() && !f.isDirectory()) {
                throw new FileNotFoundException("No suitable location to persist app");
            }
            if (!f.exists()) {
                if (!f.mkdir()) {
                    throw new FileNotFoundException("No suitable location to persist app");
                }
            }

            f = new File(appHomePath +
                    File.separator + RunConfiguration.APP_ARCHIVE_FILE);
            if(f.exists() && f.isDirectory()) {
                throw new FileNotFoundException("No suitable location to persist app");
            }
            if (!f.exists()) {
                this.archive = ArchiveFactory.createArchiveFile(appHomePath +
                        File.separator + RunConfiguration.APP_ARCHIVE_FILE);
            } else {
                // TODO uncomment when done testing
//                this.archive = ArchiveFactory.readArchiveFile(appHomePath +
//                        File.separator + RunConfiguration.APP_ARCHIVE_FILE);
                this.archive = ArchiveFactory.createArchiveFile(appHomePath +
                        File.separator + RunConfiguration.APP_ARCHIVE_FILE);
            }

        } catch(SecurityException | IOException e) {
            System.out.println(e);
            this.clean();
            System.exit(1);
        }
    }

    public void clean() throws SecurityException {
        File f = new File(this.config.getConfigString("userHome") +
                File.separator + RunConfiguration.APP_DATA_DIR +
                File.separator + RunConfiguration.CACHE_DIR);
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
