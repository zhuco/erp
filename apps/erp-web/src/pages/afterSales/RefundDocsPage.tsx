import { Button, Card, Col, Form, Input, Row, Select, Space, Tag, Typography, message } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type { RefundDoc, RefundDocListQuery } from '@/api/afterSalesOps';
import { listRefundDocs, requestAfterSalesRefund } from '@/api/afterSalesOps';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

const statusOptions = [
  { label: '待处理', value: 'PENDING' },
  { label: '已发送', value: 'SENT' },
  { label: '失败', value: 'FAILED' },
  { label: '完成', value: 'DONE' }
];

const statusColorMap: Record<string, string> = {
  PENDING: 'orange',
  SENT: 'blue',
  FAILED: 'red',
  DONE: 'green'
};

const statusLabelMap = new Map(statusOptions.map((option) => [option.value, option.label]));
const channelLabelMap: Record<string, string> = {
  PLATFORM: '平台',
  MANUAL: '人工'
};

const formatDateTime = (value?: string) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type FilterValues = {
  status?: string;
  after_sales_id?: string;
};

export const RefundDocsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const query = useMemo<RefundDocListQuery>(() => {
    return {
      status: filters.status as RefundDocListQuery['status'],
      after_sales_id: filters.after_sales_id?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['refund-docs', tenantId, query],
    queryFn: () => listRefundDocs(tenantId, query),
    keepPreviousData: true
  });

  const handleRetry = async (record: RefundDoc) => {
    try {
      await requestAfterSalesRefund(tenantId, record.after_sales_id, {
        amount_cent: record.amount_cent,
        channel: record.channel
      });
      message.success('已重新发起退款');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const columns = useMemo<ColumnsType<RefundDoc>>(
    () => [
      {
        title: '退款单号',
        dataIndex: 'id'
      },
      {
        title: '售后单号',
        dataIndex: 'after_sales_id',
        render: (_, record) => <Link to={`/after-sales/${record.after_sales_id}`}>{record.after_sales_id}</Link>
      },
      {
        title: '金额',
        dataIndex: 'amount_cent',
        render: (value: number) => `¥${(value / 100).toFixed(2)}`
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: string) => (
          <Tag color={statusColorMap[value] ?? 'default'}>
            {statusLabelMap.get(value) ?? value}
          </Tag>
        )
      },
      {
        title: '渠道',
        dataIndex: 'channel',
        render: (value: string) => channelLabelMap[value] ?? value
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) =>
          canWrite && record.status === 'FAILED' ? (
            <Button size="small" type="link" onClick={() => handleRetry(record)}>
              重试
            </Button>
          ) : null
      }
    ],
    [canWrite]
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

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} data-testid="refund-docs-title">
        退款处理
      </Typography.Title>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="选择状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="after_sales_id" label="售后单号">
                <Input allowClear placeholder="输入售后单号" />
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

  return (
    <UiModule03MainTableSection<RefundDoc>
      header={header}
      rowKey="id"
      columns={columns}
      data={data?.items ?? []}
      loading={isLoading || isFetching}
      error={error}
      onRetry={() => refetch()}
      errorTitle="退款列表加载失败"
      emptyText="暂无退款记录"
      total={data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      columnSettingsKey="refund-docs.table.columns"
    />
  );
};
