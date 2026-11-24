using Microsoft.UI.Xaml;
using PetaBorder;
using Windows.ApplicationModel.DataTransfer;

namespace PetaScreens;

// The files dropped from the explorer. They are left out of the coverage, as the developer
// agreed: SendInput cannot start the drag of files of another app. See PetaBorder.Tests.csproj.
internal sealed partial class RowView
{
    /// <summary>Files dragged over the item may be dropped: the core takes the first image.
    /// A drag of text is left to the text box.</summary>
    private void DraggedOver(object sender, DragEventArgs e)
    {
        if (!e.DataView.Contains(StandardDataFormats.StorageItems))
            return;
        e.AcceptedOperation = DataPackageOperation.Copy;
        e.Handled = true;
    }

    /// <summary>Files were dropped on the drawn item: an image goes at the end of its body.</summary>
    private async void DroppedOnPaper(object sender, DragEventArgs e)
    {
        if (model is not { } row || await DroppedFiles(e) is not { } paths)
            return;
        Shell.DropImage(row.Id, paths);
    }

    /// <summary>Files were dropped on the editor: an image goes in where they were dropped.</summary>
    private async void DroppedOnEditor(object sender, DragEventArgs e)
    {
        if (model is not { } row || !e.DataView.Contains(StandardDataFormats.StorageItems))
            return;
        // The place is read before the files: the text may change while they are read
        var point = e.GetPosition(editor);
        var place = (ulong)DropPlace.Nearest(Carets(), point.X, point.Y);
        var state = State();
        state.SelectionStart = place;
        state.SelectionEnd = place;
        if (await DroppedFiles(e) is not { } paths)
            return;
        Shell.DropImage(row.Id, state, paths);
    }

    /// <summary>
    /// The paths of the dropped files. Null when no file was dropped, as for a drop of text.
    /// A file with no path, as one of a phone, is left out.
    /// </summary>
    private static async Task<List<string>?> DroppedFiles(DragEventArgs e)
    {
        if (!e.DataView.Contains(StandardDataFormats.StorageItems))
            return null;
        e.Handled = true;
        var deferral = e.GetDeferral();
        try
        {
            var items = await e.DataView.GetStorageItemsAsync();
            return [.. items.Select(item => item.Path).Where(path => path.Length > 0)];
        }
        finally
        {
            deferral.Complete();
        }
    }
}
