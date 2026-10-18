// The bodies of the first-run steps. Every choice applies at once (language, theme) or is
// kept for the finish (character); nothing here blocks: each step can be skipped.
import { createSignal, For, onMount, Show } from 'solid-js';
import type { CharacterRef } from '../../features/characters';
import { Avatar, CharacterPicker } from '../../features/characters';
import { LOCALES, locale, t } from '../../features/i18n';
import { newProviderEntry, presetLabel, PRESETS } from '../../features/providers';
import { settings, updateSettings } from '../../features/settings';
import { ProviderEditor } from '../../features/settings/sections/ProviderEditor';
import { AGENT_KINDS, cliFor, kindLabel, loadCliStatus } from '../../features/terminal';
import { activeTheme, selectTheme, themes } from '../../features/theme';
import { currentWorkspace, folderName, openFolder } from '../../features/workspace';
import type { Locale, ProviderPreset } from '../../shared/types';
import { Button } from '../../shared/ui/Button';
import { Select } from '../../shared/ui/Field';
import { Icon } from '../../shared/ui/Icon';

export function WelcomeStep() {
  return (
    <div class="af-onboarding-welcome">
      <div class="af-onboarding-cast" aria-hidden="true">
        <For each={['owl', 'fox', 'panda']}>
          {(id) => <Avatar character={{ id, palette: 0 }} state="idle" scale={4} />}
        </For>
      </div>
      <p>{t('onboarding.welcome.text')}</p>
    </div>
  );
}

export function LanguageStep() {
  return (
    <div class="af-onboarding-options" role="radiogroup" aria-label={t('onboarding.language')}>
      <For each={LOCALES}>
        {(entry) => (
          <button
            type="button"
            role="radio"
            class="af-card af-onboarding-option"
            classList={{ selected: locale() === entry.code }}
            aria-checked={locale() === entry.code}
            onClick={() => updateSettings((s) => (s.language = entry.code satisfies Locale))}
          >
            <Icon name="languages" />
            {entry.name}
          </button>
        )}
      </For>
    </div>
  );
}

export function ThemeStep() {
  return (
    <div class="af-onboarding-options" role="radiogroup" aria-label={t('onboarding.theme')}>
      <For each={themes().filter((theme) => theme.builtin)}>
        {(theme) => (
          <button
            type="button"
            role="radio"
            class="af-card af-onboarding-option"
            classList={{ selected: activeTheme()?.id === theme.id }}
            aria-checked={activeTheme()?.id === theme.id}
            onClick={() => void selectTheme(theme.id)}
          >
            <Icon name={theme.kind === 'light' ? 'sun' : 'moon'} />
            {theme.name}
          </button>
        )}
      </For>
      <p class="af-onboarding-note">{t('onboarding.theme.live')}</p>
    </div>
  );
}

export function CharacterStep(props: {
  character: CharacterRef | null;
  onPick: (character: CharacterRef) => void;
}) {
  return (
    <div class="af-onboarding-character">
      <CharacterPicker current={props.character} onPick={props.onPick} />
    </div>
  );
}

export function ConnectStep() {
  const [adding, setAdding] = createSignal<string | null>(null);
  onMount(() => void loadCliStatus());
  const preset = () => PRESETS[0]?.id ?? 'anthropic';
  const added = () => settings.providers.find((p) => p.id === adding());
  const add = (presetId: string) => {
    const entry = newProviderEntry(presetId as ProviderPreset);
    updateSettings((s) => s.providers.push(entry));
    setAdding(entry.id);
  };
  return (
    <div class="af-onboarding-connect">
      <h3 class="af-onboarding-subtitle">{t('onboarding.connect.clis')}</h3>
      <ul class="af-onboarding-clis">
        <For each={AGENT_KINDS}>
          {(kind) => {
            const found = () => cliFor(kind)?.found ?? false;
            return (
              <li classList={{ found: found() }}>
                <Icon name={found() ? 'circle-check' : 'circle-alert'} />
                <strong>{kindLabel(kind)}</strong>
                <span>
                  {found() ? t('onboarding.connect.found') : t('onboarding.connect.missing')}
                </span>
              </li>
            );
          }}
        </For>
      </ul>
      <h3 class="af-onboarding-subtitle">{t('onboarding.connect.api')}</h3>
      <Show
        when={added()}
        fallback={
          <div class="af-onboarding-row">
            <Select
              aria-label={t('onboarding.connect.api')}
              value={preset()}
              onChange={(e) => add(e.currentTarget.value)}
            >
              <option value="" disabled selected>
                {t('onboarding.connect.pick')}
              </option>
              <For each={PRESETS}>
                {(entry) => <option value={entry.id}>{presetLabel(entry.id)}</option>}
              </For>
            </Select>
          </div>
        }
      >
        {(entry) => <ProviderEditor entry={entry()} />}
      </Show>
    </div>
  );
}

export function WorkspaceStep() {
  return (
    <div class="af-onboarding-workspace">
      <Show
        when={currentWorkspace()}
        fallback={<p class="af-onboarding-note">{t('onboarding.workspace.none')}</p>}
      >
        {(path) => (
          <p class="af-onboarding-path">
            <Icon name="folder-open" />
            <strong>{folderName(path())}</strong>
            <span>{path()}</span>
          </p>
        )}
      </Show>
      <Button
        variant={currentWorkspace() ? 'secondary' : 'primary'}
        icon="folder-open"
        onClick={() => void openFolder()}
      >
        {t(currentWorkspace() ? 'onboarding.workspace.change' : 'workspace.open')}
      </Button>
    </div>
  );
}
