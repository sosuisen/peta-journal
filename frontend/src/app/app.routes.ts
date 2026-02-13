import { Routes } from '@angular/router';
import { MainLayoutComponent } from './layout/main-layout';

export const routes: Routes = [
  {
    path: '',
    component: MainLayoutComponent,
    children: [
      { path: '', redirectTo: 'journals/today', pathMatch: 'full' },
      {
        path: 'journals/:date',
        loadComponent: () => import('./features/journal/journal').then(m => m.JournalComponent),
      },
      {
        path: 'docs/**',
        loadComponent: () => import('./features/journal/page').then(m => m.PageComponent),
      },
    ]
  }
];
