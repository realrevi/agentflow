import './assets/fonts/fonts.css';
import './app/styles/base.css';
import './app/styles/controls.css';
import './app/styles/shell.css';
import './app/styles/panels.css';
import './app/styles/statusbar.css';
import './app/styles/dialog.css';
import './shared/ui/menu.css';
import { render } from 'solid-js/web';
import App from './App';
import { bootstrap } from './app/bootstrap';

const root = document.getElementById('root');
if (root) {
  void bootstrap().finally(() => render(() => <App />, root));
}
