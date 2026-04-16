import { Button, Card, Input, InputNumber, Space, Table, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem } from '@/api/inbounds';
import { createOperationId, getInboundDetail, putawayInbound } from '@/api/inbounds';
import { listSettings } from '@/api/system';
import { ErrorState } from '@/components/QueryState';
import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';
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

  if (detailQuery.error) {
    return (
      <ErrorState
        title="上架数据加载失败"
        error={detailQuery.error}
        onRetry={() => detailQuery.refetch()}
      />
    );
  }

  const detail = detailQuery.data;
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

  const tableContent = (
    <Card>
      {requireLocation ? (
        <Typography.Text type="secondary">当前仓储要求上架必须选择货位。</Typography.Text>
      ) : null}
      <Table<InboundDocItem>
        data-testid="inbound-putaway-table"
        rowKey={(record) => record.sku_id}
        columns={columns}
        dataSource={detail.items}
        pagination={false}
      />
    </Card>
  );

  return <WorkbenchMainTableTemplate toolbarLeft={toolbarLeft} tableContent={tableContent} />;
};

