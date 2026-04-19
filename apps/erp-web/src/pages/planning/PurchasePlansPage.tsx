import { Button, Card, Descriptions, Space, Table, Tag, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  generatePlanningPurchasePlans,
  listPlanningPurchasePlans,
  type PlanningPurchasePlan
} from '@/api/planning';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

const columns: ColumnsType<PlanningPurchasePlan> = [
  { title: 'Plan ID', dataIndex: 'plan_id' },
  { title: 'Supplier', dataIndex: 'supplier' },
  { title: 'Warehouse ID', dataIndex: 'warehouse_id' },
  { title: 'Items', dataIndex: 'items' },
  { title: 'Total Qty', dataIndex: 'total_qty' },
  {
    title: 'Status',
    dataIndex: 'status',
    render: (value: string) => {
      const color =
        value === 'DONE' ? 'green' : value === 'IN_PROGRESS' ? 'blue' : value === 'FAILED' ? 'red' : 'gold';
      return <Tag color={color}>{value}</Tag>;
    }
  },
  { title: 'ETA', dataIndex: 'eta', render: (value) => value ?? '-' },
  { title: 'Source Rule', dataIndex: 'source_rule_id', render: (value) => value ?? '-' },
  { title: 'Created At', dataIndex: 'created_at' }
];

export const PurchasePlansPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();

  const listQuery = useQuery({
    queryKey: ['planning', 'purchase-plans', tenantId],
    queryFn: () => listPlanningPurchasePlans(tenantId),
    placeholderData: (previousData) => previousData
  });

  const generateMutation = useMutation({
    mutationFn: () => generatePlanningPurchasePlans(tenantId, 'REPLENISHMENT'),
    onSuccess: async (result) => {
      message.success(`Generated ${result.total} purchase plan(s)`);
      await queryClient.invalidateQueries({
        queryKey: ['planning', 'purchase-plans', tenantId]
      });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : 'Failed to generate purchase plans');
    }
  });

  const items = listQuery.data?.items ?? [];
  const summary = {
    pending: items.filter((item) => item.status === 'PENDING_APPROVAL').length,
    processing: items.filter((item) => item.status === 'IN_PROGRESS').length,
    done: items.filter((item) => item.status === 'DONE').length
  };

  return (
    <Space
      direction="vertical"
      size={16}
      style={{ width: '100%' }}
      data-testid="planning-purchase-page"
    >
      <Card
        title="Purchase Plan Workbench"
        size="small"
        extra={
          <Button
            type="primary"
            onClick={() => generateMutation.mutate()}
            loading={generateMutation.isPending}
            disabled={!canWrite}
            data-testid="planning-purchase-generate"
          >
            Generate Purchase Plans
          </Button>
        }
      >
        <Descriptions size="small" bordered column={3}>
          <Descriptions.Item label="Pending Approval">{summary.pending}</Descriptions.Item>
          <Descriptions.Item label="In Progress">{summary.processing}</Descriptions.Item>
          <Descriptions.Item label="Done">{summary.done}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card title="Purchase Plan List" size="small">
        {listQuery.error ? (
          <ErrorState
            title="Failed to load purchase plans"
            error={listQuery.error}
            onRetry={() => listQuery.refetch()}
          />
        ) : (
          <Table<PlanningPurchasePlan>
            data-testid="planning-purchase-table"
            rowKey="plan_id"
            loading={listQuery.isLoading || listQuery.isFetching}
            columns={columns}
            dataSource={items}
            pagination={false}
          />
        )}
      </Card>
    </Space>
  );
};
