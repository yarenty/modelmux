# Homebrew Formula

This directory contains the Homebrew formula for ModelMux.

## File Structure

```
packaging/
  homebrew/
    modelmux.rb    # Homebrew formula
    README.md      # This file
```

## Usage

### Testing Locally

1. **One-time:** Create a local tap (replace `yarenty` with your GitHub username):
   ```bash
   brew tap-new yarenty/tap
   ```

2. **From the modelmux repo root**, create Formula dir, copy the formula, and install (bash/zsh):
   ```bash
   TAP_DIR=$(brew --repository yarenty/tap)
   mkdir -p "$TAP_DIR/Formula"
   cp packaging/homebrew/modelmux.rb "$TAP_DIR/Formula/modelmux.rb"
   brew install --build-from-source yarenty/tap/modelmux
   brew test modelmux
   ```
   **Fish:** use `set TAP_DIR (brew --repository yarenty/tap)`.

See [docs/RELEASING.md](../../docs/RELEASING.md) section 7 for full details.

### Updating Formula

When releasing a new version:

1. Update version in `modelmux.rb`:
   - `url` field (GitHub release URL)
   - `sha256` field (from `shasum -a 256 <tarball>`)

2. Test locally before submitting

3. Copy to Homebrew tap or homebrew-core

See [../../docs/RELEASING.md](../../docs/RELEASING.md) for detailed release instructions.

## Formula Location

- **Development**: `packaging/homebrew/modelmux.rb` (this file)
- **Homebrew Tap**: `homebrew-tap/Formula/modelmux.rb` (after first release)
- **homebrew-core**: `homebrew-core/Formula/modelmux.rb` (if accepted to core)

## See Also

- [docs/RELEASING.md](../../docs/RELEASING.md) - Release process
