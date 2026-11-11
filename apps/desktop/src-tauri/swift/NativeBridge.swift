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

// Main-thread-only registry; closures are removed on every completion path.
private var cancelApprovals: [UInt64: () -> Void] = [:]
@_cdecl("aether_cancel_counter_approval")
public func aetherCancelCounterApproval(_ requestID: UInt64) {
    cancelApprovals[requestID]?()
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
        cancelApprovals.removeValue(forKey: requestID)
        if let monitor = escapeMonitor { NSEvent.removeMonitor(monitor) }
        escapeMonitor = nil
        if let observer = closingObserver { NotificationCenter.default.removeObserver(observer) }
        closingObserver = nil
        fputs("AETHER_NATIVE_APPROVAL request_id=\(requestID) result=\(result) observer_removed=true monitor_removed=true\n", stderr)
        callback(requestID, result)
    }
    cancelApprovals[requestID] = {
        finish(-1)
        parent.endSheet(alert.window, returnCode: .cancel)
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

// Observe only this application's lifecycle; no other application's windows are read.
private var lifecycleObservers: [NSObjectProtocol] = []
@_cdecl("aether_install_lifecycle_observers")
public func aetherInstallLifecycleObservers(_ terminate: @escaping @convention(c) () -> Void) {
    guard lifecycleObservers.isEmpty else { return }
    lifecycleObservers.append(NotificationCenter.default.addObserver(forName: NSApplication.willTerminateNotification, object: NSApp, queue: .main) { _ in terminate() })
    for (name, label) in [
        (NSApplication.didHideNotification, "hidden"),
        (NSApplication.didUnhideNotification, "unhidden"),
        (NSWindow.didMiniaturizeNotification, "minimized"),
        (NSWindow.didDeminiaturizeNotification, "restored")
    ] {
        lifecycleObservers.append(NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { note in
            if let window = note.object as? NSWindow, window.title != "AETHER" { return }
            let stamp = Int64(Date().timeIntervalSince1970 * 1000)
            fputs("AETHER_NATIVE_LIFECYCLE event=\(label) pid=\(ProcessInfo.processInfo.processIdentifier) time_ms=\(stamp)\n", stderr)
        })
    }
}
