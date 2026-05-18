import {
  Button,
  Col,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
  confirmWmsPickTask,
  listWmsPickTasks,
  type WmsPickTask,
  type WmsPickTaskListQuery
} from '@/api/wms';
import { listContainers } from '@/api/containers';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';
import { IndustrialCard } from '@/components/industrial';

type FilterValues = {
  warehouse_id?: string;
  outbound_doc_id?: string;
  status?: string;
};

type ConfirmState = {
  open: boolean;
  pickTask?: WmsPickTask;
};

export const WmsPickTasksPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<IndustrialTablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [confirmState, setConfirmState] = useState<ConfirmState>({ open: false });
  const [confirmForm] = Form.useForm<{ picked_qty: number; container_id?: string }>();
  const [filterForm] = Form.useForm<FilterValues>();

  const query = useMemo<WmsPickTaskListQuery>(() => {
    return {
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      outbound_doc_id: filters.outbound_doc_id?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['wms-pick-tasks', tenantId, query],
    queryFn: () => listWmsPickTasks(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const containersQuery = useQuery({
    queryKey: ['containers', tenantId, 'pick-tasks'],
    queryFn: () => listContainers(tenantId, { offset: 0, limit: 200 }),
    placeholderData: (previousData) => previousData
  });

  const containerOptions = useMemo(
    () =>
      (containersQuery.data?.items ?? []).map((container) => ({
        label: `${container.code} (${container.container_type})`,
        value: container.container_id
      })),
    [containersQuery.data]
  );

  const containerMap = useMemo(() => {
    const map = new Map<string, string>();
    (containersQuery.data?.items ?? []).forEach((container) => {
      map.set(container.container_id, container.code);
    });
    return map;
  }, [containersQuery.data]);

  const columns = useMemo<ColumnsType<WmsPickTask>>(
    () => [
      {
        title: '任务ID',
        dataIndex: 'pick_task_id'
      },
      {
        title: '出库单',
        dataIndex: 'outbound_doc_id'
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id'
      },
      {
        title: '容器',
        dataIndex: 'container_id',
        render: (value) => (value ? containerMap.get(value) ?? value : '-')
      },
      {
        title: '数量',
        dataIndex: 'qty'
      },
      {
        title: '已拣',
        dataIndex: 'picked_qty'
      },
      {
        title: '状态',
        dataIndex: 'status'
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button
            size="small"
            data-testid="pick-task-confirm-action"
            onClick={() => {
              confirmForm.setFieldsValue({
                picked_qty: record.qty - record.picked_qty,
                container_id: record.container_id ?? undefined
              });
              setConfirmState({ open: true, pickTask: record });
            }}
          >
            确认拣货
          </Button>
        )
      }
    ],
    [confirmForm, containerMap]
  );

  const handleConfirm = async (values: { picked_qty: number; container_id?: string }) => {
    if (!confirmState.pickTask) {
      return;
    }
    try {
      await confirmWmsPickTask(tenantId, confirmState.pickTask.pick_task_id, values);
      message.success('拣货已确认');
      setConfirmState({ open: false });
      confirmForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['wms-pick-tasks', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '确认失败');
    }
  };

  const toolbarLeft = (
    <Typography.Title level={3} data-testid="pick-tasks-title" style={{ margin: 0 }}>
      拣货任务
    </Typography.Title>
  );

  const header = (
    <IndustrialCard>
      <Form
        form={filterForm}
        layout="vertical"
        onFinish={(values) => {
          setFilters(values);
          setPagination((prev) => ({ ...prev, current: 1 }));
        }}
      >
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="warehouse_id" label="仓库ID">
              <Input allowClear placeholder="仓库ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="outbound_doc_id" label="出库单ID">
              <Input allowClear placeholder="出库单ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="status" label="状态">
              <Input allowClear placeholder="状态" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
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
    </IndustrialCard>
  );

  return (
    <>
      <div data-testid="pick-tasks-table">
        <UiModule03MainTableSection<WmsPickTask>
          toolbarLeft={toolbarLeft}
          header={header}
          rowKey="pick_task_id"
          columns={columns}
          data={data?.items ?? []}
          loading={isLoading || isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="拣货任务加载失败"
          emptyText="暂无拣货任务"
          total={data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          dataTestId="wms-pick-tasks-main-grid"
          gridPreset="industrial-review-compact"
          selectionCellMode="custom"
          selectionHeaderMode="allInvert"
          showColumnSettings
          columnSettingsKey="wms.pick-tasks.table.columns"
        />
      </div>
      <Modal
        title="确认拣货"
        open={confirmState.open}
        onCancel={() => setConfirmState({ open: false })}
        onOk={() => confirmForm.submit()}
        data-testid="pick-task-confirm-modal"
        okButtonProps={{ 'data-testid': 'pick-task-confirm-submit' }}
      >
        <Form form={confirmForm} layout="vertical" onFinish={handleConfirm}>
          <Form.Item name="container_id" label="容器">
            <Select
              allowClear
              options={containerOptions}
              placeholder="选择容器"
              data-testid="pick-task-container-select"
            />
          </Form.Item>
          <Form.Item name="picked_qty" label="拣货数量" rules={[{ required: true }]}>
            <InputNumber min={1} style={{ width: '100%' }} data-testid="pick-task-picked-qty" />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
