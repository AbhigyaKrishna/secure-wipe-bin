# JSON API Documentation for Electron Integration

This document describes the JSON API for integrating the secure-wipe utility as a subprocess in Electron applications.

## Command Line Usage

To enable JSON output mode, use the `--json` flag:

```bash
./secure-wipe-bin --json --target /path/to/file --algorithm dod5220 --force
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

Emitted periodically during wiping (approximately every 100ms).

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
4. Manage process lifecycle
5. Handle errors appropriately

## Error Handling

- **stdout**: Contains JSON events (one per line)
- **stderr**: Contains non-JSON error messages and debugging info
- **Exit code**: 0 for success, non-zero for failure

Always check both the exit code and listen for error events in the JSON stream.

## Performance Considerations

- Progress events are throttled to ~100ms intervals to avoid overwhelming the parent process
- In JSON mode, terminal progress bars are disabled for better performance
- Buffer size can be adjusted with `--buffer-size` for optimal throughput

## Security Notes

When integrating with Electron:

1. **Always validate user input** before passing paths to the secure-wipe process
2. **Use `--force` carefully** - it skips confirmation prompts
3. **Consider sandboxing** the secure-wipe process
4. **Validate file paths** to prevent unauthorized access
5. **Log operations** for audit trails

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
