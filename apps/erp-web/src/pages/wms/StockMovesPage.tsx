import {
  Button,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  applyStockMove,
  createStockMove,
  getStockMove,
  listStockMoves
} from '@/api/stockMoves';
import type {
  StockMoveDetail,
  StockMoveItemCreate,
  StockMoveListQuery,
  StockMoveSummary
} from '@/api/stockMoves';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { UiModule03MainTableSection, UiModule04BottomTable } from '@/components/ui-modules';
import { isTenantSelected, useTenantId } from '@/features/tenant';


const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type MoveFormValues = {
  warehouse_id: string;
  from_location_id: string;
  to_location_id: string;
  items: { sku_id: string; qty: number }[];
};

type StockMoveDetailItem = StockMoveDetail['items'][number];

export const StockMovesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeMoveId, setActiveMoveId] = useState<string | null>(null);
  const [form] = Form.useForm<MoveFormValues>();

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'stock-moves'],
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

  const listQuery = useMemo<StockMoveListQuery>(
    () => ({
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['stock-moves', tenantId, listQuery],
    queryFn: () => listStockMoves(tenantId, listQuery),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const detailQuery = useQuery({
    queryKey: ['stock-moves', tenantId, activeMoveId],
    queryFn: () => getStockMove(tenantId, activeMoveId ?? ''),
    enabled: tenantReady && Boolean(activeMoveId)
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      const values = await form.validateFields();
      const items: StockMoveItemCreate[] = values.items.map((item) => ({
        sku_id: item.sku_id,
        qty: item.qty
      }));
      return createStockMove(tenantId, {
        warehouse_id: values.warehouse_id,
        from_location_id: values.from_location_id,
        to_location_id: values.to_location_id,
        items
      });
    },
    onSuccess: async () => {
      message.success('移库单已创建');
      setCreateOpen(false);
      form.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['stock-moves', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建移库单失败');
    }
  });

  const applyMutation = useMutation({
    mutationFn: () => applyStockMove(tenantId, activeMoveId ?? ''),
    onSuccess: async (detail) => {
      message.success('移库单已应用');
      queryClient.setQueryData(['stock-moves', tenantId, activeMoveId], detail);
      await queryClient.invalidateQueries({ queryKey: ['stock-moves', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '应用移库单失败');
    }
  });

  const columns = useMemo<ColumnsType<StockMoveSummary>>(
    () => [
      {
        title: '移库单号',
        dataIndex: 'move_id'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value) => warehouseMap.get(value) ?? value
      },
      {
        title: '来源库位',
        dataIndex: 'from_location_id',
        render: (value) => value ?? '-'
      },
      {
        title: '目标库位',
        dataIndex: 'to_location_id',
        render: (value) => value ?? '-'
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
            data-testid="stock-moves-detail-button"
            onClick={() => {
              setActiveMoveId(record.move_id);
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

  const detailColumns = useMemo<ColumnsType<StockMoveDetailItem>>(
    () => [
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: '数量', dataIndex: 'qty' }
    ],
    []
  );

  const detail: StockMoveDetail | undefined = detailQuery.data;

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        移库管理
      </Typography.Title>
      <Typography.Text type="secondary">统一查看仓内库位移动记录和执行状态。</Typography.Text>
    </Space>
  );

  const toolbarRight = (
    <Space align="center" size={8} wrap>
      <Button onClick={() => listQueryState.refetch()} loading={listQueryState.isFetching}>
        刷新
      </Button>
      <Button type="primary" onClick={() => setCreateOpen(true)} data-testid="stock-moves-create-button">
        新建移库
      </Button>
      <Button disabled>导出</Button>
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
      <UiModule03MainTableSection<StockMoveSummary>
        dataTestId="stock-moves-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={header}
        rowKey="move_id"
        columns={columns}
        data={listQueryState.data?.items ?? []}
        loading={listQueryState.isLoading || listQueryState.isFetching}
        error={listQueryState.error}
        onRetry={() => listQueryState.refetch()}
        errorTitle="移库单列表加载失败"
        emptyText="暂无移库单"
        total={listQueryState.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="wms.stock-moves.table.columns"
        showColumnSettings
      />
      <Modal
        title="新建移库单"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={() => createMutation.mutate()}
        confirmLoading={createMutation.isPending}
        destroyOnHidden
      >
        <Form
          form={form}
          layout="vertical"
          initialValues={{ items: [{ sku_id: '', qty: 1 }] }}
        >
          <Form.Item
            name="warehouse_id"
            label="仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item
            name="from_location_id"
            label="来源库位"
            rules={[{ required: true, message: '请输入来源库位' }]}
          >
            <Input placeholder="来源库位 ID" />
          </Form.Item>
          <Form.Item
            name="to_location_id"
            label="目标库位"
            rules={[{ required: true, message: '请输入目标库位' }]}
          >
            <Input placeholder="目标库位 ID" />
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
        title="移库单详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={640}
        destroyOnHidden
      >
        {detailQuery.isLoading ? (
          <LoadingState loading message="正在加载移库单详情" onRetry={() => detailQuery.refetch()} />
        ) : detail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Space>
              <Button
                type="primary"
                onClick={() => applyMutation.mutate()}
                loading={applyMutation.isPending}
                disabled={detail.doc.status !== 'DRAFT'}
              >
                应用移库
              </Button>
            </Space>
            <UiModule04BottomTable syncKey={detail.doc.move_id}>
              <UiModule03MainTableSection<StockMoveDetailItem>
                dataTestId="stock-moves-detail-items-grid"
                gridPreset="industrial-review-compact"
                selectionCellMode="custom"
                selectionHeaderMode="allInvert"
                rowKey="move_item_id"
                columns={detailColumns}
                data={detail.items}
                loading={detailQuery.isLoading || detailQuery.isFetching}
                error={detailQuery.error}
                onRetry={() => detailQuery.refetch()}
                errorTitle="移库明细加载失败"
                emptyText="暂无移库明细"
                total={detail.items.length}
                pagination={{ current: 1, pageSize: detail.items.length || 1 }}
                onPaginationChange={() => undefined}
                showStatusBar={false}
              />
            </UiModule04BottomTable>
          </Space>
        ) : null}
      </Drawer>
    </>
  );
};
