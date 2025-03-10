package archive;

import java.io.*;
import java.nio.MappedByteBuffer;
import java.nio.channels.FileChannel;
import java.util.*;
import java.util.concurrent.locks.ReentrantReadWriteLock;

import util.Conversion;

public class Archive {

    private static final int MAGIC_NUMBER = 13579;

    private static final String ARCHIVE_COPY_TMP_FILENAME = "_archive_copy_tmp.dat";
    private static final String ARCHIVE_BACKUP_FILENAME = "_archive_copy.dat.bak";

    // section indices in the archive
    private static final byte NUMBER_SECTIONS = 5;
    private static final byte HEAD_S = 0; // header section
    private static final byte FLDR_S = 1; // file directory section
    private static final byte TGDR_S = 2; // tag directory section
    private static final byte TGLK_S = 3; // tag lookup section
    private static final byte FLST_S = 4; // file storage section

    private static final int MAX_FILE_DIR_SLOTS = 1 << 16;
    private static final int MAX_TAG_DIR_SLOTS = 1 << 16;

    protected static final double RESIZE_FILL_FACTOR_THRESHOLD = 0.5;
    protected static final int RESIZE_FACTOR = 2;

    private File file;

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

    private long tagLookupSectionSize; // not including the first 6 bytes
    private long tagLookupSectionSizeUsed;
    private short numTagLookupTuples;
    private MappedByteBuffer tglkMBB; // TODO may delete
    private ReentrantReadWriteLock tglkL;

    private long fileStorageSectionSize;
    private long fileStorageSectionSizeUsed;
    private ReentrantReadWriteLock flstL;

    /**
     * Constructor to create the archive object from a file. Sets up locks for the archive file,
     * validates that it is an archive file, and reads the metadata for each section.
     *
     * @param file where read the archive from.
     * @throws IOException if failed to read from the file.
     */
    protected Archive(File file) throws IOException {
        this.file = file;

        this.headL = new ReentrantReadWriteLock();
        this.fldrL = new ReentrantReadWriteLock();
        this.tgdrL = new ReentrantReadWriteLock();
        this.tglkL = new ReentrantReadWriteLock();
        this.flstL = new ReentrantReadWriteLock();

        if(!this.validateFileType()) { throw new FileNotFoundException("File is not an archive file"); }

        this.readSectionPointers();
        this.readS1Meta();
        this.readS2Meta();
        this.readS3Meta();
        this.readS4Meta();

    }

    /**
     * Copies the archive file to a backup with name given by Archive.ARCHIVE_BACKUP_FILENAME preceded
     * by a number.
     *
     * @return the backup file.
     * @throws IOException if failed to read the archive file or write to the backup file.
     */
    protected File backupArchive() throws IOException {

        try {
            this.headL.readLock().lock();
            this.fldrL.readLock().lock();
            this.tgdrL.readLock().lock();
            this.tglkL.readLock().lock();
            this.flstL.readLock().lock();

            String currFilepath = this.file.getAbsolutePath();
            String fileDir = this.file.getParent();

            int c = 0;
            File outFile;
            while((outFile = new File(fileDir + File.separator + c + Archive.ARCHIVE_BACKUP_FILENAME)).exists()) { c++; }

            FileInputStream fis = new FileInputStream(this.file);
            FileOutputStream fos = new FileOutputStream(outFile);

            byte[] buffer = new byte[1024*1024];
            int bytesRead = 0;
            while((bytesRead = fis.read(buffer, 0, 1024*1024)) != -1) {
                fos.write(buffer, 0, bytesRead);
            }

            fos.flush();

            return outFile;

        } finally {

            this.headL.readLock().unlock();
            this.fldrL.readLock().unlock();
            this.tgdrL.readLock().unlock();
            this.tglkL.readLock().unlock();
            this.flstL.readLock().unlock();

        }

    }

