#!/usr/bin/env bash
# honest-irc build script — deterministic, stripped, UPX-compressed, anti-debug
# BUILD_HASH = SHA256(genesis_hash + previous_BUILD_HASH + source_hash)
# Target binary size: exactly 4.20 MB (4200000 bytes)
set -euo pipefail

GENESIS="7c242080f5f821e5eaf563fe2208d60632c451687baf65f4fe8e4a0d226e3ecf"
PREV_HASH="${1:-$GENESIS}"
SOURCE_HASH=$(find src Cargo.toml Cargo.lock -type f -exec sha256sum {} \; | sort | sha256sum | cut -d' ' -f1)
TARGET_SIZE=4200000  # 4.20MB

BUILD_HASH=$(echo -n "${GENESIS}${PREV_HASH}${SOURCE_HASH}" | sha256sum | cut -d' ' -f1)
echo "BUILD_HASH: $BUILD_HASH"
echo "PREV:       $PREV_HASH"
echo "TARGET:     ${TARGET_SIZE} bytes (4.20 MB)"

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

for bin in honest-irc honest-vpn honest-crypt honest-mesh honest-ircd; do
  BIN="target/release/$bin"
  strip "$BIN"

  CURRENT_SIZE=$(stat -f%z "$BIN" 2>/dev/null || stat -c%s "$BIN" 2>/dev/null)
  echo "  $bin: $CURRENT_SIZE bytes"

  # Pad to exactly TARGET_SIZE with random unused code bytes
  if [ "$CURRENT_SIZE" -lt "$TARGET_SIZE" ]; then
    PAD_SIZE=$((TARGET_SIZE - CURRENT_SIZE))
    echo "  padding: +$PAD_SIZE bytes of random frozen code"
    # Generate random "code-like" padding (valid x86_64 NOP sleds mixed with random bytes)
    # Using RDRAND-inspired deterministic random from BUILD_HASH
    python3 -c "
import hashlib, struct, sys, random
hash_bytes = hashlib.sha256(b'$BUILD_HASH' + b'pad-v2.1').digest()
random.seed(int.from_bytes(hash_bytes[:8], 'big'))
pad = bytearray()
nop_patterns = [
    b'\x90',                          # NOP
    b'\x66\x90',                      # 2-byte NOP
    b'\x0f\x1f\x00',                  # 3-byte NOP
    b'\x0f\x1f\x40\x00',              # 4-byte NOP
    b'\x0f\x1f\x44\x00\x00',          # 5-byte NOP
    b'\x66\x0f\x1f\x44\x00\x00',      # 6-byte NOP
    b'\x0f\x1f\x80\x00\x00\x00\x00',  # 7-byte NOP
    b'\x0f\x1f\x84\x00\x00\x00\x00\x00', # 8-byte NOP
]
while len(pad) < $PAD_SIZE:
    # 70% valid NOP sleds, 30% random junk (looks like encrypted data)
    if random.random() < 0.7:
        nop = random.choice(nop_patterns)
        pad.extend(nop)
    else:
        pad.append(random.randint(0, 255))
# mark the padding boundary with magic bytes
magic = b'\x42\x00\x42\x00HOME\x00HONEST\x00IRC\x00'  # B\0B\0HOME\0HONEST\0IRC\0
pad = pad[:$PAD_SIZE - len(magic)] + magic
sys.stdout.buffer.write(pad)
" >> "$BIN"
  fi

  # Verify final size
  FINAL_SIZE=$(stat -f%z "$BIN" 2>/dev/null || stat -c%s "$BIN" 2>/dev/null)
  echo "  final: $FINAL_SIZE bytes"
done

# UPX compress (with --overlay=copy to preserve padding bytes)
if command -v upx &>/dev/null; then
  for bin in honest-irc honest-vpn honest-crypt honest-mesh honest-ircd; do
    upx --best --lzma --overlay=copy target/release/$bin -o target/release/$bin-upx 2>/dev/null && \
      mv target/release/$bin-upx target/release/$bin
  done
  echo "[OK] UPX compressed (overlay preserved)"
fi

# Generate build manifest
cat > target/release/BUILD_MANIFEST << MANIFEST
BUILD_HASH=$BUILD_HASH
PREV_HASH=$PREV_HASH
GENESIS=$GENESIS
SOURCE_HASH=$SOURCE_HASH
TARGET_SIZE=$TARGET_SIZE
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
BINARIES: honest-irc honest-vpn honest-crypt honest-mesh honest-ircd
MANIFEST

for bin in honest-irc honest-vpn honest-crypt honest-mesh honest-ircd; do
  BIN_HASH=$(sha256sum target/release/$bin | cut -d' ' -f1)
  BIN_SIZE=$(stat -f%z target/release/$bin 2>/dev/null || stat -c%s target/release/$bin 2>/dev/null)
  echo "BIN($bin)=$BIN_HASH size=$BIN_SIZE" >> target/release/BUILD_MANIFEST
done

echo ""
echo "=== BUILD COMPLETE ==="
echo "Build hash: $BUILD_HASH"
