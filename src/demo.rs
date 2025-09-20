use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

pub fn create_demo_file(path: &Path, size_mb: u64) -> Result<()> {
    println!("Creating demo file: {}", path.display());
    println!("Size: {} MB", size_mb);

    let file = File::create(path).with_context(|| "Failed to create demo file")?;
    let size_bytes = size_mb * 1024 * 1024;

    file.set_len(size_bytes)
        .with_context(|| "Failed to set file size")?;

    // Fill with some recognizable pattern for demo purposes
    let mut file = OpenOptions::new().write(true).open(path)?;
    let pattern = b"DEMO DATA - This will be securely wiped! ";
    let mut written = 0u64;

    let pb = ProgressBar::new(size_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("Creating [{bar:40.green/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    while written < size_bytes {
        let write_size = std::cmp::min(pattern.len() as u64, size_bytes - written) as usize;
        file.write_all(&pattern[..write_size])?;
        written += write_size as u64;
        pb.set_position(written);
    }

    pb.finish_with_message("Demo file created");
    println!("Demo file ready for secure wiping");
    Ok(())
}