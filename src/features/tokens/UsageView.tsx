// The sidebar's Usage view: the compact dashboard and a button for the full panel.
import { UsageDashboard, type UsageDashboardProps } from './UsageDashboard';

export function UsageView(props: Omit<UsageDashboardProps, 'compact'>) {
  return (
    <div class="af-view">
      <UsageDashboard
        compact
        panelTitle={props.panelTitle}
        onOpenSettings={props.onOpenSettings}
        {...(props.onOpenPanel ? { onOpenPanel: props.onOpenPanel } : {})}
      />
    </div>
  );
}
