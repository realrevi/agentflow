// The conversation's permission mode as a compact segmented control with icons; each
// segment's tooltip says what the mode allows.
import type { PermissionMode } from '../../../shared/types';
import { SegmentedControl } from '../../../shared/ui/Segmented';
import { t } from '../../i18n';

export function ModePill(props: {
  mode: PermissionMode;
  onChange: (mode: PermissionMode) => void;
}) {
  return (
    <SegmentedControl
      label={t('agent.mode')}
      value={props.mode}
      onChange={props.onChange}
      options={[
        {
          value: 'ask',
          icon: 'shield-check',
          label: t('agent.mode.ask'),
          title: t('agent.mode.askHint'),
        },
        {
          value: 'autoEdit',
          icon: 'file-pen',
          label: t('agent.mode.autoEdit'),
          title: t('agent.mode.autoEditHint'),
        },
        {
          value: 'readOnly',
          icon: 'eye',
          label: t('agent.mode.readOnly'),
          title: t('agent.mode.readOnlyHint'),
        },
      ]}
    />
  );
}
