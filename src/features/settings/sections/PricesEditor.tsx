// Prices (USD per 1M tokens): the shipped list with its source and date, OpenRouter's
// prices from its models endpoint, the user's overrides (edit, add a custom model) and
// providers marked free ($0). Overrides are saved by the backend, which reprices at once.
import { createResource, createSignal, For, onCleanup, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import type { PriceList, PriceOverride } from '../../../shared/types';
import { reportError, t } from '../../i18n';
import { providerName } from '../../providers';
import { onRepriced } from '../../tokens';
import { setPriceOverrides, settings, updateSettings } from '../settingsStore';
import { MAX_PRICE, parseMoney, removeOverride, setFree, upsertOverride } from './usageForm';
import { Button } from '../../../shared/ui/Button';
import { Checkbox, Input } from '../../../shared/ui/Field';

const FIELDS = ['input', 'output', 'cacheRead', 'cacheWrite'] as const;
type Field = (typeof FIELDS)[number];
const FIELD_LABEL = {
  input: 'settings.usage.price.input',
  output: 'settings.usage.price.output',
  cacheRead: 'settings.usage.price.cacheRead',
  cacheWrite: 'settings.usage.price.cacheWrite',
} as const;

const blank = (model = ''): PriceOverride => ({
  model,
  input: 0,
  output: 0,
  cacheRead: 0,
  cacheWrite: 0,
});

export function PricesEditor() {
  const [prices, { refetch }] = createResource<PriceList>(() => ipc.usagePrices());
  const [filter, setFilter] = createSignal('');
  const [draft, setDraft] = createSignal<PriceOverride>(blank());
  onCleanup(onRepriced(() => void refetch()));

  const save = async (next: PriceOverride[]) => {
    try {
      await setPriceOverrides(next);
      await refetch();
    } catch (err) {
      reportError('error.settingsSave', err);
    }
  };
  const shipped = () => {
    const query = filter().trim().toLowerCase();
    return (prices()?.shipped ?? []).filter((entry) => entry.model.toLowerCase().includes(query));
  };

  return (
    <>
      <h3 class="af-settings-subtitle">{t('settings.usage.free')}</h3>
      <Show
        when={settings.providers.length > 0}
        fallback={<p class="af-settings-hint">{t('settings.usage.free.none')}</p>}
      >
        <For each={settings.providers}>
          {(entry) => (
            <label class="af-settings-check">
              <Checkbox
                checked={settings.usage.freeProviders.includes(entry.id)}
                onChange={(event) => {
                  const free = event.currentTarget.checked;
                  updateSettings(
                    (s) => (s.usage.freeProviders = setFree(s.usage.freeProviders, entry.id, free)),
                  );
                }}
              />
              {providerName(entry)}
            </label>
          )}
        </For>
      </Show>

      <h3 class="af-settings-subtitle">{t('settings.usage.overrides')}</h3>
      <p class="af-settings-hint">{t('settings.usage.overrides.hint')}</p>
      <table class="af-settings-table af-price-table">
        <thead>
          <tr>
            <th>{t('settings.usage.price.model')}</th>
            <For each={FIELDS}>{(field) => <th>{t(FIELD_LABEL[field])}</th>}</For>
            <th />
          </tr>
        </thead>
        <tbody>
          <For each={settings.usage.prices}>
            {(price) => (
              <tr>
                <td>{price.model}</td>
                <For each={FIELDS}>
                  {(field) => (
                    <td>
                      <PriceInput
                        value={price[field]}
                        label={t(FIELD_LABEL[field])}
                        onValue={(value) =>
                          void save(
                            upsertOverride(settings.usage.prices, { ...price, [field]: value }),
                          )
                        }
                      />
                    </td>
                  )}
                </For>
                <td>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => void save(removeOverride(settings.usage.prices, price.model))}
                  >
                    {t('settings.usage.remove')}
                  </Button>
                </td>
              </tr>
            )}
          </For>
          <tr>
            <td>
              <Input
                placeholder={t('settings.usage.price.modelPlaceholder')}
                aria-label={t('settings.usage.price.model')}
                value={draft().model}
                onInput={(event) => {
                  const model = event.currentTarget.value;
                  setDraft((d) => ({ ...d, model }));
                }}
              />
            </td>
            <For each={FIELDS}>
              {(field: Field) => (
                <td>
                  <PriceInput
                    value={draft()[field]}
                    label={t(FIELD_LABEL[field])}
                    onValue={(value) => setDraft((d) => ({ ...d, [field]: value }))}
                  />
                </td>
              )}
            </For>
            <td>
              <Button
                size="sm"
                disabled={!draft().model.trim()}
                onClick={() => {
                  void save(upsertOverride(settings.usage.prices, draft()));
                  setDraft(blank());
                }}
              >
                {t('settings.usage.price.add')}
              </Button>
            </td>
          </tr>
        </tbody>
      </table>

      <h3 class="af-settings-subtitle">{t('settings.usage.shipped')}</h3>
      <Input
        placeholder={t('settings.usage.shipped.filter')}
        aria-label={t('settings.usage.shipped.filter')}
        value={filter()}
        onInput={(event) => setFilter(event.currentTarget.value)}
      />
      <div class="af-price-list">
        <table class="af-settings-table af-price-table">
          <thead>
            <tr>
              <th>{t('settings.usage.price.model')}</th>
              <For each={FIELDS}>{(field) => <th>{t(FIELD_LABEL[field])}</th>}</For>
              <th>{t('settings.usage.price.source')}</th>
            </tr>
          </thead>
          <tbody>
            <For each={shipped()}>
              {(entry) => (
                <tr title={entry.note ?? undefined}>
                  <td>{entry.model}</td>
                  <For each={FIELDS}>{(field) => <td class="numeric">{entry[field]}</td>}</For>
                  <td>
                    <Button
                      variant="ghost"
                      size="sm"
                      icon="external-link"
                      title={entry.source}
                      onClick={() =>
                        ipc
                          .openUrl(entry.source)
                          .catch((err: unknown) => reportError('error.openLink', err))
                      }
                    >
                      {t('settings.usage.price.updated', { date: entry.updated })}
                    </Button>
                  </td>
                </tr>
              )}
            </For>
            <For each={prices()?.dynamic ?? []}>
              {(entry) => (
                <tr>
                  <td>{entry.model}</td>
                  <For each={FIELDS}>{(field) => <td class="numeric">{entry[field]}</td>}</For>
                  <td class="af-settings-hint">{t('settings.usage.price.openrouter')}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </>
  );
}

function PriceInput(props: { value: number; label: string; onValue: (value: number) => void }) {
  return (
    <Input
      class="af-input-money"
      inputmode="decimal"
      aria-label={props.label}
      value={String(props.value)}
      onChange={(event) => {
        const value = parseMoney(event.currentTarget.value, MAX_PRICE);
        if (value === null) event.currentTarget.value = String(props.value);
        else props.onValue(value);
      }}
    />
  );
}
