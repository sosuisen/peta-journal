import { Routes } from '@angular/router';
import { MainLayoutComponent } from './layout/main-layout';

export const routes: Routes = [
  {
    path: '',
    component: MainLayoutComponent,
    children: [
      { path: '', redirectTo: 'today', pathMatch: 'full' },
      {
        path: 'today',
        loadComponent: () => import('./features/journal/page-dispatcher').then(m => m.PageDispatcherComponent),
      },
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
      {
        path: 'settings',
        loadComponent: () => import('./features/settings/settings').then(m => m.SettingsComponent),
      },
    ]
  }
];
