import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { useAsked } from './askedContext';
import type { JournalView, PageView } from './border/peta-types';
import { useCore, usePanel } from './context';
import Outline from './Outline';

/**
 * The frame of the app, as the Leptos shell drew it: the header, the sidebar, the screen and
 * the footer. The core starts once the frame is on the page, so that the first operations it
 * asks for find their components.
 */
export default function App() {
  const core = useCore();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const connection = usePanel('Connection');
  const outline = usePanel('Outline');

  useEffect(() => core.start(), [core]);
  useGlobalKeys();

  return (
    <app-main-layout>
      <div className={sidebarCollapsed ? 'layout sidebar-collapsed' : 'layout'}>
        <Header
          toggleSidebar={() => setSidebarCollapsed(collapsed => !collapsed)}
        />
        <div className="body">
          <aside
            className={sidebarCollapsed ? 'sidebar collapsed' : 'sidebar'}
            style={{ width: sidebarCollapsed ? '0px' : '260px' }}
          />
          <main className="content">
            <PageDispatcher />
          </main>
        </div>
        <footer className="footer">
          <span
            className={`connection-status ${connectionName(connection?.kind)}`}
          >
            {connectionName(connection?.kind)}
          </span>
          {outline?.focused_item_id != null && (
            <span className="item-id">Item: {outline.focused_item_id}</span>
          )}
        </footer>
      </div>
      <ConfirmDialog />
    </app-main-layout>
  );
}

function connectionName(kind: string | undefined): string {
  return (kind ?? 'Connecting').toLowerCase();
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

function Header({ toggleSidebar }: { readonly toggleSidebar: () => void }) {
  const core = useCore();
  const screen = usePanel('Screen');
  const search = usePanel('Search');
  const [menuOpen, setMenuOpen] = useState(false);
  const onDocPage = screen?.kind === 'Page' && screen.value !== null;
  return (
    <header className="header">
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
    </header>
  );
}

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
  }
  return <app-page-dispatcher>{shown}</app-page-dispatcher>;
}

function Journal({ journal }: { readonly journal: JournalView }) {
  const core = useCore();
  const scrolled = usePanel('Outline')?.scrolled_from_top ?? false;
  const kind = journal.journal_type?.kind;
  return (
    <app-journal>
      <div className={scrolled ? 'journal-header scrolled' : 'journal-header'}>
        <h2 className="journal-date">{journal.display_date}</h2>
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
      {journal.page_id !== null && <Outline />}
    </app-journal>
  );
}

function DocPage({ page }: { readonly page: PageView }) {
  const core = useCore();
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
  if (page.is_root_index) {
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
        {!page.is_root_index && page.page.dir_path !== '' && (
          <div className="title-row">
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
      <Outline footer={<Backlinks page={page} />} />
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
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (message !== null) {
      cancel.current?.focus();
    }
  }, [message]);
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
            ref={cancel}
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
