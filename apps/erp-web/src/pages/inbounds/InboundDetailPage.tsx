import { Button, Card, Descriptions, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem, InboundDocStatus, InboundInspection } from '@/api/inbounds';
import { getInboundDetail, listInboundInspections } from '@/api/inbounds';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const statusColors: Record<InboundDocStatus, string> = {
  DRAFT: 'default',
  AUDITED: 'gold',
  RECEIVING: 'blue',
  PUTAWAY_DONE: 'green',
  CANCELLED: 'red'
};

const traceStatusTextMap: Record<string, string> = {
  NOT_RECEIVED: '未收货',
  PART_RECEIVED: '閮ㄥ垎鏀惰揣',
  RECEIVED: '已收货',
  NOT_REQUIRED: '鍏嶆',
  PENDING: '寰呰川妫€',
  PASSED: '鍚堟牸',
  FAILED: '不合格',
  PENDING_REVIEW: '寰呭妫€',
  NOT_PUTAWAY: '未上架',
  PART_PUTAWAY: '閮ㄥ垎涓婃灦',
  PUTAWAYED: '已上架'
};

const traceStatusColorMap: Record<string, string> = {
  NOT_RECEIVED: 'default',
  PART_RECEIVED: 'blue',
  RECEIVED: 'green',
  NOT_REQUIRED: 'default',
  PENDING: 'gold',
  PASSED: 'green',
  FAILED: 'red',
  PENDING_REVIEW: 'orange',
  NOT_PUTAWAY: 'default',
  PART_PUTAWAY: 'blue',
  PUTAWAYED: 'green'
};

