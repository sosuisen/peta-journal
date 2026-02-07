import { Component, input } from '@angular/core';

export interface BreadcrumbItem {
  label: string;
  routerLink?: string;
}

@Component({
  selector: 'app-breadcrumb',
  template: `
    @if (items().length > 0) {
      <nav class="breadcrumb">
        @for (item of items(); track item.label; let last = $last) {
          @if (item.routerLink && !last) {
            <a class="crumb" [attr.href]="item.routerLink">{{ item.label }}</a>
            <span class="sep">/</span>
          } @else {
            <span class="crumb current">{{ item.label }}</span>
          }
        }
      </nav>
    }
  `,
  styles: [`
    .breadcrumb {
      display: flex;
      align-items: center;
      gap: 4px;
      font-size: 13px;
      color: #666;
      padding: 4px 0;
    }
    .crumb { text-decoration: none; color: #007bff; }
    .crumb.current { color: #333; }
    .sep { color: #ccc; }
  `]
})
export class BreadcrumbComponent {
  readonly items = input<BreadcrumbItem[]>([]);
}
