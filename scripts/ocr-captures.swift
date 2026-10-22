// Read the text in captures, to check evidence for personal data before it's committed (R1,
// docs/design/pii-scan.md). macOS only (Apple's Vision framework); nothing leaves the machine.
//
//   swift scripts/ocr-captures.swift docs/scrim-ports/chat/*.png > /tmp/ocr.tsv
//
// Prints one "path<TAB>line of text" row per line of text found, then search it, for example for
// a home folder: grep -E '/Users/|/home/' /tmp/ocr.tsv
import AppKit
import Foundation
import Vision

for path in CommandLine.arguments.dropFirst() {
    guard let image = NSImage(contentsOfFile: path),
          let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
        print("\(path)\t<unreadable>")
        continue
    }
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = false
    try? VNImageRequestHandler(cgImage: cg).perform([request])
    for o in request.results ?? [] {
        if let t = o.topCandidates(1).first {
            print("\(path)\t\(t.string)")
        }
    }
}
