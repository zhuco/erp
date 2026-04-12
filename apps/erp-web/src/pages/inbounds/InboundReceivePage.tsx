import { Button, Card, Form, Input, InputNumber, Select, Space, Table, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem } from '@/api/inbounds';
import { createOperationId, getInboundDetail, receiveInbound } from '@/api/inbounds';
import { createInboundRejects } from '@/api/inboundRejects';
import { searchSkus, type SkuSearchItem } from '@/api/skus';
import { ErrorState } from '@/components/QueryState';
import { UiModule02WorkbenchTemplate } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type ReceiveLine = {
  line_id: string;
  sku_id: string;
  qty: number;
};

type RejectLine = {
  line_id: string;
  sku_id: string;
  qty: number;
  reason?: string;
};

const formatSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

const createLineId = () => `line-${Date.now()}-${Math.random().toString(16).slice(2)}`;

export const InboundReceivePage = () => {
  const tenantId = useTenantId();
  const { doc_id } = useParams<{ doc_id: string }>();
  const navigate = useNavigate();
  const canWrite = useCan('wms.inbounds.write');
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const [receiveForm] = Form.useForm<{ sku_id?: string; qty?: number }>();
  const [rejectForm] = Form.useForm<{ sku_id?: string; qty?: number; reason?: string }>();
  const [receiveLines, setReceiveLines] = useState<ReceiveLine[]>([]);
  const [rejectLines, setRejectLines] = useState<RejectLine[]>([]);
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);

  const detailQuery = useQuery({
    queryKey: ['inbound-receive', tenantId, doc_id],
    queryFn: () => getInboundDetail(tenantId, doc_id ?? ''),
    enabled: tenantReady && Boolean(doc_id) && inventoryFeature.enabled,
    retry: false
  });

  const receiveMutation = useMutation({
    mutationFn: (payload: { docId: string; items: ReceiveLine[] }) =>
      receiveInbound(tenantId, payload.docId, {
        items: payload.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          operation_id: createOperationId()
        }))
      }),
    onSuccess: () => {
      message.success('收货已提交');
      setReceiveLines([]);
      navigate(`/wms/inbounds/${doc_id}`);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '收货提交失败');
    }
  });

  const rejectMutation = useMutation({
    mutationFn: (payload: { docId: string; items: RejectLine[] }) =>
      createInboundRejects(tenantId, payload.docId, {
        items: payload.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          reason: item.reason
        }))
      }),
    onSuccess: () => {
      message.success('拒收已记录');
      setRejectLines([]);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '拒收提交失败');
    }
  });

  const loadSkuOptions = async (keyword?: string) => {
    setSkuLoading(true);
    try {
      const response = await searchSkus(tenantId, keyword ?? '');
      setSkuOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'SKU 搜索失败');
    } finally {
      setSkuLoading(false);
    }
  };

  const skuLabelMap = useMemo(() => {
    const map = new Map<string, string>();
    detailQuery.data?.items.forEach((item) => {
      const name = item.sku_code ?? item.sku_id;
      const barcode = item.barcode ? ` (${item.barcode})` : '';
      map.set(item.sku_id, `${name}${barcode}`);
    });
    skuOptions.forEach((sku) => {
      map.set(sku.sku_id, formatSkuLabel(sku));
    });
    return map;
  }, [detailQuery.data?.items, skuOptions]);

  const skuSelectOptions = useMemo(
    () =>
      skuOptions.map((sku) => ({
        label: formatSkuLabel(sku),
        value: sku.sku_id
      })),
    [skuOptions]
  );

  const inboundItemColumns: ColumnsType<InboundDocItem> = [
    { title: 'SKU', dataIndex: 'sku_id', render: (value) => skuLabelMap.get(value) ?? value },
    { title: '计划数量', dataIndex: 'planned_qty' },
    { title: '已收货', dataIndex: 'received_qty' },
    { title: '已上架', dataIndex: 'putaway_qty' },
    {
      title: '待收数量',
      key: 'remaining_qty',
      render: (_, record) => Math.max(record.planned_qty - record.received_qty, 0)
    }
  ];

  const receiveLineColumns: ColumnsType<ReceiveLine> = [
    {
      title: 'SKU',
      dataIndex: 'sku_id',
      render: (value) => skuLabelMap.get(value) ?? value
    },
    { title: '数量', dataIndex: 'qty' },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Button
          type="link"
          onClick={() =>
            setReceiveLines((prev) => prev.filter((line) => line.line_id !== record.line_id))
          }
        >
          移除
        </Button>
      )
    }
  ];

  const rejectLineColumns: ColumnsType<RejectLine> = [
    {
      title: 'SKU',
      dataIndex: 'sku_id',
      render: (value) => skuLabelMap.get(value) ?? value
    },
    { title: '数量', dataIndex: 'qty' },
    { title: '原因', dataIndex: 'reason', render: (value) => value ?? '-' },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Button
          type="link"
          onClick={() =>
            setRejectLines((prev) => prev.filter((line) => line.line_id !== record.line_id))
          }
        >
          移除
        </Button>
      )
    }
  ];

  if (detailQuery.error) {
    return (
      <ErrorState
        title="收货数据加载失败"
        error={detailQuery.error}
        onRetry={() => detailQuery.refetch()}
      />
    );
  }

  const detail = detailQuery.data;
  if (!detail || !doc_id) {
    return null;
  }

  const handleAddReceiveLine = async () => {
    const values = await receiveForm.validateFields();
    if (!values.sku_id) {
      return;
    }
    if (!detail.items.find((item) => item.sku_id === values.sku_id)) {
      message.warning('SKU 不在入库单范围内');
      return;
    }
    setReceiveLines((prev) => [
      ...prev,
      {
        line_id: createLineId(),
        sku_id: values.sku_id,
        qty: Number(values.qty ?? 0)
      }
    ]);
    receiveForm.resetFields();
  };

  const handleAddRejectLine = async () => {
    const values = await rejectForm.validateFields();
    if (!values.sku_id) {
      return;
    }
    if (!detail.items.find((item) => item.sku_id === values.sku_id)) {
      message.warning('SKU 不在入库单范围内');
      return;
    }
    setRejectLines((prev) => [
      ...prev,
      {
        line_id: createLineId(),
        sku_id: values.sku_id,
        qty: Number(values.qty ?? 0),
        reason: values.reason?.trim() || undefined
      }
    ]);
    rejectForm.resetFields();
  };

  const handleSubmitReceive = () => {
    if (!canWrite) {
      message.error('暂无入库操作权限');
      return;
    }
    if (!receiveLines.length) {
      message.warning('请先录入收货明细');
      return;
    }
    receiveMutation.mutate({ docId: doc_id, items: receiveLines });
  };

  const handleSubmitRejects = () => {
    if (!canWrite) {
      message.error('暂无入库操作权限');
      return;
    }
    if (!rejectLines.length) {
      message.warning('请先录入拒收明细');
      return;
    }
    rejectMutation.mutate({ docId: doc_id, items: rejectLines });
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} data-testid="inbound-receive-title" style={{ margin: 0 }}>
        收货登记
      </Typography.Title>
      <Button onClick={() => navigate(`/wms/inbounds/${doc_id}`)}>返回详情</Button>
      <Button type="primary" onClick={handleSubmitReceive} loading={receiveMutation.isPending}>
        提交收货
      </Button>
    </Space>
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card title="入库明细">
        <Table<InboundDocItem>
          data-testid="inbound-receive-items"
          rowKey={(record) => record.sku_id}
          columns={inboundItemColumns}
          dataSource={detail.items}
          pagination={false}
        />
      </Card>
      <Card title="收货登记（支持分批）">
        <Form form={receiveForm} layout="inline" style={{ marginBottom: 12 }}>
          <Form.Item name="sku_id" rules={[{ required: true, message: '请选择 SKU' }]}>
            <Select
              showSearch
              allowClear
              placeholder="扫码/输入 SKU"
              options={skuSelectOptions}
              filterOption={false}
              onSearch={(value) => void loadSkuOptions(value)}
              onFocus={() => void loadSkuOptions()}
              loading={skuLoading}
              style={{ width: 260 }}
            />
          </Form.Item>
          <Form.Item name="qty" rules={[{ required: true, message: '请输入数量' }]}>
            <InputNumber min={1} placeholder="数量" />
          </Form.Item>
          <Form.Item>
            <Button onClick={handleAddReceiveLine}>添加</Button>
          </Form.Item>
        </Form>
        <Table<ReceiveLine>
          rowKey={(record) => record.line_id}
          columns={receiveLineColumns}
          dataSource={receiveLines}
          pagination={false}
          locale={{ emptyText: '暂无收货明细' }}
        />
      </Card>
      <Card
        title="拒收登记"
        extra={
          <Button type="primary" onClick={handleSubmitRejects} loading={rejectMutation.isPending}>
            提交拒收
          </Button>
        }
      >
        <Form form={rejectForm} layout="inline" style={{ marginBottom: 12 }}>
          <Form.Item name="sku_id" rules={[{ required: true, message: '请选择 SKU' }]}>
            <Select
              showSearch
              allowClear
              placeholder="扫码/输入 SKU"
              options={skuSelectOptions}
              filterOption={false}
              onSearch={(value) => void loadSkuOptions(value)}
              onFocus={() => void loadSkuOptions()}
              loading={skuLoading}
              style={{ width: 260 }}
            />
          </Form.Item>
          <Form.Item name="qty" rules={[{ required: true, message: '请输入数量' }]}>
            <InputNumber min={1} placeholder="数量" />
          </Form.Item>
          <Form.Item name="reason">
            <Input placeholder="拒收原因" />
          </Form.Item>
          <Form.Item>
            <Button onClick={handleAddRejectLine}>添加</Button>
          </Form.Item>
        </Form>
        <Table<RejectLine>
          rowKey={(record) => record.line_id}
          columns={rejectLineColumns}
          dataSource={rejectLines}
          pagination={false}
          locale={{ emptyText: '暂无拒收明细' }}
        />
      </Card>
    </Space>
  );

  return <UiModule02WorkbenchTemplate toolbarLeft={toolbarLeft} tableContent={tableContent} />;
};
