# Building recv

`recv` is an HTTP upload server with QR pairing. It builds two ways:

- **Standalone CLI** - its own binary, like `rftps`.
- **Library** - `recv` is also the crate behind the ExifFlow desktop app's
  browser-upload feature.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (Edition 2024 supported)

## Build Options

### Standard Build

Builds plain HTTP (no embedded TLS material). Provide certificates at runtime
with `--cert-pem`/`--key-pem` when using `--enable-https`.

```bash
cargo build --release
```

### Build with Embedded PEM Files

Embeds `cert.pem` and `key.pem` from the crate root so HTTPS works without
passing certificate paths.

```bash
cargo build --release --features include-pem-files
```

**Note:** the feature reads `cert.pem` and `key.pem` at compile time and embeds
them. Generate a self-signed pair if you do not have one:

```bash
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=RECV"
```

## Usage after Build

The compiled binary is at `target/release/recv` (`recv.exe` on Windows).

```bash
./target/release/recv --directory ./uploads
```

## Dependency on exifflow-core

Standalone builds fetch `exifflow-core` (the shared event bus / record store /
utilities) from its git tag, so the repo and tag must exist and be reachable.
Inside the ExifFlow workspace this dependency is overridden with a local path via
`[patch]` in the workspace `Cargo.toml`.
