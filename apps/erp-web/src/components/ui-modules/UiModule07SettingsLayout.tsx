import { Card, Space, Typography } from 'antd';
import type { ReactNode } from 'react';

import './UiModule07SettingsLayout.css';

type UiModule07SettingsLayoutProps = {
  title?: ReactNode;
  children: ReactNode;
};

type UiModule07SettingsSectionProps = {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
};

export const UiModule07SettingsLayout = ({ title, children }: UiModule07SettingsLayoutProps) => (
  <Space orientation="vertical" size={16} style={{ width: '100%' }} className="ui-module-07-settings-layout">
    {title ? (
      <Typography.Title level={3} className="ui-module-07-layout-title">
        {title}
      </Typography.Title>
    ) : null}
    {children}
  </Space>
);

export const UiModule07SettingsSection = ({
  title,
  description,
  actions,
  children,
  className
}: UiModule07SettingsSectionProps) => (
  <Card className={className}>
    <Space orientation="vertical" size={16} style={{ width: '100%' }} className="ui-module-07-settings-section">
      <div className="ui-module-07-section-header">
        <div className="ui-module-07-section-title">
          <Typography.Title level={4}>{title}</Typography.Title>
          {description ? (
            <Typography.Text type="secondary" className="ui-module-07-section-description">
              {description}
            </Typography.Text>
          ) : null}
        </div>
        {actions ? <div className="ui-module-07-section-actions">{actions}</div> : null}
      </div>
      {children}
    </Space>
  </Card>
);
