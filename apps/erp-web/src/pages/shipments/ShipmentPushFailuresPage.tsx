import {
  Button,
  Card,
  Col,
  Form,
  Input,
  Select,
  Modal,
  Row,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  ShipmentPushAttempt,
  ShipmentPushAttemptListQuery,
  ShipmentPushAttemptStatus
} from '@/api/shipmentPushAttempts';
import { listShipmentPushAttempts } from '@/api/shipmentPushAttempts';
import { replayDeadLetter } from '@/api/failureQueue';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type PushAttemptFilters = {
  status?: ShipmentPushAttemptStatus;
  platform_order_id?: string;
};

const statusOptions = [
  { label: 'RETRYING', value: 'RETRYING' },
  { label: 'DEAD', value: 'DEAD' },
  { label: 'FAILED', value: 'FAILED' },
  { label: 'SUCCESS', value: 'SUCCESS' }
];

const statusColors: Record<ShipmentPushAttemptStatus, string> = {
  RETRYING: 'orange',
  DEAD: 'volcano',
  FAILED: 'red',
  SUCCESS: 'green'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveTotal = (
  pagination: TablePaginationConfig,
  nextCursor?: string | null
): number => {
  const current = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  return nextCursor ? (current + 1) * pageSize : current * pageSize;
};

export const ShipmentPushFailuresPage = () => {
  const tenantId = useTenantId();
  const canReplay = useCan('dlq.replay');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<PushAttemptFilters>();
  const defaultFilters: PushAttemptFilters = { status: 'DEAD' };
  const [filters, setFilters] = useState<PushAttemptFilters>(defaultFilters);
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [cursorMap, setCursorMap] = useState<Record<number, string | undefined>>({
    1: undefined
  });
  const [detailAttempt, setDetailAttempt] = useState<ShipmentPushAttempt | null>(null);

  const query = useMemo<ShipmentPushAttemptListQuery>(
    () => ({
      status: filters.status,
      platform_order_id: filters.platform_order_id,
      cursor: cursorMap[pagination.current ?? 1],
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize, cursorMap]
  );

  const attemptsQuery = useQuery({
    queryKey: ['shipment-push-attempts', tenantId, query],
    queryFn: () => listShipmentPushAttempts(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const handleSearch = (values: PushAttemptFilters) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
    setCursorMap({ 1: undefined });
  };

  const handleReset = () => {
    form.resetFields();
    setFilters(defaultFilters);
    setPagination((prev) => ({ ...prev, current: 1 }));
    setCursorMap({ 1: undefined });
  };

  const handleReplay = async (deadLetterId: string) => {
    try {
      await replayDeadLetter(tenantId, deadLetterId);
      message.success('已重放');
      await queryClient.invalidateQueries({ queryKey: ['shipment-push-attempts', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '重放失败');
    }
  };

  const columns = useMemo<ColumnsType<ShipmentPushAttempt>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => <Tag color={statusColors[record.status]}>{record.status}</Tag>
      },
      {
        title: '平台订单号',
        dataIndex: 'platform_order_id'
      },
      {
        title: '承运商',
        dataIndex: 'carrier_code',
        render: (_, record) => record.carrier_code ?? '-'
      },
      {
        title: '运单号',
        dataIndex: 'tracking_no',
        render: (_, record) => record.tracking_no ?? '-'
      },
      {
        title: '错误码',
        dataIndex: 'error_code',
        render: (_, record) => record.error_code || '-'
      },
      {
        title: '错误原因',
        dataIndex: 'error_message',
        ellipsis: true,
        render: (_, record) => record.error_message || '-'
      },
      {
        title: '尝试次数',
        dataIndex: 'attempt'
      },
      {
        title: '耗时(ms)',
        dataIndex: 'duration_ms'
      },
      {
        title: '下次重试',
        dataIndex: 'next_retry_at',
        render: (_, record) => formatDateTime(record.next_retry_at)
      },
      {
        title: '记录时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" onClick={() => setDetailAttempt(record)}>
              详情
            </Button>
            {record.dead_letter_id && record.status === 'DEAD' && canReplay ? (
              <Button size="small" onClick={() => handleReplay(record.dead_letter_id!)}>
                重放
              </Button>
            ) : (
              <span>-</span>
            )}
          </Space>
        )
      }
    ],
    [canReplay]
  );

  const attempts = attemptsQuery.data?.data ?? [];

  if (attemptsQuery.error && attempts.length === 0) {
    return (
      <ErrorState
        title="发货回传失败加载失败"
        error={attemptsQuery.error}
        onRetry={() => attemptsQuery.refetch()}
      />
    );
  }

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      发货回传失败
    </Typography.Title>
  );

  const header = (
    <Card>
      <Form form={form} layout="vertical" onFinish={handleSearch} initialValues={defaultFilters}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="platform_order_id" label="平台订单号">
              <Input allowClear placeholder="请输入平台订单号" />
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
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <UiModule03MainTableSection<ShipmentPushAttempt>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="attempt_id"
        columns={columns}
        data={attempts}
        loading={attemptsQuery.isLoading || attemptsQuery.isFetching}
        error={attemptsQuery.error}
        onRetry={() => attemptsQuery.refetch()}
        errorTitle="发货回传失败加载失败"
        emptyText="暂无发货回传记录"
        total={resolveTotal(pagination, attemptsQuery.data?.next_cursor)}
        pagination={pagination}
        onPaginationChange={(nextPagination) => {
          const nextPageSize = nextPagination.pageSize ?? pagination.pageSize;
          if (nextPageSize !== pagination.pageSize) {
            setCursorMap({ 1: undefined });
            setPagination({ current: 1, pageSize: nextPageSize });
            return;
          }
          setPagination((prev) => ({
            ...prev,
            current: nextPagination.current,
            pageSize: nextPageSize
          }));
          if (nextPagination.current && attemptsQuery.data?.next_cursor) {
            setCursorMap((prev) => ({
              ...prev,
              [nextPagination.current + 1]: attemptsQuery.data?.next_cursor ?? undefined
            }));
          }
        }}
        columnSettingsKey="shipment-push.failures.columns"
      />
      <Modal
        open={!!detailAttempt}
        onCancel={() => setDetailAttempt(null)}
        footer={null}
        title="发货回传详情"
        width={720}
      >
        {detailAttempt && (
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            <Typography.Text strong>请求摘要</Typography.Text>
            <pre>{JSON.stringify(detailAttempt.request_json, null, 2)}</pre>
            <Typography.Text strong>响应摘要</Typography.Text>
            <pre>{JSON.stringify(detailAttempt.response_json, null, 2)}</pre>
          </Space>
        )}
      </Modal>
    </Space>
  );
};
