using System.Runtime.InteropServices;
using Microsoft.UI.Input;
using PetaBorder;
using PetaTypes;
using Windows.System;
using Windows.UI.Core;

namespace PetaScreens;

/// <summary>What WinUI does not offer, asked of Windows itself.</summary>
internal static class Native
{
    public const uint WmGetMinMaxInfo = 0x0024;
    public const uint WmNcLeftButtonDoubleClick = 0x00A3;
    public const uint WmNcRightButtonUp = 0x00A5;
    public const uint WmSysCommand = 0x0112;
    public const long SysMaximize = 0xF030;
    public const long SysClose = 0xF060;
    public const int HitCaption = 2;

    public delegate IntPtr SubclassProc(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam,
        UIntPtr id,
        UIntPtr data
    );

    [StructLayout(LayoutKind.Sequential)]
    public struct Point
    {
        public int X;
        public int Y;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct MinMaxInfo
    {
        public Point Reserved;
        public Point MaxSize;
        public Point MaxPosition;
        public Point MinTrackSize;
        public Point MaxTrackSize;
    }

    [DllImport("user32.dll")]
    public static extern uint GetDpiForWindow(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsZoomed(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsIconic(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool ScreenToClient(IntPtr window, ref Point point);

    [DllImport("comctl32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetWindowSubclass(
        IntPtr window,
        SubclassProc proc,
        UIntPtr id,
        UIntPtr data
    );

    [DllImport("comctl32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool RemoveWindowSubclass(IntPtr window, SubclassProc proc, UIntPtr id);

    /// <summary>What a queue of work of the system is made for: the thread that makes it.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct DispatcherQueueOptions
    {
        public int Size;
        public int ThreadType;
        public int ApartmentType;
    }

    public const int QueueOfThisThread = 2;
    public const int SingleThreadedApartment = 2;

    [DllImport("CoreMessaging.dll")]
    public static extern int CreateDispatcherQueueController(
        DispatcherQueueOptions options,
        out IntPtr controller
    );

    public const uint WmEraseBackground = 0x0014;
    public const uint WmSetText = 0x000C;
    public const uint WmNcPaint = 0x0085;
    public const uint WmNcActivate = 0x0086;

    /// <summary>The color of the line around a window, from Windows 11 on.</summary>
    public const int BorderColor = 34;
    public const uint NoColor = 0xFFFFFFFE;
    public const uint UsualColor = 0xFFFFFFFF;

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetWindowRect(IntPtr window, out Rect rect);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool ClientToScreen(IntPtr window, ref Point point);

    /// <summary>Cuts the window to the region. Windows owns the region from then on.</summary>
    [DllImport("user32.dll")]
    public static extern int SetWindowRgn(
        IntPtr window,
        IntPtr region,
        [MarshalAs(UnmanagedType.Bool)] bool redraw
    );

    [DllImport("dwmapi.dll")]
    public static extern int DwmSetWindowAttribute(
        IntPtr window,
        int attribute,
        ref uint value,
        int size
    );

    /// <summary>The blur behind a window: on, and only in the region that is given.</summary>
    public const uint BlurEnableInRegion = 0x1 | 0x2;

    public const int BlackBrush = 4;

    [StructLayout(LayoutKind.Sequential)]
    public struct BlurBehind
    {
        public uint Flags;

        [MarshalAs(UnmanagedType.Bool)]
        public bool Enable;
        public IntPtr Region;

        [MarshalAs(UnmanagedType.Bool)]
        public bool TransitionOnMaximized;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Margins
    {
        public int Left;
        public int Right;
        public int Top;
        public int Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Rect
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("dwmapi.dll")]
    public static extern int DwmEnableBlurBehindWindow(IntPtr window, ref BlurBehind blur);

    [DllImport("dwmapi.dll")]
    public static extern int DwmExtendFrameIntoClientArea(IntPtr window, ref Margins margins);

    [DllImport("gdi32.dll")]
    public static extern IntPtr CreateRectRgn(int left, int top, int right, int bottom);

    [DllImport("gdi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool DeleteObject(IntPtr gdiObject);

    [DllImport("gdi32.dll")]
    public static extern IntPtr GetStockObject(int kind);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetClientRect(IntPtr window, out Rect rect);

    [DllImport("user32.dll")]
    public static extern IntPtr GetWindowDC(IntPtr window);

    [DllImport("user32.dll")]
    public static extern int ReleaseDC(IntPtr window, IntPtr deviceContext);

    [DllImport("user32.dll")]
    public static extern int FillRect(IntPtr deviceContext, ref Rect rect, IntPtr brush);

    [DllImport("comctl32.dll")]
    public static extern IntPtr DefSubclassProc(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam
    );
}

/// <summary>The keys as WinUI tells them, in the words of the border.</summary>
internal static class Input
{
    /// <summary>The key that Windows names when the input method took the key.</summary>
    public const VirtualKey InputMethod = (VirtualKey)229;
    public const VirtualKey Plus = (VirtualKey)187;
    public const VirtualKey Minus = (VirtualKey)189;

    public static Held Held()
    {
        var held = PetaBorder.Held.None;
        if (IsDown(VirtualKey.Shift))
            held |= PetaBorder.Held.Shift;
        if (IsDown(VirtualKey.Control))
            held |= PetaBorder.Held.Control;
        if (IsDown(VirtualKey.Menu))
            held |= PetaBorder.Held.Alt;
        return held;
    }

    private static bool IsDown(VirtualKey key) =>
        InputKeyboardSource.GetKeyStateForCurrentThread(key).HasFlag(CoreVirtualKeyStates.Down);

    /// <summary>
    /// The key with the keys held down with it, as the core knows it (ADR 052), or nothing for
    /// a key it does not know.
    /// </summary>
    public static Chord? ChordOf(VirtualKey key, Held held)
    {
        Key? known = key switch
        {
            VirtualKey.Enter => new Key.Enter(),
            VirtualKey.Tab => new Key.Tab(),
            VirtualKey.Up => new Key.Up(),
            VirtualKey.Down => new Key.Down(),
            VirtualKey.Left => new Key.Left(),
            VirtualKey.Right => new Key.Right(),
            VirtualKey.Back => new Key.Backspace(),
            VirtualKey.Delete => new Key.Delete(),
            VirtualKey.Escape => new Key.Escape(),
            >= VirtualKey.A and <= VirtualKey.Z => new Key.Letter(
                ((char)('a' + (key - VirtualKey.A))).ToString()
            ),
            _ => null,
        };
        return known is null ? null : Keyboard.Chord(known, held);
    }
}
