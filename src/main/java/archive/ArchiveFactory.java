package archive;

import util.Conversion;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;

public class ArchiveFactory {

    private static final int INITIAL_FILE_DIR_SLOTS = 1024;
    private static final int INITIAL_TAG_DIR_SLOTS = 256;
    private static final int INITIAL_TAG_LOOKUP_SLOTS = 1024;
    private static final long INITIAL_FILE_STORAGE_SPACE_BYTES = 1024*1024*1024; // 1 GB

    public static Archive createArchiveFile(String path) throws IOException, SecurityException {

        FileOutputStream fos = new FileOutputStream(path);

        // write section 0
        fos.write(Conversion.ltoba(Archive.MAGIC_NUMBER, 2));
        int offset = 48 * 4 + 16;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 16 * 2 + 112 * ArchiveFactory.INITIAL_FILE_DIR_SLOTS;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 16 * 2 + 144 * ArchiveFactory.INITIAL_TAG_DIR_SLOTS;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 16 * 2 + (32 + 16 * 256) * ArchiveFactory.INITIAL_TAG_DIR_SLOTS;
        fos.write(Conversion.ltoba(offset, 6));

        // write section 1
        fos.write(Conversion.ltoba(ArchiveFactory.INITIAL_FILE_DIR_SLOTS, 2));
        fos.write(Conversion.ltoba(0, 2));
        byte[] buffer = new byte[ArchiveFactory.INITIAL_FILE_DIR_SLOTS * 112 / 8];
        fos.write(buffer);

        // write section 2
        fos.write(Conversion.ltoba(ArchiveFactory.INITIAL_TAG_DIR_SLOTS, 2));
        fos.write(Conversion.ltoba(0, 2));
        buffer = new byte[ArchiveFactory.INITIAL_TAG_DIR_SLOTS * 144 / 8];
        fos.write(buffer);

        // write section 3
        fos.write(Conversion.ltoba(ArchiveFactory.INITIAL_TAG_LOOKUP_SLOTS, 2));
        fos.write(Conversion.ltoba(0, 2));
        buffer = new byte[ArchiveFactory.INITIAL_TAG_LOOKUP_SLOTS * (32 + 16 * 256) / 8];
        fos.write(buffer);

        // write section 4
        buffer = new byte[1024 * 1024];
        for (int i = 0; i < ArchiveFactory.INITIAL_FILE_STORAGE_SPACE_BYTES/(1024*1024); i++) {
            fos.write(buffer);
        }
        buffer = new byte[(int) (ArchiveFactory.INITIAL_FILE_STORAGE_SPACE_BYTES % (1024*1024))];
        fos.write(buffer);

        return readArchiveFile(path);
    }

    public static Archive readArchiveFile(String path) throws IOException {
        return new Archive(new File(path));
    }


}
