//! Prints the AST the server produces for a few bodies, to compare with the Java wrapper.
use flatmarkdown::{body_to_ast_with, BodyOptions, LinkResolution};
fn main() {
    let prefixes: Vec<String> = std::env::args().skip(1).collect();
    let opts = BodyOptions { link_resolution: LinkResolution::On { prefixes } };
    for b in ["note #tag here",
        "[#test](../docs/test.md) and [Page](../docs/Page.md) and [x](docs/x.md) and [y](y.md) and [ext](https://e.x/a.md)",
        "[[Wiki]] and [[a#b|label]]", "- [ ] task", "\\#escaped",
        "[a](./docs/a.md)", "[b](../../journals/2026-01-01.md)", "[c](../x.md#frag)", "[d](<../docs/with space.md>)", "[e](/abs/x.md)", "[#f](../f.md)", "[../g](../g.md)"] {
        println!("{b} => {}", body_to_ast_with(b, &opts));
    }
}
