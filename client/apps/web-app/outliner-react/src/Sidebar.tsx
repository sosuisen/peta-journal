import { useCallback, useState, type MouseEvent, type ReactNode } from 'react';
import type { IndexSort, LoadState, SidebarPage } from './border/peta-types';
import Calendar from './Calendar';
import { useCore, usePanel } from './context';
import Frame from './Frame';
import { openInOf } from './keys';
import { useClosesOnOutside } from './menu';

/** The sections the browser keeps closed, as the Leptos shell kept them. */
const COLLAPSED_KEY = 'sidebar-collapsed';
/**
 * The sort of each folder's index list. The bridge reads it at the start and gives it to the
 * core, in the names of serde: `{"work/":{"key":"modified","order":"desc"}}`.
 */
const INDEX_SORT_KEY = 'sidebar-index-sort';

type Section = 'index' | 'contents' | 'recent';

const LOADING: LoadState = { kind: 'Loading' };

/** What the browser keeps under `key`, or `fallback` when it keeps nothing it can read. */
function stored<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem(key) ?? '') as T;
  } catch {
    return fallback;
  }
}

function saveIndexSort(dir: string, sort: IndexSort): void {
  const sorts = stored<Record<string, unknown>>(INDEX_SORT_KEY, {});
  sorts[dir] = {
    key: sort.key.kind.toLowerCase(),
    order: sort.order.kind.toLowerCase(),
  };
  localStorage.setItem(INDEX_SORT_KEY, JSON.stringify(sorts));
}

const SORTS: readonly IndexSort[] = (
  ['Name', 'Modified', 'Published'] as const
).flatMap(key =>
  (['Asc', 'Desc'] as const).map(order => ({
    key: { kind: key },
    order: { kind: order },
  })),
);

function sameSort(a: IndexSort, b: IndexSort): boolean {
  return a.key.kind === b.key.kind && a.order.kind === b.order.kind;
}

/**
 * The sidebar, ported from the Leptos shell: the calendar, the index pages, the table of
 * contents, the recent pages and the settings button. Each list waits for the core and fails
 * in a frame of its own (ADR 085), so a slow or broken list leaves the others.
 */
export default function Sidebar() {
  return (
    <app-sidebar>
      <nav className="sidebar">
        <div className="sidebar-content">
          <div className="sidebar-section">
            <Frame>
              <Calendar />
            </Frame>
          </div>
          <Frame>
            <IndexSection />
          </Frame>
          <Frame>
            <ContentsSection />
          </Frame>
          <Frame
            fallback={
              <ListSection
                section="recent"
                title="Recent Pages"
                state={LOADING}
                failed=""
              />
            }
          >
            <RecentSection />
          </Frame>
        </div>

        <div className="sidebar-footer">
          <img
            src="/peta-journal-gray.png"
            alt="PetaJournal"
            className="footer-logo"
          />
          <span className="footer-text">PetaJournal</span>
          <Frame>
            <SettingsButton />
          </Frame>
        </div>
      </nav>
    </app-sidebar>
  );
}

/**
 * Whether a section is closed, and the toggle that closes and opens it. The browser keeps the
 * closed sections together, so a toggle reads them again: each section holds only its own.
 */
function useCollapsed(section: Section): [boolean, () => void] {
  const [collapsed, setCollapsed] = useState(() =>
    new Set(stored<string[]>(COLLAPSED_KEY, [])).has(section),
  );
  const toggle = () => {
    const closed = new Set(stored<string[]>(COLLAPSED_KEY, []));
    if (!closed.delete(section)) {
      closed.add(section);
    }
    localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...closed]));
    setCollapsed(!collapsed);
  };
  return [collapsed, toggle];
}

/** Opens a page by its path, where the keys held on the click say. */
function useGo(): (path: string) => (event: MouseEvent) => void {
  const core = useCore();
  return path => event =>
    core.send({
      kind: 'GoToPage',
      path,
      open_in: openInOf(event.nativeEvent),
    });
}

/** A list that has not arrived yet. */
function Skeleton() {
  return (
    <div className="sidebar-skeleton" aria-busy="true">
      <div className="skeleton-line" />
      <div className="skeleton-line" />
      <div className="skeleton-line" />
    </div>
  );
}

/** A list that could not be loaded. */
function Failed({ text }: { readonly text: string }) {
  return <p className="sidebar-failed">{text}</p>;
}

/**
 * A section under a fixed title, which draws its list once it is loaded. While it loads it
 * shows a skeleton, and when it failed it says so.
 */
