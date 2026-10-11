// dockview renderer factories. Each one mounts a Solid tree into the element dockview gives us.
import type {
  CreateComponentOptions,
  DockviewGroupPanel,
  IContentRenderer,
  IHeaderActionsRenderer,
  IWatermarkRenderer,
} from 'dockview-core';
import { lazy, Match, Suspense, Switch } from 'solid-js';
import { t } from '../i18n';
import { AddPanelButton } from './AddPanelButton';
import { isPanelKind, parsePanelParams, type PanelParams } from './panels';
import { PlaceholderPanel } from './PlaceholderPanel';
import { createSolidPart } from './solidPart';
import { Watermark } from './Watermark';

// Lazy: the settings UI is only downloaded and parsed when its panel is first opened.
const SettingsPanel = lazy(() => import('../settings/SettingsPanel'));

function PanelView(props: { params: PanelParams }) {
  return (
    <Switch fallback={<PlaceholderPanel params={props.params} />}>
      <Match when={props.params.kind === 'settings'}>
        <Suspense fallback={<div class="af-panel-loading">{t('panel.loading')}</div>}>
          <SettingsPanel />
        </Suspense>
      </Match>
    </Switch>
  );
}

export function createContentRenderer(options: CreateComponentOptions): IContentRenderer {
  const part = createSolidPart('af-panel');
  return {
    element: part.element,
    init(init) {
      const fallbackKind = isPanelKind(options.name) ? options.name : 'agent';
      const params = parsePanelParams(init.params) ?? { kind: fallbackKind, index: 1 };
      part.mount(() => <PanelView params={params} />);
    },
    dispose: part.dispose,
  };
}

export function createHeaderActions(group: DockviewGroupPanel): IHeaderActionsRenderer {
  const part = createSolidPart('af-header-actions');
  return {
    element: part.element,
    init: () => part.mount(() => <AddPanelButton groupId={group.id} />),
    dispose: part.dispose,
  };
}

export function createWatermark(): IWatermarkRenderer {
  const part = createSolidPart('af-watermark-host');
  return {
    element: part.element,
    init: () => part.mount(() => <Watermark />),
    dispose: part.dispose,
  };
}
