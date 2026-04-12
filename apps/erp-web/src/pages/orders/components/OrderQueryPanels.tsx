import { Collapse, Tree, Typography } from 'antd';
import type { CollapseProps } from 'antd';
import type { DataNode } from 'antd/es/tree';
import type { Key } from 'react';
import type { TreeProps } from 'antd';

type OrderQueryFilterPanelProps = {
  title?: string;
  treeData: DataNode[];
  selectedKeys: Key[];
  onSelect: TreeProps['onSelect'];
};

export const OrderQueryFilterPanel = ({
  title = '筛选',
  treeData,
  selectedKeys,
  onSelect
}: OrderQueryFilterPanelProps) => (
  <div className="order-query-tree">
    <Typography.Text strong>{title}</Typography.Text>
    <Tree
      blockNode
      defaultExpandAll
      selectedKeys={selectedKeys}
      onSelect={onSelect}
      treeData={treeData}
    />
  </div>
);

type OrderQueryDetailPanelProps = {
  panels: CollapseProps['items'];
  defaultActiveKeys?: CollapseProps['defaultActiveKey'];
};

export const OrderQueryDetailPanel = ({
  panels,
  defaultActiveKeys
}: OrderQueryDetailPanelProps) => (
  <Collapse size="small" defaultActiveKey={defaultActiveKeys} items={panels} />
);
