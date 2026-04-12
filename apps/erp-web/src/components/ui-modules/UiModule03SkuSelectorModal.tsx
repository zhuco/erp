import { Input, Modal, Space, Table, Tabs, Tree, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';

import { searchSkus, type SkuKind, type SkuSearchItem } from '@/api/skus';
import { useTenantId } from '@/features/tenant';

import './UiModule03SkuSelectorModal.css';

export type UiModule03SkuSelectorModalProps = {
  open: boolean;
  onCancel: () => void;
  onConfirm: (sku: SkuSearchItem) => void;
  extraFooter?: ReactNode;
};

const skuKindTabs: Array<{ key: SkuKind; label: string }> = [
  { key: 'NORMAL', label: '货品' },
  { key: 'KIT', label: '组合装货品' }
];

export const UiModule03SkuSelectorModal = ({
  open,
  onCancel,
  onConfirm,
  extraFooter
}: UiModule03SkuSelectorModalProps) => {
  const tenantId = useTenantId();
  const [activeKind, setActiveKind] = useState<SkuKind>('NORMAL');
  const [keyword, setKeyword] = useState('');
  const [loading, setLoading] = useState(false);
  const [items, setItems] = useState<SkuSearchItem[]>([]);
  const [selectedSkuId, setSelectedSkuId] = useState<string | null>(null);

  const selectedSku = useMemo(
    () => items.find((item) => item.sku_id === selectedSkuId) ?? null,
    [items, selectedSkuId]
  );

  const treeTitle = activeKind === 'KIT' ? '全部组合装' : '全部货品';

  const treeData = useMemo(
    () => [
      {
        title: treeTitle,
        key: 'all'
      }
    ],
    [treeTitle]
  );

  const columns = useMemo<ColumnsType<SkuSearchItem>>(
    () => [
      {
        title: '货品编号',
        dataIndex: 'sku_code',
        width: 140,
        render: (value) => value ?? '-'
      },
      {
        title: '货品名称',
        dataIndex: 'name',
        width: 220,
        render: (value) => value ?? '-'
      },
      {
        title: '条码',
        dataIndex: 'barcode',
        width: 160,
        render: (value) => value ?? '-'
      },
      {
        title: '类型',
        dataIndex: 'sku_kind',
        width: 120,
        render: (value) => (value === 'KIT' ? '组合装' : '货品')
      }
    ],
    []
  );

  const loadSkus = useCallback(
    async (nextKeyword: string, kind: SkuKind) => {
      if (!tenantId) {
        return;
      }
      setLoading(true);
      try {
        const response = await searchSkus(tenantId, nextKeyword, kind);
        setItems(response.items ?? []);
      } catch (err) {
        message.error(err instanceof Error ? err.message : '货品搜索失败');
        setItems([]);
      } finally {
        setLoading(false);
      }
    },
    [tenantId]
  );

  useEffect(() => {
    if (!open) {
      return;
    }
    setSelectedSkuId(null);
    void loadSkus(keyword, activeKind);
  }, [activeKind, keyword, loadSkus, open]);

  useEffect(() => {
    if (open) {
      return;
    }
    setKeyword('');
    setItems([]);
    setSelectedSkuId(null);
  }, [open]);

  const handleConfirm = useCallback(() => {
    if (!selectedSku) {
      message.warning('请选择货品');
      return;
    }
    onConfirm(selectedSku);
  }, [onConfirm, selectedSku]);

  return (
    <Modal
      title="选择货品"
      open={open}
      onCancel={onCancel}
      onOk={handleConfirm}
      okText="确定"
      cancelText="取消"
      width={1080}
      className="order-review-sku-selector-modal"
      okButtonProps={{ disabled: !selectedSku }}
      destroyOnClose
    >
      <Tabs
        activeKey={activeKind}
        onChange={(key) => setActiveKind(key as SkuKind)}
        items={skuKindTabs.map((tab) => ({
          key: tab.key,
          label: tab.label
        }))}
      />
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Input.Search
          allowClear
          placeholder="输入货品编号/名称/条码查询"
          value={keyword}
          onChange={(event) => setKeyword(event.target.value)}
          onSearch={(value) => {
            setKeyword(value);
            void loadSkus(value, activeKind);
          }}
        />
        <div className="order-review-sku-selector-content">
          <div className="order-review-sku-selector-tree">
            <Typography.Text type="secondary">货品分类</Typography.Text>
            <Tree
              treeData={treeData}
              selectedKeys={['all']}
              defaultExpandAll
              className="order-review-sku-selector-tree-body"
            />
          </div>
          <div className="order-review-sku-selector-table">
            <Table
              rowKey="sku_id"
              size="small"
              columns={columns}
              dataSource={items}
              loading={loading}
              pagination={false}
              rowSelection={{
                type: 'radio',
                selectedRowKeys: selectedSkuId ? [selectedSkuId] : [],
                onChange: (keys) => {
                  setSelectedSkuId((keys[0] as string) ?? null);
                }
              }}
              onRow={(record) => ({
                onClick: () => setSelectedSkuId(record.sku_id),
                onDoubleClick: () => {
                  setSelectedSkuId(record.sku_id);
                  onConfirm(record);
                }
              })}
              locale={{ emptyText: '暂无货品' }}
            />
          </div>
          <div className="order-review-sku-selector-aside">
            <Typography.Text type="secondary">货品信息</Typography.Text>
            {selectedSku ? (
              <div className="order-review-sku-selector-aside-content">
                <div>编号：{selectedSku.sku_code}</div>
                <div>名称：{selectedSku.name ?? '-'}</div>
                <div>条码：{selectedSku.barcode ?? '-'}</div>
                <div>类型：{selectedSku.sku_kind === 'KIT' ? '组合装' : '货品'}</div>
              </div>
            ) : (
              <Typography.Text type="secondary">请选择货品查看详情</Typography.Text>
            )}
          </div>
        </div>
      </Space>
      {extraFooter ? <div style={{ marginTop: 8 }}>{extraFooter}</div> : null}
    </Modal>
  );
};
