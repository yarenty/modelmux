# Release Process

Step-by-step guide for releasing new versions of ModelMux, including Homebrew deployment.

## Quick Release Checklist

```bash
# 1. Update version in Cargo.toml
vim Cargo.toml  # Update version field

# 2. Update version in Homebrew formula
vim packaging/homebrew/modelmux.rb  # Update url and version

# 3. Run tests
cargo test

# 4. Build release binary
cargo build --release
./target/release/modelmux --version

# 5. Commit and tag
git add Cargo.toml packaging/homebrew/modelmux.rb
git commit -m "Release v0.1.0"
git tag v0.1.0
git push origin main --tags

# 6. Create GitHub release (via web UI)
# - Upload source tarball
# - Get SHA256: shasum -a 256 modelmux-0.1.0.tar.gz

# 7. Update Homebrew formula with SHA256
vim packaging/homebrew/modelmux.rb  # Update sha256 field

# 8. Test Homebrew formula locally
brew install --build-from-source packaging/homebrew/modelmux.rb
brew test modelmux

# 9. Submit to Homebrew (see below)
```

## Detailed Steps

### 1. Update Version

**Cargo.toml:**
```toml
[package]
version = "0.1.0"  # Update this
```

**packaging/homebrew/modelmux.rb:**
```ruby
url "https://github.com/yarenty/modelmux/archive/refs/tags/v0.1.0.tar.gz"
# Update version in URL
```

### 2. Run Tests

```bash
cargo test
cargo test --test cli_tests
cargo test --test config_tests
```

### 3. Build and Verify

```bash
cargo build --release
./target/release/modelmux --version
./target/release/modelmux --help
```

### 4. Create Git Tag and Release

```bash
git add Cargo.toml packaging/homebrew/modelmux.rb
git commit -m "Release v0.1.0"
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

### 5. Create GitHub Release

1. Go to GitHub Releases page
2. Click "Draft a new release"
3. Select tag `v0.1.0`
4. Upload source tarball (GitHub auto-generates or create manually)
5. Get SHA256:
   ```bash
   shasum -a 256 modelmux-0.1.0.tar.gz
   ```

### 6. Update Homebrew Formula SHA256

**packaging/homebrew/modelmux.rb:**
```ruby
sha256 "abc123..."  # Update with actual SHA256 from step 5
```

### 7. Test Homebrew Formula Locally

```bash
# Install from local formula
brew install --build-from-source packaging/homebrew/modelmux.rb

# Test installed binary
brew test modelmux
modelmux --version
modelmux --help
```

### 8. Submit to Homebrew

#### Option A: Homebrew Tap (Recommended for first release)

```bash
# Create tap repository: homebrew-tap
# Copy formula
cp packaging/homebrew/modelmux.rb /path/to/homebrew-tap/Formula/modelmux.rb

# Commit and push
cd /path/to/homebrew-tap
git add Formula/modelmux.rb
git commit -m "Add modelmux formula"
git push origin main
```

Users install with:
```bash
brew tap yarenty/tap
brew install modelmux
```

#### Option B: homebrew-core (After tap is established)

```bash
# Fork homebrew-core
# Create branch
git checkout -b modelmux

# Copy formula
cp packaging/homebrew/modelmux.rb /path/to/homebrew-core/Formula/modelmux.rb

# Commit and push
git add Formula/modelmux.rb
git commit -m "Add modelmux formula"
git push origin modelmux

# Create PR on GitHub
```

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes

Examples:
- `0.1.0` → `0.1.1` (patch)
- `0.1.0` → `0.2.0` (minor)
- `0.1.0` → `1.0.0` (major)

## Troubleshooting

### Homebrew install fails

```bash
# Check formula syntax
brew audit --strict packaging/homebrew/modelmux.rb

# Check for issues
brew doctor
```

### Tests fail

```bash
# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_version_flag
```

### SHA256 mismatch

```bash
# Re-download tarball and verify
curl -L -o modelmux-0.1.0.tar.gz https://github.com/yarenty/modelmux/archive/refs/tags/v0.1.0.tar.gz
shasum -a 256 modelmux-0.1.0.tar.gz
```

## See Also

- [TESTING.md](TESTING.md) - Testing guide
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
