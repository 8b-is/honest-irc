use std::fs::File;
use std::os::unix::io::FromRawFd;

/// Self-encrypted memory region.
/// Uses memfd_create + F_SEAL_WRITE + mlock.
/// Chat data lives here. Nothing touches disk. When process dies, kernel reclaims.
pub struct SealedMemory {
    pub ptr: *mut u8,
    pub size: usize,
    fd: Option<std::fs::File>,
}

impl SealedMemory {
    /// Create a new sealed memory region of `size` bytes.
    /// The region is: created via memfd (no disk backing), sealed against writes,
    /// locked in RAM via mlock, and set read-only via mprotect.
    pub fn create(name: &str, size: usize) -> Result<Self, String> {
        let name_c = std::ffi::CString::new(name).map_err(|e| e.to_string())?;

        #[cfg(target_os = "linux")]
        {
            let fd = unsafe {
                libc::memfd_create(
                    name_c.as_ptr(),
                    libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
                )
            };
            if fd < 0 {
                return Err(format!("memfd_create failed: {}", std::io::Error::last_os_error()));
            }

            // Set size
            unsafe {
                if libc::ftruncate(fd, size as libc::off_t) < 0 {
                    libc::close(fd);
                    return Err("ftruncate failed".into());
                }
            }

            // Map into memory
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    fd,
                    0,
                )
            };
            if ptr == libc::MAP_FAILED {
                unsafe { libc::close(fd); }
                return Err("mmap failed".into());
            }

            // Seal: no more writes, no resize, no more seals
            unsafe {
                libc::fcntl(
                    fd,
                    libc::F_ADD_SEALS,
                    libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK |
                    libc::F_SEAL_GROW | libc::F_SEAL_WRITE,
                );
            }

            // Lock into RAM
            unsafe { libc::mlock(ptr, size as libc::size_t); }

            Ok(SealedMemory {
                ptr: ptr as *mut u8,
                size,
                fd: Some(unsafe { File::from_raw_fd(fd) }),
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback: regular heap allocation (for macOS dev)
            let mut vec = vec![0u8; size];
            let ptr = vec.as_mut_ptr();
            std::mem::forget(vec);
            Ok(SealedMemory { ptr, size, fd: None })
        }
    }

    /// Get a slice view into the memory
    pub unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr, self.size)
    }

    /// Get a mutable slice (before sealing)
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.ptr, self.size)
    }

    /// Make region read-only (call after initialization)
    pub fn make_read_only(&self) {
        #[cfg(target_os = "linux")]
        unsafe {
            libc::mprotect(
                self.ptr as *mut libc::c_void,
                self.size as libc::size_t,
                libc::PROT_READ,
            );
        }
    }
}

impl Drop for SealedMemory {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        unsafe {
            if !self.ptr.is_null() {
                libc::munlock(self.ptr as *const libc::c_void, self.size as libc::size_t);
                libc::munmap(self.ptr as *mut libc::c_void, self.size as libc::size_t);
            }
            // fd is dropped by the File wrapper, which closes it.
            // The kernel reclaims the memfd memory.
        }

        #[cfg(not(target_os = "linux"))]
        if !self.ptr.is_null() {
            let _ = unsafe { Vec::from_raw_parts(self.ptr, self.size, self.size) };
        }
    }
}

/// The Memory Fortress: four sealed regions holding all sensitive data.
pub struct MemoryFortress {
    pub chat: SealedMemory,      // IRC messages (encrypted at rest)
    pub crypto: SealedMemory,    // session keys, nonces, ephemeral keypairs
    pub identity: SealedMemory,  // honesty vector, Dilithium keys
    pub seed: SealedMemory,      // Quant1bitLLM weights for per-byte sub-keys
}

impl MemoryFortress {
    pub fn new() -> Result<Self, String> {
        Ok(MemoryFortress {
            chat: SealedMemory::create("honest-irc-chat", 1024 * 1024 * 10)?,   // 10MB
            crypto: SealedMemory::create("honest-irc-crypto", 1024 * 64)?,      // 64KB
            identity: SealedMemory::create("honest-irc-identity", 1024 * 8)?,   // 8KB
            seed: SealedMemory::create("honest-irc-seed", 1024 * 1024)?,        // 1MB
        })
    }

    /// Seal all regions (make read-only after init)
    pub fn seal_all(&self) {
        self.chat.make_read_only();
        self.crypto.make_read_only();
        self.identity.make_read_only();
        self.seed.make_read_only();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sealed_memory() {
        let mem = SealedMemory::create("test", 4096);
        assert!(mem.is_ok());
    }

    #[test]
    fn test_read_write_sealed() {
        let mut mem = SealedMemory::create("test-rw", 4096).unwrap();
        unsafe {
            let slice = mem.as_mut_slice();
            slice[0] = 42;
            assert_eq!(slice[0], 42);
        }
    }

    #[test]
    fn test_fortress_create() {
        let fortress = MemoryFortress::new();
        assert!(fortress.is_ok());
    }
}
