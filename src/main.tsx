import './assets/fonts/fonts.css';
import './shared/ui/tokens.css';
import './app/styles/base.css';
import './shared/ui/controls.css';
import './shared/ui/dialog.css';
import './app/styles/shell.css';
import './app/styles/panels.css';
import './app/styles/statusbar.css';
import './app/styles/characters.css';
import './app/styles/workstreams.css';
import './shared/ui/menu.css';
import { render } from 'solid-js/web';
import App from './App';
import { bootstrap } from './app/bootstrap';

const root = document.getElementById('root');
if (root) {
  void bootstrap().finally(() => render(() => <App />, root));
}
