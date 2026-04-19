import { Button, Card, Descriptions, Space, Table, Tabs, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type { InboundDocItem, InboundDocStatus, InboundInspection } from '@/api/inbounds';
import { getInboundDetail, listInboundInspections } from '@/api/inbounds';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { ErrorState } from '@/components/QueryState';
import { OrderReviewWorkbenchTemplate, OrderReviewWorkbenchToolbarTemplate } from '@/components/workbench/templates';
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
  PART_RECEIVED: '部分收货',
  RECEIVED: '已收货',
  NOT_REQUIRED: '不要求',
  PENDING: '待处理',
  PASSED: '合格',
  FAILED: '不合格',
  PENDING_REVIEW: '待复检',
  NOT_PUTAWAY: '未上架',
  PART_PUTAWAY: '部分上架',
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
  const [toolbarSearchInput, setToolbarSearchInput] = useState('');
  const [toolbarSearchKeyword, setToolbarSearchKeyword] = useState('');
  const { setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible } = useWorkbenchPanels();

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
      { title: 'SKU 编码', dataIndex: 'sku_code' },
      { title: '条码', dataIndex: 'barcode' },
      { title: '计划数量', dataIndex: 'planned_qty' },
      { title: '已收货', dataIndex: 'received_qty' },
      { title: '已上架', dataIndex: 'putaway_qty' }
    ],
    []
  );

  const inspectionColumns = useMemo<ColumnsType<InboundInspection>>(
    () => [
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: '数量', dataIndex: 'qty' },
      {
        title: '质检结果',
        dataIndex: 'result',
        render: (value: string) =>
          value === 'PASSED' ? '合格' : value === 'FAILED' ? '不合格' : '待复检'
      },
      { title: '备注', dataIndex: 'note', render: (value) => value ?? '-' },
      {
        title: '质检时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
  );

  const filteredItems = useMemo(() => {
    const keyword = toolbarSearchKeyword.trim().toLowerCase();
    if (!keyword) {
      return detail?.items ?? [];
    }
    return (detail?.items ?? []).filter((item) => {
      return (
        item.sku_id.toLowerCase().includes(keyword) ||
        String(item.sku_code ?? '').toLowerCase().includes(keyword) ||
        String(item.barcode ?? '').toLowerCase().includes(keyword)
      );
    });
  }, [detail?.items, toolbarSearchKeyword]);

  const filteredInspections = useMemo(() => {
    const source = inspectionQuery.data?.items ?? [];
    const keyword = toolbarSearchKeyword.trim().toLowerCase();
    if (!keyword) {
      return source;
    }
    return source.filter((item) => {
      return (
        item.sku_id.toLowerCase().includes(keyword) ||
        String(item.result ?? '').toLowerCase().includes(keyword) ||
        String(item.note ?? '').toLowerCase().includes(keyword)
      );
    });
  }, [inspectionQuery.data?.items, toolbarSearchKeyword]);

  const leftPanel = useMemo(
    () => (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Card size="small" title="入库进度">
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            单号：{detail?.doc.doc_no ?? '-'}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            状态：{detail?.doc.status ?? '-'}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            明细行：{detail?.items.length ?? 0}
          </Typography.Paragraph>
        </Card>
        <Card size="small" title="筛选范围">
          <Typography.Paragraph style={{ marginBottom: 6 }}>
            可见明细：{filteredItems.length}
          </Typography.Paragraph>
          <Typography.Paragraph style={{ marginBottom: 0 }}>
            可见质检记录：{filteredInspections.length}
          </Typography.Paragraph>
        </Card>
      </Space>
    ),
    [detail?.doc.doc_no, detail?.doc.status, detail?.items.length, filteredInspections.length, filteredItems.length]
  );

  useEffect(() => {
    setLeftPanelCollapsed(false);
    setLeftPanel(leftPanel);
    setLeftPanelVisible(true);
    return () => {
      setLeftPanel(undefined);
      setLeftPanelVisible(false);
    };
  }, [leftPanel, setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible]);

  const rightPanel = useMemo(
    () => (
      <Card size="small" title="处理建议">
        <Typography.Paragraph style={{ marginBottom: 8 }}>
          收货、质检、上架状态会在本页联动刷新，建议按“收货 - 质检 - 上架”流程处理。
        </Typography.Paragraph>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          仓库：{warehouseMap.get(detail?.doc.warehouse_id ?? '') ?? detail?.doc.warehouse_id ?? '-'}
        </Typography.Paragraph>
      </Card>
    ),
    [detail?.doc.warehouse_id, warehouseMap]
  );

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        items={[
          {
            key: 'items',
            label: `入库明细 (${filteredItems.length})`,
            children: (
              <Table<InboundDocItem>
                size="small"
                rowKey={(record) => record.sku_id}
                columns={columns}
                dataSource={filteredItems}
                pagination={false}
              />
            )
          },
          {
            key: 'inspections',
            label: `质检璁板綍 (${filteredInspections.length})`,
            children: (
              <Table<InboundInspection>
                size="small"
                rowKey={(record) => record.inspection_id}
                columns={inspectionColumns}
                dataSource={filteredInspections}
                pagination={false}
              />
            )
          }
        ]}
      />
    ),
    [columns, filteredInspections, filteredItems, inspectionColumns]
  );

  if (!doc_id) {
    message.error('缺少入库单 ID');
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
        入库详情
      </Typography.Title>
      <Button data-testid="inbound-detail-back" onClick={() => navigate('/wms/inbounds')}>
        返回列表
      </Button>
      <Button
        data-testid="inbound-detail-receive"
        disabled={!canOperate}
        onClick={() => navigate(`/wms/inbounds/${doc_id}/receive`)}
      >
        收货
      </Button>
      <Button
        data-testid="inbound-detail-inspect"
        disabled={!canOperate}
        onClick={() => navigate(`/wms/inbounds/${doc_id}/inspect`)}
      >
        质检
      </Button>
      <Button
        data-testid="inbound-detail-putaway"
        disabled={!canOperate}
        onClick={() => navigate(`/wms/inbounds/${doc_id}/putaway`)}
      >
        上架
      </Button>
    </Space>
  );

  const toolbar = (
    <OrderReviewWorkbenchToolbarTemplate
      leftContent={toolbarLeft}
      searchValue={toolbarSearchInput}
      onSearchChange={setToolbarSearchInput}
      onSearch={() => setToolbarSearchKeyword(toolbarSearchInput.trim())}
      searchPlaceholder="按 SKU / 条码 / 质检结果筛选"
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('可使用顶部搜索快速定位明细与质检记录')}
    />
  );

  const tableContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Descriptions bordered size="small" column={2}>
        <Descriptions.Item label="入库单号">{detail.doc.doc_no}</Descriptions.Item>
        <Descriptions.Item label="状态">{statusTag}</Descriptions.Item>
        <Descriptions.Item label="入库状态">{renderTraceStatus(traceDoc.inbound_status)}</Descriptions.Item>
        <Descriptions.Item label="收货状态">{renderTraceStatus(traceDoc.receipt_status)}</Descriptions.Item>
        <Descriptions.Item label="质检状态">{renderTraceStatus(traceDoc.inbound_qc_status)}</Descriptions.Item>
        <Descriptions.Item label="上架状态">{renderTraceStatus(traceDoc.putaway_status)}</Descriptions.Item>
        <Descriptions.Item label="仓库">
          {warehouseMap.get(detail.doc.warehouse_id) ?? detail.doc.warehouse_id}
        </Descriptions.Item>
        <Descriptions.Item label="单据类型">{detail.doc.doc_type}</Descriptions.Item>
        <Descriptions.Item label="来源类型">{detail.doc.ref_type ?? '-'}</Descriptions.Item>
        <Descriptions.Item label="来源单据">{detail.doc.ref_id ?? '-'}</Descriptions.Item>
        <Descriptions.Item label="创建时间">
          {formatDateTime(detail.doc.created_at)}
        </Descriptions.Item>
        <Descriptions.Item label="更新时间">
          {formatDateTime(detail.doc.updated_at)}
        </Descriptions.Item>
      </Descriptions>
      <Table<InboundDocItem>
        data-testid="inbound-detail-items"
        rowKey={(record) => record.sku_id}
        columns={columns}
        dataSource={filteredItems}
        pagination={false}
      />
      <Card
        title="质检记录"
        loading={inspectionQuery.isFetching}
        data-testid="inbound-detail-inspections"
      >
        <Table<InboundInspection>
          data-testid="inbound-detail-inspections-table"
          rowKey={(record) => record.inspection_id}
          columns={inspectionColumns}
          dataSource={filteredInspections}
          pagination={false}
          locale={{ emptyText: '暂无质检记录' }}
        />
      </Card>
    </Space>
  );

  return (
    <OrderReviewWorkbenchTemplate
      toolbar={toolbar}
      tableContent={tableContent}
      rightPanel={rightPanel}
      bottomPanel={bottomPanel}
    />
  );
};

