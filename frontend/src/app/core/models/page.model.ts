export interface Page {
  id: string;
  dirPath: string;
  fileName: string;
  createdDate: number;
  modifiedDate: number;
  tags: string[];
  titles: PageTitle[];
  propertiesJson: string;
}

export interface PageTitle {
  pageId: string;
  orderIndex: number;
  title: string;
}
