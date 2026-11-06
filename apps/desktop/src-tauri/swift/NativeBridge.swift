import Foundation
import AppKit
import ApplicationServices

@_cdecl("aether_macos_major_version")
public func aetherMacOSMajorVersion() -> Int32 {
    Int32(ProcessInfo.processInfo.operatingSystemVersion.majorVersion)
}

@_cdecl("aether_accessibility_trust_status")
public func aetherAccessibilityTrustStatus() -> Int32 {
    AXIsProcessTrusted() ? 1 : 0
}

// A native sheet owns its wording and decision. No WebView-provided approval value is accepted.
@_cdecl("aether_request_counter_approval")
public func aetherRequestCounterApproval(
    _ pointer: UnsafeMutableRawPointer,
    _ requestID: UInt64,
    _ callback: @escaping @convention(c) (UInt64, Int32) -> Void
) {
    let parent = Unmanaged<NSWindow>.fromOpaque(pointer).takeUnretainedValue()
    guard parent.isVisible, parent.attachedSheet == nil else {
        callback(requestID, -1)
        return
    }
    let alert = NSAlert()
    alert.messageText = "이번 실행의 AETHER 실험 카운터를 1 올릴까요?"
    let cancel = alert.addButton(withTitle: "취소")
    cancel.keyEquivalent = "\r"
    let approve = alert.addButton(withTitle: "이번 한 번 승인")
    approve.keyEquivalent = ""
    alert.window.defaultButtonCell = cancel.cell as? NSButtonCell
    var finished = false
    var closingObserver: NSObjectProtocol?
    var escapeMonitor: Any?
    func finish(_ result: Int32) {
        guard !finished else { return }
        finished = true
        if let monitor = escapeMonitor { NSEvent.removeMonitor(monitor) }
        escapeMonitor = nil
        if let observer = closingObserver { NotificationCenter.default.removeObserver(observer) }
        closingObserver = nil
        callback(requestID, result)
    }
    closingObserver = NotificationCenter.default.addObserver(
        forName: NSWindow.willCloseNotification, object: parent, queue: .main
    ) { _ in
        finish(-1)
        parent.endSheet(alert.window, returnCode: .cancel)
    }
    escapeMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
        if event.keyCode == 53, event.window === alert.window {
            parent.endSheet(alert.window, returnCode: .cancel)
            return nil
        }
        return event
    }
    alert.beginSheetModal(for: parent) { response in
        finish(response == .alertSecondButtonReturn ? 1 : 0)
    }
}
