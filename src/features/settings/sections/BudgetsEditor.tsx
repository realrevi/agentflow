// Budgets: daily and monthly limits for all sources and per provider (API entries and the
// agent CLIs), plus the hard stop (API chats only; CLI agents can only be warned).
import { createSignal, For, Show } from 'solid-js';
import type { Budgets } from '../../../shared/types';
import { t } from '../../i18n';
import { providerName } from '../../providers';
import { scopeLabel } from '../../tokens';
import { settings, updateSettings } from '../settingsStore';
import { Row } from './Section';
import { moneyText, parseMoney, setProviderBudget } from './usageForm';
import { Button } from '../../../shared/ui/Button';
import { Input, Toggle } from '../../../shared/ui/Field';

const CLI_KEYS = ['claude-code', 'codex', 'gemini-cli'];

/** `change` is pure: it gets a plain copy and returns the new budgets. */
function setBudgets(change: (budgets: Budgets) => Budgets): void {
  updateSettings((s) => {
    const current = s.usage.budgets;
    const plain: Budgets = {
      daily: current.daily,
      monthly: current.monthly,
      hardStop: current.hardStop,
      providers: current.providers.map((entry) => ({ ...entry })),
    };
    s.usage.budgets = change(plain);
  });
}

function MoneyInput(props: {
  id?: string;
  value: number | null;
  onValue: (v: number | null) => void;
  label: string;
}) {
  return (
    <Input
      class="af-input-money"
      id={props.id}
      inputmode="decimal"
      placeholder={t('settings.usage.budget.none')}
      aria-label={props.label}
      value={moneyText(props.value)}
      onChange={(event) => {
        const text = event.currentTarget.value;
        const value = parseMoney(text);
        if (value !== null || text.trim() === '') props.onValue(value === 0 ? null : value);
        else event.currentTarget.value = moneyText(props.value);
      }}
    />
  );
}

export function BudgetsEditor() {
  const [adding, setAdding] = createSignal('');
  const budgets = () => settings.usage.budgets;
  const candidates = () => {
    const used = new Set(budgets().providers.map((entry) => entry.provider));
    const api = settings.providers.map((entry) => ({ id: entry.id, label: providerName(entry) }));
    const cli = CLI_KEYS.map((id) => ({ id, label: scopeLabel(id) }));
    return [...api, ...cli].filter((entry) => !used.has(entry.id));
  };

  return (
    <>
      <Row label={t('settings.usage.budget.daily')} for="af-budget-daily">
        <MoneyInput
          id="af-budget-daily"
          label={t('settings.usage.budget.daily')}
          value={budgets().daily}
          onValue={(daily) => setBudgets((b) => ({ ...b, daily }))}
        />
      </Row>
      <Row label={t('settings.usage.budget.monthly')} for="af-budget-monthly">
        <MoneyInput
          id="af-budget-monthly"
          label={t('settings.usage.budget.monthly')}
          value={budgets().monthly}
          onValue={(monthly) => setBudgets((b) => ({ ...b, monthly }))}
        />
      </Row>
      <Show when={budgets().providers.length > 0}>
        <table class="af-settings-table">
          <thead>
            <tr>
              <th>{t('settings.usage.budget.provider')}</th>
              <th>{t('settings.usage.budget.daily')}</th>
              <th>{t('settings.usage.budget.monthly')}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <For each={budgets().providers}>
              {(entry) => (
                <tr>
                  <td>{scopeLabel(entry.provider)}</td>
                  <td>
                    <MoneyInput
                      label={t('settings.usage.budget.daily')}
                      value={entry.daily}
                      onValue={(daily) =>
                        setBudgets((b) => setProviderBudget(b, entry.provider, { daily }))
                      }
                    />
                  </td>
                  <td>
                    <MoneyInput
                      label={t('settings.usage.budget.monthly')}
                      value={entry.monthly}
                      onValue={(monthly) =>
                        setBudgets((b) => setProviderBudget(b, entry.provider, { monthly }))
                      }
                    />
                  </td>
                  <td>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() =>
                        setBudgets((b) =>
                          setProviderBudget(b, entry.provider, { daily: null, monthly: null }),
                        )
                      }
                    >
                      {t('settings.usage.remove')}
                    </Button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
      <Show when={candidates().length > 0}>
        <Row label={t('settings.usage.budget.add')} for="af-budget-add">
          <select
            id="af-budget-add"
            class="af-input"
            value={adding()}
            onChange={(event) => {
              const provider = event.currentTarget.value;
              setAdding('');
              event.currentTarget.value = '';
              // A new row starts with a monthly limit to fill in.
              if (provider) setBudgets((b) => setProviderBudget(b, provider, { monthly: 10 }));
            }}
          >
            <option value="">{t('settings.usage.budget.pick')}</option>
            <For each={candidates()}>
              {(entry) => <option value={entry.id}>{entry.label}</option>}
            </For>
          </select>
        </Row>
      </Show>
      <Row label={t('settings.usage.budget.hardStop')} for="af-budget-stop">
        <Toggle
          id="af-budget-stop"
          checked={budgets().hardStop}
          onChange={(event) => {
            const on = event.currentTarget.checked;
            setBudgets((b) => ({ ...b, hardStop: on }));
          }}
        />
      </Row>
      <p class="af-settings-hint">{t('settings.usage.budget.hint')}</p>
    </>
  );
}
