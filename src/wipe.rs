use anyhow::{Context, Result};
use crossterm::{
    style::{Color, ResetColor, SetForegroundColor},
    ExecutableCommand,
};
use indicatif::{ProgressBar, ProgressStyle};
use rand::{thread_rng, RngCore};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufWriter, Seek, SeekFrom, Write},
    path::Path,
    time::{Duration, Instant},
};

use crate::{
    algorithms::{get_algorithm_pass_count, get_pass_pattern, get_pattern_name, WipePattern},
    args::WipeAlgorithm,
    progress::{emit_event, ProgressEvent},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

pub struct WipeContext {
    file: File,
    size: u64,
    buffer_size: usize,
    algorithm: WipeAlgorithm,
    passes: usize,
    json_mode: bool,
}

impl WipeContext {
    pub fn new(
        path: &Path,
        algorithm: WipeAlgorithm,
        passes: usize,
        buffer_size: usize,
        json_mode: bool,
    ) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true).read(true);

        #[cfg(unix)]
        options.custom_flags(libc::O_SYNC); // Force synchronous writes

        let file = options
            .open(path)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        let metadata = file
            .metadata()
            .with_context(|| "Failed to get file metadata")?;

        let size = metadata.len();

        Ok(WipeContext {
            file,
            size,
            buffer_size,
            algorithm,
            passes,
            json_mode,
        })
    }

    pub fn wipe(&mut self) -> Result<()> {
        let total_passes = get_algorithm_pass_count(&self.algorithm, self.passes);

        if self.json_mode {
            let _ = emit_event(&ProgressEvent::Start {
                algorithm: format!("{:?}", self.algorithm),
                total_passes,
                file_size_bytes: self.size,
                buffer_size_kb: self.buffer_size,
            });
        } else {
            println!(
                "Starting secure wipe using {:?} algorithm ({} passes)",
                self.algorithm, total_passes
            );
            println!("File size: {:.2} MB", self.size as f64 / 1_048_576.0);
            println!("Buffer size: {} KB", self.buffer_size);
            println!();
        }

        let start_time = Instant::now();

        for pass in 1..=total_passes {
            self.wipe_pass(pass, total_passes)?;
        }

        let elapsed = start_time.elapsed();
        let throughput =
            (self.size as f64 * total_passes as f64) / elapsed.as_secs_f64() / 1_048_576.0;

        if self.json_mode {
            let _ = emit_event(&ProgressEvent::Complete {
                total_time_seconds: elapsed.as_secs_f64(),
                average_throughput_mb_s: throughput,
            });
        } else {
            println!();
            io::stdout().execute(SetForegroundColor(Color::Green))?;
            println!("Secure wipe completed successfully!");
            io::stdout().execute(ResetColor)?;
            println!("Total time: {:.2} seconds", elapsed.as_secs_f64());
            println!("Average throughput: {:.2} MB/s", throughput);
        }

        Ok(())
    }

    fn wipe_pass(&mut self, pass: usize, total_passes: usize) -> Result<()> {
        self.file
            .seek(SeekFrom::Start(0))
            .with_context(|| "Failed to seek to beginning of file")?;

        let pattern = get_pass_pattern(&self.algorithm, pass);
        let pattern_name = get_pattern_name(&self.algorithm, pass);

        if self.json_mode {
            let _ = emit_event(&ProgressEvent::PassStart {
                pass,
                total_passes,
                pattern: pattern_name.to_string(),
            });
        }

        let pb = if !self.json_mode {
            let pb = ProgressBar::new(self.size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&format!(
                        "Pass {}/{} [{}] {{bar:40.cyan/blue}} {{bytes}}/{{total_bytes}} ({{bytes_per_sec}}) {{msg}}",
                        pass, total_passes, pattern_name
                    ))?
                    .progress_chars("█▉▊▋▌▍▎▏  "),
            );
            Some(pb)
        } else {
            None
        };

        let mut buffer = vec![0u8; self.buffer_size * 1024];
        let mut total_written = 0u64;
        let mut writer = BufWriter::new(&mut self.file);
        let mut last_progress_time = Instant::now();
        let mut last_bytes = 0u64;

        while total_written < self.size {
            let write_size = std::cmp::min(buffer.len(), (self.size - total_written) as usize);

            match &pattern {
                WipePattern::Fixed(byte) => buffer[..write_size].fill(*byte),
                WipePattern::Random => thread_rng().fill_bytes(&mut buffer[..write_size]),
                WipePattern::Gutmann(patterns) => {
                    let pattern_idx = (pass - 1) % patterns.len();
                    if patterns[pattern_idx].len() == 1 {
                        buffer[..write_size].fill(patterns[pattern_idx][0]);
                    } else {
                        for (i, byte) in buffer[..write_size].iter_mut().enumerate() {
                            *byte = patterns[pattern_idx][i % patterns[pattern_idx].len()];
                        }
                    }
                }
            }

            writer
                .write_all(&buffer[..write_size])
                .with_context(|| "Failed to write data")?;

            total_written += write_size as u64;

            // Update progress
            if let Some(ref pb) = pb {
                pb.set_position(total_written);
            }

            // Emit JSON progress events periodically
            if self.json_mode {
                let now = Instant::now();
                if now.duration_since(last_progress_time) >= Duration::from_millis(100) {
                    let elapsed = now.duration_since(last_progress_time);
                    let bytes_diff = total_written - last_bytes;
                    let bytes_per_second = if elapsed.as_secs_f64() > 0.0 {
                        bytes_diff as f64 / elapsed.as_secs_f64()
                    } else {
                        0.0
                    };

                    let _ = emit_event(&ProgressEvent::Progress {
                        pass,
                        total_passes,
                        bytes_written: total_written,
                        total_bytes: self.size,
                        percent: (total_written as f64 / self.size as f64) * 100.0,
                        bytes_per_second,
                    });

                    last_progress_time = now;
                    last_bytes = total_written;
                }
            }

            // Add small delay for demo visualization (only in non-JSON mode)
            if !self.json_mode {
                std::thread::sleep(Duration::from_millis(1));
            }
        }

        writer.flush().with_context(|| "Failed to flush buffer")?;

        #[cfg(unix)]
        unsafe {
            libc::fsync(writer.get_ref().as_raw_fd());
        }

        if let Some(pb) = pb {
            pb.finish_with_message("Completed");
        }

        if self.json_mode {
            let _ = emit_event(&ProgressEvent::PassComplete { pass, total_passes });
        }

        Ok(())
    }
}
