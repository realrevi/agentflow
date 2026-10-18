import { t } from '../features/i18n';
import { SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, updateSettings } from '../features/settings';

/** Drag handle on the sidebar's right edge. Pointer-driven, no polling. */
export function SidebarSash() {
  const onPointerDown = (event: PointerEvent & { currentTarget: HTMLDivElement }) => {
    const sash = event.currentTarget;
    const sidebar = sash.parentElement;
    if (!sidebar) return;
    const left = sidebar.getBoundingClientRect().left;
    sash.setPointerCapture(event.pointerId);
    document.body.classList.add('af-resizing');

    const onMove = (move: PointerEvent) => {
      const width = Math.round(move.clientX - left);
      const clamped = Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, width));
      updateSettings((s) => (s.ui.sidebarWidth = clamped));
    };
    const onUp = () => {
      document.body.classList.remove('af-resizing');
      sash.removeEventListener('pointermove', onMove);
      sash.removeEventListener('pointerup', onUp);
      sash.removeEventListener('pointercancel', onUp);
    };
    sash.addEventListener('pointermove', onMove);
    sash.addEventListener('pointerup', onUp);
    sash.addEventListener('pointercancel', onUp);
  };

  return (
    <div
      class="af-sidebar-sash"
      role="separator"
      aria-orientation="vertical"
      title={t('sidebar.resize')}
      onPointerDown={onPointerDown}
    />
  );
}

/**
 * Shown while the sidebar is hidden: a slim strip on the activity bar's edge. Click it to
 * bring the sidebar back, or drag it out to the width you want.
 */
export function SidebarHandle() {
  const onPointerDown = (event: PointerEvent & { currentTarget: HTMLButtonElement }) => {
    const handle = event.currentTarget;
    const startX = event.clientX;
    const left = handle.getBoundingClientRect().left;
    let dragged = false;
    handle.setPointerCapture(event.pointerId);
    const onMove = (move: PointerEvent) => {
      if (!dragged && Math.abs(move.clientX - startX) < 4) return;
      dragged = true;
      const width = Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, move.clientX - left));
      updateSettings((s) => {
        s.ui.sidebarVisible = true;
        s.ui.sidebarWidth = Math.round(width);
      });
    };
    const onUp = () => {
      handle.removeEventListener('pointermove', onMove);
      handle.removeEventListener('pointerup', onUp);
      handle.removeEventListener('pointercancel', onUp);
      if (!dragged) updateSettings((s) => (s.ui.sidebarVisible = true));
    };
    handle.addEventListener('pointermove', onMove);
    handle.addEventListener('pointerup', onUp);
    handle.addEventListener('pointercancel', onUp);
  };
  return (
    <button
      type="button"
      class="af-sidebar-handle"
      aria-label={t('sidebar.show')}
      title={t('sidebar.show')}
      onPointerDown={onPointerDown}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          updateSettings((s) => (s.ui.sidebarVisible = true));
        }
      }}
    />
  );
}
