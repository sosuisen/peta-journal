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
