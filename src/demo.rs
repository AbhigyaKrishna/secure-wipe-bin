use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use crate::progress::{emit_event, ProgressEvent};

pub fn create_demo_file(path: &Path, size_mb: u64, json_mode: bool) -> Result<()> {
    if json_mode {
        let _ = emit_event(&ProgressEvent::Info {
            message: format!(
                "Creating demo file: {} (Size: {} MB)",
                path.display(),
                size_mb
            ),
        });
    } else {
        println!("Creating demo file: {}", path.display());
        println!("Size: {} MB", size_mb);
    }

    let file = File::create(path).with_context(|| "Failed to create demo file")?;
    let size_bytes = size_mb * 1024 * 1024;

    file.set_len(size_bytes)
        .with_context(|| "Failed to set file size")?;

    // Fill with some recognizable pattern for demo purposes
    let mut file = OpenOptions::new().write(true).open(path)?;
    let pattern = b"DEMO DATA - This will be securely wiped! ";
    let mut written = 0u64;

    let pb = if !json_mode {
        let pb = ProgressBar::new(size_bytes);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("Creating [{bar:40.green/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏  "),
        );
        Some(pb)
    } else {
        None
    };

    let mut last_progress_time = Instant::now();

    while written < size_bytes {
        let write_size = std::cmp::min(pattern.len() as u64, size_bytes - written) as usize;
        file.write_all(&pattern[..write_size])?;
        written += write_size as u64;

        if let Some(ref pb) = pb {
            pb.set_position(written);
        }

        // Emit JSON progress events periodically
        if json_mode {
            let now = Instant::now();
            if now.duration_since(last_progress_time) >= Duration::from_millis(100) {
                let _ = emit_event(&ProgressEvent::DemoFileCreating {
                    bytes_written: written,
                    total_bytes: size_bytes,
                    percent: (written as f64 / size_bytes as f64) * 100.0,
                });
                last_progress_time = now;
            }
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message("Demo file created");
    }

    if json_mode {
        let _ = emit_event(&ProgressEvent::DemoFileCreated {
            path: path.display().to_string(),
            size_mb,
        });
    } else {
        println!("Demo file ready for secure wiping");
    }

    Ok(())
}
