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
  readonly markdownDir = signal('');
  readonly weekNumbering = signal<WeekNumbering>('us');
  readonly logicalOutdent = signal(false);
  readonly propertyDefinitions = signal<Record<string, string>>({});

  constructor() {
    this.config = this.loadConfig();
    this.sidebarWidth.set(this.config.sidebarWidth);
    this.loadServerConfig();
    this.loadPropertyDefinitionsFromServer();
  }

  loadServerConfig(): void {
    this.api.getConfig().then(c => {
      this.markdownDir.set(c.markdownDir || '');
      this.weekNumbering.set((c.weekNumbering as WeekNumbering) || 'us');
      this.logicalOutdent.set(c.logicalOutdent ?? false);
    }).catch(e => {
      console.error('Failed to load config from server:', e);
    });
  }

  private loadPropertyDefinitionsFromServer(): void {
    this.api.getPropertyDefinitions().then(r => {
      this.propertyDefinitions.set(r.propertyDefinitions);
    }).catch(e => {
      console.error('Failed to load property definitions:', e);
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

  /**
   * Updates the sidebar width and persists it to local storage.
   * @param width - The new sidebar width in pixels.
   */
  setSidebarWidth(width: number): void {
    this.config.sidebarWidth = width;
    this.sidebarWidth.set(width);
    this.saveConfig();
  }

  /**
   * Updates the week numbering system and persists it to the server.
   * @param value - The week numbering system to use ('us' or 'iso').
   */
  setWeekNumbering(value: WeekNumbering): void {
    this.weekNumbering.set(value);
    this.api.updateConfig({ weekNumbering: value }).catch(e => {
      console.error('Failed to save config to server:', e);
    });
  }

  setMarkdownDir(value: string): void {
    this.markdownDir.set(value);
    this.api.updateConfig({ markdownDir: value }).catch(e => {
      console.error('Failed to save config to server:', e);
    });
  }

  setLogicalOutdent(value: boolean): void {
    this.logicalOutdent.set(value);
    this.api.updateConfig({ logicalOutdent: value }).catch(e => {
      console.error('Failed to save config to server:', e);
    });
  }
}
