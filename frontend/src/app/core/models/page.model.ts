export interface Page {
  id: string;
  dirPath: string;
  fileName: string;
  type: string;       // "journal" or "doc"
  createdDate: number;
  modifiedDate: number;
  tags: string[];
  propertiesJson: string;
}
