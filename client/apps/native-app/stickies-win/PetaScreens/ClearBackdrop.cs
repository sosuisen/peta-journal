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
internal sealed class ClearBackdrop(IntPtr window) : SystemBackdrop
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
        // The brush alone is not enough: DWM still draws the window on its own background.
        // A blur behind an empty region makes DWM keep the alpha of the window. This is how
        // WinUIEx makes its TransparentTintBackdrop
        Blur(enable: true);
        var noFrame = new Native.Margins();
        _ = Native.DwmExtendFrameIntoClientArea(window, ref noFrame);
        Wipe();
        target.SystemBackdrop = Compositor().CreateColorBrush(Color.FromArgb(0, 0xFF, 0xFF, 0xFF));
    }

    protected override void OnTargetDisconnected(ICompositionSupportsSystemBackdrop target)
    {
        base.OnTargetDisconnected(target);
        target.SystemBackdrop = null;
        Blur(enable: false);
    }

    /// <summary>
    /// Paints the background of the window black, when Windows asks for it. Black has no
    /// alpha in the background of a window: DWM shows what is behind it there.
    /// </summary>
    public void Erase(IntPtr deviceContext)
    {
        if (deviceContext == IntPtr.Zero || !Native.GetClientRect(window, out var rect))
            return;
        _ = Native.FillRect(deviceContext, ref rect, Native.GetStockObject(Native.BlackBrush));
    }

    /// <summary>
    /// Paints the whole window black, also its frame. Windows still draws the title bar of
    /// old in it when the window gets or loses the focus, or gets a new title. Through a
    /// window without paper that title would show under the name of the sticky.
    /// </summary>
    public void Wipe()
    {
        if (!Native.GetWindowRect(window, out var outer))
            return;
        var rect = new Native.Rect
        {
            Right = outer.Right - outer.Left,
            Bottom = outer.Bottom - outer.Top,
        };
        var deviceContext = Native.GetWindowDC(window);
        if (deviceContext == IntPtr.Zero)
            return;
        _ = Native.FillRect(deviceContext, ref rect, Native.GetStockObject(Native.BlackBrush));
        _ = Native.ReleaseDC(window, deviceContext);
    }

    private void Blur(bool enable)
    {
        // A region outside the window: nothing is blurred
        var region = Native.CreateRectRgn(-2, -2, -1, -1);
        var blur = new Native.BlurBehind
        {
            Flags = Native.BlurEnableInRegion,
            Enable = enable,
            Region = region,
        };
        _ = Native.DwmEnableBlurBehindWindow(window, ref blur);
        _ = Native.DeleteObject(region);
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
