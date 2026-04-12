import { Button, Card, DatePicker, Form, Input, InputNumber, Space, Table, Tag, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  generatePlanningPromoStocking,
  listPlanningPromoStocking,
  type PlanningPromoStockingCampaign
} from '@/api/planning';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type PromoFormValues = {
  campaign_name: string;
  period: [dayjs.Dayjs, dayjs.Dayjs];
  locked_budget_cent?: number;
};

const columns: ColumnsType<PlanningPromoStockingCampaign> = [
  { title: 'Campaign ID', dataIndex: 'campaign_id' },
  { title: 'Campaign Name', dataIndex: 'campaign_name' },
  { title: 'Period Start', dataIndex: 'period_start' },
  { title: 'Period End', dataIndex: 'period_end' },
  { title: 'Locked Budget (Cent)', dataIndex: 'locked_budget_cent' },
  {
    title: 'Status',
    dataIndex: 'status',
    render: (value: string) => <Tag color={value === 'DONE' ? 'green' : 'blue'}>{value}</Tag>
  },
  { title: 'Created At', dataIndex: 'created_at' }
];

export const PromoStockingPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('inventory.write');
  const [form] = Form.useForm<PromoFormValues>();
  const queryClient = useQueryClient();

  const listQuery = useQuery({
    queryKey: ['planning', 'promo-stocking', tenantId],
    queryFn: () => listPlanningPromoStocking(tenantId),
    keepPreviousData: true
  });

  const generateMutation = useMutation({
    mutationFn: (values: PromoFormValues) =>
      generatePlanningPromoStocking(tenantId, {
        campaign_name: values.campaign_name.trim(),
        period_start: values.period[0].format('YYYY-MM-DD'),
        period_end: values.period[1].format('YYYY-MM-DD'),
        locked_budget_cent: values.locked_budget_cent ?? 0
      }),
    onSuccess: async () => {
      message.success('Promo stocking campaign generated');
      await queryClient.invalidateQueries({
        queryKey: ['planning', 'promo-stocking', tenantId]
      });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : 'Failed to generate promo stocking campaign');
    }
  });

  const handleGenerate = async () => {
    const values = await form.validateFields();
    if (!canWrite) {
      message.warning('No permission to generate promo stocking campaign');
      return;
    }
    generateMutation.mutate(values);
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }} data-testid="planning-promo-page">
      <Card title="Promo Stocking Campaign" size="small">
        <Form
          form={form}
          layout="inline"
          initialValues={{
            period: [dayjs().startOf('month'), dayjs().startOf('month').add(14, 'day')],
            locked_budget_cent: 100000
          }}
        >
          <Form.Item
            name="campaign_name"
            label="Campaign Name"
            rules={[{ required: true, message: 'Required' }]}
          >
            <Input style={{ width: 220 }} placeholder="e.g. 618 Campaign" />
          </Form.Item>
          <Form.Item name="period" label="Period" rules={[{ required: true, message: 'Required' }]}>
            <DatePicker.RangePicker allowClear={false} />
          </Form.Item>
          <Form.Item name="locked_budget_cent" label="Budget (Cent)">
            <InputNumber min={0} style={{ width: 160 }} />
          </Form.Item>
          <Form.Item>
            <Button
              type="primary"
              onClick={handleGenerate}
              loading={generateMutation.isPending}
              disabled={!canWrite}
              data-testid="planning-promo-generate"
            >
              Generate Campaign
            </Button>
          </Form.Item>
        </Form>
      </Card>

      <Card title="Promo Stocking Campaign List" size="small">
        {listQuery.error ? (
          <ErrorState
            title="Failed to load promo stocking campaigns"
            error={listQuery.error}
            onRetry={() => listQuery.refetch()}
          />
        ) : (
          <Table<PlanningPromoStockingCampaign>
            data-testid="planning-promo-table"
            rowKey="campaign_id"
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
