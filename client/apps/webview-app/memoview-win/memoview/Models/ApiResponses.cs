namespace MemoView.Models;

public class GetPageByIdResponse
{
    public PageModel Page { get; set; } = new();
    public List<ItemModel> Items { get; set; } = [];
}

/// <summary>The values of the configuration of the server with which the links of a page are made.</summary>
public class ConfigResponse
{
    public string WikilinkHref { get; set; } = "";
    public string HashtagHref { get; set; } = "";
    public string PagesBase { get; set; } = "";
    public string WikilinkFragmentPattern { get; set; } = "";
}

public class SearchPageNamesResponse
{
    public List<SearchResultModel> Results { get; set; } = [];
}
