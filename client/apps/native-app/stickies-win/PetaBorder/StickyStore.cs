namespace PetaBorder;

/// <summary>
/// The stickies as a file keeps them, as <see cref="PetaCore.PetaSticky"/>. Every change is written at
/// once: the app may be ended at any time. The rules are in Rust, in <c>client-stickies</c>, for
/// the stickies of macOS and Windows. The place of a sticky is the top left corner of its
/// window, in the pixels of the screens as Windows counts them over all screens. Its size is in
/// pixels as they are at a scale of 100%: a sticky is as large on a screen with another scale.
/// </summary>
public sealed class StickyStore
{
    private readonly PetaCore.PetaStickyStore file;

    /// <summary>The file of the app, in the folder of the user for the data of apps.</summary>
    public static string StandardFile =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "PetaStickies",
            "stickies.json"
        );

    /// <summary>
    /// Reads the file. A file that is not there, or cannot be read, gives no stickies.
    /// </summary>
    public StickyStore(string file)
    {
        ArgumentNullException.ThrowIfNull(file);
        this.file = new PetaCore.PetaStickyStore(file);
    }

    public IReadOnlyList<PetaCore.PetaSticky> Stickies => file.Stickies();

    /// <summary>Adds the sticky, or changes the one with its id.</summary>
    public void Put(PetaCore.PetaSticky sticky) => file.Put(sticky);

    public void Remove(string id) => file.Remove(id);
}
