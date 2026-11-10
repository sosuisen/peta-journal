using System.Reflection;

namespace Stickies;

/// <summary>
/// Loads the script of flatmarkdown-ast2html from an embedded resource.
/// The build downloads the file from unpkg (see Stickies.csproj).
/// The script is the Rust renderer built as WebAssembly, with the wasm inside the file.
/// It adds the global FmAst2Html.renderToHtml(ast), which throws for input that is not an AST.
/// </summary>
internal static class FmAst2HtmlBundle
{
    internal static readonly string Script = LoadScript();

    private static string LoadScript()
    {
        using var stream = Assembly.GetExecutingAssembly()
            .GetManifestResourceStream("fm-ast2html.js")!;
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }
}
