import Foundation

func parseISODate(_ text: String) -> Date? {
    let fractional = ISO8601DateFormatter()
    fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    if let date = fractional.date(from: text) {
        return date
    }

    let plain = ISO8601DateFormatter()
    plain.formatOptions = [.withInternetDateTime]
    return plain.date(from: text)
}

final class Shell: @unchecked Sendable {
    @discardableResult
    static func run(_ executable: String, _ arguments: [String] = []) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments

        let output = Pipe()
        let error = Pipe()
        process.standardOutput = output
        process.standardError = error

        try process.run()
        process.waitUntilExit()

        let stdout = String(data: output.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        let stderr = String(data: error.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""

        guard process.terminationStatus == 0 else {
            let message = stderr.trimmingCharacters(in: .whitespacesAndNewlines)
            throw NSError(domain: "CodexSwitcher.Shell", code: Int(process.terminationStatus), userInfo: [
                NSLocalizedDescriptionKey: message.isEmpty ? stdout : message
            ])
        }

        return stdout
    }
}
