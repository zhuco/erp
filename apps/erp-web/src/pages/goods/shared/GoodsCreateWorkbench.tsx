import './goodsCreateWorkbench.css';

import { Button, Space, Tabs, Tag, Typography } from 'antd';
import type { CollapseProps, TabsProps } from 'antd';
import type { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router-dom';

import { WorkbenchPageTemplate, WorkbenchToolbar } from '@/components/page-templates';
import { UiModule05DetailPanel } from '@/components/ui-modules';

import { buildGoodsCompactWorkbenchPath } from './goodsRoutes';

type GoodsCreateWorkbenchMetric = {
  key: string;
  label: string;
  value: ReactNode;
  tone?: 'default' | 'blue' | 'green' | 'gold';
};

type GoodsCreateWorkbenchProps = {
  pageTestId: string;
  title: string;
  description: string;
  statusTag?: ReactNode;
  backLabel?: string;
  onBack: () => void;
  saveLabel: string;
  formId: string;
  submitting?: boolean;
  metrics?: GoodsCreateWorkbenchMetric[];
  rightPanels?: CollapseProps['items'];
  bottomItems?: TabsProps['items'];
  children: ReactNode;
};

const toneClassNameMap: Record<NonNullable<GoodsCreateWorkbenchMetric['tone']>, string> = {
  default: 'goods-create-workbench-metric--default',
  blue: 'goods-create-workbench-metric--blue',
  green: 'goods-create-workbench-metric--green',
  gold: 'goods-create-workbench-metric--gold',
};

const buildToolbarStatusTag = (statusTag?: ReactNode) =>
  statusTag ?? <Tag color="blue">直连接口</Tag>;

export const GoodsCreateWorkbench = ({
  pageTestId,
  title,
  description,
  statusTag,
  backLabel = '返回查询',
  onBack,
  saveLabel,
  formId,
  submitting = false,
  metrics = [],
  rightPanels = [],
  bottomItems = [],
  children,
}: GoodsCreateWorkbenchProps) => {
  const location = useLocation();
  const firstBottomKey = bottomItems[0]?.key;
  const compactWorkbenchPath = buildGoodsCompactWorkbenchPath(
    location.pathname,
    location.search,
  );
  const currentPath = `${location.pathname}${location.search}`;

  if (compactWorkbenchPath !== currentPath) {
    return <Navigate replace to={compactWorkbenchPath} />;
  }

  return (
    <WorkbenchPageTemplate
      pageTestId={pageTestId}
      toolbar={(
        <WorkbenchToolbar
          searchEnabled={false}
          canExport={false}
          leftContent={(
            <div className="goods-create-workbench-toolbar-meta">
              <Space size={8} wrap>
                <Typography.Title
                  level={4}
                  className="goods-create-workbench-toolbar-title"
                >
                  {title}
                </Typography.Title>
                {buildToolbarStatusTag(statusTag)}
              </Space>
              <Typography.Text
                type="secondary"
                className="goods-create-workbench-toolbar-description"
              >
                {description}
              </Typography.Text>
            </div>
          )}
          rightContent={(
            <Space>
              <Button onClick={onBack}>{backLabel}</Button>
              <Button
                type="primary"
                htmlType="submit"
                form={formId}
                loading={submitting}
              >
                {saveLabel}
              </Button>
            </Space>
          )}
        />
      )}
      rightPanel={
        rightPanels.length > 0 ? <UiModule05DetailPanel panels={rightPanels} /> : undefined
      }
      rightPanelVisible={rightPanels.length > 0}
      bottomPanel={
        bottomItems.length > 0 ? (
          <Tabs
            defaultActiveKey={firstBottomKey == null ? undefined : String(firstBottomKey)}
            items={bottomItems}
          />
        ) : undefined
      }
      bottomPanelVisible={bottomItems.length > 0}
      tableContent={(
        <div className="goods-create-workbench">
          {metrics.length > 0 ? (
            <div className="goods-create-workbench-metrics">
              {metrics.map((metric) => {
                const tone = metric.tone ?? 'default';
                return (
                  <div
                    key={metric.key}
                    className={[
                      'goods-create-workbench-metric',
                      toneClassNameMap[tone],
                    ].join(' ')}
                  >
                    <div className="goods-create-workbench-metric-label">
                      {metric.label}
                    </div>
                    <div className="goods-create-workbench-metric-value">
                      {metric.value}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : null}
          <div className="goods-create-workbench-form-surface">{children}</div>
        </div>
      )}
    />
  );
};

export type { GoodsCreateWorkbenchMetric };
