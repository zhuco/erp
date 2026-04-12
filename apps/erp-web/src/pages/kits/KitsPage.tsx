import {
  Button,
  Descriptions,
  Empty,
  Form,
  Input,
  Modal,
  Space,
  Table,
  Tabs,
  Tag,
  Typography,
  message
} from 'antd';
import type { MenuProps, TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useRef, useState, type ChangeEvent, type Key, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { KitComponent, KitListQuery, KitSummary } from '@/api/kits';
import { getKitDetail, listKits } from '@/api/kits';
import { createProduct } from '@/api/products';
import { copySku, createSku, deleteSku } from '@/api/skus';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import {
  UiModule02ToolbarRight,
  UiModule03ContextMenu,
  UiModule03MainTableSection,
  UiModule03TableFooter,
  UiModule04BottomTable,
  UiModule05DetailPanel
} from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type CreateKitFormValues = {
  sku_code?: string;
  name?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const KitsPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const canWrite = useCan('kits.write');
  const { setLeftPanel, setRightPanel, setLeftPanelVisible, setRightPanelVisible, clearPanels } =
    useWorkbenchPanels();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [selectedKit, setSelectedKit] = useState<KitSummary | null>(null);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [searchValue, setSearchValue] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [copying, setCopying] = useState(false);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    kit: KitSummary;
  } | null>(null);
  const [activeBottomTab, setActiveBottomTab] = useState('components');
  const [createForm] = Form.useForm<CreateKitFormValues>();
  const tableContainerRef = useRef<HTMLDivElement | null>(null);

  const query = useMemo<KitListQuery>(
    () => ({
      keyword: searchKeyword.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [pagination.current, pagination.pageSize, searchKeyword]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['kits', tenantId, query],
    queryFn: () => listKits(tenantId, query),
    keepPreviousData: true
  });

  const detailQuery = useQuery({
    queryKey: ['kit-detail', tenantId, selectedKit?.kit_sku_id],
    queryFn: () => getKitDetail(tenantId, selectedKit?.kit_sku_id ?? ''),
    enabled: Boolean(tenantId && selectedKit?.kit_sku_id),
    keepPreviousData: true
  });
  const total = data?.total ?? 0;

  useEffect(() => {
    if (!selectedKit) {
      return;
    }
    const exists = data?.items.some((item) => item.kit_sku_id === selectedKit.kit_sku_id);
    if (!exists) {
      setSelectedKit(null);
      setSelectedRowKeys([]);
    }
  }, [data?.items, selectedKit]);

  useEffect(() => {
    const trimmedKeyword = searchValue.trim();
    if (!trimmedKeyword || trimmedKeyword === searchKeyword) {
      return;
    }
    const timer = window.setTimeout(() => {
      setSearchKeyword(trimmedKeyword);
      setSelectedKit(null);
      setSelectedRowKeys([]);
      setPagination((prev) => ({ ...prev, current: 1 }));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [searchKeyword, searchValue]);

  const handleSearch = (value: string) => {
    const keyword = value.trim();
    setSearchKeyword(keyword);
    setSelectedKit(null);
    setSelectedRowKeys([]);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleSearchChange = (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setSearchValue(value);
    if (!value.trim()) {
      setSearchKeyword('');
      setSelectedKit(null);
      setSelectedRowKeys([]);
      setPagination((prev) => ({ ...prev, current: 1 }));
    }
  };

  const handlePaginationChange = (next: TablePaginationConfig) => {
    setPagination((prev) => ({ ...prev, ...next }));
    setSelectedKit(null);
    setSelectedRowKeys([]);
  };

  const handleRowSelectionChange = (keys: Key[], rows: KitSummary[]) => {
    setSelectedRowKeys(keys);
    setSelectedKit(rows[0] ?? null);
  };

  const handleOpenCreate = () => {
    createForm.resetFields();
    setCreateModalOpen(true);
  };

  const handleCreateKit = async () => {
    try {
      const values = await createForm.validateFields();
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      const skuCode = values.sku_code?.trim();
      const name = values.name?.trim();
      if (!skuCode || !name) {
        return;
      }
      setCreating(true);
      const product = await createProduct(tenantId, {
        name,
        product_code: skuCode
      });
      const sku = await createSku(tenantId, {
        product_id: product.product_id,
        sku_code: skuCode,
        name,
        sku_kind: 'KIT'
      });
      message.success('组合装已创建');
      setCreateModalOpen(false);
      createForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['kits', tenantId] });
      navigate(`/kits/${sku.sku_id}`);
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    } finally {
      setCreating(false);
    }
  };

  const handleEdit = (record?: KitSummary) => {
    const target = record ?? selectedKit;
    if (!target) {
      message.warning('请先选择组合装');
      return;
    }
    navigate(`/kits/${target.kit_sku_id}`);
  };

  const handleCopy = async (record?: KitSummary) => {
    const target = record ?? selectedKit;
    if (!target) {
      message.warning('请先选择组合装');
      return;
    }
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (!canWrite) {
      message.warning('暂无编辑权限');
      return;
    }
    setCopying(true);
    try {
      const copied = await copySku(tenantId, target.kit_sku_id);
      message.success('组合装已复制');
      await queryClient.invalidateQueries({ queryKey: ['kits', tenantId] });
      navigate(`/kits/${copied.sku_id}`);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '复制组合装失败');
    } finally {
      setCopying(false);
    }
  };

  const handleDelete = (record?: KitSummary) => {
    const target = record ?? selectedKit;
    if (!target) {
      message.warning('请先选择组合装');
      return;
    }
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    Modal.confirm({
      title: '删除组合装',
      content: `确认删除组合装 ${target.kit_sku_code} 吗？`,
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        try {
          await deleteSku(tenantId, target.kit_sku_id);
          message.success('组合装已删除');
          setSelectedKit(null);
          setSelectedRowKeys([]);
          await queryClient.invalidateQueries({ queryKey: ['kits', tenantId] });
        } catch (err) {
          message.error(err instanceof Error ? err.message : '删除组合装失败');
        }
      }
    });
  };

  const columns = useMemo<ColumnsType<KitSummary>>(
    () => [
      {
        title: '组合装SKU',
        dataIndex: 'kit_sku_code',
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <span>{record.kit_sku_code}</span>
            <span>{record.kit_name ?? '-'}</span>
          </Space>
        )
      },
      {
        title: '组件数量',
        dataIndex: 'component_count',
        align: 'right'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8} wrap>
            <Button
              size="small"
              data-testid="kit-edit-link"
              onClick={() => handleEdit(record)}
              disabled={!canWrite}
            >
              编辑
            </Button>
            <Button
              size="small"
              onClick={() => handleCopy(record)}
              disabled={!canWrite}
              loading={copying}
            >
              复制
            </Button>
            <Button size="small" danger onClick={() => handleDelete(record)} disabled={!canWrite}>
              删除
            </Button>
          </Space>
        )
      }
    ],
    [canWrite, copying, handleCopy, handleDelete, handleEdit]
  );

  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${data?.items?.length ?? 0}`,
    [columns.length, data?.items?.length]
  );

  const componentColumns = useMemo<ColumnsType<KitComponent>>(
    () => [
      {
        title: '组件SKU',
        dataIndex: 'component_sku_code',
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <span>{record.component_sku_code}</span>
            <span>{record.component_name ?? '-'}</span>
          </Space>
        )
      },
      {
        title: '条码',
        dataIndex: 'component_barcode',
        render: (_, record) => record.component_barcode ?? '-'
      },
      {
        title: '数量',
        dataIndex: 'qty',
        align: 'right'
      },
      {
        title: '有效期',
        dataIndex: 'expire_at',
        render: (_, record) => (record.expire_at ? formatDateTime(record.expire_at) : '-')
      }
    ],
    []
  );

  const kitDetail = detailQuery.data;
  const componentRows = kitDetail?.components ?? [];
  const kitDisplayName = kitDetail?.kit_sku?.name ?? selectedKit?.kit_name ?? '-';
  const kitSkuCode = kitDetail?.kit_sku?.sku_code ?? selectedKit?.kit_sku_code ?? '-';
  const kitUpdatedAt = kitDetail?.updated_at ?? selectedKit?.updated_at ?? null;

  const renderDetailContent = (content: ReactNode) => {
    if (!selectedKit) {
      return <Typography.Text type="secondary">请选择组合装查看详情</Typography.Text>;
    }
    if (detailQuery.isFetching) {
      return <Typography.Text type="secondary">组合装详情加载中...</Typography.Text>;
    }
    if (detailQuery.error) {
      return (
        <Space direction="vertical">
          <Typography.Text type="secondary">组合装详情加载失败</Typography.Text>
          <Button size="small" onClick={() => detailQuery.refetch()}>
            重试
          </Button>
        </Space>
      );
    }
    return content;
  };

  const detailPanels = useMemo(
    () => [
      {
        key: 'detail',
        label: '组合装详情',
        children: renderDetailContent(
          <Descriptions size="small" column={1}>
            <Descriptions.Item label="组合装SKU">{kitSkuCode}</Descriptions.Item>
            <Descriptions.Item label="组合装名称">{kitDisplayName}</Descriptions.Item>
            <Descriptions.Item label="组件数量">
              {kitDetail?.components.length ?? selectedKit?.component_count ?? 0}
            </Descriptions.Item>
            <Descriptions.Item label="状态">
              <Tag color="green">启用</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="更新时间">{formatDateTime(kitUpdatedAt)}</Descriptions.Item>
          </Descriptions>
        )
      },
      {
        key: 'actions',
        label: '快捷操作',
        children: (
          <Space size={8} wrap>
            <Button size="small" onClick={() => handleEdit()} disabled={!canWrite || !selectedKit}>
              编辑组合装
            </Button>
            <Button
              size="small"
              onClick={() => handleCopy()}
              disabled={!canWrite || !selectedKit}
              loading={copying}
            >
              复制组合装
            </Button>
          </Space>
        )
      }
    ],
    [
      canWrite,
      copying,
      handleCopy,
      handleEdit,
      kitDetail?.components.length,
      kitDisplayName,
      kitSkuCode,
      kitUpdatedAt,
      renderDetailContent,
      selectedKit
    ]
  );

  const rightPanel = useMemo(
    () => <UiModule05DetailPanel panels={detailPanels} defaultActiveKeys={['detail']} />,
    [detailPanels]
  );

  const bottomPanel = useMemo(
    () => (
      <div className="kits-bottom-panel">
        <Tabs
          size="small"
          type="card"
          activeKey={activeBottomTab}
          onChange={(key) => setActiveBottomTab(key)}
          items={[
            {
              key: 'components',
              label: `组件清单 (${componentRows.length})`,
              children: selectedKit ? (
                <UiModule04BottomTable
                  syncKey={`components-${componentRows.length}-${selectedKit.kit_sku_id}`}
                >
                  <Table
                    size="small"
                    rowKey="component_sku_id"
                    columns={componentColumns}
                    dataSource={componentRows}
                    loading={detailQuery.isFetching}
                    pagination={false}
                    scroll={{ x: 'max-content' }}
                    locale={{
                      emptyText: detailQuery.error ? (
                        <Empty description="组件加载失败" />
                      ) : (
                        <Empty description="暂无组件" />
                      )
                    }}
                  />
                </UiModule04BottomTable>
              ) : (
                <Empty description="请选择组合装查看组件" />
              )
            }
          ]}
        />
      </div>
    ),
    [activeBottomTab, componentColumns, componentRows, detailQuery, selectedKit]
  );

  useEffect(() => {
    setLeftPanelVisible(false);
    setRightPanelVisible(true);
    setLeftPanel(undefined);
    setRightPanel(rightPanel);
    return () => clearPanels();
  }, [clearPanels, rightPanel, setLeftPanel, setLeftPanelVisible, setRightPanel, setRightPanelVisible]);

  const toolbarLeft = (
    <Space size={8} wrap>
      <Button type="primary" onClick={handleOpenCreate} disabled={!canWrite}>
        新建组合装
      </Button>
      <Button onClick={() => handleCopy()} disabled={!canWrite || !selectedKit} loading={copying}>
        复制
      </Button>
      <Button onClick={() => handleEdit()} disabled={!canWrite || !selectedKit}>
        修改
      </Button>
      <Button danger onClick={() => handleDelete()} disabled={!canWrite || !selectedKit}>
        删除
      </Button>
      <Button onClick={() => refetch()}>刷新</Button>
    </Space>
  );

  const toolbarCenter = (
    <Input.Search
      allowClear
      value={searchValue}
      placeholder="搜索组合装编号/名称"
      onSearch={handleSearch}
      onChange={handleSearchChange}
      style={{ width: 260 }}
      data-testid="kits-search"
    />
  );

  const toolbarRight = (
    <UiModule02ToolbarRight
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('帮助功能待接入')}
    />
  );

  const tableFooter = (
    <UiModule03TableFooter
      tableContainerRef={tableContainerRef}
      syncKey={tableFooterSyncKey}
      summary={`共 ${total} 条，已勾选 ${selectedRowKeys.length} 条`}
      total={total}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
    />
  );

  const header = (
    <Typography.Title level={3} style={{ margin: 0 }} data-testid="kits-title">
      组合装管理
    </Typography.Title>
  );

  const contextMenuItems: MenuProps['items'] = [
    { key: 'edit', label: '修改组合装', disabled: !canWrite },
    { key: 'copy', label: '复制组合装', disabled: !canWrite },
    { key: 'delete', label: '删除组合装', disabled: !canWrite }
  ];

  return (
    <>
      <UiModule03MainTableSection<KitSummary>
        toolbarLeft={toolbarLeft}
        toolbarCenter={toolbarCenter}
        toolbarRight={toolbarRight}
        header={header}
        tableContainerRef={tableContainerRef}
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="组合装列表加载失败"
        emptyText="暂无组合装"
        total={total}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        columnSettingsKey="kits.table.columns"
        rowKey="kit_sku_id"
        rowSelection={{
          type: 'radio',
          selectedRowKeys,
          onChange: handleRowSelectionChange
        }}
        rowClassName={(record) =>
          record.kit_sku_id === selectedKit?.kit_sku_id ? 'ant-table-row-selected' : ''
        }
        onContextMenuSelect={(record) => setSelectedKit(record)}
        onRow={(record) => ({
          onClick: () => {
            setSelectedKit(record);
            setSelectedRowKeys([record.kit_sku_id]);
          },
          onDoubleClick: () => {
            if (!canWrite) {
              message.warning('暂无编辑权限');
              return;
            }
            handleEdit(record);
          },
          onContextMenu: (event) => {
            event.preventDefault();
            setSelectedKit(record);
            setSelectedRowKeys([record.kit_sku_id]);
            setContextMenu({ x: event.clientX, y: event.clientY, kit: record });
          }
        })}
        tableFooter={tableFooter}
        bottomPanel={bottomPanel}
      />

      <UiModule03ContextMenu
        open={Boolean(contextMenu)}
        position={contextMenu}
        menu={{
          items: contextMenuItems,
          onClick: ({ key }) => {
            if (!contextMenu) {
              return;
            }
            const record = contextMenu.kit;
            if (key === 'edit') {
              handleEdit(record);
            } else if (key === 'copy') {
              handleCopy(record);
            } else if (key === 'delete') {
              handleDelete(record);
            }
            setContextMenu(null);
          }
        }}
        onClose={() => setContextMenu(null)}
      />

      <Modal
        title="新建组合装"
        open={createModalOpen}
        onCancel={() => setCreateModalOpen(false)}
        onOk={handleCreateKit}
        okText="创建"
        cancelText="取消"
        okButtonProps={{ loading: creating, disabled: !canWrite }}
      >
        <Form form={createForm} layout="vertical">
          <Form.Item
            label="组合装编号"
            name="sku_code"
            rules={[{ required: true, message: '请输入组合装编号' }]}
          >
            <Input placeholder="请输入组合装编号" />
          </Form.Item>
          <Form.Item
            label="组合装名称"
            name="name"
            rules={[{ required: true, message: '请输入组合装名称' }]}
          >
            <Input placeholder="请输入组合装名称" />
          </Form.Item>
          <Typography.Text type="secondary">
            创建后将进入组合装编辑页面配置组件清单。
          </Typography.Text>
        </Form>
      </Modal>
    </>
  );
};
