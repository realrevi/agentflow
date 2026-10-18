import { createSignal, Show } from 'solid-js';
import { LOCALES, locale, t } from '../features/i18n';
import { settings, updateSettings } from '../features/settings';
import { openPanel } from '../features/layout';
import { activeTheme, selectTheme, themes } from '../features/theme';
import { costText, formatTokens, totalTokens, usageSummary } from '../features/tokens';
import type { Locale } from '../shared/types';
import { globeIcon, paletteIcon, stopIcon, usageIcon } from '../shared/ui/icons';
import { anyAutoRunning, stopAll } from '../features/workstreams';
import { Menu, type MenuItem } from '../shared/ui/Menu';
import { PixelIcon } from '../shared/ui/PixelIcon';
import { WorkspaceStatus } from './WorkspaceStatus';

type OpenMenu = 'theme' | 'language' | null;

export function StatusBar() {
  const [open, setOpen] = createSignal<OpenMenu>(null);
  let themeButton: HTMLButtonElement | undefined;
  let languageButton: HTMLButtonElement | undefined;

  const toggle = (menu: Exclude<OpenMenu, null>) =>
    setOpen((current) => (current === menu ? null : menu));
  const localeName = () => LOCALES.find((entry) => entry.code === locale())?.name ?? locale();

  const themeItems = (): MenuItem[] => {
    let lastGroup: boolean | undefined;
    return themes().map((theme) => {
      const heading =
        theme.builtin === lastGroup
          ? undefined
          : t(theme.builtin ? 'settings.theme.builtin' : 'settings.theme.custom');
      lastGroup = theme.builtin;
      return {
        id: theme.id,
        label: theme.name,
        checked: theme.id === settings.appearance.themeId,
        ...(heading ? { heading } : {}),
      };
    });
  };
  const languageItems = (): MenuItem[] =>
    LOCALES.map((entry) => ({
      id: entry.code,
      label: entry.name,
      checked: entry.code === locale(),
    }));

  const selectLanguage = (code: string) => {
    const entry = LOCALES.find((candidate) => candidate.code === code);
    if (entry) updateSettings((s) => (s.language = entry.code satisfies Locale));
  };

  return (
    <footer class="af-statusbar">
      <span
        class="af-status-item af-status-brand"
        title={t('status.version', { version: __APP_VERSION__ })}
      >
        AgentFlow v{__APP_VERSION__}
      </span>
      <WorkspaceStatus />
      <div class="af-statusbar-spacer" />
      <Show when={anyAutoRunning()}>
        <button
          type="button"
          class="af-status-item af-status-stop"
          title={t('status.stopWorkstreams')}
          onClick={stopAll}
        >
          <PixelIcon art={stopIcon} scale={1.5} />
          {t('status.stopWorkstreams')}
        </button>
      </Show>
      <Show when={usageSummary()?.today}>
        {(today) => (
          <button
            type="button"
            class="af-status-item"
            title={t('status.usage', { tokens: formatTokens(totalTokens(today())) })}
            onClick={() => openPanel('usage')}
          >
            <PixelIcon art={usageIcon} scale={1} />
            {t('status.usageToday', { cost: costText(today()) })}
          </button>
        )}
      </Show>
      <button
        ref={themeButton}
        type="button"
        class="af-status-item"
        title={t('status.theme')}
        aria-haspopup="menu"
        aria-expanded={open() === 'theme'}
        onClick={() => toggle('theme')}
      >
        <PixelIcon art={paletteIcon} scale={1.5} />
        {activeTheme()?.name ?? settings.appearance.themeId}
      </button>
      <button
        ref={languageButton}
        type="button"
        class="af-status-item"
        title={t('status.language')}
        aria-haspopup="menu"
        aria-expanded={open() === 'language'}
        onClick={() => toggle('language')}
      >
        <PixelIcon art={globeIcon} scale={1.5} />
        {localeName()}
      </button>

      <Show when={open() === 'theme' && themeButton}>
        {(anchor) => (
          <Menu
            anchor={anchor()}
            items={themeItems()}
            placement="above-end"
            onSelect={selectTheme}
            onClose={() => setOpen(null)}
          />
        )}
      </Show>
      <Show when={open() === 'language' && languageButton}>
        {(anchor) => (
          <Menu
            anchor={anchor()}
            items={languageItems()}
            placement="above-end"
            onSelect={selectLanguage}
            onClose={() => setOpen(null)}
          />
        )}
      </Show>
    </footer>
  );
}
