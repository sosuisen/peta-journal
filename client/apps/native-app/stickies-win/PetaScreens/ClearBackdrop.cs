using System.Runtime.InteropServices;
using Microsoft.UI.Composition;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

namespace PetaScreens;

/// <summary>
/// What is behind a window with no paper: nothing, so the windows behind it show through.
/// WinUI has no such backdrop. Mica and acrylic blur what is behind them.
/// </summary>
internal sealed class ClearBackdrop : SystemBackdrop
{
    /// <summary>
    /// The compositor of the system, which draws the backdrops. It is not the compositor of
    /// WinUI: the window takes only a brush of the system.
    /// </summary>
    private static Windows.UI.Composition.Compositor? compositor;

    /// <summary>The queue of work that the compositor of the system needs on its thread.</summary>
    private static IntPtr queue;

    protected override void OnTargetConnected(
        ICompositionSupportsSystemBackdrop target,
        XamlRoot root
    )
    {
        base.OnTargetConnected(target, root);
        target.SystemBackdrop = Compositor().CreateColorBrush(Color.FromArgb(0, 0xFF, 0xFF, 0xFF));
    }

    protected override void OnTargetDisconnected(ICompositionSupportsSystemBackdrop target)
    {
        base.OnTargetDisconnected(target);
        target.SystemBackdrop = null;
    }

    private static Windows.UI.Composition.Compositor Compositor()
    {
        if (compositor is not null)
            return compositor;
        // WinUI makes a queue of its own on the thread, not one of the system
        if (Windows.System.DispatcherQueue.GetForCurrentThread() is null && queue == IntPtr.Zero)
        {
            var options = new Native.DispatcherQueueOptions
            {
                Size = Marshal.SizeOf<Native.DispatcherQueueOptions>(),
                ThreadType = Native.QueueOfThisThread,
                ApartmentType = Native.SingleThreadedApartment,
            };
            Marshal.ThrowExceptionForHR(Native.CreateDispatcherQueueController(options, out queue));
        }
        compositor = new Windows.UI.Composition.Compositor();
        return compositor;
    }
}
