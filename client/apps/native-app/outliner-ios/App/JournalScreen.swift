import PetaBorder
import PetaSync
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

    /// The file of the assets folder that is shown over the screen.
    @State private var file: URL?
    /// Whether the search field has the keyboard.
    @FocusState private var searches: Bool
    /// How much of the page the keyboard covers.
    @State private var keyboard: CGFloat = 0

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
        let edited = searching ? nil : view.outline.rows.first(where: \.isEditing)?.item.id
        // What the keyboard and the tools cover of the page
        let covered = keyboard + (edited == nil ? 0 : Tools.height)
        return ZStack(alignment: .bottom) {
            page(view: view, searching: searching, waiting: waiting, covered: covered)
                // The page keeps its height when the keyboard comes up, and what the keyboard
                // covers is told to it as room at its bottom. A page that got lower would
                // drop the item under the keyboard from the screen, with its field, just
                // when that field takes the keyboard
                .ignoresSafeArea(.keyboard, edges: .bottom)
            // Above the keyboard
            if let edited { Tools(itemId: edited, shell: shell) }
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
            // The button for yes names what a question about a loss does
            if question.destructive {
                Button("Delete", role: .destructive) { shell.answer(yes: true) }
            } else {
                Button("OK") { shell.answer(yes: true) }
            }
            Button("Cancel", role: .cancel) { shell.answer(yes: false) }
        }
        .quickLookPreview($file)
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
                // The field also tells of a text that the core gave it, as the empty one after
                // a result was chosen: that is no typing, and must not search again
                TextField("Search", text: Binding(get: { shell.view.search.query }, set: { if $0 != shell.view.search.query { shell.search($0) } }))
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
                        if let missing = shell.missing { MissingPage(name: missing, onMake: shell.make) }
                        if let missing = shell.missingFile {
                            Text("No file yet: \(missing)").padding(.horizontal, 12).padding(.vertical, 4)
                        }
                        Outline(shell: shell, covered: covered, onFile: { file = $0 })
                    }
                }
                // Over the page, so that it takes no room. Not while the dialog says the same
                if waiting == nil, let cloud { SyncCallout(state: cloud, onSync: onSync) }
            }
            .frame(maxHeight: .infinity)
            // The results, the page and the callout stay clear of the keyboard and the tools
            .safeAreaInset(edge: .bottom, spacing: 0) { Color.clear.frame(height: covered) }
        }
    }
}

/// How far a sync that moves files is, in words and as a part of the whole, or nil.
private func progress(_ state: SyncState?) -> (String, Double)? {
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
private struct Waiting: View {
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
/// why the last one failed. It lies over the bottom of the page, and takes no room there.
private struct SyncCallout: View {
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
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .background(Color(.label).opacity(0.9), in: RoundedRectangle(cornerRadius: 6))
            .shadow(radius: 4)
            .padding(12)
        }
    }

    private var words: String? {
        switch state {
        case .downloading, .uploading: return progress(state)?.0 ?? "Syncing…"
        case .failed(let reason): return "Sync failed: \(reason)"
        case .signedOut, .idle: return nil
        }
    }
}

/// The menu at the left of the bar, with what is done to the sync. There is none before the
/// sign-in: the button for the sign-in is on the screen then.
private struct SyncMenu: View {
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

/// A link named a page that is not there. The page is made only when the user asks for it
/// here (ADR 031).
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

private struct MissingPage: View {
    let name: String
    let onMake: () -> Void

    var body: some View {
        HStack {
            Text("No page yet: \(name)").frame(maxWidth: .infinity, alignment: .leading)
            Button("Make it", action: onMake)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
    }
}

private struct Results: View {
    let entries: [SearchEntry]
    let onPick: (SearchEntry) -> Void

    var body: some View {
        List(entries, id: \.position) { entry in
            Button {
                onPick(entry)
            } label: {
                VStack(alignment: .leading, spacing: 2) {
                    Text(label(entry)).font(.headline)
                    if !entry.snippet.isEmpty { Text(entry.snippet).font(.subheadline).lineLimit(2) }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .foregroundStyle(.primary)
        }
        .listStyle(.plain)
    }

    /// What a search result shows as its first line. Two kinds of result make a page.
    private func label(_ entry: SearchEntry) -> String {
        switch entry.kind {
        case .today: return "Open Today"
        case .create: return "New page: \(entry.title)"
        case .page, .item: return entry.title
        }
    }
}

/// What changes the item that is edited, in a row above the keyboard (ADR 031). A phone has
/// no Tab key and no key with Alt. A touch on a tool leaves the cursor in the field.
private struct Tools: View {
    let itemId: String
    let shell: Shell

    /// How high the row of the tools is.
    static let height: CGFloat = 44

    var body: some View {
        HStack(spacing: 0) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 0) {
                    tool("Outdent", "decrease.indent") { shell.outdent(itemId) }
                    tool("Indent", "increase.indent") { shell.indent(itemId) }
                    tool("Move up", "arrow.up") { shell.moveUp(itemId) }
                    tool("Move down", "arrow.down") { shell.moveDown(itemId) }
                    tool("Task", "checkmark.square") { shell.cycleTask(itemId) }
                    tool("Delete item", "xmark") { shell.delete(itemId) }.foregroundStyle(.red)
                }
            }
            // The field lets go of the keyboard, and tells the core that the edit has ended
            Button("Done") {
                // First: a field that only loses the keyboard may be one that the list took
                // off the screen, and its edit goes on
                shell.finish(itemId)
                UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
            }
            .padding(.horizontal, 12)
        }
        .background(Color(.secondarySystemBackground))
    }

    /// A tool with a drawing for its face. `name` says what it does, for a screen reader.
    private func tool(_ name: String, _ face: String, _ does: @escaping () -> Void) -> some View {
        Button(action: does) {
            Image(systemName: face).font(.title3).frame(width: 48, height: 44)
        }
        .accessibilityLabel(name)
    }
}

private struct Outline: View {
    @ObservedObject var shell: Shell
    /// What the keyboard and the tools cover at the bottom of the page.
    let covered: CGFloat
    let onFile: (URL) -> Void

    var body: some View {
        let view = shell.view
        let focus = view.outline.focusRequest
        ScrollViewReader { list in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    Text(title(view)).font(.title2).padding(.vertical, 8)
                    ForEach(view.outline.rows, id: \.item.id) { row in
                        OutlineRow(row: row, tagsDir: view.config.tagsDir, shell: shell, onFile: onFile)
                            .id(row.item.id)
                    }
                }
                .padding(.horizontal, 12)
                // Room for the callout of the sync under the last item
                .padding(.bottom, 60)
            }
            // A drag of the page puts the keyboard away, and so ends the edit
            .scrollDismissesKeyboard(.interactively)
            // The item that the core moved the edit to may be off the screen, as a new item
            // at the end of a long page
            .onChange(of: focus?.seq) {
                if let focus { list.scrollTo(focus.itemId) }
            }
            // The keyboard and the tools cover the bottom of the page. The item that is
            // edited may be under them then, and it is brought to what is seen
            .onChange(of: covered) {
                guard let edited = shell.view.outline.rows.first(where: \.isEditing)?.item.id else { return }
                withAnimation { list.scrollTo(edited) }
            }
        }
    }

    private func title(_ view: ViewModel) -> String {
        switch view.screen {
        case .page(let page): return page?.title ?? ""
        case .journal(let journal): return journal.displayDate
        default: return ""
        }
    }
}
