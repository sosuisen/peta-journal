using System.Runtime.InteropServices;
using PetaBorder;
using Windows.System;

namespace PetaScreens.Tests;

/// <summary>
/// Keys sent through Windows, as the user types them (ADR 059). They go to the window in front
/// and to the control in it with the focus. A test that sends them fails when somebody uses the
/// keyboard or the mouse at the same time.
/// </summary>
internal static class UserInput
{
    private const uint KeyUp = 0x0002;
    private const uint Unicode = 0x0004;
    private const uint ExtendedKey = 0x0001;

    /// <summary>
    /// The keys outside the number pad that share their codes with it. Sent without the mark of
    /// an extended key, an arrow is a key of the number pad. Windows then lets Shift go while
    /// Num Lock is on.
    /// </summary>
    private static readonly VirtualKey[] Extended =
    [
        VirtualKey.Up,
        VirtualKey.Down,
        VirtualKey.Left,
        VirtualKey.Right,
        VirtualKey.Home,
        VirtualKey.End,
        VirtualKey.PageUp,
        VirtualKey.PageDown,
        VirtualKey.Insert,
        VirtualKey.Delete,
    ];

    /// <summary>Presses the key with the keys held down with it, and lets them all go.</summary>
    public static void Press(VirtualKey key, Held held = Held.None)
    {
        var modifiers = Modifiers(held);
        foreach (var modifier in modifiers)
            Send(modifier, '\0', 0);
        var extended = Extended.Contains(key) ? ExtendedKey : 0;
        Send((ushort)key, '\0', extended);
        Send((ushort)key, '\0', extended | KeyUp);
        foreach (var modifier in modifiers.Reverse())
            Send(modifier, '\0', KeyUp);
    }

    /// <summary>Types the text, one letter after another, as the keyboard sends letters.</summary>
    public static void Type(string text)
    {
        foreach (var letter in text)
        {
            Send(0, letter, Unicode);
            Send(0, letter, Unicode | KeyUp);
        }
    }

    private static ushort[] Modifiers(Held held)
    {
        var keys = new List<ushort>();
        if (held.HasFlag(Held.Control))
            keys.Add((ushort)VirtualKey.Control);
        if (held.HasFlag(Held.Shift))
            keys.Add((ushort)VirtualKey.Shift);
        if (held.HasFlag(Held.Alt))
            keys.Add((ushort)VirtualKey.Menu);
        return [.. keys];
    }

    /// <summary>
    /// Clicks the left button at the place on the screen, in pixels, with the keys held down.
    /// The window sees each step before the next: the mouse comes, the button goes down, and
    /// it comes up. A click without these pauses is not seen as a tap. A right click shows a
    /// menu.
    /// </summary>
    public static async Task Click(int x, int y, Held held = Held.None, bool right = false)
    {
        if (!SetCursorPos(x, y))
            throw new InvalidOperationException("Windows did not move the mouse");
        await Task.Delay(Pause);
        var modifiers = Modifiers(held);
        foreach (var modifier in modifiers)
            Send(modifier, '\0', 0);
        Send(
            new Input
            {
                Type = 0,
                Mouse = new MouseInput { Flags = right ? RightDown : LeftDown },
            }
        );
        await Task.Delay(Pause);
        Send(
            new Input
            {
                Type = 0,
                Mouse = new MouseInput { Flags = right ? RightUp : LeftUp },
            }
        );
        foreach (var modifier in modifiers.Reverse())
            Send(modifier, '\0', KeyUp);
        await Task.Delay(Pause);
    }

    /// <summary>
    /// Clicks twice with the left button at the place on the screen. The four presses go to
    /// Windows at once: a pause after the first press would start a move of the window.
    /// </summary>
    public static async Task DoubleClick(int x, int y)
    {
        if (!SetCursorPos(x, y))
            throw new InvalidOperationException("Windows did not move the mouse");
        await Task.Delay(Pause);
        foreach (var flags in new[] { LeftDown, LeftUp, LeftDown, LeftUp })
            Send(
                new Input
                {
                    Type = 0,
                    Mouse = new MouseInput { Flags = flags },
                }
            );
        await Task.Delay(Pause);
    }

