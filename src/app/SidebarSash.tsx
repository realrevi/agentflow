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
