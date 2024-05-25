package archive;

import util.Conversion;

import java.io.ByteArrayOutputStream;
import java.util.BitSet;

public class TagDirectoryEntry {

    public static final int SIZE_BYTES = 184/8;

    // all start values are inclusive and end values are exclusive
    // all values are given in bits
    private static final int VALID_INDEX_START = 0;
    private static final int VALID_INDEX_END = 1;
    private static final int LENGTH_INDEX_START = 1;
    private static final int LENGTH_INDEX_END = 16;
    private static final int TNAME_INDEX_START = 16;
    private static final int TNAME_INDEX_END = 144;
    private static final int TPTR_INDEX_START = 144;
    private static final int TPTR_INDEX_END = 184;

    private BitSet tde;
    private short tagno;

    public TagDirectoryEntry(short tagno, boolean valid, String name, long offset) {
        ByteArrayOutputStream bb = new ByteArrayOutputStream(SIZE_BYTES);
        bb.write(Conversion.ltoba(((valid ? 1 : 0) << 15) + tagno, 2), 0, 2);
        bb.write(name.getBytes(), 0, 16);
        bb.write(Conversion.ltoba(offset, 5), 0, 5);
        this.tde = BitSet.valueOf(bb.toByteArray());
        this.tagno = tagno;
    }

    public TagDirectoryEntry(byte[] tde, short tagno) {
        if(tde.length != SIZE_BYTES) {
            throw new IllegalArgumentException("Invalid size tag directory entry");
        }
        this.tde = BitSet.valueOf(tde);
        this.tagno = tagno;
    }

    public short getTagno() { return this.tagno; }

    public boolean getValid() {
        return this.tde.get(VALID_INDEX_START);
    }

    public String getTagname() {
        return new String(this.tde.get(TNAME_INDEX_START, TNAME_INDEX_END).toByteArray());
    }

    public long getTagOffset() {
        return Conversion.batol(this.tde.get(TPTR_INDEX_START, TPTR_INDEX_END).toByteArray(), 5);
    }

    public byte[] toBytes() { return tde.toByteArray(); }

}
