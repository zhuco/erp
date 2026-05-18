import { Progress, Space, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useQuery } from '@tanstack/react-query';

import { listPlanningPlanExecutions, type PlanningPlanExecution } from '@/api/planning';
import { ErpDataGrid } from '@/components/ErpDataGrid';
import { ErrorState } from '@/components/QueryState';
import { useTenantId } from '@/features/tenant';

const columns: ColumnsType<PlanningPlanExecution> = [
  { title: 'Execution ID', dataIndex: 'execution_id' },
  { title: 'Plan Type', dataIndex: 'plan_type' },
  { title: 'Related Plan', dataIndex: 'related_plan' },
  { title: 'Owner', dataIndex: 'owner' },
  {
    title: 'Progress',
    dataIndex: 'progress',
    render: (value: number) => <Progress percent={Math.max(0, Math.min(100, value))} size="small" />
  },
  {
    title: 'Status',
    dataIndex: 'status',
    render: (value: string) => (
      <Tag color={value === 'COMPLETED' ? 'green' : value === 'WARNING' ? 'red' : 'blue'}>{value}</Tag>
    )
  },
  { title: 'Variance', dataIndex: 'variance' }
];

const avgProgress = (items: PlanningPlanExecution[], type: string) => {
  const picked = items.filter((item) => item.plan_type === type);
  if (!picked.length) {
    return 0;
  }
  return Math.round(picked.reduce((sum, item) => sum + item.progress, 0) / picked.length);
};

export const PlanExecutionPage = () => {
  const tenantId = useTenantId();
  const query = useQuery({
    queryKey: ['planning', 'plan-executions', tenantId],
    queryFn: () => listPlanningPlanExecutions(tenantId),
    placeholderData: (previousData) => previousData
  });

  const items = query.data?.items ?? [];
  const purchaseProgress = avgProgress(items, 'PURCHASE_PLAN');
  const transferProgress = avgProgress(items, 'TRANSFER_SUGGESTION');
  const promoProgress = avgProgress(items, 'PROMO_STOCKING');

  return (
    <Space
      direction="vertical"
      size={16}
      style={{ width: '100%' }}
      data-testid="planning-execution-page"
    >
      <section className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          Execution Overview
        </Typography.Title>
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Typography.Text>Purchase Plans</Typography.Text>
          <Progress percent={purchaseProgress} />
          <Typography.Text>Transfer Suggestions</Typography.Text>
          <Progress percent={transferProgress} />
          <Typography.Text>Promo Stocking</Typography.Text>
          <Progress percent={promoProgress} status="active" />
        </Space>
      </section>

      <section className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          Execution Tracker
        </Typography.Title>
        {query.error ? (
          <ErrorState
            title="Failed to load plan execution rows"
            error={query.error}
            onRetry={() => query.refetch()}
          />
        ) : (
          <ErpDataGrid<PlanningPlanExecution>
            data-testid="planning-execution-table"
            rowKey={(record) => record.execution_id}
            loading={query.isLoading || query.isFetching}
            columns={columns}
            data={items}
            compact
            showRowIndex={false}
          />
        )}
      </section>
    </Space>
  );
};
