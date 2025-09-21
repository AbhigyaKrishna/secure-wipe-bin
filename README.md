# Secure Wipe - File and Partition Wiping Utility

A fast, secure file and partition wiping utility with real-time progress visualization and JSON API support for integration with desktop applications like Electron.

## Features

- **Multiple Wiping Algorithms**: Zero, Random, DoD 5220.22-M, Gutmann (35-pass), and custom pass counts
- **File and Partition Support**: Securely wipe files or entire block devices/partitions
- **Real-time Progress**: Visual progress bars with throughput information
- **JSON API**: Machine-readable output for integration with GUI applications
- **Demo Mode**: Safe testing with temporary files
- **Cross-platform**: Works on Unix-like systems (Linux, macOS)
- **Modular Architecture**: Clean, maintainable codebase split into focused modules

## Installation

### From Source

```bash
git clone <repository-url>
cd secure-wipe-bin
cargo build --release
```

The binary will be available at `./target/release/secure-wipe-bin`.

### Dependencies

- Rust 1.70+ (with Cargo)
- Unix-like system for block device support
- Root privileges for partition wiping

## Usage

### Basic File Wiping

```bash
# Wipe a file with random data (1 pass)
./secure-wipe-bin --target /path/to/file.txt

# Use DoD 5220.22-M standard (3 passes)
./secure-wipe-bin --target /path/to/file.txt --algorithm dod5220

# Gutmann method (35 passes) - most secure
./secure-wipe-bin --target /path/to/file.txt --algorithm gutmann

# Custom number of passes
./secure-wipe-bin --target /path/to/file.txt --algorithm custom --passes 7
```

### Fast Mode (High Performance)

For maximum speed when security is less critical:

```bash
# Fast mode - disables O_SYNC for better performance
./secure-wipe-bin --target /path/to/file.txt --fast

# Fast mode with large buffer for maximum throughput
./secure-wipe-bin --target /path/to/file.txt --fast --buffer-size 16384

# Fast partition wipe (DANGEROUS but fastest!)
sudo ./secure-wipe-bin --target /dev/sda1 --algorithm random --fast --force
```

**⚠️ Fast Mode Trade-offs:**

- **Higher Performance**: Up to 10x faster on some systems
- **Less Data Integrity**: Disables synchronous writes (O_SYNC)
- **Use Cases**: Non-critical data, SSD wiping, performance testing

### Partition Wiping

**⚠️ WARNING: Partition wiping is irreversible and requires elevated privileges!**

#### Linux/Unix

```bash
# Wipe an entire partition (DANGEROUS!)
sudo ./secure-wipe-bin --target /dev/sda1 --algorithm dod5220 --force

# Check partition info first
lsblk -o NAME,TYPE,SIZE,MOUNTPOINT
sudo fdisk -l

# Always unmount before wiping
sudo umount /dev/sda1
```

#### Windows

```cmd
# Run as Administrator
# Wipe entire physical drive (EXTREMELY DANGEROUS!)
.\secure-wipe-bin.exe --target \\.\PhysicalDrive1 --algorithm dod5220 --force

# Wipe logical drive/partition
.\secure-wipe-bin.exe --target \\.\E: --algorithm random --force

# List available drives first
.\list-drives.exe
```

### List Available Drives

```bash
# List drives in human-readable format
./secure-wipe-bin --list-drives

# List drives in JSON format (for programmatic use)
./secure-wipe-bin --list-drives --json
```

### Demo Mode

Test the utility safely with temporary files:

```bash
# Create and wipe a 100MB demo file
./secure-wipe-bin --demo --demo-size 100 --algorithm dod5220

# Small 5MB demo with custom passes
./secure-wipe-bin --demo --demo-size 5 --algorithm custom --passes 3
```

### JSON Mode (for GUI Integration)

```bash
# Enable JSON output for programmatic use
./secure-wipe-bin --json --demo --demo-size 10 --force

# JSON output with partition wiping
sudo ./secure-wipe-bin --json --target /dev/sda1 --algorithm random --force
```

## Command Line Options

```
Usage: secure-wipe-bin [OPTIONS]

Options:
  -t, --target <TARGET>              Target file or block device/partition to wipe (Unix: /dev/sda1, Windows: \\\\.\\.\PhysicalDrive0 or \\\\.\\.\C:). Optional in demo mode.
  -a, --algorithm <ALGORITHM>        Wiping algorithm to use [default: random] [possible values: zero, random, dod5220, gutmann, custom]
  -p, --passes <PASSES>              Number of passes (for custom algorithm) [default: 3]
  -d, --demo                         Demo mode - creates and wipes test file safely
      --demo-size <DEMO_SIZE>        Size of demo file in MB [default: 100]
      --buffer-size <BUFFER_SIZE>    Buffer size in KB for wiping operations [default: 1024]
  -f, --force                        Force wipe without confirmation (dangerous!)
      --fast                         Fast mode - disable O_SYNC for better performance (less safe)
  -v, --verify                       Verify wipe by reading back data (not yet implemented)
      --json                         Output machine-readable JSON for subprocess integration
  -l, --list-drives                  List available drives and partitions instead of wiping
  -h, --help                         Print help
```

## Wiping Algorithms

