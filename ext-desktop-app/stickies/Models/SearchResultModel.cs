namespace Stickies.Models;

public class SearchResultModel
{
    public string PageId { get; set; } = "";
    public string PageTitle { get; set; } = "";
    public string? BlockId { get; set; }
    public string? Snippet { get; set; }
    public float Score { get; set; }
}
