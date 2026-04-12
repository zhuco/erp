import {
  Alert,
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  inspectWmsOutbound,
  listWmsOutbounds,
  shipWmsOutbound,
  type WmsOutboundDoc,
  type WmsOutboundListQuery,
  type WmsShipPayload
} from '@/api/wms';
import { listWarehouses } from '@/api/warehouses';
import { getWarehouseWorkflowConfig } from '@/api/workflowConfigs';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  status?: string;
};

type ShipFormValues = {
  shipped_at?: Dayjs;
};

const statusOptions = [
  { label: 'PICKED', value: 'PICKED' },
  { label: 'PACKED', value: 'PACKED' },
  { label: 'INSPECTED', value: 'INSPECTED' },
  { label: 'SHIPPED', value: 'SHIPPED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const WmsShipPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('wms.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [filterForm] = Form.useForm<FilterValues>();
  const [shipForm] = Form.useForm<ShipFormValues>();
  const [shipOpen, setShipOpen] = useState(false);
  const [shipTarget, setShipTarget] = useState<WmsOutboundDoc | null>(null);

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'wms-ship'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId)
  });

  const defaultWarehouseId = useMemo(() => {
    const warehouses = warehousesQuery.data?.items ?? [];
    return warehouses.find((item) => item.is_default)?.warehouse_id ?? warehouses[0]?.warehouse_id;
  }, [warehousesQuery.data]);

  const activeWarehouseId = filters.warehouse_id?.trim() || defaultWarehouseId;

  const workflowQuery = useQuery({
    queryKey: ['workflow-config', tenantId, activeWarehouseId],
    queryFn: () => getWarehouseWorkflowConfig(tenantId, activeWarehouseId ?? ''),
    enabled: Boolean(activeWarehouseId)
  });

  const workflowConfig = workflowQuery.data;

  useEffect(() => {
    if (!workflowConfig) {
      return;
    }
    setFilters((prev) => {
      if (prev.status) {
        return prev;
      }
      return {
        ...prev,
        status: workflowConfig.require_pack ? 'PACKED' : 'PICKED'
      };
    });
  }, [workflowConfig]);

  useEffect(() => {
    filterForm.setFieldsValue(filters);
  }, [filterForm, filters]);

  const query = useMemo<WmsOutboundListQuery>(() => {
    return {
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['wms-ship-outbounds', tenantId, query],
    queryFn: () => listWmsOutbounds(tenantId, query),
    keepPreviousData: true
  });

  const inspectMutation = useMutation({
    mutationFn: (outboundDocId: string) => inspectWmsOutbound(tenantId, outboundDocId),
    onSuccess: () => {
      message.success('验货完成');
      queryClient.invalidateQueries({ queryKey: ['wms-ship-outbounds', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '验货失败');
    }
  });

  const shipMutation = useMutation({
    mutationFn: ({ outboundDocId, payload }: { outboundDocId: string; payload: WmsShipPayload }) =>
      shipWmsOutbound(tenantId, outboundDocId, payload),
    onSuccess: () => {
      message.success('发货已提交');
      queryClient.invalidateQueries({ queryKey: ['wms-ship-outbounds', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '发货失败');
    }
  });

  const canInspect = (status?: string | null) =>
    workflowConfig?.require_inspection && status === 'PACKED';

  const canShip = (status?: string | null) => {
    if (!status) {
      return false;
    }
    if (!workflowConfig) {
      return false;
    }
    if (!workflowConfig.require_pack) {
      return ['PICKED', 'PACKED', 'INSPECTED'].includes(status);
    }
    if (workflowConfig.require_inspection) {
      return status === 'INSPECTED';
    }
    return status === 'PACKED' || status === 'INSPECTED';
  };

  const columns = useMemo<ColumnsType<WmsOutboundDoc>>(
    () => [
      {
        title: '出库单号',
        dataIndex: 'doc_no'
      },
      {
        title: '状态',
        dataIndex: 'status'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            {canInspect(record.status) ? (
              <Button
                size="small"
                data-testid="ship-inspect-action"
                disabled={!canWrite}
                loading={inspectMutation.isPending}
                onClick={() => inspectMutation.mutate(record.outbound_doc_id)}
              >
                验货
              </Button>
            ) : null}
            <Button
              size="small"
              data-testid="ship-action"
              disabled={!canWrite || !canShip(record.status)}
              onClick={() => {
                setShipTarget(record);
                shipForm.resetFields();
                setShipOpen(true);
              }}
            >
              发货
            </Button>
          </Space>
        )
      }
    ],
    [canWrite, inspectMutation.isPending, shipForm, workflowConfig]
  );

  const handleShip = async (values: ShipFormValues) => {
    if (!shipTarget) {
      return;
    }
    try {
      const payload: WmsShipPayload = {
        shipped_at: values.shipped_at?.toISOString()
      };
      await shipMutation.mutateAsync({ outboundDocId: shipTarget.outbound_doc_id, payload });
      setShipOpen(false);
      setShipTarget(null);
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  };

  const toolbarLeft = (
    <Typography.Title level={3} data-testid="ship-title" style={{ margin: 0 }}>
      发货交接
    </Typography.Title>
  );

  const header = (
    <Space direction="vertical" size={12}>
      {workflowConfig && !workflowConfig.require_pack ? (
        <Alert
          type="info"
          showIcon
          message="当前仓库未启用打包登记"
          description="允许拣货完成后直接发货。"
        />
      ) : null}
      {workflowConfig && !workflowConfig.require_inspection ? (
        <Alert
          type="info"
          showIcon
          message="验货步骤已关闭"
          description="系统将跳过验货流程。"
        />
      ) : null}
      <Card>
        <Form
          form={filterForm}
          layout="vertical"
          onFinish={(values) => {
            setFilters(values);
            setPagination((prev) => ({ ...prev, current: 1 }));
          }}
          initialValues={filters}
        >
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库ID">
                <Input allowClear placeholder="仓库ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="全部状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button
                  onClick={() => {
                    filterForm.resetFields();
                    setFilters({});
                    setPagination((prev) => ({ ...prev, current: 1 }));
                  }}
                >
                  重置
                </Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <div data-testid="ship-table">
        <UiModule03MainTableSection<WmsOutboundDoc>
          toolbarLeft={toolbarLeft}
          header={header}
          rowKey="outbound_doc_id"
          columns={columns}
          data={data?.items ?? []}
          loading={isLoading || isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="出库单加载失败"
          emptyText="暂无可发货出库单"
          total={data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="wms.ship.table.columns"
        />
      </div>
      <Modal
        title="确认发货"
        open={shipOpen}
        onCancel={() => {
          setShipOpen(false);
          setShipTarget(null);
        }}
        onOk={() => shipForm.submit()}
        okText="发货"
        confirmLoading={shipMutation.isPending}
        data-testid="ship-modal"
        okButtonProps={{ 'data-testid': 'ship-submit' }}
      >
        <Form form={shipForm} layout="vertical" onFinish={handleShip}>
          <Form.Item name="shipped_at" label="发货时间">
            <DatePicker
              showTime
              allowClear
              style={{ width: '100%' }}
              data-testid="ship-date-picker"
            />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
