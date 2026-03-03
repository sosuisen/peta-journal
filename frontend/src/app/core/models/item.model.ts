export interface Item {
  id: string;
  pageId: string;
  dirPath: string;
  name: string;
  type: string;       // "journal" or "doc" (from parent page)
  createdDate: number;
  modifiedDate: number;
  idx: string;
  level: number;
  body: string;
  html: string;
  collapsed: boolean;
  types: string[];
  properties: Record<string, string | string[]>;
}
