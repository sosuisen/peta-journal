import { Component, inject, computed } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { toSignal } from '@angular/core/rxjs-interop';
import { classifyJournalFileName } from '../../core/utils/journal-filename.utils';
import { JournalComponent } from './journal';
import { PageComponent } from './page';

@Component({
  selector: 'app-page-dispatcher',
  imports: [JournalComponent, PageComponent],
  template: `
    @if (isJournal()) {
      <app-journal [pageName]="pageName()" />
    } @else {
      <app-page [pageName]="pageName()" [focusItemId]="fragment()" />
    }
  `,
  styles: [`:host { display: flex; flex-direction: column; height: 100%; }`]
})
export class PageDispatcherComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly urlSegments = toSignal(this.route.url, { initialValue: [] });
  readonly fragment = toSignal(this.route.fragment, { initialValue: '' });

  readonly pageName = computed(() => {
    const segments = this.urlSegments();
    const parts = segments.map(s => s.path).filter(s => s !== 'pages' && s !== 'docs');
    return parts.join('/') || 'today';
  });

  readonly isJournal = computed(() => {
    const name = this.pageName();
    return name === 'today' || classifyJournalFileName(name) !== 'none';
  });
}
