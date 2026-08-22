import Foundation
import Vision

guard CommandLine.arguments.count >= 2 else {
    fputs("missing image path\n", stderr)
    exit(2)
}

let path = CommandLine.arguments[1]
let url = URL(fileURLWithPath: path)

let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.usesLanguageCorrection = true
if #available(macOS 11.0, *) {
    request.recognitionLanguages = ["zh-Hans", "en-US", "zh-Hant", "ja-JP", "ko-KR"]
}

let handler = VNImageRequestHandler(url: url, options: [:])
do {
    try handler.perform([request])
} catch {
    fputs("vision failed: \(error)\n", stderr)
    exit(3)
}

guard let observations = request.results, !observations.isEmpty else {
    exit(4)
}

var lines: [String] = []
for obs in observations {
    if let s = obs.topCandidates(1).first?.string.trimmingCharacters(in: .whitespacesAndNewlines),
       !s.isEmpty {
        lines.append(s)
    }
}

let text = lines.joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
if text.isEmpty {
    exit(4)
}
print(text)
