package loader;

import data.*;

import javax.imageio.ImageIO;
import java.awt.image.BufferedImage;
import java.io.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.ArrayList;
import java.util.zip.DataFormatException;

public class FileImporter {

    private final String loadingDir;

    public FileImporter(String directory) { this.loadingDir = directory; }

    public Directory loadAll() throws IOException {
        return this.loadDirectory(new File(this.loadingDir).getCanonicalPath(), true);
    }

    public Directory loadDirectory(String path, boolean recursive) {
        File dir = new File(path);
        File[] files = dir.listFiles();
        if(files == null) {
            return null;
        }

        ArrayList<FileInstance> children = new ArrayList<>();
        long size = 0;

        for (File file : files) {
            FileInstance fi = null;

            if (file.isDirectory() && recursive) {
                fi = this.loadDirectory(file.getAbsolutePath(), true);
            } else if (!file.getName().contains(".")) { // if no file extension, load as plain text
                fi = this.loadPlainText(file.getAbsolutePath());
            } else if (file.getName().endsWith(".txt")) {
                fi = this.loadPlainText(file.getAbsolutePath());
            } else if (file.getName().endsWith(".rtf")) {
                fi = this.loadRichText(file.getAbsolutePath());
            } else if (file.getName().endsWith(".png")) {
                fi = this.loadPNGImage(file.getAbsolutePath());
            } else if (file.getName().endsWith(".jpg") || file.getName().endsWith(".jpeg")) {
                fi = this.loadJPGImage(file.getAbsolutePath());
            } else if (file.getName().endsWith(".lnk")) {
                fi = this.loadLink(file.getAbsolutePath());
            } else {
                fi = this.loadUnsupported(file.getAbsolutePath());
            }

            size += fi.getSize();
            children.add(fi);
        }

        Directory d = new Directory(path, dir.getName(), size, children);
        for(FileInstance fi : children) {
            fi.setParent(d);
        }
        return d;
    }

    public FileInstance loadPlainText(String path) {
        try {
            BasicFileAttributes attr = Files.readAttributes(Path.of(path), BasicFileAttributes.class);
            return new PlainText(path, new File(path).getName(), ".txt", attr.size(),
                    Files.readString(Path.of(path)));
        } catch(Exception e) {
            File f = new File(path);
            return new ErrorFile(path, f.getName(), f.length(),
                    new DataFormatException("Invalid file extension type"));
        }
    }

    public FileInstance loadRichText(String path) {
        try {
            throw new DataFormatException("Rich text file parsing is not implemented yet");
        } catch(Exception e) {
            File f = new File(path);
            return new ErrorFile(path, f.getName(), f.length(),
                    new DataFormatException("Invalid file extension type"));
        }
    }

    public FileInstance loadPNGImage(String path) {
        try {
            BasicFileAttributes attr = Files.readAttributes(Path.of(path), BasicFileAttributes.class);
            BufferedImage img = ImageIO.read(new File(path));
            return new ImageFile(path, new File(path).getName(), FileType.PNG, attr.size(), img);
        } catch(Exception e) {
            File f = new File(path);
            return new ErrorFile(path, f.getName(), f.length(),
                    new DataFormatException("Invalid file extension type"));
        }
    }

    public FileInstance loadJPGImage(String path) {
        try {
            BasicFileAttributes attr = Files.readAttributes(Path.of(path), BasicFileAttributes.class);
            BufferedImage img = ImageIO.read(new File(path));
            return new ImageFile(path, new File(path).getName(), FileType.JPG, attr.size(), img);
        } catch(Exception e) {
            File f = new File(path);
            return new ErrorFile(path, f.getName(), f.length(),
                    new DataFormatException("Invalid file extension type"));
        }
    }

    public FileInstance loadLink(String path) {
        try {
            BasicFileAttributes attr = Files.readAttributes(Path.of(path), BasicFileAttributes.class);
            String link = Files.readString(Path.of(path));
            return new LinkFile(path, new File(path).getName(), ".lnk", attr.size(), link);
        } catch(Exception e) {
            File f = new File(path);
            return new ErrorFile(path, f.getName(), f.length(),
                    new DataFormatException("Invalid file extension type"));
        }
    }

    public FileInstance loadUnsupported(String path) {
        File f = new File(path);
        return new ErrorFile(path, f.getName(), f.length(),
                new DataFormatException("Invalid file extension type"));
    }

}
