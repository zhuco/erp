import { Button, Space, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { ReactNode } from 'react';

import type { OrderSummary } from '@/api/orders';
import { PlatformTag } from '@/components/PlatformTag';

type OrderStatusMeta = Record<string, { label: string; color: string }>;

export type OrderManagementColumnOptions = {
  rowOffset: number;
  orderStatusMeta: OrderStatusMeta;
  formatCurrency: (value?: number | null) => string;
  formatDateTime: (value?: string | null) => string;
  formatDuration?: (value?: string | null) => string;
  resolveOrderNo: (record: OrderSummary) => ReactNode;
  resolveShopName: (record: OrderSummary) => ReactNode;
  resolveWarehouseName: (record: OrderSummary) => ReactNode;
  resolveItemSummary: (record: OrderSummary) => ReactNode;
  resolveItemQuantity: (record: OrderSummary) => ReactNode;
  resolveReceiverAddress: (record: OrderSummary) => ReactNode;
  resolveRemark: (record: OrderSummary) => ReactNode;
  resolveOrderSource: (record: OrderSummary) => ReactNode;
  resolveLogisticsCompany: (record: OrderSummary) => ReactNode;
  resolveTags: (record: OrderSummary) => ReactNode;
  resolveEstimatedPostage?: (record: OrderSummary) => number | null;
  resolveEstimatedProfit?: (record: OrderSummary) => number | null;
  resolveEstimatedProfitRate?: (record: OrderSummary) => number | null;
  includeTags?: boolean;
  includeExceptions?: boolean;
  includeProcessingTime?: boolean;
  includeRemark?: boolean;
  includeOrderSource?: boolean;
  includeEstimatedProfit?: boolean;
  includeTrackingNo?: boolean;
  includeActions?: boolean;
  onViewDetail?: (record: OrderSummary) => void;
};

export const buildOrderManagementColumns = ({
  rowOffset,
  orderStatusMeta,
  formatCurrency,
  formatDateTime,
  formatDuration,
  resolveOrderNo,
  resolveShopName,
  resolveWarehouseName,
  resolveItemSummary,
  resolveItemQuantity,
  resolveReceiverAddress,
  resolveRemark,
  resolveOrderSource,
  resolveLogisticsCompany,
  resolveTags,
  resolveEstimatedPostage,
  resolveEstimatedProfit,
  resolveEstimatedProfitRate,
  includeTags = true,
  includeExceptions = false,
  includeProcessingTime = true,
  includeRemark = true,
  includeOrderSource = true,
  includeEstimatedProfit = true,
  includeTrackingNo = false,
  includeActions = false,
  onViewDetail
}: OrderManagementColumnOptions): ColumnsType<OrderSummary> => {
  const processingFormatter = formatDuration ?? formatDateTime;

  const columnList: Array<ColumnsType<OrderSummary>[number] | null> = [
    {
      title: '',
      key: 'row_index',
      width: 40,
      render: (_, __, index) => rowOffset + index + 1
    },
    includeTags
      ? {
          title: '标记',
          key: 'tags',
          width: 120,
          render: (_, record) => resolveTags(record)
        }
      : null,
    {
      title: '订单编号',
      dataIndex: 'id',
      width: 170,
      render: (_, record) => resolveOrderNo(record)
    },
    {
      title: '订单状态',
      dataIndex: 'oms_status',
      width: 120,
      render: (_, record) => {
        const meta = orderStatusMeta[record.oms_status ?? ''] ?? {
          label: record.oms_status ?? '-',
          color: 'default'
        };
        return <Tag color={meta.color}>{meta.label}</Tag>;
      }
    },
    includeExceptions
      ? {
          title: '异常',
          key: 'exceptions',
          width: 120,
          render: (_, record) => {
            const flags = record.exception_flags ?? [];
            const tags: ReactNode[] = [];
            if (record.mapping_missing) {
              tags.push(
                <Tag key="mapping" color="red">
                  无效货品
                </Tag>
              );
            }
            if (flags.includes('ORDER_FROZEN')) {
              tags.push(
                <Tag key="frozen" color="volcano">
                  冻结
                </Tag>
              );
            }
            if (flags.includes('ORDER_MERGED')) {
              tags.push(
                <Tag key="merged" color="blue">
                  合并
                </Tag>
              );
            }
            return tags.length ? (
              <Space size={[4, 4]} wrap>
                {tags}
              </Space>
            ) : (
              '-'
            );
          }
        }
      : null,
    {
      title: '销售渠道',
      dataIndex: 'shop_id',
      width: 160,
      render: (_, record) => resolveShopName(record)
    },
    {
      title: '销售平台',
      dataIndex: 'platform',
      width: 120,
      render: (_, record) => <PlatformTag platform={record.platform} />
    },
    includeProcessingTime
      ? {
          title: '处理时间',
          dataIndex: 'created_at',
          key: 'processing_time',
          width: 140,
          render: (_, record) => processingFormatter(record.created_at)
        }
      : null,
    {
      title: '付款时间',
      dataIndex: 'created_at',
      width: 160,
      render: (_, record) => formatDateTime(record.created_at)
    },
    {
      title: '发货仓库',
      dataIndex: 'warehouse_name',
      width: 140,
      render: (_, record) => resolveWarehouseName(record)
    },
    {
      title: '货品信息',
      dataIndex: 'items',
      width: 220,
      render: (_, record) => resolveItemSummary(record)
    },
    {
      title: '应收合计',
      dataIndex: 'total_amount_cent',
      width: 120,
      render: (value) => formatCurrency(value)
    },
    {
      title: '货品数量',
      key: 'goods_qty',
      width: 100,
      render: (_, record) => {
        const qty = resolveItemQuantity(record);
        return qty == null || qty === '' ? '-' : qty;
      }
    },
    {
      title: '客户帐号',
      dataIndex: 'customer_name',
      key: 'customer_account',
      width: 140,
      render: (_, record) => record.customer_name ?? record.customer_id ?? '-'
    },
    {
      title: '收货人',
      dataIndex: 'receiver_name',
      width: 120,
      render: (value) => value ?? '-'
    },
    {
      title: '手机',
      dataIndex: 'receiver_phone',
      width: 140,
      render: (value) => value ?? '-'
    },
    {
      title: '收货地址',
      dataIndex: 'receiver_address',
      width: 220,
      render: (_, record) => resolveReceiverAddress(record)
    },
    includeRemark
      ? {
          title: '备注',
          dataIndex: 'remark',
          width: 160,
          render: (_, record) => resolveRemark(record)
        }
      : null,
    includeOrderSource
      ? {
          title: '订单来源',
          dataIndex: 'order_source',
          width: 120,
          render: (_, record) => resolveOrderSource(record)
        }
      : null,
    {
      title: '物流公司',
      dataIndex: 'logistics_company',
      width: 120,
      render: (_, record) => resolveLogisticsCompany(record)
    },
    includeTrackingNo
      ? {
          title: '物流单号',
          dataIndex: 'tracking_no',
          width: 140,
          render: (value) => value ?? '-'
        }
      : null,
    includeEstimatedProfit
      ? {
          title: '预估邮费',
          key: 'estimated_postage',
          width: 120,
          render: (_, record) =>
            formatCurrency(resolveEstimatedPostage?.(record) ?? null)
        }
      : null,
    includeEstimatedProfit
      ? {
          title: '预估毛利',
          key: 'estimated_profit',
          width: 120,
          render: (_, record) => {
            const profit = resolveEstimatedProfit?.(record) ?? null;
            return profit == null ? '-' : formatCurrency(profit);
          }
        }
      : null,
    includeEstimatedProfit
      ? {
          title: '预估毛利率',
          key: 'estimated_profit_rate',
          width: 120,
          render: (_, record) => {
            const rate = resolveEstimatedProfitRate?.(record) ?? null;
            return rate == null ? '-' : `${(rate * 100).toFixed(2)}%`;
          }
        }
      : null,
    {
      title: '网店订单号',
      dataIndex: 'platform_order_id',
      width: 180,
      render: (value) => value ?? '-'
    },
    includeActions && onViewDetail
      ? {
          title: '操作',
          key: 'actions',
          width: 80,
          render: (_, record) => (
            <Button
              type="link"
              size="small"
              data-testid="order-detail-link"
              onClick={(event) => {
                event.stopPropagation();
                onViewDetail(record);
              }}
            >
              详情
            </Button>
          )
        }
      : null
  ];

  return columnList.filter((column): column is ColumnsType<OrderSummary>[number] =>
    Boolean(column)
  );
};
