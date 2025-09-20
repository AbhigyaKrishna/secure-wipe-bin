use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
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

    let size_bytes = size_mb * 1024 * 1024;

    // Create file with proper options for Windows
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("Failed to create demo file: {}", path.display()))?;

    // On Windows, pre-allocating large files can be problematic
    // Instead, we'll write in chunks and let the filesystem handle allocation
    let mut writer = BufWriter::new(file);
    let pattern = b"DEMO DATA - This will be securely wiped! ";
    let mut written = 0u64;

    let pb = if !json_mode {
        let pb = ProgressBar::new(size_bytes);
        // Use a more Windows-compatible progress bar template
        let template = if cfg!(windows) {
            "[{bar:40}] {bytes}/{total_bytes} ({bytes_per_sec})"
        } else {
            "Creating [{bar:40.green/blue}] {bytes}/{total_bytes} ({bytes_per_sec})"
        };

        pb.set_style(
            ProgressStyle::default_bar()
                .template(template)
                .with_context(|| "Failed to create progress bar style")?
                .progress_chars("█▉▊▋▌▍▎▏  "),
        );
        Some(pb)
    } else {
        None
    };

    let mut last_progress_time = Instant::now();
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer for better performance

    // Write data in chunks
    while written < size_bytes {
        let remaining = size_bytes - written;
        let chunk_size = std::cmp::min(buffer.len() as u64, remaining) as usize;

        // Fill buffer with pattern
        for i in 0..chunk_size {
            buffer[i] = pattern[i % pattern.len()];
        }

        writer
            .write_all(&buffer[..chunk_size])
            .with_context(|| format!("Failed to write demo data at offset {}", written))?;

        written += chunk_size as u64;

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

        // Small delay to prevent overwhelming the system
        if !json_mode {
            std::thread::sleep(Duration::from_micros(100));
        }
    }

    // Ensure all data is written to disk
    writer
        .flush()
        .with_context(|| "Failed to flush demo file")?;

    // On Unix systems, also sync to ensure data is on disk
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        unsafe {
            libc::fsync(writer.get_ref().as_raw_fd());
        }
    }

    // On Windows, use FlushFileBuffers
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use winapi::um::fileapi::FlushFileBuffers;

        unsafe {
            use winapi::ctypes::c_void;
            FlushFileBuffers(writer.get_ref().as_raw_handle() as *mut c_void);
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
