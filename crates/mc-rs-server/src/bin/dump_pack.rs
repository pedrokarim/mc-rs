use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::CompressionMethod;

fn add_dir<W: Write + std::io::Seek>(
    writer: &mut zip::ZipWriter<W>,
    base: &Path,
    current: &Path,
    options: FileOptions,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap();
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            add_dir(writer, base, &path, options)?;
        } else {
            writer
                .start_file(&rel_str, options)
                .map_err(std::io::Error::other)?;
            let bytes = std::fs::read(&path)?;
            writer.write_all(&bytes)?;
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("resource_packs/mcrs_ui"));
    let dst = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("mcrs_ui_dump.zip"));
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let opts: FileOptions = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);
        add_dir(&mut w, &src, &src, opts).unwrap();
        w.finish().unwrap();
    }
    let data = buf.into_inner();
    std::fs::write(&dst, &data).unwrap();
    println!("Zipped {:?} ({} bytes) → {:?}", src, data.len(), dst);
}
