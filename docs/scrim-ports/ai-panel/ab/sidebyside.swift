// sidebyside.swift DESIGN.png APP.png OUT.png "title": the design's frame and the app's capture next
// to each other on a neutral sheet, each labelled, at 1x, tops aligned.
import AppKit
let a = CommandLine.arguments
guard a.count >= 5, let design = NSImage(contentsOfFile: a[1]), let app = NSImage(contentsOfFile: a[2]) else { exit(1) }
func px(_ i: NSImage) -> NSSize {
    guard let r = i.representations.first else { return i.size }
    return NSSize(width: r.pixelsWide, height: r.pixelsHigh)
}
let (d, p) = (px(design), px(app))
let gap: CGFloat = 32, top: CGFloat = 36, pad: CGFloat = 24
let w = pad + d.width + gap + p.width + pad
let h = top + max(d.height, p.height) + pad
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(w), pixelsHigh: Int(h), bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
rep.size = NSSize(width: w, height: h)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
NSColor(calibratedWhite: 0.55, alpha: 1).setFill()
NSRect(x: 0, y: 0, width: w, height: h).fill()
let attrs: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: 13, weight: .semibold), .foregroundColor: NSColor.white]
("Design · " + a[4] as NSString).draw(at: NSPoint(x: pad, y: h - 26), withAttributes: attrs)
("Scrim (real app, scratch settings)" as NSString).draw(at: NSPoint(x: pad + d.width + gap, y: h - 26), withAttributes: attrs)
design.draw(in: NSRect(x: pad, y: h - top - d.height, width: d.width, height: d.height))
app.draw(in: NSRect(x: pad + d.width + gap, y: h - top - p.height, width: p.width, height: p.height))
NSGraphicsContext.restoreGraphicsState()
try? rep.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: a[3]))
