import { Button, Card, Form, Input, InputNumber, Select, Space, Table, Tabs, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem } from '@/api/inbounds';
import { createOperationId, getInboundDetail, inspectInbound } from '@/api/inbounds';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { ErrorState } from '@/components/QueryState';
import { WorkbenchPageTemplate, WorkbenchToolbar } from '@/components/page-templates';
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
  { label: '鍚堟牸', value: 'PASSED' },
  { label: '不合格', value: 'FAILED' },
  { label: '寰呭妫€', value: 'PENDING' }
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
  const [toolbarSearchInput, setToolbarSearchInput] = useState('');
  const [toolbarSearchKeyword, setToolbarSearchKeyword] = useState('');
  const { setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible } = useWorkbenchPanels();

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
      { title: 'SKU 缂栫爜', dataIndex: 'sku_code' },
      { title: '鏉＄爜', dataIndex: 'barcode' },
      { title: '璁″垝鏁伴噺', dataIndex: 'planned_qty' },
      { title: '已收货', dataIndex: 'received_qty' },
      { title: '已上架', dataIndex: 'putaway_qty' }
    ],
    []
  );

  const inspectLineColumns = useMemo<ColumnsType<InspectLine>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: '鏁伴噺', dataIndex: 'qty' },
      {
        title: '缁撴灉',
        dataIndex: 'result',
        render: (value) => resultOptions.find((item) => item.value === value)?.label ?? value
      },
      { title: '澶囨敞', dataIndex: 'note', render: (value) => value ?? '-' },
      {
        title: '鎿嶄綔',
        key: 'actions',
        render: (_, record) => (
          <Button
            type="link"
            disabled={!canWrite}
            onClick={() =>
              setInspectLines((prev) => prev.filter((line) => line.line_id !== record.line_id))
            }
          >
            绉婚櫎
          </Button>
        )
      }
    ],
    [canWrite]
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
        String(item.barcode ?? '').toLowerCase().includes(keyword)
      );
    });
  }, [detail?.items, toolbarSearchKeyword]);

  const leftPanel = useMemo(
    () => (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Card size="small" title="质检流程">
          <Typography.Paragraph style={{ marginBottom: 8 }}>
            1. 选择 SKU、数量和质检结果。
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            2. 提交后自动回到入库详情页。
          </Typography.Paragraph>
        </Card>
        <Card size="small" title="当前进度">
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            单据状态：{detail?.doc.status ?? '-'}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            待提交质检：{inspectLines.length} 行
          </Typography.Paragraph>
        </Card>
      </Space>
    ),
    [detail?.doc.status, inspectLines.length]
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
      <Card size="small" title="质检摘要">
        <Typography.Paragraph style={{ marginBottom: 6 }}>
          入库明细：{detail?.items.length ?? 0} 行
        </Typography.Paragraph>
        <Typography.Paragraph style={{ marginBottom: 6 }}>
          可见明细：{filteredItems.length} 行
        </Typography.Paragraph>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          待提交质检：{inspectLines.length} 行
        </Typography.Paragraph>
      </Card>
    ),
    [detail?.items.length, filteredItems.length, inspectLines.length]
  );

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        items={[
          {
            key: 'inspect-lines',
            label: `待提交质检 (${inspectLines.length})`,
            children: (
              <Table<InspectLine>
                size="small"
                rowKey={(record) => record.line_id}
                columns={inspectLineColumns}
                dataSource={inspectLines}
                pagination={false}
              />
            )
          }
        ]}
      />
    ),
    [inspectLineColumns, inspectLines]
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
      message.warning('SKU 涓嶅湪入库鍗曡寖鍥村唴');
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
      message.error('暂无入库鎿嶄綔权限');
      return;
    }
    if (!inspectLines.length) {
      message.warning('璇峰厛褰曞叆质检明细');
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

  const toolbar = (
    <WorkbenchToolbar
      leftContent={toolbarLeft}
      searchValue={toolbarSearchInput}
      onSearchChange={setToolbarSearchInput}
      onSearch={() => setToolbarSearchKeyword(toolbarSearchInput.trim())}
      searchPlaceholder="按 SKU / 条码筛选入库明细"
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('顶部搜索仅用于筛选展示，不会影响提交数据')}
    />
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card title="入库明细">
        <Table<InboundDocItem>
          data-testid="inbound-inspect-items"
          rowKey={(record) => record.sku_id}
          columns={inboundItemColumns}
          dataSource={filteredItems}
          pagination={false}
        />
      </Card>
      <Card title="质检鐧昏">
        <Form form={inspectForm} layout="inline" style={{ marginBottom: 12 }}>
          <Form.Item name="sku_id" rules={[{ required: true, message: '璇烽€夋嫨 SKU' }]}>
            <Select
              allowClear
              disabled={!canWrite}
              placeholder="閫夋嫨 SKU"
              options={skuOptions}
              style={{ width: 240 }}
            />
          </Form.Item>
          <Form.Item name="qty" rules={[{ required: true, message: '请输入数量' }]}>
            <InputNumber min={1} disabled={!canWrite} placeholder="鏁伴噺" />
          </Form.Item>
          <Form.Item name="result" rules={[{ required: true, message: '璇烽€夋嫨缁撴灉' }]}>
            <Select
              disabled={!canWrite}
              options={resultOptions}
              placeholder="缁撴灉"
              style={{ width: 160 }}
            />
          </Form.Item>
          <Form.Item name="note">
            <Input disabled={!canWrite} placeholder="澶囨敞" />
          </Form.Item>
          <Form.Item>
            <Button disabled={!canWrite} onClick={handleAddInspectLine}>
              娣诲姞
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

  return (
    <WorkbenchPageTemplate
      toolbar={toolbar}
      tableContent={tableContent}
      rightPanel={rightPanel}
      bottomPanel={bottomPanel}
    />
  );
};
