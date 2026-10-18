// "Usage & budgets": CLI log reading, the subscription label, prompt caching, compaction
// (threshold, kept turns, summary model, auto-compact), budgets and prices.
import { For, Show } from 'solid-js';
import { t } from '../../i18n';
import { modelsOf, providerName } from '../../providers';
import { settings, updateSettings } from '../settingsStore';
import { BudgetsEditor } from './BudgetsEditor';
import { PricesEditor } from './PricesEditor';
import { Row, Section } from './Section';
import { KEEP_TURNS, percentFromShare, shareFromPercent, THRESHOLD_PERCENT } from './usageForm';
import { Input, Select, Toggle } from '../../../shared/ui/Field';

function SettingToggle(props: { id: string; checked: boolean; onChange: (on: boolean) => void }) {
  return (
    <Toggle
      id={props.id}
      checked={props.checked}
      onChange={(event) => props.onChange(event.currentTarget.checked)}
    />
  );
}

export function UsageSection() {
  const compactProvider = () => settings.chat.compactProviderId;
  const modelSuggestions = () => {
    const id = compactProvider();
    return id ? modelsOf(id).list : [];
  };
  return (
    <Section title={t('settings.usage')} hint={t('settings.usage.hint')}>
      <Row label={t('settings.usage.trackCli')} for="af-usage-cli">
        <SettingToggle
          id="af-usage-cli"
          checked={settings.usage.trackCli}
          onChange={(on) => updateSettings((s) => (s.usage.trackCli = on))}
        />
      </Row>
      <Row label={t('settings.usage.subscription')} for="af-usage-sub">
        <SettingToggle
          id="af-usage-sub"
          checked={settings.usage.cliSubscription}
          onChange={(on) => updateSettings((s) => (s.usage.cliSubscription = on))}
        />
      </Row>
      <p class="af-settings-hint">{t('settings.usage.subscriptionHint')}</p>
      <Row label={t('settings.usage.caching')} for="af-usage-cache">
        <SettingToggle
          id="af-usage-cache"
          checked={settings.chat.promptCaching}
          onChange={(on) => updateSettings((s) => (s.chat.promptCaching = on))}
        />
      </Row>

      <h3 class="af-settings-subtitle">{t('settings.usage.compact')}</h3>
      <Row label={t('settings.usage.autoCompact')} for="af-usage-auto">
        <SettingToggle
          id="af-usage-auto"
          checked={settings.chat.autoCompact}
          onChange={(on) => updateSettings((s) => (s.chat.autoCompact = on))}
        />
      </Row>
      <Row label={t('settings.usage.threshold')} for="af-usage-threshold">
        <Input
          id="af-usage-threshold"
          type="number"
          min={THRESHOLD_PERCENT.min}
          max={THRESHOLD_PERCENT.max}
          step={5}
          value={percentFromShare(settings.chat.compactThreshold)}
          onChange={(event) => {
            const percent = event.currentTarget.valueAsNumber;
            if (!Number.isFinite(percent)) return;
            const share = shareFromPercent(percent);
            event.currentTarget.value = String(percentFromShare(share));
            updateSettings((s) => (s.chat.compactThreshold = share));
          }}
        />
      </Row>
      <Row label={t('settings.usage.keepTurns')} for="af-usage-keep">
        <Input
          id="af-usage-keep"
          type="number"
          min={KEEP_TURNS.min}
          max={KEEP_TURNS.max}
          value={settings.chat.compactKeepTurns}
          onChange={(event) => {
            const turns = Math.round(event.currentTarget.valueAsNumber);
            if (turns >= KEEP_TURNS.min && turns <= KEEP_TURNS.max) {
              updateSettings((s) => (s.chat.compactKeepTurns = turns));
            }
          }}
        />
      </Row>
      <Row label={t('settings.usage.compactProvider')} for="af-usage-cprov">
        <Select
          id="af-usage-cprov"
          value={compactProvider() ?? ''}
          onChange={(event) => {
            const id = event.currentTarget.value || null;
            updateSettings((s) => (s.chat.compactProviderId = id));
          }}
        >
          <option value="">{t('settings.usage.compactProvider.same')}</option>
          <For each={settings.providers}>
            {(entry) => <option value={entry.id}>{providerName(entry)}</option>}
          </For>
        </Select>
      </Row>
      <Row label={t('settings.usage.compactModel')} for="af-usage-cmodel">
        <Input
          id="af-usage-cmodel"
          list="af-usage-cmodels"
          placeholder={t('settings.usage.compactModel.same')}
          value={settings.chat.compactModel}
          onChange={(event) => {
            const model = event.currentTarget.value.trim();
            updateSettings((s) => (s.chat.compactModel = model));
          }}
        />
        <Show when={modelSuggestions().length > 0}>
          <datalist id="af-usage-cmodels">
            <For each={modelSuggestions()}>{(model) => <option value={model.id} />}</For>
          </datalist>
        </Show>
      </Row>

      <h3 class="af-settings-subtitle">{t('settings.usage.budgets')}</h3>
      <BudgetsEditor />
      <PricesEditor />
    </Section>
  );
}
