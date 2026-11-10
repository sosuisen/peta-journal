using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;
using Stickies.Models;

namespace Stickies.Services;

public class JournalApiService : IDisposable
{
    private readonly HttpClient _http;
    private CancellationTokenSource? _sseCts;
    private Task? _sseTask;
    private CancellationTokenSource? _reconnectCts;
    private readonly string _baseUrl;
    private bool _intentionalDisconnect;
    private bool _isConnected;

    private static readonly TimeSpan InitialDelay = TimeSpan.FromSeconds(1);
    private static readonly TimeSpan MaxDelay = TimeSpan.FromSeconds(30);

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    public event Action? Connected;
    public event Action? Disconnected;
    public event Action? Reconnecting;
    public event Action<string>? PageUpdated;

    public JournalApiService(string host = "localhost", int port = 8087)
    {
        _baseUrl = $"http://{host}:{port}";
        _http = new HttpClient { BaseAddress = new Uri(_baseUrl) };
    }

    public bool IsConnected => _isConnected;

    /// <summary>The address of the journal server, without a slash at the end.</summary>
    public string BaseUrl => _baseUrl;

    public async Task ConnectAsync()
    {
        _intentionalDisconnect = false;
        _reconnectCts?.Cancel();
        _reconnectCts = new CancellationTokenSource();
        await ConnectSseWithRetryAsync(_reconnectCts.Token);
    }

    private async Task ConnectSseWithRetryAsync(CancellationToken ct)
    {
        var delay = InitialDelay;
        while (!ct.IsCancellationRequested)
        {
            try
            {
                _sseCts?.Cancel();
                if (_sseTask != null)
                {
                    try
                    {
                        await _sseTask;
                    }
                    catch { }
                    _sseTask = null;
                }
                _sseCts = new CancellationTokenSource();

                var connectedTcs = new TaskCompletionSource(
                    TaskCreationOptions.RunContinuationsAsynchronously
                );

                // Start SSE listener in background — it will signal connectedTcs when stream is established
                _sseTask = Task.Run(
                    () => SseListenLoop(_sseCts.Token, connectedTcs),
                    _sseCts.Token
                );

                // Wait until SSE stream is actually established (like EventSource.onopen)
                await connectedTcs.Task;
                return;
            }
            catch when (!ct.IsCancellationRequested)
            {
                _isConnected = false;
                Reconnecting?.Invoke();
                try
                {
                    await Task.Delay(delay, ct);
                }
                catch (OperationCanceledException)
                {
                    return;
                }
                delay = TimeSpan.FromSeconds(
                    Math.Min(delay.TotalSeconds * 2, MaxDelay.TotalSeconds)
                );
            }
        }
    }

    private async Task SseListenLoop(
        CancellationToken ct,
        TaskCompletionSource? connectedTcs = null
    )
    {
        var wasConnected = false;
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, "/api/events");
            request.Headers.Accept.Add(
                new System.Net.Http.Headers.MediaTypeWithQualityHeaderValue("text/event-stream")
            );

            using var response = await _http.SendAsync(
                request,
                HttpCompletionOption.ResponseHeadersRead,
                ct
            );
            response.EnsureSuccessStatusCode();

            // SSE stream established — fire Connected (equivalent to EventSource.onopen)
            _isConnected = true;
            wasConnected = true;
            Connected?.Invoke();
            connectedTcs?.TrySetResult();

            using var stream = await response.Content.ReadAsStreamAsync(ct);
            using var reader = new StreamReader(stream);

            string? data = null;

            while (!ct.IsCancellationRequested)
            {
                var line = await reader.ReadLineAsync(ct);
                if (line == null)
                    break; // stream ended

                if (line.StartsWith("data:"))
                {
                    data = line["data:".Length..].Trim();
                }
                else if (line.Length == 0 && data != null)
                {
                    // End of event — extract type from JSON data
                    HandleSseData(data);
                    data = null;
                }
            }
        }
        catch (OperationCanceledException)
        {
            connectedTcs?.TrySetCanceled();
        }
        catch (Exception ex)
        {
            connectedTcs?.TrySetException(ex);
        }
        finally
        {
            _isConnected = false;
            Disconnected?.Invoke();

            // Only auto-reconnect if the stream was previously established then dropped.
            // If connection was never established, ConnectSseWithRetryAsync's retry loop handles it.
            if (
                wasConnected
                && !_intentionalDisconnect
                && _reconnectCts is { IsCancellationRequested: false }
            )
            {
                _ = Task.Run(() => ConnectSseWithRetryAsync(_reconnectCts.Token));
            }
        }
    }

    private void HandleSseData(string data)
    {
        try
        {
            var json = JsonSerializer.Deserialize<JsonElement>(data);
            if (!json.TryGetProperty("type", out var typeEl))
                return;
            var eventType = typeEl.GetString();

            if (eventType == "pagesUpdated" && json.TryGetProperty("pageId", out var pageIdEl))
            {
                PageUpdated?.Invoke(pageIdEl.GetString() ?? "");
            }
        }
        catch { }
    }

    public async Task<SearchPageNamesResponse?> SearchPageNamesAsync(
        string query,
        int maxResults = 20
    )
    {
        var url = $"/api/search/pages?q={Uri.EscapeDataString(query)}&maxResults={maxResults}";
        var response = await _http.GetFromJsonAsync<SearchPageNamesResponse>(url, JsonOptions);
        return response;
    }

    public async Task<GetPageByIdResponse?> GetPageByIdAsync(string pageId)
    {
        var url = $"/api/pages/{pageId}?fields={ItemModel.Fields}";
        var response = await _http.GetFromJsonAsync<GetPageByIdResponse>(url, JsonOptions);
        return response;
    }

    /// <summary>
    /// The folder of the tag pages. It is empty for the pages root and ends with a slash otherwise.
    /// </summary>
    public async Task<string> GetTagsDirAsync()
    {
        var config = await _http.GetFromJsonAsync<ConfigResponse>("/api/config", JsonOptions);
        return config?.TagsDir ?? "";
    }

    public void Dispose()
    {
        _intentionalDisconnect = true;
        _reconnectCts?.Cancel();
        _reconnectCts?.Dispose();
        _sseCts?.Cancel();
        _sseCts?.Dispose();
        _http.Dispose();
    }
}
