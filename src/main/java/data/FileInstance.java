package data;
import java.awt.Desktop;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.text.DecimalFormat;
import java.util.ArrayList;
import java.util.Date;

public abstract class FileInstance {

	protected String name;
	protected final FileType type;
	protected final long size;
	protected final String path;
	protected Directory parent;

	public FileInstance(String path, String name, FileType type, Long size) {
		this.name = name;
		this.path = path;
		this.type = type != null ? type : FileType.UNK;
		this.size = size != null ? size : 0;
	}

	public String getName() { return this.name; }
	public FileType getType() { return this.type; }
	public long getSize() {	return this.size; }
	public String getPath() { return this.path; }
	public boolean isDirectory() { return this.type == FileType.DIR; }
	public boolean isEncryptable() { return false; }
	public Directory getParent() { return this.parent; }

	public String getFormattedSize() {
		if(size <= 0) return "0 B";
		final String[] units = new String[] { "B", "kB", "MB", "GB", "TB" };
		int digitGroups = (int) (Math.log10(size)/Math.log10(1024));
		return new DecimalFormat("#,##0.#").format(size/Math.pow(1024, digitGroups)) + " " + units[digitGroups];
	}

	public abstract ByteArrayOutputStream view() throws IOException;

	public void open() throws IOException {
		if(!Desktop.isDesktopSupported()) {  
			throw new UnsupportedOperationException("Opening files is not supported");
		}
		
		Desktop desktop = Desktop.getDesktop();  
		
		File file = new File(this.path);   
		if(file.exists()) {
			desktop.open(file);
		} else {
			throw new UnsupportedOperationException("Unable to open missing file");
		}
	}  
	
	public void openNew() throws IOException {
		if(!Desktop.isDesktopSupported()) {  
			throw new UnsupportedOperationException("Opening files is not supported");
		}
		
		Desktop desktop = Desktop.getDesktop();  
		
		String path = this.path.replace(this.name, "");
		String filename = null;
		if(this.name.contains(".")) {
			String[] parts = this.name.split("\\.");
			String ext = parts[parts.length-1];
			filename = this.name.replace("." + ext, "") + "-temp." + ext;
		} else {
			filename = this.name + "-temp";
		}
		
		Files.write(Paths.get(path + filename), view().toByteArray());
		
		File file = new File(path + filename);  
		if(file.exists()) {
			desktop.open(file);			
			// TODO may want to delete temp file
//			try { Thread.sleep(100); } catch (InterruptedException e) {}
//			Files.delete(Paths.get(file.getAbsolutePath()));
		} else {
			throw new UnsupportedOperationException("Unable to open missing file");
		}
	} 
	
	public void delete() throws IOException {
		Files.delete(Paths.get(this.path));
	}

	public void setName(String name) { this.name = name; }
	public void setParent(Directory parent) { this.parent = parent; }

	public String toString() { 
		return this.name + (this.type == FileType.DIR ? "/" : "") + 
				" (" + getFormattedSize() + ")"; 
	}




}
