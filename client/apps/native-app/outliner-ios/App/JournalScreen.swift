import PetaBorder
import PetaTypes
import QuickLook
import SwiftUI

/// The one screen of the app: a bar with the menu, the search field and the button for today,
/// and under it the results of the search or the page that is open. `cloud` is where the sync
/// with the cloud stands, and `nil` in an app that syncs nothing. `syncInFront` says that the
/// user waits for the sync.
struct JournalScreen: View {
    @ObservedObject var shell: Shell
    var cloud: SyncState?
    var syncInFront = false
    var onSignIn: () -> Void = {}
    var onSync: () -> Void = {}
    var onSignOut: () -> Void = {}

    /// Whether the search field has the keyboard.
    @FocusState private var searches: Bool
    /// How much of the page the keyboard covers.
    @State private var keyboard: CGFloat = 0
    /// Whether the tools are full or shrunk to a button, whatever is edited or selected. Full
    /// at first.
    @State private var fullTools = true

    var body: some View {
        let view = shell.view
        let searching = view.search.showDropdown
        let waiting = syncInFront ? progress(cloud) : nil
        // A new app before the sign-in: the bar would make pages before the first download
        if shell.unopened, cloud == .signedOut {
            Button("Sign in to Dropbox", action: onSignIn)
                .buttonStyle(.borderedProminent)
                .padding(8)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        } else {
            whole(view: view, searching: searching, waiting: waiting)
        }
    }

    private func whole(view: ViewModel, searching: Bool, waiting: (String, Double)?) -> some View {
        let rows = view.outline.rows
        // The tools act on the item that is edited, or on the first that is selected
        let target = searching ? nil : rows.first(where: \.isEditing) ?? rows.first(where: \.isSelected)
        // What the keyboard and the tools cover of the page
        let covered = keyboard + (target != nil && fullTools ? Tools.height : 0)
        return ZStack(alignment: .bottom) {
            page(view: view, searching: searching, waiting: waiting, covered: covered)
                // The page keeps its height when the keyboard comes up, and what the keyboard
                // covers is told to it as room at its bottom. A page that got lower would
                // drop the item under the keyboard from the screen, with its field, just
                // when that field takes the keyboard
                .ignoresSafeArea(.keyboard, edges: .bottom)
            // Above the keyboard
            if let target {
                if fullTools {
                    Tools(target: target, shell: shell) { fullTools = false }
                } else {
                    ShrunkTools { fullTools = true }.frame(maxWidth: .infinity, alignment: .trailing)
                }
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: UIResponder.keyboardWillChangeFrameNotification)) { told in
            guard let frame = told.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect else { return }
            keyboard = covers(frame)
        }
        .onReceive(NotificationCenter.default.publisher(for: UIResponder.keyboardWillHideNotification)) { _ in
            keyboard = 0
        }
        .background { KeyboardKeeping().frame(width: 0, height: 0) }
        .overlay { if let waiting { Waiting(words: waiting.0, part: waiting.1) } }
        .alert(
            shell.question?.text ?? "", isPresented: Binding(get: { shell.question != nil }, set: { _ in }),
            presenting: shell.question
        ) { question in
            if !question.waits {
                // A notice: there is nothing to choose
                Button("OK", role: .cancel) { shell.answer(yes: true) }
            } else {
                // The button for yes names what a question about a loss does
                if question.destructive {
                    Button("Delete", role: .destructive) { shell.answer(yes: true) }
                } else {
                    Button("OK") { shell.answer(yes: true) }
                }
                Button("Cancel", role: .cancel) { shell.answer(yes: false) }
            }
        }
        .quickLookPreview($shell.file)
    }

    /// How much of the page the keyboard covers, above the edge that the page keeps free at
    /// the bottom of the screen by itself.
    private func covers(_ keyboard: CGRect) -> CGFloat {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        guard let window = scenes.compactMap(\.keyWindow).first else { return 0 }
        return max(0, window.bounds.maxY - keyboard.minY - window.safeAreaInsets.bottom)
    }

