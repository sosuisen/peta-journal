import { FmRoot, type ReactRenderOptions } from 'flatmarkdown-ast2react';
import type { MouseEvent } from 'react';
import { serverUrl, type Item } from './api';
import { isOnLink } from './dom';
import { getDue, getPlan, pagePath, taskLabel } from './tasks';

const fmOptions: ReactRenderOptions = {
  wikilink: { hrefPrefix: `${serverUrl}/pages/` },
};

function ItemDates({ item }: { item: Item }) {
  const plan = getPlan(item);
  const due = getDue(item);
  if (!plan && !due) return null;
  return (
    <span className="item-dates">
      {plan && <span>開始: {plan}</span>}
      {due && <span>期限: {due}</span>}
    </span>
  );
}

interface ItemRowProps {
  item: Item;
  onCycle: (item: Item) => void;
}

/**
 * One task: a click on the row opens its page in PetaJournal, and a click on the badge
 * moves the task to its next state. A link in the text of the task works as a link.
 */
export default function ItemRow({ item, onCycle }: ItemRowProps) {
  const label = taskLabel(item);
  const openInPetaJournal = (event: MouseEvent<HTMLDivElement>) => {
    // A click on a link in the text follows that link, and does not open the page too
    if (isOnLink(event.target as Element)) return;
    window.open(`${serverUrl}${pagePath(item)}`, 'peta-journal');
  };
  return (
    <div className="item-row" onClick={openInPetaJournal}>
      <div className="item-main">
        <span className="item-page">{item.name}</span>
        <span className="item-content">
          <FmRoot ast={item.ast} options={fmOptions} />
        </span>
        <ItemDates item={item} />
      </div>
      <button
        className={`badge badge-${label.toLowerCase()}`}
        onClick={e => {
          e.stopPropagation();
          onCycle(item);
        }}
        title="Click to cycle task"
      >
        {label}
      </button>
    </div>
  );
}
