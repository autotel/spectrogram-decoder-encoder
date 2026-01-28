# Building for Multiple Platforms

## Option 1: GitHub Actions (Recommended)

The easiest way to build for all platforms is using GitHub Actions:

1. Push this code to GitHub
2. Go to the "Actions" tab in your repo
3. Run the "Build Release" workflow manually, OR
4. Create a git tag to trigger automatic builds:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

This will automatically build for:
- Linux x86_64
- Windows x86_64
- macOS x86_64 (Intel)
- macOS ARM64 (Apple Silicon)

The binaries will be available as:
- Artifacts (for manual workflow runs)
- Release assets (for tagged releases)

## Option 2: cargo-zigbuild (Local cross-compilation)

```bash
# Install dependencies
cargo install cargo-zigbuild
sudo apt install zig  # or download from ziglang.org

# Build for each platform
cargo zigbuild --release --target x86_64-pc-windows-gnu
cargo zigbuild --release --target x86_64-unknown-linux-gnu
cargo zigbuild --release --target x86_64-apple-darwin
cargo zigbuild --release --target aarch64-apple-darwin
```

Binaries will be in `target/<target>/release/`

## Option 3: Native compilation

Build on each platform separately:
- Linux: `cargo build --release`
- Windows: `cargo build --release`
- macOS: `cargo build --release`

## Current Build Status

GitHub Actions will show build status for all platforms at:
`https://github.com/<your-username>/<repo-name>/actions`
