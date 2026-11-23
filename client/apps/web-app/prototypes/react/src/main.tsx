import { createRoot } from 'react-dom/client';
import '../../shared/src/styles.css';
import { startCore } from '../../shared/src/core';
import App from './App';

const core = await startCore();
const root = document.getElementById('root');
if (root === null) {
  throw new Error('the page has no #root');
}
createRoot(root).render(<App core={core} />);
