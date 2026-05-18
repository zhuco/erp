import { Button, Col, DatePicker, Form, Input, Modal, Row, Select, Space, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type {
  AfterSalesIntercept,
  AfterSalesInterceptCreatePayload,
  AfterSalesInterceptListQuery,
  AfterSalesInterceptStatus
} from '@/api/afterSalesWorkflow';
import {
  createAfterSalesIntercept,
  escalateAfterSalesIntercept,
  listAfterSalesIntercepts,
  retryAfterSalesIntercept
} from '@/api/afterSalesWorkflow';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  status?: AfterSalesInterceptStatus;
  order_id?: string;
  tracking_no?: string;
  requested_range?: [Dayjs, Dayjs];
};

type CreateValues = {
  order_id: string;
  after_sales_id?: string;
  carrier_name?: string;
  tracking_no?: string;
  status?: AfterSalesInterceptStatus;
  reason?: string;
  requested_at?: Dayjs;
};

const statusOptions = [
  { label: '拦截中', value: 'PROCESSING' },
  { label: '拦截成功', value: 'SUCCESS' },
  { label: '拦截失败', value: 'FAILED' },
  { label: '已升级', value: 'ESCALATED' }
];

const statusColors: Record<AfterSalesInterceptStatus, string> = {
  PROCESSING: 'blue',
  SUCCESS: 'green',
  FAILED: 'red',
  ESCALATED: 'gold'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const AfterSalesInterceptsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const [form] = Form.useForm<FilterValues>();
  const [createForm] = Form.useForm<CreateValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<IndustrialTablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [creating, setCreating] = useState(false);

  const query = useMemo<AfterSalesInterceptListQuery>(() => {
    const range = filters.requested_range;
    return {
      status: filters.status,
      order_id: filters.order_id?.trim() || undefined,
      tracking_no: filters.tracking_no?.trim() || undefined,
      requested_from: range?.[0]?.toISOString(),
      requested_to: range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sales-intercepts', tenantId, query],
    queryFn: () => listAfterSalesIntercepts(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleCreate = async () => {
    try {
      const values = await createForm.validateFields();
      setCreating(true);
      const payload: AfterSalesInterceptCreatePayload = {
        order_id: values.order_id.trim(),
        after_sales_id: values.after_sales_id?.trim() || undefined,
        carrier_name: values.carrier_name?.trim() || undefined,
        tracking_no: values.tracking_no?.trim() || undefined,
        status: values.status,
        reason: values.reason?.trim() || undefined,
        requested_at: values.requested_at?.toISOString()
      };
      await createAfterSalesIntercept(tenantId, payload);
      message.success('拦截工单已创建');
      setCreateOpen(false);
      createForm.resetFields();
      await refetch();
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setCreating(false);
    }
  };

  const handleRetry = async (record: AfterSalesIntercept) => {
    try {
      await retryAfterSalesIntercept(tenantId, record.intercept_id);
      message.success('已重新发起拦截');
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '重试失败');
    }
  };

  const handleEscalate = async (record: AfterSalesIntercept) => {
    try {
      await escalateAfterSalesIntercept(tenantId, record.intercept_id);
      message.success('已升级处理');
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '升级失败');
    }
  };

  const columns = useMemo<ColumnsType<AfterSalesIntercept>>(
    () => [
      { title: '拦截编号', dataIndex: 'intercept_id' },
      { title: '订单号', dataIndex: 'order_id' },
      { title: '售后单号', dataIndex: 'after_sales_id', render: (value) => value || '-' },
      { title: '承运商', dataIndex: 'carrier_name', render: (value) => value || '-' },
      { title: '运单号', dataIndex: 'tracking_no', render: (value) => value || '-' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: AfterSalesInterceptStatus) => (
          <Tag color={statusColors[value]}>{statusOptions.find((item) => item.value === value)?.label ?? value}</Tag>
        )
      },
      { title: '原因', dataIndex: 'reason', render: (value) => value || '-' },
      { title: '申请时间', dataIndex: 'requested_at', render: (value) => formatDateTime(value) },
      { title: '完成时间', dataIndex: 'completed_at', render: (value) => formatDateTime(value) },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          if (!canWrite) {
            return null;
          }
          const canRetry = record.status === 'FAILED';
          const canEscalate = record.status === 'PROCESSING' || record.status === 'FAILED';
          return (
            <Space>
              {canRetry ? (
                <Button size="small" type="link" onClick={() => handleRetry(record)}>
                  重试
                </Button>
              ) : null}
              {canEscalate ? (
                <Button size="small" type="link" onClick={() => handleEscalate(record)}>
                  升级
                </Button>
              ) : null}
            </Space>
          );
        }
      }
    ],
    [canWrite]
  );

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }} data-testid="after-sales-intercepts-title">
      售后拦截
    </Typography.Title>
  );

  const toolbarRight = (
    <Space>
      <Button onClick={() => refetch()}>刷新</Button>
      {canWrite ? (
        <Button
          type="primary"
          data-testid="after-sales-intercepts-create"
          onClick={() => setCreateOpen(true)}
        >
          发起拦截
        </Button>
      ) : null}
    </Space>
  );

  const header = (
    <Form form={form} layout="vertical" onFinish={handleSearch}>
      <Row gutter={16}>
        <Col xs={24} md={6}>
          <Form.Item label="订单号" name="order_id">
            <Input allowClear placeholder="输入订单号" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item label="运单号" name="tracking_no">
            <Input allowClear placeholder="输入运单号" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item label="状态" name="status">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item label="申请时间" name="requested_range">
            <DatePicker.RangePicker
              allowClear
              showTime
              style={{ width: '100%' }}
              placeholder={['开始时间', '结束时间']}
            />
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
  );

  return (
    <>
      <div data-testid="after-sales-intercepts-table">
        <UiModule03MainTableSection<AfterSalesIntercept>
          toolbarLeft={toolbarLeft}
          toolbarRight={toolbarRight}
          header={header}
          rowKey="intercept_id"
          columns={columns}
          data={data?.items ?? []}
          loading={isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="拦截列表加载失败"
          emptyText="暂无拦截记录"
          total={data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          dataTestId="after-sales-intercepts-main-grid"
          gridPreset="industrial-review-compact"
          selectionCellMode="custom"
          selectionHeaderMode="allInvert"
          showColumnSettings
          columnSettingsKey="after-sales.intercepts.columns"
        />
      </div>
      <Modal
        title="发起拦截"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={handleCreate}
        okText="提交"
        okButtonProps={{ 'data-testid': 'after-sales-intercepts-submit' }}
        confirmLoading={creating}
      >
        <Form form={createForm} layout="vertical" initialValues={{ status: 'PROCESSING' }}>
          <Form.Item
            label="订单号"
            name="order_id"
            rules={[{ required: true, message: '请填写订单号' }]}
          >
            <Input data-testid="after-sales-intercepts-order" placeholder="订单号" />
          </Form.Item>
          <Form.Item label="售后单号" name="after_sales_id">
            <Input placeholder="售后单号" />
          </Form.Item>
          <Form.Item label="承运商" name="carrier_name">
            <Input placeholder="承运商名称" />
          </Form.Item>
          <Form.Item label="运单号" name="tracking_no">
            <Input placeholder="运单号" />
          </Form.Item>
          <Form.Item label="状态" name="status">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item label="原因" name="reason">
            <Input.TextArea rows={2} placeholder="拦截原因" />
          </Form.Item>
          <Form.Item label="申请时间" name="requested_at">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
