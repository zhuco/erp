import { Button, Input, Radio, Select, Space } from 'antd';
import {
  EllipsisOutlined,
  ReloadOutlined,
  SearchOutlined,
  SettingOutlined,
  SyncOutlined,
} from '@ant-design/icons';

import './UiModule08StrategyScreenFilterForm.css';

export type UiModule08StrategyScreenFilterState = {
  orderQuery: string;
  salesChannel: string;
  warehouse: string;
  createdFrom: string;
  createdTo: string;
  customerRemark: 'any' | 'has' | 'none' | 'exclude';
  serviceRemark: 'any' | 'has' | 'none' | 'exclude';
  orderTag: string;
  excludedTag: string;
  carrierLoss: string;
  regionScope: 'all' | 'exclude';
  regionKeyword: string;
  orderType: string;
  includeSkuMode: 'any' | 'all';
  includeSkuKeyword: string;
};

export const createUiModule08StrategyScreenFilterState = (): UiModule08StrategyScreenFilterState => ({
  orderQuery: '',
  salesChannel: '',
  warehouse: '',
  createdFrom: '',
  createdTo: '',
  customerRemark: 'any',
  serviceRemark: 'any',
  orderTag: '',
  excludedTag: '',
  carrierLoss: '',
  regionScope: 'all',
  regionKeyword: '',
  orderType: '',
  includeSkuMode: 'any',
  includeSkuKeyword: '',
});

export type UiModule08StrategyScreenFilterChangeHandler = <
  K extends keyof UiModule08StrategyScreenFilterState,
>(
  key: K,
  value: UiModule08StrategyScreenFilterState[K],
) => void;

type UiModule08StrategyScreenFilterFormProps = {
  value: UiModule08StrategyScreenFilterState;
  onFieldChange: UiModule08StrategyScreenFilterChangeHandler;
  onApply: () => void;
  onReset: () => void;
  showHeader?: boolean;
};

