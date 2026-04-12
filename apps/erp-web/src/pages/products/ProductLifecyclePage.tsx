import {
  Button,
  Card,
  Col,
  Form,
  Input,
  Popconfirm,
  Row,
  Select,
  Space,
  Steps,
  Tag,
  Typography,
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { ProductLifecycleListQuery } from '@/api/productLifecycles';
import {
  deleteProductLifecycle,
  listProductLifecycles,
  updateProductLifecycle,
} from '@/api/productLifecycles';
import { listSkus } from '@/api/skus';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type LifecycleStatus = 'DRAFT' | 'ACTIVE' | 'DISABLED' | 'ARCHIVED' | 'DELETED';

type LifecycleRecord = {
  id: string;
  sku_id: string;
  sku_code: string;
  sku_name: string;
  status: LifecycleStatus;
  inventory: number;
  updated_at: string;
  remark: string;
};

type FilterValues = {
  keyword?: string;
  status?: LifecycleStatus;
};

const statusLabels: Record<LifecycleStatus, string> = {
  DRAFT: '草稿',
  ACTIVE: '启用',
  DISABLED: '停用',
  ARCHIVED: '归档',
  DELETED: '已删除',
};

const statusColors: Record<LifecycleStatus, string> = {
  DRAFT: 'default',
  ACTIVE: 'green',
  DISABLED: 'orange',
  ARCHIVED: 'blue',
  DELETED: 'red',
};

export const ProductLifecyclePage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });

  const listQuery = useMemo<ProductLifecycleListQuery>(
    () => ({
      status: filters.status,
      offset: 0,
      limit: 200,
    }),
    [filters.status],
  );

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['product-lifecycles', tenantId, listQuery],
    queryFn: () => listProductLifecycles(tenantId, listQuery),
    enabled: Boolean(tenantId),
  });

  const skuQuery = useQuery({
    queryKey: ['skus', tenantId],
    queryFn: () => listSkus(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const skuMap = useMemo(
    () => new Map((skuQuery.data?.data ?? []).map((sku) => [sku.sku_id, sku])),
    [skuQuery.data?.data],
  );

  const records = useMemo<LifecycleRecord[]>(
    () =>
      (data?.items ?? []).map((item) => {
        const sku = skuMap.get(item.sku_id);
        return {
          id: item.lifecycle_id,
          sku_id: item.sku_id,
          sku_code: item.sku_code ?? sku?.sku_code ?? item.sku_id,
          sku_name: item.sku_name ?? sku?.name ?? '',
          status: item.status as LifecycleStatus,
          inventory: item.inventory_qty ?? 0,
          updated_at: item.updated_at ?? item.created_at ?? '',
          remark: item.remark ?? '',
        };
      }),
    [data?.items, skuMap],
  );

  const filteredData = useMemo(() => {
    const keyword = filters.keyword?.trim();
    return records.filter((item) => {
      if (filters.status && item.status !== filters.status) {
        return false;
      }
      if (keyword) {
        const target = `${item.sku_code}${item.sku_name}`.toLowerCase();
        return target.includes(keyword.toLowerCase());
      }
      return true;
    });
  }, [filters.keyword, filters.status, records]);

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const updateStatus = async (
    record: LifecycleRecord,
    status: LifecycleStatus,
    remark: string,
  ): Promise<boolean> => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return false;
    }
    try {
      await updateProductLifecycle(tenantId, record.id, { status, remark });
      await queryClient.invalidateQueries({ queryKey: ['product-lifecycles', tenantId] });
      return true;
    } catch (error) {
      message.error(error instanceof Error ? error.message : '更新生命周期失败');
      return false;
    }
  };

  const handleToggle = async (record: LifecycleRecord) => {
    if (!canWrite) {
      message.warning('暂无生命周期管理权限');
      return;
    }
    if (record.status === 'ACTIVE') {
      const success = await updateStatus(record, 'DISABLED', '手动停用');
      if (success) {
        message.success('商品已停用');
      }
      return;
    }
    if (record.status === 'DISABLED' || record.status === 'DRAFT') {
      const success = await updateStatus(record, 'ACTIVE', '重新启用');
      if (success) {
        message.success('商品已启用');
      }
      return;
    }
    message.warning('当前状态不可切换');
  };

  const handleArchive = async (record: LifecycleRecord) => {
    if (!canWrite) {
      message.warning('暂无生命周期管理权限');
      return;
    }
    const success = await updateStatus(record, 'ARCHIVED', '归档处理');
    if (success) {
      message.success('商品已归档');
    }
  };

  const handleDelete = async (record: LifecycleRecord) => {
    if (!canWrite) {
      message.warning('暂无生命周期管理权限');
      return;
    }
    if (record.inventory > 0) {
      message.warning('存在库存占用，无法删除');
      return;
    }
    if (record.status !== 'ARCHIVED') {
      message.warning('请先归档后再删除');
      return;
    }
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      await deleteProductLifecycle(tenantId, record.id);
      message.success('商品已删除');
      await queryClient.invalidateQueries({ queryKey: ['product-lifecycles', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除生命周期失败');
    }
  };

  const columns = useMemo<ColumnsType<LifecycleRecord>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: '商品名称', dataIndex: 'sku_name' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: LifecycleStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        ),
      },
      { title: '库存占用', dataIndex: 'inventory' },
      { title: '备注', dataIndex: 'remark' },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm'),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => handleToggle(record)}>
              {record.status === 'ACTIVE' ? '停用' : '启用'}
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.status === 'ARCHIVED'}
              onClick={() => handleArchive(record)}
            >
              归档
            </Button>
            <Popconfirm
              title="确认删除该商品？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => handleDelete(record)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        ),
      },
    ],
    [canWrite],
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }} data-testid="product-lifecycle-title">
        商品生命周期管理
      </Typography.Title>
      <Card title="生命周期流程" data-testid="product-lifecycle-steps">
        <Steps
          items={[
            { title: '草稿', description: '信息完善后提交' },
            { title: '启用', description: '可用于下单与履约' },
            { title: '停用', description: '禁止新单进入' },
            { title: '归档', description: '仅保留历史记录' },
            { title: '删除', description: '满足条件后清理' },
          ]}
        />
      </Card>
      <Card title="删除校验规则">
        <Space direction="vertical">
          <Typography.Text>1. 商品需处于归档状态。</Typography.Text>
          <Typography.Text>2. 无库存占用与在途调拨。</Typography.Text>
          <Typography.Text>3. 无未结算采购/销售单据。</Typography.Text>
        </Space>
      </Card>
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSearch}
          data-testid="product-lifecycle-filter-form"
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="SKU/商品名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select
                  allowClear
                  options={Object.entries(statusLabels).map(([value, label]) => ({
                    value,
                    label,
                  }))}
                  placeholder="全部状态"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      <Card title="生命周期清单">
        <UiModule03MainTableSection<LifecycleRecord>
          dataTestId="product-lifecycle-table"
          rowKey="id"
          columns={columns}
          data={filteredData}
          loading={isLoading || isFetching}
          onRetry={() => refetch()}
          errorTitle="生命周期加载失败"
          emptyText="暂无生命周期记录"
          total={filteredData.length}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="product.lifecycle.table.columns"
          enablePanels={false}
        />
      </Card>
    </Space>
  );
};