| Algorithm | Passes       | Description                    | Use Case                                      |
| --------- | ------------ | ------------------------------ | --------------------------------------------- |
| `zero`    | 1            | Simple zero overwrite          | Fast, basic wiping                            |
| `random`  | 1            | Random data overwrite          | Default, good security/speed balance          |
| `dod5220` | 3            | DoD 5220.22-M standard         | Government standard                           |
| `gutmann` | 35           | Gutmann method                 | Maximum security (overkill for modern drives) |
| `custom`  | User-defined | Custom number of random passes | Configurable security level                   |

## JSON API Integration

The `--json` flag enables machine-readable output for integration with desktop applications. See [JSON_API.md](JSON_API.md) for complete documentation.

### Example JSON Events

```json
{"type": "start", "algorithm": "Dod5220", "total_passes": 3, "file_size_bytes": 1048576}
{"type": "progress", "pass": 1, "percent": 50.0, "bytes_per_second": 10485760.0}
{"type": "complete", "total_time_seconds": 2.5, "average_throughput_mb_s": 10.24}
```

### Node.js Integration Example

```javascript
const { spawn } = require("child_process");

function secureWipe(targetPath, options = {}) {
  const args = ["--json", "--force", "--target", targetPath];
  if (options.algorithm) args.push("--algorithm", options.algorithm);

  const process = spawn("./secure-wipe-bin", args);

  process.stdout.on("data", (data) => {
    const events = data.toString().trim().split("\\n");
    events.forEach((line) => {
      if (line) {
        const event = JSON.parse(line);
        handleProgressEvent(event);
      }
    });
  });

  return new Promise((resolve, reject) => {
    process.on("close", (code) => {
      code === 0 ? resolve() : reject(new Error(`Exit code: ${code}`));
    });
  });
}
```

## Architecture

The codebase is split into focused modules:

- `src/main.rs` - Main entry point and CLI coordination
- `src/args.rs` - Command-line argument parsing
- `src/algorithms.rs` - Wiping algorithm definitions and patterns
- `src/wipe.rs` - Core wiping logic and progress handling
- `src/demo.rs` - Demo file creation utilities
- `src/ui.rs` - User interaction and confirmation prompts
- `src/progress.rs` - JSON progress event system

## Security Considerations

### General

- **Verify targets** before wiping to prevent accidental data loss
- **Use appropriate algorithms** based on security requirements
- **Test with demo mode** before wiping real data
- **Keep logs** of wiping operations for audit trails

### Partition Wiping

- **Requires root privileges** - handle elevation carefully in GUI apps
- **Unmount partitions** before wiping to prevent corruption
- **Double-check device paths** - `/dev/sda` vs `/dev/sda1` makes a huge difference
- **Verify device identity** using `lsblk`, `fdisk -l`, or similar tools
- **Consider backup** critical data before wiping

### Integration Security

- **Validate all user inputs** before passing to secure-wipe
- **Sandbox the process** when possible
- **Use `--force` cautiously** - it bypasses safety prompts
- **Handle elevated privileges** securely in GUI applications

## Performance

### Optimization Tips

- **Buffer size**: The tool automatically selects optimal buffer sizes based on device type:
  - **Block devices**: 2-16MB (depending on available memory)
  - **Files**: 1-8MB (depending on available memory)
  - **Manual override**: Use `--buffer-size` to specify custom size in KB
- **Fast mode**: Use `--fast` to disable O_SYNC for up to 10x performance improvement
- **Algorithm choice**:
  - `zero` or `random` (1 pass) for best speed
  - Avoid `gutmann` (35 passes) unless maximum security is required
- **Progress throttling**: JSON events limited to reduce I/O overhead
- **Direct I/O**: Automatically enabled for block devices to bypass kernel caching

### Performance Comparison

| Mode                                 | Speed            | Security | Use Case                 |
| ------------------------------------ | ---------------- | -------- | ------------------------ |
| Normal                               | Baseline         | High     | Production wiping        |
| Fast (`--fast`)                      | Up to 10x faster | Medium   | SSD wiping, testing      |
| Large buffer (`--buffer-size 16384`) | 2-5x faster      | High     | Large files/devices      |
| Fast + Large buffer                  | Up to 15x faster | Medium   | Non-critical bulk wiping |

- **Progress throttling**: JSON events limited to ~100ms intervals
- **Synchronous writes**: Uses O_SYNC for data integrity (disabled in fast mode)
- **Block device optimization**: Direct device access for partitions

## Platform Support

- **Linux**: Full support for files and block devices
- **macOS**: File support, limited block device support
- **Windows**: Full support for files and disk/partition wiping (requires Administrator privileges)

### Windows Device Paths

- **Physical drives**: `\\.\PhysicalDrive0`, `\\.\PhysicalDrive1`, etc.
- **Logical drives**: `\\.\C:`, `\\.\D:`, etc.
- Use the `list-drives` utility to enumerate available devices

See [WINDOWS_SUPPORT.md](WINDOWS_SUPPORT.md) for detailed Windows-specific documentation.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure `cargo test` passes
6. Submit a pull request

## License

[Add your license here]

## Disclaimer

**THIS SOFTWARE CAN PERMANENTLY DESTROY DATA. USE WITH EXTREME CAUTION.**

The authors are not responsible for data loss resulting from the use of this software. Always verify your target files/devices before running secure wipe operations. When in doubt, use demo mode first.