    /**
     * Resizes the archive based on the current fill factor. For each section, if the space used
     * is greater than Archive.RESIZE_FILL_FACTOR_THRESHOLD the archive section size is multiplied by
     * Archive.RESIZE_FACTOR. The data is copied to a temporary file given by Archive.ARCHIVE_COPY_TMP_FILENAME
     * before the temporary file is renamed to the original file.
     *
     * @throws IOException if failed to read the archive file or write to the resized file.
     */
    protected void resizeArchive() throws IOException {

        try {
            this.headL.writeLock().lock();
            this.fldrL.writeLock().lock();
            this.tgdrL.writeLock().lock();
            this.tglkL.writeLock().lock();
            this.flstL.writeLock().lock();

            String currFilepath = this.file.getAbsolutePath();
            String fileDir = this.file.getParent();

            int c = 0;
            File outFile;
            while((outFile = new File(fileDir + File.separator + c + Archive.ARCHIVE_BACKUP_FILENAME)).exists()) { c++; }

            FileInputStream fis = new FileInputStream(this.file);
            FileOutputStream fos = new FileOutputStream(outFile);

            // TODO write sections correctly here

            int newNumFileDirSlots = this.numFileDirSlots;
            if(this.numFileDirSlotsUsed > this.numFileDirSlots * Archive.RESIZE_FILL_FACTOR_THRESHOLD) {
                newNumFileDirSlots = Math.max(this.numFileDirSlots * Archive.RESIZE_FACTOR, Archive.MAX_FILE_DIR_SLOTS);
            }
            int newNumTagDirSlots = this.numTagDirSlots;
            if(this.numTagDirSlotsUsed > this.numTagDirSlots * Archive.RESIZE_FILL_FACTOR_THRESHOLD) {
                newNumTagDirSlots = Math.max(this.numTagDirSlots * Archive.RESIZE_FACTOR, Archive.MAX_TAG_DIR_SLOTS);
            }
            long newTagLookupSectionSize = this.tagLookupSectionSize;
            if(this.tagLookupSectionSizeUsed > this.tagLookupSectionSize * Archive.RESIZE_FILL_FACTOR_THRESHOLD) {
                newTagLookupSectionSize = this.tagLookupSectionSize * Archive.RESIZE_FACTOR;
            }
            long newFileStorageSectionSize = this.fileStorageSectionSize;
            if(this.fileStorageSectionSizeUsed > this.fileStorageSectionSize * Archive.RESIZE_FILL_FACTOR_THRESHOLD) {
                newFileStorageSectionSize = this.fileStorageSectionSize * Archive.RESIZE_FACTOR;
            }

            int bufSize = 1024*1024;
            byte[] buffer = new byte[bufSize];
            int bytesRead = 0;
            long bytesLeft = 0;


            // write section 0
            fos.write(Conversion.ltoba(Archive.MAGIC_NUMBER, 2));
            int offset = 48 * 4 + 16;
            fos.write(Conversion.ltoba(offset, 6));
            offset += 16 * 2 + FileDirectoryEntry.SIZE_BYTES * newNumFileDirSlots;
            fos.write(Conversion.ltoba(offset, 6));
            offset += 16 * 2 + TagDirectoryEntry.SIZE_BYTES * newNumTagDirSlots;
            fos.write(Conversion.ltoba(offset, 6));
            offset += 32 + 16 + newTagLookupSectionSize;
            fos.write(Conversion.ltoba(offset, 6));

            // write section 1
            fos.write(Conversion.ltoba(newNumFileDirSlots, 2));
            fis.skip(2);
            bytesLeft = this.numFileDirSlots * FileDirectoryEntry.SIZE_BYTES + 2;
            while((bytesRead = fis.read(buffer, 0, (int) Math.min(bufSize, bytesLeft))) != 0) {
                fos.write(buffer, 0, bytesRead);
                bytesLeft -= bytesRead;
            }
            Archive.writeEmpty(fos, (newNumFileDirSlots - this.numFileDirSlots) * FileDirectoryEntry.SIZE_BYTES);

            // write section 2
            fos.write(Conversion.ltoba(newNumFileDirSlots, 2));
            fis.skip(2);
            bytesLeft = this.numTagDirSlots * TagDirectoryEntry.SIZE_BYTES + 2;
            while((bytesRead = fis.read(buffer, 0, (int) Math.min(bufSize, bytesLeft))) != 0) {
                fos.write(buffer, 0, bytesRead);
                bytesLeft -= bytesRead;
            }
            Archive.writeEmpty(fos, (newNumTagDirSlots - this.numTagDirSlots) * TagDirectoryEntry.SIZE_BYTES);

            // write section 3
            fos.write(Conversion.ltoba(newTagLookupSectionSize, 2));
            fis.skip(2);
            bytesLeft = this.tagLookupSectionSize + 2;
            while((bytesRead = fis.read(buffer, 0, (int) Math.min(bufSize, bytesLeft))) != 0) {
                fos.write(buffer, 0, bytesRead);
                bytesLeft -= bytesRead;
            }
            Archive.writeEmpty(fos, newTagLookupSectionSize - this.tagLookupSectionSize);

            // write section 4
            bytesLeft = this.fileStorageSectionSize;
            while((bytesRead = fis.read(buffer, 0, (int) Math.min(bufSize, bytesLeft))) != 0) {
                fos.write(buffer, 0, bytesRead);
                bytesLeft -= bytesRead;
            }

            long fileLength = newFileStorageSectionSize - this.fileStorageSectionSize -
                    FileMetadata.MIN_SIZE_BYTES - FileEndMetadata.SIZE_BYTES;
            FileMetadata fm = new FileMetadata(fileLength,
                    false, (short) -1, (short) -1, (byte) 0, null, null);
            fos.write(fm.toBytes());
            Archive.writeEmpty(fos, fileLength);
            FileEndMetadata fme = new FileEndMetadata(fileLength);
            fos.write(fme.toBytes());

            fos.flush();

            if(!this.file.delete()) {
                throw new IOException("Failed to resize archive");
            }
            this.file = new File(currFilepath);
            if(!outFile.renameTo(this.file)) {
                throw new IOException("Failed to resize archive");
            }
            this.file = new File(currFilepath);


        } finally {

            this.headL.writeLock().unlock();
            this.fldrL.writeLock().unlock();
            this.tgdrL.writeLock().unlock();
            this.tglkL.writeLock().unlock();
            this.flstL.writeLock().unlock();

        }

        this.readSectionPointers();
        this.readS1Meta();
        this.readS2Meta();
        this.readS3Meta();
        this.readS4Meta();

    }

