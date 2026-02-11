import { ApplicationConfig, provideBrowserGlobalErrorListeners, APP_INITIALIZER } from '@angular/core';
import { provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';

import { routes } from './app.routes';
import { GlobalUrlSchemeInterceptorService } from './core/services/global-url-scheme-interceptor.service';
import { ExtensionFunctionService } from './core/services/extension-function.service';

function initializeUrlSchemeInterceptor(interceptor: GlobalUrlSchemeInterceptorService) {
  return () => {
    interceptor.initialize();
  };
}

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    provideRouter(routes),
    provideHttpClient(),
    {
      provide: APP_INITIALIZER,
      useFactory: initializeUrlSchemeInterceptor,
      deps: [GlobalUrlSchemeInterceptorService],
      multi: true
    },
    {
      provide: APP_INITIALIZER,
      useFactory: (svc: ExtensionFunctionService) => () => svc.initialize(),
      deps: [ExtensionFunctionService],
      multi: true
    }
  ]
};
