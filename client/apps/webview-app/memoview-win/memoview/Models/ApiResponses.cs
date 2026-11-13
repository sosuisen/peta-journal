namespace MemoView.Models;

public class GetPageByIdResponse
{
    public PageModel Page { get; set; } = new();
    public List<ItemModel> Items { get; set; } = [];
}

public class ConfigResponse
{
    public string TagsDir { get; set; } = "";
}

public class SearchPageNamesResponse
{
    public List<SearchResultModel> Results { get; set; } = [];
}
