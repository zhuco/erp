import {
  Button,
  Card,
  Col,
  Form,
  Input,
  Row,
  Select,
  Space,
  TablePaginationConfig,
  Tabs,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  DeadLetter,
  DeadLetterListQuery,
  DeadLetterStatus,
  OutboxEvent,
  OutboxEventListQuery,
  OutboxStatus
} from '@/api/failureQueue';
import {
  listDeadLetters,
  listOutboxEvents,
  replayDeadLetter,
  requeueOutboxEvent
} from '@/api/failureQueue';
import { ServerTable } from '@/components/ServerTable';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type DeadLetterFilters = {
  status?: DeadLetterStatus;
};

type OutboxFilters = {
  status?: OutboxStatus;
  event_type?: string;
};

const deadLetterStatusOptions = [
  { label: 'PENDING', value: 'PENDING' },
  { label: 'REPLAYED', value: 'REPLAYED' }
];

const outboxStatusOptions = [
  { label: 'PENDING', value: 'PENDING' },
  { label: 'FAILED', value: 'FAILED' },
  { label: 'DISPATCHING', value: 'DISPATCHING' },
  { label: 'SENT', value: 'SENT' },
  { label: 'DEAD', value: 'DEAD' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const deadLetterStatusColors: Record<DeadLetterStatus, string> = {
  PENDING: 'orange',
  REPLAYED: 'green'
};

const outboxStatusColors: Record<OutboxStatus, string> = {
  PENDING: 'blue',
  FAILED: 'red',
  DISPATCHING: 'orange',
  SENT: 'green',
  DEAD: 'volcano'
};

const resolveTotal = (
  pagination: TablePaginationConfig,
  nextCursor?: string | null
): number => {
  const current = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  return nextCursor ? (current + 1) * pageSize : current * pageSize;
};

export const FailureQueuePage = () => {
  const tenantId = useTenantId();
  const canReplay = useCan('dlq.replay');
  const queryClient = useQueryClient();
  const [deadForm] = Form.useForm<DeadLetterFilters>();
  const [outboxForm] = Form.useForm<OutboxFilters>();
  const [deadFilters, setDeadFilters] = useState<DeadLetterFilters>({});
  const [outboxFilters, setOutboxFilters] = useState<OutboxFilters>({});
  const [deadPagination, setDeadPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [outboxPagination, setOutboxPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [deadCursorMap, setDeadCursorMap] = useState<Record<number, string | undefined>>({
    1: undefined
  });
  const [outboxCursorMap, setOutboxCursorMap] = useState<Record<number, string | undefined>>({
    1: undefined
  });

  const deadQuery = useMemo<DeadLetterListQuery>(
    () => ({
      status: deadFilters.status,
      cursor: deadCursorMap[deadPagination.current ?? 1],
      limit: deadPagination.pageSize ?? 20
    }),
    [deadFilters, deadPagination.current, deadPagination.pageSize, deadCursorMap]
  );

  const outboxQuery = useMemo<OutboxEventListQuery>(
    () => ({
      status: outboxFilters.status,
      event_type: outboxFilters.event_type,
      cursor: outboxCursorMap[outboxPagination.current ?? 1],
      limit: outboxPagination.pageSize ?? 20
    }),
    [outboxFilters, outboxPagination.current, outboxPagination.pageSize, outboxCursorMap]
  );

  const deadLettersQuery = useQuery({
    queryKey: ['dead-letters', tenantId, deadQuery],
    queryFn: () => listDeadLetters(tenantId, deadQuery),
    keepPreviousData: true
  });

  const outboxEventsQuery = useQuery({
    queryKey: ['outbox-events', tenantId, outboxQuery],
    queryFn: () => listOutboxEvents(tenantId, outboxQuery),
    keepPreviousData: true
  });

  const handleDeadSearch = (values: DeadLetterFilters) => {
    setDeadFilters(values);
    setDeadPagination((prev) => ({ ...prev, current: 1 }));
    setDeadCursorMap({ 1: undefined });
  };

  const handleDeadReset = () => {
    deadForm.resetFields();
    setDeadFilters({});
    setDeadPagination((prev) => ({ ...prev, current: 1 }));
    setDeadCursorMap({ 1: undefined });
  };

  const handleOutboxSearch = (values: OutboxFilters) => {
    setOutboxFilters(values);
    setOutboxPagination((prev) => ({ ...prev, current: 1 }));
    setOutboxCursorMap({ 1: undefined });
  };

  const handleOutboxReset = () => {
    outboxForm.resetFields();
    setOutboxFilters({});
    setOutboxPagination((prev) => ({ ...prev, current: 1 }));
    setOutboxCursorMap({ 1: undefined });
  };

  const handleReplay = async (deadLetterId: string) => {
    try {
      await replayDeadLetter(tenantId, deadLetterId);
      message.success('重放已提交');
      await queryClient.invalidateQueries({ queryKey: ['dead-letters', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['outbox-events', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '重放失败');
    }
  };

  const handleRequeue = async (outboxId: string) => {
    try {
      await requeueOutboxEvent(tenantId, outboxId);
      message.success('已重新入队');
      await queryClient.invalidateQueries({ queryKey: ['outbox-events', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '重试失败');
    }
  };

  const deadLetterColumns = useMemo<ColumnsType<DeadLetter>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={deadLetterStatusColors[record.status]}>{record.status}</Tag>
        )
      },
      {
        title: '事件类型',
        dataIndex: 'event_type'
      },
      {
        title: '错误',
        dataIndex: 'error',
        ellipsis: true,
        render: (_, record) => record.error || '-'
      },
      {
        title: '尝试次数',
        dataIndex: 'attempts'
      },
      {
        title: '分类',
        dataIndex: 'classification'
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: '重放时间',
        dataIndex: 'replayed_at',
        render: (_, record) => formatDateTime(record.replayed_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) =>
          record.status === 'PENDING' && canReplay ? (
            <Button size="small" onClick={() => handleReplay(record.dead_letter_id)}>
              重放
            </Button>
          ) : (
            <span>-</span>
          )
      }
    ],
    []
  );

  const outboxColumns = useMemo<ColumnsType<OutboxEvent>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={outboxStatusColors[record.status]}>{record.status}</Tag>
        )
      },
      {
        title: '事件类型',
        dataIndex: 'event_type'
      },
      {
        title: '尝试次数',
        dataIndex: 'attempts'
      },
      {
        title: '错误',
        dataIndex: 'last_error',
        ellipsis: true,
        render: (_, record) => record.last_error || '-'
      },
      {
        title: '下次重试',
        dataIndex: 'next_retry_at',
        render: (_, record) => formatDateTime(record.next_retry_at)
      },
      {
        title: '最近尝试',
        dataIndex: 'last_attempt_at',
        render: (_, record) => formatDateTime(record.last_attempt_at)
      },
      {
        title: '投递时间',
        dataIndex: 'published_at',
        render: (_, record) => formatDateTime(record.published_at)
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) =>
          ['FAILED', 'DEAD', 'DISPATCHING'].includes(record.status) && canReplay ? (
            <Button size="small" onClick={() => handleRequeue(record.outbox_id)}>
              重试
            </Button>
          ) : (
            <span>-</span>
          )
      }
    ],
    []
  );

  const deadLetters = deadLettersQuery.data?.data ?? [];
  const outboxEvents = outboxEventsQuery.data?.data ?? [];

  if (deadLettersQuery.error && deadLetters.length === 0) {
    return (
      <ErrorState
        title="失败任务加载失败"
        error={deadLettersQuery.error}
        onRetry={() => deadLettersQuery.refetch()}
      />
    );
  }

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3}>失败队列</Typography.Title>
      <Tabs
        items={[
          {
            key: 'dead-letters',
            label: '失败任务',
            children: (
              <Space direction="vertical" size={16} style={{ width: '100%' }}>
                <Card>
                  <Form form={deadForm} layout="vertical" onFinish={handleDeadSearch}>
                    <Row gutter={16}>
                      <Col xs={24} md={6}>
                        <Form.Item name="status" label="状态">
                          <Select allowClear options={deadLetterStatusOptions} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
                        <Space>
                          <Button type="primary" htmlType="submit">
                            查询
                          </Button>
                          <Button onClick={handleDeadReset}>重置</Button>
                        </Space>
                      </Col>
                    </Row>
                  </Form>
                </Card>
                <ServerTable<DeadLetter>
                  rowKey="dead_letter_id"
                  columns={deadLetterColumns}
                  data={deadLetters}
                  loading={deadLettersQuery.isLoading || deadLettersQuery.isFetching}
                  error={deadLettersQuery.error}
                  onRetry={() => deadLettersQuery.refetch()}
                  errorTitle="失败任务加载失败"
                  emptyText="暂无失败任务"
                  total={resolveTotal(deadPagination, deadLettersQuery.data?.next_cursor)}
                  pagination={deadPagination}
                  onPaginationChange={(pagination) => {
                    const nextPageSize = pagination.pageSize ?? deadPagination.pageSize;
                    if (nextPageSize !== deadPagination.pageSize) {
                      setDeadCursorMap({ 1: undefined });
                      setDeadPagination({ current: 1, pageSize: nextPageSize });
                      return;
                    }
                    setDeadPagination((prev) => ({
                      ...prev,
                      current: pagination.current,
                      pageSize: nextPageSize
                    }));
                    if (pagination.current && deadLettersQuery.data?.next_cursor) {
                      setDeadCursorMap((prev) => ({
                        ...prev,
                        [pagination.current + 1]: deadLettersQuery.data?.next_cursor ?? undefined
                      }));
                    }
                  }}
                  columnSettingsKey="failure-queue.dead-letters.columns"
                />
              </Space>
            )
          },
          {
            key: 'outbox',
            label: 'Outbox堆积',
            children: (
              <Space direction="vertical" size={16} style={{ width: '100%' }}>
                <Card>
                  <Form form={outboxForm} layout="vertical" onFinish={handleOutboxSearch}>
                    <Row gutter={16}>
                      <Col xs={24} md={6}>
                        <Form.Item name="status" label="状态">
                          <Select allowClear options={outboxStatusOptions} />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={6}>
                        <Form.Item name="event_type" label="事件类型">
                          <Input allowClear placeholder="ORDER_UPSERTED" />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
                        <Space>
                          <Button type="primary" htmlType="submit">
                            查询
                          </Button>
                          <Button onClick={handleOutboxReset}>重置</Button>
                        </Space>
                      </Col>
                    </Row>
                  </Form>
                </Card>
                <ServerTable<OutboxEvent>
                  rowKey="outbox_id"
                  columns={outboxColumns}
                  data={outboxEvents}
                  loading={outboxEventsQuery.isLoading || outboxEventsQuery.isFetching}
                  error={outboxEventsQuery.error}
                  onRetry={() => outboxEventsQuery.refetch()}
                  errorTitle="Outbox加载失败"
                  emptyText="暂无Outbox事件"
                  total={resolveTotal(outboxPagination, outboxEventsQuery.data?.next_cursor)}
                  pagination={outboxPagination}
                  onPaginationChange={(pagination) => {
                    const nextPageSize = pagination.pageSize ?? outboxPagination.pageSize;
                    if (nextPageSize !== outboxPagination.pageSize) {
                      setOutboxCursorMap({ 1: undefined });
                      setOutboxPagination({ current: 1, pageSize: nextPageSize });
                      return;
                    }
                    setOutboxPagination((prev) => ({
                      ...prev,
                      current: pagination.current,
                      pageSize: nextPageSize
                    }));
                    if (pagination.current && outboxEventsQuery.data?.next_cursor) {
                      setOutboxCursorMap((prev) => ({
                        ...prev,
                        [pagination.current + 1]: outboxEventsQuery.data?.next_cursor ?? undefined
                      }));
                    }
                  }}
                  columnSettingsKey="failure-queue.outbox.columns"
                />
              </Space>
            )
          }
        ]}
      />
    </Space>
  );
};
