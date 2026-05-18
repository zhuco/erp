import { Button, Card, Input, InputNumber, Space, Table, Tabs, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem } from '@/api/inbounds';
import { createOperationId, getInboundDetail, putawayInbound } from '@/api/inbounds';
import { listSettings } from '@/api/system';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { ErrorState } from '@/components/QueryState';
import { WorkbenchPageTemplate, WorkbenchToolbar } from '@/components/page-templates';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

export const InboundPutawayPage = () => {
  const tenantId = useTenantId();
  const { doc_id } = useParams<{ doc_id: string }>();
  const navigate = useNavigate();
  const canWrite = useCan('wms.inbounds.write');
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const [quantities, setQuantities] = useState<Record<string, number>>({});
  const [locations, setLocations] = useState<Record<string, string>>({});
  const [toolbarSearchInput, setToolbarSearchInput] = useState('');
  const [toolbarSearchKeyword, setToolbarSearchKeyword] = useState('');
  const { setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible } = useWorkbenchPanels();

  const detailQuery = useQuery({
    queryKey: ['inbound-putaway', tenantId, doc_id],
    queryFn: () => getInboundDetail(tenantId, doc_id ?? ''),
    enabled: tenantReady && Boolean(doc_id) && inventoryFeature.enabled,
    retry: false
  });

  const settingQuery = useQuery({
    queryKey: ['settings', tenantId, 'putaway-require-location'],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: 'wms.putaway_require_location'
      }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const requireLocation = useMemo(() => {
    const setting = settingQuery.data?.[0];
    if (!setting) {
      return false;
    }
    const raw = setting.value_json as unknown;
    if (typeof raw === 'boolean') {
      return raw;
    }
    if (raw && typeof raw === 'object' && 'value' in raw) {
      const value = (raw as { value?: unknown }).value;
      return Boolean(value);
    }
    return false;
  }, [settingQuery.data]);

  const mutation = useMutation({
    mutationFn: (payload: {
      docId: string;
      items: Array<{ sku_id: string; qty: number; to_location_id?: string }>;
    }) =>
      putawayInbound(tenantId, payload.docId, {
        items: payload.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          operation_id: createOperationId(),
          from_area: 'RECEIVING',
          to_location_id: item.to_location_id
        }))
      }),
    onSuccess: () => {
      message.success('上架已提交');
      navigate(`/wms/inbounds/${doc_id}`);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '上架提交失败');
    }
  });

  useEffect(() => {
    if (!detailQuery.data) {
      return;
    }
    const nextQty: Record<string, number> = {};
    detailQuery.data.items.forEach((item) => {
      const remaining = item.received_qty - item.putaway_qty;
      if (remaining > 0) {
        nextQty[item.sku_id] = remaining;
      }
    });
    setQuantities(nextQty);
  }, [detailQuery.data]);

  const detail = detailQuery.data;

  const columns = useMemo<ColumnsType<InboundDocItem>>(
    () => [
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: '已收货', dataIndex: 'received_qty' },
      { title: '已上架', dataIndex: 'putaway_qty' },
      {
        title: '本次上架',
        key: 'putaway_qty',
        render: (_, record) => (
          <InputNumber
            min={0}
            disabled={!canWrite}
            value={quantities[record.sku_id] ?? 0}
            onChange={(value) =>
              setQuantities((prev) => ({ ...prev, [record.sku_id]: Number(value ?? 0) }))
            }
          />
        )
      },
      {
        title: '目标库位',
        key: 'location',
        render: (_, record) => (
          <Input
            placeholder="库位 ID"
            disabled={!canWrite}
            value={locations[record.sku_id] ?? ''}
            onChange={(event) =>
              setLocations((prev) => ({ ...prev, [record.sku_id]: event.target.value }))
            }
          />
        )
      }
    ],
    [canWrite, locations, quantities]
  );

  const filteredItems = useMemo(() => {
    const keyword = toolbarSearchKeyword.trim().toLowerCase();
    if (!keyword) {
      return detail?.items ?? [];
    }
    return (detail?.items ?? []).filter((item) => {
      return (
        item.sku_id.toLowerCase().includes(keyword) ||
        String(item.sku_code ?? '').toLowerCase().includes(keyword) ||
        String(locations[item.sku_id] ?? '').toLowerCase().includes(keyword)
      );
    });
  }, [detail?.items, locations, toolbarSearchKeyword]);

  const leftPanel = useMemo(
    () => (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Card size="small" title="上架说明">
          <Typography.Paragraph style={{ marginBottom: 8 }}>
            本页用于把“已收货未上架”的数量分配到目标库位。
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            若系统开启库位必填，提交前必须填写每条明细的目标库位。
          </Typography.Paragraph>
        </Card>
        <Card size="small" title="当前进度">
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            单据状态：{detail?.doc.status ?? '-'}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            可见明细：{filteredItems.length} 行
          </Typography.Paragraph>
        </Card>
      </Space>
    ),
    [detail?.doc.status, filteredItems.length]
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

  const rightPanel = useMemo(
    () => (
      <Card size="small" title="上架摘要">
        <Typography.Paragraph style={{ marginBottom: 6 }}>
          入库明细：{detail?.items.length ?? 0}
        </Typography.Paragraph>
        <Typography.Paragraph style={{ marginBottom: 6 }}>
          可见明细：{filteredItems.length}
        </Typography.Paragraph>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          库位必填：{requireLocation ? '是' : '否'}
        </Typography.Paragraph>
      </Card>
    ),
    [detail?.items.length, filteredItems.length, requireLocation]
  );

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        items={[
          {
            key: 'putaway-preview',
            label: `上架明细 (${filteredItems.length})`,
            children: (
              <Table<InboundDocItem>
                size="small"
                rowKey={(record) => record.sku_id}
                columns={columns}
                dataSource={filteredItems}
                pagination={false}
              />
            )
          }
        ]}
      />
    ),
    [columns, filteredItems]
  );

  if (detailQuery.error) {
    return (
      <ErrorState
        title="上架数据加载失败"
        error={detailQuery.error}
        onRetry={() => detailQuery.refetch()}
      />
    );
  }

  if (!detail || !doc_id) {
    return null;
  }

  const handleSubmit = () => {
    if (!canWrite) {
      message.error('暂无入库操作权限');
      return;
    }
    const items = detail.items
      .map((item) => ({
        sku_id: item.sku_id,
        qty: quantities[item.sku_id] ?? 0,
        to_location_id: locations[item.sku_id]?.trim() || undefined
      }))
      .filter((item) => item.qty > 0);
    if (!items.length) {
      message.warning('请输入上架数量');
      return;
    }
    if (requireLocation && items.some((item) => !item.to_location_id)) {
      message.warning('上架必须选择货位');
      return;
    }
    mutation.mutate({ docId: doc_id, items });
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} data-testid="inbound-putaway-title" style={{ margin: 0 }}>
        上架确认
      </Typography.Title>
      <Button onClick={() => navigate(`/wms/inbounds/${doc_id}`)}>返回详情</Button>
      <Button type="primary" disabled={!canWrite} onClick={handleSubmit} loading={mutation.isPending}>
        提交上架
      </Button>
    </Space>
  );

  const toolbar = (
    <WorkbenchToolbar
      leftContent={toolbarLeft}
      searchValue={toolbarSearchInput}
      onSearchChange={setToolbarSearchInput}
      onSearch={() => setToolbarSearchKeyword(toolbarSearchInput.trim())}
      searchPlaceholder="按 SKU / 库位筛选明细"
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('可使用顶部搜索快速筛选上架明细')}
    />
  );

  const tableContent = (
    <Card>
      {requireLocation ? (
        <Typography.Text type="secondary">当前仓储要求上架必须选择货位。</Typography.Text>
      ) : null}
      <Table<InboundDocItem>
        data-testid="inbound-putaway-table"
        rowKey={(record) => record.sku_id}
        columns={columns}
        dataSource={filteredItems}
        pagination={false}
      />
    </Card>
  );

  return (
    <WorkbenchPageTemplate
      toolbar={toolbar}
      tableContent={tableContent}
      rightPanel={rightPanel}
      bottomPanel={bottomPanel}
    />
  );
};
