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
  confirmStockTransferInbound,
  confirmStockTransferOutbound,
  createStockTransfer,
  getStockTransfer,
  listStockTransfers
} from '@/api/stockTransfers';
import type {
  StockTransferItemCreate,
  StockTransferListQuery,
  StockTransferSummary
} from '@/api/stockTransfers';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type TransferFormValues = {
  from_warehouse_id: string;
  to_warehouse_id: string;
  items: { sku_id: string; qty: number }[];
};

export const StockTransfersPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeTransferId, setActiveTransferId] = useState<string | null>(null);
  const [form] = Form.useForm<TransferFormValues>();

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'stock-transfers'],
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

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const listQuery = useMemo<StockTransferListQuery>(
    () => ({
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['stock-transfers', tenantId, listQuery],
    queryFn: () => listStockTransfers(tenantId, listQuery),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const detailQuery = useQuery({
    queryKey: ['stock-transfers', tenantId, activeTransferId],
    queryFn: () => getStockTransfer(tenantId, activeTransferId ?? ''),
    enabled: tenantReady && Boolean(activeTransferId)
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      const values = await form.validateFields();
      const items: StockTransferItemCreate[] = values.items.map((item) => ({
        sku_id: item.sku_id,
        qty: item.qty
      }));
      return createStockTransfer(tenantId, {
        from_warehouse_id: values.from_warehouse_id,
        to_warehouse_id: values.to_warehouse_id,
        items
      });
    },
    onSuccess: async () => {
      message.success('调拨单已创建');
      setCreateOpen(false);
      form.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['stock-transfers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建调拨单失败');
    }
  });

  const outboundMutation = useMutation({
    mutationFn: () => confirmStockTransferOutbound(tenantId, activeTransferId ?? ''),
    onSuccess: async (detail) => {
      message.success('调拨出库已确认');
      queryClient.setQueryData(['stock-transfers', tenantId, activeTransferId], detail);
      await queryClient.invalidateQueries({ queryKey: ['stock-transfers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '出库确认失败');
    }
  });

  const inboundMutation = useMutation({
    mutationFn: () => confirmStockTransferInbound(tenantId, activeTransferId ?? ''),
    onSuccess: async (detail) => {
      message.success('调拨入库已确认');
      queryClient.setQueryData(['stock-transfers', tenantId, activeTransferId], detail);
      await queryClient.invalidateQueries({ queryKey: ['stock-transfers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '入库确认失败');
    }
  });

  const columns = useMemo<ColumnsType<StockTransferSummary>>(
    () => [
      {
        title: '调拨单号',
        dataIndex: 'transfer_id'
      },
      {
        title: '调出仓',
        dataIndex: 'from_warehouse_id',
        render: (value) => warehouseMap.get(value) ?? value
      },
      {
        title: '调入仓',
        dataIndex: 'to_warehouse_id',
        render: (value) => warehouseMap.get(value) ?? value
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
              setActiveTransferId(record.transfer_id);
              setDetailOpen(true);
            }}
          >
            详情
          </Button>
        )
      }
    ],
    [warehouseMap]
  );

  const detail = detailQuery.data;

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        调拨管理
      </Typography.Title>
      <Button type="primary" onClick={() => setCreateOpen(true)}>
        新建调拨
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
      <UiModule03MainTableSection<StockTransferSummary>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="transfer_id"
        columns={columns}
        data={listQueryState.data?.items ?? []}
        loading={listQueryState.isLoading || listQueryState.isFetching}
        error={listQueryState.error}
        onRetry={() => listQueryState.refetch()}
        errorTitle="调拨列表加载失败"
        emptyText="暂无调拨单"
        total={listQueryState.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
      />
      <Modal
        title="新建调拨单"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={() => createMutation.mutate()}
        confirmLoading={createMutation.isPending}
        destroyOnHidden
      >
        <Form form={form} layout="vertical" initialValues={{ items: [{ sku_id: '', qty: 1 }] }}>
          <Form.Item
            name="from_warehouse_id"
            label="调出仓库"
            rules={[{ required: true, message: '请选择调出仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择调出仓库" />
          </Form.Item>
          <Form.Item
            name="to_warehouse_id"
            label="调入仓库"
            rules={[{ required: true, message: '请选择调入仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择调入仓库" />
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
        title="调拨详情"
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
                onClick={() => outboundMutation.mutate()}
                loading={outboundMutation.isPending}
                disabled={detail.transfer.status !== 'DRAFT'}
              >
                出库确认
              </Button>
              <Button
                type="primary"
                onClick={() => inboundMutation.mutate()}
                loading={inboundMutation.isPending}
                disabled={detail.transfer.status !== 'OUT_CONFIRMED'}
              >
                入库确认
              </Button>
            </Space>
            <Table
              rowKey="transfer_item_id"
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
