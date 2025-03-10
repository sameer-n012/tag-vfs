package data;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.util.ArrayList;

public class Directory extends FileInstance {
	
	private final ArrayList<FileInstance> children;
	
	public Directory(String path, String name, Long size) {
		super(path, name, FileType.DIR, size);
		this.children = new ArrayList<>();
	}
	
	public Directory(String path, String name, Long size, ArrayList<FileInstance> children) {
		super(path, name, FileType.DIR, size);
		this.children = children;
	}
	
	public boolean addChild(FileInstance child) {
		for (FileInstance fi : children) {
			if(fi.name.equals(child.name)) {
				return false;
			}
		}
		
		children.add(child);
		return true;
	}
	
	public boolean removeChild(FileInstance child) {
		return children.remove(child);
	}
	
	/**
	 * Returns a copy of the children array list
	 * @return an arraylist of the children
	 */
	public ArrayList<FileInstance> getChildren() {
		return new ArrayList<FileInstance>(children);
	}

	@Override
	public ByteArrayOutputStream view() throws IOException {
		StringBuilder sb = new StringBuilder();
		sb.append(this.name + "/\n");
		for(FileInstance fi : children) {
			sb.append(" - " + fi.name);
			sb.append(fi.isDirectory() ? "/\n" : "\n");
		}
		
		ByteArrayOutputStream os = new ByteArrayOutputStream();
		os.write(sb.toString().getBytes());
		return os;
	}
	
}
