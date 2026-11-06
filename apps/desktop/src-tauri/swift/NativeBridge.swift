import Foundation
import ApplicationServices

@_cdecl("aether_macos_major_version")
public func aetherMacOSMajorVersion() -> Int32 {
    Int32(ProcessInfo.processInfo.operatingSystemVersion.majorVersion)
}

@_cdecl("aether_accessibility_trust_status")
public func aetherAccessibilityTrustStatus() -> Int32 {
    AXIsProcessTrusted() ? 1 : 0
}
