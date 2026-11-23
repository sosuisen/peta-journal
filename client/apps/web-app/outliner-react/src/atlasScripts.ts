// The canvas of an atlas is the class of atlas.js, drawn by PixiJS. The scripts put their
// classes on the window, as the Leptos shell loaded them in its index.html. Vite gives their
// URLs, and copies them into the build.
import pixi from '../../../../../atlas/web/pixi.min.js?url';
import tweedle from '../../../../../atlas/web/tweedle.umd.js?url';
// @pixi/ui 2.4.1 for the dialogs of the atlas; it needs tweedle.js 2.1.0 as the global tweedle_js
import pixiUi from '../../../../../atlas/web/pixi-ui.js?url';
import atlas from '../../../../../atlas/web/atlas.js?url';

/** The scripts of the canvas, each after the ones whose globals it uses. */
export const ATLAS_SCRIPTS: readonly string[] = [pixi, tweedle, pixiUi, atlas];

/**
 * A loader of the scripts at `urls`. It adds them to the page one by one, in order, and only
 * once: each later call gives the promise of the first.
 */
export function scriptsLoader(urls: readonly string[]): () => Promise<void> {
  let loading: Promise<void> | undefined;
  return () => {
    loading ??= (async () => {
      for (const url of urls) {
        await addScript(url);
      }
    })();
    return loading;
  };
}

function addScript(url: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const script = document.createElement('script');
    script.src = url;
    script.addEventListener('load', () => resolve());
    script.addEventListener('error', () =>
      reject(new Error(`the script ${url} could not be loaded`)),
    );
    document.head.append(script);
  });
}

/** Loads the scripts of the canvas of an atlas, once. */
export const loadAtlasScripts = scriptsLoader(ATLAS_SCRIPTS);
