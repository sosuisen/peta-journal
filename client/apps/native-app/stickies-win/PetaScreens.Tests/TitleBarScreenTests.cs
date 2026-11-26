using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.VisualStudio.TestTools.UnitTesting.AppContainer;
using PetaBorder;
using Windows.System;

namespace PetaScreens.Tests;

/// <summary>
/// The title bar of a sticky: its clicks, its menu and its buttons, sent through Windows.
/// These need a real journal server, so they are skipped without <c>PETA_E2E_SERVER</c>.
/// </summary>
[TestClass]
public sealed class TitleBarScreenTests
{
    private const int Port = 18117;

    // タイトルバーをダブルクリックすると、付箋紙はタイトルバーまで巻き上がる。もう一度で、元の高さに戻る。
    [UITestMethod]
    public async Task A_double_click_on_the_title_bar_rolls_the_sticky_up_and_down()
    {
        await using var screen = await Screen.Open(Port);
        var height = screen.Window.AppWindow.Size.Height;

        await screen.ClickTitleBar(twice: true);

        await Screen.Until("the sticky is rolled up to its title bar", () =>
            screen.Window.RolledUp && screen.Window.AppWindow.ClientSize.Height < height / 4
        );

        await screen.ClickTitleBar(twice: true);

        await Screen.Until("the sticky is as high as before", () =>
            !screen.Window.RolledUp && screen.Window.AppWindow.Size.Height == height
        );
    }

    // 最大化のボタンで画面いっぱいになり、もう一度押すと元に戻る。
    [UITestMethod]
    public async Task The_maximize_button_makes_the_sticky_as_large_as_the_screen_and_back()
    {
        await using var screen = await Screen.Open(Port);
        var presenter = (OverlappedPresenter)screen.Window.AppWindow.Presenter;

        await screen.Click(Screen.ButtonOf(screen.Window, "Maximize"));

        await Screen.Until("the sticky is maximized", () =>
            presenter.State == OverlappedPresenterState.Maximized
        );

        await screen.Click(Screen.ButtonOf(screen.Window, "Maximize"));

        await Screen.Until("the sticky is restored", () =>
            presenter.State == OverlappedPresenterState.Restored
        );
    }

    // 画面いっぱいの付箋紙は、タイトルバーのダブルクリックで元に戻る。巻き上がりはしない。
    [UITestMethod]
    public async Task A_double_click_on_the_title_bar_of_a_maximized_sticky_restores_it()
    {
        await using var screen = await Screen.Open(Port);
        var presenter = (OverlappedPresenter)screen.Window.AppWindow.Presenter;
        presenter.Maximize();
        await Screen.Until("the sticky is maximized", () =>
            presenter.State == OverlappedPresenterState.Maximized
        );

        await screen.ClickTitleBar(twice: true);

        await Screen.Until("the sticky is restored", () =>
            presenter.State == OverlappedPresenterState.Restored
        );
        Assert.IsFalse(screen.Window.RolledUp);
    }

    // 最小化のボタンで、付箋紙は最小化する。
    [UITestMethod]
    public async Task The_minimize_button_minimizes_the_sticky()
    {
        await using var screen = await Screen.Open(Port);
        var presenter = (OverlappedPresenter)screen.Window.AppWindow.Presenter;

        await screen.Click(Screen.ButtonOf(screen.Window, "Minimize"));

        await Screen.Until("the sticky is minimized", () =>
            presenter.State == OverlappedPresenterState.Minimized
        );
    }

    // タイトルバーの右クリックでメニューが出る。「Keep on top」で、付箋紙はほかのアプリの窓より上に留まる。
    [UITestMethod]
    public async Task Keep_on_top_in_the_menu_of_the_title_bar_keeps_the_sticky_above_other_windows()
    {
        await using var screen = await Screen.Open(Port);
        var presenter = (OverlappedPresenter)screen.Window.AppWindow.Presenter;

        await screen.ClickTitleBar(right: true);
        var entry = await screen.MenuEntry("Keep on top");
        // Why the marks: the test fails now and then in the CI, and never on a developer's PC.
        // They tell which part of the click the entry saw, and when
        var clock = System.Diagnostics.Stopwatch.StartNew();
        var seen = new List<string>();
        void Mark(string what) => seen.Add($"{what}@{clock.ElapsedMilliseconds}ms");
        entry.AddHandler(
            UIElement.PointerPressedEvent,
            new PointerEventHandler((_, _) => Mark("pressed")),
            true
        );
        entry.AddHandler(
            UIElement.PointerReleasedEvent,
            new PointerEventHandler((_, _) => Mark("released")),
            true
        );
        entry.Click += (_, _) => Mark("click");
        await screen.Click(entry);
        Mark("sent");

        await Screen.Until(
            "the sticky stays on top",
            () => screen.Window.Look.AlwaysOnTop && presenter.IsAlwaysOnTop,
            () =>
                $"entry saw=[{string.Join(", ", seen)}] checked={(entry as ToggleMenuFlyoutItem)?.IsChecked} "
                + $"look on top={screen.Window.Look.AlwaysOnTop} presenter on top={presenter.IsAlwaysOnTop} "
                + $"menus open={VisualTreeHelper.GetOpenPopupsForXamlRoot(screen.Window.Content.XamlRoot).Count}"
        );
    }

    // メニューで色を選ぶと、付箋紙の紙がその色になる。
    [UITestMethod]
    public async Task A_color_chosen_in_the_menu_of_the_title_bar_is_the_color_of_the_paper()
    {
        await using var screen = await Screen.Open(Port);
        var (name, hex) = StickyLook.Colors.First(color => color.Hex != screen.Window.Look.Color);

        await screen.ClickTitleBar(right: true);
        await screen.Click(await screen.MenuEntry(name));

        await Screen.Until("the sticky has the color", () => screen.Window.Look.Color == hex);
    }

    // 紙のない付箋紙は、ほかの付箋紙で作業しているあいだ、書いたものだけを見せる。クリックで戻る。
    [UITestMethod]
    public async Task A_sticky_without_paper_shows_only_what_is_written_while_another_is_worked_on()
    {
        await using var screen = await Screen.Open(Port);
        var (name, _) = StickyLook.Colors[^1];
        await screen.ClickTitleBar(right: true);
        await screen.Click(await screen.MenuEntry(name));
        await Screen.Until("the sticky has no paper", () => screen.Window.Look.Transparent);
        Assert.IsFalse(screen.Window.Bare);

        Screen.Press(VirtualKey.N, Held.Control);

        await Screen.Until("the sticky shows only what is written", () =>
            screen.Desk.Stickies.Count == 2 && screen.Window.Bare
        );

        await screen.Click(screen.Items()[0]);

        await Screen.Until("the sticky shows its title bar again", () => !screen.Window.Bare);
    }
}
