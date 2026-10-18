// All characters, animating, with a color variant switch. Clicking one gives it to the
// focused agent panel. The animation loop runs only while this view is on screen.
import { createMemo, createSignal, For, onMount } from 'solid-js';
import { Avatar, characters, loadCharacters } from '../../features/characters';
import { reportError, t } from '../../features/i18n';
import { focusedAgent, openPanels, setPanelCharacter } from '../../features/layout';
import { settings } from '../../features/settings';
import { SegmentedControl } from '../../shared/ui/Segmented';

const GALLERY_SCALE = 3;

export function CharactersView() {
  const [palette, setPalette] = createSignal(0);
  const [hovered, setHovered] = createSignal<string | null>(null);
  onMount(
    () => void loadCharacters().catch((err: unknown) => reportError('error.characters', err)),
  );

  const target = createMemo(() => {
    const agent = focusedAgent();
    return agent ? openPanels().find((panel) => panel.id === agent.id) : undefined;
  });
  const paletteCount = () => Math.max(1, ...characters().map((sheet) => sheet.palettes.length));
  const colors = () => Array.from({ length: paletteCount() }, (_, index) => index);

  return (
    <div class="af-view af-characters">
      <p class="af-view-hint">
        {target()
          ? t('characters.view.assignTo', { panel: target()?.title ?? '' })
          : t('characters.view.noPanel')}
      </p>
      <SegmentedControl
        label={t('characters.view.colors')}
        value={String(palette())}
        onChange={(value) => setPalette(Number(value))}
        options={colors().map((index) => ({
          value: String(index),
          label: t('characters.palette', { n: index + 1 }),
        }))}
      />
      <ul class="af-gallery">
        <For each={characters()}>
          {(sheet) => {
            const ref = () => ({
              id: sheet.id,
              palette: Math.min(palette(), sheet.palettes.length - 1),
            });
            const current = () => target()?.character?.id === sheet.id;
            return (
              <li>
                <button
                  type="button"
                  class="af-gallery-cell"
                  classList={{ selected: current() }}
                  aria-pressed={current()}
                  aria-disabled={target() ? undefined : 'true'}
                  onPointerEnter={() => setHovered(sheet.id)}
                  onPointerLeave={() => setHovered(null)}
                  onClick={() => {
                    const panel = target();
                    if (panel) setPanelCharacter(panel.id, ref());
                  }}
                >
                  <Avatar
                    character={ref()}
                    state={hovered() === sheet.id ? 'walk' : 'idle'}
                    scale={GALLERY_SCALE}
                    speed={settings.characters.speed}
                    showcase
                  />
                  <span class="af-gallery-name">{sheet.name}</span>
                </button>
              </li>
            );
          }}
        </For>
      </ul>
    </div>
  );
}
