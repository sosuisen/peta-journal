import CryptoKit
import Foundation

private let block = 4 * 1024 * 1024

/// The content hash of Dropbox: the SHA-256 of the SHA-256 of each block of 4MB, as hex. It
/// tells whether a file here has the content of the file in the cloud, with no download.
/// `nil` when the file cannot be read.
func dropboxContentHash(of file: URL) -> String? {
    guard let input = try? FileHandle(forReadingFrom: file) else { return nil }
    defer { try? input.close() }
    var whole = SHA256()
    while let read = try? input.read(upToCount: block), !read.isEmpty {
        whole.update(data: Data(SHA256.hash(data: read)))
    }
    return whole.finalize().map { String(format: "%02x", $0) }.joined()
}