export const UiModule08StrategyScreenFilterForm = ({
  value,
  onFieldChange,
  onApply,
  onReset,
  showHeader = true,
}: UiModule08StrategyScreenFilterFormProps) => (
  <div className="strategy-left-filter-form">
    {showHeader ? (
      <div className="strategy-left-filter-tools">
        <span className="strategy-left-filter-title">筛选条件</span>
        <Space size={2}>
          <Button type="text" size="small" icon={<SettingOutlined />} />
          <Button type="text" size="small" icon={<SyncOutlined />} />
        </Space>
      </div>
    ) : null}
    <div className="strategy-filter-field">
      <label>按订单编号查询</label>
      <Select
        mode="tags"
        tokenSeparators={[',', '，', ' ']}
        size="small"
        placeholder="请输入单号，多个单号可多选"
        value={value.orderQuery
          .split(/[,\s，]+/)
          .map((item) => item.trim())
          .filter(Boolean)}
        onChange={(next) => onFieldChange('orderQuery', next.join(','))}
      />
    </div>
    <div className="strategy-filter-field">
      <label>销售渠道</label>
      <Select
        size="small"
        allowClear
        value={value.salesChannel || undefined}
        placeholder="请选择..."
        options={[
          { label: '淘宝', value: '淘宝' },
          { label: '京东', value: '京东' },
          { label: '拼多多', value: '拼多多' },
        ]}
        onChange={(next) => onFieldChange('salesChannel', next ?? '')}
      />
    </div>
    <div className="strategy-filter-field">
      <label>发货仓库</label>
      <Select
        size="small"
        allowClear
        value={value.warehouse || undefined}
        placeholder="请选择..."
        options={[
          { label: '杭州仓', value: '杭州仓' },
          { label: '广州仓', value: '广州仓' },
          { label: '义乌仓', value: '义乌仓' },
        ]}
        onChange={(next) => onFieldChange('warehouse', next ?? '')}
      />
    </div>
    <div className="strategy-filter-field">
      <label>建单时间</label>
      <div className="strategy-filter-date-row">
        <span>从</span>
        <Input
          size="small"
          data-testid="strategy-screen-created-from"
          placeholder="YYYY-MM-DD"
          value={value.createdFrom}
          onChange={(event) => onFieldChange('createdFrom', event.target.value)}
        />
      </div>
      <div className="strategy-filter-date-row">
        <span>到</span>
        <Input
          size="small"
          data-testid="strategy-screen-created-to"
          placeholder="YYYY-MM-DD"
          value={value.createdTo}
          onChange={(event) => onFieldChange('createdTo', event.target.value)}
        />
      </div>
    </div>
    <div className="strategy-filter-field">
      <label>客户备注</label>
      <Radio.Group
        size="small"
        value={value.customerRemark}
        onChange={(event) => onFieldChange('customerRemark', event.target.value)}
      >
        <Radio value="any">不限</Radio>
        <Radio value="has">有</Radio>
        <Radio value="none">无</Radio>
        <Radio value="exclude">不含</Radio>
      </Radio.Group>
    </div>
    <div className="strategy-filter-field">
      <label>客服备注</label>
      <Radio.Group
        size="small"
        value={value.serviceRemark}
        onChange={(event) => onFieldChange('serviceRemark', event.target.value)}
      >
        <Radio value="any">不限</Radio>
        <Radio value="has">有</Radio>
        <Radio value="none">无</Radio>
        <Radio value="exclude">不含</Radio>
      </Radio.Group>
    </div>
    <div className="strategy-filter-field">
      <label>订单标记</label>
      <Select
        size="small"
        allowClear
        value={value.orderTag || undefined}
        placeholder="请选择..."
        options={[
          { label: '异常', value: '异常' },
          { label: '备注', value: '备注' },
          { label: '复核', value: '复核' },
        ]}
        onChange={(next) => onFieldChange('orderTag', next ?? '')}
      />
    </div>
    <div className="strategy-filter-field">
      <label>不含订单标记</label>
      <Select
        size="small"
        allowClear
        value={value.excludedTag || undefined}
        placeholder="请选择..."
        options={[
          { label: '草稿', value: '草稿' },
          { label: '停用', value: '停用' },
        ]}
        onChange={(next) => onFieldChange('excludedTag', next ?? '')}
      />
    </div>
    <div className="strategy-filter-field">
      <label>承担方丢失</label>
      <Input
        size="small"
        value={value.carrierLoss}
        onChange={(event) => onFieldChange('carrierLoss', event.target.value)}
      />
    </div>
    <div className="strategy-filter-field">
      <label>国家/省市区</label>
      <Radio.Group
        size="small"
        value={value.regionScope}
        onChange={(event) => onFieldChange('regionScope', event.target.value)}
      >
        <Radio value="all">包含</Radio>
        <Radio value="exclude">不包含</Radio>
      </Radio.Group>
      <div className="strategy-filter-inline">
        <Input
          size="small"
          placeholder="请选择..."
          value={value.regionKeyword}
          onChange={(event) => onFieldChange('regionKeyword', event.target.value)}
        />
        <Button size="small" icon={<EllipsisOutlined />} />
      </div>
    </div>
    <div className="strategy-filter-field">
      <label>订单类型</label>
      <Select
        size="small"
        allowClear
        value={value.orderType || undefined}
        placeholder="请选择..."
        options={[
          { label: '普通订单', value: '普通订单' },
          { label: '预售订单', value: '预售订单' },
          { label: '售后换货', value: '售后换货' },
        ]}
        onChange={(next) => onFieldChange('orderType', next ?? '')}
      />
    </div>
    <div className="strategy-filter-field">
      <label>包含货品</label>
      <Radio.Group
        size="small"
        value={value.includeSkuMode}
        onChange={(event) => onFieldChange('includeSkuMode', event.target.value)}
      >
        <Radio value="any">任意一种</Radio>
        <Radio value="all">同时满足</Radio>
      </Radio.Group>
      <div className="strategy-filter-inline">
        <Input
          size="small"
          placeholder="输入货品编码..."
          data-testid="strategy-screen-include-sku"
          value={value.includeSkuKeyword}
          onChange={(event) => onFieldChange('includeSkuKeyword', event.target.value)}
        />
        <Button size="small" icon={<EllipsisOutlined />} />
      </div>
    </div>
    <div className="strategy-left-filter-actions">
      <Button
        type="primary"
        size="small"
        icon={<SearchOutlined />}
        onClick={onApply}
        data-testid="strategy-screen-apply"
      >
        筛选
      </Button>
      <Button size="small" icon={<ReloadOutlined />} onClick={onReset} data-testid="strategy-screen-reset">
        重置
      </Button>
    </div>
  </div>
);
