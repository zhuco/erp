import { Button, Card, Descriptions, Modal, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { AfterSaleCaseDetail } from '@/api/afterSales';
import { getAfterSaleCaseDetail, getAfterSaleCaseRaw } from '@/api/afterSales';
import { PlatformTag } from '@/components/PlatformTag';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { UiModule07SettingsLayout } from '@/components/ui-modules';
import { hasPermission } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';

type AfterSaleCaseItem = AfterSaleCaseDetail['items'][number];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveRefundAmount = (value?: number | null, fallback?: number | null) => {
  if (value != null) {
    return value / 100;
  }
  if (fallback != null) {
    return fallback;
  }
  return null;
};

export const AfterSaleCaseDetailPage = () => {
  const { case_id } = useParams();
  const navigate = useNavigate();
  const tenantId = useTenantId();
  const permissions = useSessionStore((state) => state.permissions);
  const canReadRaw = hasPermission(permissions, 'aftersales.read_raw');
  const [rawVisible, setRawVisible] = useState(false);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sale-case', tenantId, case_id],
    queryFn: () => getAfterSaleCaseDetail(tenantId, case_id ?? ''),
    enabled: Boolean(case_id)
  });

  const rawQuery = useQuery({
    queryKey: ['after-sale-case-raw', tenantId, case_id],
    queryFn: () => getAfterSaleCaseRaw(tenantId, case_id ?? ''),
    enabled: false
  });

  const rawSummary = useMemo(() => {
    if (!data) {
      return '-';
    }
    const caseInfo = data.case;
    return JSON.stringify(
      {
        case_id: caseInfo.case_id,
        platform: caseInfo.platform,
        shop_id: caseInfo.shop_id,
        case_status: caseInfo.case_status ?? caseInfo.after_sale_state,
        case_type: caseInfo.case_type ?? caseInfo.after_sale_type,
        platform_order_id: caseInfo.platform_order_id,
        refund_amount:
          resolveRefundAmount(caseInfo.refund_amount_cent, caseInfo.refund_amount) ?? null,
        item_count: caseInfo.item_count ?? data.items.length,
        updated_at: caseInfo.updated_at
      },
      null,
      2
    );
  }, [data]);

  const caseLoading = (isLoading || isFetching) && !data;

  const columns = useMemo<ColumnsType<AfterSaleCaseItem>>(
    () => [
      {
        title: '订单/子单',
        dataIndex: 'order_id',
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <span>订单: {record.order_id ?? '-'}</span>
            <span>子单: {record.line_id ?? record.item_id ?? '-'}</span>
          </Space>
        )
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (_, record) => record.sku_id ?? '-'
      },
      {
        title: '商品名称',
        dataIndex: 'title',
        render: (_, record) => record.title ?? '-'
      },
      {
        title: '数量',
        dataIndex: 'qty',
        render: (_, record) => record.qty ?? '-'
      },
      {
        title: '退款金额',
        dataIndex: 'refund_amount_cent',
        render: (_, record) => {
          const amount = resolveRefundAmount(record.refund_amount_cent, record.refund_amount);
          return amount == null ? '-' : `¥${amount.toFixed(2)}`;
        }
      },
      {
        title: '售后状态',
        dataIndex: 'after_sale_state',
        render: (_, record) => record.after_sale_state ?? '-'
      },
      {
        title: '退款状态',
        dataIndex: 'refund_state',
        render: (_, record) => record.refund_state ?? '-'
      },
      {
        title: '已入库',
        dataIndex: 'return_stocked',
        render: (_, record) =>
          record.return_stocked == null ? '-' : (
            <Tag color={record.return_stocked ? 'green' : 'default'}>
              {record.return_stocked ? '是' : '否'}
            </Tag>
          )
      },
      {
        title: '入库仓',
        dataIndex: 'return_warehouse_id',
        render: (_, record) => record.return_warehouse_id ?? '-'
      },
      {
        title: '入库时间',
        dataIndex: 'return_at',
        render: (_, record) => formatDateTime(record.return_at)
      }
    ],
    []
  );

  if (!case_id) {
    return (
      <ErrorState
        title="缺少售后单号"
        description="无法加载售后详情"
        hint="请返回列表重新选择售后单。"
        action={<Button onClick={() => navigate('/after-sale-cases')}>返回列表</Button>}
      />
    );
  }

  if (error) {
    return <ErrorState title="售后详情加载失败" error={error} onRetry={() => refetch()} />;
  }

  const caseInfo = data?.case;
  const refundAmount = resolveRefundAmount(caseInfo?.refund_amount_cent, caseInfo?.refund_amount);

  return (
    <UiModule07SettingsLayout>
      <Space align="center" size={12}>
        <Button onClick={() => navigate('/after-sale-cases')}>返回列表</Button>
        <Typography.Title level={3} style={{ margin: 0 }}>
          售后 Case 详情
        </Typography.Title>
      </Space>
      <LoadingState loading={caseLoading} onRetry={() => refetch()} />
      {caseInfo ? (
        <Card>
          <Descriptions column={3} bordered size="small">
            <Descriptions.Item label="平台售后单号">
              {caseInfo.platform_case_id ?? caseInfo.after_sale_id ?? caseInfo.case_id}
            </Descriptions.Item>
            <Descriptions.Item label="平台">
              <PlatformTag platform={caseInfo.platform} />
            </Descriptions.Item>
            <Descriptions.Item label="店铺">{caseInfo.shop_id}</Descriptions.Item>
            <Descriptions.Item label="售后类型">
              {caseInfo.case_type ?? caseInfo.after_sale_type ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="售后状态">
              {caseInfo.case_status ?? caseInfo.after_sale_state ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="退款状态">
              {caseInfo.refund_state ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="平台订单号">
              {caseInfo.platform_order_id ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="退款金额">
              {refundAmount == null ? '-' : `¥${refundAmount.toFixed(2)}`}
            </Descriptions.Item>
            <Descriptions.Item label="子单数">
              {caseInfo.item_count ?? data?.items.length ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="订单数">
              {caseInfo.order_count ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="总件数">
              {caseInfo.total_qty ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="更新时间">
              {formatDateTime(caseInfo.updated_at ?? caseInfo.created_at)}
            </Descriptions.Item>
          </Descriptions>
        </Card>
      ) : null}
      <Card
        title="Raw 摘要"
        extra={
          canReadRaw ? (
            <Button
              onClick={() => {
                setRawVisible(true);
                if (!rawQuery.data) {
                  rawQuery.refetch();
                }
              }}
            >
              查看 raw
            </Button>
          ) : null
        }
      >
        <Typography.Paragraph code style={{ whiteSpace: 'pre-wrap' }}>
          {rawSummary}
        </Typography.Paragraph>
      </Card>
      <Card title="关联订单与子单">
        <Table<AfterSaleCaseItem>
          rowKey={(record) => record.line_id ?? record.item_id ?? record.order_id ?? record.case_id}
          columns={columns}
          dataSource={data?.items ?? []}
          pagination={false}
        />
      </Card>
      <Modal
        title="Raw 数据"
        open={rawVisible}
        onCancel={() => setRawVisible(false)}
        footer={null}
        width={860}
      >
        {rawQuery.isFetching ? (
          <LoadingState loading={rawQuery.isFetching} onRetry={() => rawQuery.refetch()} />
        ) : rawQuery.error ? (
          <ErrorState
            title="Raw 数据加载失败"
            error={rawQuery.error}
            onRetry={() => rawQuery.refetch()}
          />
        ) : (
          <Typography.Paragraph code style={{ whiteSpace: 'pre-wrap' }}>
            {JSON.stringify(rawQuery.data ?? {}, null, 2)}
          </Typography.Paragraph>
        )}
      </Modal>
    </UiModule07SettingsLayout>
  );
};
