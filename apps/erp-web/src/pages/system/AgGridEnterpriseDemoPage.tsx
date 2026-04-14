import { Alert, Card, Input, Segmented, Space, Statistic, Typography } from 'antd';
import { useMemo, useState } from 'react';
import type { ColDef } from 'ag-grid-community';
import { ModuleRegistry } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';

import 'ag-grid-community/styles/ag-grid.css';
import 'ag-grid-community/styles/ag-theme-quartz.css';

import { ENTERPRISE_DEMO_MODULES } from '@/components/agGridEnterpriseModules';

import './agGridEnterpriseDemoPage.css';

ModuleRegistry.registerModules([...ENTERPRISE_DEMO_MODULES]);

type EnterpriseRow = {
  id: string;
  orderNo: string;
  shop: string;
  platform: string;
  region: string;
  status: string;
  qty: number;
  amount: number;
  createdAt: string;
};

const SHOPS = ['旗舰店', '直播店', '分销店', '自营店'];
const PLATFORMS = ['淘宝', '京东', '拼多多', '抖店'];
const REGIONS = ['华东', '华南', '华北', '华中'];
const STATUS = ['待审', '已审', '锁定', '待配货'];

const buildRows = (count: number): EnterpriseRow[] =>
  Array.from({ length: count }, (_, index) => {
    const rowIndex = index + 1;
    const qty = 1 + (rowIndex % 8);
    const amount = Number((qty * (55.4 + (rowIndex % 6) * 12.3)).toFixed(2));
    return {
      id: `ag-e-${rowIndex}`,
      orderNo: `SO-${String(2027000 + rowIndex).padStart(7, '0')}`,
      shop: SHOPS[index % SHOPS.length]!,
      platform: PLATFORMS[index % PLATFORMS.length]!,
      region: REGIONS[index % REGIONS.length]!,
      status: STATUS[index % STATUS.length]!,
      qty,
      amount,
      createdAt: `2026-04-${String((rowIndex % 28) + 1).padStart(2, '0')} 10:${String(
        rowIndex % 60,
      ).padStart(2, '0')}:00`,
    };
  });

export const AgGridEnterpriseDemoPage = () => {
  const [keyword, setKeyword] = useState('');
  const [mode, setMode] = useState<'group' | 'pivot'>('group');
  const rowData = useMemo(() => buildRows(1500), []);

  const columnDefs = useMemo<ColDef<EnterpriseRow>[]>(
    () => [
      {
        headerName: '订单号',
        field: 'orderNo',
        pinned: 'left',
        minWidth: 170,
        checkboxSelection: true,
        headerCheckboxSelection: true,
      },
      {
        headerName: '店铺',
        field: 'shop',
        enableRowGroup: true,
        enablePivot: true,
        minWidth: 120,
      },
      {
        headerName: '平台',
        field: 'platform',
        enableRowGroup: true,
        enablePivot: true,
        rowGroup: mode === 'group',
        minWidth: 110,
      },
      {
        headerName: '区域',
        field: 'region',
        enableRowGroup: true,
        enablePivot: true,
        minWidth: 100,
      },
      {
        headerName: '状态',
        field: 'status',
        enableRowGroup: true,
        enablePivot: true,
        pivot: mode === 'pivot',
        minWidth: 110,
      },
      {
        headerName: '数量',
        field: 'qty',
        type: 'numericColumn',
        enableValue: true,
        aggFunc: 'sum',
        minWidth: 90,
      },
      {
        headerName: '金额',
        field: 'amount',
        type: 'numericColumn',
        enableValue: true,
        aggFunc: 'sum',
        minWidth: 120,
        valueFormatter: (params) => `¥${Number(params.value ?? 0).toFixed(2)}`,
      },
      {
        headerName: '建单时间',
        field: 'createdAt',
        minWidth: 180,
        enablePivot: true,
      },
    ],
    [mode],
  );

  const defaultColDef = useMemo<ColDef<EnterpriseRow>>(
    () => ({
      sortable: true,
      filter: true,
      floatingFilter: true,
      resizable: true,
      editable: false,
      enableRowGroup: true,
      enablePivot: true,
      enableValue: true,
      minWidth: 100,
    }),
    [],
  );

  return (
    <div className="ag-grid-enterprise-page" data-testid="ag-grid-enterprise-page">
      <Space direction="vertical" size={16} style={{ width: '100%' }}>
        <Typography.Title level={4} style={{ margin: 0 }}>
          AG Grid Enterprise 功能演示
        </Typography.Title>
        <Typography.Text type="secondary">
          用于验证企业版效果：分组、透视、侧边栏、状态栏与范围选择等核心交互能力。
        </Typography.Text>
        <Alert
          type="info"
          showIcon
          message="企业版未设置 License Key 时会显示试用水印"
          description="先用于效果验证，后续如果确定采用 Enterprise 再配置正式 License。"
        />

        <Card size="small">
          <Space size={20} wrap style={{ width: '100%', justifyContent: 'space-between' }}>
            <Input.Search
              allowClear
              placeholder="快速搜索订单/店铺/平台/状态"
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              style={{ maxWidth: 360 }}
            />
            <Space size={16}>
              <Segmented
                value={mode}
                onChange={(value) => setMode(value as 'group' | 'pivot')}
                options={[
                  { label: '分组模式', value: 'group' },
                  { label: '透视模式', value: 'pivot' },
                ]}
              />
              <Statistic title="总行数" value={rowData.length} />
            </Space>
          </Space>
        </Card>

        <Card size="small">
          <div className="ag-theme-quartz ag-grid-enterprise-shell">
            <AgGridReact<EnterpriseRow>
              theme="legacy"
              rowData={rowData}
              columnDefs={columnDefs}
              defaultColDef={defaultColDef}
              quickFilterText={keyword}
              rowSelection="multiple"
              suppressRowClickSelection
              animateRows
              pagination
              paginationPageSize={50}
              sideBar
              statusBar={{
                statusPanels: [
                  { statusPanel: 'agTotalAndFilteredRowCountComponent', align: 'left' },
                  { statusPanel: 'agAggregationComponent', align: 'right' },
                ],
              }}
              rowGroupPanelShow="always"
              pivotPanelShow={mode === 'pivot' ? 'always' : 'never'}
              pivotMode={mode === 'pivot'}
              cellSelection
              suppressAggFuncInHeader
            />
          </div>
        </Card>
      </Space>
    </div>
  );
};
