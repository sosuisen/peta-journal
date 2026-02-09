import { Injectable, inject, signal } from '@angular/core';
import { ApiService } from './api.service';

export type WeekNumbering = 'us' | 'iso';

interface AppConfig {
  sidebarWidth: number;
}

const DEFAULT_CONFIG: AppConfig = {
  sidebarWidth: 260,
};

const CONFIG_KEY = 'peta-journal-config';

@Injectable({ providedIn: 'root' })
export class ConfigService {
  private readonly api = inject(ApiService);
  private config: AppConfig;
  readonly sidebarWidth = signal(DEFAULT_CONFIG.sidebarWidth);
  readonly weekNumbering = signal<WeekNumbering>('us');

  constructor() {
    this.config = this.loadConfig();
    this.sidebarWidth.set(this.config.sidebarWidth);
    this.loadWeekNumberingFromServer();
  }

  private loadWeekNumberingFromServer(): void {
    this.api.getConfig().then(c => {
      this.weekNumbering.set((c.weekNumbering as WeekNumbering) || 'us');
    }).catch(e => {
      console.error('Failed to load config from server:', e);
    });
  }

  private loadConfig(): AppConfig {
    try {
      const stored = localStorage.getItem(CONFIG_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        return { ...DEFAULT_CONFIG, ...parsed };
      }
    } catch (e) {
      console.error('Failed to load config:', e);
    }
    return { ...DEFAULT_CONFIG };
  }

  private saveConfig(): void {
    try {
      localStorage.setItem(CONFIG_KEY, JSON.stringify(this.config));
    } catch (e) {
      console.error('Failed to save config:', e);
    }
  }

  setSidebarWidth(width: number): void {
    this.config.sidebarWidth = width;
    this.sidebarWidth.set(width);
    this.saveConfig();
  }

  setWeekNumbering(value: WeekNumbering): void {
    this.weekNumbering.set(value);
    this.api.updateConfig(value).catch(e => {
      console.error('Failed to save weekNumbering to server:', e);
    });
  }
}
