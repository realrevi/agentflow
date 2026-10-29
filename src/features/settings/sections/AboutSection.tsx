// "About": the version with its release notes, the license and third-party licenses (a lazy
// dialog), and updates: the launch check setting and a manual check / install.
import { createSignal, lazy, Match, Show, Suspense, Switch } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import { Button } from '../../../shared/ui/Button';
import { Toggle } from '../../../shared/ui/Field';
import { t } from '../../i18n';
import {
  availableUpdate,
  checkForUpdates,
  installUpdate,
  updateProgress,
  updateStatus,
} from '../../updates';
import { settings, updateSettings } from '../settingsStore';
import { Row, Section } from './Section';

const LicensesDialog = lazy(() => import('./LicensesDialog'));

export const REPO_URL = 'https://github.com/realrevi/agentflow';

/** The GitHub release page of `version` (the tag is `v` + version). */
export function releaseNotesUrl(version: string): string {
  return `${REPO_URL}/releases/tag/v${version}`;
}

function UpdateStatusText() {
  const percent = () => {
    const share = updateProgress();
    return share === null ? null : Math.round(share * 100);
  };
  return (
    <Switch>
      <Match when={updateStatus() === 'checking'}>{t('settings.about.checking')}</Match>
      <Match when={updateStatus() === 'upToDate'}>{t('settings.about.upToDate')}</Match>
      <Match when={updateStatus() === 'available' && availableUpdate()}>
        {(info) => t('settings.about.available', { version: info().version })}
      </Match>
      <Match when={updateStatus() === 'installing'}>
        {percent() === null
          ? t('settings.about.installing')
          : t('settings.about.downloading', { percent: percent() ?? 0 })}
      </Match>
    </Switch>
  );
}

export function AboutSection() {
  const [licensesOpen, setLicensesOpen] = createSignal(false);
  const busy = () => updateStatus() === 'checking' || updateStatus() === 'installing';
  return (
    <Section title={t('settings.about')} hint={t('settings.about.hint')}>
      <Row label={t('settings.about.version')}>
        <span class="af-about-version">AgentFlow {__APP_VERSION__}</span>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void ipc.openUrl(releaseNotesUrl(__APP_VERSION__))}
        >
          {t('settings.about.releaseNotes')}
        </Button>
      </Row>
      <Row label={t('settings.about.license')}>
        <span>{t('settings.about.mit')}</span>
        <Button variant="ghost" size="sm" onClick={() => setLicensesOpen(true)}>
          {t('settings.about.licenses')}
        </Button>
        <Button variant="ghost" size="sm" onClick={() => void ipc.openUrl(REPO_URL)}>
          {t('settings.about.source')}
        </Button>
      </Row>
      <Row label={t('settings.about.checkOnStartup')} for="af-about-updates">
        <Toggle
          id="af-about-updates"
          checked={settings.updates.checkOnStartup}
          onChange={(event) => {
            const on = event.currentTarget.checked;
            updateSettings((s) => (s.updates.checkOnStartup = on));
          }}
        />
      </Row>
      <Row label={t('settings.about.updates')}>
        <Show
          when={updateStatus() === 'available'}
          fallback={
            <Button
              variant="secondary"
              disabled={busy()}
              onClick={() => void checkForUpdates(true)}
            >
              {t('settings.about.checkNow')}
            </Button>
          }
        >
          <Button onClick={() => void installUpdate()}>{t('updates.install')}</Button>
        </Show>
        <span class="af-settings-hint inline" role="status">
          <UpdateStatusText />
        </span>
      </Row>
      <Show when={licensesOpen()}>
        <Suspense>
          <LicensesDialog onClose={() => setLicensesOpen(false)} />
        </Suspense>
      </Show>
    </Section>
  );
}
