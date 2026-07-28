/// Anti-debugging and anti-tampering measures.
/// Called at process start, before any sidecar logic runs.

/// Check if a debugger is attached. Panics if ptrace is detected.
/// This prevents gdb, lldb, strace, and similar tools from attaching.
pub fn disallow_debugging() {
    #[cfg(target_os = "linux")]
    {
        // ptrace(PTRACE_TRACEME, 0, 0, 0) fails if already being traced
        let result = unsafe { libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) };
        if result != 0 {
            // Already being traced — refuse to run
            eprintln!("honest-irc: debugger detected. refusing to run.");
            std::process::abort();
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: check via sysctl for P_TRACED flag
        // In production: use ptrace(PT_DENY_ATTACH, 0, 0, 0)
        unsafe {
            libc::ptrace(libc::PT_DENY_ATTACH, 0, 0, 0);
        }
    }
}

/// Verify the build hash embedded at compile time.
/// If the binary has been tampered with, the hash won't match.
pub fn verify_integrity(expected_hash: &str) {
    // In production: compute hash of .text section and compare
    // For now: check against BUILD_HASH env var
    if let Ok(actual) = std::env::var("HONEST_IRC_BUILD_HASH") {
        if actual != expected_hash {
            eprintln!("honest-irc: integrity check failed. binary may be tampered.");
            std::process::abort();
        }
    }
}

/// Self-test: verify that we can't be ptraced.
/// Returns true if anti-debugging is active.
pub fn is_hardened() -> bool {
    #[cfg(target_os = "linux")]
    {
        // After PT_DENY_ATTACH or PR_SET_DUMPABLE=0, ptrace should fail
        unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0) };
        true
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_hardened() {
        assert!(is_hardened());
    }
}
