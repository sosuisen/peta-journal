import 'highlight.js/styles/github.css';
import './styles.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { AskedOps } from './asked';
import { AskedOpsContext } from './askedContext';
import { CoreContext } from './context';
import { Core } from './core';
import init from './wasm/bridge';

const asked = new AskedOps();
// Not awaited: the frame is drawn while the wasm loads, and each part waits for the core
const core = init().then(() => {
  const made = new Core(asked.take);
  asked.answerTo(made);
  return made;
});

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
