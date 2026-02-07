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
  status: string;  // normal, todo, running, done
  types: string[];
  propertiesJson: string;
}
