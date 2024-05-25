package archive;

import util.Conversion;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.BitSet;

public class FileEndMetadata {

    public static final int SIZE_BYTES = 40/8;


    private BitSet fm;
    private int length;

    public FileEndMetadata(long length) {
        ByteArrayOutputStream bb = new ByteArrayOutputStream(SIZE_BYTES);
        bb.write(Conversion.ltoba(length, 5), 0, 5);
        this.fm = BitSet.valueOf(bb.toByteArray());
        this.length = this.fm.length();
    }

    public FileEndMetadata(byte[] fm) {
        if(fm.length < SIZE_BYTES) {
            throw new IllegalArgumentException("Invalid size file directory entry");
        }
        this.fm = BitSet.valueOf(fm);
        this.length = fm.length;
    }

    public long getLength() {
        return Conversion.batol(this.fm.toByteArray(), 5);
    }

    public int getEndMetadataLength() {
        return this.length;
    }

    public byte[] toBytes() { return fm.toByteArray(); }




}
