export const auditStatuses = [
  'AUTO_PASSED',
  'REVIEW_REQUIRED',
  'UNDER_REVIEW',
  'REVIEW_PASSED',
  'REVIEW_REJECTED'
] as const;

export const auditPassedStatuses = ['AUTO_PASSED', 'REVIEW_PASSED'] as const;

export const exceptionStatuses = [
  'NONE',
  'OPEN',
  'ASSIGNED',
  'PROCESSING',
  'ESCALATED',
  'RESOLVED',
  'CLOSED'
] as const;

export const exceptionResolvableStatuses = ['NONE', 'RESOLVED', 'CLOSED'] as const;

export const fulfillmentStatuses = [
  'NOT_SUBMITTED',
  'WAVE_PENDING',
  'WAVED',
  'PICKING',
  'PACKING',
  'HANDOVER_PENDING',
  'HANDOVERED',
  'SHIPPING_PUSH_FAILED',
  'SHIPPED'
] as const;

export const reserveStatuses = ['RESERVED', 'RESERVE_FAILED'] as const;

export const waveStatuses = ['CREATED', 'RELEASED', 'CANCELED'] as const;

export const pickStatuses = ['PICKING', 'PICKED'] as const;

export const packStatuses = ['PACKING', 'CHECKED', 'WEIGHED', 'PACKED'] as const;

export const handoverStatuses = ['HANDOVERED', 'PUSHED'] as const;

export const auditStatusLabels: Record<(typeof auditStatuses)[number], string> = {
  AUTO_PASSED: '自动审核通过',
  REVIEW_REQUIRED: '待复核',
  UNDER_REVIEW: '待复核',
  REVIEW_PASSED: '复核通过',
  REVIEW_REJECTED: '复核驳回'
};

export const exceptionStatusLabels: Record<(typeof exceptionStatuses)[number], string> = {
  NONE: '无异常',
  OPEN: '待处理',
  ASSIGNED: '已分派',
  PROCESSING: '处理中',
  ESCALATED: '已升级',
  RESOLVED: '已解决',
  CLOSED: '已关闭'
};

export const fulfillmentStatusLabels: Record<(typeof fulfillmentStatuses)[number], string> = {
  NOT_SUBMITTED: '未递交WMS',
  WAVE_PENDING: '待建波次',
  WAVED: '已建波次',
  PICKING: '拣货中',
  PACKING: '打包中',
  HANDOVER_PENDING: '待交接',
  HANDOVERED: '已交接',
  SHIPPING_PUSH_FAILED: '发货推单失败',
  SHIPPED: '已发货'
};

export const reserveStatusLabels: Record<(typeof reserveStatuses)[number], string> = {
  RESERVED: '已占库存',
  RESERVE_FAILED: '占库存失败'
};

export const waveStatusLabels: Record<(typeof waveStatuses)[number], string> = {
  CREATED: '已创建',
  RELEASED: '已释放',
  CANCELED: '已取消'
};

export const pickStatusLabels: Record<(typeof pickStatuses)[number], string> = {
  PICKING: '拣货中',
  PICKED: '已拣货'
};

export const packStatusLabels: Record<(typeof packStatuses)[number], string> = {
  PACKING: '打包中',
  CHECKED: '已复核',
  WEIGHED: '已称重',
  PACKED: '已打包'
};

export const handoverStatusLabels: Record<(typeof handoverStatuses)[number], string> = {
  HANDOVERED: '已交接',
  PUSHED: '已推单'
};

const reviewStatusLabels: Record<(typeof auditStatuses)[number], string> = {
  AUTO_PASSED: '无需复核',
  REVIEW_REQUIRED: '待复核',
  UNDER_REVIEW: '待复核',
  REVIEW_PASSED: '复核通过',
  REVIEW_REJECTED: '复核驳回'
};

export const deriveReviewStatusLabel = (input: { audit_status?: string | null }): string =>
  reviewStatusLabels[(input.audit_status ?? '') as keyof typeof reviewStatusLabels] ?? '待复核';
