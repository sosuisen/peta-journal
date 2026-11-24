use super::links::target;
use super::*;
use crate::asts::parse;

/// The body with this Markdown as a native shell shows it, with the tags in `tags/`.
/// The counts of the letters before each paragraph are left out: one test is about them.
fn shown(markdown: &str) -> Vec<RichBlock> {
    let mut blocks = RichText::of(parse(markdown), "tags/").blocks;
    without_starts(&mut blocks);
    blocks
}

fn without_starts(blocks: &mut [RichBlock]) {
    for block in blocks {
        match block {
            RichBlock::Text { paragraphs } => paragraphs.iter_mut().for_each(|p| p.start = 0),
            RichBlock::Code { start, .. } => *start = 0,
            RichBlock::Table { rows } => rows
                .iter_mut()
                .flat_map(|row| row.cells.iter_mut())
                .for_each(|cell| cell.start = 0),
            RichBlock::Image { .. } => {}
            RichBlock::Entries { entries, .. } => entries
                .iter_mut()
                .for_each(|entry| without_starts(&mut entry.blocks)),
            RichBlock::Quote { blocks, .. } => without_starts(blocks),
        }
    }
}

/// The paragraphs of the one text that the body is.
fn paragraphs(markdown: &str) -> Vec<RichParagraph> {
    match shown(markdown).as_slice() {
        [RichBlock::Text { paragraphs }] => paragraphs.clone(),
        blocks => panic!("not one text: {blocks:?}"),
    }
}

/// The runs of the paragraphs of the one text that the body is.
fn runs(markdown: &str) -> Vec<Vec<RichRun>> {
    paragraphs(markdown).into_iter().map(|p| p.runs).collect()
}

fn run(text: &str) -> RichRun {
    RichRun {
        text: text.to_string(),
        ..RichRun::default()
    }
}

fn styled(text: &str, set: impl FnOnce(&mut Looks)) -> RichRun {
    let mut looks = Looks::default();
    set(&mut looks);
    RichRun { looks, ..run(text) }
}

fn linked(text: &str, link: RichLink) -> RichRun {
    RichRun {
        link: Some(link),
        ..run(text)
    }
}

fn page(url: &str) -> RichLink {
    RichLink::Page {
        url: url.to_string(),
    }
}

fn paragraph(runs: Vec<RichRun>) -> RichParagraph {
    RichParagraph {
        runs,
        ..RichParagraph::default()
    }
}

