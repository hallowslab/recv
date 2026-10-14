# recv

[![Build Status](https://github.com/hallowslab/recv/actions/workflows/rust.yml/badge.svg)](https://github.com/hallowslab/recv/actions/workflows/rust.yml)

`recv` is an HTTP upload server with QR pairing. It ships two ways:

* **Standalone CLI** - its own binary, like `rftps`.
* **Library** - the crate behind ExifFlow's desktop browser-upload feature.

It is a sibling of `rftps`: same upload root, same event bus, same durable record
store. A phone scans a QR code, pairs with a URL-fragment token, and uploads files
from the browser. Uploads feed the same replication pipeline as FTP.

## Why a fragment token

The QR encodes `http://<lan-ip>:<port>/#t=<token>`. The `#fragment` is never sent
to the server, so it cannot land in server logs, proxy logs, or the `Referer`
header. The page reads the fragment, clears it from the address bar, then POSTs
the token in the request body to exchange it for an `HttpOnly; SameSite=Strict`
session cookie. Uploads then authenticate with that cookie.

## Quick start

```bash
recv --directory ./uploads
```

Prints the URL, credentials, and a scannable QR code. Scan it with a phone and
upload.

## Command line options

| Option | Default | Description |
| ------ | ------- | ----------- |
| `--address` | `0.0.0.0` | bind address |
| `--port` | `41213` | HTTP port |
| `--directory` | `./recv` | upload root (auto-created) |
| `--username` | `recv` | username reported for uploads |
| `--password` | random | password used with `--require-password` |
| `--require-password` | off | require username and password on pairing |
| `--enable-https` | `false` | serve HTTPS |
| `--cert-pem`, `--key-pem` | - | TLS certificate and key |
| `--external-host` | LAN IP | host advertised in the QR URL |
| `--record-store` | - | SQLite durable store the replicator replays |
| `--token` | random | fixed pairing token |
| `--no-qr` | off | do not print the QR code |
| `--max-upload-size` | unlimited | maximum accepted upload size in bytes |

## Replication parity

With `--record-store`, uploads are appended as `pending` records to the same
SQLite schema `rftps` writes, and a `FileUploaded` event is published on the
shared event bus. `replicator-rs` replays them exactly like FTP uploads.

```bash
recv --directory ./uploads --record-store replication.db
replicator --store replication.db --config bg.json
```

## HTTPS

HTTPS is off by default because self-signed certificates make phone browsers show
a warning. When enabled, pass `--cert-pem`/`--key-pem`, or build with
`--features include-pem-files` to embed `cert.pem`/`key.pem` from the crate root.

## Build

```bash
cargo build --release
cargo test
```

With embedded TLS certificates (HTTPS without passing cert paths):

```bash
cargo build --release --features include-pem-files
```

See [`docs/build.md`](docs/build.md) for details, including the
`exifflow-core` git dependency for standalone builds.

## Releases

Tagged releases (`v*.*.*`) build binaries for Linux, macOS (x64 and arm64), and
Windows via GitHub Actions, then publish them with the notes in
`docs/releases/<tag>.md`. See
[`.github/workflows/rust.yml`](.github/workflows/rust.yml).
