export interface Block {
  id: string;
  pageId: string;
  createdDate: number;
  modifiedDate: number;
  idx: string;
  level: number;
  body: string;
  html: string;
  collapsed: boolean;
  types: string[];
  propertiesJson: string;
}
