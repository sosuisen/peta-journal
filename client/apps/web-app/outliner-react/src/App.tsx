import { useEffect, useState, useSyncExternalStore } from 'react';
import { useAsked } from './askedContext';
import Atlas from './Atlas';
import type { JournalView, PageView } from './border/peta-types';
import Calendar from './Calendar';
import { useCore, usePanel } from './context';
import Frame from './Frame';
import Outline from './Outline';
import PageProperties, { type Draft } from './PageProperties';
import SearchBar from './SearchBar';
import Settings from './Settings';
import Sidebar from './Sidebar';
import { useSidebarWidth } from './sidebarWidth';

/**
 * The frame of the app, as the Leptos shell drew it: the header, the sidebar, the screen and
 * the footer. The core starts once the frame is on the page, so that the first operations it
 * asks for find their components.
 */
export default function App() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [sidebarWidth, startResize] = useSidebarWidth();

  return (
    <app-main-layout>
      <Icons />
      <div className={sidebarCollapsed ? 'layout sidebar-collapsed' : 'layout'}>
        <header className="header">
          <Frame>
            <Header
              toggleSidebar={() => setSidebarCollapsed(collapsed => !collapsed)}
            />
          </Frame>
        </header>
        <div className="body">
          <aside
            className={sidebarCollapsed ? 'sidebar collapsed' : 'sidebar'}
            style={{ width: sidebarCollapsed ? '0px' : `${sidebarWidth}px` }}
          >
            <Frame>
              <Sidebar />
            </Frame>
          </aside>
          {!sidebarCollapsed && (
            <div className="resize-handle" onMouseDown={startResize} />
          )}
          <main className="content">
            <Frame>
              <PageDispatcher />
            </Frame>
          </main>
        </div>
        <footer className="footer">
          <Frame fallback={<Connection kind={undefined} />}>
            <Status />
            <Start />
          </Frame>
        </footer>
      </div>
      <ConfirmDialog />
    </app-main-layout>
  );
}

/**
 * Starts the core, and listens to the page. In the footer, the last part of the frame: its
 * effect runs after those of the other parts, so that the first operations the core asks for
 * find their components. Not in a part of its own, which would tell a failed wasm twice.
 */
function Start() {
  const core = useCore();
  useEffect(() => core.start(), [core]);
  useGlobalKeys();
  useDayWatch();
  return null;
}

/** The connection to the server, and the item that has the focus. */
function Status() {
  const connection = usePanel('Connection');
  const outline = usePanel('Outline');
  return (
    <>
      <Connection kind={connection?.kind} />
      {outline?.focused_item_id != null && (
        <span className="item-id">Item: {outline.focused_item_id}</span>
      )}
    </>
  );
}

/** The state of the connection, as connecting before the core first tells it. */
function Connection({ kind }: { readonly kind: string | undefined }) {
  const name = (kind ?? 'Connecting').toLowerCase();
  return <span className={`connection-status ${name}`}>{name}</span>;
}

/** The keys of the whole page: Ctrl+Alt+Enter opens Home. Files dropped off an item stay out. */
function useGlobalKeys(): void {
  const core = useCore();
  const asked = useAsked();
  useEffect(() => {
    // A key pressed while an editor waits for its focus is kept for that editor. An editor
    // that has the focus keeps such a key by itself
    const late = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        event.defaultPrevented ||
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement
      ) {
        return;
      }
      if (asked.keepForReplay(event)) {
        event.preventDefault();
      }
    };
    // In the capture phase, and stopped there: an editor would take it as its Ctrl+Enter
    const home = (event: KeyboardEvent) => {
      if (
        event.key === 'Enter' &&
        event.ctrlKey &&
        event.altKey &&
        !event.shiftKey &&
        !event.metaKey &&
        !event.isComposing
      ) {
        event.preventDefault();
        event.stopPropagation();
        core.send({ kind: 'NavigateHome' });
      }
    };
    // Without these the browser opens a file dropped outside an item, in place of the app
    const refuse = (event: Event) => event.preventDefault();
    document.addEventListener('keydown', home, true);
    document.addEventListener('keydown', late);
    document.addEventListener('dragover', refuse);
    document.addEventListener('drop', refuse);
    return () => {
      document.removeEventListener('keydown', home, true);
      document.removeEventListener('keydown', late);
      document.removeEventListener('dragover', refuse);
      document.removeEventListener('drop', refuse);
    };
  }, [asked, core]);
}

