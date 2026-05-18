import { Button, Card, Form, Input, Space, Table, Tabs, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import type { InboundDocStatus, InboundDocSummary, InboundDocType } from '@/api/inbounds';
import { listInbounds } from '@/api/inbounds';
import { listWarehouses } from '@/api/warehouses';
import { useFeatureFlag } from '@/features/featureFlags';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { WorkbenchPageTemplate, WorkbenchToolbar } from '@/components/page-templates';
import { isTenantSelected, useTenantId } from '@/features/tenant';
import { useQuery } from '@tanstack/react-query';

type FormValues = {
  keyword: string;
};

const docTypeOptions: Array<{ label: string; value: InboundDocType }> = [
  { label: '采购入库', value: 'PURCHASE' },
  { label: '退货入库', value: 'RETURN' },
  { label: '其他入库', value: 'OTHER' }
];

const statusColors: Record<InboundDocStatus, string> = {
  DRAFT: 'default',
  AUDITED: 'gold',
  RECEIVING: 'blue',
  PUTAWAY_DONE: 'green',
  CANCELLED: 'red'
};

export const InboundQuickReceivePage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const tenantReady = isTenantSelected(tenantId);
  const [form] = Form.useForm<FormValues>();
  const [loading, setLoading] = useState(false);
  const [results, setResults] = useState<InboundDocSummary[]>([]);
  const [toolbarSearchInput, setToolbarSearchInput] = useState('');
  const [toolbarSearchKeyword, setToolbarSearchKeyword] = useState('');
  const { setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible } = useWorkbenchPanels();

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inbound-quick'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const columns = useMemo<ColumnsType<InboundDocSummary>>(
    () => [
      { title: '入库单号', dataIndex: 'doc_no' },
      {
        title: '类型',
        dataIndex: 'doc_type',
        render: (value: InboundDocType) => docTypeOptions.find((item) => item.value === value)?.label
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: InboundDocStatus) => (
          <Tag color={statusColors[value] || 'default'}>{value}</Tag>
        )
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" type="primary" onClick={() => navigate(`/wms/inbounds/${record.doc_id}/receive`)}>
            进入收货
          </Button>
        )
      }
    ],
    [navigate, warehouseMap]
  );

  const filteredResults = useMemo(() => {
    const keyword = toolbarSearchKeyword.trim().toLowerCase();
    if (!keyword) {
      return results;
    }
    return results.filter((item) => {
      const warehouseName = warehouseMap.get(item.warehouse_id) ?? item.warehouse_id;
      return (
        item.doc_no.toLowerCase().includes(keyword) ||
        item.doc_type.toLowerCase().includes(keyword) ||
        item.status.toLowerCase().includes(keyword) ||
        String(warehouseName).toLowerCase().includes(keyword)
      );
    });
  }, [results, toolbarSearchKeyword, warehouseMap]);

  const leftPanel = useMemo(
    () => (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Card size="small" title="快速流程">
          <Typography.Paragraph style={{ marginBottom: 8 }}>
            1. 输入 PO 或到货单号并查询。
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            2. 匹配到目标单据后点击“进入收货”。
          </Typography.Paragraph>
        </Card>
        <Card size="small" title="查询状态">
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            查询结果：{results.length} 条
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            可见结果：{filteredResults.length} 条
          </Typography.Paragraph>
        </Card>
      </Space>
    ),
    [filteredResults.length, results.length]
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
      <Card size="small" title="命中概览">
        <Typography.Paragraph style={{ marginBottom: 6 }}>
          总命中：{results.length}
        </Typography.Paragraph>
        <Typography.Paragraph style={{ marginBottom: 6 }}>
          当前筛选：{filteredResults.length}
        </Typography.Paragraph>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          仓库配置加载：{warehouseQuery.isSuccess ? '已加载' : '未就绪'}
        </Typography.Paragraph>
      </Card>
    ),
    [filteredResults.length, results.length, warehouseQuery.isSuccess]
  );

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        items={[
          {
            key: 'result-list',
            label: `匹配结果 (${filteredResults.length})`,
            children: (
              <Table<InboundDocSummary>
                size="small"
                rowKey={(record) => record.doc_id}
                columns={columns}
                dataSource={filteredResults}
                pagination={false}
                locale={{ emptyText: '暂无匹配结果' }}
              />
            )
          }
        ]}
      />
    ),
    [columns, filteredResults]
  );

  const handleSearch = async () => {
    const values = await form.validateFields();
    const keyword = values.keyword?.trim();
    if (!keyword) {
      message.warning('请输入 PO 或到货单号');
      return;
    }
    setLoading(true);
    try {
      const response = await listInbounds(tenantId, {
        doc_no_keyword: keyword,
        limit: 20,
        offset: 0
      });
      setResults(response.items);
      if (response.items.length === 1) {
        navigate(`/wms/inbounds/${response.items[0].doc_id}/receive`);
      } else if (!response.items.length) {
        message.info('未找到匹配的入库单');
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '查询入库单失败');
    } finally {
      setLoading(false);
    }
  };

  const toolbarLeft = (
    <Space align="center" size={10} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        收货登记
      </Typography.Title>
      <Typography.Text type="secondary">支持按单号快速跳转到收货页</Typography.Text>
    </Space>
  );

  const toolbar = (
    <WorkbenchToolbar
      leftContent={toolbarLeft}
      searchValue={toolbarSearchInput}
      onSearchChange={setToolbarSearchInput}
      onSearch={() => setToolbarSearchKeyword(toolbarSearchInput.trim())}
      searchPlaceholder="筛选结果：单号 / 类型 / 状态 / 仓库"
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('先通过表单查询，再用上方搜索框二次筛选')}
    />
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card>
        <Form form={form} layout="inline">
          <Form.Item name="keyword" rules={[{ required: true, message: '请输入 PO 或到货单号' }]}>
            <Input placeholder="PO/到货单号" style={{ width: 320 }} />
          </Form.Item>
          <Form.Item>
            <Button type="primary" loading={loading} onClick={handleSearch}>
              查询
            </Button>
          </Form.Item>
        </Form>
      </Card>
      <Card title="匹配结果">
        <Table<InboundDocSummary>
          rowKey={(record) => record.doc_id}
          columns={columns}
          dataSource={filteredResults}
          pagination={false}
          loading={loading}
          locale={{ emptyText: '暂无结果' }}
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
