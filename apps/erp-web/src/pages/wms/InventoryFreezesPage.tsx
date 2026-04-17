import {
  Button,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  cancelInventoryFreeze,
  createInventoryFreeze,
  listInventoryFreezes
} from '@/api/inventoryFreezes';
import type {
  InventoryFreeze,
  InventoryFreezeCreatePayload,
  InventoryFreezeListQuery
} from '@/api/inventoryFreezes';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { ErrorState } from '@/components/QueryState';
import { useFeatureFlag } from '@/features/featureFlags';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const InventoryFreezesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const autoFreezeFlag = useFeatureFlag('flows.auto_freeze');
  const queryClient = useQueryClient();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [form] = Form.useForm<InventoryFreezeCreatePayload>();

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inventory-freezes'],
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

  const listQuery = useMemo<InventoryFreezeListQuery>(
    () => ({
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['inventory-freezes', tenantId, listQuery],
    queryFn: () => listInventoryFreezes(tenantId, listQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      const values = await form.validateFields();
      return createInventoryFreeze(tenantId, values);
    },
    onSuccess: async () => {
      message.success('冻结已创建');
      setCreateOpen(false);
      form.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['inventory-freezes', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建冻结失败');
    }
  });

  const cancelMutation = useMutation({
    mutationFn: async (freeze: InventoryFreeze) =>
      cancelInventoryFreeze(tenantId, freeze.freeze_id),
    onSuccess: async () => {
      message.success('冻结已取消');
      await queryClient.invalidateQueries({ queryKey: ['inventory-freezes', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '取消冻结失败');
    }
  });

  const columns = useMemo<ColumnsType<InventoryFreeze>>(
    () => [
      {
        title: '冻结单号',
        dataIndex: 'freeze_id'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value) =>
          warehouseOptions.find((option) => option.value === value)?.label ?? value
      },
      {
        title: 'SKU ID',
        dataIndex: 'sku_id'
      },
      {
        title: '数量',
        dataIndex: 'qty'
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
            disabled={record.status !== 'ACTIVE'}
            loading={cancelMutation.isPending && record.status === 'ACTIVE'}
            onClick={() => cancelMutation.mutate(record)}
          >
            取消冻结
          </Button>
        )
      }
    ],
    [cancelMutation, warehouseOptions]
  );

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        库存冻结
      </Typography.Title>
      <Button
        type="primary"
        onClick={() => setCreateOpen(true)}
        disabled={!autoFreezeFlag.enabled}
      >
        新建冻结
      </Button>
    </Space>
  );

  const header = !autoFreezeFlag.enabled || warehouseQuery.error ? (
    <Space direction="vertical" size={8}>
      {!autoFreezeFlag.enabled ? (
        <Typography.Text type="warning">{autoFreezeFlag.reason ?? '自动冻结已关闭'}</Typography.Text>
      ) : null}
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
    </Space>
  ) : null;

  return (
    <>
      <UiModule03MainTableSection<InventoryFreeze>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="freeze_id"
        columns={columns}
        data={listQueryState.data?.items ?? []}
        loading={listQueryState.isLoading || listQueryState.isFetching}
        error={listQueryState.error}
        onRetry={() => listQueryState.refetch()}
        errorTitle="冻结列表加载失败"
        emptyText="暂无冻结记录"
        total={listQueryState.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
      />
      <Modal
        title="新建冻结"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={() => createMutation.mutate()}
        confirmLoading={createMutation.isPending}
        destroyOnHidden
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="warehouse_id"
            label="仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item
            name="sku_id"
            label="SKU ID"
            rules={[{ required: true, message: '请输入SKU ID' }]}
          >
            <Input placeholder="SKU ID" />
          </Form.Item>
          <Form.Item
            name="qty"
            label="数量"
            rules={[{ required: true, message: '请输入数量' }]}
          >
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="reason" label="原因">
            <Input placeholder="冻结原因" />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
