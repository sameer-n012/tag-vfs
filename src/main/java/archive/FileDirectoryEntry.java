package archive;

import util.Conversion;

import java.util.BitSet;

public class FileDirectoryEntry {

    public static final int SIZE_BYTES = 112/8;

    // all start values are inclusive and end values are exclusive
    // all values are given in bits
    private static final int LENGTH_INDEX_START = 0;
    private static final int LENGTH_INDEX_END = 39;
    private static final int VALID_INDEX_START = 39;
    private static final int VALID_INDEX_END = 40;
    private static final int PARENT_INDEX_START = 40;
    private static final int PARENT_INDEX_END = 56;
    private static final int FNAME_INDEX_START = 56;
    private static final int FNAME_INDEX_END = 72;
    private static final int FPTR_INDEX_START = 72;
    private static final int FPTR_INDEX_END = 112;

    private BitSet fde;
    private short fileno;

    public FileDirectoryEntry(byte[] fde, short fileno) {
        if(fde.length != SIZE_BYTES) {
            throw new IllegalArgumentException("Invalid size file directory entry");
        }
        this.fde = BitSet.valueOf(fde);
        this.fileno = fileno;
    }

    public short getFileno() { return this.fileno; }

    public boolean getValid() {
        return this.fde.get(VALID_INDEX_START);
    }

    public long getLength() {
        return Conversion.batol(this.fde.get(LENGTH_INDEX_START, LENGTH_INDEX_END).toByteArray(), 5) >> 1;
    }

    public short getParent() {
        return Conversion.batosh(this.fde.get(PARENT_INDEX_START, PARENT_INDEX_END).toByteArray());
    }

    public short getFilenameHash() {
        return Conversion.batosh(this.fde.get(FNAME_INDEX_START, FNAME_INDEX_END).toByteArray());
    }

    public long getFileOffset() {
        return Conversion.batol(this.fde.get(FPTR_INDEX_START, FPTR_INDEX_END).toByteArray(), 5);
    }


}
