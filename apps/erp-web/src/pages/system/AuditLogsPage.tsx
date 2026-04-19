import { Button, Card, Col, DatePicker, Form, Input, Row, Space, Typography } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import type { Dayjs } from 'dayjs';
import { useQuery } from '@tanstack/react-query';

import { listAuditLogs, type AuditLog, type AuditLogListQuery } from '@/api/system';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { isTenantSelected, useTenantId } from '@/features/tenant';
import {
  resolveAuditEntityTypeLabel,
  resolveOperationActionLabel
} from '@/utils/logLabels';

type FilterValues = {
  action?: string;
  entity_type?: string;
  entity_id?: string;
  user_id?: string;
  created_range?: [Dayjs, Dayjs];
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

export const AuditLogsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [cursorMap, setCursorMap] = useState<Record<number, string | undefined>>({
    1: undefined
  });

  const query = useMemo<AuditLogListQuery>(() => {
    const range = filters.created_range;
    return {
      action: filters.action?.trim() || undefined,
      entity_type: filters.entity_type?.trim() || undefined,
      entity_id: filters.entity_id?.trim() || undefined,
      user_id: filters.user_id?.trim() || undefined,
      created_from: range?.[0]?.toISOString(),
      created_to: range?.[1]?.toISOString(),
      cursor: cursorMap[pagination.current ?? 1],
      limit: pagination.pageSize ?? 20
    };
  }, [cursorMap, filters, pagination.current, pagination.pageSize]);

  const auditQuery = useQuery({
    queryKey: ['audit-logs', tenantId, query],
    queryFn: () => listAuditLogs(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  useEffect(() => {
    const nextCursor = auditQuery.data?.next_cursor ?? undefined;
    if (!nextCursor) {
      return;
    }
    const current = pagination.current ?? 1;
    setCursorMap((prev) => {
      if (prev[current + 1]) {
        return prev;
      }
      return { ...prev, [current + 1]: nextCursor };
    });
  }, [auditQuery.data?.next_cursor, pagination.current]);

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
    setCursorMap({ 1: undefined });
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
    setCursorMap({ 1: undefined });
  };

  const columns = useMemo<ColumnsType<AuditLog>>(
    () => [
      {
        title: '动作',
        dataIndex: 'action',
        render: (value) => resolveOperationActionLabel(value)
      },
      {
        title: '实体类型',
        dataIndex: 'entity_type',
        render: (value) => resolveAuditEntityTypeLabel(value)
      },
      { title: '实体ID', dataIndex: 'entity_id' },
      { title: '用户ID', dataIndex: 'user_id' },
      { title: 'IP', dataIndex: 'ip' },
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      }
    ],
    []
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        审计日志
      </Typography.Title>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col span={6}>
              <Form.Item name="action" label="动作">
                <Input placeholder="OMS.REVIEW_PASS" />
              </Form.Item>
            </Col>
            <Col span={6}>
              <Form.Item name="entity_type" label="实体类型">
                <Input placeholder="ORDER" />
              </Form.Item>
            </Col>
            <Col span={6}>
              <Form.Item name="entity_id" label="实体ID">
                <Input placeholder="实体ID" />
              </Form.Item>
            </Col>
            <Col span={6}>
              <Form.Item name="user_id" label="用户ID">
                <Input placeholder="用户ID" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="created_range" label="时间范围">
                <DatePicker.RangePicker
                  showTime
                  allowClear
                  style={{ width: '100%' }}
                  placeholder={['开始时间', '结束时间']}
                />
              </Form.Item>
            </Col>
            <Col span={12} style={{ display: 'flex', alignItems: 'end' }}>
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
    </Space>
  );

  return (
    <UiModule03MainTableSection<AuditLog>
      header={header}
      columns={columns}
      data={auditQuery.data?.data ?? []}
      loading={auditQuery.isFetching}
      error={auditQuery.error}
      onRetry={() => auditQuery.refetch()}
      errorTitle="加载审计日志失败"
      emptyText="暂无审计日志"
      total={resolveTotal(pagination, auditQuery.data?.next_cursor)}
      pagination={pagination}
      onPaginationChange={setPagination}
      rowKey="audit_id"
      enablePanels={false}
    />
  );
};