/**
 * Tells the core that the day may have changed when the page comes back. The browser holds
 * the timers of a hidden tab and of a PC that sleeps, and the core would open the new day
 * only up to a minute later (#1031).
 */
function useDayWatch(): void {
  const core = useCore();
  useEffect(() => {
    const tell = () => core.send({ kind: 'DateMayHaveChanged' });
    const shown = () => {
      if (document.visibilityState === 'visible') {
        tell();
      }
    };
    document.addEventListener('visibilitychange', shown);
    window.addEventListener('focus', tell);
    return () => {
      document.removeEventListener('visibilitychange', shown);
      window.removeEventListener('focus', tell);
    };
  }, [core]);
}

/** The buttons and the search of the header. */
function Header({ toggleSidebar }: { readonly toggleSidebar: () => void }) {
  const core = useCore();
  const screen = usePanel('Screen');
  const search = usePanel('Search');
  const [menuOpen, setMenuOpen] = useState(false);
  const onDocPage = screen?.kind === 'Page' && screen.value !== null;
  return (
    <>
      <div className="header-left">
        <button className="menu-btn" onClick={toggleSidebar}>
          ☰
        </button>
        <button
          className="root-btn"
          title="Root"
          onClick={() => core.send({ kind: 'NavigateToRoot' })}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" {...STROKE}>
            <path d="M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-2H5a2 2 0 0 0-2 2z" />
          </svg>
        </button>
        <button
          className="tags-btn"
          title="Tags"
          onClick={() => core.send({ kind: 'NavigateToTags' })}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" {...STROKE}>
            <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z" />
            <line x1="7" y1="7" x2="7.01" y2="7" />
          </svg>
        </button>
        <button
          className="journal-btn"
          title="Today"
          onClick={() => core.send({ kind: 'NavigateToToday' })}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" {...STROKE}>
            <rect x="3" y="1" width="16" height="20" rx="1.5" ry="1.5" />
            <line x1="3" y1="13" x2="19" y2="13" strokeWidth="1" />
            <line x1="3" y1="17" x2="19" y2="17" strokeWidth="1" />
            <line x1="14" y1="2" x2="14" y2="20" strokeWidth="3" />
            <rect x="5" y="21" width="3" height="2" fill="#666" stroke="none" />
          </svg>
        </button>
      </div>
      <div className="header-center">
        <button
          className="new-page-btn"
          title="New page"
          onClick={() =>
            core.send({
              kind: 'CreateNewPage',
              search_query: search?.query ?? '',
            })
          }
        >
          +
        </button>
        <SearchBar />
      </div>
      <div className="header-right">
        <button
          className="menu-btn home-btn"
          title="Home (Ctrl+Alt+Enter)"
          onClick={() => core.send({ kind: 'NavigateHome' })}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" {...STROKE}>
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
            <polyline points="9 22 9 12 15 12 15 22" />
          </svg>
        </button>
        <div className="page-menu">
          <button
            className="page-menu-btn"
            onClick={() => setMenuOpen(open => !open)}
          >
            ⋮
          </button>
          {menuOpen && (
            <div
              className="page-menu-dropdown"
              onClick={() => setMenuOpen(false)}
            >
              {onDocPage && (
                <button
                  className="page-menu-item"
                  onClick={() => core.send({ kind: 'EditPageProperties' })}
                >
                  Edit properties
                </button>
              )}
              <button
                className="page-menu-item"
                onClick={() => core.send({ kind: 'DeleteCurrentPage' })}
              >
                Delete page
              </button>
            </div>
          )}
        </div>
      </div>
    </>
  );
}

/** The icons of the pages and the items, which the sidebar and the search draw with `<use>`. */
function Icons() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" style={{ display: 'none' }}>
      <symbol id="icon-page" viewBox="0 0 24 24" {...ICON_STROKE}>
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
      </symbol>
      <symbol
        id="icon-page-filled"
        viewBox="0 0 24 24"
        {...ICON_STROKE}
        fill="currentColor"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" fill="none" stroke="#fff" />
      </symbol>
      <symbol id="icon-folder" viewBox="0 0 28 24" {...ICON_STROKE}>
        <path d="M26 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h6l3 3h11a2 2 0 0 1 2 2z" />
      </symbol>
      <symbol
        id="icon-item"
        viewBox="0 0 24 24"
        {...ICON_STROKE}
        strokeWidth="2.5"
      >
        <circle cx="5" cy="7" r="2.5" fill="currentColor" stroke="none" />
        <line x1="11" y1="7" x2="21" y2="7" />
        <line x1="11" y1="13" x2="19" y2="13" />
        <line x1="11" y1="19" x2="17" y2="19" />
      </symbol>
    </svg>
  );
}

