/// Windows-specific utilities for disk and partition handling
#[cfg(windows)]
pub mod windows {
    use anyhow::{Context, Result};
    use std::path::Path;
    use winapi::{
        shared::minwindef::{DWORD, FALSE, LPVOID, TRUE},
        um::{
            errhandlingapi::GetLastError,
            fileapi::{CreateFileW, OPEN_EXISTING},
            handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
            winbase::{FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH},
            winnt::{FILE_ATTRIBUTE_NORMAL, GENERIC_READ, GENERIC_WRITE, HANDLE},
        },
    };

    /// Check if a path represents a Windows physical drive or logical drive
    pub fn is_windows_device_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        // Physical drives: \\.\PhysicalDrive0, \\.\PhysicalDrive1, etc.
        // Logical drives: \\.\C:, \\.\D:, etc.
        path_str.starts_with(r"\\.\PhysicalDrive")
            || (path_str.len() == 6 && path_str.starts_with(r"\\.\") && path_str.ends_with(':'))
    }

    /// Get the device type from a Windows device path
    pub fn get_device_type(path: &Path) -> DeviceType {
        let path_str = path.to_string_lossy();
        if path_str.starts_with(r"\\.\PhysicalDrive") {
            DeviceType::PhysicalDrive
        } else if path_str.len() == 6 && path_str.starts_with(r"\\.\") && path_str.ends_with(':') {
            DeviceType::LogicalDrive
        } else {
            DeviceType::File
        }
    }

    #[derive(Debug, PartialEq)]
    pub enum DeviceType {
        PhysicalDrive,
        LogicalDrive,
        File,
    }

    /// List available physical drives on Windows
    pub fn list_physical_drives() -> Result<Vec<String>> {
        let mut drives = Vec::new();

        for i in 0..32 {
            // Check up to 32 physical drives
            let drive_path = format!(r"\\.\PhysicalDrive{}", i);
            if test_drive_access(&drive_path) {
                drives.push(drive_path);
            }
        }

        Ok(drives)
    }

    /// List available logical drives on Windows
    pub fn list_logical_drives() -> Result<Vec<String>> {
        let mut drives = Vec::new();

        unsafe {
            let drive_mask = winapi::um::fileapi::GetLogicalDrives();
            if drive_mask == 0 {
                return Err(anyhow::anyhow!("Failed to get logical drives"));
            }

            for i in 0..26 {
                // A-Z drives
                if (drive_mask >> i) & 1 == 1 {
                    let drive_letter = (b'A' + i) as char;
                    let drive_path = format!(r"\\.\{}:", drive_letter);
                    drives.push(drive_path);
                }
            }
        }

        Ok(drives)
    }

    /// Test if we can access a drive (for enumeration)
    fn test_drive_access(drive_path: &str) -> bool {
        let wide_path: Vec<u16> = drive_path.encode_utf16().chain(Some(0)).collect();

        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                0, // No access, just test existence
                0, // No sharing
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );

            if handle != INVALID_HANDLE_VALUE {
                CloseHandle(handle);
                true
            } else {
                false
            }
        }
    }

    /// Get drive information for display purposes
    pub fn get_drive_info(drive_path: &str) -> Result<DriveInfo> {
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

            if handle == INVALID_HANDLE_VALUE {
                return Err(anyhow::anyhow!("Failed to open drive: {}", drive_path));
            }

            // Get drive geometry
            use winapi::um::{
                ioapiset::DeviceIoControl,
                winioctl::{DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX},
            };

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

            if success == 0 {
                return Err(anyhow::anyhow!("Failed to get drive geometry"));
            }

            Ok(DriveInfo {
                path: drive_path.to_string(),
                size_bytes: geometry.DiskSize as u64,
                cylinders: geometry.Geometry.Cylinders as u64,
                sectors_per_track: geometry.Geometry.SectorsPerTrack,
                bytes_per_sector: geometry.Geometry.BytesPerSector,
            })
        }
    }

    #[derive(Debug)]
    pub struct DriveInfo {
        pub path: String,
        pub size_bytes: u64,
        pub cylinders: u64,
        pub sectors_per_track: u32,
        pub bytes_per_sector: u32,
    }
}

#[cfg(not(windows))]
pub mod windows {
    use anyhow::Result;
    use std::path::Path;

    pub fn is_windows_device_path(_path: &Path) -> bool {
        false
    }

    pub fn get_device_type(_path: &Path) -> DeviceType {
        DeviceType::File
    }

    #[derive(Debug, PartialEq)]
    pub enum DeviceType {
        File,
    }

    pub fn list_physical_drives() -> Result<Vec<String>> {
        Ok(vec![])
    }

    pub fn list_logical_drives() -> Result<Vec<String>> {
        Ok(vec![])
    }
}
