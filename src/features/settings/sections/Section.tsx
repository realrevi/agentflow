import type { JSX } from 'solid-js';

export function Section(props: { title: string; hint?: string; children: JSX.Element }) {
  return (
    <section class="af-settings-section">
      <h2 class="af-settings-title">{props.title}</h2>
      {props.hint && <p class="af-settings-hint">{props.hint}</p>}
      {props.children}
    </section>
  );
}

export function Row(props: { label: string; hint?: string; for?: string; children: JSX.Element }) {
  return (
    <div class="af-settings-row">
      <label class="af-settings-label" for={props.for}>
        {props.label}
        {props.hint && <span class="af-settings-hint">{props.hint}</span>}
      </label>
      <div class="af-settings-control">{props.children}</div>
    </div>
  );
}
