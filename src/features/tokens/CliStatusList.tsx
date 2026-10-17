// Which agent CLIs' usage AgentFlow reads (their own session logs, read-only).
import { createResource, For, Show } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { CliSupport } from '../../shared/types';
import { t } from '../i18n';
import { SOURCE_COLOR, SOURCE_LABEL } from './labels';

function statusKey(cli: CliSupport) {
  if (!cli.available) return 'usage.cli.unavailable' as const;
  if (cli.watching) return 'usage.cli.watching' as const;
  if (!cli.found) return 'usage.cli.notFound' as const;
  return 'usage.cli.off' as const;
}

export function CliStatusList() {
  const [status] = createResource(() =>
    ipc.usageCliStatus().catch((err: unknown) => {
      console.debug('[agentflow] CLI status:', err);
      return [] as CliSupport[];
    }),
  );
  return (
    <ul class="af-usage-cli">
      <For each={status() ?? []}>
        {(cli) => (
          <li title={cli.root ?? undefined}>
            <i style={{ background: SOURCE_COLOR[cli.source] }} />
            <span class="af-usage-cli-name">{t(SOURCE_LABEL[cli.source])}</span>
            <span class="af-usage-muted" classList={{ ok: cli.watching }}>
              {t(statusKey(cli))}
            </span>
          </li>
        )}
      </For>
      <Show when={status() && (status() ?? []).length === 0}>
        <li class="af-usage-muted">{t('usage.cli.unknown')}</li>
      </Show>
    </ul>
  );
}
