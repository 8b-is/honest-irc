#!/usr/bin/env bash
# honest-irc build script — deterministic, stripped, UPX-compressed, anti-debug
# BUILD_HASH = SHA256(genesis_hash + previous_BUILD_HASH + source_hash)
set -euo pipefail

GENESIS="7c242080f5f821e5eaf563fe2208d60632c451687baf65f4fe8e4a0d226e3ecf"
PREV_HASH="${1:-$GENESIS}"  # first build uses genesis as previous
SOURCE_HASH=$(find src Cargo.toml Cargo.lock -type f -exec sha256sum {} \; | sort | sha256sum | cut -d' ' -f1)

# Deterministic build hash chain
BUILD_HASH=$(echo -n "${GENESIS}${PREV_HASH}${SOURCE_HASH}" | sha256sum | cut -d' ' -f1)
echo "BUILD_HASH: $BUILD_HASH"
echo "PREV:       $PREV_HASH"
echo "SOURCE:     $SOURCE_HASH"

# Build with hardened flags
export RUSTFLAGS="\
  -C link-arg=-Wl,-z,relro \
  -C link-arg=-Wl,-z,now \
  -C link-arg=-Wl,-z,noexecstack \
  -C link-arg=-Wl,-z,separate-code \
  -C link-arg=-Wl,--build-id=sha1 \
  -C link-arg=-Wl,--strip-all \
  -C debuginfo=0 \
  -C strip=symbols \
  -C opt-level=3 \
  -C lto=fat \
  -C codegen-units=1 \
  -C panic=abort"

cargo build --release

# Strip all symbols
strip target/release/honest-irc
strip target/release/honest-vpn
strip target/release/honest-crypt
strip target/release/honest-mesh
strip target/release/honest-ircd

# UPX compress (if available)
if command -v upx &>/dev/null; then
  for bin in honest-irc honest-vpn honest-crypt honest-mesh honest-ircd; do
    upx --best --lzma target/release/$bin -o target/release/$bin-upx
    mv target/release/$bin-upx target/release/$bin
  done
  echo "[OK] UPX compressed"
fi

# Generate build manifest
cat > target/release/BUILD_MANIFEST << MANIFEST
BUILD_HASH=$BUILD_HASH
PREV_HASH=$PREV_HASH
GENESIS=$GENESIS
SOURCE_HASH=$SOURCE_HASH
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
BINARIES: honest-irc honest-vpn honest-crypt honest-mesh honest-ircd
MANIFEST

# Verify binary integrity
for bin in honest-irc honest-vpn honest-crypt honest-mesh honest-ircd; do
  BIN_HASH=$(sha256sum target/release/$bin | cut -d' ' -f1)
  echo "BIN_HASH($bin)=$BIN_HASH" >> target/release/BUILD_MANIFEST
done

echo ""
echo "=== BUILD COMPLETE ==="
echo "Build hash: $BUILD_HASH"
echo "Manifest:   target/release/BUILD_MANIFEST"
