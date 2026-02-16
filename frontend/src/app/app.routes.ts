import { Routes } from '@angular/router';
import { MainLayoutComponent } from './layout/main-layout';

export const routes: Routes = [
  {
    path: '',
    component: MainLayoutComponent,
    children: [
      { path: '', redirectTo: 'pages/today', pathMatch: 'full' },
      {
        path: 'pages/**',
        loadComponent: () => import('./features/journal/page-dispatcher').then(m => m.PageDispatcherComponent),
      },
      // Backward compat redirects
      { path: 'journals/:date', redirectTo: 'pages/:date' },
      {
        path: 'docs/**',
        loadComponent: () => import('./features/journal/page-dispatcher').then(m => m.PageDispatcherComponent),
      },
    ]
  }
];
