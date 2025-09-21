use anyhow::{Context, Result};
use crossterm::{
    style::{Color, ResetColor, SetForegroundColor},
    ExecutableCommand,
};
use indicatif::{ProgressBar, ProgressStyle};
use rand::{thread_rng, RngCore};
use std::{
    fs::{File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
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

/// Get optimal buffer size based on device type and available memory
fn get_optimal_buffer_size(is_block_device: bool, requested_size: usize) -> usize {
    // If user specified a size, use it
    if requested_size != 1024 {
        return requested_size;
    }

    // Try to determine available system memory
    let system_memory_kb = get_available_memory_kb().unwrap_or(8 * 1024 * 1024); // Default to 8GB

    // Calculate optimal buffer size - be more aggressive for better performance
    let optimal_kb = if is_block_device {
        // For block devices, use larger buffers (8-64MB) for maximum throughput
        let max_buffer = std::cmp::min(64 * 1024, system_memory_kb / 50); // Max 64MB or 2% of system memory
        std::cmp::max(8 * 1024, max_buffer) // Min 8MB
    } else {
        // For files, use moderate buffers (4-32MB)
        let max_buffer = std::cmp::min(32 * 1024, system_memory_kb / 100); // Max 32MB or 1% of system memory
        std::cmp::max(4 * 1024, max_buffer) // Min 4MB
    };

    optimal_kb
}

/// Get available system memory in KB
fn get_available_memory_kb() -> Option<usize> {
    #[cfg(unix)]
    {
        // Try to read /proc/meminfo on Linux
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<usize>() {
                            return Some(kb);
                        }
                    }
                }
            }
        }

        // Fallback: use sysconf on Unix systems
        unsafe {
            let pages = libc::sysconf(libc::_SC_AVPHYS_PAGES);
            let page_size = libc::sysconf(libc::_SC_PAGE_SIZE);
            if pages > 0 && page_size > 0 {
                return Some((pages * page_size / 1024) as usize);
            }
        }
    }

    #[cfg(windows)]
    {
        use winapi::um::sysinfoapi::{
            GetPhysicallyInstalledSystemMemory, GlobalMemoryStatusEx, MEMORYSTATUSEX,
        };

        unsafe {
            let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
            mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

            if GlobalMemoryStatusEx(&mut mem_status) != 0 {
                return Some((mem_status.ullAvailPhys / 1024) as usize);
            }
        }
    }

    None
}

#[cfg(windows)]
use winapi::{
    shared::minwindef::{DWORD, LPVOID},
    um::{
        ioapiset::DeviceIoControl,
        winioctl::{DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX},
    },
};

pub struct WipeContext {
    file: File,
    size: u64,
    buffer_size: usize,
    algorithm: WipeAlgorithm,
    passes: usize,
    json_mode: bool,
    fast_mode: bool,
    is_block_device: bool,
    // Pre-allocated reusable buffer to avoid repeated allocations
    write_buffer: Vec<u8>,
}

