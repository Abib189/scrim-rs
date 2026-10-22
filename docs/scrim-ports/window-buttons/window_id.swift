// window_id.swift PID — print "id x y w h" for each on-screen window of process PID, largest
// first. Used to capture one window with its own window buttons (`screencapture -l <id>`), never
// the whole screen.
import CoreGraphics
import Foundation

guard CommandLine.arguments.count > 1, let pid = Int32(CommandLine.arguments[1]) else { exit(2) }
let list = (CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]]) ?? []
var rows: [(Int, Double, Double, Double, Double)] = []
for w in list where (w[kCGWindowOwnerPID as String] as? Int32) == pid {
    guard let id = w[kCGWindowNumber as String] as? Int, let b = w[kCGWindowBounds as String] as? [String: Double] else { continue }
    rows.append((id, b["X"] ?? 0, b["Y"] ?? 0, b["Width"] ?? 0, b["Height"] ?? 0))
}
for r in rows.sorted(by: { $0.3 * $0.4 > $1.3 * $1.4 }) {
    print("\(r.0) \(Int(r.1)) \(Int(r.2)) \(Int(r.3)) \(Int(r.4))")
}
