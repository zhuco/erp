import {
  Button,
  Form,
  Input,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { applyInvoice, listInvoices, type Invoice } from '@/api/finance';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type InvoiceFormValues = {
  order_id: string;
  invoice_title: string;
  tax_no?: string;
};

type InvoiceFilters = {
  status?: string;
  order_id?: string;
};

const statusOptions = [
  { label: 'APPLIED', value: 'APPLIED' },
  { label: 'ISSUED', value: 'ISSUED' },
  { label: 'FAILED', value: 'FAILED' }
];

const statusTag = (status: string) => {
  if (status === 'ISSUED') {
    return <Tag color="green">ISSUED</Tag>;
  }
  if (status === 'FAILED') {
    return <Tag color="red">FAILED</Tag>;
  }
  return <Tag color="blue">APPLIED</Tag>;
};

export const InvoicesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('finance.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<InvoiceFormValues>();
  const [filters, setFilters] = useState<InvoiceFilters>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [submitting, setSubmitting] = useState(false);

  const query = useMemo(() => {
    const offset = ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 10);
    return {
      status: filters.status,
      order_id: filters.order_id,
      offset,
      limit: pagination.pageSize ?? 10
    };
  }, [filters.order_id, filters.status, pagination.current, pagination.pageSize]);

  const listQuery = useQuery({
    queryKey: ['invoices', tenantId, query],
    queryFn: () => listInvoices(tenantId, query),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData
  });

  const handleApply = async (values: InvoiceFormValues) => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    setSubmitting(true);
    try {
      await applyInvoice(tenantId, values);
      message.success('开票申请已提交');
      form.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['invoices', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '开票申请失败');
    } finally {
      setSubmitting(false);
    }
  };

  const columns = useMemo<ColumnsType<Invoice>>(
    () => [
      { title: '订单ID', dataIndex: 'order_id', ellipsis: true },
      { title: '抬头', dataIndex: 'invoice_title' },
      { title: '税号', dataIndex: 'tax_no', render: (_, record) => record.tax_no ?? '-' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => statusTag(record.status)
      },
      { title: '申请时间', dataIndex: 'created_at' }
    ],
    []
  );

  const invoiceTableTitle = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={4} style={{ margin: 0 }}>
        发票管理
      </Typography.Title>
      <Typography.Text type="secondary">管理开票申请、开票状态与订单发票记录</Typography.Text>
    </Space>
  );

  const invoiceTableActions = (
    <Space wrap>
      <Button onClick={() => listQuery.refetch()} loading={listQuery.isFetching}>
        刷新
      </Button>
      <Button
        type="primary"
        htmlType="submit"
        form="finance-invoice-apply-form"
        loading={submitting}
        disabled={!canWrite}
      >
        开票申请
      </Button>
    </Space>
  );

  const invoiceFilterBar = (
    <Space wrap>
      <Select
        allowClear
        placeholder="状态"
        style={{ width: 160 }}
        options={statusOptions}
        onChange={(value) => {
          setFilters((prev) => ({ ...prev, status: value || undefined }));
          setPagination((prev) => ({ ...prev, current: 1 }));
        }}
      />
      <Input
        placeholder="订单ID"
        style={{ width: 220 }}
        value={filters.order_id}
        onChange={(event) =>
          setFilters((prev) => ({ ...prev, order_id: event.target.value || undefined }))
        }
      />
    </Space>
  );

  const header = (
      <Form id="finance-invoice-apply-form" form={form} layout="inline" onFinish={handleApply}>
        <Form.Item name="order_id" label="订单ID" rules={[{ required: true }]}>
          <Input style={{ width: 220 }} />
        </Form.Item>
        <Form.Item name="invoice_title" label="抬头" rules={[{ required: true }]}>
          <Input style={{ width: 220 }} />
        </Form.Item>
        <Form.Item name="tax_no" label="税号">
          <Input style={{ width: 220 }} />
        </Form.Item>
      </Form>
  );

  return (
    <UiModule03MainTableSection<Invoice>
      dataTestId="finance-invoices-main-grid"
      gridPreset="industrial-review-compact"
      selectionCellMode="custom"
      selectionHeaderMode="allInvert"
      toolbarLeft={invoiceTableTitle}
      toolbarRight={invoiceTableActions}
      header={header}
      actionBar={invoiceFilterBar}
      rowKey="invoice_id"
      columns={columns}
      data={listQuery.data?.items ?? []}
      loading={listQuery.isFetching}
      error={listQuery.error}
      onRetry={() => listQuery.refetch()}
      errorTitle="开票列表加载失败"
      emptyText="暂无开票记录"
      total={listQuery.data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      columnSettingsKey="finance.invoices.table.columns"
      showColumnSettings
    />
  );
};
