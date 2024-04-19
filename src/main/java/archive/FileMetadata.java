package archive;

import util.Conversion;

import java.nio.charset.StandardCharsets;
import java.util.BitSet;

public class FileMetadata {

    public static final int MIN_SIZE_BYTES = 104/8;
    public static final int TAG_SIZE_BYTES = 16/2;

    // all start values are inclusive and end values are exclusive
    // all values are given in bits
    private static final int LENGTH_INDEX_START = 0;
    private static final int LENGTH_INDEX_END = 39;
    private static final int VALID_INDEX_START = 39;
    private static final int VALID_INDEX_END = 40;
    private static final int FLNO_INDEX_START = 40;
    private static final int FLNO_INDEX_END = 56;
    private static final int PARENT_INDEX_START = 56;
    private static final int PARENT_INDEX_END = 72;
    private static final int TYPE_INDEX_START = 72;
    private static final int TYPE_INDEX_END = 80;
    private static final int NMSZ_INDEX_START = 80;
    private static final int NMSZ_INDEX_END = 88;
    private static final int TGNO_INDEX_START = 88;
    private static final int TGNO_INDEX_END = 104;
    private static final int TAGS_INDEX_START = 104;

    private BitSet fm;
    private int length;
    private int noTags;
    private int nmSz;

    public FileMetadata(byte[] fm) {
        if(fm.length < MIN_SIZE_BYTES) {
            throw new IllegalArgumentException("Invalid size file directory entry");
        }
        this.fm = BitSet.valueOf(fm);
        this.length = fm.length;
        this.noTags = this.getNumberTags();
        this.nmSz = this.getFilenameSize();
    }

    public boolean getValid() {
        return this.fm.get(VALID_INDEX_START);
    }

    public long getLength() {
        return Conversion.batol(this.fm.get(LENGTH_INDEX_START, LENGTH_INDEX_END).toByteArray(), 5) >> 1;
    }

    public short getFileno() {
        return Conversion.batosh(this.fm.get(FLNO_INDEX_START, FLNO_INDEX_END).toByteArray());
    }

    public short getParent() {
        return Conversion.batosh(this.fm.get(PARENT_INDEX_START, PARENT_INDEX_END).toByteArray());
    }

    public byte getFileType() {
        return this.fm.get(TYPE_INDEX_START, TYPE_INDEX_END).toByteArray()[0];
    }

    public byte getFilenameSize() {
        return this.fm.get(NMSZ_INDEX_START, NMSZ_INDEX_END).toByteArray()[0];
    }

    public short getNumberTags() {
        return Conversion.batosh(this.fm.get(TGNO_INDEX_START, TGNO_INDEX_END).toByteArray());
    }

    public short[] getTags() {
        byte[] buffer = (this.fm.get(TGNO_INDEX_START, TGNO_INDEX_END + this.noTags*TAG_SIZE_BYTES).toByteArray());
        short[] out = new short[buffer.length/2];
        for(int i = 0; i < out.length; i++) {
            out[i] = (short) ((buffer[2*i] << 8) + buffer[2*i+1]);
        }
        return out;
    }

    public String getFilename() {
        byte[] buffer = (this.fm.get(TGNO_INDEX_END + this.noTags*TAG_SIZE_BYTES,
                TGNO_INDEX_END + this.noTags*TAG_SIZE_BYTES + this.nmSz).toByteArray());
        return new String(buffer, StandardCharsets.UTF_8);
    }

    public int getMetadataLength() {
        return this.length;
    }



}
