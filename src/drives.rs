use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub path: String,
    pub drive_type: String,
    pub size_bytes: Option<u64>,
    pub size_gb: Option<f64>,
    pub description: String,
}

pub fn list_drives(json_mode: bool) -> Result<()> {
    let mut drives = Vec::new();

    // Get platform-specific drives
    #[cfg(unix)]
    {
        drives.extend(list_unix_drives()?);
    }

    #[cfg(windows)]
    {
        drives.extend(list_windows_drives()?);
    }

    if json_mode {
        // Output JSON format
        let json_output = serde_json::json!({
            "type": "drive_list",
            "drives": drives
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        // Output human-readable format
        print_drives_human_readable(&drives);
    }

    Ok(())
}

#[cfg(unix)]
fn list_unix_drives() -> Result<Vec<DriveInfo>> {
    let mut drives = Vec::new();

    // Try to use lsblk for detailed information
    match get_lsblk_drives() {
        Ok(mut lsblk_drives) => drives.append(&mut lsblk_drives),
        Err(_) => {
            // Fallback to common device paths
            drives.extend(get_common_unix_devices());
        }
    }

    Ok(drives)
}

#[cfg(unix)]
fn get_lsblk_drives() -> Result<Vec<DriveInfo>> {
    use std::process::Command;

    let output = Command::new("lsblk")
        .args(["-J", "-o", "NAME,TYPE,SIZE,MOUNTPOINT"])
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("lsblk command failed"));
    }

    let json_str = String::from_utf8(output.stdout)?;
    let lsblk_output: serde_json::Value = serde_json::from_str(&json_str)?;

    let mut drives = Vec::new();

    if let Some(blockdevices) = lsblk_output["blockdevices"].as_array() {
        for device in blockdevices {
            if let (Some(name), Some(device_type), size) = (
                device["name"].as_str(),
                device["type"].as_str(),
                device["size"].as_str(),
            ) {
                let path = format!("/dev/{}", name);
                let size_info = size.unwrap_or("Unknown").to_string();

                drives.push(DriveInfo {
                    path: path.clone(),
                    drive_type: device_type.to_string(),
                    size_bytes: None, // lsblk doesn't give exact bytes easily
                    size_gb: parse_size_to_gb(size.unwrap_or("")),
                    description: format!("{} - {} {}", path, device_type, size_info),
                });

                // Add partitions
                if let Some(children) = device["children"].as_array() {
                    for child in children {
                        if let (Some(child_name), Some(child_type), child_size) = (
                            child["name"].as_str(),
                            child["type"].as_str(),
                            child["size"].as_str(),
                        ) {
                            let child_path = format!("/dev/{}", child_name);
                            let child_size_info = child_size.unwrap_or("Unknown").to_string();

                            drives.push(DriveInfo {
                                path: child_path.clone(),
                                drive_type: child_type.to_string(),
                                size_bytes: None,
                                size_gb: parse_size_to_gb(child_size.unwrap_or("")),
                                description: format!(
                                    "{} - {} {}",
                                    child_path, child_type, child_size_info
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(drives)
}

#[cfg(unix)]
fn get_common_unix_devices() -> Vec<DriveInfo> {
    vec![
        DriveInfo {
            path: "/dev/sda".to_string(),
            drive_type: "disk".to_string(),
            size_bytes: None,
            size_gb: None,
            description: "/dev/sda - SATA disk (example)".to_string(),
        },
        DriveInfo {
            path: "/dev/sda1".to_string(),
            drive_type: "part".to_string(),
            size_bytes: None,
            size_gb: None,
            description: "/dev/sda1 - SATA partition (example)".to_string(),
        },
        DriveInfo {
            path: "/dev/nvme0n1".to_string(),
            drive_type: "disk".to_string(),
            size_bytes: None,
            size_gb: None,
            description: "/dev/nvme0n1 - NVMe disk (example)".to_string(),
        },
        DriveInfo {
            path: "/dev/nvme0n1p1".to_string(),
            drive_type: "part".to_string(),
            size_bytes: None,
            size_gb: None,
            description: "/dev/nvme0n1p1 - NVMe partition (example)".to_string(),
        },
    ]
}

#[cfg(windows)]
fn list_windows_drives() -> Result<Vec<DriveInfo>> {
    let mut drives = Vec::new();

    // Add physical drives
    drives.extend(get_windows_physical_drives()?);

    // Add logical drives
    drives.extend(get_windows_logical_drives()?);

    Ok(drives)
}

#[cfg(windows)]
fn get_windows_physical_drives() -> Result<Vec<DriveInfo>> {
    use winapi::{
        shared::minwindef::{DWORD, LPVOID},
        um::{
            fileapi::{CreateFileW, OPEN_EXISTING},
            handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
            ioapiset::DeviceIoControl,
            winioctl::{DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX},
            winnt::{FILE_ATTRIBUTE_NORMAL, GENERIC_READ},
        },
    };

    let mut drives = Vec::new();

    for i in 0..10 {
        // Check first 10 physical drives
        let drive_path = format!(r"\\.\PhysicalDrive{}", i);
        let wide_path: Vec<u16> = drive_path.encode_utf16().chain(Some(0)).collect();

        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                0,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );

            if handle != INVALID_HANDLE_VALUE {
                let mut geometry: DISK_GEOMETRY_EX = std::mem::zeroed();
                let mut bytes_returned: DWORD = 0;

                let success = DeviceIoControl(
                    handle,
                    IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                    std::ptr::null_mut(),
                    0,
                    &mut geometry as *mut _ as LPVOID,
                    std::mem::size_of::<DISK_GEOMETRY_EX>() as DWORD,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                );

                CloseHandle(handle);

                if success != 0 {
                    let size_bytes = *geometry.DiskSize.QuadPart() as u64;
                    let size_gb = size_bytes as f64 / 1_073_741_824.0;

                    drives.push(DriveInfo {
                        path: drive_path.clone(),
                        drive_type: "disk".to_string(),
                        size_bytes: Some(size_bytes),
                        size_gb: Some(size_gb),
                        description: format!("{} - Physical Drive ({:.2} GB)", drive_path, size_gb),
                    });
                } else {
                    drives.push(DriveInfo {
                        path: drive_path.clone(),
                        drive_type: "disk".to_string(),
                        size_bytes: None,
                        size_gb: None,
                        description: format!("{} - Physical Drive (size unknown)", drive_path),
                    });
                }
            }
        }
    }

    Ok(drives)
}

#[cfg(windows)]
fn get_windows_logical_drives() -> Result<Vec<DriveInfo>> {
    use winapi::um::fileapi::GetLogicalDrives;

    let mut drives = Vec::new();

    unsafe {
        let drive_mask = GetLogicalDrives();
        if drive_mask != 0 {
            for i in 0..26 {
                if (drive_mask >> i) & 1 == 1 {
                    let drive_letter = (b'A' + i) as char;
                    let drive_path = format!(r"\\.\{}:", drive_letter);

                    drives.push(DriveInfo {
                        path: drive_path.clone(),
                        drive_type: "volume".to_string(),
                        size_bytes: None,
                        size_gb: None,
                        description: format!("{} - Logical Volume", drive_path),
                    });
                }
            }
        }
    }

    Ok(drives)
}

#[cfg(not(any(unix, windows)))]
fn list_unix_drives() -> Result<Vec<DriveInfo>> {
    Ok(vec![])
}

#[cfg(not(any(unix, windows)))]
fn list_windows_drives() -> Result<Vec<DriveInfo>> {
    Ok(vec![])
}

fn parse_size_to_gb(size_str: &str) -> Option<f64> {
    if size_str.is_empty() {
        return None;
    }

    let size_str = size_str.to_uppercase();
    let (number_part, unit) = if size_str.ends_with('G') || size_str.ends_with("GB") {
        (size_str.trim_end_matches("GB").trim_end_matches('G'), 1.0)
    } else if size_str.ends_with('M') || size_str.ends_with("MB") {
        (size_str.trim_end_matches("MB").trim_end_matches('M'), 0.001)
    } else if size_str.ends_with('K') || size_str.ends_with("KB") {
        (
            size_str.trim_end_matches("KB").trim_end_matches('K'),
            0.000001,
        )
    } else {
        (size_str.as_str(), 0.000000001) // Assume bytes
    };

    if let Ok(number) = number_part.parse::<f64>() {
        Some(number * unit)
    } else {
        None
    }
}

fn print_drives_human_readable(drives: &[DriveInfo]) {
    if drives.is_empty() {
        println!("No drives found or platform not supported for drive enumeration.");
        return;
    }

    println!("Available drives and partitions for secure wiping:");
    println!();

    // Group by drive type
    let mut physical_drives = Vec::new();
    let mut partitions = Vec::new();
    let mut volumes = Vec::new();
    let mut other = Vec::new();

    for drive in drives {
        match drive.drive_type.as_str() {
            "disk" => physical_drives.push(drive),
            "part" => partitions.push(drive),
            "volume" => volumes.push(drive),
            _ => other.push(drive),
        }
    }

    if !physical_drives.is_empty() {
        println!("Physical Drives:");
        for drive in physical_drives {
            println!("  {}", drive.description);
        }
        println!();
    }

    if !partitions.is_empty() {
        println!("Partitions:");
        for drive in partitions {
            println!("  {}", drive.description);
        }
        println!();
    }

    if !volumes.is_empty() {
        println!("Volumes:");
        for drive in volumes {
            println!("  {}", drive.description);
        }
        println!();
    }

    if !other.is_empty() {
        println!("Other Devices:");
        for drive in other {
            println!("  {}", drive.description);
        }
        println!();
    }

    println!("Usage examples:");
    println!();

    #[cfg(unix)]
    {
        println!("Unix/Linux:");
        println!("  sudo ./secure-wipe-bin --target /dev/sda1 --algorithm dod5220 --force");
        println!("  sudo ./secure-wipe-bin --target /dev/nvme0n1p1 --algorithm gutmann");
        println!();
    }

    #[cfg(windows)]
    {
        println!("Windows (run as Administrator):");
        println!(
            "  .\\secure-wipe-bin.exe --target \\\\.\\PhysicalDrive0 --algorithm dod5220 --force"
        );
        println!("  .\\secure-wipe-bin.exe --target \\\\.\\C: --algorithm random --force");
        println!();
    }

    println!("⚠️  WARNING: Partition wiping is IRREVERSIBLE!");
    println!("   Always verify the target device before proceeding.");
    println!("   Use demo mode for safe testing: --demo --demo-size 10");
}
