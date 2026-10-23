import { createRoot } from 'react-dom/client';
import App from './App';
import './styles.css';

const root = document.getElementById('root');
if (root === null) throw new Error('Reader root is missing.');
createRoot(root, {
  onUncaughtError(error) {
    console.error('Reader failed to render', error);
    root.replaceChildren();
    const message = document.createElement('main');
    message.className = 'page';
    const heading = document.createElement('h1');
    heading.textContent = 'Fid';
    const detail = document.createElement('p');
    detail.textContent =
      'The briefing could not be displayed. Reload to try again.';
    const reload = document.createElement('a');
    reload.href = location.href;
    reload.textContent = 'Reload the briefing';
    message.append(heading, detail, reload);
    root.append(message);
  },
}).render(<App />);
