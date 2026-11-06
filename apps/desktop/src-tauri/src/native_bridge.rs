unsafe extern "C" {
    fn aether_macos_major_version() -> i32;
    fn aether_accessibility_trust_status() -> i32;
}

pub fn accessibility_trust_status() -> Result<bool, &'static str> {
    // SAFETY: The linked Swift C entry point takes no arguments and returns Int32.
    // It only queries this process; it does not request trust or transfer ownership.
    match unsafe { aether_accessibility_trust_status() } {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err("Unexpected accessibility trust status"),
    }
}

pub fn macos_major_version() -> i32 {
    // SAFETY: The linked Swift C entry point takes no arguments and returns Int32.
    // It reads ProcessInfo only; no pointers, ownership transfer, or callbacks cross the ABI.
    unsafe { aether_macos_major_version() }
}
