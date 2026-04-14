import { Button, Card, Descriptions, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type {
  AfterSalesItem,
  AfterSalesDetail,
  AfterSalesAuditLog,
  AfterSalesShipment,
  AfterSalesStatus,
  AfterSalesType
} from '@/api/afterSalesOps';
import {
  getAfterSalesDetail,
  approveAfterSales,
  rejectAfterSales,
  requestAfterSalesRefund,
  createAfterSalesExchangeShipment,
  createAfterSalesResendShipment,
  cancelAfterSalesReceive,
  putawayAfterSales
} from '@/api/afterSalesOps';
import { UiModule07SettingsLayout } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { getAfterSalesActionFlags } from './afterSalesActionGuards';

const statusLabelMap: Record<AfterSalesStatus, string> = {
  OPEN: '待审核',
  APPROVED: '已同意',
  RECEIVED: '已收货',
  REFUNDED: '已退款',
  CLOSED: '已关闭',
  REJECTED: '已驳回'
};

const statusColorMap: Record<AfterSalesStatus, string> = {
  OPEN: 'orange',
  APPROVED: 'blue',
  RECEIVED: 'purple',
  REFUNDED: 'green',
  CLOSED: 'default',
  REJECTED: 'red'
};

const typeLabelMap: Record<AfterSalesType, string> = {
  REFUND_ONLY: '仅退款',
  RETURN_REFUND: '退货退款',
  EXCHANGE: '换货',
  RESEND: '补发'
};

const typeColorMap: Record<AfterSalesType, string> = {
  REFUND_ONLY: 'gold',
  RETURN_REFUND: 'cyan',
  EXCHANGE: 'blue',
  RESEND: 'purple'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const slaLabelMap: Record<string, string> = {
  ON_TRACK: '进行中',
  DUE_SOON: '临近超时',
  OVERDUE: '已超时',
  CLOSED: '已关闭'
};

const formatSlaSummary = (detail: AfterSalesDetail) => {
  const status = detail.after_sales.sla_status;
  const remaining = detail.after_sales.sla_remaining_minutes;
  const statusLabel = status ? slaLabelMap[status] ?? status : '未配置';
  const remainingLabel =
    typeof remaining === 'number'
      ? remaining >= 0
        ? `${remaining} 分钟`
        : `超时 ${Math.abs(remaining)} 分钟`
      : '未计算';
  const roleLabel = detail.after_sales.escalation_role ?? '未升级';
  return `${statusLabel} / ${remainingLabel} / ${roleLabel}`;
};

const formatReceiveSummary = (detail: AfterSalesDetail) => {
  const receipt = detail.return_receipt;
  if (!receipt) {
    return '未登记';
  }
  return `${receipt.status} / ${receipt.warehouse_id}`;
};

const formatRefundSummary = (detail: AfterSalesDetail) => {
  const refundDoc = detail.refund_doc;
  if (!refundDoc) {
    return '未生成退款单';
  }
  return `${refundDoc.status} / ${refundDoc.channel}`;
};

const formatShipmentSummary = (detail: AfterSalesDetail) => {
  const shipmentCount = detail.shipments?.length ?? 0;
  if (shipmentCount > 0) {
    return `已生成 ${shipmentCount} 个包裹`;
  }
  if (detail.after_sales.type === 'EXCHANGE' || detail.after_sales.type === 'RESEND') {
    return '待生成包裹';
  }
  return '不适用';
};

export const AfterSalesDetailPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const navigate = useNavigate();
  const { after_sales_id } = useParams<{ after_sales_id: string }>();
  const [putawaying, setPutawaying] = useState(false);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['after-sales', tenantId, after_sales_id],
    queryFn: () => getAfterSalesDetail(tenantId, after_sales_id ?? ''),
    enabled: Boolean(after_sales_id)
  });

  const columns = useMemo<ColumnsType<AfterSalesItem>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: '数量', dataIndex: 'qty' },
      {
        title: '退款金额',
        dataIndex: 'refund_cent',
        render: (value: number) => `¥${(value / 100).toFixed(2)}`
      }
    ],
    []
  );

  const auditColumns = useMemo<ColumnsType<AfterSalesAuditLog>>(
    () => [
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      { title: '动作', dataIndex: 'action' },
      {
        title: '操作人',
        dataIndex: 'user_id',
        render: (value?: string | null) => value || '-'
      },
      {
        title: '变更',
        key: 'payload',
        render: (_, record) => (
          <pre style={{ margin: 0, whiteSpace: 'pre-wrap' }}>
            {JSON.stringify(
              {
                before: record.before_json ?? null,
                after: record.after_json ?? null
              },
              null,
              2
            )}
          </pre>
        )
      }
    ],
    []
  );

  const shipmentColumns = useMemo<ColumnsType<AfterSalesShipment>>(
    () => [
      { title: '发货单号', dataIndex: 'shipment_id' },
      { title: '类型', dataIndex: 'shipment_type' },
      {
        title: '原发货单',
        dataIndex: 'origin_shipment_id',
        render: (value?: string | null) => value || '-'
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
  );

  const handleApprove = async (detail: AfterSalesDetail) => {
    try {
      await approveAfterSales(tenantId, detail.after_sales.id);
      message.success('售后已同意');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const handleReject = async (detail: AfterSalesDetail) => {
    try {
      await rejectAfterSales(tenantId, detail.after_sales.id);
      message.success('售后已驳回');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const handleRefund = async (detail: AfterSalesDetail) => {
    try {
      await requestAfterSalesRefund(tenantId, detail.after_sales.id, {
        amount_cent: detail.after_sales.amount_cent,
        channel: 'MANUAL'
      });
      message.success('已发起退款');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const handleCreateShipment = async (detail: AfterSalesDetail) => {
    try {
      if (detail.after_sales.type === 'EXCHANGE') {
        await createAfterSalesExchangeShipment(tenantId, detail.after_sales.id, {});
        message.success('换货包裹已生成');
      } else if (detail.after_sales.type === 'RESEND') {
        await createAfterSalesResendShipment(tenantId, detail.after_sales.id, {});
        message.success('补发包裹已生成');
      }
      navigate('/oms/shipments');
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const handleCancelReceive = async (detail: AfterSalesDetail) => {
    try {
      await cancelAfterSalesReceive(tenantId, detail.after_sales.id);
      message.success('退货入库已撤销');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const handlePutaway = async (detail: AfterSalesDetail) => {
    if (!detail.return_receipt) {
      message.warning('请先登记退货收货');
      return;
    }
    if (detail.return_receipt.status === 'PUTAWAY_DONE') {
      message.info('库存已回补');
      return;
    }
    setPutawaying(true);
    try {
      await putawayAfterSales(tenantId, detail.after_sales.id);
      message.success('库存回补已完成');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    } finally {
      setPutawaying(false);
    }
  };

  if (!data && (isLoading || !after_sales_id)) {
    return <Typography.Text>加载中...</Typography.Text>;
  }

  if (error || !data) {
    return (
      <Space direction="vertical">
        <Typography.Text type="danger">售后详情加载失败</Typography.Text>
      </Space>
    );
  }

  const detail = data;
  const { after_sales: afterSales, return_receipt: receipt, refund_doc: refundDoc } = detail;
  const actionFlags = getAfterSalesActionFlags({
    status: afterSales.status,
    type: afterSales.type,
    sla_status: afterSales.sla_status,
    return_receipt: receipt,
    canWrite
  });
  const latestAuditLog = detail.audit_logs?.[0];
  return (
    <UiModule07SettingsLayout>
      <Space align="center" style={{ justifyContent: 'space-between', width: '100%' }}>
        <Typography.Title level={3} data-testid="after-sales-detail-title">
          售后详情
        </Typography.Title>
        <Space>
          <Button onClick={() => navigate('/after-sales')}>返回列表</Button>
          {actionFlags.canApprove && (
            <>
              <Button type="primary" onClick={() => handleApprove(detail)}>
                同意
              </Button>
              <Button danger onClick={() => handleReject(detail)}>
                驳回
              </Button>
            </>
          )}
          {actionFlags.canReceive && (
            <Button onClick={() => navigate(`/after-sales/${afterSales.id}/receive`)}>
              收货
            </Button>
          )}
          {actionFlags.canCancelReceive && (
            <Button danger onClick={() => handleCancelReceive(detail)}>
              撤销入库
            </Button>
          )}
          {actionFlags.canRedoReceive && (
            <Button onClick={() => navigate(`/after-sales/${afterSales.id}/receive`)}>
              重做收货
            </Button>
          )}
          {actionFlags.canPutaway && (
            <Button loading={putawaying} onClick={() => handlePutaway(detail)}>
              库存回补
            </Button>
          )}
          {actionFlags.canCreateShipment && (
            <Button type="dashed" onClick={() => handleCreateShipment(detail)}>
              {afterSales.type === 'EXCHANGE' ? '生成换货包裹' : '生成补发包裹'}
            </Button>
          )}
          {actionFlags.canRefund && (
            <Button type="dashed" onClick={() => handleRefund(detail)}>
              退款
            </Button>
          )}
        </Space>
      </Space>
      <Card>
        <Descriptions column={2} bordered size="small">
          <Descriptions.Item label="售后单号">{afterSales.id}</Descriptions.Item>
          <Descriptions.Item label="订单号">
            <Link to={`/orders/${afterSales.order_id}`}>{afterSales.order_id}</Link>
          </Descriptions.Item>
          <Descriptions.Item label="类型">
            <Tag color={typeColorMap[afterSales.type] ?? 'default'}>
              {typeLabelMap[afterSales.type] ?? afterSales.type}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="状态">
            <Tag color={statusColorMap[afterSales.status] ?? 'default'}>
              {statusLabelMap[afterSales.status] ?? afterSales.status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="原因">{afterSales.reason_code ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="退款金额">
            ¥{(afterSales.amount_cent / 100).toFixed(2)}
          </Descriptions.Item>
          <Descriptions.Item label="创建时间">
            {formatDateTime(afterSales.created_at)}
          </Descriptions.Item>
          <Descriptions.Item label="更新时间">
            {formatDateTime(afterSales.updated_at)}
          </Descriptions.Item>
        </Descriptions>
      </Card>
      <Card title="工业级闭环证据">
        <Descriptions
          column={2}
          bordered
          size="small"
          data-testid="after-sales-closure-evidence"
        >
          <Descriptions.Item label="闭环阶段">
            {statusLabelMap[afterSales.status] ?? afterSales.status}
          </Descriptions.Item>
          <Descriptions.Item label="售后类型">
            {typeLabelMap[afterSales.type] ?? afterSales.type}
          </Descriptions.Item>
          <Descriptions.Item label="退货收货">
            {formatReceiveSummary(detail)}
          </Descriptions.Item>
          <Descriptions.Item label="退款财务">
            {formatRefundSummary(detail)}
          </Descriptions.Item>
          <Descriptions.Item label="换补发运">
            {formatShipmentSummary(detail)}
          </Descriptions.Item>
          <Descriptions.Item label="SLA/升级">
            {formatSlaSummary(detail)}
          </Descriptions.Item>
          <Descriptions.Item label="审计记录">
            {`${detail.audit_logs?.length ?? 0} 条`}
          </Descriptions.Item>
          <Descriptions.Item label="最近动作">
            {latestAuditLog
              ? `${latestAuditLog.action} / ${formatDateTime(latestAuditLog.created_at)}`
              : '-'}
          </Descriptions.Item>
        </Descriptions>
      </Card>
      <Card title="售后明细">
        <Table
          rowKey="sku_id"
          columns={columns}
          dataSource={detail.items}
          pagination={false}
        />
      </Card>
      <Card title="退货收货">
        {receipt ? (
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label="收货单号">{receipt.id}</Descriptions.Item>
            <Descriptions.Item label="仓库">{receipt.warehouse_id}</Descriptions.Item>
            <Descriptions.Item label="状态">{receipt.status}</Descriptions.Item>
            <Descriptions.Item label="收货时间">{formatDateTime(receipt.received_at)}</Descriptions.Item>
            <Descriptions.Item label="上架时间">{formatDateTime(receipt.putaway_at)}</Descriptions.Item>
          </Descriptions>
        ) : (
          <Typography.Text type="secondary">暂无收货记录</Typography.Text>
        )}
      </Card>
      <Card title="换货/补发发货记录">
        {detail.shipments && detail.shipments.length > 0 ? (
          <Table
            rowKey="shipment_id"
            columns={shipmentColumns}
            dataSource={detail.shipments}
            pagination={false}
          />
        ) : (
          <Typography.Text type="secondary">暂无发货记录</Typography.Text>
        )}
      </Card>
      <Card title="退款信息" extra={<Link to="/refund-docs">退款处理</Link>}>
        {refundDoc ? (
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label="退款单号">{refundDoc.id}</Descriptions.Item>
            <Descriptions.Item label="状态">{refundDoc.status}</Descriptions.Item>
            <Descriptions.Item label="渠道">{refundDoc.channel}</Descriptions.Item>
            <Descriptions.Item label="退款金额">
              ¥{(refundDoc.amount_cent / 100).toFixed(2)}
            </Descriptions.Item>
            <Descriptions.Item label="创建时间">
              {formatDateTime(refundDoc.created_at)}
            </Descriptions.Item>
          </Descriptions>
        ) : (
          <Typography.Text type="secondary">暂无退款记录</Typography.Text>
        )}
      </Card>
      <Card title="原始数据">
        {detail.raw_json ? (
          <pre style={{ margin: 0, whiteSpace: 'pre-wrap' }}>
            {JSON.stringify(detail.raw_json, null, 2)}
          </pre>
        ) : (
          <Typography.Text type="secondary">暂无原始数据</Typography.Text>
        )}
      </Card>
      <Card title="处理记录">
        {detail.audit_logs && detail.audit_logs.length > 0 ? (
          <Table
            rowKey="audit_id"
            columns={auditColumns}
            dataSource={detail.audit_logs}
            pagination={false}
          />
        ) : (
          <Typography.Text type="secondary">暂无处理记录</Typography.Text>
        )}
      </Card>
    </UiModule07SettingsLayout>
  );
};
