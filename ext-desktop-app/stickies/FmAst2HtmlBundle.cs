using System.Reflection;

namespace Stickies;

/// <summary>
/// Loads the flatmarkdown-ast2html IIFE bundle from an embedded resource.
/// The JS file is downloaded from unpkg during build (see Stickies.csproj).
/// Exposes global FmAst2Html.renderToHtml(ast, options?).
/// </summary>
internal static class FmAst2HtmlBundle
{
    internal static readonly string Script = LoadScript();

    private static string LoadScript()
    {
        using var stream = Assembly.GetExecutingAssembly()
            .GetManifestResourceStream("fm-ast2html.min.js")!;
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }
}
