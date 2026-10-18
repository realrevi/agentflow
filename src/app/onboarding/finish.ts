// After the first-run flow: one ready agent panel (the first agent CLI found, else an API
// chat with the provider just added, else a shell) with the chosen character.
import type { CharacterRef } from '../../features/characters';
import { openTerminal, setPanelCharacter, startChat } from '../../features/layout';
import { settings } from '../../features/settings';
import { AGENT_KINDS, cliFor, newTerminalConfig } from '../../features/terminal';

export async function openReadyAgent(character: CharacterRef | null): Promise<void> {
  const cli = AGENT_KINDS.find((kind) => cliFor(kind)?.found);
  const provider = settings.providers.find((p) => p.defaultModel.trim());
  let id: string | null;
  if (cli) id = openTerminal(newTerminalConfig(cli));
  else if (provider) id = await startChat(provider.id, provider.defaultModel);
  else id = openTerminal(newTerminalConfig('shell'));
  if (id && character) setPanelCharacter(id, character);
}