type InboundTraceDoc = {
  receipt_status?: string | null;
  inbound_qc_status?: string | null;
  putaway_status?: string | null;
  inbound_status?: string | null;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const InboundDetailPage = () => {
  const tenantId = useTenantId();
  const { doc_id } = useParams<{ doc_id: string }>();
  const navigate = useNavigate();
  const canWrite = useCan('wms.inbounds.write');
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');

  const detailQuery = useQuery({
    queryKey: ['inbound-detail', tenantId, doc_id],
    queryFn: () => getInboundDetail(tenantId, doc_id ?? ''),
    enabled: tenantReady && Boolean(doc_id) && inventoryFeature.enabled,
    retry: false
  });

  const inspectionQuery = useQuery({
    queryKey: ['inbound-inspections', tenantId, doc_id],
    queryFn: () => listInboundInspections(tenantId, doc_id ?? ''),
    enabled: tenantReady && Boolean(doc_id) && inventoryFeature.enabled,
    retry: false
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inbound-detail'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const detail = detailQuery.data;

  const columns = useMemo<ColumnsType<InboundDocItem>>(
    () => [
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: 'SKU 缂栫爜', dataIndex: 'sku_code' },
      { title: '鏉＄爜', dataIndex: 'barcode' },
      { title: '璁″垝鏁伴噺', dataIndex: 'planned_qty' },
      { title: '已收货', dataIndex: 'received_qty' },
      { title: '已上架', dataIndex: 'putaway_qty' }
    ],
    []
  );

  const inspectionColumns = useMemo<ColumnsType<InboundInspection>>(
    () => [
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: '鏁伴噺', dataIndex: 'qty' },
      {
        title: '缁撴灉',
        dataIndex: 'result',
        render: (value: string) =>
          value === 'PASSED' ? '合格' : value === 'FAILED' ? '不合格' : '待复检'
      },
      { title: '澶囨敞', dataIndex: 'note', render: (value) => value ?? '-' },
      {
        title: '璐ㄦ鏃堕棿',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
  );

  if (!doc_id) {
    message.error('缂哄皯鍏ュ簱鍗?ID');
  }

  if (detailQuery.error) {
    return (
      <ErrorState
        title="入库单加载失败"
        error={detailQuery.error}
        onRetry={() => detailQuery.refetch()}
      />
    );
  }

  if (!detail) {
    return null;
  }

  const statusTag = (
    <Tag color={statusColors[detail.doc.status] || 'default'}>{detail.doc.status}</Tag>
  );
  const traceDoc = detail.doc as InboundTraceDoc;
  const renderTraceStatus = (value?: string | null) => {
    if (!value) {
      return '-';
    }
    return (
      <Tag color={traceStatusColorMap[value] || 'default'}>
        {traceStatusTextMap[value] ?? value}
      </Tag>
    );
  };

  const canOperate =
    canWrite && detail.doc.status !== 'PUTAWAY_DONE' && detail.doc.status !== 'CANCELLED';

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} data-testid="inbound-detail-title" style={{ margin: 0 }}>
        鍏ュ簱鍗曡鎯?
      </Typography.Title>
      <Button data-testid="inbound-detail-back" onClick={() => navigate('/wms/inbounds')}>
        杩斿洖鍒楄〃
      </Button>
      <Button
        data-testid="inbound-detail-receive"
        disabled={!canOperate}
        onClick={() => navigate(`/wms/inbounds/${doc_id}/receive`)}
      >
        鏀惰揣
      </Button>
      <Button
        data-testid="inbound-detail-inspect"
        disabled={!canOperate}
        onClick={() => navigate(`/wms/inbounds/${doc_id}/inspect`)}
      >
        璐ㄦ
      </Button>
      <Button
        data-testid="inbound-detail-putaway"
        disabled={!canOperate}
        onClick={() => navigate(`/wms/inbounds/${doc_id}/putaway`)}
      >
        涓婃灦
      </Button>
    </Space>
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Descriptions bordered size="small" column={2}>
        <Descriptions.Item label="鍏ュ簱鍗曞彿">{detail.doc.doc_no}</Descriptions.Item>
        <Descriptions.Item label="状态">{statusTag}</Descriptions.Item>
        <Descriptions.Item label="鍏ュ簱杩涘害">{renderTraceStatus(traceDoc.inbound_status)}</Descriptions.Item>
        <Descriptions.Item label="收货状态">{renderTraceStatus(traceDoc.receipt_status)}</Descriptions.Item>
        <Descriptions.Item label="质检状态">{renderTraceStatus(traceDoc.inbound_qc_status)}</Descriptions.Item>
        <Descriptions.Item label="上架状态">{renderTraceStatus(traceDoc.putaway_status)}</Descriptions.Item>
        <Descriptions.Item label="浠撳簱">
          {warehouseMap.get(detail.doc.warehouse_id) ?? detail.doc.warehouse_id}
        </Descriptions.Item>
        <Descriptions.Item label="绫诲瀷">{detail.doc.doc_type}</Descriptions.Item>
        <Descriptions.Item label="鏉ユ簮绫诲瀷">{detail.doc.ref_type ?? '-'}</Descriptions.Item>
        <Descriptions.Item label="鏉ユ簮鍗曞彿">{detail.doc.ref_id ?? '-'}</Descriptions.Item>
        <Descriptions.Item label="鍒涘缓鏃堕棿">
          {formatDateTime(detail.doc.created_at)}
        </Descriptions.Item>
        <Descriptions.Item label="鏇存柊鏃堕棿">
          {formatDateTime(detail.doc.updated_at)}
        </Descriptions.Item>
      </Descriptions>
      <Table<InboundDocItem>
        data-testid="inbound-detail-items"
        rowKey={(record) => record.sku_id}
        columns={columns}
        dataSource={detail.items}
        pagination={false}
      />
      <Card
        title="璐ㄦ璁板綍"
        loading={inspectionQuery.isFetching}
        data-testid="inbound-detail-inspections"
      >
        <Table<InboundInspection>
          data-testid="inbound-detail-inspections-table"
          rowKey={(record) => record.inspection_id}
          columns={inspectionColumns}
          dataSource={inspectionQuery.data?.items ?? []}
          pagination={false}
          locale={{ emptyText: '鏆傛棤璐ㄦ璁板綍' }}
        />
      </Card>
    </Space>
  );

  return <WorkbenchMainTableTemplate toolbarLeft={toolbarLeft} tableContent={tableContent} />;
};

