import { Button, Card, Form, Input, InputNumber, Select, Space, Table, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem } from '@/api/inbounds';
import { createOperationId, getInboundDetail, inspectInbound } from '@/api/inbounds';
import { ErrorState } from '@/components/QueryState';
import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';
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
      message.error(err instanceof Error ? err.message : '璐ㄦ鎻愪氦澶辫触');
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

  if (detailQuery.error) {
    return (
      <ErrorState
        title="璐ㄦ鏁版嵁鍔犺浇澶辫触"
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
      message.warning('SKU 涓嶅湪鍏ュ簱鍗曡寖鍥村唴');
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
      message.error('鏆傛棤鍏ュ簱鎿嶄綔鏉冮檺');
      return;
    }
    if (!inspectLines.length) {
      message.warning('璇峰厛褰曞叆璐ㄦ鏄庣粏');
      return;
    }
    inspectMutation.mutate({ docId: doc_id, items: inspectLines });
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} data-testid="inbound-inspect-title" style={{ margin: 0 }}>
        鍏ュ簱璐ㄦ
      </Typography.Title>
      <Button onClick={() => navigate(`/wms/inbounds/${doc_id}`)}>杩斿洖璇︽儏</Button>
      <Button
        type="primary"
        disabled={!canWrite}
        onClick={handleSubmitInspect}
        loading={inspectMutation.isPending}
      >
        鎻愪氦璐ㄦ
      </Button>
    </Space>
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card title="鍏ュ簱鏄庣粏">
        <Table<InboundDocItem>
          data-testid="inbound-inspect-items"
          rowKey={(record) => record.sku_id}
          columns={inboundItemColumns}
          dataSource={detail.items}
          pagination={false}
        />
      </Card>
      <Card title="璐ㄦ鐧昏">
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
          locale={{ emptyText: '鏆傛棤璐ㄦ鏄庣粏' }}
        />
      </Card>
    </Space>
  );

  return <WorkbenchMainTableTemplate toolbarLeft={toolbarLeft} tableContent={tableContent} />;
};

