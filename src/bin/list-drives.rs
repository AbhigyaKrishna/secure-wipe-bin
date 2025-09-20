use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "list-drives")]
#[command(about = "Utility for listing drives and devices for secure-wipe")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available drives and partitions (default)
    List,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Commands::List) | None => list_drives()?,
    }

    Ok(())
}

#[cfg(windows)]
fn list_windows_drives() -> Result<()> {
    use winapi::{
        shared::minwindef::{DWORD, LPVOID},
        um::{
            fileapi::{CreateFileW, GetLogicalDrives, OPEN_EXISTING},
            handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
            ioapiset::DeviceIoControl,
            winioctl::{DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX},
            winnt::{FILE_ATTRIBUTE_NORMAL, GENERIC_READ},
        },
    };

    println!("Windows Physical Drives:");

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
                // Try to get drive size
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

                if success != 0 {
                    let size_gb = geometry.DiskSize as f64 / 1_073_741_824.0;
                    println!("  {} - {:.2} GB", drive_path, size_gb);
                } else {
                    println!("  {} - (unable to get size)", drive_path);
                }

                CloseHandle(handle);
            }
        }
    }

    println!("\nWindows Logical Drives:");
    unsafe {
        let drive_mask = GetLogicalDrives();
        if drive_mask != 0 {
            for i in 0..26 {
                if (drive_mask >> i) & 1 == 1 {
                    let drive_letter = (b'A' + i) as char;
                    let drive_path = format!(r"\\.\{}:", drive_letter);
                    println!("  {} - Volume", drive_path);
                }
            }
        }
    }

    Ok(())
}

fn list_drives() -> Result<()> {
    println!("Available drives and partitions for secure wiping:");
    println!();

    #[cfg(unix)]
    {
        use std::process::Command;

        println!("Unix Block Devices:");
        match Command::new("lsblk")
            .args(["-o", "NAME,TYPE,SIZE,MOUNTPOINT"])
            .output()
        {
            Ok(output) if output.status.success() => {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            _ => {
                println!("Failed to run lsblk. Common device paths:");
                println!("  /dev/sda, /dev/sda1, /dev/sda2, ... (SATA drives)");
                println!("  /dev/nvme0n1, /dev/nvme0n1p1, ... (NVMe drives)");
                println!("  /dev/mmcblk0, /dev/mmcblk0p1, ... (SD cards)");
            }
        }
    }

    #[cfg(windows)]
    {
        list_windows_drives()?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        println!("Platform not supported for drive enumeration.");
    }

    println!();
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

    Ok(())
}
