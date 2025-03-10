package archive;

public class TagLookupEntry {

    public static final int MIN_SIZE_BYTES = (80 + 16*16)/8;
    public static final int INITIAL_NUMBER_SLOTS = 16;
    public static final int SLOT_EXPANSION_FACTOR = 2;

    // TODO
    public TagLookupEntry(short tagno, boolean valid, byte slots, short numfiles, short[] files, long offset) {}

    // TODO
    public byte[] toBytes() { return null; }

    public int getLength() { return 0; }



}
