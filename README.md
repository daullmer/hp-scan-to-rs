# hp-scan-to

Scan to directory or email from HP network scanners using the eSCL protocol.

The application registers one or more destinations on your HP scanner's LCD panel. When you select a destination and press Scan, the scanned pages are saved as PDF/JPEG to a local directory or emailed as a PDF attachment via [Resend](https://resend.com).

## Features

- **Scan to directory** — save as PDF or JPEG
- **Scan to email** — send PDF attachments via the Resend API
- **Multiple destinations** — each with independent scan settings
- **Duplex scanning** — for scanners with ADF duplex support
- **Configurable resolution, color mode, and paper size**

## Configuration

Create a `config.toml` file:

```toml
[scanner]
ip = "192.168.1.53"

# Save scans as PDF to a local directory
[[destinations]]
label = "Scan to Documents"
output = "directory"
directory = "/scans"
format = "pdf"
resolution = 300
color_mode = "color"

# Email scans as PDF
[[destinations]]
label = "Email Alice"
output = "email"
to = "alice@example.com"
from = "scanner@company.com"
subject = "Scan from HP"
resolution = 200
color_mode = "gray"
```

### Destination options

| Option       | Values                                              | Default   |
| ------------ | --------------------------------------------------- | --------- |
| `label`      | Text shown on the scanner LCD (required)            | —         |
| `output`     | `"directory"` or `"email"` (required)               | —         |
| `directory`  | Path to save files (required for `directory` output) | —         |
| `format`     | `"pdf"` or `"jpeg"` (required for `directory`)       | —         |
| `to`         | Recipient address (required for `email`)             | —         |
| `from`       | Sender address (required for `email`)                | —         |
| `subject`    | Email subject line                                   | —         |
| `resolution` | DPI                                                  | `200`     |
| `color_mode` | `"color"`, `"gray"`, or `"bw"`                       | `"color"` |
| `paper_size` | `"a3"`, `"a4"`, `"a5"`, `"b5"`, `"letter"`, `"legal"`, `"max"` | `"a4"` |
| `duplex`     | `true` / `false`                                     | `false`   |

### Environment variables

| Variable         | Description                                   |
| ---------------- | --------------------------------------------- |
| `RESEND_API_KEY` | Required when using `email` output destinations |
| `RUST_LOG`       | Log level filter (e.g. `hp_scan_to=debug`)     |

## Docker

Pre-built images for `linux/amd64` and `linux/arm64` are published to GitHub Container Registry.

```sh
docker run -d \
  --name hp-scan-to \
  --network host \
  -v /path/to/config.toml:/config/config.toml:ro \
  -v /path/to/scans:/scans \
  ghcr.io/daullmer/hp-scan-to-rs:latest
```

If using email destinations, pass the Resend API key:

```sh
docker run -d \
  --name hp-scan-to \
  --network host \
  -e RESEND_API_KEY=re_xxxxxxxxxx \
  -v /path/to/config.toml:/config/config.toml:ro \
  -v /path/to/scans:/scans \
  ghcr.io/daullmer/hp-scan-to-rs:latest
```

> `--network host` is recommended so the container can discover and communicate with the scanner on your local network.

## Docker Compose

```yaml
services:
  hp-scan-to:
    image: ghcr.io/daullmer/hp-scan-to-rs:latest
    container_name: hp-scan-to
    network_mode: host
    restart: unless-stopped
    volumes:
      - ./config.toml:/config/config.toml:ro
      - ./scans:/scans
    # environment:
    #   RESEND_API_KEY: re_xxxxxxxxxx
```

```sh
docker compose up -d
```

## Pre-built binaries

Statically linked binaries for Linux are attached to each [GitHub release](https://github.com/daullmer/hp-scan-to-rs/releases):

| Archive | Architecture |
| ------- | ------------ |
| `hp-scan-to-amd64.tar.gz` | x86-64 |
| `hp-scan-to-arm64.tar.gz` | ARM64 (Raspberry Pi 4/5, etc.) |

```sh
tar xzf hp-scan-to-amd64.tar.gz
./hp-scan-to --config config.toml
```

No runtime dependencies required — the binary is fully self-contained.

## Building from source

```sh
cargo build --release
```

Run directly:

```sh
./target/release/hp-scan-to --config config.toml
```

### CLI options

```
Usage: hp-scan-to [OPTIONS]

Options:
  -c, --config <FILE>  Path to the TOML configuration file
                        [default: ~/.config/hp-scan-to/config.toml]
      --ip <IP>        Override the scanner IP from the config file
  -h, --help           Print help
```
