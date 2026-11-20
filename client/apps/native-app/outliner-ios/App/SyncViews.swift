import PetaBorder
import SwiftUI

/// How far a sync that moves files is, in words and as a part of the whole, or nil.
func progress(_ state: SyncState?) -> (String, Double)? {
    switch state {
    case .downloading(let done, let total) where total > 0:
        return ("Downloading \(done) of \(total)", Double(done) / Double(total))
    case .uploading(let done, let total) where total > 0:
        return ("Uploading \(done) of \(total)", Double(done) / Double(total))
    default:
        return nil
    }
}

/// Where the sync stands, over the page, while the user waits for it: the app came to the
/// front, and the pages may be old until the files have moved. The dialog cannot be closed:
/// an edit of an old page would be merged with the new one later.
struct Waiting: View {
    let words: String
    let part: Double

    var body: some View {
        ZStack {
            Color.black.opacity(0.3).ignoresSafeArea()
            VStack(alignment: .leading, spacing: 16) {
                Text(words)
                ProgressView(value: part)
            }
            .padding(24)
            .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 16))
            .padding(40)
        }
        .accessibilityAddTraits(.isModal)
    }
}

/// Where a sync stands that the user does not wait for, such as the one after an edit, and
/// why the last one failed. It lies over the top right of the page, and takes no room there.
struct SyncCallout: View {
    let state: SyncState
    let onSync: () -> Void

    var body: some View {
        if let words {
            HStack {
                Text(words).foregroundStyle(Color(.systemBackground))
                if case .failed = state {
                    Button("Try again", action: onSync).foregroundStyle(Color(.systemTeal))
                }
            }
            .padding(.leading, 16)
            .padding(.trailing, 16 + Callout.tail)
            .padding(.vertical, 12)
            .background(Color(.label).opacity(0.9), in: Callout())
            .shadow(radius: 4)
            .padding(12)
        }
    }

    private var words: String? {
        switch state {
        case .downloading, .uploading: return progress(state)?.0 ?? "Syncing…"
        // The words of the SDK went to the log: they are long, and mean nothing to the user
        case .failed(let offline): return offline ? "Sync failed: no connection" : "Sync failed"
        case .signedOut, .idle: return nil
        }
    }
}

/// A speech balloon with its tail on the right side, as the callout of the sync.
private struct Callout: Shape {
    /// How far the tail reaches out of the right side.
    static let tail: CGFloat = 8

    func path(in rect: CGRect) -> Path {
        let side = rect.maxX - Callout.tail
        var path = Path(
            roundedRect: CGRect(x: rect.minX, y: rect.minY, width: side - rect.minX, height: rect.height),
            cornerRadius: 6)
        path.move(to: CGPoint(x: side, y: rect.midY - Callout.tail))
        path.addLine(to: CGPoint(x: rect.maxX, y: rect.midY))
        path.addLine(to: CGPoint(x: side, y: rect.midY + Callout.tail))
        path.closeSubpath()
        return path
    }
}

/// The menu at the left of the bar, with what is done to the sync. There is none before the
/// sign-in: the button for the sign-in is on the screen then.
struct SyncMenu: View {
    let state: SyncState
    let onSignIn: () -> Void
    let onSync: () -> Void
    let onSignOut: () -> Void

    var body: some View {
        if state != .signedOut {
            Menu {
                // Nothing is asked of a sync that runs
                Button("Sync", action: onSync).disabled(state.isRunning)
                // A token that Dropbox no longer takes is replaced by a new sign-in
                if case .failed = state { Button("Sign in to Dropbox", action: onSignIn) }
                Button("Sign out", action: onSignOut).disabled(state.isRunning)
            } label: {
                Image(systemName: "line.3.horizontal").frame(width: 44, height: 44)
            }
            .accessibilityLabel("Menu")
        }
    }
}