    /**
     * Validates that the file given is an archive file for this application using the magic number.
     *
     * @return true if the file is valid and false otherwise.
     * @throws IOException if it failed to read the archive file.
     */
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

    /**
     * Reads the pointers to each section found in the archive header.
     *
     * @throws IOException if it failed to read the archive file.
     */
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

    /**
     * Reads the metadata found in the file directory section including
     * current storage section fill, total slots, and slots used.
     *
     * @throws IOException if it failed to read the archive file.
     */
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

            long bytesRead = 0;
            int filesFound = 0;
            long spaceUsed = 0;
            buffer = new byte[5];
            while(bytesRead < this.numFileDirSlots * FileDirectoryEntry.SIZE_BYTES && filesFound < this.numFileDirSlotsUsed) {
                raf.read(buffer, 0, 5);
                long l = Conversion.batol(buffer, 5);
                if(l % 2 == 1) {
                    spaceUsed += l >> 1;
                    filesFound++;
                }
                bytesRead += FileDirectoryEntry.SIZE_BYTES - 5;
            }
            this.fileStorageSectionSizeUsed = spaceUsed;

            raf.close();

        } finally {
            this.fldrL.readLock().unlock();
        }

    }

    /**
     * Reads the metadata found in the tag directory section including
     * total slots and slots used.
     *
     * @throws IOException if it failed to read the archive file.
     */
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

    /**
     * Reads the metadata found in the tag lookup section including
     * total section size, space used, and number of lookup tuples.
     *
     * @throws IOException if it failed to read the archive file.
     */
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

            long bytesRead = 0;
            int tuplesFound = 0;
            int numFileSlots = 0;
            long spaceUsed = 0;
            while(bytesRead < this.tagLookupSectionSize && tuplesFound < this.numTagLookupTuples) {
                if((raf.readByte() & 0x8000) != 0) {
                    raf.skipBytes(1);
                    numFileSlots = raf.readByte() & 0xffff;
                    raf.skipBytes(2 + 2*numFileSlots + 5);
                    spaceUsed += 2 + 1 + 2 + 2*numFileSlots + 5;
                    bytesRead += 2 + 1 + 2 + 2*numFileSlots + 5;
                    tuplesFound++;
                } else {
                    raf.skipBytes(1);
                    raf.skipBytes(2 + (2*0) + 5);
                    bytesRead += 2 + 1 + 2 + 2*0 + 5;
                }
            }

            this.tagLookupSectionSizeUsed = spaceUsed;

            raf.close();

        } finally {
            this.tglkL.readLock().unlock();
        }

    }

    /**
     * Reads the metadata found in the file storage section including
     * total section size.
     *
     * @throws IOException if it failed to read the archive file.
     */
    private void readS4Meta() throws IOException {

        try {

            this.flstL.readLock().lock();

            RandomAccessFile raf = this.craf();

            raf.seek(this.sectionOffset[Archive.FLST_S]);

            this.fileStorageSectionSize = raf.length() - raf.getFilePointer();

            raf.close();

        } finally {
            this.flstL.readLock().unlock();
        }

    }

    /**
     * Gets the corresponding list of file directory entries that match
     * the given filename. Note that multiple files can have the same filename.
     * Uses the filename hash to quickly match filenames before checking the file metadata.
     *
     * @param filename the filename to search for.
     * @return an arraylist of file directory entries.
     * @throws IOException if it failed to read the archive file.
     */
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

    /**
     * Gets the corresponding file directory entry that matches
     * the given file number.
     *
     * @param fileno the file number to search for.
     * @return a file directory entry.
     * @throws IOException if it failed to read the archive file.
     */
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

    /**
     * Gets the file metadata located at a specific offset.
     *
     * @param offset the offset into the file storage section where the metadata is located.
     * @return the file metadata.
     * @throws IOException if it failed to read the archive file.
     */
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

    /**
     * Creates the file directory entry in the file directory entry section. Will attempt to resize
     * the archive if there is no space, and if unable to do so, will not create the entry.
     *
     * @param length the length of the file.
     * @param parent the file number of the parent of the file (-1 if parent is root).
     * @param filename the name of the file.
     * @param offset the offset into the file storage section at which the file (and its metadata) is
     *               located in the file storage section.
     * @return the new file directory entry, or null if none was able to be created.
     * @throws IOException if it failed to read the archive file.
     */
    private FileDirectoryEntry createFDE(long length, short parent, String filename, long offset) throws IOException {
        if(this.numFileDirSlotsUsed >= this.numFileDirSlots) {
            if(this.numFileDirSlots >= Short.MAX_VALUE) { // TODO fix, make unsigned
                throw new IOException("Not enough space to write file metadata");
            } else {
                this.resizeArchive();
            }
        }

        this.fldrL.writeLock().lock();

        for(int i = 0; i < this.numFileDirSlots; i++) {
            byte b = this.fldrMBB.get(i*FileDirectoryEntry.SIZE_BYTES + 5);
            if(b % 2 == 0) {
                FileDirectoryEntry fde = new FileDirectoryEntry((short) i, length, true, parent,
                        Archive.hashFileName(filename), offset);
                this.fldrMBB.put(i*FileDirectoryEntry.SIZE_BYTES, fde.toBytes(), 0, FileDirectoryEntry.SIZE_BYTES);
                this.numFileDirSlotsUsed++;
                return fde;
            }

        }

        return null;

    }

    private void deleteFDE(short fileno) {
        this.numFileDirSlotsUsed--;
        // TODO
    }

    /**
     * Finds space in the file storage section to place a file. If no space is found, will return -1.
     * The space found should be a contiguous block large enough for the entire file, its metadata,
     * and its end-metadata.
     *
     * @param length the size of the file.
     * @param filenameSize the length of the filename.
     * @param numTags the number of tags on the file.
     * @return the offset into the file storage section at which there is free space
     *         (starting from the metadata location), or -1 if there is no space.
     * @throws IOException if it failed to read the archive file.
     */
    private long findFileSpace(long length, short filenameSize, short numTags) throws IOException {

        if(this.fileStorageSectionSize - this.fileStorageSectionSizeUsed < length) {
            return -1;
        }

        long lenReq = length + FileMetadata.calculateMetadataLength(filenameSize, numTags) +
                FileEndMetadata.SIZE_BYTES;

        try {

            flstL.readLock().lock();
            RandomAccessFile raf = this.craf();

            raf.seek(this.sectionOffset[Archive.FLST_S]);
            long offset = 0;
            while(offset < this.fileStorageSectionSize) {

                byte[] buffer = new byte[FileMetadata.MIN_SIZE_BYTES];
                raf.read(buffer, 0, FileMetadata.MIN_SIZE_BYTES);
                FileMetadata fm = new FileMetadata(buffer);

                if(fm.getValid() || fm.getLength() < lenReq) {
                    offset += fm.getLength() + fm.getMetadataLength() + FileEndMetadata.SIZE_BYTES;
                    raf.seek(this.sectionOffset[Archive.FLST_S] + offset);
                } else {
                    return offset;
                }

            }

        } finally {
            flstL.readLock().unlock();
        }


        return -1;
    }

    /**
     * Creates space for a file at a given offset by writing the file metaadta and file end-metadata.
     *
     * @param offset the offset into the file storage section indicating the beginning of the file metadata.
     * @param length the length of the file.
     * @param fileno the file number.
     * @param parent the parent of the file (-1 if the parent is root).
     * @param type the type of file.
     * @param filename the name of the file.
     * @param tags a list of tag IDs for the file
     * @return the file metadata created.
     * @throws IOException if it failed to read the archive file.
     */
    private FileMetadata createFileSpace(long offset, long length, short fileno, short parent, byte type, String filename, short[] tags) throws IOException {
        // TODO

        try {
            this.flstL.writeLock().lock();

            RandomAccessFile raf = this.craf();
            raf.seek(sectionOffset[Archive.FLST_S] + offset);

            FileMetadata fm = new FileMetadata(length, true, fileno, parent, type, filename, tags);
            raf.write(fm.toBytes(), 0, fm.getMetadataLength());

            raf.seek(sectionOffset[Archive.FLST_S] + offset + length);

            FileEndMetadata fme = new FileEndMetadata(length);
            raf.write(fme.toBytes(), 0, fme.getEndMetadataLength());

            return fm;

        } finally {

            this.flstL.writeLock().unlock();

        }

    }

    /**
     * Creates a tag by creating the corresponding tag directory entry and initial tag lookup tuple.
     * Will try to resize archive if there is not enough space. Does not add the tag if a tag with the same
     * name already exists.
     *
     * @param tag the name of the tag to add.
     * @return the tag directory entry of the new tag.
     * @throws IOException if it failed to read the archive file.
     */
    private TagDirectoryEntry createTag(String tag) throws IOException {
        // TODO

        try {
            TagDirectoryEntry tde = null;
            if ((tde = this.getTDE(tag)) != null) {
                return tde;
            }

            // resize if necessary
            if(this.numTagDirSlotsUsed >= this.numTagDirSlots ||
                    this.tagLookupSectionSizeUsed + TagLookupEntry.MIN_SIZE_BYTES>= this.tagLookupSectionSize) {
                this.resizeArchive();
            }

            this.tglkL.writeLock().lock();

            seek(this.tglkMBB, 4);
            byte[] tmp = new byte[3];
            int off = 0;
            TagLookupEntry tle = null;
            while(off <= this.tagLookupSectionSize) {

                this.tglkMBB.get(tmp, 0, 3);

                int size = 3 + 2 + tmp[2]*2 + 5;

                // check if valid is false, then found
                if(tmp[0] < 0 && size >= TagLookupEntry.MIN_SIZE_BYTES) { break; }

            }

            this.tgdrL.writeLock().lock();

            seek(this.tgdrMBB, 4);

            byte[] buffer = new byte[TagDirectoryEntry.SIZE_BYTES];

            short tidx = 0;
            for (int i = 0; i < this.numTagDirSlots; i++) {
                this.tgdrMBB.get(buffer, 0, TagDirectoryEntry.SIZE_BYTES);

                // check if valid bit is false (we can insert here)
                if(buffer[0] >> 3 == 0) {
                    // create tde
                    tidx = (short) i;
                    tde = new TagDirectoryEntry((short) i, true, tag, off);

                    // rewind so we can put data in
                    seek(this.tgdrMBB, -TagDirectoryEntry.SIZE_BYTES);
                    this.tgdrMBB.put(tde.toBytes());
                }
            }

            tglkMBB.rewind();
            seek(tglkMBB, off);
            tle = new TagLookupEntry(tidx, true, (byte) TagLookupEntry.INITIAL_NUMBER_SLOTS,
                    (short) 0, null, -1);
            tglkMBB.put(tle.toBytes());

            this.numTagDirSlotsUsed++;
            this.tagLookupSectionSizeUsed += tle.getLength();

            return tde;

        } finally {

            this.tgdrMBB.rewind();
            this.tglkMBB.rewind();

            this.tgdrL.writeLock().unlock();
            this.tglkL.writeLock().unlock();

        }

    }

    private void removeTag(short tagno) {
        // TODO
        this.numTagDirSlotsUsed--;
    }

    private TagDirectoryEntry addTagToFile(short tagno, short fileno) {
        // TODO add to tag lookup/dir + file metadata
        return null;
    }

    private TagLookupEntry addTagToFileInTagLookup(short tagno, short fileno) {
        // TODO
        return null;
    }

    private void removeTagFromFile(short tagno, short fileno) {
        // TODO remove from tag lookup + file metadata
        // remove from dir if no references
    }

    private void removeTagFromFileInTagLookup(short tagno, short fileno) {
        // TODO
    }

    /**
     * Searches for a valid tag directory entry with a tag that matches the name
     * given.
     *
     * @param tag the name of the tag to search for.
     * @return the tag directory entry found.
     * @throws IOException if it failed to read the archive file.
     */
    protected TagDirectoryEntry getTDE(String tag) throws IOException {

        try {

            int j = 0;
            byte[] buffer = new byte[TagDirectoryEntry.SIZE_BYTES];
            byte[] name = new byte[16];

            this.tgdrL.readLock().lock();

            seek(this.tgdrMBB, 4);

            for (int i = 0; i < this.numTagDirSlots; i++) {
                this.tgdrMBB.get(buffer, 0, TagDirectoryEntry.SIZE_BYTES);

                // check if valid bit is true
                if(buffer[0] < 0) {
                    // increment number of valid tdes for early break
                    j++;

                    // check if tag name matches
                    System.arraycopy(buffer, 2, name, 0, 16);
                    if(tag.equals(new String(name))) { return new TagDirectoryEntry(buffer, (short) i); }
                }

                // break early if already checked all valid tdes
                if(j == numTagDirSlotsUsed) { break; }

            }

        } finally {
            this.tgdrMBB.rewind();
            this.tgdrL.readLock().unlock();
        }


        return null;
    }

    private void defragmentFileStorage() {
        // TODO
    }

    private void defragmentTagLookup() {
        // TODO
    }

    private void coalesceFileStorage(long offset) {
        // TODO
    }

    private void coalesceTagLookup(long offset) {
        // TODO
    }

    protected void delete(short fileno) {
        // TODO
    }

    /**
     * Writes a new file in the archive using bytes from an input stream. Finds space for the file, or tries to
     * resize the archive if space is not available.
     *
     * @param is the input stream to read from.
     * @param length the length of the file to write.
     * @param parent the file number of the parent (-1 if root).
     * @param filename the file name.
     * @param tags a list of tag IDs for the file
     * @return the file number of the created file.
     * @throws IOException if it failed to read the archive file.
     */
    protected long write(InputStream is, long length, short parent, String filename, short[] tags) throws IOException {

        try {

            flstL.readLock().lock();

            long offset = this.findFileSpace(length, (short) filename.length(), (short) tags.length);
            if(offset == -1) {
                this.resizeArchive();
                offset = this.findFileSpace(length, (short) filename.length(), (short) tags.length);
            }
            FileDirectoryEntry fde = this.createFDE(length, parent, filename, offset);

            byte type = 0;
            FileMetadata fm = this.createFileSpace(offset, length, fde.getFileno(), parent, type, filename, tags);

            flstL.writeLock().lock();

            RandomAccessFile raf = this.craf();

            raf.seek(this.sectionOffset[Archive.FLST_S] + offset + fm.getMetadataLength());

            int bufSize = 1024 * 1024;
            byte[] buffer = new byte[bufSize];
            int bytesRead = 0;
            while ((bytesRead = is.read(buffer, 0, bufSize)) != -1) {
                raf.write(buffer, 0, bytesRead);
            }

            raf.close();

            this.fileStorageSectionSizeUsed += fm.getMetadataLength() + fm.getLength() + FileEndMetadata.SIZE_BYTES;

            // TODO write to tag lookup

            return fde.getFileno();

        } finally {
            flstL.writeLock().unlock();
            flstL.readLock().unlock();
        }

    }

    /**
     * Writes the data in a file to an output stream. Does not write the file metadata or end-metadata to
     * the output stream. Requires the offset into the file data (not metadata).
     *
     * @param offset the offset at which the file data is located.
     * @param length the length of the file.
     * @param os the output stream to write to.
     * @throws IOException if it failed to read the archive file.
     */
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

    /**
     * Closes the archive object by closing any input streams, unlocking all locks, and
     * closing any file pointers. Currently has no effect.
     *
     * @throws IOException if it failed to read the archive file.
     */
    protected void close() throws IOException { }

    /**
     * Seeks and reads bytes from a file at a given offset.
     *
     * @param file the RandomAccessFile pointer to a file.
     * @param arr the array to read bytes into.
     * @param offset the offset in the file at which to start reading.
     * @param length the number of bytes to read from the file.
     * @return the number of bytes read from the file.
     * @throws IOException if it failed to read the archive file.
     */
    private static int sread(RandomAccessFile file, byte[] arr, int offset, int length) throws IOException {
        file.seek(offset);
        return file.read(arr, 0, length);
    }

    /**
     * Creates a new RandomAccessFile file pointer to the archive. This
     * has read and write permissions.
     *
     * @return the random access file.
     * @throws FileNotFoundException if the archive file was not found.
     */
    private RandomAccessFile craf() throws FileNotFoundException {
        return new RandomAccessFile(this.file, "rws");
    }

    /**
     * Hashes a filename using the default string hashing algorithm. Uses the last 2 bytes as the hash.
     *
     * @param filename the filename to hash.
     * @return the 2-byte hash.
     */
    private static short hashFileName(String filename) {
        return (short) (filename.hashCode() & 0xffff);
    }

    /**
     * Moves the position in a Mapped Byte Buffer forward by n bytes.
     *
     * @param mbb the mapped byte buffer.
     * @param n the number of bytes to move forward.
     */
    private static void seek(MappedByteBuffer mbb, int n) {
        mbb.position(mbb.position() + n);
    }

    /**
     * Creates an archive file by writing archive metadata to the file. The newly created archive
     * will be empty except for the metadata.
     *
     * @param f the file object to write to.
     * @param fileDirSlots the number of file directory slots to create.
     * @param tagDirSlots the number of tag directory slots to create.
     * @param tagLookupSize the size of the tag lookup section created.
     * @param fileStorageSpace the size of the file storage section created.
     * @throws IOException if it failed to read the archive file.
     * @throws SecurityException if it did not have permission to write to the file.
     */
    public static void create(File f, int fileDirSlots, int tagDirSlots, long tagLookupSize,
                              long fileStorageSpace) throws IOException, SecurityException {

        FileOutputStream fos = new FileOutputStream(f);

        // write section 0
        fos.write(Conversion.ltoba(Archive.MAGIC_NUMBER, 2));
        int offset = 48 * 4 + 16;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 16 * 2 + FileDirectoryEntry.SIZE_BYTES * fileDirSlots;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 16 * 2 + TagDirectoryEntry.SIZE_BYTES * tagDirSlots;
        fos.write(Conversion.ltoba(offset, 6));
        offset += 32 + 16 + TagLookupEntry.MIN_SIZE_BYTES * tagLookupSize;
        fos.write(Conversion.ltoba(offset, 6));

        // write section 1
        fos.write(Conversion.ltoba(fileDirSlots, 2));
        fos.write(Conversion.ltoba(0, 2));
        Archive.writeEmpty(fos, fileDirSlots * (long) FileDirectoryEntry.SIZE_BYTES);

        // write section 2
        fos.write(Conversion.ltoba(tagDirSlots, 2));
        fos.write(Conversion.ltoba(0, 2));
        Archive.writeEmpty(fos, tagDirSlots * (long) TagDirectoryEntry.SIZE_BYTES);

        // write section 3
        fos.write(Conversion.ltoba(tagLookupSize, 4));
        fos.write(Conversion.ltoba(0, 2));
        Archive.writeEmpty(fos, tagLookupSize);

        // write section 4
        long fileLength = fileStorageSpace - FileMetadata.MIN_SIZE_BYTES - FileEndMetadata.SIZE_BYTES;
        FileMetadata fm = new FileMetadata(fileLength,
                false, (short) -1, (short) -1, (byte) 0, null, null);
        fos.write(fm.toBytes());
        Archive.writeEmpty(fos, fileLength);
        FileEndMetadata fme = new FileEndMetadata(fileLength);
        fos.write(fme.toBytes());

    }

    /**
     * Writes a given number of bytes to a file output stream. Writes in block sizes of 1 MB.
     *
     * @param os the file output stream.
     * @param size the number of bytes to write.
     * @throws IOException if it failed to read the archive file.
     */
    private static void writeEmpty(FileOutputStream os, long size) throws IOException {

        int bufSize = 1024*1024;

        byte[] buffer = new byte[bufSize];
        for (int i = 0; i < size/bufSize; i++) {
            os.write(buffer);
        }
        buffer = new byte[(int) (size % bufSize)];
        os.write(buffer);

    }

}
