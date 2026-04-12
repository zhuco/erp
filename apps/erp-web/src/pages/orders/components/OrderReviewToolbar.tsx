import { Button, Dropdown, Tooltip } from 'antd';
import type { MenuProps } from 'antd';
import { DownOutlined } from '@ant-design/icons';
type OrderReviewToolbarLeftProps = {
  onReview: () => void;
  onEdit: () => void;
  onMerge: () => void;
  onSplit: () => void;
  mergeSubmitting: boolean;
  canWriteOrders: boolean;
  hasReviewTargets: boolean;
  hasMergeTargets: boolean;
  canViewStrategies: boolean;
  strategyMenuItems: MenuProps['items'];
  onStrategyMenuClick: MenuProps['onClick'];
  showReviewAction?: boolean;
  showEditAction?: boolean;
  showStrategyAction?: boolean;
};

export const OrderReviewToolbarLeft = ({
  onReview,
  onEdit,
  onMerge,
  onSplit,
  mergeSubmitting,
  canWriteOrders,
  hasReviewTargets,
  hasMergeTargets,
  canViewStrategies,
  strategyMenuItems,
  onStrategyMenuClick,
  showReviewAction = true,
  showEditAction = true,
  showStrategyAction = true,
}: OrderReviewToolbarLeftProps) => {
  const reviewDisabledReason = !canWriteOrders
    ? '暂无订单审核权限'
    : !hasReviewTargets
      ? '请先选择订单'
      : undefined;
  const editDisabledReason = !canWriteOrders ? '暂无订单操作权限' : undefined;
  const mergeDisabledReason = !canWriteOrders
    ? '暂无订单操作权限'
    : !hasMergeTargets
      ? '请至少选择两单'
      : undefined;
  const splitDisabledReason = !canWriteOrders ? '暂无订单操作权限' : undefined;

  return (
    <>
      {showReviewAction ? (
        <Tooltip title={reviewDisabledReason}>
          <span>
            <Button
              size="small"
              data-testid="order-review-approve"
              onClick={onReview}
              disabled={Boolean(reviewDisabledReason)}
            >
              审核
            </Button>
          </span>
        </Tooltip>
      ) : null}
      {showEditAction ? (
        <Tooltip title={editDisabledReason}>
          <span>
            <Button size="small" onClick={onEdit} disabled={Boolean(editDisabledReason)}>
              编辑
            </Button>
          </span>
        </Tooltip>
      ) : null}
      <Tooltip title={mergeDisabledReason}>
        <span>
          <Button
            size="small"
            loading={mergeSubmitting}
            onClick={onMerge}
            disabled={Boolean(mergeDisabledReason)}
          >
            合并
          </Button>
        </span>
      </Tooltip>
      <Tooltip title={splitDisabledReason}>
        <span>
          <Button
            size="small"
            data-testid="order-review-split"
            onClick={onSplit}
            disabled={Boolean(splitDisabledReason)}
          >
            拆分
          </Button>
        </span>
      </Tooltip>
      {showStrategyAction ? (
        <Dropdown
          menu={{ items: strategyMenuItems, onClick: onStrategyMenuClick }}
          placement="bottomRight"
          disabled={!canViewStrategies}
        >
          <Button size="small" icon={<DownOutlined />} disabled={!canViewStrategies}>
            策略
          </Button>
        </Dropdown>
      ) : null}
    </>
  );
};

type OrderReviewToolbarRightProps = {
  canExport: boolean;
  onExport: () => void;
  onPrint: () => void;
  onHelp: () => void;
};

export const OrderReviewToolbarRight = ({
  canExport,
  onExport,
  onPrint,
  onHelp
}: OrderReviewToolbarRightProps) => (
  <>
    <Button size="small" onClick={onExport} disabled={!canExport}>
      导出
    </Button>
    <Button size="small" onClick={onPrint}>
      打印
    </Button>
    <Button size="small" onClick={onHelp}>
      帮助
    </Button>
  </>
);
