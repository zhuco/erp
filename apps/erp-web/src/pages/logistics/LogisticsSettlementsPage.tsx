import {
  Button,
  Descriptions,
  Form,
  Input,
  InputNumber,
  Modal,
  Space,
  Table,
  Tag,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { LogisticsSettlementBatch, LogisticsSettlementRule } from '@/api/logisticsSettlements';
import {
  confirmLogisticsSettlementBatch,
  getLogisticsSettlementRule,
  listLogisticsSettlementBatches,
  updateLogisticsSettlementRule
} from '@/api/logisticsSettlements';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type SettlementStatus = 'DRAFT' | 'CONFIRMED' | 'PAID';

const formatCurrency = (value: number) => `¥${(value / 100).toFixed(2)}`;

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusLabels: Record<SettlementStatus, string> = {
  DRAFT: '待确认',
  CONFIRMED: '已确认',
  PAID: '已支付'
};

const statusColors: Record<SettlementStatus, string> = {
  DRAFT: 'orange',
  CONFIRMED: 'blue',
  PAID: 'green'
};

export const LogisticsSettlementsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [ruleModalOpen, setRuleModalOpen] = useState(false);
  const [detailModalOpen, setDetailModalOpen] = useState(false);
  const [activeBatch, setActiveBatch] = useState<LogisticsSettlementBatch | null>(null);
  const [form] = Form.useForm<LogisticsSettlementRule>();

  const ruleQuery = useQuery({
    queryKey: ['logistics-settlement-rule', tenantId],
    queryFn: () => getLogisticsSettlementRule(tenantId)
  });

  const batchesQuery = useQuery({
    queryKey: ['logistics-settlement-batches', tenantId],
    queryFn: () =>
      listLogisticsSettlementBatches(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const rule = ruleQuery.data;
  const batches = batchesQuery.data?.items ?? [];

  useEffect(() => {
    if (ruleModalOpen && rule) {
      form.setFieldsValue(rule);
    }
  }, [form, rule, ruleModalOpen]);

  const handleSaveRule = async () => {
    if (!canWrite) {
      message.warning('暂无权限');
      return;
    }
    try {
      const values = await form.validateFields();
      await updateLogisticsSettlementRule(tenantId, values);
      setRuleModalOpen(false);
      message.success('计费规则已更新');
      await queryClient.invalidateQueries({ queryKey: ['logistics-settlement-rule', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  };

  const handleConfirm = async (batch: LogisticsSettlementBatch) => {
    if (!canWrite) {
      return;
    }
    try {
      await confirmLogisticsSettlementBatch(tenantId, batch.batch_id, {});
      message.success('结算单已确认并生成付款计划');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-settlement-batches', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '确认失败');
    }
  };

  const handleExport = (batch: LogisticsSettlementBatch) => {
    const rows = [
      ['批次号', batch.batch_id],
      ['承运商', batch.carrier],
      ['账期开始', batch.period_start],
      ['账期结束', batch.period_end],
      ['运单总数', String(batch.total_shipments)],
      ['应付金额', formatCurrency(batch.expected_cent)],
      ['实付金额', formatCurrency(batch.actual_cent)],
      ['差异', formatCurrency(batch.diff_cent)]
    ];
    const csvContent = rows.map((row) => row.join(',')).join('\n');
    const blob = new Blob([`\uFEFF${csvContent}`], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${batch.batch_id}-settlement.csv`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const columns = useMemo<ColumnsType<LogisticsSettlementBatch>>(
    () => [
      { title: '结算批次', dataIndex: 'batch_id' },
      { title: '承运商', dataIndex: 'carrier' },
      {
        title: '账期',
        key: 'period',
        render: (_, record) => `${record.period_start} ~ ${record.period_end}`
      },
      { title: '运单量', dataIndex: 'total_shipments' },
      {
        title: '应付金额',
        dataIndex: 'expected_cent',
        render: (value: number) => formatCurrency(value)
      },
      {
        title: '实付金额',
        dataIndex: 'actual_cent',
        render: (value: number) => formatCurrency(value)
      },
      {
        title: '差异',
        dataIndex: 'diff_cent',
        render: (value: number) => (
          <Tag color={value === 0 ? 'green' : 'orange'}>{formatCurrency(value)}</Tag>
        )
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: SettlementStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        )
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8}>
            <Button
              size="small"
              onClick={() => {
                setActiveBatch(record);
                setDetailModalOpen(true);
              }}
            >
              查看
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.status !== 'DRAFT'}
              onClick={() => handleConfirm(record)}
            >
              确认结算
            </Button>
            <Button size="small" onClick={() => handleExport(record)}>
              导出
            </Button>
          </Space>
        )
      }
    ],
    [canWrite]
  );

  return (
    <>
      <UiModule07SettingsLayout title="物流费用结算">
        <UiModule07SettingsSection
          title="计费规则校验"
          actions={
            <Button type="primary" disabled={!canWrite} onClick={() => setRuleModalOpen(true)}>
              编辑规则
            </Button>
          }
        >
          <Descriptions size="small" bordered column={2}>
            <Descriptions.Item label="重量进位">
              {rule?.weight_rounding ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="最低费用">
              {formatCurrency(rule?.min_fee_cent ?? 0)}
            </Descriptions.Item>
            <Descriptions.Item label="超规格附加">
              {formatCurrency(rule?.oversize_fee_cent ?? 0)}
            </Descriptions.Item>
            <Descriptions.Item label="超时附加">
              {formatCurrency(rule?.overtime_fee_cent ?? 0)}
            </Descriptions.Item>
          </Descriptions>
        </UiModule07SettingsSection>
        <UiModule07SettingsSection title="结算批次">
          <Table
            rowKey="batch_id"
            columns={columns}
            dataSource={batches}
            loading={batchesQuery.isLoading}
          />
        </UiModule07SettingsSection>
      </UiModule07SettingsLayout>
      <Modal
        title="计费规则"
        open={ruleModalOpen}
        onCancel={() => setRuleModalOpen(false)}
        onOk={handleSaveRule}
        okText="保存"
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="weight_rounding"
            label="重量进位规则"
            rules={[{ required: true, message: '请输入规则' }]}
          >
            <Input placeholder="例如：按 0.5KG 进位" />
          </Form.Item>
          <Form.Item
            name="min_fee_cent"
            label="最低费用(分)"
            rules={[{ required: true, message: '请输入最低费用' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="oversize_fee_cent"
            label="超规格附加(分)"
            rules={[{ required: true, message: '请输入附加费' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="overtime_fee_cent"
            label="超时附加(分)"
            rules={[{ required: true, message: '请输入附加费' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="结算明细"
        open={detailModalOpen}
        onCancel={() => setDetailModalOpen(false)}
        footer={[
          <Button key="close" onClick={() => setDetailModalOpen(false)}>
            关闭
          </Button>
        ]}
      >
        {activeBatch ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions size="small" bordered column={1}>
              <Descriptions.Item label="结算批次">{activeBatch.batch_id}</Descriptions.Item>
              <Descriptions.Item label="承运商">{activeBatch.carrier}</Descriptions.Item>
              <Descriptions.Item label="账期">
                {activeBatch.period_start} ~ {activeBatch.period_end}
              </Descriptions.Item>
              <Descriptions.Item label="运单数量">
                {activeBatch.total_shipments}
              </Descriptions.Item>
              <Descriptions.Item label="差异">
                {formatCurrency(activeBatch.diff_cent)}
              </Descriptions.Item>
              <Descriptions.Item label="扣减记录">
                {activeBatch.deductions.length ? activeBatch.deductions.join('、') : '-'}
              </Descriptions.Item>
              <Descriptions.Item label="付款计划">
                {activeBatch.payment_plan_id ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="创建时间">
                {formatDateTime(activeBatch.created_at)}
              </Descriptions.Item>
            </Descriptions>
          </Space>
        ) : null}
      </Modal>
    </>
  );
};
