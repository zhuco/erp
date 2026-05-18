import { Button, Form, Select, Space, Switch, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type { AfterSalesItem } from '@/api/afterSalesOps';
import { getAfterSalesDetail, receiveAfterSales, redoAfterSalesReceive } from '@/api/afterSalesOps';
import { SettingsPageTemplate } from '@/components/page-templates';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { listWarehouses } from '@/api/warehouses';
import { useTenantId } from '@/features/tenant';
import { buildReceiveModel, NO_WAREHOUSE_HINT } from './afterSalesReceiveModel';

export const AfterSalesReceivePage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const { after_sales_id } = useParams<{ after_sales_id: string }>();
  const [form] = Form.useForm<{ warehouse_id: string; putaway_done: boolean }>();

  const {
    data,
    error,
    isFetching,
    isLoading,
    refetch
  } = useQuery({
    queryKey: ['after-sales-detail', tenantId, after_sales_id],
    queryFn: () => getAfterSalesDetail(tenantId, after_sales_id ?? ''),
    enabled: Boolean(after_sales_id)
  });

  const { data: warehouses } = useQuery({
    queryKey: ['warehouses', tenantId],
    queryFn: () => listWarehouses(tenantId, { limit: 100 }),
    staleTime: 60_000
  });

  const columns = useMemo<ColumnsType<AfterSalesItem>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: '数量', dataIndex: 'qty' },
      {
        title: '退款金额',
        dataIndex: 'refund_cent',
        render: (value: number) => `¥${(value / 100).toFixed(2)}`
      }
    ],
    []
  );

  const receivePagination = useMemo<IndustrialTablePaginationConfig>(
    () => ({
      current: 1,
      pageSize: Math.max(data?.items.length ?? 0, 1),
      total: data?.items.length ?? 0,
      showSizeChanger: false
    }),
    [data?.items.length]
  );

  const model = buildReceiveModel({
    warehouses: (warehouses?.items ?? []).map((warehouse) => ({
      warehouse_id: warehouse.warehouse_id,
      name: warehouse.name,
      is_default: warehouse.is_default
    })),
    returnReceiptStatus: data?.return_receipt?.status
  });

  useEffect(() => {
    if (!model.canSubmit) {
      return;
    }
    const current = form.getFieldValue('warehouse_id');
    if (!current && model.initialWarehouseId) {
      form.setFieldValue('warehouse_id', model.initialWarehouseId);
    }
  }, [form, model.canSubmit, model.initialWarehouseId]);

  const titleNode = (
    <Typography.Title level={3} data-testid="after-sales-receive-title">
      退货收货
    </Typography.Title>
  );

  const handleSubmit = async () => {
    const values = await form.validateFields();
    if (!after_sales_id) {
      return;
    }
    try {
      const handler =
        data?.return_receipt?.status === 'CANCELLED'
          ? redoAfterSalesReceive
          : receiveAfterSales;
      await handler(tenantId, after_sales_id, {
        warehouse_id: values.warehouse_id,
        putaway_done: values.putaway_done
      });
      message.success(data?.return_receipt?.status === 'CANCELLED' ? '重做收货完成' : '收货完成');
      navigate(`/after-sales/${after_sales_id}`);
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  if (!data && isLoading) {
    return (
      <SettingsPageTemplate>
        {titleNode}
        <Typography.Text>加载中...</Typography.Text>
      </SettingsPageTemplate>
    );
  }

  if (!data) {
    return (
      <SettingsPageTemplate>
        {titleNode}
        <Typography.Text type="danger">售后记录不存在</Typography.Text>
      </SettingsPageTemplate>
    );
  }

  const header = (
    <Space direction="vertical" size={4}>
      {titleNode}
      <Typography.Text strong>{model.modeLabel}</Typography.Text>
      <Typography.Paragraph type="secondary" style={{ margin: 0 }}>
        {model.modeDescription}
      </Typography.Paragraph>
      {!model.canSubmit && <Typography.Text type="danger">{NO_WAREHOUSE_HINT}</Typography.Text>}
    </Space>
  );

  const toolbarLeft = (
    <Form form={form} layout="inline" initialValues={{ putaway_done: false }}>
      <Form.Item
        name="warehouse_id"
        label="收货仓库"
        rules={[{ required: true, message: '请选择仓库' }]}
        style={{ marginBottom: 0 }}
      >
        <Select
          data-testid="after-sales-receive-warehouse"
          options={(warehouses?.items ?? []).map((wh) => ({
            label: wh.name,
            value: wh.warehouse_id
          }))}
          placeholder="选择仓库"
          disabled={!model.canSubmit}
          style={{ minWidth: 180 }}
        />
      </Form.Item>
      <Form.Item
        name="putaway_done"
        label="是否立即上架"
        valuePropName="checked"
        style={{ marginBottom: 0 }}
      >
        <Switch />
      </Form.Item>
    </Form>
  );

  const toolbarRight = (
    <Space>
      <Button
        type="primary"
        data-testid="after-sales-receive-submit"
        onClick={handleSubmit}
        disabled={!model.canSubmit}
      >
        {data.return_receipt?.status === 'CANCELLED' ? '重做收货' : '确认收货'}
      </Button>
      <Button onClick={() => navigate(`/after-sales/${data.after_sales.id}`)}>返回</Button>
    </Space>
  );

  return (
    <UiModule03MainTableSection<AfterSalesItem>
      header={header}
      toolbarLeft={toolbarLeft}
      toolbarRight={toolbarRight}
      rowKey="sku_id"
      columns={columns}
      data={data.items}
      loading={isFetching}
      error={error}
      onRetry={() => refetch()}
      errorTitle="退货明细加载失败"
      emptyText="暂无退货明细"
      total={data.items.length}
      pagination={receivePagination}
      onPaginationChange={() => undefined}
      dataTestId="after-sales-receive-main-grid"
      gridPreset="industrial-review-compact"
      selectionCellMode="custom"
      selectionHeaderMode="allInvert"
      showColumnSettings
      columnSettingsKey="after-sales.receive.main-grid"
    />
  );
};
