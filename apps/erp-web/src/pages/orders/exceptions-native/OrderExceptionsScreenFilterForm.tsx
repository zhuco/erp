import { ReloadOutlined, SearchOutlined } from '@ant-design/icons';
import { Button, Input, Select } from 'antd';

import '@/components/ui-modules/UiModule08StrategyScreenFilterForm.css';

import type {
  OrderExceptionsScreenFilterOption,
  OrderExceptionsScreenFilterState,
} from './orderExceptionsScreenFilters';

type OrderExceptionsScreenFilterFormProps = {
  value: OrderExceptionsScreenFilterState;
  platformOptions: OrderExceptionsScreenFilterOption[];
  shopOptions: OrderExceptionsScreenFilterOption[];
  auditStatusOptions: OrderExceptionsScreenFilterOption[];
  exceptionStatusOptions: OrderExceptionsScreenFilterOption[];
  slaStatusOptions: OrderExceptionsScreenFilterOption[];
  warehouseOptions: OrderExceptionsScreenFilterOption[];
  assigneeOptions: OrderExceptionsScreenFilterOption[];
  onFieldChange: <K extends keyof OrderExceptionsScreenFilterState>(
    key: K,
    value: OrderExceptionsScreenFilterState[K],
  ) => void;
  onApply: () => void;
  onReset: () => void;
};

export const OrderExceptionsScreenFilterForm = ({
  value,
  platformOptions,
  shopOptions,
  auditStatusOptions,
  exceptionStatusOptions,
  slaStatusOptions,
  warehouseOptions,
  assigneeOptions,
  onFieldChange,
  onApply,
  onReset,
}: OrderExceptionsScreenFilterFormProps) => (
  <div className="strategy-left-filter-form" data-testid="order-exceptions-screen-filter-form">
    <div className="strategy-filter-field">
      <label>查询关键字</label>
      <Input
        size="small"
        placeholder="订单号/收件人/阻断原因"
        value={value.orderQuery}
        onChange={(event) => onFieldChange('orderQuery', event.target.value)}
      />
    </div>

    <div className="strategy-filter-field">
      <label>平台</label>
      <Select
        size="small"
        allowClear
        value={value.platform || undefined}
        options={platformOptions}
        placeholder="全部平台"
        onChange={(next) => onFieldChange('platform', String(next ?? ''))}
      />
    </div>

    <div className="strategy-filter-field">
      <label>店铺</label>
      <Select
        size="small"
        allowClear
        value={value.shopId || undefined}
        options={shopOptions}
        placeholder="全部店铺"
        onChange={(next) => onFieldChange('shopId', String(next ?? ''))}
      />
    </div>

    <div className="strategy-filter-field">
      <label>审核状态</label>
      <Select
        size="small"
        allowClear
        value={value.auditStatus || undefined}
        options={auditStatusOptions}
        placeholder="全部审核状态"
        onChange={(next) => onFieldChange('auditStatus', String(next ?? ''))}
      />
    </div>

    <div className="strategy-filter-field">
      <label>异常状态</label>
      <Select
        size="small"
        allowClear
        value={value.exceptionStatus || undefined}
        options={exceptionStatusOptions}
        placeholder="全部异常状态"
        onChange={(next) => onFieldChange('exceptionStatus', String(next ?? ''))}
      />
    </div>

    <div className="strategy-filter-field">
      <label>SLA 状态</label>
      <Select
        size="small"
        allowClear
        value={value.slaStatus || undefined}
        options={slaStatusOptions}
        placeholder="全部 SLA 状态"
        onChange={(next) => onFieldChange('slaStatus', String(next ?? ''))}
      />
    </div>

    <div className="strategy-filter-field">
      <label>发货仓</label>
      <Select
        size="small"
        allowClear
        value={value.warehouseId || undefined}
        options={warehouseOptions}
        placeholder="全部发货仓"
        onChange={(next) => onFieldChange('warehouseId', String(next ?? ''))}
      />
    </div>

    <div className="strategy-filter-field">
      <label>责任人</label>
      <Select
        size="small"
        allowClear
        value={value.assigneeName || undefined}
        options={assigneeOptions}
        placeholder="全部责任人"
        onChange={(next) => onFieldChange('assigneeName', String(next ?? ''))}
      />
    </div>

    <div className="strategy-filter-field">
      <label>阻断原因</label>
      <Input
        size="small"
        placeholder="输入阻断原因关键字"
        value={value.blockingReasonKeyword}
        onChange={(event) => onFieldChange('blockingReasonKeyword', event.target.value)}
      />
    </div>

    <div className="strategy-left-filter-actions">
      <Button
        type="primary"
        size="small"
        icon={<SearchOutlined />}
        onClick={onApply}
        data-testid="order-exceptions-screen-filter-apply"
      >
        筛选
      </Button>
      <Button
        size="small"
        icon={<ReloadOutlined />}
        onClick={onReset}
        data-testid="order-exceptions-screen-filter-reset"
      >
        重置
      </Button>
    </div>
  </div>
);
