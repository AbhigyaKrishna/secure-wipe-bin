use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

mod algorithms;
mod args;
mod demo;
mod ui;
mod wipe;

use args::Args;
use demo::create_demo_file;
use ui::confirm_wipe;
use wipe::WipeContext;

fn main() -> Result<()> {
    let args = Args::parse();

    let target_path = if args.demo {
        let demo_path = PathBuf::from(format!("/tmp/secure_wipe_demo_{}.img", std::process::id()));

        create_demo_file(&demo_path, args.demo_size)?;
        demo_path
    } else {
        args.target.clone()
    };

    if !target_path.exists() && !args.demo {
        anyhow::bail!("Target file does not exist: {}", target_path.display());
    }

    if !args.force && !confirm_wipe(&target_path, args.demo)? {
        println!("Operation cancelled by user");
        return Ok(());
    }

    let mut wipe_context =
        WipeContext::new(&target_path, args.algorithm, args.passes, args.buffer_size)?;

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
    use tempfile::NamedTempFile;
    use crate::args::WipeAlgorithm;

    #[test]
    fn test_demo_file_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = create_demo_file(temp_file.path(), 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wipe_context_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test data").unwrap();

        let result = WipeContext::new(temp_file.path(), WipeAlgorithm::Zero, 1, 1024);
        assert!(result.is_ok());
    }
}