/// The letters of a part and of the parts in it, without their looks.
fn letters(block: &RichBlock) -> String {
    let of_paragraphs = |paragraphs: &[RichParagraph]| {
        paragraphs
            .iter()
            .map(|p| {
                let mark = p
                    .mark
                    .as_deref()
                    .map(|m| format!("{m} "))
                    .unwrap_or_default();
                let line = if p.rule {
                    "────".to_string()
                } else {
                    String::new()
                };
                let runs: String = p.runs.iter().map(|r| r.text.as_str()).collect();
                format!("{mark}{line}{runs}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    match block {
        RichBlock::Text { paragraphs } => of_paragraphs(paragraphs),
        RichBlock::Code { lines, .. } => lines.clone(),
        RichBlock::Table { rows } => rows
            .iter()
            .map(|row| of_paragraphs(&row.cells))
            .collect::<Vec<_>>()
            .join("\n"),
        RichBlock::Image { image } => image.alt.clone(),
        RichBlock::Entries { entries, .. } => entries
            .iter()
            .map(|e| format!("{} {}", e.mark, all_letters(&e.blocks)))
            .collect::<Vec<_>>()
            .join("\n"),
        RichBlock::Quote { blocks, .. } => all_letters(blocks),
    }
}

fn all_letters(blocks: &[RichBlock]) -> String {
    blocks.iter().map(letters).collect::<Vec<_>>().join("\n")
}

fn image(path: &str, alt: &str, width: Option<u32>, height: Option<u32>) -> RichBlock {
    RichBlock::Image {
        image: RichImage {
            path: path.to_string(),
            alt: alt.to_string(),
            width,
            height,
        },
    }
}

// 段落は、1つの段落になる。
#[test]
fn a_paragraph_is_one_paragraph() {
    assert_eq!(paragraphs("plain"), [paragraph(vec![run("plain")])]);
}

// 段落が2つあれば、2つの段落になる。
#[test]
fn two_paragraphs_are_two_paragraphs() {
    assert_eq!(runs("one\n\ntwo"), [vec![run("one")], vec![run("two")]]);
}

// 太字・斜体・取り消し線・下線・ハイライト・コードは、その部分だけの見た目になる。
#[test]
fn the_looks_of_letters_belong_to_their_run_only() {
    assert_eq!(
        runs("a **b** *c* ~~d~~ __e__ ==f== `g`"),
        [vec![
            run("a "),
            styled("b", |l| l.bold = true),
            run(" "),
            styled("c", |l| l.italic = true),
            run(" "),
            styled("d", |l| l.struck = true),
            run(" "),
            styled("e", |l| l.underlined = true),
            run(" "),
            styled("f", |l| l.marked = true),
            run(" "),
            styled("g", |l| l.code = true),
        ]]
    );
}

// 入れ子の装飾は、両方の見た目を持つ。
#[test]
fn looks_inside_each_other_add_up() {
    assert_eq!(
        runs("**a *b***"),
        [vec![
            styled("a ", |l| l.bold = true),
            styled("b", |l| {
                l.bold = true;
                l.italic = true
            }),
        ]]
    );
}

// 上付き・下付き・ネタバレも見た目になる。
#[test]
fn raised_lowered_and_hidden_are_looks() {
    assert_eq!(
        runs("x^a^ x~b~ ||c||"),
        [vec![
            run("x"),
            styled("a", |l| l.raised = true),
            run(" x"),
            styled("b", |l| l.lowered = true),
            run(" "),
            styled("c", |l| l.hidden = true),
        ]]
    );
}

// リンクの中の装飾は、リンク先と見た目の両方を持つ。
#[test]
fn looks_inside_a_link_keep_its_target() {
    let web = RichLink::Web {
        url: "https://e.example/".to_string(),
    };
    assert_eq!(
        runs("[a **b**](https://e.example/)"),
        [vec![
            linked("a ", web.clone()),
            RichRun {
                looks: Looks {
                    bold: true,
                    ..Looks::default()
                },
                ..linked("b", web)
            },
        ]]
    );
}

// ページへのリンクとタグは、アプリの中のページの、符号化したURLへのリンクになる。タグのページは、タグのフォルダーの下にある。
#[test]
fn a_link_to_a_page_and_a_tag_lead_to_their_page_in_the_app() {
    assert_eq!(
        runs("[[other page]] and #topic"),
        [vec![
            linked("other page", page("/pages/other%20page")),
            run(" and "),
            linked("#topic", page("/pages/tags/topic")),
        ]]
    );
}

// wikilinkの名前の#はページ名の一部で、項目のidの前の#だけが、ページの中の場所を指す。
#[test]
fn a_hash_of_a_wikilink_is_a_part_of_the_name_unless_an_item_id_follows_it() {
    assert_eq!(
        runs("[[C#]] [[C##2020-01-01-00-00-00-AAAAAAA]]"),
        [vec![
            linked("C#", page("/pages/C%23")),
            run(" "),
            linked(
                "C##2020-01-01-00-00-00-AAAAAAA",
                page("/pages/C%23#2020-01-01-00-00-00-AAAAAAA")
            ),
        ]]
    );
}

// .mdへの相対リンクは、そのページへのリンクになる。ページの中の項目への印も残る。
#[test]
fn a_relative_link_to_an_md_file_leads_to_its_page() {
    assert_eq!(
        runs("[the other](./other.md#2020-01-01-00-00-00-AAAAAAA)"),
        [vec![linked(
            "the other",
            page("/pages/other#2020-01-01-00-00-00-AAAAAAA")
        )]]
    );
}

// リンクの行き先は、Web、ページ、assetsのファイルに分かれる。ほかのリンクは、どこにも行かない。
#[test]
fn a_link_leads_to_the_web_to_a_page_or_to_a_file_of_the_assets_folder() {
    let web = "https://example.com/a?b=c";
    assert_eq!(
        target(web),
        Some(RichLink::Web {
            url: web.to_string()
        })
    );
    assert_eq!(
        target("HTTP://example.com/"),
        Some(RichLink::Web {
            url: "HTTP://example.com/".to_string()
        })
    );
    assert_eq!(
        target("/pages/other%20page"),
        Some(page("/pages/other%20page"))
    );
    assert_eq!(
        target("/pages/other page"),
        Some(page("/pages/other%20page"))
    );
    assert_eq!(
        target("/pages/C%23#20260426035700_QZ998PX"),
        Some(page("/pages/C%23#20260426035700_QZ998PX"))
    );
    assert_eq!(
        target("../pages/./x/../notes?q=1"),
        Some(page("/pages/notes"))
    );
    let file = |path: &str| {
        Some(RichLink::File {
            path: path.to_string(),
        })
    };
    assert_eq!(
        target("../assets/Report%201.pdf"),
        file("assets/Report 1.pdf")
    );
    assert_eq!(target("../../Assets/a.pdf#page=2"), file("Assets/a.pdf"));
    assert_eq!(target("file:///C:/notes.txt"), None);
    assert_eq!(target("../other/report.pdf"), None);
    assert_eq!(target("mailto:someone@example.com"), None);
    assert_eq!(target("//host/assets/a.pdf"), None);
    assert_eq!(target("C:/assets/a.pdf"), None);
    // A colon after a space starts no scheme: the address is a relative path
    assert_eq!(target("my notes:1/../assets/a.pdf"), file("assets/a.pdf"));
}

// 見出しは、そのレベルを持つ。
#[test]
fn a_heading_has_its_level() {
    assert_eq!(
        paragraphs("## Title"),
        [RichParagraph {
            heading: 2,
            scale: 1.4,
            ..paragraph(vec![run("Title")])
        }]
    );
}

// 見出しの段落は、レベルの倍率を持つ。h6はh5と同じ1で、見出しでない段落も1である（#795）。
#[test]
fn a_heading_has_the_scale_of_its_level() {
    let scales: Vec<_> =
        paragraphs("# 1\n\n## 2\n\n### 3\n\n#### 4\n\n##### 5\n\n###### 6\n\ntext")
            .iter()
            .map(|p| p.scale)
            .collect();
    assert_eq!(scales, [1.6, 1.4, 1.2, 1.1, 1.0, 1.0, 1.0]);
}

// 段落の字下げは、リストと引用の深さの和の1.5em倍である（#795）。
#[test]
fn a_paragraph_is_indented_by_its_depth_and_its_quotes() {
    let indents: Vec<_> = paragraphs("text\n\n- one\n  - two\n\n> - quoted\n>\n> ***")
        .iter()
        .map(|p| p.indent)
        .collect();
    assert_eq!(indents, [0.0, 1.5, 3.0, 3.0, 1.5]);
}

// 段落の中の改行は空白になり、改行の記号は改行になる。
#[test]
fn a_line_end_is_a_space_and_a_break_is_a_line_break() {
    assert_eq!(runs("one\ntwo\\\nthree"), [vec![run("one two\nthree")]]);
}

// 箇条書きの項目は、印と深さを持つ段落になる。
#[test]
fn the_entries_of_a_list_have_a_mark_and_a_depth() {
    let entry = |depth: u32, text| RichParagraph {
        depth,
        indent: depth as f32 * 1.5,
        mark: Some("•".to_string()),
        ..paragraph(vec![run(text)])
    };
    assert_eq!(
        paragraphs("- one\n- two\n  - inner"),
        [entry(1, "one"), entry(1, "two"), entry(2, "inner")]
    );
}

// 番号付きリストの印は、始まりの番号から数える。
#[test]
fn the_marks_of_a_numbered_list_count_from_its_start() {
    let marks: Vec<_> = paragraphs("3. c\n4. d\n\ntext\n\n1. a")
        .into_iter()
        .map(|p| p.mark)
        .collect();
    let mark = |m: &str| Some(m.to_string());
    assert_eq!(marks, [mark("3."), mark("4."), None, mark("1.")]);
}

// タスクの項目の印は、チェックボックスである。
#[test]
fn the_mark_of_a_task_entry_is_a_checkbox() {
    let marks: Vec<_> = paragraphs("- plain\n- [ ] open\n- [x] done")
        .into_iter()
        .map(|p| p.mark.unwrap_or_default())
        .collect();
    assert_eq!(marks, ["•", "☐", "☑"]);
}

// 項目の中の2つ目の段落は、印を持たず、同じ深さにある。
#[test]
fn the_second_paragraph_of_an_entry_has_no_mark() {
    assert_eq!(
        paragraphs("- one\n\n  more"),
        [
            RichParagraph {
                depth: 1,
                indent: 1.5,
                mark: Some("•".to_string()),
                ..paragraph(vec![run("one")])
            },
            RichParagraph {
                depth: 1,
                indent: 1.5,
                ..paragraph(vec![run("more")])
            },
        ]
    );
}

// 文字のない項目も、印だけの段落になる。
#[test]
fn an_entry_without_text_is_a_paragraph_with_only_its_mark() {
    assert_eq!(
        paragraphs("- a\n-\n- b"),
        [
            RichParagraph {
                depth: 1,
                indent: 1.5,
                mark: Some("•".to_string()),
                ..paragraph(vec![run("a")])
            },
            RichParagraph {
                depth: 1,
                indent: 1.5,
                mark: Some("•".to_string()),
                ..paragraph(vec![])
            },
            RichParagraph {
                depth: 1,
                indent: 1.5,
                mark: Some("•".to_string()),
                ..paragraph(vec![run("b")])
            },
        ]
    );
}

// 引用の中の段落は、引用の深さを持つ。
#[test]
fn a_paragraph_in_a_quote_knows_how_deep_it_is_quoted() {
    assert_eq!(
        paragraphs("> said\n>\n> > before\n\nafter"),
        [
            RichParagraph {
                quoted: 1,
                indent: 1.5,
                ..paragraph(vec![run("said")])
            },
            RichParagraph {
                quoted: 2,
                indent: 3.0,
                ..paragraph(vec![run("before")])
            },
            paragraph(vec![run("after")]),
        ]
    );
}

// アラートは、題を最初の段落に持つ引用になる。題がなければ、種類の名前が題である。
#[test]
fn an_alert_is_a_quote_with_its_title_first() {
    let quoted = |text| RichParagraph {
        quoted: 1,
        indent: 1.5,
        ..paragraph(vec![run(text)])
    };
    assert_eq!(
        paragraphs("> [!WARNING]\n> careful"),
        [quoted("Warning"), quoted("careful")]
    );
    assert_eq!(
        paragraphs("> [!NOTE] Mind\n> this"),
        [quoted("Mind"), quoted("this")]
    );
}

// 区切り線は、線だけの段落になる。
#[test]
fn a_thematic_break_is_a_paragraph_that_is_a_rule() {
    assert_eq!(
        paragraphs("a\n\n---\n\nb"),
        [
            paragraph(vec![run("a")]),
            RichParagraph {
                rule: true,
                ..RichParagraph::default()
            },
            paragraph(vec![run("b")]),
        ]
    );
}

// 脚注は、その名前を前に置いた段落になる。脚注への参照は、上付きの名前になる。
#[test]
fn a_footnote_starts_with_its_name_and_a_reference_is_its_name_raised() {
    assert_eq!(
        runs("see^[note]"),
        [
            vec![run("see"), styled("[1]", |l| l.raised = true)],
            vec![run("[1]: note")],
        ]
    );
}

// 項目は、Markdownの記号ではなく、飾りの付いた文字になる。
#[test]
fn an_item_is_text_with_its_decoration_and_not_the_marks_of_markdown() {
    assert_eq!(
        runs("some **bold** text"),
        [vec![
            run("some "),
            styled("bold", |l| l.bold = true),
            run(" text")
        ]]
    );
}

// コードブロックは、行が分かれたまま、書かれたとおりの部品になる。フェンスの記号は入らない。
#[test]
fn a_code_block_keeps_its_lines_as_they_were_written() {
    let blocks = shown("before\n\n```kotlin\nval x = 1\n  val y = 2\n```\n\nafter");
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    assert_eq!(letters(&blocks[0]), "before");
    assert_eq!(
        blocks[1],
        RichBlock::Code {
            lines: "val x = 1\n  val y = 2".to_string(),
            start: 0,
        }
    );
    assert_eq!(letters(&blocks[2]), "after");
}

// 表は、セルごとに分かれた部品になる。セルの中の文字の飾りも付く。
#[test]
fn a_table_has_each_cell_apart() {
    let blocks = shown("| name | count |\n|---|---|\n| **apple** | 2 |");
    let [RichBlock::Table { rows }] = blocks.as_slice() else {
        panic!("no table: {blocks:?}");
    };
    let headers: Vec<_> = rows.iter().map(|r| r.header).collect();
    assert_eq!(headers, [true, false]);
    assert_eq!(
        rows[0].cells,
        [paragraph(vec![run("name")]), paragraph(vec![run("count")])]
    );
    assert_eq!(
        rows[1].cells,
        [
            paragraph(vec![styled("apple", |l| l.bold = true)]),
            paragraph(vec![run("2")])
        ]
    );
}

// 空のセルも、空の段落として残る。
#[test]
fn an_empty_cell_stays_as_an_empty_paragraph() {
    let blocks = shown("| a | b |\n|---|---|\n|  | x |");
    let [RichBlock::Table { rows }] = blocks.as_slice() else {
        panic!("no table: {blocks:?}");
    };
    assert_eq!(
        rows[1].cells,
        [paragraph(vec![]), paragraph(vec![run("x")])]
    );
}

// assetsの画像は、前後の文字のあいだの、画像の部品になる。
#[test]
fn an_image_of_the_assets_folder_stands_between_the_text_around_it() {
    let blocks = shown("before ![a cat](../assets/Cat.png) after");
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    assert_eq!(letters(&blocks[0]), "before");
    assert_eq!(blocks[1], image("assets/Cat.png", "a cat", None, None));
    assert_eq!(letters(&blocks[2]), "after");
}

// 空白だけを挟んだ2つの画像は、あいだに文字の部品を持たない。
#[test]
fn two_images_with_only_a_space_between_them_have_no_text_between_them() {
    assert_eq!(
        shown("![a](../assets/a.png) ![b](../assets/b.png)"),
        [
            image("assets/a.png", "a", None, None),
            image("assets/b.png", "b", None, None)
        ]
    );
}

// 画像は、ページが決めた幅と高さを持つ。決めていない辺や、0より大きな数でない辺は、ない。
#[test]
fn an_image_has_the_sides_that_the_page_asks_for() {
    assert_eq!(
        shown("![both](../assets/cat.png#w=120&h=30)"),
        [image("assets/cat.png", "both", Some(120), Some(30))]
    );
    assert_eq!(
        shown("![tall](../assets/cat.png#h=60)"),
        [image("assets/cat.png", "tall", None, Some(60))]
    );
    assert_eq!(
        shown("![odd](../assets/cat.png#w=0&h=wide)"),
        [image("assets/cat.png", "odd", None, None)]
    );
}

// Web上の画像は、画像の部品にならない。代わりの文字のリンクになる。代わりの文字がなければ、アドレスが出る。
#[test]
fn an_image_of_the_web_is_a_link_and_not_a_picture() {
    let bird = RichLink::Web {
        url: "https://example.com/bird.png".to_string(),
    };
    assert_eq!(
        runs("see ![a bird](https://example.com/bird.png) here"),
        [vec![
            run("see "),
            linked("a bird", bird.clone()),
            run(" here")
        ]]
    );
    assert_eq!(
        runs("![](https://example.com/bird.png)"),
        [vec![linked("https://example.com/bird.png", bird)]]
    );
}

// assetsの外の画像は、代わりの文字になる。
#[test]
fn an_image_outside_the_assets_folder_is_its_words() {
    assert_eq!(runs("![a dog](images/dog.png)"), [vec![run("a dog")]]);
}

// リストの中のコードブロックも、行が分かれたまま残る。リストの項目は、番号と一緒に残る。
#[test]
fn a_code_block_inside_a_list_keeps_its_lines_and_the_entries_keep_their_numbers() {
    let blocks = shown("1. first\n\n   ```\n   a\n     b\n   ```\n2. second");
    let [RichBlock::Entries { entries, .. }] = blocks.as_slice() else {
        panic!("no entries: {blocks:?}");
    };
    let marks: Vec<_> = entries.iter().map(|e| e.mark.as_str()).collect();
    assert_eq!(marks, ["1.", "2."]);
    assert_eq!(entries[0].blocks.len(), 2);
    assert_eq!(letters(&entries[0].blocks[0]), "first");
    assert_eq!(
        entries[0].blocks[1],
        RichBlock::Code {
            lines: "a\n  b".to_string(),
            start: 0,
        }
    );
    assert_eq!(all_letters(&entries[1].blocks), "second");
}

// 部品を持つリストと引用は、中のものを1段（1.5em）右へ寄せる。段落の字下げと同じ幅である（#795）。
#[test]
fn a_list_or_a_quote_that_holds_a_part_moves_it_one_step_right() {
    let code = "```\na\n```";
    let in_list = shown(&format!("- a\n\n  {}", code.replace('\n', "\n  ")));
    let in_quote = shown(&format!("> {}", code.replace('\n', "\n> ")));
    let indents: Vec<_> = [in_list, in_quote]
        .iter()
        .map(|blocks| match blocks.as_slice() {
            [RichBlock::Entries { indent, .. }] | [RichBlock::Quote { indent, .. }] => *indent,
            blocks => panic!("not one list or quote: {blocks:?}"),
        })
        .collect();
    assert_eq!(indents, [1.5, 1.5]);
}

// 引用の中の表も、セルごとに分かれて残る。
#[test]
fn a_table_inside_a_quote_has_each_cell_apart() {
    let blocks = shown("> said:\n>\n> | name | count |\n> |---|---|\n> | apple | 2 |");
    let [RichBlock::Quote { blocks: inside, .. }] = blocks.as_slice() else {
        panic!("no quote: {blocks:?}");
    };
    assert_eq!(letters(&inside[0]), "said:");
    let Some(RichBlock::Table { rows }) = inside.last() else {
        panic!("no table: {inside:?}");
    };
    assert_eq!(rows.len(), 2);
}

// リストの中のリストの画像も、画像の部品になる。
#[test]
fn an_image_inside_a_list_inside_a_list_is_a_picture() {
    let blocks = shown("- outer\n  - look ![a cat](../assets/cat.png)\n- next");
    let [RichBlock::Entries { entries: outer, .. }] = blocks.as_slice() else {
        panic!("no entries: {blocks:?}");
    };
    let marks: Vec<_> = outer.iter().map(|e| e.mark.as_str()).collect();
    assert_eq!(marks, ["•", "•"]);
    let Some(RichBlock::Entries { entries: inner, .. }) = outer[0].blocks.last() else {
        panic!("no inner list: {:?}", outer[0]);
    };
    assert_eq!(
        inner[0].blocks.last(),
        Some(&image("assets/cat.png", "a cat", None, None))
    );
}

// 自分の部品を持たないリストと引用は、分けられずに1つの文字のままである。
#[test]
fn a_list_and_a_quote_that_hold_no_part_of_their_own_stay_one_text() {
    let marks: Vec<_> = paragraphs("- one\n- two")
        .into_iter()
        .map(|p| p.mark)
        .collect();
    assert_eq!(marks, [Some("•".to_string()), Some("•".to_string())]);
    assert_eq!(paragraphs("> a\n>\n> b").len(), 2);
}

// 複数行の引用も、自分の部品を持てば分けられる。
#[test]
fn a_multiline_quote_with_a_part_of_its_own_is_taken_apart() {
    let blocks = shown(">>>\nsaid\n\n```\ncode\n```\n>>>");
    let [RichBlock::Quote { blocks: inside, .. }] = blocks.as_slice() else {
        panic!("no quote: {blocks:?}");
    };
    assert_eq!(all_letters(inside), "said\ncode");
}

// リストの中のタスクの項目は、チェックボックスの印を持つ。
#[test]
fn the_entries_of_a_task_list_have_a_checkbox() {
    let blocks = shown("- plain\n- [ ] open\n\n  ```\n  code\n  ```\n- [x] done");
    let [RichBlock::Entries { entries, .. }] = blocks.as_slice() else {
        panic!("no entries: {blocks:?}");
    };
    let marks: Vec<_> = entries.iter().map(|e| e.mark.as_str()).collect();
    assert_eq!(marks, ["•", "☐", "☑"]);
}

// 本文のない項目は、部品を持たない。
#[test]
fn an_empty_item_has_no_parts() {
    assert_eq!(shown(""), []);
}

/// A body with every kind of node that the AST has, and for each kind what of it has to be
/// seen. `document`, `item`, `table_row` and `table_cell` are seen by what is in them.
const EVERY_KIND: [(&str, &str, &str); 33] = [
    ("heading", "## HEADING", "HEADING"),
    ("paragraph", "PARAGRAPH", "PARAGRAPH"),
    ("text", "TEXT", "TEXT"),
    ("softbreak", "SOFT\nBREAK", "SOFT BREAK"),
    ("linebreak", "LINE\\\nBROKEN", "LINE\nBROKEN"),
    ("emph", "*EMPH*", "EMPH"),
    ("strong", "**STRONG**", "STRONG"),
    ("strikethrough", "~~STRUCK~~", "STRUCK"),
    ("underline", "__UNDERLINED__", "UNDERLINED"),
    ("highlight", "==HIGHLIGHTED==", "HIGHLIGHTED"),
    ("superscript", "x^SUPER^", "SUPER"),
    ("subscript", "x~SUB~", "SUB"),
    ("spoilered_text", "||SPOILER||", "SPOILER"),
    ("code", "`CODE`", "CODE"),
    ("link", "[LINK](https://example.com/)", "LINK"),
    ("embed", "![ALT](images/a.png)", "ALT"),
    ("wikilink", "[[WIKI]]", "WIKI"),
    ("hashtag", "#hashtag", "#hashtag"),
    ("shortcode", ":smile:", "😄"),
    ("math", "$`MATH`$", "MATH"),
    ("html_inline", "<kbd>KEY</kbd>", "KEY"),
    ("footnote_reference", "NOTED[^note]", "[note]"),
    (
        "footnote_definition",
        "[^note]: FOOTNOTE",
        "[note]: FOOTNOTE",
    ),
    ("code_block", "```rust\nCODEBLOCK\n```", "CODEBLOCK"),
    ("block_quote", "> QUOTED", "QUOTED"),
    (
        "multiline_block_quote",
        ">>>\nMULTIQUOTED\n>>>",
        "MULTIQUOTED",
    ),
    ("alert", "> [!TIP]\n> ALERTED", "Tip\nALERTED"),
    ("list", "1. LISTED", "1. LISTED"),
    ("task_item", "- [ ] OPEN\n- [x] CLOSED", "☐ OPEN\n☑ CLOSED"),
    ("table", "| HEAD |\n|---|\n| CELL |", "HEAD\nCELL"),
    ("thematic_break", "---", "────"),
    ("subtext", "-# SUBTEXT", "SUBTEXT"),
    (
        "html_block",
        "<details>\nHTMLBLOCK\n</details>",
        "HTMLBLOCK",
    ),
];

// ASTのどの種類の中身も、表示する部品に入っている。
#[test]
fn what_is_in_every_kind_of_node_is_shown() {
    let body = EVERY_KIND.map(|(_, markdown, _)| markdown).join("\n\n");
    let ast = parse(&body);
    let mut kinds = std::collections::HashSet::new();
    ast.walk(&mut |node| {
        kinds.insert(node.type_name().to_string());
    });
    let seen = all_letters(&RichText::of(ast, "tags/").blocks);
    for (kind, _, letters) in EVERY_KIND {
        assert!(
            kinds.contains(kind),
            "the body has no node of the kind {kind}"
        );
        assert!(
            seen.contains(letters),
            "the kind {kind} is not shown: {letters:?} is missing from {seen:?}"
        );
    }
}

// 未知の種類は、その中の文字が残る。
#[test]
fn the_text_inside_an_unknown_kind_stays() {
    let unknown = flatmarkdown_ast::Node::Document {
        children: vec![flatmarkdown_ast::Node::Paragraph {
            children: vec![flatmarkdown_ast::Node::Unknown {
                node_type: "insert".to_string(),
                children: vec![flatmarkdown_ast::Node::Text {
                    value: "kept".to_string(),
                }],
            }],
        }],
    };
    assert_eq!(
        RichText::of(unknown, "").blocks,
        [RichBlock::Text {
            paragraphs: vec![paragraph(vec![run("kept")])]
        }]
    );
}

/// The paragraphs that these nodes make as text, without the parts of their own taken out
/// first.
fn as_text(nodes: Vec<flatmarkdown_ast::Node>) -> Vec<RichParagraph> {
    super::paragraphs::paragraphs(&nodes, "tags/")
}

fn text_node(value: &str) -> flatmarkdown_ast::Node {
    flatmarkdown_ast::Node::Text {
        value: value.to_string(),
    }
}

// 文の中のコードブロックは、コードの見た目の段落になる。
#[test]
fn a_code_block_in_a_text_is_a_paragraph_that_looks_like_code() {
    let code = flatmarkdown_ast::Node::CodeBlock {
        fenced: true,
        info: String::new(),
        literal: "let x = 1\n\n".to_string(),
    };
    assert_eq!(
        as_text(vec![code]),
        [paragraph(vec![styled("let x = 1", |l| l.code = true)])]
    );
}

// 文の中の表は、行ごとの段落になる。
#[test]
fn a_table_in_a_text_is_a_paragraph_for_each_row() {
    let ast = parse("| a | b |\n|---|---|\n| c | d |");
    assert_eq!(
        as_text(ast.children().to_vec()),
        [paragraph(vec![run("ab")]), paragraph(vec![run("cd")])]
    );
}

// 段落の外の文字も、1つの段落になる。行の中のブロックは、その文字だけになる。
#[test]
fn letters_outside_a_block_are_a_paragraph_and_a_block_in_a_line_is_its_letters() {
    let quote_in_line = flatmarkdown_ast::Node::Paragraph {
        children: vec![flatmarkdown_ast::Node::BlockQuote {
            children: vec![text_node("inner")],
        }],
    };
    assert_eq!(
        as_text(vec![text_node("loose"), quote_in_line]),
        [paragraph(vec![run("loose")]), paragraph(vec![run("inner")])]
    );
}

// アラートの題は、種類ごとの名前である。
#[test]
fn the_title_of_an_alert_is_the_name_of_its_kind() {
    let titles: Vec<_> = ["NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION"]
        .into_iter()
        .map(|kind| runs(&format!("> [!{kind}]\n> x"))[0][0].text.clone())
        .collect();
    assert_eq!(titles, ["Note", "Tip", "Important", "Warning", "Caution"]);
}

// 画像の前の文字の終わりの空白は、文字に残らない。
#[test]
fn the_space_before_an_image_is_not_left_in_the_text() {
    let blocks = shown("**a** ![x](../assets/x.png)");
    assert_eq!(
        blocks[0],
        RichBlock::Text {
            paragraphs: vec![paragraph(vec![styled("a", |l| l.bold = true)])]
        }
    );
}

// 区切り線とリストの後の段落は、そこでのリストと引用の深さを持つ。
#[test]
fn a_rule_and_a_paragraph_after_a_list_know_how_deep_they_are() {
    let shown: Vec<_> = paragraphs("> - a\n>\n>   ---\n>\n> b")
        .into_iter()
        .map(|p| (p.depth, p.quoted, p.rule))
        .collect();
    assert_eq!(shown, [(1, 1, false), (1, 1, true), (0, 1, false)]);
}

// 引用の中の、文字のない項目も、引用の深さを持つ。
#[test]
fn an_entry_without_text_in_a_quote_knows_how_deep_it_is_quoted() {
    let quoted: Vec<_> = paragraphs("> - a\n> -")
        .into_iter()
        .map(|p| (p.quoted, p.mark))
        .collect();
    let bullet = Some("•".to_string());
    assert_eq!(quoted, [(1, bullet.clone()), (1, bullet)]);
}

// 小さな注記の文字は、下付きの見た目になる。
#[test]
fn a_subtext_is_lowered() {
    assert_eq!(
        runs("-# small"),
        [vec![styled("small", |l| l.lowered = true)]]
    );
}

// 行の中のHTMLは、書かれたまま文字になる。
#[test]
fn html_in_a_line_stays_as_it_was_written() {
    assert_eq!(runs("a <kbd>K</kbd>"), [vec![run("a <kbd>K</kbd>")]]);
}

// 未知の種類の中の装飾も残る。
#[test]
fn the_looks_inside_an_unknown_kind_stay() {
    use flatmarkdown_ast::Node;
    let unknown = Node::Paragraph {
        children: vec![Node::Unknown {
            node_type: "insert".to_string(),
            children: vec![Node::Strong {
                children: vec![text_node("kept")],
            }],
        }],
    };
    assert_eq!(
        as_text(vec![unknown]),
        [paragraph(vec![styled("kept", |l| l.bold = true)])]
    );
}

// 記号のない本文では、表示の位置がそのまま本文の位置である。
#[test]
fn in_a_body_without_marks_the_place_is_the_same() {
    assert_eq!(markdown_place("plain text", "plain"), 5);
}

// 前に文字のない触れ方は、本文の先頭である。
#[test]
fn a_touch_with_no_letter_before_it_is_at_the_start() {
    assert_eq!(markdown_place("**a**", ""), 0);
}

// 装飾の記号は飛ばされ、触れた文字のすぐ後ろに置かれる。
#[test]
fn the_marks_of_the_looks_are_skipped() {
    assert_eq!(markdown_place("**bold** x", "bo"), 4);
}

// リンクの宛先は表示されないので、飛ばされる。
#[test]
fn the_address_of_a_link_is_skipped() {
    let markdown = "[link](https://example.com/a) and more";

    assert_eq!(
        markdown_place(markdown, "link and m"),
        markdown.find(" and m").unwrap() + " and m".len()
    );
}

// アラートの題のあとの文字は、アラートの本文に置かれる。
#[test]
fn the_letters_after_the_title_of_an_alert_are_in_its_body() {
    let markdown = "> [!TIP]\n> said here";

    assert_eq!(
        markdown_place(markdown, "Tip\nsaid"),
        markdown.find("said").unwrap() + "said".len()
    );
}

// アラートの題に触れると、本文が長くても、アラートの記号の名前に置かれる。
#[test]
fn a_touch_on_the_title_of_an_alert_is_at_its_name_however_long_the_body() {
    let markdown = "> [!TIP]\n> a long body that ends in a cup";

    assert_eq!(
        markdown_place(markdown, "Tip"),
        markdown.find("TIP").unwrap() + "TIP".len()
    );
}

// 表の2つめの升の先頭に触れると、その升の中に置かれる。
#[test]
fn a_touch_at_the_start_of_the_second_cell_of_a_table_is_in_that_cell() {
    let markdown = "| a | b is a cell with many words in it |\n|---|---|\n| b | c |";

    assert_eq!(
        markdown_place(markdown, "a\n"),
        markdown.find("| b").unwrap() + "|".len()
    );
}

// ショートコードの絵文字に触れると、本文のずっと先にある同じ絵文字へは跳ばない。
#[test]
fn a_touch_on_an_emoji_of_a_shortcode_does_not_jump_to_the_same_emoji_far_on() {
    let markdown = "x :tada: and then a long sentence that ends in 🎉";

    let place = markdown_place(markdown, "x 🎉");

    assert!(place <= "x :tada:".len(), "{place}");
}

// 項目の文字の後ろに触れると、本文の末尾に置かれる。
#[test]
fn a_touch_after_the_text_of_an_item_is_at_the_end_of_its_markdown() {
    assert_eq!(markdown_place("abc", "abc\n"), 3);
}

// 本文の位置は、バイトではなく文字で数える。
#[test]
fn the_place_counts_characters_and_not_bytes() {
    assert_eq!(markdown_place("**日本**", "日"), 3);
}

/// What each paragraph and code block says of the letters before it, from top to bottom.
fn starts(blocks: &[RichBlock]) -> Vec<u32> {
    blocks
        .iter()
        .flat_map(|block| match block {
            RichBlock::Text { paragraphs } => paragraphs.iter().map(|p| p.start).collect(),
            RichBlock::Code { start, .. } => vec![*start],
            RichBlock::Table { rows } => rows
                .iter()
                .flat_map(|row| row.cells.iter().map(|cell| cell.start))
                .collect(),
            RichBlock::Image { .. } => vec![],
            RichBlock::Entries { entries, .. } => entries
                .iter()
                .flat_map(|entry| starts(&entry.blocks))
                .collect(),
            RichBlock::Quote { blocks, .. } => starts(blocks),
        })
        .collect()
}

// 段落とコードブロックは、項目の表示の文字のうち、その前に立つ文字の数を持つ。各段落とコードブロックのあとには、改行が1つあると数える。
#[test]
fn each_paragraph_and_code_block_knows_how_many_shown_letters_stand_before_it() {
    let body = "**a** b\n\n```\nxy\n```\n\n- c\n\n  ```\n  z\n  ```\n\n| h |\n|---|\n| d |\n\n---";

    let blocks = RichText::of(parse(body), "tags/").blocks;

    assert_eq!(starts(&blocks), [0, 4, 7, 9, 11, 13, 15]);
}
