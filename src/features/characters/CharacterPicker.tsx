// A grid of all characters plus the color variants of the chosen one. Used by the tab
// avatar's popover; the Characters view has its own bigger, animated gallery.
import { For, onMount, Show } from 'solid-js';
import { t } from '../i18n';
import { Avatar } from './Avatar';
import { characterById, characters, loadCharacters } from './catalog';
import type { CharacterRef } from './types';

interface CharacterPickerProps {
  current: CharacterRef | null;
  onPick: (character: CharacterRef) => void;
}

export function CharacterPicker(props: CharacterPickerProps) {
  onMount(() => void loadCharacters());
  const currentSheet = () => (props.current ? characterById(props.current.id) : undefined);
  return (
    <div class="af-picker">
      <div class="af-picker-grid" role="listbox" aria-label={t('characters.picker.title')}>
        <For each={characters()}>
          {(sheet) => (
            <button
              type="button"
              class="af-picker-cell"
              classList={{ selected: props.current?.id === sheet.id }}
              role="option"
              aria-selected={props.current?.id === sheet.id}
              title={sheet.name}
              onClick={() => props.onPick({ id: sheet.id, palette: 0 })}
            >
              <Avatar
                character={{ id: sheet.id, palette: 0 }}
                state="idle"
                scale={2}
                animate={false}
              />
            </button>
          )}
        </For>
      </div>
      <Show when={currentSheet()}>
        {(sheet) => (
          <div class="af-picker-palettes">
            <span class="af-picker-name">{sheet().name}</span>
            <For each={sheet().palettes}>
              {(palette, index) => (
                <button
                  type="button"
                  class="af-swatch"
                  classList={{ selected: props.current?.palette === index() }}
                  style={{ background: palette[1] ?? palette[0] ?? 'transparent' }}
                  title={t('characters.palette', { n: index() + 1 })}
                  aria-label={t('characters.palette', { n: index() + 1 })}
                  onClick={() => props.onPick({ id: sheet().id, palette: index() })}
                />
              )}
            </For>
          </div>
        )}
      </Show>
    </div>
  );
}
