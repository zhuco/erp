import { Button, Col, DatePicker, Form, Input, Row, Select, Space, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type { AfterSaleCaseSummary, AfterSaleListQuery } from '@/api/afterSales';
import { listAfterSales } from '@/api/afterSales';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  platform?: PlatformCode;
  shop_id?: string;
  case_status?: string;
  platform_order_id?: string;
  updated_range?: [Dayjs, Dayjs];
};

const formatDateTime = (value?: string) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveCaseStatus = (record: AfterSaleCaseSummary) =>
  record.case_status ?? record.after_sale_state ?? record.refund_state ?? '-';

const resolveCaseType = (record: AfterSaleCaseSummary) =>
  record.case_type ?? record.after_sale_type ?? '-';

const resolveRefundAmount = (record: AfterSaleCaseSummary) => {
  if (record.refund_amount_cent != null) {
    return record.refund_amount_cent / 100;
  }
  if (record.refund_amount != null) {
    return record.refund_amount;
  }
  return null;
};

const resolveCaseId = (record: AfterSaleCaseSummary) =>
  record.case_id ?? record.after_sale_id ?? record.platform_case_id ?? record.id ?? '';

const resolvePlatformCaseId = (record: AfterSaleCaseSummary) =>
  record.platform_case_id ?? record.after_sale_id ?? record.case_id ?? record.id ?? '-';

const resolveRelatedOrder = (record: AfterSaleCaseSummary) => {
  const orderId = record.platform_order_id ?? '-';
  const subOrderId =
    (record as AfterSaleCaseSummary & { platform_order_item_id?: string }).platform_order_item_id ??
    '-';
  return (
    <Space direction="vertical" size={0}>
      <span>订单: {orderId}</span>
      <span>子单: {subOrderId}</span>
    </Space>
  );
};

export const AfterSaleCaseListPage = () => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<IndustrialTablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const query = useMemo<AfterSaleListQuery>(() => {
    const range = filters.updated_range;
    return {
      platform: filters.platform,
      shop_id: filters.shop_id?.trim() || undefined,
      case_status: filters.case_status?.trim() || undefined,
      platform_order_id: filters.platform_order_id?.trim() || undefined,
      updated_from: range?.[0]?.toISOString(),
      updated_to: range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sale-cases', tenantId, query],
    queryFn: () => listAfterSales(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const columns = useMemo<ColumnsType<AfterSaleCaseSummary>>(
    () => [
      {
        title: '平台售后单号',
        dataIndex: 'platform_case_id',
        render: (_, record) => {
          const caseId = resolveCaseId(record);
          return (
            <Link to={`/after-sale-cases/${caseId}`} data-testid="after-sale-case-link">
              {resolvePlatformCaseId(record)}
            </Link>
          );
        }
      },
      {
        title: '平台',
        dataIndex: 'platform',
        render: (_, record) => <PlatformTag platform={record.platform} />
      },
      {
        title: '售后类型',
        dataIndex: 'case_type',
        render: (_, record) => resolveCaseType(record)
      },
      {
        title: '售后状态',
        dataIndex: 'case_status',
        render: (_, record) => resolveCaseStatus(record)
      },
      {
        title: '退款金额',
        dataIndex: 'refund_amount_cent',
        render: (_, record) => {
          const amount = resolveRefundAmount(record);
          return amount == null ? '-' : `¥${amount.toFixed(2)}`;
        }
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '关联订单/子单',
        dataIndex: 'platform_order_id',
        render: (_, record) => resolveRelatedOrder(record)
      }
    ],
    []
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

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        售后 Case 列表
      </Typography.Title>
      <Typography.Text type="secondary">
        汇总平台售后 Case，支持按订单、状态与更新时间快速定位。
      </Typography.Text>
    </Space>
  );

  const toolbarRight = (
    <Button onClick={() => void refetch()} loading={isFetching}>
      刷新
    </Button>
  );

  const header = (
    <Form form={form} layout="vertical" onFinish={handleSearch}>
      <Row gutter={16}>
        <Col xs={24} md={6}>
          <Form.Item name="platform" label="平台">
            <Select allowClear options={platformOptions} placeholder="选择平台" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="shop_id" label="店铺">
            <Input allowClear placeholder="输入店铺 ID" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="case_status" label="售后状态">
            <Input allowClear placeholder="输入售后状态" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="platform_order_id" label="平台订单号">
            <Input allowClear placeholder="输入平台订单号" />
          </Form.Item>
        </Col>
        <Col xs={24} md={12}>
          <Form.Item name="updated_range" label="更新时间">
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
    <UiModule03MainTableSection<AfterSaleCaseSummary>
      toolbarLeft={toolbarLeft}
      toolbarRight={toolbarRight}
      header={header}
      rowKey={resolveCaseId}
      columns={columns}
      data={data?.items ?? []}
      loading={isLoading || isFetching}
      error={error}
      onRetry={() => refetch()}
      errorTitle="售后列表加载失败"
      emptyText="暂无售后记录"
      total={data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      dataTestId="after-sale-case-list-main-grid"
      gridPreset="industrial-review-compact"
      selectionCellMode="custom"
      selectionHeaderMode="allInvert"
      showColumnSettings
      columnSettingsKey="after-sales.table.columns"
    />
  );
};
