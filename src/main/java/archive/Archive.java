package archive;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;

import app.App;
import util.Conversion;

public class Archive {

    protected static final int MAGIC_NUMBER = 13579;

    // section indices in the archive
    protected static final byte HEAD_S = 0; // header section
    protected static final byte FLDR_S = 1; // file directory section
    protected static final byte TGDR_S = 2; // tag directory section
    protected static final byte TGLK_S = 3; // tag lookup section
    protected static final byte FLST_S = 4; // file storage section

    private final RandomAccessFile file;

    private boolean[] sectionOffset;

    private short numFileDirSlots;
    private short numFileDirSlotsUsed;
    private HashMap<Integer, FileDirectoryEntry> fileDirectory; // TODO may delete

    private byte numTagDirSlots;
    private byte numTagDirSlotsUsed;
    private HashMap<Byte, TagDirectoryEntry> tagDirectory; // TODO may delete

    private short numTagLookupSlots;
    private short numTagLookupSlotsUsed;
    ArrayList<TagLookupEntry> tagLookup; // TODO may delete


    public Archive(File file) throws IOException {
        this.file = new RandomAccessFile(file, "rws");
        byte[] buffer = new byte[4];
        this.file.read(buffer, 0, 2);
        System.out.println(Arrays.toString(buffer));
        System.out.println(Conversion.batosh(buffer));
        if(Conversion.batosh(buffer) != Archive.MAGIC_NUMBER) {
            throw new FileNotFoundException("File is not an archive file");
        }
    }

    public void close() throws IOException {
        this.file.close();
    }

}
