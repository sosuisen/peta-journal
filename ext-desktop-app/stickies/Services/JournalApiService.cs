using System.Collections.Concurrent;
using System.Net.WebSockets;
using Google.Protobuf;
using Petajournal;

namespace Stickies.Services;

public class JournalApiService : IDisposable
{
    private ClientWebSocket? _ws;
    private CancellationTokenSource? _cts;
    private readonly ConcurrentDictionary<string, TaskCompletionSource<ApiResponse>> _pending = new();
    private int _requestId;
    private readonly string _host;
    private readonly int _port;

    public event Action? Connected;
    public event Action? Disconnected;
    public event Action<string>? PageUpdated;

    public string? SessionId { get; private set; }

    public JournalApiService(string host = "localhost", int port = 8080)
    {
        _host = host;
        _port = port;
    }

    public bool IsConnected => _ws?.State == WebSocketState.Open;

    public async Task ConnectAsync()
    {
        _ws = new ClientWebSocket();
        _cts = new CancellationTokenSource();
        var uri = new Uri($"ws://{_host}:{_port}/ws");
        await _ws.ConnectAsync(uri, _cts.Token);
        _ = Task.Run(() => ReceiveLoop(_cts.Token));
        Connected?.Invoke();
    }

    public async Task DisconnectAsync()
    {
        if (_ws?.State == WebSocketState.Open)
        {
            await _ws.CloseAsync(WebSocketCloseStatus.NormalClosure, "", CancellationToken.None);
        }
        _cts?.Cancel();
        Disconnected?.Invoke();
    }

    private async Task<ApiResponse> SendAsync(ApiRequest request)
    {
        if (_ws?.State != WebSocketState.Open)
            throw new InvalidOperationException("WebSocket is not connected");

        var id = Interlocked.Increment(ref _requestId).ToString();
        request.RequestId = id;

        var tcs = new TaskCompletionSource<ApiResponse>();
        _pending[id] = tcs;

        var bytes = request.ToByteArray();
        await _ws.SendAsync(bytes, WebSocketMessageType.Binary, true, _cts!.Token);

        using var timeoutCts = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        using var reg = timeoutCts.Token.Register(() => tcs.TrySetException(new TimeoutException("Request timed out")));

        return await tcs.Task;
    }

    private async Task ReceiveLoop(CancellationToken ct)
    {
        var buffer = new byte[64 * 1024];
        try
        {
            while (!ct.IsCancellationRequested && _ws?.State == WebSocketState.Open)
            {
                using var ms = new MemoryStream();
                WebSocketReceiveResult result;
                do
                {
                    result = await _ws.ReceiveAsync(buffer, ct);
                    ms.Write(buffer, 0, result.Count);
                } while (!result.EndOfMessage);

                if (result.MessageType == WebSocketMessageType.Close)
                    break;

                var response = ApiResponse.Parser.ParseFrom(ms.ToArray());
                if (response.PayloadCase == ApiResponse.PayloadOneofCase.ServerEvent)
                {
                    HandleServerEvent(response.ServerEvent);
                }
                else if (_pending.TryRemove(response.RequestId, out var tcs))
                {
                    tcs.TrySetResult(response);
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (WebSocketException) { }
        finally
        {
            // Reject all pending requests
            foreach (var kvp in _pending)
            {
                if (_pending.TryRemove(kvp.Key, out var tcs))
                {
                    tcs.TrySetException(new Exception("WebSocket disconnected"));
                }
            }
            Disconnected?.Invoke();
        }
    }

    private void HandleServerEvent(ServerEvent serverEvent)
    {
        switch (serverEvent.EventCase)
        {
            case ServerEvent.EventOneofCase.Connected:
                SessionId = serverEvent.Connected.SessionId;
                break;
            case ServerEvent.EventOneofCase.PagesUpdated:
                PageUpdated?.Invoke(serverEvent.PagesUpdated.PageId);
                break;
        }
    }

    public async Task<SearchPageNamesResponse?> SearchPageNamesAsync(string query, int maxResults = 20)
    {
        var request = new ApiRequest
        {
            SearchPageNames = new SearchPageNamesRequest
            {
                Query = query,
                MaxResults = maxResults
            }
        };
        var response = await SendAsync(request);
        return response.Success ? response.SearchPageNames : null;
    }

    public async Task<GetPageResponse?> GetPageAsync(string pageId)
    {
        var request = new ApiRequest
        {
            GetPage = new GetPageRequest { PageId = pageId }
        };
        var response = await SendAsync(request);
        return response.Success ? response.GetPage : null;
    }

    public void Dispose()
    {
        _cts?.Cancel();
        _ws?.Dispose();
        _cts?.Dispose();
    }
}
