export interface Page {
  id: string;
  dirPath: string;
  fileName: string;
  createdDate: number;
  modifiedDate: number;
  tags: string[];
  propertiesJson: string;
}
