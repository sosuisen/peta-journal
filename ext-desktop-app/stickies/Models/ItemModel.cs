namespace Stickies.Models;

public class ItemModel
{
    public string Id { get; set; } = "";
    public string PageId { get; set; } = "";
    public string DirPath { get; set; } = "";
    public string Name { get; set; } = "";
    public string Type { get; set; } = "";
    public long CreatedDate { get; set; }
    public long ModifiedDate { get; set; }
    public string Idx { get; set; } = "";
    public int Level { get; set; }
    public string Body { get; set; } = "";
    public string Ast { get; set; } = "";
    public bool Collapsed { get; set; }
    public string Status { get; set; } = "normal";
    public List<string> Types { get; set; } = [];
}
