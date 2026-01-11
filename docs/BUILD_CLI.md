# Building and Running the CLI

## Quick Start

### Build

**Basic build (without OpenAPI support):**
```bash
cargo build --release --bin data-modelling-cli --features cli
```

**Build with OpenAPI support:**
```bash
cargo build --release --bin data-modelling-cli --features cli,openapi
```

### Run
```bash
# Using cargo run (development)
cargo run --bin data-modelling-cli --features cli -- --help

# Using the built binary
./target/release/data-modelling-cli --help
```

### Install

**Basic installation (without OpenAPI support):**
```bash
cargo install --path . --bin data-modelling-cli --features cli
```

**Install with OpenAPI support:**
```bash
cargo install --path . --bin data-modelling-cli --features cli,openapi
```

## GitHub Releases

The CLI is automatically built and published to GitHub Releases when you push a version tag (e.g., `v1.6.2`).

### Manual Release

To manually trigger a CLI release:

1. Create and push a version tag:
   ```bash
   git tag v1.6.2
   git push origin v1.6.2
   ```

2. The `.github/workflows/release-cli.yml` workflow will automatically:
   - Build binaries for Linux (x86_64), macOS (Intel & Apple Silicon), and Windows (x86_64)
   - Create a GitHub Release with all binaries
   - Include checksums for verification

### Download Pre-built Binaries

Visit the [Releases page](https://github.com/OffeneDatenmodellierung/data-modelling-sdk/releases) to download pre-built binaries for your platform.

## CI Integration

The CLI is built as part of the CI pipeline (`.github/workflows/ci.yml`) to ensure it compiles successfully on every PR and push to main.
