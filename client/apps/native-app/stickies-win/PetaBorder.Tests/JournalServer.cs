using System.Diagnostics;
using System.Text.Json;

namespace PetaBorder.Tests;

/// <summary>
/// A journal server for a test, with its files in a folder of its own. It needs the program:
/// the tests that use it are skipped when <c>PETA_E2E_SERVER</c> does not name it.
/// </summary>
internal sealed class JournalServer : IDisposable
{
    private static readonly HttpClient Http = new();

    /// <summary>The client that <see cref="Send"/> changes things as.</summary>
    private static readonly string AnotherClient = Guid.NewGuid().ToString();
    private readonly Process process;
    private readonly string root;

    public string Address { get; }

    /// <summary>
    /// Starts the server and waits until it answers. <paramref name="functions"/> are the
    /// extensions of the server: the <c>main.js</c> of each, by its name.
    /// </summary>
    public JournalServer(int port, IReadOnlyDictionary<string, string>? functions = null)
    {
        var program = Environment.GetEnvironmentVariable("PETA_E2E_SERVER");
        SkipTest.When(
            string.IsNullOrEmpty(program),
            "PETA_E2E_SERVER does not name the journal-server program"
        );
        Address = $"http://127.0.0.1:{port}";
        root = Path.Combine(Path.GetTempPath(), $"peta-winui-test-{Guid.NewGuid()}");
        var config = Path.Combine(root, "config");
        var markdown = Path.Combine(root, "markdown");
        Directory.CreateDirectory(config);
        Directory.CreateDirectory(markdown);
        // Without this the server would write into the notes of whoever runs the test
        var settings = new Dictionary<string, string>
        {
            ["markdownDir"] = markdown,
            ["weekNumbering"] = "iso",
        };
        File.WriteAllText(Path.Combine(config, "config.json"), JsonSerializer.Serialize(settings));
        foreach (var (name, mainJs) in functions ?? new Dictionary<string, string>())
        {
            var folder = Path.Combine(config, "functions", name);
            Directory.CreateDirectory(folder);
            var described = new Dictionary<string, string> { ["name"] = name };
            File.WriteAllText(
                Path.Combine(folder, "config.json"),
                JsonSerializer.Serialize(described)
            );
            File.WriteAllText(Path.Combine(folder, "main.js"), mainJs);
        }

        var start = new ProcessStartInfo(program!)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        foreach (
            var argument in (string[])
                [
                    "--port",
                    $"{port}",
                    "--data-dir",
                    Path.Combine(root, "data"),
                    "--index-dir",
                    Path.Combine(root, "index"),
                    "--config-dir",
                    config,
                ]
        )
            start.ArgumentList.Add(argument);
        process = Process.Start(start)!;
        // Nobody reads what the server prints, and a full pipe would stop it
        process.OutputDataReceived += (_, _) => { };
        process.ErrorDataReceived += (_, _) => { };
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
        WaitUntilUp(markdown);
    }

    private void WaitUntilUp(string markdown)
    {
        var waited = Stopwatch.StartNew();
        while (waited.Elapsed < TimeSpan.FromSeconds(20))
        {
            if (Get("/api/config") is { } config)
            {
                var inUse = JsonElement.Parse(config).GetProperty("markdownDir").GetString();
                if (!SameFolder(inUse, markdown))
                {
                    Dispose();
                    Assert.Fail($"the server writes elsewhere: {inUse}");
                }
                return;
            }
            Thread.Sleep(50);
        }
        Dispose();
        Assert.Fail("the server never came up");
    }

    private static bool SameFolder(string? one, string other) =>
        one is not null
        && string.Equals(
            Path.TrimEndingDirectorySeparator(Path.GetFullPath(one)),
            Path.TrimEndingDirectorySeparator(Path.GetFullPath(other)),
            StringComparison.OrdinalIgnoreCase
        );

    /// <summary>
    /// The body of the answer to a GET. <c>null</c> when the server does not answer, or
    /// answers that there is no such thing.
    /// </summary>
    public string? Get(string path)
    {
        try
        {
            using var answer = Http.GetAsync(Address + path).GetAwaiter().GetResult();
            return answer.IsSuccessStatusCode
                ? answer.Content.ReadAsStringAsync().GetAwaiter().GetResult()
                : null;
        }
        catch (HttpRequestException)
        {
            return null;
        }
    }

    /// <summary>
    /// Changes something on the server as another client does: the web frontend or a tool.
    /// True when the server took it.
    /// </summary>
    public bool Send(HttpMethod method, string path, string json)
    {
        using var request = new HttpRequestMessage(method, Address + path)
        {
            Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json"),
        };
        // A change names its client (ADR 015). This one is none of the shells of the test
        request.Headers.Add("X-Client-Id", AnotherClient);
        using var answer = Http.SendAsync(request).GetAwaiter().GetResult();
        return answer.IsSuccessStatusCode;
    }

    public void Dispose()
    {
        if (!process.HasExited)
        {
            process.Kill();
            process.WaitForExit();
        }
        process.Dispose();
        try
        {
            Directory.Delete(root, recursive: true);
        }
        // The files of a program that has just ended may still be held for a moment. The
        // folder is in the folder for temporary files
        catch (IOException) { }
        catch (UnauthorizedAccessException) { }
    }
}
