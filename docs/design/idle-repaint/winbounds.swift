// winbounds.swift PID: the on-screen bounds of PID's windows (CGWindowList: no permission needed),
// and the system pointer, in the same top-left screen points.
import CoreGraphics
import AppKit
let pid = Int32(CommandLine.arguments[1]) ?? 0
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
for w in list where (w[kCGWindowOwnerPID as String] as? Int32) == pid {
    if let b = w[kCGWindowBounds as String] as? [String: Double] {
        print("window", Int(b["X"] ?? 0), Int(b["Y"] ?? 0), Int(b["Width"] ?? 0), Int(b["Height"] ?? 0))
    }
}
let p = NSEvent.mouseLocation
let h = NSScreen.screens.first?.frame.height ?? 0
print("pointer", Int(p.x), Int(h - p.y))
