package data;

import java.awt.Desktop;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.OutputStream;
import java.util.Date;
import java.util.HashMap;

public class PlainText extends FileInstance {

	private final String text;

	public PlainText(String path, String name, String extension, Long size, String text) {
		super(path, name, FileType.TXT, size);
		this.text = text;

	}
	
	@Override
	public ByteArrayOutputStream view() throws IOException {
		ByteArrayOutputStream os = new ByteArrayOutputStream();
		os.write(this.text.getBytes());
		return os; 
	}

}
