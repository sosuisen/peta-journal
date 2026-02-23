import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet],
  template: `
    <svg xmlns="http://www.w3.org/2000/svg" style="display:none">
      <symbol id="icon-page" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>
      </symbol>
      <symbol id="icon-block" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="5" cy="7" r="2.5" fill="currentColor" stroke="none"/><line x1="11" y1="7" x2="21" y2="7"/><line x1="11" y1="13" x2="19" y2="13"/><line x1="11" y1="19" x2="17" y2="19"/>
      </symbol>
    </svg>
    <router-outlet />
  `,
  styles: [`:host { display: block; height: 100vh; }`]
})
export class App {}