function ListSection({
  section,
  title,
  state,
  failed,
  children,
}: {
  readonly section: Section;
  readonly title: string;
  readonly state: LoadState;
  readonly failed: string;
  readonly children?: ReactNode;
}) {
  const [collapsed, toggle] = useCollapsed(section);
  let body: ReactNode = children;
  if (state.kind === 'Loading') {
    body = <Skeleton />;
  } else if (state.kind === 'Failed') {
    body = <Failed text={failed} />;
  }
  return (
    <div className="sidebar-section">
      <div className="section-header">
        <h3 className="section-title">{title}</h3>
        <SectionToggle collapsed={collapsed} onToggle={toggle} />
      </div>
      {!collapsed && body}
    </div>
  );
}

/** The pages of the folder of the page on screen, under the folder's name. */
function IndexSection() {
  const index = usePanel('IndexPages');
  const go = useGo();
  const [collapsed, toggle] = useCollapsed('index');
  // The name of a folder still loading is not known yet
  if (index === undefined || index.state.kind === 'Loading') {
    return (
      <div className="sidebar-section">
        <Skeleton />
      </div>
    );
  }
  if (index.state.kind === 'Failed') {
    return (
      <div className="sidebar-section">
        <Failed text="The folder could not be loaded." />
      </div>
    );
  }
  const parent = index.parent_path;
  if (index.pages.length === 0 && parent === null) {
    return null;
  }
  return (
    <div className="sidebar-section">
      <div className="section-header">
        <h3
          className="section-title section-title-link index-header-title"
          onClick={go(index.path)}
        >
          <svg className="page-icon" width="14" height="12">
            <use href="#icon-folder" />
          </svg>
          <span className="index-header-name">{index.header}</span>
        </h3>
        <IndexSortControl dir={index.header} sort={index.sort} />
        <SectionToggle collapsed={collapsed} onToggle={toggle} />
      </div>
      {!collapsed && (
        <div className="index-page-list">
          {parent !== null && (
            <div
              className="sidebar-item recent-page-item folder-item"
              onClick={go(parent)}
            >
              <svg className="page-icon" width="14" height="12">
                <use href="#icon-folder" />
              </svg>
              <span className="page-label">
                <span className="page-name">../</span>
              </span>
            </div>
          )}
          <PageList pages={index.pages} current={index.current_page_id} />
        </div>
      )}
    </div>
  );
}

/** The headings of the doc page on screen. A page without headings has no section. */
function ContentsSection() {
  const core = useCore();
  const contents = usePanel('Headings');
  const state = contents?.state ?? LOADING;
  const headings = contents === undefined ? [] : contents.headings;
  if (state.kind === 'Loaded' && headings.length === 0) {
    return null;
  }
  return (
    <ListSection
      section="contents"
      title="Contents"
      state={state}
      failed="The contents could not be loaded."
    >
      <div className="contents-list">
        {headings.map(heading => (
          <div
            key={heading.id}
            className={`contents-item contents-h${heading.level}`}
            onClick={() =>
              core.send({
                kind: 'ScrollToItemRequested',
                item_id: heading.id,
              })
            }
          >
            {heading.text}
          </div>
        ))}
      </div>
    </ListSection>
  );
}

/** The doc pages changed last. The page on screen is not marked here. */
function RecentSection() {
  const recent = usePanel('RecentPages');
  return (
    <ListSection
      section="recent"
      title="Recent Pages"
      state={recent?.state ?? LOADING}
      failed="The recent pages could not be loaded."
    >
      <PageList pages={recent?.pages ?? []} current={null} />
    </ListSection>
  );
}

/**
 * The pages of a list; the page `current` is marked. A right click on a page opens a menu
 * that shows the folder of its markdown file.
 */
function PageList({
  pages,
  current,
}: {
  readonly pages: readonly SidebarPage[];
  readonly current: string | null;
}) {
  const core = useCore();
  const go = useGo();
  const [contextMenu, setContextMenu] = useState<{
    pageId: string;
    x: number;
    y: number;
  } | null>(null);
  const closeContextMenu = useCallback(() => setContextMenu(null), []);
  return (
    <>
      {pages.map(shown => (
        <PageEntry
          key={shown.page.id}
          shown={shown}
          active={current === shown.page.id}
          onClick={go(shown.path)}
          onContextMenu={event => {
            event.preventDefault();
            setContextMenu({
              pageId: shown.page.id,
              x: event.clientX,
              y: event.clientY,
            });
          }}
        />
      ))}
      {contextMenu !== null && (
        <PageContextMenu
          at={contextMenu}
          close={closeContextMenu}
          open={() =>
            core.send({ kind: 'OpenExportFolder', page_id: contextMenu.pageId })
          }
        />
      )}
    </>
  );
}

function SettingsButton() {
  const core = useCore();
  return (
    <button
      className="settings-btn"
      title="Settings"
      onClick={() => core.go('/settings')}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="#888"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>
  );
}

