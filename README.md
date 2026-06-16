# rust-beyond-http

Companion code for the blog post **"Beyond HTTP in Rust: Real-Time Sockets and FTP — Built From Scratch"**.

Demonstrates:
- **Socket.IO server** using `socketioxide 0.18` + `axum 0.8`
- **FTP server** using `libunftp 0.23` + `unftp-sbe-fs 0.3` + `unftp-sbe-restrict 0.1.2`
- **FTP client** using `suppaftp 7` with retry logic
- **Combined system**: FTP upload → broadcast channel → Socket.IO push to all browsers

---

## Project Structure

```
src/
├── ftp_server/main.rs   — FTP server (auth + upload-only via unftp-sbe-restrict)
├── ftp_client/main.rs   — FTP client (upload, download, list, retry)
├── socket/main.rs       — Socket.IO server (rooms, events, lifecycle)
└── combined/main.rs     — Full pipeline: FTP upload → Socket.IO notification
index.html               — Browser test client
```

---

## Dependencies (all from crates.io — no local paths)

| Crate | Version | Purpose |
|-------|---------|---------|
| `socketioxide` | 0.18 | Socket.IO server |
| `axum` | 0.8 | Web framework |
| `suppaftp` | 7 | Async FTP client |
| `libunftp` | 0.23 | FTP server core |
| `unftp-sbe-fs` | 0.3 | Filesystem storage backend |
| `unftp-sbe-restrict` | 0.1.2 | Upload-only permission layer |

---

## Running

### FTP Server

```bash
FTP_ROOT=/tmp/ftp-uploads \
FTP_USER=uploader \
FTP_PASS=secret123 \
cargo run --bin ftp_server
```

### FTP Client

```bash
FTP_HOST=127.0.0.1:2121 \
FTP_USER=uploader \
FTP_PASS=secret123 \
cargo run --bin ftp_client
```

### Socket.IO Server

```bash
cargo run --bin socket_server
# Connect from browser: io("http://localhost:3000")
```

### Combined System

Start the FTP server first, then:

```bash
FTP_HOST=127.0.0.1:2121 \
FTP_USER=uploader \
FTP_PASS=secret123 \
cargo run --bin combined
```

Then open `index.html` in a browser — upload notifications arrive in real time.

---

## Permissions via unftp-sbe-restrict

`unftp-sbe-restrict` is an official crate that wraps any libunftp storage backend
and restricts operations per-user via `VfsOperations` bitflags.

| Operation | Allowed |
|-----------|---------|
| Upload (PUT) | ✅ |
| Download (GET) | ✅ |
| List (LIST) | ✅ |
| Delete (DELE) | ❌ |
| Rename (RNFR/RNTO) | ❌ |
| Make directory (MKD) | ❌ |
| Remove directory (RMD) | ❌ |

---

## Environment Variables

| Variable   | Default            | Description              |
|------------|--------------------|--------------------------|
| `FTP_HOST` | `127.0.0.1:2121`   | FTP server address       |
| `FTP_USER` | `uploader`         | FTP username             |
| `FTP_PASS` | `secret123`        | FTP password             |
| `FTP_ROOT` | `/tmp/ftp-uploads` | Server storage directory |
