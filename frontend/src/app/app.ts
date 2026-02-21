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
    </svg>
    <router-outlet />
  `,
  styles: [`:host { display: block; height: 100vh; }`]
})
export class App {}
