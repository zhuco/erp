import { Button, DatePicker, Form, Input, Select, Space, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { type Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import { listProfitSnapshots, type ProfitSnapshot } from '@/api/finance';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';
import { shopOptions } from '@/features/tenantOptions';

type ProfitFilters = {
  shop_id?: string;
  sku_id?: string;
  created_range?: [Dayjs, Dayjs];
};

export const ProfitPage = () => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<ProfitFilters>();
  const [filters, setFilters] = useState<ProfitFilters>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });

  const query = useMemo(() => {
    const offset = ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 10);
    const range = filters.created_range;
    return {
      shop_id: filters.shop_id,
      sku_id: filters.sku_id,
      created_from: range?.[0]?.toISOString(),
      created_to: range?.[1]?.toISOString(),
      offset,
      limit: pagination.pageSize ?? 10
    };
  }, [filters.created_range, filters.shop_id, filters.sku_id, pagination.current, pagination.pageSize]);

  const listQuery = useQuery({
    queryKey: ['profitSnapshots', tenantId, query],
    queryFn: () => listProfitSnapshots(tenantId, query),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData
  });

  const columns = useMemo<ColumnsType<ProfitSnapshot>>(
    () => [
      { title: '平台订单号', dataIndex: 'platform_order_id', ellipsis: true },
      { title: '店铺', dataIndex: 'shop_id' },
      {
        title: '下单时间',
        dataIndex: 'order_created_at',
        render: (_, record) => dayjs(record.order_created_at).format('YYYY-MM-DD HH:mm')
      },
      { title: '收入', dataIndex: 'revenue_cent' },
      { title: '成本', dataIndex: 'cost_cent' },
      { title: '平台费', dataIndex: 'platform_fee_cent' },
      { title: '运费', dataIndex: 'shipping_fee_cent' },
      { title: '毛利', dataIndex: 'gross_profit_cent' },
      {
        title: '快照时间',
        dataIndex: 'computed_at',
        render: (_, record) => dayjs(record.computed_at).format('YYYY-MM-DD HH:mm')
      }
    ],
    []
  );

  const handleSearch = (values: ProfitFilters) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        利润分析
      </Typography.Title>
      <Typography.Text type="secondary">按店铺、SKU 与下单时间查看利润快照。</Typography.Text>
    </Space>
  );

  const toolbarRight = (
    <Space size={8}>
      <Button size="small" onClick={() => listQuery.refetch()} loading={listQuery.isFetching}>
        刷新
      </Button>
      <Button size="small" disabled>
        导出
      </Button>
    </Space>
  );

  const header = (
    <Form form={form} layout="inline" onFinish={handleSearch}>
      <Form.Item name="shop_id" label="店铺">
        <Select allowClear style={{ width: 200 }} options={shopOptions} showSearch />
      </Form.Item>
      <Form.Item name="sku_id" label="SKU">
        <Input style={{ width: 200 }} />
      </Form.Item>
      <Form.Item name="created_range" label="下单时间">
        <DatePicker.RangePicker showTime />
      </Form.Item>
      <Form.Item>
        <Space>
          <Button type="primary" htmlType="submit">
            查询
          </Button>
          <Button onClick={handleReset}>重置</Button>
        </Space>
      </Form.Item>
    </Form>
  );

  return (
    <UiModule03MainTableSection<ProfitSnapshot>
      toolbarLeft={toolbarLeft}
      toolbarRight={toolbarRight}
      header={header}
      dataTestId="finance-profit-grid"
      gridPreset="industrial-review-compact"
      selectionCellMode="custom"
      selectionHeaderMode="allInvert"
      rowKey="order_id"
      columns={columns}
      data={listQuery.data?.items ?? []}
      loading={listQuery.isFetching}
      error={listQuery.error}
      onRetry={() => listQuery.refetch()}
      errorTitle="利润快照加载失败"
      emptyText="暂无利润数据"
      total={listQuery.data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      columnSettingsKey="finance.profit.table.columns"
      showColumnSettings
    />
  );
};