    private func page(view: ViewModel, searching: Bool, waiting: (String, Double)?, covered: CGFloat) -> some View {
        VStack(spacing: 0) {
            // At the top, where it stays while an item is edited and the keyboard is up
            HStack(spacing: 4) {
                if let cloud { SyncMenu(state: cloud, onSignIn: onSignIn, onSync: onSync, onSignOut: onSignOut) }
                // Without a page in its direction a button stays, and cannot be touched: the
                // bar does not move (ADR 035)
                HistoryButton(name: "Back", symbol: "chevron.left", enabled: view.history.canGoBack, action: shell.back)
                HistoryButton(
                    name: "Forward", symbol: "chevron.right", enabled: view.history.canGoForward, action: shell.forward)
                // The field also tells of a text that the core gave it, as the empty one after
                // a result was chosen: that is no typing, and must not search again
                TextField(
                    "Search",
                    text: Binding(
                        get: { shell.view.search.query }, set: { if $0 != shell.view.search.query { shell.search($0) } }
                    )
                )
                .focused($searches)
                .textFieldStyle(.roundedBorder)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .accessibilityIdentifier("search")
                Button(action: shell.today) {
                    TodayIcon().frame(width: 24, height: 24).frame(width: 44, height: 44)
                }
                .accessibilityLabel("Today")
            }
            .padding(.horizontal, 8)
            // The room of the line is always there: the app syncs while the user types, and
            // the page must not jump
            ZStack {
                if shell.syncing { ProgressView().progressViewStyle(.linear) }
            }
            .frame(height: 4)
            if cloud == .signedOut {
                Button("Sign in to Dropbox", action: onSignIn)
                    .buttonStyle(.borderedProminent)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 4)
            }
            // The rest of the height
            ZStack(alignment: .bottom) {
                if searching {
                    Results(entries: view.search.results) { entry in
                        // The page takes the place of the results, and the keyboard goes
                        searches = false
                        shell.pick(entry)
                    }
                } else {
                    VStack(alignment: .leading, spacing: 0) {
                        if let missing = shell.missingFile {
                            Text("No file yet: \(missing)").padding(.horizontal, 12).padding(.vertical, 4)
                        }
                        Outline(shell: shell, covered: covered)
                    }
                }
                // Over the top of the page, so that it takes no room. Not at the bottom: the user
                // types on the last lines most of the time, and it hid them. Not in the middle:
                // it hid the line that is edited (#884). Not while the dialog says the same
                if waiting == nil, let cloud {
                    SyncCallout(state: cloud, onSync: onSync)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
                }
            }
            .frame(maxHeight: .infinity)
            // The results, the page and the callout stay clear of the keyboard and the tools
            .safeAreaInset(edge: .bottom, spacing: 0) { Color.clear.frame(height: covered) }
        }
    }
}

/// A button of the bar that goes back or forward among the pages that were open.
private struct HistoryButton: View {
    let name: String
    let symbol: String
    let enabled: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol).font(.body.weight(.semibold)).frame(width: 32, height: 44)
        }
        .disabled(!enabled)
        .accessibilityLabel(name)
    }
}

/// The icon of today of the web frontend: a book with a ribbon. SF Symbols has no such book,
/// so it is drawn here, with the lines of the Android shell (`TodayButton`).
private struct TodayIcon: View {
    var body: some View {
        Canvas { context, size in
            // The icon is drawn in a square of 24 units
            let unit = size.width / 24
            func line(_ fromX: CGFloat, _ fromY: CGFloat, _ toX: CGFloat, _ toY: CGFloat, width: CGFloat) {
                var path = Path()
                path.move(to: CGPoint(x: fromX * unit, y: fromY * unit))
                path.addLine(to: CGPoint(x: toX * unit, y: toY * unit))
                context.stroke(path, with: .foreground, lineWidth: width * unit)
            }
            let cover = CGRect(x: 3 * unit, y: 1 * unit, width: 16 * unit, height: 20 * unit)
            context.stroke(
                Path(roundedRect: cover, cornerRadius: 1.5 * unit), with: .foreground, lineWidth: 2 * unit)
            line(3, 13, 19, 13, width: 1)
            line(3, 17, 19, 17, width: 1)
            line(14, 2, 14, 20, width: 3)
            let ribbon = CGRect(x: 5 * unit, y: 21 * unit, width: 3 * unit, height: 2 * unit)
            context.fill(Path(ribbon), with: .foreground)
        }
    }
}

/// A link named a page that is not there. The page is made only when the user asks for it
/// here (ADR 031).
private struct Results: View {
    let entries: [SearchEntry]
    let onPick: (SearchEntry) -> Void

    var body: some View {
        List(entries, id: \.key) { entry in
            Button {
                onPick(entry)
            } label: {
                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.label).font(.headline)
                    if !entry.snippet.isEmpty { Text(entry.snippet).font(.subheadline).lineLimit(2) }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .foregroundStyle(.primary)
        }
        .listStyle(.plain)
    }
}
