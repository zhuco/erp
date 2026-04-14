import { Button, Card, Form, Input, InputNumber, Select, Space, Table, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem } from '@/api/inbounds';
import { createOperationId, getInboundDetail, inspectInbound } from '@/api/inbounds';
import { ErrorState } from '@/components/QueryState';
import { UiModule02WorkbenchTemplate } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type InspectLine = {
  line_id: string;
  sku_id: string;
  qty: number;
  result: string;
  note?: string;
};

const resultOptions = [
  { label: '合格', value: 'PASSED' },
  { label: '不合格', value: 'FAILED' },
  { label: '待复检', value: 'PENDING' }
];

const createLineId = () => `line-${Date.now()}-${Math.random().toString(16).slice(2)}`;

export const InboundInspectPage = () => {
  const tenantId = useTenantId();
  const { doc_id } = useParams<{ doc_id: string }>();
  const navigate = useNavigate();
  const canWrite = useCan('wms.inbounds.write');
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const [inspectForm] = Form.useForm<InspectLine>();
  const [inspectLines, setInspectLines] = useState<InspectLine[]>([]);

  const detailQuery = useQuery({
    queryKey: ['inbound-inspect', tenantId, doc_id],
    queryFn: () => getInboundDetail(tenantId, doc_id ?? ''),
    enabled: tenantReady && Boolean(doc_id) && inventoryFeature.enabled,
    retry: false
  });

  const inspectMutation = useMutation({
    mutationFn: (payload: { docId: string; items: InspectLine[] }) =>
      inspectInbound(tenantId, payload.docId, {
        items: payload.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          result: item.result,
          note: item.note?.trim() || undefined,
          operation_id: createOperationId()
        }))
      }),
    onSuccess: () => {
      message.success('质检已提交');
      setInspectLines([]);
      navigate(`/wms/inbounds/${doc_id}`);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '质检提交失败');
    }
  });

  const detail = detailQuery.data;
  const skuOptions = useMemo(
    () =>
      (detail?.items ?? []).map((item) => ({
        label: item.sku_code ? `${item.sku_code} (${item.sku_id})` : item.sku_id,
        value: item.sku_id
      })),
    [detail?.items]
  );

  const inboundItemColumns = useMemo<ColumnsType<InboundDocItem>>(
    () => [
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: 'SKU 编码', dataIndex: 'sku_code' },
      { title: '条码', dataIndex: 'barcode' },
      { title: '计划数量', dataIndex: 'planned_qty' },
      { title: '已收货', dataIndex: 'received_qty' },
      { title: '已上架', dataIndex: 'putaway_qty' }
    ],
    []
  );

  const inspectLineColumns = useMemo<ColumnsType<InspectLine>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: '数量', dataIndex: 'qty' },
      {
        title: '结果',
        dataIndex: 'result',
        render: (value) => resultOptions.find((item) => item.value === value)?.label ?? value
      },
      { title: '备注', dataIndex: 'note', render: (value) => value ?? '-' },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button
            type="link"
            disabled={!canWrite}
            onClick={() =>
              setInspectLines((prev) => prev.filter((line) => line.line_id !== record.line_id))
            }
          >
            移除
          </Button>
        )
      }
    ],
    [canWrite]
  );

  if (detailQuery.error) {
    return (
      <ErrorState
        title="质检数据加载失败"
        error={detailQuery.error}
        onRetry={() => detailQuery.refetch()}
      />
    );
  }

  if (!detail || !doc_id) {
    return null;
  }


  const handleAddInspectLine = async () => {
    const values = await inspectForm.validateFields();
    if (!values.sku_id) {
      return;
    }
    if (!detail.items.find((item) => item.sku_id === values.sku_id)) {
      message.warning('SKU 不在入库单范围内');
      return;
    }
    setInspectLines((prev) => [
      ...prev,
      {
        line_id: createLineId(),
        sku_id: values.sku_id,
        qty: Number(values.qty ?? 0),
        result: values.result,
        note: values.note
      }
    ]);
    inspectForm.resetFields();
  };

  const handleSubmitInspect = () => {
    if (!canWrite) {
      message.error('暂无入库操作权限');
      return;
    }
    if (!inspectLines.length) {
      message.warning('请先录入质检明细');
      return;
    }
    inspectMutation.mutate({ docId: doc_id, items: inspectLines });
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} data-testid="inbound-inspect-title" style={{ margin: 0 }}>
        入库质检
      </Typography.Title>
      <Button onClick={() => navigate(`/wms/inbounds/${doc_id}`)}>返回详情</Button>
      <Button
        type="primary"
        disabled={!canWrite}
        onClick={handleSubmitInspect}
        loading={inspectMutation.isPending}
      >
        提交质检
      </Button>
    </Space>
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card title="入库明细">
        <Table<InboundDocItem>
          data-testid="inbound-inspect-items"
          rowKey={(record) => record.sku_id}
          columns={inboundItemColumns}
          dataSource={detail.items}
          pagination={false}
        />
      </Card>
      <Card title="质检登记">
        <Form form={inspectForm} layout="inline" style={{ marginBottom: 12 }}>
          <Form.Item name="sku_id" rules={[{ required: true, message: '请选择 SKU' }]}>
            <Select
              allowClear
              disabled={!canWrite}
              placeholder="选择 SKU"
              options={skuOptions}
              style={{ width: 240 }}
            />
          </Form.Item>
          <Form.Item name="qty" rules={[{ required: true, message: '请输入数量' }]}>
            <InputNumber min={1} disabled={!canWrite} placeholder="数量" />
          </Form.Item>
          <Form.Item name="result" rules={[{ required: true, message: '请选择结果' }]}>
            <Select
              disabled={!canWrite}
              options={resultOptions}
              placeholder="结果"
              style={{ width: 160 }}
            />
          </Form.Item>
          <Form.Item name="note">
            <Input disabled={!canWrite} placeholder="备注" />
          </Form.Item>
          <Form.Item>
            <Button disabled={!canWrite} onClick={handleAddInspectLine}>
              添加
            </Button>
          </Form.Item>
        </Form>
        <Table<InspectLine>
          rowKey={(record) => record.line_id}
          columns={inspectLineColumns}
          dataSource={inspectLines}
          pagination={false}
          locale={{ emptyText: '暂无质检明细' }}
        />
      </Card>
    </Space>
  );

  return <UiModule02WorkbenchTemplate toolbarLeft={toolbarLeft} tableContent={tableContent} />;
};
