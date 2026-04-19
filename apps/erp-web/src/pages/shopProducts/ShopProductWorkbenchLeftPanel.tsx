import type { CellClickedEvent } from 'ag-grid-community';
import type { ReactNode } from 'react';
import { useMemo, useState } from 'react';

import {
  UiModule08OrderReviewLeftPanelTemplate,
  type UiModule08LeftPanelTabKey,
  type UiModule08OrderReviewLeftRenderRow,
  type UiModule08OrderReviewLeftStatsItem,
} from '@/components/ui-modules/UiModule08OrderReviewLeftPanelTemplate';

import type { ShopProductWorkbenchSummary } from './shopProductWorkbenchModel';

type ShopProductWorkbenchLeftPanelProps = {
  filterContent: ReactNode;
  summary: ShopProductWorkbenchSummary;
  apparelTemplateName?: string;
};

const buildLeftStatsItems = (summary: ShopProductWorkbenchSummary): UiModule08OrderReviewLeftStatsItem[] => [
  { key: 'matched', label: '已匹配', value: summary.matched, color: '#1677ff' },
  { key: 'unmatched', label: '未匹配', value: summary.unmatched, color: '#faad14' },
  { key: 'conflict', label: '冲突', value: summary.conflict, color: '#f5222d' },
  { key: 'selected', label: '已选中', value: summary.selected, color: '#52c41a' },
];

const buildPieBackground = (items: UiModule08OrderReviewLeftStatsItem[]) => {
  const total = items.reduce((sum, item) => sum + item.value, 0);
  if (total <= 0) {
    return 'conic-gradient(#dce4ee 0deg 360deg)';
  }

  let start = 0;
  const segments = items.map((item) => {
    const angle = (item.value / total) * 360;
    const end = start + angle;
    const segment = `${item.color} ${start}deg ${end}deg`;
    start = end;
    return segment;
  });
  return `conic-gradient(${segments.join(', ')})`;
};

const buildLeftRows = (
  summary: ShopProductWorkbenchSummary,
  apparelTemplateName?: string,
  checkedKey?: string,
): UiModule08OrderReviewLeftRenderRow[] => [
  {
    key: 'all',
    label: '全部商品',
    count: summary.total,
    nodeType: 'group',
    level: 0,
    hasChildren: false,
    expanded: false,
    checked: checkedKey === 'all',
  },
  {
    key: 'matched',
    label: '已匹配',
    count: summary.matched,
    nodeType: 'tag',
    level: 0,
    hasChildren: false,
    expanded: false,
    checked: checkedKey === 'matched',
  },
  {
    key: 'unmatched',
    label: '未匹配',
    count: summary.unmatched,
    nodeType: 'remark',
    level: 0,
    hasChildren: false,
    expanded: false,
    checked: checkedKey === 'unmatched',
  },
  {
    key: 'conflict',
    label: '冲突待处理',
    count: summary.conflict,
    nodeType: 'exception',
    level: 0,
    hasChildren: false,
    expanded: false,
    checked: checkedKey === 'conflict',
  },
  {
    key: 'selected',
    label: '当前选中',
    count: summary.selected,
    nodeType: 'platform',
    level: 0,
    hasChildren: false,
    expanded: false,
    checked: checkedKey === 'selected',
  },
  {
    key: 'template',
    label: apparelTemplateName ? `鞋服模板 ${apparelTemplateName}` : '鞋服模板未启用',
    nodeType: 'tag',
    level: 0,
    hasChildren: false,
    expanded: false,
    checked: checkedKey === 'template',
  },
];

export const ShopProductWorkbenchLeftPanel = ({
  filterContent,
  summary,
  apparelTemplateName,
}: ShopProductWorkbenchLeftPanelProps) => {
  const [leftTab, setLeftTab] = useState<UiModule08LeftPanelTabKey>('filter');
  const [checkedKey, setCheckedKey] = useState<string>('all');

  const leftStatsItems = useMemo(() => buildLeftStatsItems(summary), [summary]);
  const leftStatsPieBackground = useMemo(() => buildPieBackground(leftStatsItems), [leftStatsItems]);
  const leftRowsForRender = useMemo(
    () => buildLeftRows(summary, apparelTemplateName, checkedKey),
    [apparelTemplateName, checkedKey, summary],
  );

  const handleLeftRowClick = (event: CellClickedEvent<UiModule08OrderReviewLeftRenderRow>) => {
    const rowKey = event.data?.key;
    if (!rowKey) {
      return;
    }
    setCheckedKey(rowKey);
  };

  return (
    <UiModule08OrderReviewLeftPanelTemplate
      className="shop-products-left-panel"
      dataTestId="shop-products-left-panel"
      leftTab={leftTab}
      onLeftTabChange={setLeftTab}
      leftRowsForRender={leftRowsForRender}
      onLeftRowClick={handleLeftRowClick}
      leftStatsItems={leftStatsItems}
      leftStatsPieBackground={leftStatsPieBackground}
      globalOrderTotal={summary.total}
      statsScopeText={apparelTemplateName ? `模板：${apparelTemplateName}` : '口径：当前商品下载对比列表'}
      screenContent={filterContent}
    />
  );
};
