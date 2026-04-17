import {
  Button,
  Card,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  applyStockAdjustment,
  createStockAdjustment,
  getStockAdjustment,
  listStockAdjustments
} from '@/api/stockAdjustments';
import type {
  StockAdjustDetail,
  StockAdjustItemCreate,
  StockAdjustListQuery,
  StockAdjustSummary
} from '@/api/stockAdjustments';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { ErrorState } from '@/components/QueryState';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type AdjustFormValues = {
  warehouse_id: string;
  reason: string;
  items: { sku_id: string; qty: number }[];
};

const reasonOptions = [
  { label: '报损(SCRAP)', value: 'SCRAP' },
  { label: '报溢(GAIN)', value: 'GAIN' }
];

export const StockAdjustmentsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeAdjustId, setActiveAdjustId] = useState<string | null>(null);
  const [form] = Form.useForm<AdjustFormValues>();

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'stock-adjustments'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const listQuery = useMemo<StockAdjustListQuery>(
    () => ({
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['stock-adjustments', tenantId, listQuery],
    queryFn: () => listStockAdjustments(tenantId, listQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const detailQuery = useQuery({
    queryKey: ['stock-adjustments', tenantId, activeAdjustId],
    queryFn: () => getStockAdjustment(tenantId, activeAdjustId ?? ''),
    enabled: tenantReady && Boolean(activeAdjustId)
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      const values = await form.validateFields();
      const items: StockAdjustItemCreate[] = values.items.map((item) => ({
        sku_id: item.sku_id,
        qty: item.qty
      }));
      return createStockAdjustment(tenantId, {
        warehouse_id: values.warehouse_id,
        reason: values.reason,
        items
      });
    },
    onSuccess: async () => {
      message.success('调整单已创建');
      setCreateOpen(false);
      form.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['stock-adjustments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建调整单失败');
    }
  });

  const applyMutation = useMutation({
    mutationFn: () => applyStockAdjustment(tenantId, activeAdjustId ?? ''),
    onSuccess: async (detail) => {
      message.success('调整单已应用');
      queryClient.setQueryData(['stock-adjustments', tenantId, activeAdjustId], detail);
      await queryClient.invalidateQueries({ queryKey: ['stock-adjustments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '应用调整单失败');
    }
  });

  const columns = useMemo<ColumnsType<StockAdjustSummary>>(
    () => [
      {
        title: '调整单号',
        dataIndex: 'adjust_id'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value) =>
          warehouseOptions.find((option) => option.value === value)?.label ?? value
      },
      {
        title: '原因',
        dataIndex: 'reason'
      },
      {
        title: '状态',
        dataIndex: 'status'
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button
            size="small"
            onClick={() => {
              setActiveAdjustId(record.adjust_id);
              setDetailOpen(true);
            }}
          >
            详情
          </Button>
        )
      }
    ],
    [warehouseOptions]
  );

  const detail: StockAdjustDetail | undefined = detailQuery.data;

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        库存调整
      </Typography.Title>
      <Button type="primary" onClick={() => setCreateOpen(true)}>
        新建调整
      </Button>
    </Space>
  );

  const header = warehouseQuery.error ? (
    <ErrorState
      title="仓库加载失败"
      error={warehouseQuery.error}
      onRetry={() => warehouseQuery.refetch()}
    />
  ) : null;

  return (
    <>
      <UiModule03MainTableSection<StockAdjustSummary>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="adjust_id"
        columns={columns}
        data={listQueryState.data?.items ?? []}
        loading={listQueryState.isLoading || listQueryState.isFetching}
        error={listQueryState.error}
        onRetry={() => listQueryState.refetch()}
        errorTitle="调整单列表加载失败"
        emptyText="暂无调整单"
        total={listQueryState.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
      />
      <Modal
        title="新建调整单"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={() => createMutation.mutate()}
        confirmLoading={createMutation.isPending}
        destroyOnHidden
      >
        <Form form={form} layout="vertical" initialValues={{ items: [{ sku_id: '', qty: 1 }] }}>
          <Form.Item
            name="warehouse_id"
            label="仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item
            name="reason"
            label="原因"
            rules={[{ required: true, message: '请选择原因' }]}
          >
            <Select options={reasonOptions} placeholder="选择原因" />
          </Form.Item>
          <Form.List name="items">
            {(fields, { add, remove }) => (
              <Space direction="vertical" style={{ width: '100%' }}>
                {fields.map((field) => (
                  <Space key={field.key} align="baseline" style={{ width: '100%' }}>
                    <Form.Item
                      {...field}
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true, message: '请输入SKU' }]}
                    >
                      <Input placeholder="SKU ID" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'qty']}
                      rules={[{ required: true, message: '请输入数量' }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    <Button onClick={() => remove(field.name)}>删除</Button>
                  </Space>
                ))}
                <Button onClick={() => add({ sku_id: '', qty: 1 })}>添加SKU</Button>
              </Space>
            )}
          </Form.List>
        </Form>
      </Modal>
      <Drawer
        title="调整单详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={640}
        destroyOnHidden
      >
        {detailQuery.isLoading ? (
          <Card loading />
        ) : detail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Space>
              <Button
                type="primary"
                onClick={() => applyMutation.mutate()}
                loading={applyMutation.isPending}
                disabled={detail.doc.status !== 'DRAFT'}
              >
                应用调整
              </Button>
            </Space>
            <Table
              rowKey="adjust_item_id"
              columns={[
                { title: 'SKU ID', dataIndex: 'sku_id' },
                { title: '数量', dataIndex: 'qty' }
              ]}
              dataSource={detail.items}
              pagination={false}
              size="small"
            />
          </Space>
        ) : null}
      </Drawer>
    </>
  );
};
