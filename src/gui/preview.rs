use std::path::Path;
use std::process::Command;

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "ico"];
const VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv", "avi", "webm", "m4v", "wmv", "flv"];

pub enum Kind {
    Image,
    Video,
    Unsupported,
}

/**
 * Classifies a filename as an image, a video, or unsupported for preview
 * purposes, based on its extension.
 *
 * @param name the filename to classify.
 * @return the file's preview Kind.
 */
pub fn kind_for(name: &str) -> Kind {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if IMAGE_EXTS.contains(&ext.as_str()) {
        Kind::Image
    } else if VIDEO_EXTS.contains(&ext.as_str()) {
        Kind::Video
    } else {
        Kind::Unsupported
    }
}

/**
 * Extracts a single frame from video bytes using the system `ffmpeg`
 * binary. This runs a blocking subprocess, so it is only used for the one
 * file currently selected in the detail panel, not for every row in the
 * file list. Returns None if `ffmpeg` is not on PATH, or if it fails to
 * decode the file (unsupported codec, corrupt data, and so on).
 *
 * @param bytes the raw video file bytes.
 * @param ext the file's extension, so ffmpeg's demuxer can detect the format.
 * @return the extracted frame, encoded as PNG bytes.
 */
pub fn video_thumbnail(bytes: &[u8], ext: &str) -> Option<Vec<u8>> {
    let dir = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let input_path = dir.join(format!("tag-vfs-preview-{}.{}", stamp, ext));
    let output_path = dir.join(format!("tag-vfs-preview-{}.png", stamp));

    std::fs::write(&input_path, bytes).ok()?;

    let status = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-i"])
        .arg(&input_path)
        .args(["-vframes", "1", "-vf", "scale=320:-1", "-f", "image2"])
        .arg(&output_path)
        .status();

    let _ = std::fs::remove_file(&input_path);

    let thumbnail = match status {
        Ok(status) if status.success() => std::fs::read(&output_path).ok(),
        _ => None,
    };
    let _ = std::fs::remove_file(&output_path);
    thumbnail
}
