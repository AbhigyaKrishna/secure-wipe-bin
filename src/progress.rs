use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProgressEvent {
    #[serde(rename = "start")]
    Start {
        algorithm: String,
        total_passes: usize,
        file_size_bytes: u64,
        buffer_size_kb: usize,
    },
    #[serde(rename = "pass_start")]
    PassStart {
        pass: usize,
        total_passes: usize,
        pattern: String,
    },
    #[serde(rename = "progress")]
    Progress {
        pass: usize,
        total_passes: usize,
        bytes_written: u64,
        total_bytes: u64,
        percent: f64,
        bytes_per_second: f64,
    },
    #[serde(rename = "pass_complete")]
    PassComplete { pass: usize, total_passes: usize },
    #[serde(rename = "complete")]
    Complete {
        total_time_seconds: f64,
        average_throughput_mb_s: f64,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "demo_file_created")]
    DemoFileCreated { path: String, size_mb: u64 },
    #[serde(rename = "demo_file_creating")]
    DemoFileCreating {
        bytes_written: u64,
        total_bytes: u64,
        percent: f64,
    },
    #[serde(rename = "info")]
    Info { message: String },
}

pub fn emit_event(event: &ProgressEvent) -> io::Result<()> {
    let json = serde_json::to_string(event)?;
    println!("{}", json);
    io::stdout().flush()?;
    Ok(())
}
