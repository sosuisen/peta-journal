import { Component, inject } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../core/services/api.service';
import { ConfigService, WeekNumbering } from '../../core/services/config.service';

@Component({
  selector: 'app-settings',
  imports: [FormsModule],
  template: `
    <div class="settings">
      <h2>Settings</h2>

      <div class="setting-group">
        <label class="setting-label">Markdown working directory</label>
        <div class="input-with-button">
          <input
            type="text"
            class="setting-input"
            [ngModel]="config.markdownDir()"
            readonly
          />
          <button class="browse-btn" (click)="browseDirectory()">Browse...</button>
        </div>
        <p class="setting-description">Changes require a server restart to take effect.</p>
      </div>

      <div class="setting-group">
        <label class="setting-label">Week numbering</label>
        <select
          class="setting-select"
          [ngModel]="config.weekNumbering()"
          (ngModelChange)="config.setWeekNumbering($event)"
        >
          <option value="us">US (Sunday start)</option>
          <option value="iso">ISO (Monday start)</option>
        </select>
      </div>

      <div class="setting-group">
        <label class="setting-checkbox">
          <input
            type="checkbox"
            [ngModel]="config.logicalOutdent()"
            (ngModelChange)="config.setLogicalOutdent($event)"
          />
          Logical outdent
        </label>
        <p class="setting-description">When outdenting, do not break parent-child relationships.</p>
      </div>
    </div>
  `,
  styles: [`
    .settings {
      max-width: 600px;
      padding: 24px;
    }
    h2 {
      margin: 0 0 24px;
      font-size: 20px;
      font-weight: 600;
    }
    .setting-group {
      margin-bottom: 20px;
    }
    .setting-label {
      display: block;
      font-size: 13px;
      font-weight: 500;
      margin-bottom: 6px;
      color: #333;
    }
    .setting-input, .setting-select {
      width: 100%;
      padding: 6px 10px;
      border: 1px solid #ccc;
      border-radius: 4px;
      font-size: 14px;
      box-sizing: border-box;
    }
    .input-with-button {
      display: flex;
      gap: 6px;
    }
    .input-with-button .setting-input {
      flex: 1;
      background: #f0f0f0;
      color: #666;
    }
    .browse-btn {
      padding: 6px 12px;
      border: 1px solid #ccc;
      border-radius: 4px;
      background: #f8f9fa;
      cursor: pointer;
      font-size: 13px;
      white-space: nowrap;
    }
    .browse-btn:hover { background: #e9ecef; }
    .setting-input:focus, .setting-select:focus {
      outline: none;
      border-color: #007bff;
    }
    .setting-checkbox {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 14px;
      cursor: pointer;
    }
    .setting-description {
      margin: 4px 0 0;
      font-size: 12px;
      color: #888;
    }
  `]
})
export class SettingsComponent {
  readonly config = inject(ConfigService);
  private readonly api = inject(ApiService);

  constructor() {
    inject(Title).setTitle('Settings');
    this.config.loadServerConfig();
  }

  async browseDirectory(): Promise<void> {
    const result = await this.api.browseFolder();
    if (result.selected) {
      this.config.setMarkdownDir(result.path);
    }
  }
}
