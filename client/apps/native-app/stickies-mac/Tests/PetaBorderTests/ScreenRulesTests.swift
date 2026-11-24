import PetaTypes
import XCTest

@testable import PetaScreens

/// What the screens show, decided from the view model alone.
final class ScreenRulesTests: XCTestCase {
    // ページを開いているとき、サーバーにつながっていなければそう出る。つなぎ始めは、つないでいる途中と出る。
    func testTheStatusSaysWhatIsWrongWithTheServer() {
        let page = "/pages/a"

        XCTAssertEqual(OutlineScreen.status(url: page, connection: .connected), "")
        XCTAssertEqual(OutlineScreen.status(url: page, connection: .connecting), "Connecting to the journal server…")
        XCTAssertEqual(
            OutlineScreen.status(url: page, connection: .disconnected), "No connection to the journal server")
        XCTAssertEqual(OutlineScreen.status(url: page, connection: .error), "No connection to the journal server")
    }

    // ページのない画面には、サーバーの状態は出ない。待つ項目がないためである。
    func testAScreenWithoutAPageShowsNoStatus() {
        XCTAssertEqual(OutlineScreen.status(url: nil, connection: .disconnected), "")
        XCTAssertEqual(OutlineScreen.status(url: nil, connection: .connecting), "")
    }

    private func search(_ query: String, results: [SearchEntry] = [], searching: Bool = false) -> SearchView {
        SearchView(query: query, results: results, showDropdown: true, selectedIndex: nil, searching: searching)
    }

    private func entry(title: String, snippet: String) -> SearchEntry {
        SearchEntry(position: 0, kind: .page, title: title, label: title, snippet: snippet)
    }

    // 入力した文字で探し終えて、何も見つからなかったときだけ、見つからないと出る。
    func testNothingFoundIsSaidOnlyWhenASearchForTypedTextFoundNothing() {
        XCTAssertTrue(search("zzz").foundNothing)
        XCTAssertFalse(search("zzz", searching: true).foundNothing)
        XCTAssertFalse(search("").foundNothing)
        XCTAssertFalse(search("apple", results: [entry(title: "apple-pie", snippet: "")]).foundNothing)
    }

    // 見つけた文の前後は、項目の名前の下に出る。名前で見つけたページでは、名前と同じなので出さない。
    func testTheSnippetIsShownUnlessItIsTheName() {
        XCTAssertEqual(entry(title: "apple-pie", snippet: "an apple a day").shownSnippet, "an apple a day")
        XCTAssertNil(entry(title: "apple-pie", snippet: "apple-pie").shownSnippet)
        XCTAssertNil(entry(title: "apple-pie", snippet: "").shownSnippet)
    }

    // 戻ると進むの矢印は、出ていて、行き先があるときだけ押せる。フォーカスのない付箋紙では、見えない矢印の場所を押しても戻らない。
    func testTheArrowsOfTheHistoryAreUsableOnlyWhenShownAndThereIsAPageToGoTo() {
        XCTAssertTrue(StickyTools.usable(shown: true, possible: true))
        XCTAssertFalse(StickyTools.usable(shown: false, possible: true))
        XCTAssertFalse(StickyTools.usable(shown: true, possible: false))
    }
}
