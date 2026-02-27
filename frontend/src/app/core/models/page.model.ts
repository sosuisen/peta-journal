export interface Page {
  id: string;
  parentId: string;
  dirPath: string;
  name: string;
  type: string;       // "journal" or "doc"
  createdDate: number;
  modifiedDate: number;
  tags: string[];
}
