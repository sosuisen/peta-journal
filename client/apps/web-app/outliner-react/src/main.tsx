import 'highlight.js/styles/github.css';
// The styles of the Leptos shell, which draws the same elements. They move here when it goes
import '../../outliner/styles.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { AskedOps } from './asked';
import { AskedOpsContext } from './askedContext';
import { CoreContext } from './context';
import { Core } from './core';
import init from './wasm/bridge';

await init();
const asked = new AskedOps();
const core = new Core(asked.take);
asked.answerTo(core);

const root = document.getElementById('root');
if (root === null) {
  throw new Error('index.html has no #root');
}
createRoot(root).render(
  <StrictMode>
    <CoreContext value={core}>
      <AskedOpsContext value={asked}>
        <App />
      </AskedOpsContext>
    </CoreContext>
  </StrictMode>,
);
