package archive;

import java.io.*;
import java.nio.MappedByteBuffer;
import java.nio.channels.Channels;
import java.nio.channels.FileChannel;
import java.util.*;
import java.util.concurrent.locks.ReentrantReadWriteLock;

import org.apache.commons.io.IOUtils;
import util.Conversion;

public class Archive {

    protected static final int MAGIC_NUMBER = 13579;

    // section indices in the archive
    protected static final byte NUMBER_SECTIONS = 5;
    protected static final byte HEAD_S = 0; // header section
    protected static final byte FLDR_S = 1; // file directory section
    protected static final byte TGDR_S = 2; // tag directory section
    protected static final byte TGLK_S = 3; // tag lookup section
    protected static final byte FLST_S = 4; // file storage section

    private final File file;

    // section offsets in bytes
    private long[] sectionOffset;
    private ReentrantReadWriteLock headL;

    private short numFileDirSlots;
    private short numFileDirSlotsUsed;
    private MappedByteBuffer fldrMBB; // TODO may delete
    private ReentrantReadWriteLock fldrL;

    private short numTagDirSlots;
    private short numTagDirSlotsUsed;
    private MappedByteBuffer tgdrMBB; // TODO may delete
    private ReentrantReadWriteLock tgdrL;

    private short tagLookupSectionSize;
    private short numTagLookupTuples;
    private MappedByteBuffer tglkMBB; // TODO may delete
    private ReentrantReadWriteLock tglkL;

    private ReentrantReadWriteLock flstL;

    protected Archive(File file) throws IOException {
        this.file = file;

        this.headL = new ReentrantReadWriteLock();
        this.fldrL = new ReentrantReadWriteLock();
        this.tgdrL = new ReentrantReadWriteLock();
        this.tglkL = new ReentrantReadWriteLock();
        this.flstL = new ReentrantReadWriteLock();

        if(!this.validateFileType()) { throw new FileNotFoundException("File is not an archive file"); }

        readSectionPointers();
        readS1Meta();
        readS2Meta();
        readS3Meta();

    }

    private boolean validateFileType() throws IOException {
        try {
            byte[] buffer = new byte[2];

            this.headL.readLock().lock();

            RandomAccessFile raf = this.craf();
            raf.seek(0);
            raf.read(buffer, 0, 2);
            raf.close();

            System.out.println(Arrays.toString(buffer));
            System.out.println(Conversion.batosh(buffer));

            return Conversion.batosh(buffer) == Archive.MAGIC_NUMBER;

        } finally {
            this.headL.readLock().unlock();
        }

    }

    private void readSectionPointers() throws IOException {

        try {
            this.sectionOffset = new long[Archive.NUMBER_SECTIONS-1];
            byte[] buffer = new byte[6];

            this.headL.readLock().lock();

            RandomAccessFile raf = this.craf();
            raf.seek(2);

            for(int i = 0; i < Archive.NUMBER_SECTIONS-1; i++) {
                raf.read(buffer, 0, 6);
                this.sectionOffset[i] = Conversion.batol(buffer, 6);
            }

            raf.close();

        } finally {
            this.headL.readLock().unlock();
        }

    }

    private void readS1Meta() throws IOException {

        try {
            byte[] buffer = new byte[2];

            this.fldrL.readLock().lock();

            RandomAccessFile raf = this.craf();

            raf.seek(this.sectionOffset[Archive.FLDR_S]);
            raf.read(buffer, 0, 2);
            this.numFileDirSlots = Conversion.batosh(buffer);

            buffer = new byte[2];

            raf.seek(this.sectionOffset[Archive.FLDR_S] + 2);
            raf.read(buffer, 0, 2);
            this.numFileDirSlotsUsed = Conversion.batosh(buffer);

            this.fldrMBB = raf.getChannel().map(FileChannel.MapMode.READ_WRITE,
                    sectionOffset[Archive.FLDR_S],
                    this.numFileDirSlots * FileDirectoryEntry.SIZE_BYTES);

            raf.close();

        } finally {
            this.fldrL.readLock().unlock();
        }

    }

    private void readS2Meta() throws IOException {

        try {
            byte[] buffer = new byte[2];

            this.tgdrL.readLock().lock();

            RandomAccessFile raf = this.craf();

            raf.seek(this.sectionOffset[Archive.TGDR_S]);
            raf.read(buffer, 0, 2);
            this.numTagDirSlots =  Conversion.batosh(buffer);

            buffer = new byte[2];

            raf.seek(this.sectionOffset[Archive.TGDR_S] + 2);
            raf.read(buffer, 0, 2);
            this.numTagDirSlotsUsed = Conversion.batosh(buffer);

            this.tgdrMBB = raf.getChannel().map(FileChannel.MapMode.READ_WRITE,
                    sectionOffset[Archive.TGDR_S],
                    this.numTagDirSlots * TagDirectoryEntry.SIZE_BYTES);

            raf.close();

        } finally {
            this.tgdrL.readLock().unlock();
        }

    }

