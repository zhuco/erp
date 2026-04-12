import { auditPassedStatuses, exceptionResolvableStatuses } from './statuses';

type GuardResult = {
  allowed: boolean;
  reasons: string[];
};

type SubmitForFulfillmentInput = {
  audit_status?: string | null;
  exception_status?: string | null;
  reserve_precheck_passed?: boolean | null;
};

const buildGuardResult = (reasons: string[]): GuardResult => ({
  allowed: reasons.length === 0,
  reasons
});

export const canSubmitForFulfillment = (input: SubmitForFulfillmentInput): GuardResult => {
  const reasons: string[] = [];

  if (!auditPassedStatuses.includes((input.audit_status ?? '') as (typeof auditPassedStatuses)[number])) {
    reasons.push('审核未通过，禁止提交发货');
  }

  if (
    !exceptionResolvableStatuses.includes(
      (input.exception_status ?? '') as (typeof exceptionResolvableStatuses)[number]
    )
  ) {
    reasons.push('异常未闭环，禁止提交发货');
  }

  if (input.reserve_precheck_passed !== true) {
    reasons.push('占库存预检未通过，禁止提交发货');
  }

  return buildGuardResult(reasons);
};

type CreateWaveInput = {
  reserve_status?: string | null;
};

export const canCreateWave = (input: CreateWaveInput): GuardResult => {
  const reasons: string[] = [];
  if (input.reserve_status !== 'RESERVED') {
    reasons.push('仅已占库存订单允许建波次');
  }
  return buildGuardResult(reasons);
};

type StartPickingInput = {
  wave_status?: string | null;
};

export const canStartPicking = (input: StartPickingInput): GuardResult => {
  const reasons: string[] = [];
  if (input.wave_status !== 'RELEASED') {
    reasons.push('波次未释放，禁止开始拣货');
  }
  return buildGuardResult(reasons);
};

type ReviewWeighingInput = {
  pick_status?: string | null;
};

export const canReviewAndWeigh = (input: ReviewWeighingInput): GuardResult => {
  const reasons: string[] = [];
  if (input.pick_status !== 'PICKED') {
    reasons.push('拣货未完成，禁止复核称重');
  }
  return buildGuardResult(reasons);
};

type HandoverInput = {
  pack_status?: string | null;
};

export const canHandoverOutbound = (input: HandoverInput): GuardResult => {
  const reasons: string[] = [];
  if (input.pack_status !== 'PACKED') {
    reasons.push('打包未完成，禁止出库交接');
  }
  return buildGuardResult(reasons);
};

type RetryShippingPushInput = {
  fulfillment_status?: string | null;
};

export const canRetryShippingPush = (input: RetryShippingPushInput): GuardResult => {
  const reasons: string[] = [];
  if (input.fulfillment_status !== 'SHIPPING_PUSH_FAILED') {
    reasons.push('仅物流回传失败订单允许重试');
  }
  return buildGuardResult(reasons);
};
