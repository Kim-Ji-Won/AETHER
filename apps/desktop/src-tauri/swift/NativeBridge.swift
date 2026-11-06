import Foundation

@_cdecl("aether_macos_major_version")
public func aetherMacOSMajorVersion() -> Int32 {
    Int32(ProcessInfo.processInfo.operatingSystemVersion.majorVersion)
}
