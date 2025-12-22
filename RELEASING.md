# Releasing vectX

This document describes how to create a new release of vectX.

## Prerequisites

- Write access to the repository
- [GitHub CLI](https://cli.github.com/) installed and authenticated (optional, for manual releases)

## Automated Release Process

vectX uses GitHub Actions to automatically build and publish releases when a version tag is pushed.

### Step 1: Update Version Numbers

Update the version in all `Cargo.toml` files:

```bash
# Main crate
# vectx/Cargo.toml -> version = "0.2.0"

# Sub-crates
# vectx/lib/core/Cargo.toml -> version = "0.2.0"
# vectx/lib/storage/Cargo.toml -> version = "0.2.0" and vectx-core dependency
# vectx/lib/api/Cargo.toml -> version = "0.2.0" and vectx-core, vectx-storage dependencies
```

Files to update:
- `Cargo.toml` (root)
- `lib/core/Cargo.toml`
- `lib/storage/Cargo.toml`
- `lib/api/Cargo.toml`

### Step 2: Update Release Notes

Edit `RELEASE_NOTES.md` with the changes for this version:

```markdown
# vectX v0.2.0

## Highlights
- New feature X
- Performance improvement Y

## Changes
- Added: ...
- Fixed: ...
- Changed: ...
```

### Step 3: Commit and Tag

```bash
# Stage all changes
git add -A

# Commit with release message
git commit -m "Release v0.2.0"

# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push commit and tag
git push && git push origin v0.2.0
```

### Step 4: Monitor the Release

The GitHub Actions workflow will automatically:
1. Build binaries for all platforms (macOS, Linux)
2. Create SHA256 checksums
3. Create a GitHub Release with all assets

Monitor progress at: https://github.com/antonellof/vectX/actions

### Step 5: Publish to crates.io (Optional)

After the GitHub release is created, publish to crates.io:

```bash
# Publish in dependency order
cargo publish -p vectx-core
cargo publish -p vectx-storage
cargo publish -p vectx-api
cargo publish -p vectx
```

## Release Artifacts

Each release includes:

| Platform | File | Description |
|----------|------|-------------|
| macOS ARM64 | `vectx-aarch64-apple-darwin.tar.gz` | Apple Silicon Macs |
| macOS x86_64 | `vectx-x86_64-apple-darwin.tar.gz` | Intel Macs |
| Linux x86_64 | `vectx-x86_64-unknown-linux-gnu.tar.gz` | Linux with glibc |
| Linux x86_64 | `vectx-x86_64-unknown-linux-musl.tar.gz` | Linux static binary |

Each archive includes:
- `vectx` binary
- `README.md`
- `LICENSE-MIT`
- `LICENSE-APACHE`

## Local Release Build

To build a release locally for the current platform:

```bash
./scripts/release.sh [version]

# Example
./scripts/release.sh 0.2.0
```

Output will be in `releases/v0.2.0/`.

## Troubleshooting

### GitHub Actions fails

1. Check the Actions logs: https://github.com/antonellof/vectX/actions
2. Common issues:
   - Missing `protobuf-compiler` → Already handled in workflow
   - Rust compilation errors → Fix and re-tag

### Re-releasing a version

If you need to fix a release:

```bash
# Delete local tag
git tag -d v0.2.0

# Delete remote tag
git push origin :refs/tags/v0.2.0

# Make fixes, then re-tag
git add -A && git commit -m "Fix release v0.2.0"
git tag -a v0.2.0 -m "Release v0.2.0"
git push && git push origin v0.2.0
```

### Manual GitHub Release

If automation fails, create manually:

```bash
# Build locally
./scripts/release.sh 0.2.0

# Create release via CLI
gh release create v0.2.0 releases/v0.2.0/* \
  --title "v0.2.0" \
  --notes-file RELEASE_NOTES.md
```

Or use the GitHub web interface at:
https://github.com/antonellof/vectX/releases/new

## Version Naming

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0): Breaking API changes
- **MINOR** (0.2.0): New features, backwards compatible
- **PATCH** (0.1.1): Bug fixes, backwards compatible

Pre-release versions:
- `v0.2.0-alpha.1`
- `v0.2.0-beta.1`
- `v0.2.0-rc.1`
