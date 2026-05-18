import { Button, Form, Input, InputNumber, Select, Space, message, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  listPlanningDemandForecasts,
  type PlanningDemandForecast,
  upsertPlanningDemandForecast
} from '@/api/planning';
import { ErpDataGrid } from '@/components/ErpDataGrid';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type ForecastFormValues = {
  sku_id: string;
  sku_code?: string;
  warehouse_id: string;
  forecast_qty: number;
  model: string;
  confidence: string;
  version: string;
};

const columns: ColumnsType<PlanningDemandForecast> = [
  { title: 'Forecast ID', dataIndex: 'forecast_id' },
  { title: 'SKU ID', dataIndex: 'sku_id' },
  { title: 'SKU Code', dataIndex: 'sku_code', render: (value) => value ?? '-' },
  { title: 'Warehouse ID', dataIndex: 'warehouse_id' },
  { title: 'Forecast Qty', dataIndex: 'forecast_qty' },
  { title: 'Model', dataIndex: 'model' },
  { title: 'Confidence', dataIndex: 'confidence' },
  { title: 'Version', dataIndex: 'version' },
  { title: 'Updated At', dataIndex: 'updated_at' }
];

export const SalesForecastPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('inventory.write');
  const [form] = Form.useForm<ForecastFormValues>();
  const queryClient = useQueryClient();

  const listQuery = useQuery({
    queryKey: ['planning', 'forecasts', tenantId],
    queryFn: () => listPlanningDemandForecasts(tenantId),
    placeholderData: (previousData) => previousData
  });

  const upsertMutation = useMutation({
    mutationFn: (values: ForecastFormValues) =>
      upsertPlanningDemandForecast(tenantId, {
        sku_id: values.sku_id.trim(),
        sku_code: values.sku_code?.trim() || null,
        warehouse_id: values.warehouse_id.trim(),
        forecast_qty: values.forecast_qty,
        model: values.model,
        confidence: values.confidence,
        version: values.version.trim()
      }),
    onSuccess: async () => {
      message.success('Forecast saved');
      await queryClient.invalidateQueries({
        queryKey: ['planning', 'forecasts', tenantId]
      });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : 'Failed to save forecast');
    }
  });

  const handleSubmit = async () => {
    const values = await form.validateFields();
    if (!canWrite) {
      message.warning('No permission to save forecast');
      return;
    }
    upsertMutation.mutate(values);
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }} data-testid="planning-forecast-page">
      <section className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          Demand Forecast Input
        </Typography.Title>
        <Form
          form={form}
          layout="inline"
          initialValues={{
            model: 'SEASONAL',
            confidence: 'MEDIUM',
            version: 'LATEST'
          }}
        >
          <Form.Item name="sku_id" label="SKU ID" rules={[{ required: true, message: 'Required' }]}>
            <Input style={{ width: 240 }} placeholder="UUID" />
          </Form.Item>
          <Form.Item name="sku_code" label="SKU Code">
            <Input style={{ width: 160 }} />
          </Form.Item>
          <Form.Item
            name="warehouse_id"
            label="Warehouse ID"
            rules={[{ required: true, message: 'Required' }]}
          >
            <Input style={{ width: 240 }} placeholder="UUID" />
          </Form.Item>
          <Form.Item
            name="forecast_qty"
            label="Forecast Qty"
            rules={[{ required: true, message: 'Required' }]}
          >
            <InputNumber min={0} />
          </Form.Item>
          <Form.Item name="model" label="Model">
            <Select
              style={{ width: 140 }}
              options={[
                { value: 'SEASONAL', label: 'SEASONAL' },
                { value: 'TREND', label: 'TREND' },
                { value: 'PROMO', label: 'PROMO' }
              ]}
            />
          </Form.Item>
          <Form.Item name="confidence" label="Confidence">
            <Select
              style={{ width: 140 }}
              options={[
                { value: 'HIGH', label: 'HIGH' },
                { value: 'MEDIUM', label: 'MEDIUM' },
                { value: 'LOW', label: 'LOW' }
              ]}
            />
          </Form.Item>
          <Form.Item name="version" label="Version">
            <Input style={{ width: 140 }} />
          </Form.Item>
          <Form.Item>
            <Button
              type="primary"
              onClick={handleSubmit}
              loading={upsertMutation.isPending}
              disabled={!canWrite}
              data-testid="planning-forecast-save"
            >
              Save
            </Button>
          </Form.Item>
        </Form>
      </section>

      <section className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          Demand Forecast List
        </Typography.Title>
        {listQuery.error ? (
          <ErrorState
            title="Failed to load demand forecasts"
            error={listQuery.error}
            onRetry={() => listQuery.refetch()}
          />
        ) : (
          <ErpDataGrid<PlanningDemandForecast>
            data-testid="planning-forecast-table"
            rowKey={(record) => record.forecast_id}
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
