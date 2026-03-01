namespace Stickies.Models;

public class GetPageByIdResponse
{
    public PageModel Page { get; set; } = new();
    public List<BlockModel> Blocks { get; set; } = [];
}

public class SearchPageNamesResponse
{
    public List<SearchResultModel> Results { get; set; } = [];
}
