use super::*;
use client_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, UiOp};
use client_core::model::{Page, PageWithItems};
use client_core::testing::plain_item;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

/// What the fake browser was asked for, and the answers it still owes.
#[derive(Default)]
struct Browser {
    calls: RefCell<VecDeque<(ApiOp, Done<ApiResult<ApiOutput>>)>>,
    asked: RefCell<Vec<(u32, UiOp)>>,
    renders: Cell<usize>,
}

/// A browser that answers nothing by itself: the test lets the server answer.
struct FakeHost(Rc<Browser>);

impl Host for FakeHost {
    type File = String;

    fn now(&self) -> (LocalDateTimeDto, u64) {
        let local = LocalDateTimeDto {
            year: 2026,
            month: 2,
            day: 10,
            hour: 9,
            minute: 0,
        };
        (local, 1_770_000_000_000)
    }

    fn call(&self, op: ApiOp, done: Done<ApiResult<ApiOutput>>) {
        self.0.calls.borrow_mut().push_back((op, done));
    }

    fn ask(&self, id: u32, op: &UiOp) {
        self.0.asked.borrow_mut().push((id, op.clone()));
    }

    fn rendered(&self) {
        self.0.renders.set(self.0.renders.get() + 1);
    }
}

/// The answer of the fake server.
fn answer_of(op: &ApiOp) -> ApiResult<ApiOutput> {
    match op {
        ApiOp::GetOrCreateDoc { .. } | ApiOp::GetPageById { .. } => {
            Ok(ApiOutput::Page(PageWithItems {
                page: Page {
                    id: "p".into(),
                    name: "notes".into(),
                    page_type: "doc".into(),
                    ..Default::default()
                },
                items: vec![
                    plain_item("a", 1, "the body of a"),
                    plain_item("b", 1, "the body of b"),
                ],
                created: false,
                transcluded_trees: None,
            }))
        }
        _ => Err(ApiError("the fake server does not know this".into())),
    }
}

/// The web shell on a fake browser.
struct Screen {
    bridge: Rc<Bridge<FakeHost>>,
    browser: Rc<Browser>,
}

impl Screen {
    fn new() -> Screen {
        let browser = Rc::new(Browser::default());
        let bridge = Bridge::new(FakeHost(browser.clone()));
        Screen { bridge, browser }
    }

    /// Lets the server answer every request, those its answers bring too.
    fn serve(&self) {
        loop {
            let next = self.browser.calls.borrow_mut().pop_front();
            let Some((op, done)) = next else {
                return;
            };
            done(answer_of(&op));
        }
    }

    fn renders(&self) -> usize {
        self.browser.renders.get()
    }
}

// シェルが開いたページは、サーバーから読まれ、答えが来るとシェルに描き直しを知らせる。
#[test]
fn a_page_the_shell_opens_is_read_and_the_shell_is_told_to_draw_it_when_the_answer_comes() {
    let screen = Screen::new();
    screen.bridge.route("/pages/notes");
    assert_eq!(screen.renders(), 0);

    screen.serve();

    assert!(screen.renders() > 0);
    let changes = screen.bridge.changes();
    assert!(changes.contains("the body of a"), "{changes}");
}
