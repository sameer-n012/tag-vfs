package data;

import java.awt.image.BufferedImage;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.util.Date;

import javax.imageio.ImageIO;

public class ImageFile extends FileInstance {
	
	private final BufferedImage img;
	
	public ImageFile(String path, String name, FileType type, Long size, BufferedImage img) {
		super(path, name, type, size);
		this.img = img;
	}

	@Override
	public ByteArrayOutputStream view() throws IOException {
		ByteArrayOutputStream os = new ByteArrayOutputStream();
		if(this.type != FileType.JPG && this.type != FileType.PNG) {
			throw new IOException("Cannot output non-jpg/png image");
		}
		ImageIO.write(img, type == FileType.JPG ? "jpg" : "png", os); 
		return os;
	}

}
