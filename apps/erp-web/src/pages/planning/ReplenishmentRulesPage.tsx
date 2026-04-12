import { Button, Card, Form, Input, InputNumber, Select, Space, Table, Tag, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  generatePlanningPurchasePlans,
  listPlanningReplenishmentRules,
  type PlanningReplenishmentRule,
  upsertPlanningReplenishmentRule
} from '@/api/planning';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type RuleFormValues = {
  warehouse_id: string;
  category: string;
  safety_stock_days: number;
  replenishment_cycle_days: number;
  min_replenishment_qty: number;
  status: string;
};

const columns: ColumnsType<PlanningReplenishmentRule> = [
  { title: 'Rule ID', dataIndex: 'rule_id' },
  { title: 'Warehouse ID', dataIndex: 'warehouse_id' },
  { title: 'Category', dataIndex: 'category' },
  { title: 'Safety Stock Days', dataIndex: 'safety_stock_days' },
  { title: 'Replenishment Cycle Days', dataIndex: 'replenishment_cycle_days' },
  { title: 'Min Replenishment Qty', dataIndex: 'min_replenishment_qty' },
  {
    title: 'Status',
    dataIndex: 'status',
    render: (value: string) => (
      <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value}</Tag>
    )
  },
  { title: 'Updated At', dataIndex: 'updated_at' }
];

export const ReplenishmentRulesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('inventory.write');
  const [form] = Form.useForm<RuleFormValues>();
  const queryClient = useQueryClient();

  const listQuery = useQuery({
    queryKey: ['planning', 'replenishment-rules', tenantId],
    queryFn: () => listPlanningReplenishmentRules(tenantId),
    keepPreviousData: true
  });

  const upsertMutation = useMutation({
    mutationFn: (values: RuleFormValues) =>
      upsertPlanningReplenishmentRule(tenantId, {
        warehouse_id: values.warehouse_id.trim(),
        category: values.category.trim(),
        safety_stock_days: values.safety_stock_days,
        replenishment_cycle_days: values.replenishment_cycle_days,
        min_replenishment_qty: values.min_replenishment_qty,
        status: values.status
      }),
    onSuccess: async () => {
      message.success('Replenishment rule saved');
      await queryClient.invalidateQueries({
        queryKey: ['planning', 'replenishment-rules', tenantId]
      });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : 'Failed to save replenishment rule');
    }
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

  const handleSave = async () => {
    const values = await form.validateFields();
    if (!canWrite) {
      message.warning('No permission to save rule');
      return;
    }
    upsertMutation.mutate(values);
  };

  return (
    <Space
      direction="vertical"
      size={16}
      style={{ width: '100%' }}
      data-testid="planning-replenishment-page"
    >
      <Card title="Replenishment Rule Input" size="small">
        <Form
          form={form}
          layout="inline"
          initialValues={{
            category: 'BASIC',
            safety_stock_days: 15,
            replenishment_cycle_days: 7,
            min_replenishment_qty: 120,
            status: 'ACTIVE'
          }}
        >
          <Form.Item
            name="warehouse_id"
            label="Warehouse ID"
            rules={[{ required: true, message: 'Required' }]}
          >
            <Input style={{ width: 240 }} placeholder="UUID" />
          </Form.Item>
          <Form.Item name="category" label="Category" rules={[{ required: true, message: 'Required' }]}>
            <Input style={{ width: 140 }} />
          </Form.Item>
          <Form.Item
            name="safety_stock_days"
            label="Safety Days"
            rules={[{ required: true, message: 'Required' }]}
          >
            <InputNumber min={1} />
          </Form.Item>
          <Form.Item
            name="replenishment_cycle_days"
            label="Cycle Days"
            rules={[{ required: true, message: 'Required' }]}
          >
            <InputNumber min={1} />
          </Form.Item>
          <Form.Item
            name="min_replenishment_qty"
            label="Min Qty"
            rules={[{ required: true, message: 'Required' }]}
          >
            <InputNumber min={1} />
          </Form.Item>
          <Form.Item name="status" label="Status">
            <Select
              style={{ width: 120 }}
              options={[
                { value: 'ACTIVE', label: 'ACTIVE' },
                { value: 'INACTIVE', label: 'INACTIVE' }
              ]}
            />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button
                type="primary"
                onClick={handleSave}
                loading={upsertMutation.isPending}
                disabled={!canWrite}
                data-testid="planning-replenishment-save"
              >
                Save Rule
              </Button>
              <Button
                onClick={() => generateMutation.mutate()}
                loading={generateMutation.isPending}
                disabled={!canWrite}
                data-testid="planning-generate-purchase"
              >
                Generate Purchase Plans
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      <Card title="Replenishment Rules" size="small">
        {listQuery.error ? (
          <ErrorState
            title="Failed to load replenishment rules"
            error={listQuery.error}
            onRetry={() => listQuery.refetch()}
          />
        ) : (
          <Table<PlanningReplenishmentRule>
            data-testid="planning-replenishment-table"
            rowKey="rule_id"
            loading={listQuery.isLoading || listQuery.isFetching}
            columns={columns}
            dataSource={listQuery.data?.items ?? []}
            pagination={false}
          />
        )}
      </Card>
    </Space>
  );
};
