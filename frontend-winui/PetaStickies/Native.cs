using System.Runtime.InteropServices;
using Microsoft.UI.Input;
using PetaBorder;
using Windows.System;
using Windows.UI.Core;

namespace PetaStickies;

/// <summary>What WinUI does not offer, asked of Windows itself.</summary>
internal static class Native
{
    public const uint WmGetMinMaxInfo = 0x0024;
    public const uint WmNcLeftButtonDoubleClick = 0x00A3;
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

    /// <summary>The key as the outline knows it, or nothing for a key it does not know.</summary>
    public static Key? KeyOf(VirtualKey key) =>
        key switch
        {
            VirtualKey.Enter => Key.Enter,
            VirtualKey.Tab => Key.Tab,
            VirtualKey.Up => Key.Up,
            VirtualKey.Down => Key.Down,
            VirtualKey.Left => Key.Left,
            VirtualKey.Right => Key.Right,
            VirtualKey.Back => Key.Backspace,
            VirtualKey.Delete => Key.Delete,
            VirtualKey.Escape => Key.Escape,
            VirtualKey.C => Key.C,
            VirtualKey.X => Key.X,
            VirtualKey.Z => Key.Z,
            _ => null,
        };
}
