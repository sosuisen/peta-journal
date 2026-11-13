using System.Reflection;
using MemoView.Services;

namespace MemoView;

/// <summary>
/// Loads the style sheet and the scripts of a page from the embedded resources.
/// <list type="bullet">
/// <item><c>page.css</c> and <c>page.js</c> are the files in <c>Web/</c>.</item>
/// <item><c>fm-ast2html.js</c> is the script of flatmarkdown-ast2html. The build downloads it
/// from unpkg (see MemoView.csproj). It is the Rust renderer built as WebAssembly, with the
/// wasm inside the file. It adds the global FmAst2Html.renderToHtml(ast), which throws for
/// input that is not an AST.</item>
/// </list>
/// </summary>
internal static class EmbeddedPageAssets
{
    internal static readonly PageAssets Value = new(
        Css: Read("page.css"),
        RendererScript: Read("fm-ast2html.js"),
        PageScript: Read("page.js")
    );

    private static string Read(string name)
    {
        using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream(name)!;
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }
}
