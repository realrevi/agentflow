import type { JSX } from 'solid-js';
import { render } from 'solid-js/web';

/** A DOM element that dockview owns and Solid renders into. */
export interface SolidPart {
  element: HTMLElement;
  mount(view: () => JSX.Element): void;
  dispose(): void;
}

export function createSolidPart(className: string): SolidPart {
  const element = document.createElement('div');
  element.className = className;
  let disposeRoot: (() => void) | undefined;
  return {
    element,
    mount(view) {
      disposeRoot?.();
      disposeRoot = render(view, element);
    },
    dispose() {
      disposeRoot?.();
      disposeRoot = undefined;
    },
  };
}