impl WipeContext {
    pub fn new(
        path: &Path,
        algorithm: WipeAlgorithm,
        passes: usize,
        buffer_size: usize,
        json_mode: bool,
        is_block_device: bool,
        fast_mode: bool,
    ) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true).read(true);

        #[cfg(unix)]
        {
            // Only use O_SYNC for block devices in non-fast mode for data integrity
            // Remove O_SYNC for files to improve performance - we'll sync at the end of each pass
            if is_block_device && !fast_mode {
                options.custom_flags(libc::O_SYNC);
            }
            // Consider O_DIRECT for block devices if buffer alignment is handled properly
            // This would bypass the kernel page cache for better performance with large sequential writes
        }

        let file = options
            .open(path)
            .with_context(|| format!("Failed to open file or device: {}", path.display()))?;

        // Get optimal buffer size
        let optimal_buffer_size = get_optimal_buffer_size(is_block_device, buffer_size);

        // For block devices, get size using platform-specific methods
        let size = if is_block_device {
            #[cfg(unix)]
            {
                use std::os::unix::io::AsRawFd;
                let fd = file.as_raw_fd();
                let mut size: u64 = 0;
                unsafe {
                    // BLKGETSIZE64 ioctl
                    if libc::ioctl(fd, 0x80081272, &mut size) == 0 {
                        size
                    } else {
                        return Err(anyhow::anyhow!("Failed to get block device size"));
                    }
                }
            }
            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                let handle = file.as_raw_handle();
                let mut geometry: DISK_GEOMETRY_EX = unsafe { std::mem::zeroed() };
                let mut bytes_returned: DWORD = 0;

                unsafe {
                    use winapi::ctypes::c_void;
                    if DeviceIoControl(
                        handle as *mut c_void,
                        IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                        std::ptr::null_mut(),
                        0,
                        &mut geometry as *mut _ as LPVOID,
                        std::mem::size_of::<DISK_GEOMETRY_EX>() as DWORD,
                        &mut bytes_returned,
                        std::ptr::null_mut(),
                    ) != 0
                    {
                        // Convert LARGE_INTEGER to u64 properly
                        let size = *geometry.DiskSize.QuadPart();
                        size as u64
                    } else {
                        return Err(anyhow::anyhow!("Failed to get Windows disk size"));
                    }
                }
            }
            #[cfg(not(any(unix, windows)))]
            {
                return Err(anyhow::anyhow!(
                    "Block device wiping is not supported on this platform"
                ));
            }
        } else {
            let metadata = file
                .metadata()
                .with_context(|| "Failed to get file metadata")?;
            metadata.len()
        };

        // Pre-allocate buffer once to avoid repeated allocations during wiping
        let write_buffer = vec![0u8; optimal_buffer_size * 1024];

        Ok(WipeContext {
            file,
            size,
            buffer_size: optimal_buffer_size,
            algorithm,
            passes,
            json_mode,
            fast_mode,
            is_block_device,
            write_buffer,
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

        // Pre-fill buffer with pattern to avoid repeated pattern generation
        // This significantly improves performance for fixed patterns
        match &pattern {
            WipePattern::Fixed(byte) => {
                self.write_buffer.fill(*byte);
            }
            WipePattern::Gutmann(patterns) => {
                let pattern_idx = (pass - 1) % patterns.len();
                if patterns[pattern_idx].len() == 1 {
                    self.write_buffer.fill(patterns[pattern_idx][0]);
                } else {
                    for (i, byte) in self.write_buffer.iter_mut().enumerate() {
                        *byte = patterns[pattern_idx][i % patterns[pattern_idx].len()];
                    }
                }
            }
            WipePattern::Random => {
                // For random patterns, we'll generate fresh random data each iteration
                // to avoid predictable patterns
            }
        }

        let mut total_written = 0u64;
        let mut last_progress_time = Instant::now();
        let mut last_bytes = 0u64;

        // Optimize progress reporting frequency based on mode
        let progress_interval = if self.fast_mode {
            Duration::from_secs(2) // Much less frequent in fast mode
        } else if self.json_mode {
            Duration::from_millis(500)
        } else {
            Duration::from_millis(200)
        };

        // Main write loop - optimized for performance
        while total_written < self.size {
            let write_size = std::cmp::min(
                self.write_buffer.len(),
                (self.size - total_written) as usize,
            );

            // Generate fresh random data only when needed
            if matches!(pattern, WipePattern::Random) {
                thread_rng().fill_bytes(&mut self.write_buffer[..write_size]);
            }

            // Direct write to file without BufWriter to avoid double buffering overhead
            self.file
                .write_all(&self.write_buffer[..write_size])
                .with_context(|| "Failed to write data")?;

            total_written += write_size as u64;

            // Update progress less frequently to reduce overhead
            let now = Instant::now();
            let should_update_progress =
                now.duration_since(last_progress_time) >= progress_interval;

            if should_update_progress {
                // Update progress bar
                if let Some(ref pb) = pb {
                    pb.set_position(total_written);
                }

                // Emit JSON progress events
                if self.json_mode {
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
                }

                last_progress_time = now;
                last_bytes = total_written;
            }
        }

        // Sync only at the end of each pass, not during writes
        // This provides a good balance between performance and data integrity
        if !self.fast_mode {
            #[cfg(unix)]
            unsafe {
                libc::fsync(self.file.as_raw_fd());
            }

            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                use winapi::um::{fileapi::FlushFileBuffers, handleapi::INVALID_HANDLE_VALUE};

                unsafe {
                    use winapi::ctypes::c_void;
                    let handle = self.file.as_raw_handle() as *mut c_void;
                    if handle != INVALID_HANDLE_VALUE as *mut c_void {
                        FlushFileBuffers(handle);
                    }
                }
            }
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
