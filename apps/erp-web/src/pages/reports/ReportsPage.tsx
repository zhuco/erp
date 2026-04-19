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
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';

const formatCent = (value: number) => (value / 100).toFixed(2);

type DashboardMetric = components['schemas']['DashboardMetric'];
type InventoryTurnoverItem = components['schemas']['InventoryTurnoverItem'];

const metricColumns: ColumnsType<DashboardMetric> = [
  { title: '鍒嗙粍', dataIndex: 'group_key' },
  { title: 'Order Count', dataIndex: 'order_count' },
  {
    title: 'GMV(鍏?',
    dataIndex: 'gmv_cent',
    render: (value: number) => formatCent(value)
  },
  {
    title: '閫€娆鹃(鍏?',
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
    placeholderData: (previousData) => previousData
  });

  const fulfillmentQuery = useQuery({
    queryKey: ['reports', 'fulfillment', baseQuery],
    queryFn: () => getFulfillmentReport(baseQuery),
    placeholderData: (previousData) => previousData
  });

  const inventoryQueryResult = useQuery({
    queryKey: ['reports', 'inventory', inventoryQuery],
    queryFn: () => getInventoryReport(inventoryQuery),
    placeholderData: (previousData) => previousData
  });

  const financeQuery = useQuery({
    queryKey: ['reports', 'finance', baseQuery],
    queryFn: () => getFinanceReport(baseQuery),
    placeholderData: (previousData) => previousData
  });

  const inventoryColumns: ColumnsType<InventoryTurnoverItem> = useMemo(
    () => [
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: '浠撳簱', dataIndex: 'warehouse_id' },
      { title: '鐜拌揣', dataIndex: 'on_hand' },
      { title: '閿佸畾', dataIndex: 'reserved' },
      { title: '鍙敤', dataIndex: 'available' },
      { title: 'Sold Qty', dataIndex: 'sold_qty' },
      {
        title: '鍛ㄨ浆澶╂暟',
        dataIndex: 'turnover_days',
        render: (value?: number | null) =>
          value != null ? value.toFixed(1) : '-'
      },
      {
        title: '棰勮',
        dataIndex: 'warning',
        render: (_, record) =>
          record.out_of_stock ? (
            <Tag color="red">缂鸿揣</Tag>
          ) : record.warning ? (
            <Tag color="orange">棰勮</Tag>
          ) : (
            <Tag color="green">姝ｅ父</Tag>
          )
      }
    ],
    []
  );

  return (
    <UiModule07SettingsLayout title="缁忚惀鎶ヨ〃">
      <UiModule07SettingsSection title="鏌ヨ鏉′欢">
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
          <span>Inventory warning threshold</span>
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

      <UiModule07SettingsSection title="缁忚惀姒傝">
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Table
            size="small"
            rowKey="group_key"
            columns={metricColumns}
            dataSource={dashboardQuery.data?.by_shop ?? []}
            pagination={false}
            title={() => 'By Shop'}
          />
          <Table
            size="small"
            rowKey="group_key"
            columns={metricColumns}
            dataSource={dashboardQuery.data?.by_platform ?? []}
            pagination={false}
            title={() => 'By Platform'}
          />
          <Table
            size="small"
            rowKey="group_key"
            columns={metricColumns}
            dataSource={dashboardQuery.data?.by_date ?? []}
            pagination={false}
            title={() => 'By Date'}
          />
        </Space>
      </UiModule07SettingsSection>

      <UiModule07SettingsSection title="灞ョ害鏁堢巼">
        {fulfillmentQuery.data ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Alert
              type="info"
              showIcon
              message={`瓒呮椂鏈彂璐ц鍗曪細${fulfillmentQuery.data.overdue_count}`}
            />
            <Table
              size="small"
              rowKey="bucket"
              columns={[
                { title: '鍖洪棿', dataIndex: 'bucket' },
                { title: 'Order Count', dataIndex: 'order_count' }
              ]}
              dataSource={fulfillmentQuery.data.buckets}
              pagination={false}
            />
          </Space>
        ) : null}
      </UiModule07SettingsSection>

      <UiModule07SettingsSection title="搴撳瓨鍛ㄨ浆">
        <UiModule03MainTableSection<InventoryTurnoverItem>
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

      <UiModule07SettingsSection title="璐㈠姟宸紓">
        {financeQuery.data ? (
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label="Order Count">
              {financeQuery.data.order_count}
            </Descriptions.Item>
            <Descriptions.Item label="GMV(鍏?">
              {formatCent(financeQuery.data.gmv_cent)}
            </Descriptions.Item>
            <Descriptions.Item label="明细閲戦(鍏?">
              {formatCent(financeQuery.data.items_amount_cent)}
            </Descriptions.Item>
            <Descriptions.Item label="閫€娆鹃(鍏?">
              {formatCent(financeQuery.data.refund_cent)}
            </Descriptions.Item>
            <Descriptions.Item label="宸紓(鍏?">
              {formatCent(financeQuery.data.discrepancy_cent)}
            </Descriptions.Item>
          </Descriptions>
        ) : null}
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};

