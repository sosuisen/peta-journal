namespace MemoView.Models;

public class SearchResultModel
{
    public string PageId { get; set; } = "";
    public string PageTitle { get; set; } = "";
    public string? ItemId { get; set; }
    public string? Snippet { get; set; }
    public float Score { get; set; }
}
