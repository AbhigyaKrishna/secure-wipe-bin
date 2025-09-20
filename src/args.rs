use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum WipeAlgorithm {
    /// Simple zero overwrite (1 pass)
    Zero,
    /// Random data overwrite (1 pass)
    Random,
    /// DoD 5220.22-M standard (3 passes: 0x00, 0xFF, random)
    Dod5220,
    /// Gutmann method (35 passes)
    Gutmann,
    /// Custom number of random passes
    Custom,
}

#[derive(Debug, Parser)]
#[command(name = "secure-wipe")]
#[command(about = "Secure file/device wiping utility with real-time visualization")]
pub struct Args {
    /// Target file or block device/partition to wipe (e.g. /dev/sda1, /dev/nvme0n1p1, or a file path). Optional in demo mode.
    #[arg(short, long)]
    pub target: Option<PathBuf>,

    /// Wiping algorithm to use
    #[arg(short, long, value_enum, default_value_t = WipeAlgorithm::Random)]
    pub algorithm: WipeAlgorithm,

    /// Number of passes (for custom algorithm)
    #[arg(short, long, default_value_t = 3)]
    pub passes: usize,

    /// Demo mode - creates and wipes test file safely
    #[arg(short, long)]
    pub demo: bool,

    /// Size of demo file in MB
    #[arg(long, default_value_t = 100)]
    pub demo_size: u64,

    /// Buffer size in KB for wiping operations
    #[arg(long, default_value_t = 1024)]
    pub buffer_size: usize,

    /// Force wipe without confirmation (dangerous!)
    #[arg(short, long)]
    pub force: bool,

    /// Verify wipe by reading back data
    #[arg(short, long)]
    pub verify: bool,

    /// Output machine-readable JSON for subprocess integration
    #[arg(long)]
    pub json: bool,
}
