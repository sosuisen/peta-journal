import { Injectable, signal } from '@angular/core';

interface AppConfig {
  sidebarWidth: number;
}

const DEFAULT_CONFIG: AppConfig = {
  sidebarWidth: 260,
};

const CONFIG_KEY = 'peta-journal-config';

@Injectable({ providedIn: 'root' })
export class ConfigService {
  private config: AppConfig;
  readonly sidebarWidth = signal(DEFAULT_CONFIG.sidebarWidth);

  constructor() {
    this.config = this.loadConfig();
    this.sidebarWidth.set(this.config.sidebarWidth);
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
}
