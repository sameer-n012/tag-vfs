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

        FileOutputStream fos = new FileOutputStream(this.runConfig.getArchivePathAbsolute());

        // write section 0
        fos.write(Conversion.ltoba(Archive.MAGIC_NUMBER, 2));
        int offset = 48 * 4 + 16;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 16 * 2 + FileDirectoryEntry.SIZE_BYTES * ArchiveManager.INITIAL_FILE_DIR_SLOTS;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 16 * 2 + TagDirectoryEntry.SIZE_BYTES * ArchiveManager.INITIAL_TAG_DIR_SLOTS;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 32 + 16 + TagLookupEntry.MIN_SIZE_BYTES * ArchiveManager.INITIAL_TAG_DIR_SLOTS;
        fos.write(Conversion.ltoba(offset, 6));

        // write section 1
        fos.write(Conversion.ltoba(ArchiveManager.INITIAL_FILE_DIR_SLOTS, 2));
        fos.write(Conversion.ltoba(0, 2));
        byte[] buffer = new byte[ArchiveManager.INITIAL_FILE_DIR_SLOTS * FileDirectoryEntry.SIZE_BYTES];
        fos.write(buffer);

        // write section 2
        fos.write(Conversion.ltoba(ArchiveManager.INITIAL_TAG_DIR_SLOTS, 2));
        fos.write(Conversion.ltoba(0, 2));
        buffer = new byte[ArchiveManager.INITIAL_TAG_DIR_SLOTS * TagDirectoryEntry.SIZE_BYTES];
        fos.write(buffer);

        // write section 3
        fos.write(Conversion.ltoba(ArchiveManager.INITIAL_TAG_LOOKUP_SPACE_BYTES, 4));
        fos.write(Conversion.ltoba(0, 2));
        buffer = new byte[ArchiveManager.INITIAL_TAG_LOOKUP_SPACE_BYTES];
        fos.write(buffer);

        // write section 4
        int bufSize = 1024 * 1024;
        buffer = new byte[bufSize];
        for (int i = 0; i < ArchiveManager.INITIAL_FILE_STORAGE_SPACE_BYTES/bufSize; i++) {
            fos.write(buffer);
        }
        buffer = new byte[(int) (ArchiveManager.INITIAL_FILE_STORAGE_SPACE_BYTES % bufSize)];
        fos.write(buffer);

        this.readArchiveFile();
    }

    public void readArchiveFile() throws IOException {
        this.archive = new Archive(new File(this.runConfig.getArchivePathAbsolute()));
    }

    public void open(String filename) throws IOException {
        this.cache(filename, true);
    }

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

    public void flush(Set<String> filenames, Set<String> tags) {

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

            // TODO flush file here

        }

    }

    public void flushAll() {
        this.flush(null, null);
    }

    public void destroy(Set<String> filenames, Set<String> tags) {

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

    public void remove(Set<String> filenames, Set<String> tags) {}

    public void importFiles(Set<String> paths, boolean recursive) {}

    public void addTags(Set<String> filenames, Set<String> tags) {}

    public void removeTags(Set<String> filenames, Set<String> tags) {}

    public void listFiles(Set<String> filenames, Set<String> tags) {}

    public void sizeOf(Set<String> filenames, Set<String> tags) {}

    public void apply(Set<String> filenames, Set<String> tags) {}

    public void scrape(Set<String> filenames, Set<String> tags) {}

    public void merge(File f) {}

    public void expand(String destination, String filepath) {}

    public void expand(String destination) {}

    public void reduce(Set<String> paths, boolean recursive) {}






}
