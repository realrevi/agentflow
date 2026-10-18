// What a workstream member can be in the "New workstream" dialog: an open agent panel that
// is in no workstream yet, a new agent CLI, a new API agent or an installed AgentFlow
// agent. Creating the member opens its panel and returns the panel id.
import { agentProfiles } from '../../features/agents';
import { t } from '../../features/i18n';
import { openPanels, openProfileChat, openTerminal, startChat } from '../../features/layout';
import { providerName } from '../../features/providers';
import { settings } from '../../features/settings';
import { AGENT_KINDS, kindLabel, newTerminalConfig } from '../../features/terminal';
import { workstreamOf } from '../../features/workstreams';

export interface MemberOption {
  /** `panel:<id>`, `cli:<kind>`, `api:<provider id>` or `profile:<id>`. */
  value: string;
  label: string;
  group: 'open' | 'new';
}

export function memberOptions(): MemberOption[] {
  const open = openPanels()
    .filter((panel) => panel.agent && !workstreamOf(panel.id))
    .map((panel) => ({ value: `panel:${panel.id}`, label: panel.title, group: 'open' as const }));
  const clis = AGENT_KINDS.map((kind) => ({
    value: `cli:${kind}`,
    label: t('workstream.dialog.newCli', { name: kindLabel(kind) }),
    group: 'new' as const,
  }));
  const apis = settings.providers
    .filter((entry) => entry.defaultModel.trim())
    .map((entry) => ({
      value: `api:${entry.id}`,
      label: t('workstream.dialog.newApi', {
        name: `${providerName(entry)} · ${entry.defaultModel}`,
      }),
      group: 'new' as const,
    }));
  const profiles = agentProfiles().map((profile) => ({
    value: `profile:${profile.id}`,
    label: t('panel.add.profile', { name: profile.name }),
    group: 'new' as const,
  }));
  return [...open, ...clis, ...apis, ...profiles];
}

/** Opens (or picks) the member's panel; null if it could not be created. */
export async function createMember(value: string): Promise<string | null> {
  const [type, id = ''] = splitOnce(value);
  if (type === 'panel') return id;
  if (type === 'cli') {
    const kind = AGENT_KINDS.find((k) => k === id);
    return kind ? openTerminal(newTerminalConfig(kind)) : null;
  }
  if (type === 'api') {
    const entry = settings.providers.find((p) => p.id === id);
    return entry ? startChat(entry.id, entry.defaultModel) : null;
  }
  const profile = agentProfiles().find((p) => p.id === id);
  return profile ? openProfileChat(profile) : null;
}

function splitOnce(value: string): [string, string] {
  const at = value.indexOf(':');
  return at < 0 ? [value, ''] : [value.slice(0, at), value.slice(at + 1)];
}

/** Protocol names must be unique in a workstream: "Fox", "Fox 2", ... */
export function uniqueNames(wanted: readonly [string, string][]): Record<string, string> {
  const out: Record<string, string> = {};
  const used = new Set<string>();
  for (const [panel, name] of wanted) {
    let candidate = name;
    for (let n = 2; used.has(candidate.toLowerCase()); n++) candidate = `${name} ${n}`;
    used.add(candidate.toLowerCase());
    out[panel] = candidate;
  }
  return out;
}