const ICON_STROKE = {
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 3,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
} as const;

const STROKE = {
  fill: 'none',
  stroke: '#666',
  strokeWidth: 2,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
} as const;

function PageDispatcher() {
  const screen = usePanel('Screen');
  let shown = null;
  if (screen?.kind === 'Journal') {
    shown = <Journal journal={screen.value} />;
  } else if (screen?.kind === 'Page' && screen.value !== null) {
    shown = <DocPage page={screen.value} />;
  } else if (screen?.kind === 'Settings') {
    shown = <Settings />;
  }
  return <app-page-dispatcher>{shown}</app-page-dispatcher>;
}

function Journal({ journal }: { readonly journal: JournalView }) {
  const core = useCore();
  const scrolled = usePanel('Outline')?.scrolled_from_top ?? false;
  const kind = journal.journal_type?.kind;
  const [showCalendar, setShowCalendar] = useState(false);
  const toggle = () => {
    if (!showCalendar) {
      // The calendar opens at the month of today, as the sidebar's shows
      core.send({ kind: 'CalendarNav', value: { kind: 'Today' } });
    }
    setShowCalendar(!showCalendar);
  };
  return (
    <app-journal>
      <div className={scrolled ? 'journal-header scrolled' : 'journal-header'}>
        <h2 className="journal-date" onClick={toggle}>
          {journal.display_date}
        </h2>
        <button className="calendar-btn" onClick={toggle}>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#666"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
            <line x1="16" y1="2" x2="16" y2="6" />
            <line x1="8" y1="2" x2="8" y2="6" />
            <line x1="3" y1="10" x2="21" y2="10" />
          </svg>
        </button>
        {(kind === 'Daily' || kind === 'Weekly') && (
          <>
            <button
              className="nav-btn"
              onClick={() =>
                core.send({ kind: 'JournalNav', value: { kind: 'Prev' } })
              }
            >
              {'<'}
            </button>
            <button
              className="nav-btn"
              onClick={() =>
                core.send({ kind: 'JournalNav', value: { kind: 'Next' } })
              }
            >
              {'>'}
            </button>
          </>
        )}
      </div>
      {/* A journal opened in another tab or window leaves this one, and its calendar, open */}
      {showCalendar && (
        <Calendar
          selects="JournalDay"
          onNavigate={elsewhere => setShowCalendar(elsewhere)}
        />
      )}
      {journal.page_id !== null && <Outline />}
    </app-journal>
  );
}

/**
 * Switches between the index page of a folder and its atlas. The icon names the page the
 * button opens: a map on the index page, a bulleted outline on the atlas page.
 */
function AtlasButton({ atlas }: { readonly atlas: boolean }) {
  const core = useCore();
  return (
    <button
      className={atlas ? 'atlas-btn active' : 'atlas-btn'}
      title={atlas ? 'Index' : 'Atlas'}
      aria-pressed={atlas}
      onClick={() => core.send({ kind: 'ToggleAtlas' })}
    >
      <svg
        width="22"
        height="22"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        {atlas ? (
          <>
            <line x1="9" y1="6" x2="20" y2="6" />
            <line x1="9" y1="12" x2="20" y2="12" />
            <line x1="9" y1="18" x2="20" y2="18" />
            <circle cx="4" cy="6" r="1" fill="currentColor" />
            <circle cx="4" cy="12" r="1" fill="currentColor" />
            <circle cx="4" cy="18" r="1" fill="currentColor" />
          </>
        ) : (
          <>
            <polygon points="1 6 1 22 8 18 16 22 23 18 23 2 16 6 8 2 1 6" />
            <line x1="8" y1="2" x2="8" y2="18" />
            <line x1="16" y1="6" x2="16" y2="22" />
          </>
        )}
      </svg>
    </button>
  );
}

