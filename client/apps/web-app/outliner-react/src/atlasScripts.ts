// The canvas of an atlas is the class of atlas.js, drawn by PixiJS. The scripts put their
// classes on the window, as the Leptos shell loaded them in its index.html. Vite gives their
// URLs, and copies them into the build.
import pixi from '@atlas-web/pixi.min.js?url';
import atlas from '@atlas-web/atlas.js?url';
// Noto Sans JP at weight 400 (OFL, see NotoSansJP-OFL.txt beside it)
import labelFont from '@client-assets/NotoSansJP-Regular.ttf?url';
import titleFont from '@client-assets/851tegaki_zatsu_normal_0883.ttf?url';

/**
 * The options of the canvas: the labels in the font that a static site draws an atlas in
 * too, so a label takes the same width in both. The group titles and the callouts are in a
 * font of their own.
 */
export const ATLAS_OPTIONS = {
  labelFont: { family: 'PetaAtlas Label', url: labelFont },
  titleFont: { family: '851 Tegaki Zatsu', url: titleFont },
} as const;

/** The scripts of the canvas, each after the ones whose globals it uses. */
export const ATLAS_SCRIPTS: readonly string[] = [pixi, atlas];

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
