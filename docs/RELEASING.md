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
git add Cargo.toml packaging/homebrew/modelmux.rb CHANGELOG.md
git commit -m "Release v0.5.0"
git tag v0.5.0
git push origin main --tags

# 6. Create GitHub release (via web UI)
# - Upload source tarball
# - Get SHA256: shasum -a 256 modelmux-0.1.0.tar.gz

# 7. Update Homebrew formula with SHA256
vim packaging/homebrew/modelmux.rb  # Update sha256 field

# 8. Test Homebrew formula locally (use a local tap - see section 7)
# 9. Publish to your Homebrew tap (see section 8)
```

## Detailed Steps

### 1. Update Version

**Cargo.toml:**
```toml
[package]
version = "0.5.0"  # Update this
```

**packaging/homebrew/modelmux.rb:**
```ruby
url "https://github.com/yarenty/modelmux/archive/refs/tags/v0.5.0.tar.gz"
# Update version in URL; replace sha256 after creating the release tarball
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
git add Cargo.toml packaging/homebrew/modelmux.rb CHANGELOG.md
git commit -m "Release v0.5.0"
git tag v0.5.0
git push origin main
git push origin v0.5.0
```

### 5. Create GitHub Release

1. Go to GitHub Releases page
2. Click "Draft a new release"
3. Select tag `v0.5.0`
4. Upload source tarball (GitHub auto-generates or create manually)
5. Get SHA256:
   ```bash
      curl -sL https://github.com/yarenty/modelmux/archive/refs/tags/v0.5.0.tar.gz | shasum -a 256
     ```

### 6. Update Homebrew Formula SHA256

**packaging/homebrew/modelmux.rb:**
```ruby
sha256 "abc123..."  # Update with actual SHA256 from step 5
```

### 7. Test Homebrew Formula Locally

Modern Homebrew requires formulae to be in a tap; it will not install from a bare file path. Use a **local tap** to test before publishing.

**One-time setup:** Create a local tap (replace `yarenty` with your GitHub username):

```bash
brew tap-new yarenty/tap
```

This creates a directory like `$(brew --prefix)/Library/Taps/yarenty/homebrew-tap/`. You can use it only locally or later push it to GitHub.

**Each time you want to test the formula** (from the modelmux repo root). Use the tap path from brew (bash/zsh):

```bash
# Create Formula dir if needed, then copy formula into your local tap
TAP_DIR=$(brew --repository yarenty/tap)
mkdir -p "$TAP_DIR/Formula"
cp packaging/homebrew/modelmux.rb "$TAP_DIR/Formula/modelmux.rb"

# Install from the tap (build from source)
brew install --build-from-source yarenty/tap/modelmux

# Test
brew test modelmux
modelmux --version
modelmux --help
```

To re-test after changing the formula, copy again and run `brew reinstall yarenty/tap/modelmux`.

**Fish shell:** use `set TAP_DIR (brew --repository yarenty/tap)` instead of `TAP_DIR=...`. If you get "Permission denied", the tap path may be under a different prefix; run `brew --repository yarenty/tap` to see the real path and use it.

### 8. Publish to Homebrew tap

Tap repo: **https://github.com/yarenty/homebrew-tap**

**Update formula and push** (after a new release: bump version + SHA256 in `packaging/homebrew/modelmux.rb`, then):

```bash
TAP_DIR=$(brew --repository yarenty/tap)
cp packaging/homebrew/modelmux.rb "$TAP_DIR/Formula/modelmux.rb"
cd "$TAP_DIR"
git add Formula/modelmux.rb
git commit -m "modelmux 0.5.0"
git push origin main
```

**Install (for users):** `brew tap yarenty/tap` then `brew install modelmux`

## Version Numbering

[Semantic Versioning](https://semver.org/): MAJOR.MINOR.PATCH (breaking / feature / fix)

## See Also

- [TESTING.md](TESTING.md) - Testing guide
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