    private void readS3Meta() throws IOException {

        try {
            byte[] buffer = new byte[2];

            this.tglkL.readLock().lock();

            RandomAccessFile raf = this.craf();

            raf.seek(this.sectionOffset[Archive.TGLK_S]);
            raf.read(buffer, 0, 2);
            this.tagLookupSectionSize =  Conversion.batosh(buffer);

            buffer = new byte[2];

            raf.seek(this.sectionOffset[Archive.TGLK_S] + 2);
            raf.read(buffer, 0, 2);
            this.numTagLookupTuples = Conversion.batosh(buffer);

            this.tglkMBB = raf.getChannel().map(FileChannel.MapMode.READ_WRITE,
                    sectionOffset[Archive.TGLK_S],
                    this.tagLookupSectionSize);

            raf.close();

        } finally {
            this.tglkL.readLock().unlock();
        }

    }

    protected ArrayList<FileDirectoryEntry> getFDE(String filename) throws IOException {

        try {

            ArrayList<FileDirectoryEntry> fdes = new ArrayList<>();
            short fnHash = Archive.hashFileName(filename);
            int j = 0;
            byte[] buffer = new byte[FileDirectoryEntry.SIZE_BYTES];

            this.fldrL.readLock().lock();

            seek(this.fldrMBB, 4);

            for (int i = 0; i < this.numFileDirSlots; i++) {
                this.fldrMBB.get(buffer, 0, FileDirectoryEntry.SIZE_BYTES);

                // check if valid bit is true
                if(buffer[4] % 2 == 1) {
                    // increment number of valid fdes for early break
                    j++;

                    // check if file name hash matches fde file name hash
                    if(fnHash == (buffer[7] >> 8) + buffer[8]) { fdes.add(new FileDirectoryEntry(buffer, (short) i)); }
                }

                // break early if already checked all valid fdes
                if(j == numFileDirSlotsUsed) { break; }

            }

            for (Iterator<FileDirectoryEntry> it = fdes.iterator(); it.hasNext(); ) {
                if (filename.equals(this.getFM(it.next().getFileOffset()).getFilename())) {
                    it.remove();
                }
            }

            return fdes;

        } finally {
            this.fldrMBB.rewind();
            this.fldrL.readLock().unlock();
        }
    }

    protected FileDirectoryEntry getFDE(short fileno) throws IOException {

        try {

            int j = 0;
            byte[] buffer = new byte[FileDirectoryEntry.SIZE_BYTES];

            this.fldrL.readLock().lock();

            seek(this.fldrMBB, 4);
            seek(this.fldrMBB, FileDirectoryEntry.SIZE_BYTES * (fileno-1));

            this.fldrMBB.get(buffer, 0, FileDirectoryEntry.SIZE_BYTES);

            return new FileDirectoryEntry(buffer, fileno);

        } finally {
            this.fldrMBB.rewind();
            this.fldrL.readLock().unlock();
        }

    }

    protected FileMetadata getFM(long offset) throws IOException {

        try {

            byte[] buffer = new byte[2];

            this.flstL.readLock().lock();

            RandomAccessFile raf = this.craf();

            raf.seek(this.sectionOffset[Archive.FLST_S] + offset);
            raf.seek(10);
            raf.read(buffer, 0, 1);
            byte nameSize = buffer[0];
            raf.read(buffer, 0, 2);
            short numTags = Conversion.batosh(buffer);
            int metadataLength = FileMetadata.MIN_SIZE_BYTES + numTags*FileMetadata.TAG_SIZE_BYTES + nameSize;

            buffer = new byte[metadataLength];
            raf.seek(this.sectionOffset[Archive.FLST_S] + offset);
            raf.read(buffer, 0, metadataLength);

            return new FileMetadata(buffer);

        } finally {
            this.flstL.readLock().unlock();
        }

    }

    protected void writeFileDataToStream(long offset, long length, OutputStream os) throws IOException {
        try {
            int bufSize = 1024*1024;
            byte[] buffer = new byte[bufSize];

            this.flstL.readLock().lock();

            RandomAccessFile raf = this.craf();
            raf.seek(this.sectionOffset[Archive.FLST_S] + offset);

            ByteArrayOutputStream baos = new ByteArrayOutputStream();

            for(int i = 0; i < length/bufSize; i++) {
                raf.read(buffer, 0, bufSize);
                baos.write(buffer, 0, bufSize);
                baos.writeTo(os);
                baos.reset();
            }
            buffer = new byte[(int) (length % bufSize)];
            raf.read(buffer, 0, (int) (length % bufSize));
            baos.write(buffer, 0, (int) (length % bufSize));
            baos.writeTo(os);
            baos.reset();

        } finally {
            this.flstL.readLock().unlock();
        }
    }

    protected void close() throws IOException { }

    private static int sread(RandomAccessFile file, byte[] arr, int offset, int length) throws IOException {
        file.seek(offset);
        return file.read(arr, 0, length);
    }

    private RandomAccessFile craf() throws FileNotFoundException {
        return new RandomAccessFile(this.file, "rws");
    }

    private static short hashFileName(String filename) {
        return (short) (filename.hashCode() >> 16);
    }

    private static void seek(MappedByteBuffer mbb, int n) {
        mbb.position(mbb.position() + n);
    }

}
