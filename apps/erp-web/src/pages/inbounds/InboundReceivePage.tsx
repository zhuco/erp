import { Alert, Button, Card, Form, Input, InputNumber, Select, Space, Table, Tabs, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem } from '@/api/inbounds';
import { createOperationId, getInboundDetail, receiveInbound } from '@/api/inbounds';
import { createInboundRejects } from '@/api/inboundRejects';
import { searchSkus, type SkuSearchItem } from '@/api/skus';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { ErrorState } from '@/components/QueryState';
import { OrderReviewWorkbenchTemplate, OrderReviewWorkbenchToolbarTemplate } from '@/components/workbench/templates';
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
  const [toolbarSearchInput, setToolbarSearchInput] = useState('');
  const [toolbarSearchKeyword, setToolbarSearchKeyword] = useState('');
  const { setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible } = useWorkbenchPanels();

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

  const detail = detailQuery.data;

  const skuLabelMap = useMemo(() => {
    const map = new Map<string, string>();
    (detail?.items ?? []).forEach((item) => {
      const name = item.sku_code ?? item.sku_id;
      const barcode = item.barcode ? ` (${item.barcode})` : '';
      map.set(item.sku_id, `${name}${barcode}`);
    });
    skuOptions.forEach((sku) => {
      map.set(sku.sku_id, formatSkuLabel(sku));
    });
    return map;
  }, [detail?.items, skuOptions]);

  const skuSelectOptions = useMemo(
    () =>
      skuOptions.map((sku) => ({
        label: formatSkuLabel(sku),
        value: sku.sku_id
      })),
    [skuOptions]
  );

  const itemBySku = useMemo(
    () => new Map((detail?.items ?? []).map((item) => [item.sku_id, item])),
    [detail?.items]
  );

  const pendingReceiveBySku = useMemo(() => {
    const map = new Map<string, number>();
    receiveLines.forEach((line) => {
      map.set(line.sku_id, (map.get(line.sku_id) ?? 0) + line.qty);
    });
    return map;
  }, [receiveLines]);

  const pendingRejectBySku = useMemo(() => {
    const map = new Map<string, number>();
    rejectLines.forEach((line) => {
      map.set(line.sku_id, (map.get(line.sku_id) ?? 0) + line.qty);
    });
    return map;
  }, [rejectLines]);

  const filteredInboundItems = useMemo(() => {
    const keyword = toolbarSearchKeyword.trim().toLowerCase();
    if (!keyword) {
      return detail?.items ?? [];
    }
    return (detail?.items ?? []).filter((item) => {
      const label = (skuLabelMap.get(item.sku_id) ?? item.sku_id).toLowerCase();
      return (
        label.includes(keyword) ||
        item.sku_id.toLowerCase().includes(keyword) ||
        String(item.sku_code ?? '').toLowerCase().includes(keyword) ||
        String(item.barcode ?? '').toLowerCase().includes(keyword)
      );
    });
  }, [detail?.items, skuLabelMap, toolbarSearchKeyword]);

  const leftPanel = useMemo(
    () => (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Card size="small" title="收货说明">
          <Typography.Paragraph style={{ marginBottom: 8 }}>
            1. 选择 SKU，并填写收货数量或拒收数量。</Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            2. 提交后系统会校验总量，超出待处理数量时请走补录入库流程。</Typography.Paragraph>
        </Card>
        <Card size="small" title="当前进度">
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            单据状态：{detail?.doc.status ?? '-'}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            待提交收货：{receiveLines.length} 行</Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            待提交拒收：{rejectLines.length} 行</Typography.Paragraph>
        </Card>
      </Space>
    ),
    [detail?.doc.status, receiveLines.length, rejectLines.length]
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
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Card size="small" title="收货摘要">
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            单据：{detail?.doc.doc_no ?? '-'}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            明细行数：{detail?.items.length ?? 0}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            可见行数：{filteredInboundItems.length}
          </Typography.Paragraph>
        </Card>
      </Space>
    ),
    [detail?.doc.doc_no, detail?.items.length, filteredInboundItems.length]
  );
  const getAvailableQty = (skuId: string) => {
    const item = itemBySku.get(skuId);
    if (!item) {
      return 0;
    }
    const baseRemaining =
      item.planned_qty > 0 ? Math.max(item.planned_qty - item.received_qty, 0) : Number.MAX_SAFE_INTEGER;
    const pendingUsed = (pendingReceiveBySku.get(skuId) ?? 0) + (pendingRejectBySku.get(skuId) ?? 0);
    return Math.max(baseRemaining - pendingUsed, 0);
  };

  const inboundItemColumns: ColumnsType<InboundDocItem> = [
    { title: 'SKU', dataIndex: 'sku_id', render: (value) => skuLabelMap.get(value) ?? value },
    { title: '计划数量', dataIndex: 'planned_qty' },
    { title: '已收货', dataIndex: 'received_qty' },
    { title: '已上架', dataIndex: 'putaway_qty' },
    {
      title: '待处理数量',
      key: 'remaining_qty',
      render: (_, record) =>
        record.planned_qty > 0 ? Math.max(record.planned_qty - record.received_qty, 0) : '-'
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
          删除
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
    { title: '拒收原因', dataIndex: 'reason', render: (value) => value ?? '-' },
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
          删除
        </Button>
      )
    }
  ];

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        items={[
          {
            key: 'receive',
            label: `待提交收货 (${receiveLines.length})`,
            children: (
              <Table<ReceiveLine>
                size="small"
                rowKey={(record) => record.line_id}
                columns={receiveLineColumns}
                dataSource={receiveLines}
                pagination={false}
                locale={{ emptyText: '暂无待提交收货行' }}
              />
            )
          },
          {
            key: 'reject',
            label: `待提交拒收 (${rejectLines.length})`,
            children: (
              <Table<RejectLine>
                size="small"
                rowKey={(record) => record.line_id}
                columns={rejectLineColumns}
                dataSource={rejectLines}
                pagination={false}
                locale={{ emptyText: '暂无待提交拒收行' }}
              />
            )
          }
        ]}
      />
    ),
    [receiveLineColumns, receiveLines, rejectLineColumns, rejectLines]
  );

  if (detailQuery.error) {
    return (
      <ErrorState
        title="收货数据加载失败"
        error={detailQuery.error}
        onRetry={() => detailQuery.refetch()}
      />
    );
  }

  if (!detail || !doc_id) {
    return null;
  }

  const handleAddReceiveLine = async () => {
    const values = await receiveForm.validateFields();
    if (!values.sku_id) {
      return;
    }
    if (!detail.items.find((item) => item.sku_id === values.sku_id)) {
      message.warning('SKU 不在当前入库单明细中');
      return;
    }
    const qty = Number(values.qty ?? 0);
    if (qty <= 0) {
      message.warning('收货数量必须大于 0');
      return;
    }
    const available = getAvailableQty(values.sku_id);
    if (qty > available) {
      message.warning(`收货数量不能超过当前可处理数量：${available}`);
      return;
    }
    setReceiveLines((prev) => [
      ...prev,
      {
        line_id: createLineId(),
        sku_id: values.sku_id,
        qty
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
      message.warning('SKU 不在当前入库单明细中');
      return;
    }
    const qty = Number(values.qty ?? 0);
    if (qty <= 0) {
      message.warning('拒收数量必须大于 0');
      return;
    }
    const available = getAvailableQty(values.sku_id);
    if (qty > available) {
      message.warning(`拒收数量不能超过当前可处理数量：${available}`);
      return;
    }
    setRejectLines((prev) => [
      ...prev,
      {
        line_id: createLineId(),
        sku_id: values.sku_id,
        qty,
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
    if (detail.doc.status === 'PUTAWAY_DONE' || detail.doc.status === 'CANCELLED') {
      message.warning('当前入库单状态不允许继续收货，请走补录流程');
      return;
    }
    if (!receiveLines.length) {
      message.warning('请先添加收货明细');
      return;
    }
    receiveMutation.mutate({ docId: doc_id, items: receiveLines });
  };

  const handleSubmitRejects = () => {
    if (!canWrite) {
      message.error('暂无入库操作权限');
      return;
    }
    if (detail.doc.status === 'PUTAWAY_DONE' || detail.doc.status === 'CANCELLED') {
      message.warning('当前入库单状态不允许继续拒收，请走补录流程');
      return;
    }
    if (!rejectLines.length) {
      message.warning('请先添加拒收明细');
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
      <Button onClick={() => navigate('/wms/inbounds')}>
        返回列表
      </Button>
    </Space>
  );

  const toolbar = (
    <OrderReviewWorkbenchToolbarTemplate
      leftContent={toolbarLeft}
      searchValue={toolbarSearchInput}
      onSearchChange={setToolbarSearchInput}
      onSearch={() => setToolbarSearchKeyword(toolbarSearchInput.trim())}
      searchPlaceholder="按 SKU / 条码筛选入库明细"
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('可通过上方搜索框快速筛选入库明细')}
    />
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card title="入库明细">
        <Table<InboundDocItem>
          data-testid="inbound-receive-items"
          rowKey={(record) => record.sku_id}
          columns={inboundItemColumns}
          dataSource={filteredInboundItems}
          pagination={false}
        />
      </Card>
      <Card title="收货登记">
        <Alert
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
          message="系统会校验“收货 + 拒收”总量不超过待处理数量，超量请走补录入库。"
        />
        <Form form={receiveForm} layout="inline" style={{ marginBottom: 12 }}>
          <Form.Item name="sku_id" rules={[{ required: true, message: '请选择 SKU' }]}>
            <Select
              showSearch
              allowClear
              placeholder="搜索/选择 SKU"
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
        <Alert
          type="warning"
          showIcon
          style={{ marginBottom: 12 }}
          message="拒收记录会占用本单待处理数量，后续补货请通过“去补录入库”新建补录单。"
        />
        <Form form={rejectForm} layout="inline" style={{ marginBottom: 12 }}>
          <Form.Item name="sku_id" rules={[{ required: true, message: '请选择 SKU' }]}>
            <Select
              showSearch
              allowClear
              placeholder="搜索/选择 SKU"
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

  return (
    <OrderReviewWorkbenchTemplate
      toolbar={toolbar}
      tableContent={tableContent}
      rightPanel={rightPanel}
      bottomPanel={bottomPanel}
    />
  );
};

