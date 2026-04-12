import { Alert, DatePicker, Descriptions, InputNumber, Space, Table, Tag } from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { components } from '@/api/generated/schema';
import {
  getDashboardReport,
  getFinanceReport,
  getFulfillmentReport,
  getInventoryReport
} from '@/api/reports';
import { ServerTable } from '@/components/ServerTable';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';

const formatCent = (value: number) => (value / 100).toFixed(2);

type DashboardMetric = components['schemas']['DashboardMetric'];
type InventoryTurnoverItem = components['schemas']['InventoryTurnoverItem'];

const metricColumns: ColumnsType<DashboardMetric> = [
  { title: '分组', dataIndex: 'group_key' },
  { title: '订单量', dataIndex: 'order_count' },
  {
    title: 'GMV(元)',
    dataIndex: 'gmv_cent',
    render: (value: number) => formatCent(value)
  },
  {
    title: '退款额(元)',
    dataIndex: 'refund_cent',
    render: (value: number) => formatCent(value)
  }
];

export const ReportsPage = () => {
  const [range, setRange] = useState<[Dayjs, Dayjs]>([
    dayjs().subtract(7, 'day'),
    dayjs()
  ]);
  const [warningThreshold, setWarningThreshold] = useState<number>(5);
  const [inventoryPagination, setInventoryPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const baseQuery = useMemo(
    () => ({
      start_at: range?.[0]?.toISOString(),
      end_at: range?.[1]?.toISOString()
    }),
    [range]
  );

  const inventoryQuery = useMemo(
    () => ({
      ...baseQuery,
      warning_threshold: warningThreshold,
      limit: inventoryPagination.pageSize ?? 20,
      offset: ((inventoryPagination.current ?? 1) - 1) * (inventoryPagination.pageSize ?? 20)
    }),
    [baseQuery, warningThreshold, inventoryPagination]
  );

  const dashboardQuery = useQuery({
    queryKey: ['reports', 'dashboard', baseQuery],
    queryFn: () => getDashboardReport(baseQuery),
    keepPreviousData: true
  });

  const fulfillmentQuery = useQuery({
    queryKey: ['reports', 'fulfillment', baseQuery],
    queryFn: () => getFulfillmentReport(baseQuery),
    keepPreviousData: true
  });

  const inventoryQueryResult = useQuery({
    queryKey: ['reports', 'inventory', inventoryQuery],
    queryFn: () => getInventoryReport(inventoryQuery),
    keepPreviousData: true
  });

  const financeQuery = useQuery({
    queryKey: ['reports', 'finance', baseQuery],
    queryFn: () => getFinanceReport(baseQuery),
    keepPreviousData: true
  });

  const inventoryColumns: ColumnsType<InventoryTurnoverItem> = useMemo(
    () => [
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: '仓库', dataIndex: 'warehouse_id' },
      { title: '现货', dataIndex: 'on_hand' },
      { title: '锁定', dataIndex: 'reserved' },
      { title: '可用', dataIndex: 'available' },
      { title: '销量', dataIndex: 'sold_qty' },
      {
        title: '周转天数',
        dataIndex: 'turnover_days',
        render: (value?: number | null) =>
          value != null ? value.toFixed(1) : '-'
      },
      {
        title: '预警',
        dataIndex: 'warning',
        render: (_, record) =>
          record.out_of_stock ? (
            <Tag color="red">缺货</Tag>
          ) : record.warning ? (
            <Tag color="orange">预警</Tag>
          ) : (
            <Tag color="green">正常</Tag>
          )
      }
    ],
    []
  );

  return (
    <UiModule07SettingsLayout title="经营报表">
      <UiModule07SettingsSection title="查询条件">
        <Space wrap>
          <DatePicker.RangePicker
            allowClear={false}
            value={range}
            onChange={(value) => {
              if (value && value[0] && value[1]) {
                setRange(value as [Dayjs, Dayjs]);
                setInventoryPagination((prev) => ({ ...prev, current: 1 }));
              }
            }}
          />
          <span>库存预警阈值</span>
          <InputNumber
            min={0}
            value={warningThreshold}
            onChange={(value) => {
              setWarningThreshold(typeof value === 'number' ? value : 0);
              setInventoryPagination((prev) => ({ ...prev, current: 1 }));
            }}
          />
        </Space>
      </UiModule07SettingsSection>

      <UiModule07SettingsSection title="经营概览">
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Table
            size="small"
            rowKey="group_key"
            columns={metricColumns}
            dataSource={dashboardQuery.data?.by_shop ?? []}
            pagination={false}
            title={() => '按店铺'}
          />
          <Table
            size="small"
            rowKey="group_key"
            columns={metricColumns}
            dataSource={dashboardQuery.data?.by_platform ?? []}
            pagination={false}
            title={() => '按平台'}
          />
          <Table
            size="small"
            rowKey="group_key"
            columns={metricColumns}
            dataSource={dashboardQuery.data?.by_date ?? []}
            pagination={false}
            title={() => '按日期'}
          />
        </Space>
      </UiModule07SettingsSection>

      <UiModule07SettingsSection title="履约效率">
        {fulfillmentQuery.data ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Alert
              type="info"
              showIcon
              message={`超时未发货订单：${fulfillmentQuery.data.overdue_count}`}
            />
            <Table
              size="small"
              rowKey="bucket"
              columns={[
                { title: '区间', dataIndex: 'bucket' },
                { title: '订单量', dataIndex: 'order_count' }
              ]}
              dataSource={fulfillmentQuery.data.buckets}
              pagination={false}
            />
          </Space>
        ) : null}
      </UiModule07SettingsSection>

      <UiModule07SettingsSection title="库存周转">
        <ServerTable<InventoryTurnoverItem>
          rowKey="sku_id"
          columns={inventoryColumns}
          data={inventoryQueryResult.data?.items ?? []}
          loading={inventoryQueryResult.isLoading || inventoryQueryResult.isFetching}
          error={inventoryQueryResult.error}
          onRetry={() => inventoryQueryResult.refetch()}
          total={inventoryQueryResult.data?.total ?? 0}
          pagination={inventoryPagination}
          onPaginationChange={setInventoryPagination}
          columnSettingsKey="reports.inventory.columns"
          emptyText="暂无库存数据"
        />
      </UiModule07SettingsSection>

      <UiModule07SettingsSection title="财务差异">
        {financeQuery.data ? (
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label="订单量">
              {financeQuery.data.order_count}
            </Descriptions.Item>
            <Descriptions.Item label="GMV(元)">
              {formatCent(financeQuery.data.gmv_cent)}
            </Descriptions.Item>
            <Descriptions.Item label="明细金额(元)">
              {formatCent(financeQuery.data.items_amount_cent)}
            </Descriptions.Item>
            <Descriptions.Item label="退款额(元)">
              {formatCent(financeQuery.data.refund_cent)}
            </Descriptions.Item>
            <Descriptions.Item label="差异(元)">
              {formatCent(financeQuery.data.discrepancy_cent)}
            </Descriptions.Item>
          </Descriptions>
        ) : null}
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};
