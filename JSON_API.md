# JSON API Documentation for Electron Integration

This document describes the JSON API for integrating the secure-wipe utility as a subprocess in Electron applications. The utility supports wiping both files and block devices (partitions).

## Command Line Usage

To enable JSON output mode, use the `--json` flag:

```bash
# Wipe a file
./secure-wipe-bin --json --target /path/to/file --algorithm dod5220 --force

# Wipe a partition (Unix - requires root privileges)
sudo ./secure-wipe-bin --json --target /dev/sda1 --algorithm gutmann --force

# Wipe a partition (Windows - requires Administrator privileges)
.\secure-wipe-bin.exe --json --target \\.\PhysicalDrive0 --algorithm dod5220 --force
.\secure-wipe-bin.exe --json --target \\.\C: --algorithm random --force

# List available drives in JSON format
./secure-wipe-bin --list-drives --json

# Get system information in JSON format
./secure-wipe-bin --system-info --json

# Demo mode (creates temporary file)
./secure-wipe-bin --json --demo --demo-size 100 --algorithm random --force
```

## Drive Listing

The `--list-drives` flag can be combined with `--json` to get machine-readable drive information:

```bash
# Get drive list in JSON format
./secure-wipe-bin --list-drives --json
```

### Drive List JSON Output

```json
{
  "type": "drive_list",
  "drives": [
    {
      "path": "/dev/sda",
      "drive_type": "disk",
      "size_bytes": 1000204886016,
      "size_gb": 931.5,
      "description": "/dev/sda - disk 931.5G"
    },
    {
      "path": "/dev/sda1",
      "drive_type": "part",
      "size_bytes": null,
      "size_gb": 100.0,
      "description": "/dev/sda1 - part 100G"
    }
  ]
}
```

## System Information

The `--system-info` flag can be combined with `--json` to get machine-readable system information:

```bash
# Get system info in JSON format
./secure-wipe-bin --system-info --json
```

### System Info JSON Output

```json
{
  "os_name": "Linux",
  "os_version": "Linux version 6.5.0-generic",
  "architecture": "x86_64",
  "hostname": "workstation",
  "username": "user",
  "total_memory_bytes": 16777216000,
  "available_memory_bytes": 8388608000,
  "cpu_info": {
    "logical_cores": 8,
    "physical_cores": 4,
    "model_name": "Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz",
    "frequency_mhz": 3700
  },
  "storage_devices": [
    {
      "name": "sda1",
      "device_path": "/dev/sda1",
      "size_bytes": 1000204886016,
      "device_type": "block",
      "mount_point": "/",
      "file_system": "ext4"
    },
    {
      "name": "sdb1",
      "device_path": "/dev/sdb1",
      "size_bytes": 2000398934016,
      "device_type": "block",
      "mount_point": "/home",
      "file_system": "ext4"
    }
  ]
}
```

## JSON Event Types

All events are emitted as single-line JSON objects to stdout. Each event has a `type` field indicating the event type.

### Start Event

Emitted when the wiping process begins.

```json
{
  "type": "start",
  "algorithm": "Dod5220",
  "total_passes": 3,
  "file_size_bytes": 1048576,
  "buffer_size_kb": 1024
}
```

### Pass Start Event

Emitted when a new wiping pass begins.

```json
{
  "type": "pass_start",
  "pass": 1,
  "total_passes": 3,
  "pattern": "0x00"
}
```

### Progress Event

Emitted periodically during wiping (intervals optimized based on mode: 200ms for UI, 500ms for JSON, 2s for fast mode).

```json
{
  "type": "progress",
  "pass": 1,
  "total_passes": 3,
  "bytes_written": 524288,
  "total_bytes": 1048576,
  "percent": 50.0,
  "bytes_per_second": 10485760.0
}
```

### Pass Complete Event

Emitted when a wiping pass is finished.

```json
{
  "type": "pass_complete",
  "pass": 1,
  "total_passes": 3
}
```

### Complete Event

Emitted when the entire wiping process is finished.

```json
{
  "type": "complete",
  "total_time_seconds": 2.5,
  "average_throughput_mb_s": 10.24
}
```

### Demo File Events

For demo mode, additional events are emitted during file creation.

#### Demo File Creating

```json
{
  "type": "demo_file_creating",
  "bytes_written": 524288,
  "total_bytes": 1048576,
  "percent": 50.0
}
```

#### Demo File Created

```json
{
  "type": "demo_file_created",
  "path": "/tmp/secure_wipe_demo_12345.img",
  "size_mb": 1
}
```

### Info Event

General informational messages.

```json
{
  "type": "info",
  "message": "Creating demo file: /tmp/demo.img (Size: 5 MB)"
}
```

### Error Event

Error messages and failures.

```json
{
  "type": "error",
  "message": "Failed to open file: Permission denied"
}
```

## Integration Example

See `example-electron-integration.js` for a complete Node.js example showing how to:

1. Spawn the secure-wipe process
2. Parse JSON events from stdout
3. Handle progress updates
4. Handle errors appropriately
5. Manage process lifecycle

## Error Handling

- **stdout**: Contains JSON events (one per line)
- **stderr**: Contains non-JSON error messages and debugging info
- **Exit code**: 0 for success, non-zero for failure

Always check both the exit code and listen for error events in the JSON stream.

## Performance Considerations

- Progress events are throttled to optimize performance (200ms intervals for UI mode, 500ms for JSON mode, 2s for fast mode)
- In JSON mode, terminal progress bars are disabled for better performance
- Buffer size can be adjusted with `--buffer-size` for optimal throughput
- Larger buffer sizes (4-64MB) are automatically selected based on available system memory and device type
- Direct I/O is used instead of buffered writes for maximum performance
- Synchronization is performed only at the end of each pass, not during writes

## Security Notes

When integrating with Electron:

1. **Always validate user input** before passing paths to the secure-wipe process
2. **Use `--force` carefully** - it skips confirmation prompts
3. **Consider sandboxing** the secure-wipe process
4. **Validate file paths** to prevent unauthorized access
5. **Log operations** for audit trails
6. **Partition wiping requires elevated privileges** - ensure proper permission handling
7. **Block device detection** - verify the target is the intended device before wiping
8. **Unmount partitions** before wiping to prevent data corruption
9. **Double-check device paths** - wiping the wrong partition is irreversible

## Example Integration in Electron Main Process

```javascript
const { spawn } = require("child_process");
const { dialog } = require("electron");

async function secureWipeFile(filePath, onProgress) {
  const wipeProcess = spawn("./secure-wipe-bin", [
    "--json",
    "--force",
    "--target",
    filePath,
    "--algorithm",
    "dod5220",
  ]);

  wipeProcess.stdout.on("data", (data) => {
    const lines = data.toString().trim().split("\\n");
    lines.forEach((line) => {
      if (line.trim()) {
        const event = JSON.parse(line);
        onProgress(event);
      }
    });
  });

  return new Promise((resolve, reject) => {
    wipeProcess.on("close", (code) => {
      code === 0 ? resolve() : reject(new Error(`Exit code: ${code}`));
    });
  });
}
```
