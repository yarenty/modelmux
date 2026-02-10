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

```bash
# Install from local formula
brew install --build-from-source packaging/homebrew/modelmux.rb

# Test installed binary
brew test modelmux
```

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
- [HOMEBREW_READINESS.md](../../HOMEBREW_READINESS.md) - Readiness checklist
