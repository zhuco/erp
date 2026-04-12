import { Button, Card, Col, Form, Row, Select, Space, Tag, Typography, message } from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useCallback, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type {
  AfterSalesListQuery,
  AfterSalesStatus,
  AfterSalesSummary,
  AfterSalesType
} from '@/api/afterSalesOps';
import {
  createAfterSalesExchangeShipment,
  createAfterSalesResendShipment,
  listAfterSales
} from '@/api/afterSalesOps';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  type?: 'ALL' | AfterSalesType;
  status?: 'ALL' | AfterSalesStatus;
};

const typeOptions = [
  { label: '全部', value: 'ALL' },
  { label: '换货', value: 'EXCHANGE' },
  { label: '补发', value: 'RESEND' }
];

const statusOptions = [
  { label: '全部', value: 'ALL' },
  { label: '待审核', value: 'OPEN' },
  { label: '已同意', value: 'APPROVED' },
  { label: '已收货', value: 'RECEIVED' },
  { label: '已退款', value: 'REFUNDED' },
  { label: '已关闭', value: 'CLOSED' },
  { label: '已驳回', value: 'REJECTED' }
];

const statusColorMap: Record<string, string> = {
  OPEN: 'orange',
  APPROVED: 'blue',
  RECEIVED: 'purple',
  REFUNDED: 'green',
  CLOSED: 'default',
  REJECTED: 'red'
};

const statusLabelMap = new Map(statusOptions.map((option) => [option.value, option.label]));

const formatDateTime = (value?: string) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const AfterSalesExchangesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const navigate = useNavigate();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({
    type: 'ALL',
    status: 'ALL'
  });
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const query = useMemo<AfterSalesListQuery>(() => {
    const typeFilter = filters.type && filters.type !== 'ALL' ? filters.type : undefined;
    const statusFilter =
      filters.status && filters.status !== 'ALL' ? filters.status : undefined;
    return {
      after_sales_type: typeFilter,
      status: statusFilter,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters.status, filters.type, pagination.current, pagination.pageSize]);

  const { data, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sales', tenantId, 'exchange-resend', query],
    queryFn: () => listAfterSales(tenantId, query),
    keepPreviousData: true
  });

  const handleSearch = (values: FilterValues) => {
    setFilters({
      type: values.type ?? 'ALL',
      status: values.status ?? 'ALL'
    });
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({ type: 'ALL', status: 'ALL' });
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleCreateShipment = useCallback(async (record: AfterSalesSummary) => {
    try {
      if (record.type === 'EXCHANGE') {
        await createAfterSalesExchangeShipment(tenantId, record.id, {});
        message.success('换货包裹已生成');
      } else if (record.type === 'RESEND') {
        await createAfterSalesResendShipment(tenantId, record.id, {});
        message.success('补发包裹已生成');
      } else {
        message.warning('当前售后类型不支持生成包裹');
        return;
      }
      navigate('/oms/shipments');
    } catch (err) {
      message.error((err as Error).message);
    }
  }, [navigate, tenantId]);

  const columns = useMemo<ColumnsType<AfterSalesSummary>>(
    () => [
      {
        title: '售后单号',
        dataIndex: 'id',
        render: (value: string, record) => (
          <Link to={`/after-sales/${record.id}`}>{value}</Link>
        )
      },
      {
        title: '订单号',
        dataIndex: 'order_id'
      },
      {
        title: '类型',
        dataIndex: 'type',
        render: (value: AfterSalesType) => {
          const color = value === 'EXCHANGE' ? 'blue' : 'purple';
          const label = typeOptions.find((item) => item.value === value)?.label ?? value;
          return <Tag color={color}>{label}</Tag>;
        }
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: AfterSalesStatus) => (
          <Tag color={statusColorMap[value] ?? 'default'}>
            {statusLabelMap.get(value) ?? value}
          </Tag>
        )
      },
      {
        title: '退款金额',
        dataIndex: 'amount_cent',
        render: (value: number) => `¥${(value / 100).toFixed(2)}`
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value: string) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          if (!canWrite) {
            return null;
          }
          const canShipment =
            ['APPROVED', 'RECEIVED'].includes(record.status) &&
            ['EXCHANGE', 'RESEND'].includes(record.type);
          return (
            <Space>
              {canShipment && (
                <Button size="small" type="link" onClick={() => handleCreateShipment(record)}>
                  {record.type === 'EXCHANGE' ? '生成换货包裹' : '生成补发包裹'}
                </Button>
              )}
              <Button size="small" type="link" onClick={() => navigate(`/after-sales/${record.id}`)}>
                查看详情
              </Button>
            </Space>
          );
        }
      }
    ],
    [canWrite, handleCreateShipment, navigate]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title
        level={3}
        style={{ marginBottom: 0 }}
        data-testid="after-sales-exchanges-title"
      >
        换货/补发流程
      </Typography.Title>
      <Card>
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            type: 'ALL',
            status: 'ALL'
          }}
          onFinish={handleSearch}
        >
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item label="类型" name="type">
                <Select allowClear options={typeOptions} placeholder="选择类型" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item label="状态" name="status">
                <Select allowClear options={statusOptions} placeholder="选择状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
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
    </Space>
  );

  const tableFooter = (
    <Card>
      <Typography.Text type="secondary">
        支持从售后单生成换货/补发包裹，发货后可在出库与物流模块追踪状态。
      </Typography.Text>
    </Card>
  );

  return (
    <div data-testid="after-sales-exchanges-table">
      <UiModule03MainTableSection<AfterSalesSummary>
        header={header}
        rowKey="id"
        columns={columns}
        data={data?.items ?? []}
        loading={isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="售后列表加载失败"
        emptyText="暂无售后记录"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="after-sales.exchanges.columns"
        tableFooter={tableFooter}
      />
    </div>
  );
};