    /// <summary>
    /// Presses the left button at <paramref name="from"/>, moves the mouse to
    /// <paramref name="to"/> in steps, and lets the button go there. The window sees each
    /// step. <paramref name="held"/> runs while the button is still down.
    /// </summary>
    public static async Task Drag((int X, int Y) from, (int X, int Y) to, Func<Task>? held)
    {
        MoveTo(from.X, from.Y);
        await Task.Delay(Pause);
        Send(
            new Input
            {
                Type = 0,
                Mouse = new MouseInput { Flags = LeftDown },
            }
        );
        // A button that stays down after a failed test holds the mouse of the whole desktop
        try
        {
            await Task.Delay(Pause);
            const int steps = 10;
            for (var step = 1; step <= steps; step++)
            {
                MoveTo(
                    from.X + (to.X - from.X) * step / steps,
                    from.Y + (to.Y - from.Y) * step / steps
                );
                await Task.Delay(Pause);
            }
            if (held is not null)
                await held();
        }
        finally
        {
            Send(
                new Input
                {
                    Type = 0,
                    Mouse = new MouseInput { Flags = LeftUp },
                }
            );
        }
        await Task.Delay(Pause);
    }

    /// <summary>
    /// Moves the mouse to the place on the screen, as a mouse does. The window hears it as a
    /// move of the pointer, which a move by <c>SetCursorPos</c> is not.
    /// </summary>
    private static void MoveTo(int x, int y)
    {
        // An absolute move counts from 0 to 65535 across the screens
        var left = GetSystemMetrics(VirtualLeft);
        var top = GetSystemMetrics(VirtualTop);
        var width = GetSystemMetrics(VirtualWidth);
        var height = GetSystemMetrics(VirtualHeight);
        Send(
            new Input
            {
                Type = 0,
                Mouse = new MouseInput
                {
                    X = (int)Math.Round((x - left) * 65535.0 / (width - 1)),
                    Y = (int)Math.Round((y - top) * 65535.0 / (height - 1)),
                    Flags = Move | Absolute | VirtualDesk,
                },
            }
        );
    }

    private const int VirtualLeft = 76;
    private const int VirtualTop = 77;
    private const int VirtualWidth = 78;
    private const int VirtualHeight = 79;

    private const uint Absolute = 0x8000;
    private const uint VirtualDesk = 0x4000;

    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int index);

    /// <summary>
    /// A move of the mouse by nothing. Windows lets the process that sent the last input put
    /// its window in front, and this input changes nothing on the screen.
    /// </summary>
    public static void Nudge() =>
        Send(
            new Input
            {
                Type = 0,
                Mouse = new MouseInput { Flags = Move },
            }
        );

    private const int Pause = 30;

    private const uint Move = 0x0001;

    private const uint LeftDown = 0x0002;
    private const uint LeftUp = 0x0004;
    private const uint RightDown = 0x0008;
    private const uint RightUp = 0x0010;

    private static void Send(ushort key, char letter, uint flags) =>
        Send(
            new Input
            {
                Type = 1,
                Keyboard = new KeyboardInput
                {
                    Key = key,
                    Scan = letter,
                    Flags = flags,
                },
            }
        );

    private static void Send(Input input)
    {
        if (SendInput(1, [input], Marshal.SizeOf<Input>()) != 1)
            throw new InvalidOperationException(
                $"Windows took no input: error {Marshal.GetLastPInvokeError()}"
            );
    }

    /// <summary>The INPUT of Win32. Its union is 32 bytes on x64, as large as the mouse input.</summary>
    [StructLayout(LayoutKind.Explicit, Size = 40)]
    private struct Input
    {
        [FieldOffset(0)]
        public uint Type;

        [FieldOffset(8)]
        public KeyboardInput Keyboard;

        [FieldOffset(8)]
        public MouseInput Mouse;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MouseInput
    {
        public int X;
        public int Y;
        public uint Data;
        public uint Flags;
        public uint Time;
        public IntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct KeyboardInput
    {
        public ushort Key;
        public ushort Scan;
        public uint Flags;
        public uint Time;
        public IntPtr ExtraInfo;
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint count, [In] Input[] inputs, int size);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetCursorPos(int x, int y);
}
