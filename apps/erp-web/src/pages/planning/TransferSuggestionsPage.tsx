import { Button, Select, Space, Tag, message, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import {
  generatePlanningTransferSuggestions,
  listPlanningTransferSuggestions,
  type PlanningTransferSuggestion
} from '@/api/planning';
import { ErpDataGrid } from '@/components/ErpDataGrid';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

const columns: ColumnsType<PlanningTransferSuggestion> = [
  { title: 'Suggestion ID', dataIndex: 'suggestion_id' },
  { title: 'From Warehouse', dataIndex: 'from_warehouse_id' },
  { title: 'To Warehouse', dataIndex: 'to_warehouse_id' },
  { title: 'SKU Code', dataIndex: 'sku_code' },
  { title: 'Qty', dataIndex: 'qty' },
  { title: 'Reason', dataIndex: 'reason' },
  {
    title: 'Priority',
    dataIndex: 'priority',
    render: (value: string) => (
      <Tag color={value === 'HIGH' ? 'red' : value === 'MEDIUM' ? 'gold' : 'default'}>{value}</Tag>
    )
  },
  {
    title: 'Status',
    dataIndex: 'status',
    render: (value: string) => (
      <Tag color={value === 'DONE' ? 'green' : value === 'PENDING' ? 'blue' : 'default'}>{value}</Tag>
    )
  },
  { title: 'Created At', dataIndex: 'created_at' }
];

export const TransferSuggestionsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [strategy, setStrategy] = useState('BALANCE');

  const listQuery = useQuery({
    queryKey: ['planning', 'transfer-suggestions', tenantId],
    queryFn: () => listPlanningTransferSuggestions(tenantId),
    placeholderData: (previousData) => previousData
  });

  const generateMutation = useMutation({
    mutationFn: () => generatePlanningTransferSuggestions(tenantId, strategy),
    onSuccess: async (result) => {
      message.success(`Generated ${result.total} transfer suggestion(s)`);
      await queryClient.invalidateQueries({
        queryKey: ['planning', 'transfer-suggestions', tenantId]
      });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : 'Failed to generate transfer suggestions');
    }
  });

  return (
    <Space
      direction="vertical"
      size={16}
      style={{ width: '100%' }}
      data-testid="planning-transfer-page"
    >
      <section className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          Transfer Suggestion Strategy
        </Typography.Title>
        <Space wrap>
          <Select
            value={strategy}
            onChange={setStrategy}
            style={{ width: 180 }}
            options={[
              { value: 'BALANCE', label: 'BALANCE' },
              { value: 'TIME', label: 'TIME' },
              { value: 'COST', label: 'COST' }
            ]}
          />
          <Button
            type="primary"
            onClick={() => generateMutation.mutate()}
            loading={generateMutation.isPending}
            disabled={!canWrite}
            data-testid="planning-transfer-generate"
          >
            Generate Suggestions
          </Button>
        </Space>
      </section>

      <section className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          Transfer Suggestion List
        </Typography.Title>
        {listQuery.error ? (
          <ErrorState
            title="Failed to load transfer suggestions"
            error={listQuery.error}
            onRetry={() => listQuery.refetch()}
          />
        ) : (
          <ErpDataGrid<PlanningTransferSuggestion>
            data-testid="planning-transfer-table"
            rowKey={(record) => record.suggestion_id}
            loading={listQuery.isLoading || listQuery.isFetching}
            columns={columns}
            data={listQuery.data?.items ?? []}
            compact
            showRowIndex={false}
          />
        )}
      </section>
    </Space>
  );
};
