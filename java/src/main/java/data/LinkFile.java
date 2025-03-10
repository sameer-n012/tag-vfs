package data;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.util.Date;

public class LinkFile extends FileInstance {

	private final String link;
	private String cache;

	public LinkFile(String path, String name, String extension, Long size, String link) {
		super(path, name, FileType.LNK, size);
		this.link = link;
		this.cache = null;
	}

	@Override
	public ByteArrayOutputStream view() throws IOException {
		ByteArrayOutputStream os = new ByteArrayOutputStream();
		os.write(link.getBytes());
		return os;
	}

}