function PageEntry({
  shown,
  active,
  onClick,
  onContextMenu,
}: {
  readonly shown: SidebarPage;
  readonly active: boolean;
  readonly onClick: (event: MouseEvent) => void;
  readonly onContextMenu: (event: MouseEvent) => void;
}) {
  let icon = '#icon-page';
  if (shown.is_dir) {
    icon = '#icon-folder';
  } else if (shown.published) {
    icon = '#icon-page-filled';
  }
  const classes = ['sidebar-item', 'recent-page-item'];
  if (shown.is_dir) {
    classes.push('folder-item');
  }
  if (active) {
    classes.push('active');
  }
  return (
    <div
      className={classes.join(' ')}
      onClick={onClick}
      onContextMenu={onContextMenu}
    >
      <svg
        className="page-icon"
        width={shown.is_dir ? '14' : '11'}
        height={shown.is_dir ? '12' : '11'}
      >
        <use href={icon} />
      </svg>
      <span className="page-label">
        {shown.dir_label !== null && (
          <span className="page-dir">{shown.dir_label}</span>
        )}
        <span className="page-name">{shown.label}</span>
      </span>
    </div>
  );
}

function PageContextMenu({
  at,
  close,
  open,
}: {
  readonly at: { readonly x: number; readonly y: number };
  readonly close: () => void;
  readonly open: () => void;
}) {
  useClosesOnOutside('.sidebar-context-menu', close);
  return (
    <div
      className="index-sort-menu sidebar-context-menu"
      style={{ left: `${at.x}px`, top: `${at.y}px` }}
    >
      <button
        className="index-sort-item"
        onClick={() => {
          open();
          close();
        }}
      >
        Open export folder
      </button>
    </div>
  );
}

/**
 * The sort control of the index list: one small button that opens a popup menu with the six
 * sorts, the current one marked.
 */
function IndexSortControl({
  dir,
  sort,
}: {
  readonly dir: string;
  readonly sort: IndexSort;
}) {
  const core = useCore();
  const [open, setOpen] = useState(false);
  const close = useCallback(() => setOpen(false), []);
  return (
    <div className="index-sort">
      <button
        className="collapse-btn index-sort-btn"
        title="Sort"
        onClick={() => setOpen(shown => !shown)}
      >
        {/* Three bars of falling length: the usual sort glyph, unlike the collapse chevrons */}
        <svg width="12" height="12" viewBox="0 0 12 12" fill="#888">
          <rect x="1" y="2" width="10" height="1.6" />
          <rect x="1" y="5.2" width="7" height="1.6" />
          <rect x="1" y="8.4" width="4" height="1.6" />
        </svg>
      </button>
      {open && (
        <SortMenu
          sort={sort}
          close={close}
          choose={chosen => {
            saveIndexSort(dir, chosen);
            core.send({ kind: 'SetIndexSort', dir, sort: chosen });
          }}
        />
      )}
    </div>
  );
}

function SortMenu({
  sort,
  close,
  choose,
}: {
  readonly sort: IndexSort;
  readonly close: () => void;
  readonly choose: (sort: IndexSort) => void;
}) {
  // The button is inside too: its own click opens and closes the menu
  useClosesOnOutside('.index-sort', close);
  return (
    <div className="index-sort-menu">
      {SORTS.map(item => {
        const current = sameSort(sort, item);
        return (
          <button
            key={`${item.key.kind}-${item.order.kind}`}
            className={current ? 'index-sort-item current' : 'index-sort-item'}
            onClick={() => {
              choose(item);
              close();
            }}
          >
            <span className="index-sort-check">{current ? '✓' : ''}</span>
            {`${item.key.kind} ${item.order.kind === 'Asc' ? '↑' : '↓'}`}
          </button>
        );
      })}
    </div>
  );
}

function SectionToggle({
  collapsed,
  onToggle,
}: {
  readonly collapsed: boolean;
  readonly onToggle: () => void;
}) {
  return (
    <button
      className="collapse-btn"
      onClick={onToggle}
      aria-label={collapsed ? 'Expand' : 'Collapse'}
    >
      <svg width="10" height="12" viewBox="0 0 10 12">
        {collapsed ? (
          <>
            <polygon points="2,4.5 5,1.5 8,4.5" fill="#888" />
            <polygon points="2,7.5 5,10.5 8,7.5" fill="#888" />
          </>
        ) : (
          <>
            <polygon points="2,2.5 5,5.5 8,2.5" fill="#888" />
            <polygon points="2,9.5 5,6.5 8,9.5" fill="#888" />
          </>
        )}
      </svg>
    </button>
  );
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-sidebar': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
