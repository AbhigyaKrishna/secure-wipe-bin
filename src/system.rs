use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
    pub hostname: String,
    pub username: String,
    pub total_memory_bytes: Option<u64>,
    pub available_memory_bytes: Option<u64>,
    pub cpu_info: CpuInfo,
    pub storage_devices: Vec<StorageDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub logical_cores: usize,
    pub physical_cores: Option<usize>,
    pub model_name: Option<String>,
    pub frequency_mhz: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDevice {
    pub name: String,
    pub device_path: String,
    pub size_bytes: Option<u64>,
    pub device_type: String,
    pub mount_point: Option<String>,
    pub file_system: Option<String>,
}

pub fn get_system_info() -> Result<SystemInfo> {
    let os_info = get_os_info();
    let cpu_info = get_cpu_info()?;
    let memory_info = get_memory_info();
    let storage_devices = get_storage_devices()?;

    Ok(SystemInfo {
        os_name: os_info.0,
        os_version: os_info.1,
        architecture: std::env::consts::ARCH.to_string(),
        hostname: get_hostname(),
        username: get_username(),
        total_memory_bytes: memory_info.0,
        available_memory_bytes: memory_info.1,
        cpu_info,
        storage_devices,
    })
}

fn get_os_info() -> (String, String) {
    #[cfg(unix)]
    {
        use std::process::Command;

        let os_name = if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else if cfg!(target_os = "macos") {
            "macOS".to_string()
        } else if cfg!(target_os = "freebsd") {
            "FreeBSD".to_string()
        } else {
            "Unix".to_string()
        };

        let version = if cfg!(target_os = "linux") {
            // Try to get version from /proc/version
            std::fs::read_to_string("/proc/version")
                .unwrap_or_else(|_| "Unknown".to_string())
                .lines()
                .next()
                .unwrap_or("Unknown")
                .to_string()
        } else if cfg!(target_os = "macos") {
            Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            Command::new("uname")
                .arg("-r")
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        };

        (os_name, version)
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        let version = Command::new("cmd")
            .args(&["/C", "ver"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        ("Windows".to_string(), version)
    }

    #[cfg(not(any(unix, windows)))]
    {
        ("Unknown".to_string(), "Unknown".to_string())
    }
}

fn get_hostname() -> String {
    #[cfg(unix)]
    {
        use std::ffi::CStr;

        unsafe {
            let mut buf = [0u8; 256];
            if libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) == 0 {
                CStr::from_ptr(buf.as_ptr() as *const libc::c_char)
                    .to_string_lossy()
                    .to_string()
            } else {
                "unknown".to_string()
            }
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        Command::new("hostname")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    #[cfg(not(any(unix, windows)))]
    {
        "unknown".to_string()
    }
}

fn get_username() -> String {
    #[cfg(unix)]
    {
        use std::ffi::CStr;

        unsafe {
            let uid = libc::getuid();
            let passwd = libc::getpwuid(uid);
            if !passwd.is_null() {
                CStr::from_ptr((*passwd).pw_name)
                    .to_string_lossy()
                    .to_string()
            } else {
                std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .unwrap_or_else(|_| "unknown".to_string())
            }
        }
    }

    #[cfg(windows)]
    {
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string())
    }

    #[cfg(not(any(unix, windows)))]
    {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }
}

fn get_cpu_info() -> Result<CpuInfo> {
    let logical_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    #[cfg(target_os = "linux")]
    {
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

        let mut model_name = None;
        let mut physical_cores = None;
        let mut frequency_mhz = None;

        let mut core_ids = std::collections::HashSet::new();

        for line in cpuinfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "model name" if model_name.is_none() => {
                        model_name = Some(value.to_string());
                    }
                    "core id" => {
                        if let Ok(core_id) = value.parse::<u32>() {
                            core_ids.insert(core_id);
                        }
                    }
                    "cpu MHz" if frequency_mhz.is_none() => {
                        if let Ok(freq) = value.parse::<f64>() {
                            frequency_mhz = Some(freq as u64);
                        }
                    }
                    _ => {}
                }
            }
        }

        if !core_ids.is_empty() {
            physical_cores = Some(core_ids.len());
        }

        Ok(CpuInfo {
            logical_cores,
            physical_cores,
            model_name,
            frequency_mhz,
        })
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let model_name = Command::new("sysctl")
            .args(&["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string());

        let physical_cores = Command::new("sysctl")
            .args(&["-n", "hw.physicalcpu"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().parse().ok());

        let frequency_mhz = Command::new("sysctl")
            .args(&["-n", "hw.cpufrequency_max"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|hz| hz / 1_000_000); // Convert Hz to MHz

        Ok(CpuInfo {
            logical_cores,
            physical_cores,
            model_name,
            frequency_mhz,
        })
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let model_name = Command::new("wmic")
            .args(&["cpu", "get", "name", "/value"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| {
                for line in s.lines() {
                    if line.starts_with("Name=") {
                        return Some(line.strip_prefix("Name=").unwrap_or("").trim().to_string());
                    }
                }
                None
            });

        let physical_cores = Command::new("wmic")
            .args(&["cpu", "get", "NumberOfCores", "/value"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| {
                for line in s.lines() {
                    if line.starts_with("NumberOfCores=") {
                        return line.strip_prefix("NumberOfCores=")
                            .unwrap_or("")
                            .trim()
                            .parse()
                            .ok();
                    }
                }
                None
            });

        let frequency_mhz = Command::new("wmic")
            .args(&["cpu", "get", "MaxClockSpeed", "/value"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| {
                for line in s.lines() {
                    if line.starts_with("MaxClockSpeed=") {
                        return line.strip_prefix("MaxClockSpeed=")
                            .unwrap_or("")
                            .trim()
                            .parse()
                            .ok();
                    }
                }
                None
            });

        Ok(CpuInfo {
            logical_cores,
            physical_cores,
            model_name,
            frequency_mhz,
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Ok(CpuInfo {
            logical_cores,
            physical_cores: None,
            model_name: None,
            frequency_mhz: None,
        })
    }
}

fn get_memory_info() -> (Option<u64>, Option<u64>) {
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();

        let mut total_kb = None;
        let mut available_kb = None;

        for line in meminfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value_parts: Vec<&str> = value.trim().split_whitespace().collect();

                if let Some(value_str) = value_parts.first() {
                    if let Ok(kb) = value_str.parse::<u64>() {
                        match key {
                            "MemTotal" => total_kb = Some(kb),
                            "MemAvailable" => available_kb = Some(kb),
                            _ => {}
                        }
                    }
                }
            }
        }

        (
            total_kb.map(|kb| kb * 1024),
            available_kb.map(|kb| kb * 1024),
        )
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let total_bytes = Command::new("sysctl")
            .args(&["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().parse().ok());

        // Getting available memory on macOS is more complex, skipping for now
        (total_bytes, None)
    }

    #[cfg(target_os = "windows")]
    {
        use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

        unsafe {
            let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
            mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

            if GlobalMemoryStatusEx(&mut mem_status) != 0 {
                let total_bytes = mem_status.ullTotalPhys;
                let available_bytes = mem_status.ullAvailPhys;
                (Some(total_bytes), Some(available_bytes))
            } else {
                (None, None)
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        (None, None)
    }
}

fn get_storage_devices() -> Result<Vec<StorageDevice>> {
    let mut devices = Vec::new();

    #[cfg(target_os = "linux")]
    {
        // Get block devices from /proc/partitions
        if let Ok(partitions) = std::fs::read_to_string("/proc/partitions") {
            for line in partitions.lines().skip(2) {
                // Skip header lines
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let device_name = parts[3];
                    let size_kb = parts[2].parse::<u64>().unwrap_or(0);

                    // Skip partitions of main devices (simple heuristic)
                    if !device_name.chars().last().map_or(false, |c| c.is_numeric()) {
                        continue;
                    }

                    let device_path = format!("/dev/{}", device_name);
                    let size_bytes = if size_kb > 0 {
                        Some(size_kb * 1024)
                    } else {
                        None
                    };

                    // Try to get mount point
                    let mount_point = get_mount_point(&device_path);
                    let file_system = get_file_system(&device_path);

                    devices.push(StorageDevice {
                        name: device_name.to_string(),
                        device_path,
                        size_bytes,
                        device_type: "block".to_string(),
                        mount_point,
                        file_system,
                    });
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Use the existing Windows drive enumeration
        if let Ok(physical_drives) = crate::platform::windows::list_physical_drives() {
            for drive_path in physical_drives {
                if let Ok(info) = crate::platform::windows::get_drive_info(&drive_path) {
                    devices.push(StorageDevice {
                        name: drive_path.clone(),
                        device_path: drive_path,
                        size_bytes: Some(info.size_bytes),
                        device_type: "physical".to_string(),
                        mount_point: None,
                        file_system: None,
                    });
                }
            }
        }

        if let Ok(logical_drives) = crate::platform::windows::list_logical_drives() {
            for drive_path in logical_drives {
                devices.push(StorageDevice {
                    name: drive_path.clone(),
                    device_path: drive_path,
                    size_bytes: None,
                    device_type: "logical".to_string(),
                    mount_point: None,
                    file_system: None,
                });
            }
        }
    }

    Ok(devices)
}

#[cfg(target_os = "linux")]
fn get_mount_point(device_path: &str) -> Option<String> {
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == device_path {
                return Some(parts[1].to_string());
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn get_mount_point(_device_path: &str) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn get_file_system(device_path: &str) -> Option<String> {
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == device_path {
                return Some(parts[2].to_string());
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn get_file_system(_device_path: &str) -> Option<String> {
    None
}

pub fn display_system_info(system_info: &SystemInfo, json: bool) -> Result<()> {
    if json {
        let json_str = serde_json::to_string_pretty(system_info)?;
        println!("{}", json_str);
    } else {
        println!("System Information:");
        println!("==================");
        println!("OS: {} {}", system_info.os_name, system_info.os_version);
        println!("Architecture: {}", system_info.architecture);
        println!("Hostname: {}", system_info.hostname);
        println!("Username: {}", system_info.username);

        if let Some(total) = system_info.total_memory_bytes {
            print!("Memory: {} GB", total / (1024 * 1024 * 1024));
            if let Some(available) = system_info.available_memory_bytes {
                println!(" ({} GB available)", available / (1024 * 1024 * 1024));
            } else {
                println!();
            }
        }

        println!("\nCPU Information:");
        println!("  Logical cores: {}", system_info.cpu_info.logical_cores);
        if let Some(physical) = system_info.cpu_info.physical_cores {
            println!("  Physical cores: {}", physical);
        }
        if let Some(ref model) = system_info.cpu_info.model_name {
            println!("  Model: {}", model);
        }
        if let Some(freq) = system_info.cpu_info.frequency_mhz {
            println!("  Frequency: {} MHz", freq);
        }

        if !system_info.storage_devices.is_empty() {
            println!("\nStorage Devices:");
            for device in &system_info.storage_devices {
                print!("  {} ({})", device.name, device.device_path);
                if let Some(size) = device.size_bytes {
                    let size_gb = size / (1024 * 1024 * 1024);
                    if size_gb > 0 {
                        print!(" - {} GB", size_gb);
                    }
                }
                if let Some(ref mount) = device.mount_point {
                    print!(" mounted at {}", mount);
                }
                if let Some(ref fs) = device.file_system {
                    print!(" ({})", fs);
                }
                println!();
            }
        }
    }

    Ok(())
}
