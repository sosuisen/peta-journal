export interface Block {
  id: string;
  pageId: string;
  dirPath: string;
  fileName: string;
  type: string;       // "journal" or "doc" (from parent page)
  createdDate: number;
  modifiedDate: number;
  idx: string;
  level: number;
  body: string;
  html: string;
  collapsed: boolean;
  status: string;  // normal, todo, running, done
  types: string[];
}
