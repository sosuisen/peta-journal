import Foundation
@_exported import PetaTypes

/// The link of a run as a URL, which a text of SwiftUI takes, and back again when it is touched.
/// The core says where a link leads (ADR 042): a web address stays as it is, and a page or a
/// file of the assets folder gets an address of the app, which nothing outside opens.
public enum RichLinks {
    private static let scheme = "peta-journal"

    /// The URL for a link. `nil` for a web address that is no URL.
    public static func url(of link: RichLink) -> URL? {
        switch link {
        case .web(let url): return URL(string: url)
        case .page(let url): return app("page", url)
        case .file(let path): return app("file", path)
        }
    }

    /// Where the link of this URL leads. `nil` for a URL that `url(of:)` did not make.
    public static func link(of url: URL) -> RichLink? {
        if url.scheme == "https" || url.scheme == "http" { return .web(url: url.absoluteString) }
        guard url.scheme == scheme, let to = URLComponents(url: url, resolvingAgainstBaseURL: false)?
            .queryItems?.first(where: { $0.name == "to" })?.value
        else { return nil }
        switch url.host {
        case "page": return .page(url: to)
        case "file": return .file(path: to)
        default: return nil
        }
    }

    /// An address of the app that holds `to` whole: a `#` or a space in it stays as it is.
    private static func app(_ kind: String, _ to: String) -> URL? {
        let held = to.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? to
        return URL(string: "\(scheme)://\(kind)?to=\(held)")
    }
}
