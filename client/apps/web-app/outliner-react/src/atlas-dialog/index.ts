// The dialog of a text of an atlas, for the app and for a page outside it. It reads React and
// the files of this folder alone; the stylesheet of the page body that its Preview needs is
// the page's own (in the app, the `app-item-view` rules of src/styles.css).
export { default as AtlasDialog } from './AtlasDialog';
export type {
  AtlasDialogMode,
  AtlasDialogProps,
  LinkSuggestions,
} from './AtlasDialog';
