unsafe extern "C" {
    fn aether_macos_major_version() -> i32;
}

pub fn macos_major_version() -> i32 {
    // SAFETY: The linked Swift C entry point takes no arguments and returns Int32.
    // It reads ProcessInfo only; no pointers, ownership transfer, or callbacks cross the ABI.
    unsafe { aether_macos_major_version() }
}