function DocPage({ page }: { readonly page: PageView }) {
  const core = useCore();
  // Kept by the page, so the new rows stay while the core draws the properties again
  const [drafts, setDrafts] = useState<readonly Draft[]>([]);
  const scrolled = usePanel('Outline')?.scrolled_from_top ?? false;
  const crumbs = page.dir_segments.map((segment, index) => {
    const last = index + 1 === page.dir_segments.length;
    return (
      <span key={segment.index_path}>
        {page.is_index && last ? (
          <span className="dir-segment-title">{segment.name}</span>
        ) : (
          <a
            className="dir-segment"
            onClick={() =>
              core.send({
                kind: 'GoToPage',
                path: segment.index_path,
                open_in: { kind: 'Here' },
              })
            }
          >
            {segment.name}
          </a>
        )}
        <span className="breadcrumb-slash">/</span>
      </span>
    );
  });
  let title;
  if (page.is_atlas) {
    // The title of an atlas page follows its folder: no path and no index mark
    title = (
      <div className="title-row">
        <AtlasButton atlas />
        <h2 className="page-title atlas-title">{page.title}</h2>
      </div>
    );
  } else if (page.is_root_index) {
    title = (
      <>
        <div className="title-row">
          <h2 className="page-title root-index-title">(root)/</h2>
        </div>
        <div className="index-label">index</div>
      </>
    );
  } else if (page.is_index) {
    title = (
      <div className="index-label">
        index{' '}
        {page.can_demote && (
          <button
            className="demote-btn"
            onClick={() => core.send({ kind: 'DemoteIndex' })}
          >
            ページ名を {page.demote_name} に変更する
          </button>
        )}
      </div>
    );
  } else {
    title = <h2 className="page-title">{page.title}</h2>;
  }
  return (
    <app-page>
      <div className={scrolled ? 'page-header scrolled' : 'page-header'}>
        {!page.is_atlas && !page.is_root_index && page.page.dir_path !== '' && (
          <div className="title-row">
            {page.is_index && <AtlasButton atlas={false} />}
            <div
              className={
                page.is_index
                  ? 'dir-path-breadcrumb index-breadcrumb'
                  : 'dir-path-breadcrumb'
              }
            >
              {crumbs}
            </div>
          </div>
        )}
        {title}
      </div>
      {/* The canvas takes the place of the items: an atlas page shows its directory */}
      {page.is_atlas ? (
        <Atlas dirPath={page.page.dir_path} />
      ) : (
        <Outline
          header={
            page.show_properties || drafts.length > 0 ? (
              <PageProperties
                page={page}
                drafts={drafts}
                setDrafts={setDrafts}
              />
            ) : undefined
          }
          footer={<Backlinks page={page} />}
        />
      )}
    </app-page>
  );
}

/** The pages and items that link to the page, below its items. */
function Backlinks({ page }: { readonly page: PageView }) {
  const core = useCore();
  if (page.backlinks.length === 0) {
    return null;
  }
  // A plain click stays in the app. With a key held, the browser opens a tab or a window
  const follow = (url: string) => (event: React.MouseEvent) => {
    if (event.shiftKey || event.ctrlKey || event.metaKey) {
      return;
    }
    event.preventDefault();
    core.go(url);
  };
  return (
    <section className="backlinks">
      <h3 className="backlinks-title">Backlinks</h3>
      <ul className="backlink-pages">
        {page.backlinks.map(group => (
          <li key={group.url} className="backlink-page">
            <a
              className="backlink-page-link"
              href={group.url}
              onClick={follow(group.url)}
            >
              {group.title}
            </a>
            <ul className="backlink-items">
              {group.items.map(item => (
                <li key={item.url}>
                  <a
                    className="backlink-item"
                    href={item.url}
                    onClick={follow(item.url)}
                  >
                    {item.text}
                  </a>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </section>
  );
}

/**
 * The confirm of a deletion that is hard to undo. Not the `confirm` of the browser, which
 * answers OK to Enter: Cancel has the focus here.
 */
function ConfirmDialog() {
  const asked = useAsked();
  const message = useSyncExternalStore(asked.subscribe, asked.confirmMessage);
  if (message === null) {
    return null;
  }
  return (
    <div
      className="confirm-overlay"
      onKeyDown={event => {
        if (event.key === 'Escape') {
          event.preventDefault();
          asked.answerConfirm(false);
        }
      }}
    >
      <div className="confirm-dialog" role="alertdialog" aria-modal="true">
        <p className="confirm-message">{message}</p>
        <div className="confirm-buttons">
          <button
            type="button"
            // Enter on the focused button must not delete
            autoFocus
            onClick={() => asked.answerConfirm(false)}
          >
            Cancel
          </button>
          <button
            type="button"
            className="confirm-ok"
            onClick={() => asked.answerConfirm(true)}
          >
            OK
          </button>
        </div>
      </div>
    </div>
  );
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-main-layout': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      'app-page-dispatcher': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      'app-journal': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      'app-page': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
