import { Button, Card, Form, Input, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import type { InboundDocStatus, InboundDocSummary, InboundDocType } from '@/api/inbounds';
import { listInbounds } from '@/api/inbounds';
import { listWarehouses } from '@/api/warehouses';
import { useFeatureFlag } from '@/features/featureFlags';
import { UiModule02WorkbenchTemplate } from '@/components/ui-modules';
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
    <Typography.Title level={3} style={{ margin: 0 }}>
      收货登记
    </Typography.Title>
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
          dataSource={results}
          pagination={false}
          loading={loading}
          locale={{ emptyText: '暂无结果' }}
        />
      </Card>
    </Space>
  );

  return <UiModule02WorkbenchTemplate toolbarLeft={toolbarLeft} tableContent={tableContent} />;
};
