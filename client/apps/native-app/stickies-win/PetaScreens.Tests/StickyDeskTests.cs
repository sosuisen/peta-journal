using Microsoft.UI.Windowing;
using Microsoft.VisualStudio.TestTools.UnitTesting.AppContainer;
using PetaBorder;
using Windows.System;

namespace PetaScreens.Tests;

/// <summary>
/// The stickies that are open, and the ones kept for the next run of the app. The keys and the
/// clicks go through Windows. These need a real journal server, so they are skipped without
/// <c>PETA_E2E_SERVER</c>.
/// </summary>
[TestClass]
public sealed class StickyDeskTests
{
    private const int Port = 18114;

    // 何も保存されていないときは、ページのない付箋紙が1つ開く。
    [UITestMethod]
    public async Task The_desk_opens_one_sticky_without_a_page_when_nothing_is_kept()
    {
        await using var screen = await Screen.Open(Port, url: null);

        Assert.HasCount(1, screen.Desk.Stickies);
        Assert.IsNull(screen.Shell.Url);
    }

    // Ctrl+Nは、ページのない付箋紙を足す。足した付箋紙は、前の付箋紙にぴったり重ならない。
    [UITestMethod]
    public async Task Control_N_adds_a_sticky_without_a_page_that_does_not_cover_the_one_before()
    {
        await using var screen = await Screen.Open(Port);

        Screen.Press(VirtualKey.N, Held.Control);

        await Screen.Until("a second sticky is open", () => screen.Desk.Stickies.Count == 2);
        var (first, second) = (screen.Desk.Stickies[0], screen.Desk.Stickies[1]);
        Assert.IsNull(second.Shell.Url);
        Assert.AreNotEqual(first.AppWindow.Position, second.AppWindow.Position);
        Assert.AreEqual(first.AppWindow.Size, second.AppWindow.Size);
    }

    // Ctrl+Nで足した付箋紙は、押した付箋紙の左上隅から少し右下に出る。最後に開いた付箋紙の右下ではない。
    [UITestMethod]
    public async Task Control_N_adds_a_sticky_a_little_down_and_right_of_the_sticky_it_was_pressed_on()
    {
        await using var screen = await Screen.Open(Port);
        Screen.Press(VirtualKey.N, Held.Control);
        await Screen.Until("a second sticky is open", () => screen.Desk.Stickies.Count == 2);
        screen.Window.AppWindow.Move(new(200, 200));
        screen.Desk.Stickies[1].AppWindow.Move(new(700, 400));
        await Task.Delay(100);
        screen.ToFront();

        Screen.Press(VirtualKey.N, Held.Control);

        await Screen.Until("a third sticky is open", () => screen.Desk.Stickies.Count == 3);
        var corner = screen.Desk.Stickies[2].AppWindow.Position;
        Assert.AreEqual(corner.X - 200, corner.Y - 200);
        Assert.IsInRange(1, 100, corner.X - 200);
    }

    // 付箋紙は、それぞれが自分のページを持つ。1つでページを開いても、ほかの付箋紙は変わらない。
    [UITestMethod]
    public async Task Each_sticky_has_a_page_of_its_own()
    {
        await using var screen = await Screen.Open(Port);
        Screen.Press(VirtualKey.N, Held.Control);
        await Screen.Until("a second sticky is open", () => screen.Desk.Stickies.Count == 2);
        var second = screen.Desk.Stickies[1];

        second.Shell.Open("/pages/second-sticky");

        await Screen.Until("the page is shown", () => second.Shell.Title == "second-sticky");
        await Screen.Until("the window has the name of the page", () =>
            second.Title == "second-sticky"
        );
        Assert.AreEqual("/pages/screen", screen.Shell.Url);
    }

    // タイトルバーの閉じるボタンで閉じた付箋紙は、なくなり、次の起動でも戻らない。ほかの付箋紙は残る。
    [UITestMethod]
    public async Task A_sticky_closed_with_its_button_is_gone_and_does_not_come_back()
    {
        await using var screen = await Screen.Open(Port);
        Screen.Press(VirtualKey.N, Held.Control);
        await Screen.Until("a second sticky is open", () => screen.Desk.Stickies.Count == 2);
        var (first, second) = (screen.Desk.Stickies[0], screen.Desk.Stickies[1]);

        await screen.Click(Screen.ButtonOf(second, "Close"), second);

        await Screen.Until("the second sticky is gone", () => screen.Desk.Stickies.Count == 1);
        screen.RunAgain();
        CollectionAssert.AreEqual(
            new[] { first.Id },
            screen.Desk.Stickies.Select(sticky => sticky.Id).ToArray()
        );
    }

    // Alt+F4で閉じた付箋紙は、次の起動で戻る。ユーザーが片付けたのではないためである。
    [UITestMethod]
    public async Task A_sticky_closed_with_Alt_F4_comes_back()
    {
        await using var screen = await Screen.Open(Port);
        var id = screen.Window.Id;

        Screen.Press(VirtualKey.F4, Held.Alt);

        await Screen.Until("the sticky is closed", () => screen.Desk.Stickies.Count == 0);
        screen.RunAgain();
        CollectionAssert.AreEqual(
            new[] { id },
            screen.Desk.Stickies.Select(sticky => sticky.Id).ToArray()
        );
    }

    // Ctrl+Qはアプリを終える。付箋紙は、ページと位置を持って、次の起動で戻る。
    [UITestMethod]
    public async Task Control_Q_ends_the_app_and_the_stickies_come_back_with_their_page_and_place()
    {
        await using var screen = await Screen.Open(Port);
        Screen.Press(VirtualKey.N, Held.Control);
        await Screen.Until("a second sticky is open", () => screen.Desk.Stickies.Count == 2);
        var ids = screen.Desk.Stickies.Select(sticky => sticky.Id).ToArray();
        screen.Window.AppWindow.Move(new(120, 130));
        await Task.Delay(100);
        screen.ToFront();

        Screen.Press(VirtualKey.Q, Held.Control);

        await Screen.Until("the stickies are closed", () => screen.Desk.Stickies.Count == 0);
        screen.RunAgain();
        CollectionAssert.AreEqual(ids, screen.Desk.Stickies.Select(sticky => sticky.Id).ToArray());
        var next = screen.Desk.Stickies;
        Assert.AreEqual(new Windows.Graphics.PointInt32(120, 130), next[0].AppWindow.Position);
        Assert.IsNull(next[1].Shell.Url);
        await Screen.Until("the page is shown again", () => next[0].Shell.Title == "screen");
    }

    // 最小化した付箋紙は、次の起動で、最小化する前の位置と大きさで戻る。
    // 最小化した窓を、Windowsは画面の外に置き、タイトルバーだけの大きさにするためである。
    [UITestMethod]
    public async Task A_minimized_sticky_comes_back_at_the_place_and_size_before_it_was_minimized()
    {
        await using var screen = await Screen.Open(Port);
        screen.Window.AppWindow.Move(new(120, 130));
        await Task.Delay(100);
        var size = screen.Window.AppWindow.Size;
        var presenter = (OverlappedPresenter)screen.Window.AppWindow.Presenter;

        presenter.Minimize();

        await Screen.Until("the sticky is minimized", () =>
            presenter.State == OverlappedPresenterState.Minimized
        );
        await Task.Delay(100);
        screen.RunAgain();
        var next = screen.Desk.Stickies[0];
        Assert.AreEqual(new Windows.Graphics.PointInt32(120, 130), next.AppWindow.Position);
        Assert.AreEqual(size, next.AppWindow.Size);
    }
}
