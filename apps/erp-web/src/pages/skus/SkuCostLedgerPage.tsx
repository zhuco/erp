import {
  Alert,
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { SkuCostLedger } from '@/api/skus';
import { adjustSkuCost, listSkuCostLedgers, searchSkus } from '@/api/skus';
import type { SkuSearchItem } from '@/api/skus';
import { ServerTable } from '@/components/ServerTable';
import { WorkbenchListPage } from '@/components/workbench/WorkbenchListPage';
import { UiModule02ToolbarRight } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  sku_id?: string;
  date_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type AdjustFormValues = {
  sku_id?: string;
  new_cost?: number;
  ref_type?: string;
  ref_id?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;

const buildSkuOptions = (items: SkuSearchItem[]) =>
  items.map((sku) => ({
    value: sku.sku_id,
    label: `${sku.sku_code}${sku.name ? ` ${sku.name}` : ''}`
  }));

export const SkuCostLedgerPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('skus.write');
  const costFeature = useFeatureFlag('cost.enabled');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [adjustForm] = Form.useForm<AdjustFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuSearching, setSkuSearching] = useState(false);
  const [adjustOpen, setAdjustOpen] = useState(false);

  const query = useMemo(
    () => ({
      sku_id: filters.sku_id?.trim() || undefined,
      start_at: filters.date_range?.[0]?.startOf('day').toISOString(),
      end_at: filters.date_range?.[1]?.endOf('day').toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters.date_range, filters.sku_id, pagination.current, pagination.pageSize]
  );

  const costLedgerQuery = useQuery({
    queryKey: ['sku-cost-ledgers', tenantId, query],
    queryFn: () => listSkuCostLedgers(tenantId, query),
    enabled: tenantReady && costFeature.enabled,
    retry: false
  });

  const loadSkuOptions = async (keyword: string) => {
    if (!tenantId) {
      return;
    }
    setSkuSearching(true);
    try {
      const response = await searchSkus(tenantId, keyword);
      setSkuOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'SKU 搜索失败');
    } finally {
      setSkuSearching(false);
    }
  };

  const columns = useMemo<ColumnsType<SkuCostLedger>>(
    () => [
      {
        title: 'SKU ID',
        dataIndex: 'sku_id'
      },
      {
        title: '原成本',
        dataIndex: 'previous_cost_cent',
        align: 'right',
        render: (value) => formatCurrency(value)
      },
      {
        title: '新成本',
        dataIndex: 'new_cost_cent',
        align: 'right',
        render: (value) => formatCurrency(value)
      },
      {
        title: '来源',
        dataIndex: 'ref_type',
        render: (value) => (value ? <Tag>{value}</Tag> : '-')
      },
      {
        title: '关联单号',
        dataIndex: 'ref_id',
        render: (value) => value ?? '-'
      },
      {
        title: '调整时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleSubmitAdjust = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    const values = await adjustForm.validateFields();
    if (!values.sku_id) {
      message.error('请选择 SKU');
      return;
    }
    if (values.new_cost == null) {
      message.error('请输入成本');
      return;
    }
    try {
      await adjustSkuCost(tenantId, {
        sku_id: values.sku_id,
        new_cost_cent: Math.round(values.new_cost * 100),
        ref_type: values.ref_type?.trim() || null,
        ref_id: values.ref_id?.trim() || null
      });
      message.success('成本已调整');
      setAdjustOpen(false);
      adjustForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['sku-cost-ledgers', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '调整失败');
    }
  };

  const showCostDisabled = tenantReady && !costFeature.enabled;

  return (
    <>
      <WorkbenchListPage
        query={
          <Card size="small">
            <Form form={form} layout="vertical" onFinish={handleSearch}>
              <Row gutter={16}>
                <Col xs={24}>
                  <Form.Item name="sku_id" label="SKU">
                    <Select
                      allowClear
                      showSearch
                      placeholder="搜索 SKU 编码/名称"
                      options={buildSkuOptions(skuOptions)}
                      filterOption={false}
                      onSearch={(value) => loadSkuOptions(value)}
                      onFocus={() => loadSkuOptions('')}
                      loading={skuSearching}
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="date_range" label="调整时间">
                    <DatePicker.RangePicker style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Space>
                    <Button type="primary" htmlType="submit" size="small">
                      查询
                    </Button>
                    <Button onClick={handleReset} size="small">
                      重置
                    </Button>
                  </Space>
                </Col>
              </Row>
            </Form>
          </Card>
        }
        toolbarLeft={
          <Space size={8} wrap={false} align="center">
            <Typography.Title level={4} style={{ margin: 0 }}>
              SKU成本流水
            </Typography.Title>
            <Button
              type="primary"
              size="small"
              disabled={!canWrite || !costFeature.enabled}
              onClick={() => setAdjustOpen(true)}
            >
              手工调整
            </Button>
          </Space>
        }
        toolbarRight={
          <UiModule02ToolbarRight
            onExport={() => message.info('导出功能待接入')}
            onPrint={() => message.info('打印功能待接入')}
            onHelp={() => message.info('帮助功能待接入')}
          />
        }
        table={
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            {showCostDisabled ? (
              <Alert
                type="warning"
                showIcon
                message="成本管理已关闭"
                description="请在系统设置中开启成本管理。"
              />
            ) : null}
            {costFeature.enabled ? (
              <ServerTable<SkuCostLedger>
                rowKey="ledger_id"
                columns={columns}
                data={costLedgerQuery.data?.items ?? []}
                loading={costLedgerQuery.isFetching}
                error={costLedgerQuery.error}
                onRetry={() => costLedgerQuery.refetch()}
                errorTitle="成本流水加载失败"
                emptyText="暂无成本流水"
                total={costLedgerQuery.data?.total ?? 0}
                pagination={pagination}
                onPaginationChange={setPagination}
                columnSettingsKey="skus.costs.table.columns"
              />
            ) : null}
          </Space>
        }
      />
      <Modal
        title="调整 SKU 成本"
        open={adjustOpen}
        onCancel={() => setAdjustOpen(false)}
        onOk={handleSubmitAdjust}
        okText="提交"
        destroyOnClose
        okButtonProps={{ disabled: !canWrite || !costFeature.enabled }}
      >
        <Form form={adjustForm} layout="vertical" initialValues={{ new_cost: 0 }}>
          <Form.Item
            name="sku_id"
            label="SKU"
            rules={[{ required: true, message: '请选择 SKU' }]}
          >
            <Select
              allowClear
              showSearch
              placeholder="搜索 SKU 编码/名称"
              options={buildSkuOptions(skuOptions)}
              filterOption={false}
              onSearch={(value) => loadSkuOptions(value)}
              onFocus={() => loadSkuOptions('')}
              loading={skuSearching}
            />
          </Form.Item>
          <Form.Item
            name="new_cost"
            label="新成本 (CNY)"
            rules={[{ required: true, message: '请输入成本' }]}
          >
            <InputNumber min={0} precision={2} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="ref_type" label="来源类型">
            <Input placeholder="MANUAL/PURCHASE" />
          </Form.Item>
          <Form.Item name="ref_id" label="关联单号">
            <Input placeholder="可选" />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
