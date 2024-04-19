package archive;

import app.RunConfiguration;
import data.FileInstance;
import loader.FileImporter;
import util.Conversion;

import java.awt.*;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;

public class ArchiveManager {

    private static final int INITIAL_FILE_DIR_SLOTS = 1024;
    private static final int INITIAL_TAG_DIR_SLOTS = 256;
    private static final int INITIAL_TAG_LOOKUP_SLOTS = 1024;
    private static final int INITIAL_TAG_LOOKUP_SPACE_BYTES = INITIAL_TAG_LOOKUP_SLOTS * TagLookupEntry.MIN_SIZE_BYTES;
    private static final long INITIAL_FILE_STORAGE_SPACE_BYTES = 1024*1024*1024; // 1 GB

    private Archive archive;
    private RunConfiguration runConfig;

    public HashSet<Short> openFiles;
    public HashMap<Short, String> cacheFileNames;
    public FileImporter cacheFileLoader;

    public ArchiveManager(RunConfiguration runConfig) {
        this.runConfig = runConfig;
        this.openFiles = new HashSet<>();
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

    public ArrayList<FileInstance> openFile(String filename) throws IOException {
        ArrayList<FileDirectoryEntry> fdes = this.archive.getFDE(filename);
        ArrayList<FileInstance> out = new ArrayList<>(fdes.size());

        for(FileDirectoryEntry fde : fdes) {
            if(this.openFiles.contains(fde.getFileno())) {
                if(!this.runConfig.getConfigBool("gui")) {
                    System.out.println("File " + filename + " is already open as \"" +
                            this.cacheFileNames.get(fde.getFileno()) + "\"");
                }
                File f = new File(this.runConfig.getCachePathAbsolute() +
                        this.cacheFileNames.get(fde.getFileno()));
                Desktop.getDesktop().open(f);
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
            this.openFiles.add(fde.getFileno());
            File f = new File(this.runConfig.getCachePathAbsolute() +
                    cacheFilename);
            Desktop.getDesktop().open(f);
            out.add(cacheFileLoader.loadFile(f.getAbsolutePath(), false));

            if(!this.runConfig.getConfigBool("gui")) {
                System.out.println("Opened " + filename + " as \"" +
                        this.cacheFileNames.get(fde.getFileno()) + "\"");
            }

        }

        return out;

    }




}
