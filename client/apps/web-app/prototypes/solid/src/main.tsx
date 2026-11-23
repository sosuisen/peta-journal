import { render } from '@solidjs/web';
import '../../shared/src/styles.css';
import { startCore } from '../../shared/src/core';
import App from './App';

const core = await startCore();
const root = document.getElementById('root');
if (root === null) {
  throw new Error('the page has no #root');
}
render(() => <App core={core} />, root);
