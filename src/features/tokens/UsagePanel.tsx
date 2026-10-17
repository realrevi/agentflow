// Lazy-loaded Usage dock panel (see layout/renderers.tsx). Default export for `lazy()`.
import { onMount } from 'solid-js';
import { UsageDashboard, type UsageDashboardProps } from './UsageDashboard';
import { reloadUsage } from './usageStore';

export default function UsagePanel(props: Omit<UsageDashboardProps, 'compact' | 'onOpenPanel'>) {
  onMount(() => void reloadUsage());
  return (
    <div class="af-usage-panel">
      <UsageDashboard panelTitle={props.panelTitle} onOpenSettings={props.onOpenSettings} />
    </div>
  );
}
