package archive;

import app.RunConfiguration;
import data.FileInstance;
import loader.FileImporter;
import util.Conversion;

import java.awt.*;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.*;
import java.util.Map.Entry;

public class ArchiveManager {

    private static final int INITIAL_FILE_DIR_SLOTS = 1024;
    private static final int INITIAL_TAG_DIR_SLOTS = 256;
    private static final int INITIAL_TAG_LOOKUP_SLOTS = 1024;
    private static final int INITIAL_TAG_LOOKUP_SPACE_BYTES = INITIAL_TAG_LOOKUP_SLOTS * TagLookupEntry.MIN_SIZE_BYTES;
    private static final long INITIAL_FILE_STORAGE_SPACE_BYTES = 1024*1024*1024; // 1 GB

    private Archive archive;
    private RunConfiguration runConfig;

    public HashMap<Short, FileInstance> openFiles; // maps fileno to file instance object
    public HashMap<Short, String> cacheFileNames; // maps fileno to cache file name
    public FileImporter cacheFileLoader;

    public ArchiveManager(RunConfiguration runConfig) {
        this.runConfig = runConfig;
        this.openFiles = new HashMap<>();
        this.cacheFileNames = new HashMap<>();
        this.cacheFileLoader = new FileImporter(runConfig.getCachePathAbsolute());
    }


    public void createArchiveFile() throws IOException, SecurityException {
        File f = new File(this.runConfig.getArchivePathAbsolute());
        Archive.create(f,
                INITIAL_FILE_DIR_SLOTS, INITIAL_TAG_DIR_SLOTS,
                INITIAL_TAG_LOOKUP_SPACE_BYTES, INITIAL_FILE_STORAGE_SPACE_BYTES);
        this.archive = new Archive(f);
    }

    public void readArchiveFile() throws IOException {
        this.archive = new Archive(new File(this.runConfig.getArchivePathAbsolute()));
    }

    public void open(String filename) throws IOException {
        this.cache(filename, true);
    }

    public void open(ArrayList<String> filenames, ArrayList<String> tags) { }

    private ArrayList<FileInstance> cache(String filename, boolean open) throws IOException {
        ArrayList<FileDirectoryEntry> fdes = this.archive.getFDE(filename);
        ArrayList<FileInstance> out = new ArrayList<>(fdes.size());

        for(FileDirectoryEntry fde : fdes) {
            if(this.openFiles.containsKey(fde.getFileno())) {
                if(!this.runConfig.getConfigBool("gui")) {
                    System.out.println("File " + filename + " is already open as \"" +
                            this.cacheFileNames.get(fde.getFileno()) + "\"");
                }
                File f = new File(this.runConfig.getCachePathAbsolute() +
                        this.cacheFileNames.get(fde.getFileno()));
                if(open) { Desktop.getDesktop().open(f); }
                out.add(cacheFileLoader.loadFile(f.getAbsolutePath(), false));
                continue;
            }

            FileMetadata fm = this.archive.getFM(fde.getFileOffset());

            String cacheFilename = this.runConfig.getSessionID() + "_" +
                    fde.getFileno() + "_" +
                    fm.getFilename();

            FileOutputStream fos = new FileOutputStream(this.runConfig.getCachePathAbsolute() + cacheFilename);
            this.archive.writeFileDataToStream(fde.getFileOffset() + fm.getMetadataLength(), fm.getLength(), fos);
            fos.close();

            this.cacheFileNames.put(fde.getFileno(), cacheFilename);
            File f = new File(this.runConfig.getCachePathAbsolute() +
                    cacheFilename);
            FileInstance fi = cacheFileLoader.loadFile(f.getAbsolutePath(), false);
            this.openFiles.put(fde.getFileno(), fi);
            if(open) { Desktop.getDesktop().open(f); }
            out.add(fi);

            if(!this.runConfig.getConfigBool("gui")) {
                System.out.println("Opened " + filename + " as \"" +
                        this.cacheFileNames.get(fde.getFileno()) + "\"");
            }

        }

        return out;
    }

    public void flush(ArrayList<String> filenames, ArrayList<String> tags) {

        for(Entry<Short, FileInstance> e : this.openFiles.entrySet()) {

            if(filenames != null && !filenames.contains(e.getValue().getName())) {
                continue;
            }

            if(tags != null) {
                Set<String> intersection = new HashSet<>(tags);
                intersection.retainAll(new HashSet<>(e.getValue().getTags()));
                if (intersection.isEmpty()) {
                    continue;
                }
            }

            // TODO flush file here

        }

    }

    public void flushAll() {
        this.flush(null, null);
    }

    public void destroy(ArrayList<String> filenames, ArrayList<String> tags) {

        ArrayList<Short> toDestroy = new ArrayList<>();

        for(Entry<Short, FileInstance> e : this.openFiles.entrySet()) {

            if(filenames != null && !filenames.contains(e.getValue().getName())) {
                continue;
            }

            if(tags != null) {
                Set<String> intersection = new HashSet<>(tags);
                intersection.retainAll(new HashSet<String>(e.getValue().getTags()));
                if (intersection.isEmpty()) {
                    continue;
                }
            }

            toDestroy.add(e.getKey());

        }

        // TODO destroy file here
        for(Short s : toDestroy) {
            File f = new File(this.openFiles.get(s).getPath());
            f.delete();
            if(!this.runConfig.getConfigBool("gui")) {
                System.out.println("Destroyed " + this.cacheFileNames.get(s) + "\"");
            }
            this.cacheFileNames.remove(s);
            this.openFiles.remove(s);

        }

    }

    public void destroyAll() {
        this.destroy(null, null);
    }

    public void remove(ArrayList<String> filenames, ArrayList<String> tags) throws IOException {

        ArrayList<FileDirectoryEntry> fdes = new ArrayList<>();
        for(String fn : filenames) {
            fdes.addAll(this.archive.getFDE(fn));
        }

        for (Iterator<FileDirectoryEntry> it = fdes.iterator(); it.hasNext(); ) {
            short[] tmp = this.archive.getFM(it.next().getFileOffset()).getTags();
            ArrayList<Short> ts = new ArrayList<>();
            for(short t : tmp) { ts.add(t); }
            if(!ts.containsAll(tags)) { it.remove(); break; }
        }

        for(FileDirectoryEntry fde : fdes) {
            this.archive.delete(fde.getFileno());
        }

    }

    public void importFiles(ArrayList<String> paths, boolean recursive) {}

    public void addTags(ArrayList<String> filenames, ArrayList<String> tags, ArrayList<String> tagsToAdd) {}

    public void removeTags(ArrayList<String> filenames, ArrayList<String> tags, ArrayList<String> tagsToRemove) {}

    public void listFiles(ArrayList<String> tags) {}

    public void sizeOf(ArrayList<String> tags) {}

    public void apply(ArrayList<String> filenames, ArrayList<String> tags) {}

    public void scrape(ArrayList<String> filenames, ArrayList<String> tags) {}

    public void merge(String path) {}

    public void expand(String destination, String filepath) {}

    public void expand(String destination) {}

    public void reduce(ArrayList<String> paths, boolean recursive) {}






}
