import { Card, Empty, Space, Tabs, Typography, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { OrderReviewWorkbenchTemplate, OrderReviewWorkbenchToolbarTemplate } from '@/components/workbench/templates';

export const ProductDiffPage = () => {
  const [toolbarSearchInput, setToolbarSearchInput] = useState('');
  const [toolbarSearchKeyword, setToolbarSearchKeyword] = useState('');
  const { setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible } = useWorkbenchPanels();

  const leftPanel = useMemo(
    () => (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Card size="small" title="功能说明">
          <Typography.Paragraph style={{ marginBottom: 8 }}>
            商品差异页用于对比多源商品资料，识别属性与状态差异。
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            当前页面仍在建设中，暂不提供可操作数据。
          </Typography.Paragraph>
        </Card>
      </Space>
    ),
    []
  );

  useEffect(() => {
    setLeftPanelCollapsed(false);
    setLeftPanel(leftPanel);
    setLeftPanelVisible(true);
    return () => {
      setLeftPanel(undefined);
      setLeftPanelVisible(false);
    };
  }, [leftPanel, setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible]);

  const toolbar = (
    <OrderReviewWorkbenchToolbarTemplate
      leftContent={
        <Typography.Title level={3} style={{ margin: 0 }}>
          商品差异
        </Typography.Title>
      }
      searchValue={toolbarSearchInput}
      onSearchChange={setToolbarSearchInput}
      onSearch={() => setToolbarSearchKeyword(toolbarSearchInput.trim())}
      searchPlaceholder="预留：按 SKU / 商品名检索差异"
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('该页面正在建设中')}
    />
  );

  const rightPanel = (
    <Card size="small" title="页面状态">
      <Typography.Paragraph style={{ marginBottom: 6 }}>
        当前阶段：功能建设中
      </Typography.Paragraph>
      <Typography.Paragraph style={{ marginBottom: 0 }}>
        最近检索词：{toolbarSearchKeyword || '-'}
      </Typography.Paragraph>
    </Card>
  );

  const bottomPanel = (
    <Tabs
      size="small"
      items={[
        {
          key: 'overview',
          label: '差异概览',
          children: <Empty description="差异数据源尚未接入" />
        }
      ]}
    />
  );

  return (
    <OrderReviewWorkbenchTemplate
      toolbar={toolbar}
      tableContent={
        <Card>
          <Empty description="功能建设中" />
        </Card>
      }
      rightPanel={rightPanel}
      bottomPanel={bottomPanel}
    />
  );
};
