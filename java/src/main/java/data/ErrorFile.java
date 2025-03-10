package data;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.OutputStream;

public class ErrorFile extends FileInstance {
	
	Exception e;

	public ErrorFile(String path, String name, Long size, Exception e) {
		super(path, name, FileType.UNK, size != null ? size : 0L);
		this.e = e;
	}

	@Override
	public ByteArrayOutputStream view() throws IOException {
		StringBuilder sb = new StringBuilder();
		sb.append("There was an error loading this file\n");
		sb.append(e.toString());
		ByteArrayOutputStream os = new ByteArrayOutputStream();
		os.write(sb.toString().getBytes());
		return os;
	}

}
