import { Button, Card, Form, Select, Space, Switch, Table, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type { AfterSalesItem } from '@/api/afterSalesOps';
import { getAfterSalesDetail, receiveAfterSales, redoAfterSalesReceive } from '@/api/afterSalesOps';
import { listWarehouses } from '@/api/warehouses';
import { UiModule07SettingsLayout } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

export const AfterSalesReceivePage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const { after_sales_id } = useParams<{ after_sales_id: string }>();
  const [form] = Form.useForm<{ warehouse_id: string; putaway_done: boolean }>();

  const { data, isLoading } = useQuery({
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
      <UiModule07SettingsLayout>
        {titleNode}
        <Typography.Text>加载中...</Typography.Text>
      </UiModule07SettingsLayout>
    );
  }

  if (!data) {
    return (
      <UiModule07SettingsLayout>
        {titleNode}
        <Typography.Text type="danger">售后记录不存在</Typography.Text>
      </UiModule07SettingsLayout>
    );
  }

  return (
    <UiModule07SettingsLayout>
      {titleNode}
      <Card>
        <Form form={form} layout="vertical" initialValues={{ putaway_done: false }}>
          <Form.Item
            name="warehouse_id"
            label="收货仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select
              data-testid="after-sales-receive-warehouse"
              options={(warehouses?.items ?? []).map((wh) => ({
                label: wh.name,
                value: wh.warehouse_id
              }))}
              placeholder="选择仓库"
            />
          </Form.Item>
          <Form.Item name="putaway_done" label="是否立即上架" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Space>
            <Button
              type="primary"
              data-testid="after-sales-receive-submit"
              onClick={handleSubmit}
            >
              {data?.return_receipt?.status === 'CANCELLED' ? '重做收货' : '确认收货'}
            </Button>
            <Button onClick={() => navigate(`/after-sales/${data.after_sales.id}`)}>返回</Button>
          </Space>
        </Form>
      </Card>
      <Card title="退货明细">
        <Table rowKey="sku_id" columns={columns} dataSource={data.items} pagination={false} />
      </Card>
    </UiModule07SettingsLayout>
  );
};
