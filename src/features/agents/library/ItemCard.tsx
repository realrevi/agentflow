// One library item in the Browse list: name, description, source and trust, license,
// tools, safety flags and the approximate size. Clicking opens the detail view.
import { For, Show } from 'solid-js';
import type { LibraryItem } from '../../../shared/types';
import { t } from '../../i18n';
import { formatTokens } from '../../tokens';
import { TARGETS } from '../wizard/answers';
import { FLAG_LABELS, KIND_LABELS, TARGET_MARKS } from './labels';

export function TrustBadge(props: { verified: boolean }) {
  return (
    <span
      class="af-badge"
      classList={{ 'af-lib-verified': props.verified, 'af-lib-unverified': !props.verified }}
      title={t(props.verified ? 'library.verifiedHint' : 'library.unverifiedHint')}
    >
      {t(props.verified ? 'library.verified' : 'library.unverified')}
    </span>
  );
}

export function FlagChips(props: { item: LibraryItem }) {
  const ids = () => [...new Set(props.item.flags.map((f) => f.id))];
  return <For each={ids()}>{(id) => <span class="af-lib-flag">⚠ {t(FLAG_LABELS[id])}</span>}</For>;
}

export function ItemCard(props: { item: LibraryItem; onOpen: () => void }) {
  const item = () => props.item;
  return (
    <button type="button" class="af-lib-card" onClick={() => props.onOpen()}>
      <span class="af-lib-card-head">
        <strong class="af-lib-card-name">{item().name}</strong>
        <span class="af-badge">{t(KIND_LABELS[item().kind])}</span>
      </span>
      <span class="af-lib-card-desc">{item().description}</span>
      <span class="af-lib-card-meta">
        <span>{item().sourceName}</span>
        <TrustBadge verified={item().verified} />
        <span class="af-badge">{item().license}</span>
        <span class="af-lib-tokens">
          {t('library.tokens', { count: formatTokens(item().tokens) })}
        </span>
      </span>
      <span class="af-lib-card-meta">
        <For each={TARGETS.filter((target) => item().targets.includes(target.id))}>
          {(target) => (
            <span
              class="af-lib-target"
              classList={{ native: item().native.includes(target.id) }}
              title={t(target.label)}
            >
              {TARGET_MARKS[target.id]}
            </span>
          )}
        </For>
        <Show when={item().flags.length > 0}>
          <FlagChips item={item()} />
        </Show>
      </span>
    </button>
  );
}
