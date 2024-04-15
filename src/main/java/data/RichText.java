package data;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.util.Date;
import java.util.HashMap;

public class RichText extends FileInstance {

	private final String text;

	public RichText(String path, String name, Date createDate, Date modifiedDate,
			String extension, Long size, String text) {
		super(path, name, FileType.RTF, size);
		this.text = text;

	}

	@Override
	public ByteArrayOutputStream view() throws IOException {
		ByteArrayOutputStream os = new ByteArrayOutputStream();
		os.write(text.getBytes());
		return os; 
	}

}
