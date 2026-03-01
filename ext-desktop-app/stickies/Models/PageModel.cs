namespace Stickies.Models;

public class PageModel
{
    public string Id { get; set; } = "";
    public string? ParentId { get; set; }
    public string DirPath { get; set; } = "";
    public string Name { get; set; } = "";
    public string Type { get; set; } = "";
    public long CreatedDate { get; set; }
    public long ModifiedDate { get; set; }
    public List<string> Tags { get; set; } = [];
}
