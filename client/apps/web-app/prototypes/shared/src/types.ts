// The types that cross the border of the prototype (#971), written by hand. They are the
// part of `client_core::changes::Changes<HtmlBody>` that the prototype draws, in the shape
// that serde writes. How the real shell makes these types is left open by ADR 079.

export interface Item {
  readonly id: string;
  readonly page_id: string;
  readonly level: number;
  readonly body: string;
  readonly collapsed: boolean;
}

export interface Row {
  readonly item: Item;
  /** The HTML of the body, made by the core. */
  readonly body_model: string;
  readonly position: number;
  readonly indent_px: number;
  readonly is_editing: boolean;
  readonly is_selected: boolean;
  readonly is_collapsed: boolean;
  readonly has_children: boolean;
  readonly heading_level: number;
  readonly show_bullet: boolean;
  readonly date_header: string | null;
}

export type ListChange =
  | { readonly Insert: { readonly at: number; readonly item: Row } }
  | { readonly Remove: { readonly at: number } }
  | { readonly Move: { readonly from: number; readonly to: number } }
  | { readonly Update: { readonly at: number; readonly item: Row } }
  | { readonly Reset: { readonly items: readonly Row[] } };

export interface FocusRequest {
  readonly seq: number;
  readonly item_id: string;
  readonly cursor: number | null;
}

export interface OutlinePanel {
  readonly selection_mode: boolean;
  readonly focus_request: FocusRequest | null;
  readonly focused_item_id: string | null;
}

export type SearchKind = 'today' | 'page' | 'create' | 'item';

export interface SearchEntry {
  readonly position: number;
  readonly kind: SearchKind;
  readonly title: string;
  readonly label: string;
  readonly snippet: string;
}

export interface SearchView {
  readonly query: string;
  readonly results: readonly SearchEntry[];
  readonly show_dropdown: boolean;
  readonly selected_index: number | null;
  readonly searching: boolean;
}

/** The panels the prototype draws. The others pass by. */
export type Panel =
  | { readonly Outline: OutlinePanel }
  | { readonly Search: SearchView }
  | { readonly Title: string }
  | Readonly<Record<string, unknown>>;

export interface Changes {
  readonly rows: readonly ListChange[];
  readonly panels: readonly Panel[];
}

/** A request of the core that the shell performs, as `UiOp` is written. */
export type UiOp =
  | {
      readonly FocusEditor: {
        readonly item_id: string | null;
        readonly replay_keys: readonly unknown[];
      };
    }
  | { readonly ScrollToItem: { readonly item_id: string; readonly smooth: boolean } }
  | { readonly Alert: string }
  | { readonly Confirm: string }
  | { readonly ConfirmDestructive: string }
  | { readonly Prompt: string }
  | Readonly<Record<string, unknown>>;
