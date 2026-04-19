import {
  Button,
  Card,
  Descriptions,
  Drawer,
  Input,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { closeApDoc, getApDoc, listApDocs, markApDocPaid, type ApDoc } from '@/api/finance';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type ApDocFilters = {
  status?: string;
  supplier_id?: string;
};

const statusOptions = [
  { label: 'OPEN', value: 'OPEN' },
  { label: 'PAID', value: 'PAID' },
  { label: 'CLOSED', value: 'CLOSED' }
];

const statusTag = (status: string) => {
  if (status === 'PAID') {
    return <Tag color="green">PAID</Tag>;
  }
  if (status === 'CLOSED') {
    return <Tag color="default">CLOSED</Tag>;
  }
  return <Tag color="blue">OPEN</Tag>;
};

const settlementTag = (status: string) => {
  if (status === 'SETTLED') {
    return <Tag color="green">已结清</Tag>;
  }
  if (status === 'SETTLING') {
    return <Tag color="orange">结算中</Tag>;
  }
  return <Tag>未结清</Tag>;
};

const formatAmount = (value: number) => `￥${(value / 100).toFixed(2)}`;

export const ApDocsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('finance.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<ApDocFilters>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [selectedApId, setSelectedApId] = useState<string | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [marking, setMarking] = useState(false);
  const [closing, setClosing] = useState(false);

  const query = useMemo(() => {
    const offset = ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 10);
    return {
      status: filters.status,
      supplier_id: filters.supplier_id,
      offset,
      limit: pagination.pageSize ?? 10
    };
  }, [filters.status, filters.supplier_id, pagination.current, pagination.pageSize]);

  const listQuery = useQuery({
    queryKey: ['apDocs', tenantId, query],
    queryFn: () => listApDocs(tenantId, query),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData
  });

  const detailQuery = useQuery({
    queryKey: ['apDoc', tenantId, selectedApId],
    queryFn: () => getApDoc(tenantId, selectedApId ?? ''),
    enabled: Boolean(tenantId && selectedApId),
    placeholderData: (previousData) => previousData
  });

  const handleView = (doc: ApDoc) => {
    setSelectedApId(doc.ap_id);
    setDrawerOpen(true);
  };

  const handleMarkPaid = async () => {
    if (!tenantId || !selectedApId) {
      message.error('商家信息缺失');
      return;
    }
    setMarking(true);
    try {
      await markApDocPaid(tenantId, selectedApId);
      message.success('已标记为已付款');
      await queryClient.invalidateQueries({ queryKey: ['apDocs', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['apDoc', tenantId, selectedApId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '标记失败');
    } finally {
      setMarking(false);
    }
  };

  const handleCloseDoc = async () => {
    if (!tenantId || !selectedApId) {
      message.error('应付单据缺失');
      return;
    }
    setClosing(true);
    try {
      await closeApDoc(tenantId, selectedApId);
      message.success('已完成关单');
      await queryClient.invalidateQueries({ queryKey: ['apDocs', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['apDoc', tenantId, selectedApId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '关单失败');
    } finally {
      setClosing(false);
    }
  };

  const columns = useMemo<ColumnsType<ApDoc>>(
    () => [
      { title: '供应商', dataIndex: 'supplier_name', render: (_, record) => record.supplier_name ?? '-' },
      { title: '金额', dataIndex: 'amount_cent', render: (value) => formatAmount(value) },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => statusTag(record.status)
      },
      {
        title: '结算状态',
        dataIndex: 'settlement_status',
        render: (_, record) => settlementTag(record.settlement_status)
      },
      {
        title: '来源',
        render: (_, record) => `${record.ref_type}:${record.ref_id}`
      },
      { title: '创建时间', dataIndex: 'created_at' },
      {
        title: '操作',
        render: (_, record) => (
          <Button size="small" onClick={() => handleView(record)}>
            详情
          </Button>
        )
      }
    ],
    []
  );

  const detail = detailQuery.data;
  const canMarkPaid = detail?.status === 'OPEN';
  const canClose = detail?.status === 'PAID';

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        应付列表
      </Typography.Title>
      <Card>
        <Space wrap>
          <Select
            allowClear
            placeholder="状态"
            style={{ width: 140 }}
            options={statusOptions}
            onChange={(value) => {
              setFilters((prev) => ({ ...prev, status: value || undefined }));
              setPagination((prev) => ({ ...prev, current: 1 }));
            }}
          />
          <Input
            placeholder="供应商ID"
            style={{ width: 220 }}
            value={filters.supplier_id}
            onChange={(event) =>
              setFilters((prev) => ({ ...prev, supplier_id: event.target.value || undefined }))
            }
          />
        </Space>
      </Card>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<ApDoc>
        header={header}
        rowKey="ap_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="应付列表加载失败"
        emptyText="暂无应付记录"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="finance-ap-docs.table.columns"
      />
      <Drawer
        title="应付详情"
        width={480}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        extra={
          <Space>
            <Button
              type="primary"
              disabled={!canWrite || !canMarkPaid}
              loading={marking}
              onClick={handleMarkPaid}
            >
              标记已付款
            </Button>
            <Button
              disabled={!canWrite || !canClose}
              loading={closing}
              onClick={handleCloseDoc}
            >
              关闭单据
            </Button>
          </Space>
        }
      >
        <Descriptions column={1} bordered size="small">
          <Descriptions.Item label="供应商">
            {detail?.supplier_name ?? detail?.supplier_id ?? '-'}
          </Descriptions.Item>
          <Descriptions.Item label="金额">
            {detail ? formatAmount(detail.amount_cent) : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="状态">
            {detail ? statusTag(detail.status) : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="结算状态">
            {detail ? settlementTag(detail.settlement_status) : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="来源">
            {detail ? `${detail.ref_type}:${detail.ref_id}` : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="创建时间">{detail?.created_at ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="更新时间">{detail?.updated_at ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="付款时间">{detail?.paid_at ?? '-'}</Descriptions.Item>
        </Descriptions>
      </Drawer>
    </>
  );
};
