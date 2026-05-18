import { SettingOutlined } from '@ant-design/icons';
import type { CustomFloatingFilterProps } from 'ag-grid-react';

import './workbenchColumnSettings.css';

type WorkbenchColumnSettingsFloatingControlProps = CustomFloatingFilterProps & {
  ariaLabel?: string;
  buttonLabel?: string;
  buttonClassName?: string;
  controlClassName?: string;
  onOpenColumnSettings?: () => void;
  testId?: string;
};

export const WorkbenchColumnSettingsFloatingControl = ({
  ariaLabel = '列设置',
  buttonLabel = '',
  buttonClassName,
  controlClassName,
  onOpenColumnSettings,
  testId = 'workbench-column-settings-trigger',
}: WorkbenchColumnSettingsFloatingControlProps) => (
  <div
    className={['workbench-column-settings-floating-control', controlClassName].filter(Boolean).join(' ')}
    onClick={(event) => event.stopPropagation()}
    onMouseDown={(event) => event.stopPropagation()}
  >
    <button
      type="button"
      className={['workbench-column-settings-floating-button', buttonClassName].filter(Boolean).join(' ')}
      data-testid={testId}
      aria-label={ariaLabel}
      title={ariaLabel}
      disabled={typeof onOpenColumnSettings !== 'function'}
      onClick={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onOpenColumnSettings?.();
      }}
    >
      <SettingOutlined />
      {buttonLabel ? <span>{buttonLabel}</span> : null}
    </button>
  </div>
);
