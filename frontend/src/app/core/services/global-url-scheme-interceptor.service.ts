import { Injectable, inject } from '@angular/core';
import { AppLauncherService } from './app-launcher.service';

/**
 * Global service that intercepts ALL clicks on links with custom URL schemes,
 * including those in dynamically generated HTML ([innerHTML], markdown rendering, etc.)
 *
 * This service should be provided in app.config.ts or main.ts to run globally.
 */
@Injectable({
  providedIn: 'root'
})
export class GlobalUrlSchemeInterceptorService {
  private readonly launcher = inject(AppLauncherService);

  // Standard protocols that should NOT be intercepted
  private readonly standardProtocols = [
    'http://',
    'https://',
    'mailto:',
    'tel:',
    'sms:',
    'file:',
    'data:',
    'javascript:',
    'about:',
  ];

  private isInitialized = false;
  private readonly boundHandleClick = this.handleClick.bind(this);

  /**
   * Initialize the global click interceptor.
   * Call this once in app initialization.
   */
  initialize(): void {
    if (this.isInitialized) {
      console.warn('GlobalUrlSchemeInterceptorService already initialized');
      return;
    }

    document.addEventListener('click', this.boundHandleClick, true);
    this.isInitialized = true;

    console.log('GlobalUrlSchemeInterceptorService initialized');
  }

  /**
   * Remove the global click listener and reset state.
   */
  destroy(): void {
    document.removeEventListener('click', this.boundHandleClick, true);
    this.isInitialized = false;
  }

  private handleClick(event: MouseEvent): void {
    // Find the closest <a> tag
    const target = event.target as HTMLElement;
    const anchor = target.closest('a');

    if (!anchor) {
      return;
    }

    const href = anchor.getAttribute('href');

    if (!href) {
      return;
    }

    // Check if this is a custom protocol
    if (!this.hasCustomProtocol(href)) {
      return;
    }

    // Intercept the click
    event.preventDefault();
    event.stopPropagation();

    // Decode HTML entities
    const decodedHref = this.decodeHtmlEntities(href);

    console.log('Intercepted custom protocol link:', decodedHref);

    // Launch via backend API
    this.launcher.launch(decodedHref).then(success => {
      if (!success) {
        console.error('Failed to launch app with URL:', decodedHref);
      }
    });
  }

  private hasCustomProtocol(href: string): boolean {
    // Relative links
    if (href.startsWith('/') || href.startsWith('./') || href.startsWith('../') || href.startsWith('#')) {
      return false;
    }

    // Check if it's a standard protocol
    const lowerHref = href.toLowerCase();
    const isStandard = this.standardProtocols.some(protocol =>
      lowerHref.startsWith(protocol)
    );

    if (isStandard) {
      return false;
    }

    // Check if it contains :// (custom protocol indicator)
    return href.includes('://');
  }

  private decodeHtmlEntities(text: string): string {
    const textarea = document.createElement('textarea');
    textarea.innerHTML = text;
    return textarea.value;
  }
}
