namespace PetaBorder.Tests;

public sealed class DayChangeTests
{
    private const uint PowerBroadcast = 0x0218;

    // スリープからの復帰は、日付が変わったかもしれないことを示す。人が戻ったときも、自動で戻ったときもである。
    [Theory]
    [InlineData(0x0012)]
    [InlineData(0x0007)]
    public void Waking_from_sleep_means_the_day_may_have_changed(int power)
    {
        Assert.True(DayChange.MayFollow(PowerBroadcast, power));
    }

    // スリープに入ることや、ほかのメッセージは、日付が変わったかもしれないことを示さない。
    [Theory]
    [InlineData(PowerBroadcast, 0x0004)]
    [InlineData(0x000F, 0x0012)]
    [InlineData(0x0006, 0x0007)]
    public void Going_to_sleep_or_other_messages_do_not_mean_a_new_day(uint message, int wParam)
    {
        Assert.False(DayChange.MayFollow(message, wParam));
    }
}
