use anyhow::{Context, Result};
use clap::Parser;

mod algorithms;
mod args;
mod demo;
mod drives;
mod platform;
mod progress;
mod ui;
mod wipe;

use args::Args;
use demo::create_demo_file;
use drives::list_drives;
use ui::confirm_wipe;
use wipe::WipeContext;

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle list drives command
    if args.list_drives {
        return list_drives(args.json);
    }

    // Validate arguments for wiping operations
    if !args.demo && args.target.is_none() {
        anyhow::bail!(
            "Target file must be specified when not in demo mode. Use --target <PATH> or --demo"
        );
    }

    let target_path = if args.demo {
        let demo_path =
            std::env::temp_dir().join(format!("secure_wipe_demo_{}.img", std::process::id()));
        create_demo_file(&demo_path, args.demo_size, args.json)?;
        demo_path
    } else {
        args.target.clone().unwrap() // Safe to unwrap because we validated above
    };

    // Check if target is a block device (platform-specific)
    let is_block_device = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            match std::fs::metadata(&target_path) {
                Ok(meta) => meta.file_type().is_block_device(),
                Err(_) => false,
            }
        }
        #[cfg(windows)]
        {
            platform::windows::is_windows_device_path(&target_path)
        }
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    };

    if !target_path.exists() && !args.demo && !is_block_device {
        anyhow::bail!(
            "Target file or device does not exist: {}",
            target_path.display()
        );
    }

    if !args.force && !confirm_wipe(&target_path, args.demo)? {
        println!("Operation cancelled by user");
        return Ok(());
    }

    let mut wipe_context = WipeContext::new(
        &target_path,
        args.algorithm,
        args.passes,
        args.buffer_size,
        args.json,
        is_block_device,
        args.fast,
    )?;

    wipe_context.wipe()?;

    if args.verify {
        println!("\nVerifying wipe...");
        // TODO: Implement verification
        println!("Verification not yet implemented");
    }

    if args.demo {
        std::fs::remove_file(&target_path).with_context(|| "Failed to clean up demo file")?;
        println!("Demo file cleaned up");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::WipeAlgorithm;
    use tempfile::NamedTempFile;

    #[test]
    fn test_demo_file_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = create_demo_file(temp_file.path(), 1, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wipe_context_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test data").unwrap();

        let result = WipeContext::new(
            temp_file.path(),
            WipeAlgorithm::Zero,
            1,
            1024,
            false,
            false,
            false,
        );
        assert!(result.is_ok());
    }
}
