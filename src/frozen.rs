/// Frozen random noise embedded at compile time.
/// These bytes exist only to pad the binary to exactly 4.20 MB.
/// They are valid x86_64 NOP sleds interspersed with random data.
/// UPX is configured with --overlay=copy to preserve them.
/// They serve no functional purpose — they are structural noise
/// that increases the binary's entropy and frustrates static analysis.

/// The number 420 in hex, repeated, as a structural marker.
#[allow(dead_code)]
const MAGIC: [u8; 20] = *b"\x42\x00\x42\x00HOME\0HONEST\0IRC\0";

/// Frozen NOP sleds. These are valid x86_64 instructions that do nothing.
/// They exist only to occupy space and confuse disassemblers.
#[allow(dead_code)]
const NOPS: &[u8] = &[
    0x90,                         // NOP
    0x66, 0x90,                   // 2-byte NOP
    0x0f, 0x1f, 0x00,             // 3-byte NOP
    0x0f, 0x1f, 0x40, 0x00,       // 4-byte NOP
    0x0f, 0x1f, 0x44, 0x00, 0x00, // 5-byte NOP
    0x66, 0x0f, 0x1f, 0x44, 0x00, 0x00, // 6-byte NOP
    0x0f, 0x1f, 0x80, 0x00, 0x00, 0x00, 0x00, // 7-byte NOP
    0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00, // 8-byte NOP
];

/// Filler function that exists solely to consume binary space.
/// The compiler cannot optimize it away because it uses volatile reads
/// and writes through raw pointers to stack variables.
#[allow(dead_code)]
#[inline(never)]
pub fn frozen_noise() -> usize {
    let mut acc: usize = 0;

    // Volatile reads from NOP sled — prevents dead code elimination
    for i in 0..NOPS.len() {
        let ptr: *const u8 = &NOPS[i];
        unsafe { std::ptr::read_volatile(ptr); }
        unsafe { std::ptr::read_volatile(ptr); }
        acc = acc.wrapping_add(NOPS[i] as usize);
    }

    // Volatile writes to a stack dummy — prevents optimization
    let mut dummy: [u8; 64] = [0; 64];
    for i in 0..dummy.len() {
        let ptr: *mut u8 = &mut dummy[i];
        unsafe { std::ptr::write_volatile(ptr, MAGIC[i % MAGIC.len()]); }
    }

    // Prevent the dummy from being optimized away
    let sum: usize = dummy.iter().fold(0usize, |a: usize, b: &u8| a.wrapping_add(*b as usize));
    acc ^= sum;

    acc
}

/// Returns the target binary size: 4.20 MB.
/// The build script pads each binary to exactly this size.
pub const fn target_size() -> usize { 4_200_000 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frozen_noise_is_deterministic() {
        let a = frozen_noise();
        let b = frozen_noise();
        assert_eq!(a, b);
    }

    #[test]
    fn test_magic_is_correct() {
        assert_eq!(&MAGIC[0..2], b"\x42\x00");
    }

    #[test]
    fn test_target_size() {
        assert_eq!(target_size(), 4_200_000);
    }
}
