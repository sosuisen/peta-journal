import type { Filters } from './tasks';

interface FilterCheckboxProps {
  label: string;
  checked: boolean;
  onChange(checked: boolean): void;
}

function FilterCheckbox({ label, checked, onChange }: FilterCheckboxProps) {
  return (
    <label className="checkbox">
      <input
        type="checkbox"
        checked={checked}
        onChange={e => onChange(e.target.checked)}
      />
      {label}
    </label>
  );
}

interface FilterBarProps {
  filters: Filters;
  onChange(filters: Filters): void;
}

/** The controls that choose which tasks are listed. */
export default function FilterBar({ filters, onChange }: FilterBarProps) {
  return (
    <div className="filter-bar">
      <label>
        開始日:
        <input
          type="date"
          value={filters.untilDate}
          onChange={e => onChange({ ...filters, untilDate: e.target.value })}
        />
      </label>

      <FilterCheckbox
        label="Done"
        checked={filters.showDone}
        onChange={showDone => onChange({ ...filters, showDone })}
      />
      <FilterCheckbox
        label="Journal"
        checked={filters.showJournal}
        onChange={showJournal => onChange({ ...filters, showJournal })}
      />
      <FilterCheckbox
        label="Page"
        checked={filters.showPage}
        onChange={showPage => onChange({ ...filters, showPage })}
      />
    </div>
  );
}
