import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';

interface LaunchResponse {
  success: boolean;
  message: string;
}

@Injectable({
  providedIn: 'root'
})
export class AppLauncherService {
  private readonly http = inject(HttpClient);
  private readonly apiUrl = 'http://localhost:8080/api/launcher';

  /**
   * Launch the desktop app with a URL scheme
   * @param url URL scheme (e.g., "peta-journal://page/123")
   */
  async launch(url: string): Promise<boolean> {
    try {
      const response = await firstValueFrom(
        this.http.post<LaunchResponse>(`${this.apiUrl}/open`, { url })
      );
      return response.success;
    } catch (error) {
      console.error('Failed to launch app:', error);
      return false;
    }
  }

  /**
   * Open a specific page in the desktop app
   * @param pageId Page UUID
   */
  async openPage(pageId: string): Promise<boolean> {
    return this.launch(`peta-journal://page/${pageId}`);
  }

  /**
   * Open a journal entry in the desktop app
   * @param date ISO date string (e.g., "2026-02-09")
   */
  async openJournal(date: string): Promise<boolean> {
    return this.launch(`peta-journal://journal/${date}`);
  }

  /**
   * Open search in the desktop app
   * @param query Search query
   */
  async openSearch(query?: string): Promise<boolean> {
    const url = query
      ? `peta-journal://search?q=${encodeURIComponent(query)}`
      : 'peta-journal://search';
    return this.launch(url);
  }
}
