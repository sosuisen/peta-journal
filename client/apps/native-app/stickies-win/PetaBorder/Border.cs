using PetaTypes;

namespace PetaBorder;

/// <summary>
/// The core behind its border of bytes (ADR 024). Events and answers go in and requests come
/// out, as the generated types. The bytes are bincode.
/// </summary>
public sealed class Border : IDisposable
{
    private readonly PetaCore.PetaCore core;

    /// <summary>
    /// Starts the core for the journal server at <paramref name="server"/>, such as
    /// <c>http://localhost:8080</c>. <paramref name="onRequests"/> gets what the core asks of
    /// the shell. The core may call it from any thread, and for the first time before this
    /// returns.
    /// </summary>
    public Border(string server, Action<IReadOnlyList<Request>> onRequests)
    {
        ArgumentNullException.ThrowIfNull(server);
        ArgumentNullException.ThrowIfNull(onRequests);
        core = new PetaCore.PetaCore(server, new Listener(onRequests));
    }

    /// <summary>Opens what the URL names, such as <c>/pages/notes</c> or <c>/today</c>.</summary>
    /// <exception cref="PetaCore.BoltException">The URL names nothing.</exception>
    public void Open(string url) => core.Open(url);

    /// <summary>Sends an event of the shell.</summary>
    public void Send(Event sent)
    {
        ArgumentNullException.ThrowIfNull(sent);
        core.Update(sent.BincodeSerialize());
    }

    /// <summary>
    /// Hands a key to the core, which decides what it means (ADR 052). False when it means
    /// nothing to the outline there.
    /// </summary>
    public bool Press(KeyPress press)
    {
        ArgumentNullException.ThrowIfNull(press);
        return core.Press(press.BincodeSerialize());
    }

    /// <summary>
    /// Hands a change of the Markdown of an item in its text field to the core, which adds the
    /// time.
    /// </summary>
    public void Typed(Typing typing)
    {
        ArgumentNullException.ThrowIfNull(typing);
        core.Typed(typing.BincodeSerialize());
    }

    /// <summary>
    /// Hands a step of a drag of the mouse to the core. True when it told the core something.
    /// </summary>
    public bool Drag(DragMove step)
    {
        ArgumentNullException.ThrowIfNull(step);
        return core.Drag(step.BincodeSerialize());
    }

    /// <summary>
    /// Answers the request with the number <paramref name="id"/>. A request that only tells the
    /// shell something, such as a render, has no answer.
    /// </summary>
    /// <exception cref="PetaCore.BoltException">The core does not wait for this answer.</exception>
    public void Answer(uint id, UiOutput output)
    {
        ArgumentNullException.ThrowIfNull(output);
        core.Resolve(id, output.BincodeSerialize());
    }

    /// <summary>The whole view model, to draw after a render request.</summary>
    public ViewModel View() => ViewModel.BincodeDeserialize(core.View());

    public void Dispose() => core.Dispose();

    /// <summary>Hears the core and reads its bytes.</summary>
    private sealed class Listener(Action<IReadOnlyList<Request>> onRequests) : PetaCore.PetaShell
    {
        // The types here are generated from the Rust that wrote the bytes. Bytes that cannot be
        // read are a build that mixes two versions, and the exception ends the app
        public void ProcessEffects(byte[] bytes) =>
            onRequests(Requests.BincodeDeserialize(bytes).Value);
    }
}
