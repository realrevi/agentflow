// Thin Solid wrapper around the vanilla dockview-core engine.
// The stylesheet ships in the `dockview` package, which re-exports dockview-core.
import 'dockview/dist/styles/dockview.css';
import './dockview-theme.css';
import { createDockview, type DockviewTheme } from 'dockview-core';
import { onCleanup, onMount } from 'solid-js';
import { attachLayout } from './layoutStore';
import {
  createContentRenderer,
  createHeaderActions,
  createTabRenderer,
  createWatermark,
} from './renderers';

/** All colors come from `--af-*` variables (see dockview-theme.css), so one theme fits all. */
const DOCK_THEME: DockviewTheme = {
  name: 'agentflow',
  className: 'af-dockview',
  dndOverlayMounting: 'absolute',
  dndPanelOverlay: 'group',
};

export function DockviewHost() {
  let host: HTMLDivElement | undefined;

  onMount(() => {
    if (!host) return;
    const dock = createDockview(host, {
      theme: DOCK_THEME,
      createComponent: createContentRenderer,
      createTabComponent: createTabRenderer,
      createRightHeaderActionComponent: createHeaderActions,
      createWatermarkComponent: createWatermark,
    });
    const detach = attachLayout(dock);
    onCleanup(() => {
      detach();
      dock.dispose();
    });
  });

  return <div ref={host} class="af-dock" />;
}
