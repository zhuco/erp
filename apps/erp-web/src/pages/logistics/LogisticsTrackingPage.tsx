import { Button, Descriptions, Drawer, Form } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useSearchParams } from 'react-router-dom';

import type {
  LogisticsShipmentListQuery,
  LogisticsShipmentSummary,
  ShipmentTrackingResponse
} from '@/api/logistics';
import { getShipmentTracking, listLogisticsShipments } from '@/api/logistics';
import { ShipmentStatusTag } from '@/components/ShipmentStatusTag';
import { ErrorState, LoadingState } from '@/components/QueryState';
import {
  UiModule01LogisticsQuerySettings,
  UiModule03MainTableSection,
  type UiModule01LogisticsQueryValues
} from '@/components/ui-modules';
import { useWorkbenchTablePagination } from '@/components/page-templates';
import { useTenantId } from '@/features/tenant';

type FilterValues = UiModule01LogisticsQueryValues;

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const LogisticsTrackingPage = () => {
  const tenantId = useTenantId();
  const [searchParams] = useSearchParams();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [prefilled, setPrefilled] = useState(false);
  const { pagination, resetPagination, handlePaginationChange } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions: ['20', '50', '100', '200'],
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeShipment, setActiveShipment] = useState<LogisticsShipmentSummary | null>(null);

  useEffect(() => {
    if (prefilled) {
      return;
    }
    const trackingNo = searchParams.get('tracking_no');
    if (!trackingNo) {
      setPrefilled(true);
      return;
    }
    form.setFieldsValue({ tracking_no: trackingNo });
    setFilters({ tracking_no: trackingNo });
    resetPagination();
    setPrefilled(true);
  }, [form, prefilled, searchParams]);

  const query = useMemo<LogisticsShipmentListQuery>(
    () => ({
      tracking_no: filters.tracking_no?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['logistics-shipments', tenantId, query],
    queryFn: () => listLogisticsShipments(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const {
    data: tracking,
    isLoading: trackingLoading,
    isFetching: trackingFetching,
    error: trackingError,
    refetch: trackingRefetch
  } = useQuery<ShipmentTrackingResponse>({
    queryKey: ['shipment-tracking', tenantId, activeShipment?.shipment_id],
    queryFn: () => getShipmentTracking(tenantId, activeShipment?.shipment_id ?? ''),
    enabled: Boolean(activeShipment)
  });

  const columns = useMemo<ColumnsType<LogisticsShipmentSummary>>(
    () => [
      {
        title: '运单号',
        dataIndex: 'tracking_no',
        render: (_, record) => record.tracking_no ?? '-'
      },
      {
        title: '承运商',
        dataIndex: 'carrier_code',
        render: (_, record) => record.carrier_code ?? '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => <ShipmentStatusTag status={record.status} />
      },
      {
        title: '重量(克)',
        dataIndex: 'weight_gram',
        render: (_, record) => record.weight_gram ?? '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button
            size="small"
            onClick={() => {
              setActiveShipment(record);
              setDetailOpen(true);
            }}
          >
            查看
          </Button>
        )
      }
    ],
    []
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    resetPagination();
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    resetPagination();
  };

  const header = (
    <UiModule01LogisticsQuerySettings
      form={form}
      onSearch={handleSearch}
      onReset={handleReset}
    />
  );
  const footerSummary = `共 ${data?.total ?? 0} 条物流记录`;

  return (
    <>
      <UiModule03MainTableSection<LogisticsShipmentSummary>
        header={header}
        rowKey="shipment_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="物流查询失败"
        emptyText="暂无物流记录"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        footerSummary={footerSummary}
        columnSettingsKey="logistics.table.columns"
        enablePanels={false}
      />
      <Drawer
        title="物流详情"
        open={detailOpen}
        onClose={() => {
          setDetailOpen(false);
          setActiveShipment(null);
        }}
        width={520}
        destroyOnHidden
      >
        {trackingLoading || trackingFetching ? (
          <LoadingState
            loading={trackingLoading || trackingFetching}
            onRetry={() => trackingRefetch()}
          />
        ) : trackingError ? (
          <ErrorState
            title="物流轨迹加载失败"
            error={trackingError}
            onRetry={() => trackingRefetch()}
          />
        ) : tracking ? (
          <Descriptions column={1} bordered size="small">
            <Descriptions.Item label="运单号">{tracking.tracking_no ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="承运商">{tracking.carrier_code ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="最新状态">{tracking.latest_status}</Descriptions.Item>
            <Descriptions.Item label="状态描述">
              {tracking.latest_description ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="更新时间">
              {formatDateTime(tracking.latest_at)}
            </Descriptions.Item>
          </Descriptions>
        ) : null}
      </Drawer>
    </>
  );
};

