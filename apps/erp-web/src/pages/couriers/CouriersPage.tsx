import { Button, Card, Col, Form, Input, Row, Space, Switch, Tag, Typography, message } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Courier } from '@/api/couriers';
import { listCouriers, updateCourier } from '@/api/couriers';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const CouriersPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('couriers.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [updatingId, setUpdatingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['couriers', tenantId, query],
    queryFn: () => listCouriers(tenantId, query),
    keepPreviousData: true
  });

  const updateMutation = useMutation({
    mutationFn: ({ courierId, status }: { courierId: string; status: string }) =>
      updateCourier(tenantId, courierId, { status }),
    onMutate: ({ courierId }) => {
      setUpdatingId(courierId);
    },
    onSuccess: () => {
      message.success('快递状态已更新');
      queryClient.invalidateQueries({ queryKey: ['couriers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新快递状态失败');
    },
    onSettled: () => {
      setUpdatingId(null);
    }
  });

  const columns = useMemo<ColumnsType<Courier>>(
    () => [
      {
        title: '快递名称',
        dataIndex: 'name'
      },
      {
        title: '快递编码',
        dataIndex: 'code'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value, record) => {
          const enabled = value === 'ACTIVE';
          return (
            <Space>
              <Switch
                checked={enabled}
                disabled={!canWrite || !tenantId}
                loading={updatingId === record.courier_id}
                onChange={(checked) => {
                  if (!tenantId) {
                    message.warning('请先选择商家');
                    return;
                  }
                  updateMutation.mutate({
                    courierId: record.courier_id,
                    status: checked ? 'ACTIVE' : 'DISABLED'
                  });
                }}
              />
              {enabled ? <Tag color="green">启用</Tag> : <Tag>停用</Tag>}
            </Space>
          );
        }
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      }
    ],
    [canWrite, updateMutation, updatingId]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      快递目录
    </Typography.Title>
  );

  const header = (
    <Card>
      <Form form={form} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={8}>
            <Form.Item name="keyword" label="关键词">
              <Input allowClear placeholder="输入快递名称/编码" />
            </Form.Item>
          </Col>
          <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
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
  );

  return (
    <UiModule03MainTableSection<Courier>
      toolbarLeft={toolbarLeft}
      header={header}
      rowKey="courier_id"
      columns={columns}
      data={data?.items ?? []}
      loading={isLoading || isFetching}
      error={error}
      onRetry={() => refetch()}
      errorTitle="快递列表加载失败"
      emptyText="暂无快递"
      total={data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      columnSettingsKey="couriers.table.columns"
    />
  );
};
