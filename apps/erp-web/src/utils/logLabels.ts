export const operationActionLabels: Record<string, string> = {
  SHIPMENT_REQUESTED: '发货申请',
  SHIPMENT_CALLBACK: '发货回调',
  SHIPMENT_CONFIRMED: '发货确认',
  INVENTORY_ADJUSTED: '库存调整',
  INVENTORY_INBOUND: '库存入库',
  INVENTORY_OUTBOUND: '库存出库',
  INVENTORY_RESERVED: '库存占用',
  INVENTORY_RELEASED: '库存释放',
  INVENTORY_SYNC_RETRY: '库存同步重试',
  INVENTORY_SYNC_RUN: '库存同步执行',
  INVENTORY_FREEZE: '库存冻结',
  INVENTORY_UNFREEZE: '库存解冻',
  RETURN_STOCKED: '退货入库',
  'OMS.ALLOCATED': '订单分配',
  'OMS.REVIEW_PASS': '订单审核通过',
  'OMS.REVIEW_REJECT': '订单审核拒绝',
  'OMS.REVIEW_FLAG': '订单审核拦截',
  'OMS.REVIEW_AUTO_PASS': '自动审核通过',
  'OMS.REVIEW_AUTO_REJECT': '自动审核拒绝',
  'OMS.REVIEW_AUTO_FLAG': '自动审核拦截',
  'OMS.EXCEPTION_RESOLVE': '异常处理完成',
  'OMS.EXCEPTION_REJECT': '异常驳回',
  'OMS.EXCEPTION_REVOKE': '异常撤回',
  'OMS.SHIPMENT_CREATED': 'OMS发货单创建',
  'OMS.SHIPMENT_SHIPPED': 'OMS发货完成',
  'OMS.ORDER_SPLIT_SOURCE': '订单拆分原单',
  'OMS.ORDER_SPLIT_CHILD': '订单拆分子单',
  'OMS.ORDER_EDIT': '订单编辑',
  'OMS.ORDER_RECEIVER_UPDATE': '修改收货信息',
  'OMS.RESEND_CREATED': '补发创建',
  'OMS.RESERVATION_RELEASED': '释放库存占用',
  'OMS.SHIPMENT_CANCEL_REQUESTED': '发货取消申请',
  STOCK_COUNT_CREATED: '盘点创建',
  STOCK_COUNT_CONFIRMED: '盘点确认',
  STOCK_COUNT_APPLIED: '盘点生效',
  STOCK_TRANSFER_CREATED: '调拨创建',
  STOCK_TRANSFER_OUT: '调拨出库',
  STOCK_TRANSFER_IN: '调拨入库',
  STOCK_ADJUST_CREATED: '调整单创建',
  STOCK_ADJUST_APPLIED: '调整单生效',
  STOCK_MOVE_CREATED: '移库单创建',
  STOCK_MOVE_APPLIED: '移库单生效',
  'WMS.INBOUND_CREATED': '入库单创建',
  'WMS.INBOUND_AUDITED': '入库单审核',
  'WMS.INBOUND_RECEIVED': '入库收货',
  'WMS.INBOUND_PUTAWAY': '入库上架',
  'WMS.INBOUND_INSPECTED': '入库质检',
  'WMS.OUTBOUND_AUDITED': '出库单审核',
  'WMS.OUTBOUND_CHECKED': '出库复核',
  'WMS.OUTBOUND_WEIGHED': '出库称重'
};

export const operationRefTypeLabels: Record<string, string> = {
  ORDER: '订单',
  ORDER_ITEM: '订单明细',
  SHIPMENT: '发货单',
  OMS_SHIPMENT: 'OMS发货单',
  OUTBOUND_DOC: '出库单',
  INBOUND_DOC: '入库单',
  WAVE: '波次',
  INVENTORY: '库存',
  INVENTORY_FREEZE: '库存冻结',
  SKU: '商品',
  PRODUCT: '商品',
  KIT: '组合装',
  STOCK_COUNT: '库存盘点',
  STOCK_TRANSFER: '库存调拨',
  STOCK_ADJUST: '库存调整',
  STOCK_MOVE: '库存移库',
  STOCK_SYNC_JOB: '库存同步任务',
  SKU_SERIAL: '序列号'
};

export const auditEntityTypeLabels: Record<string, string> = {
  after_sales: '售后单',
  after_sales_quality_rule: '售后质检规则',
  after_sales_reason: '售后原因',
  ap_doc: '应付单',
  carrier: '承运商',
  customer: '客户',
  CUSTOMER_BLACKLIST: '客户黑名单',
  customer_category: '客户分类',
  customer_tag: '客户标签',
  EXPORT_JOB: '导出任务',
  INBOUND_DOC: '入库单',
  INVENTORY_BALANCE: '库存余额',
  INVENTORY_FREEZE: '库存冻结',
  invoice: '发票',
  logistics_match_rule: '物流匹配规则',
  logistics_price: '物流报价',
  logistics_product: '物流产品',
  member_level: '会员等级',
  ORDER: '订单',
  ORDER_REVIEW_RULE: '订单审核规则',
  order_tag_definition: '订单标签',
  OUTBOUND_DOC: '出库单',
  PICK_RECOMMEND_STRATEGY: '拣货推荐策略',
  PICK_TASK: '拣货任务',
  platform_statement: '平台对账单',
  print_job: '打印任务',
  print_template: '打印模板',
  PRODUCT: '商品',
  purchase_order: '采购单',
  purchase_return: '采购退货',
  reconcile_job: '对账任务',
  refund_doc: '退款单',
  shipping_statement_lines: '物流结算明细',
  shop_carrier_account: '店铺承运商账号',
  SKU: '商品',
  sku_tag_definition: 'SKU标签',
  KIT: '组合装',
  STOCK_ADJUST: '库存调整单',
  STOCK_COUNT: '库存盘点',
  STOCK_MOVE: '库存移库',
  STOCK_SYNC_JOB: '库存同步任务',
  STOCK_TRANSFER: '库存调拨',
  supplier: '供应商',
  WAVE: '波次'
};

export const actorTypeLabels: Record<string, string> = {
  SYSTEM: '系统',
  USER: '用户',
  API: '接口',
  JOB: '任务',
  WORKER: '任务'
};

export const resolveOperationActionLabel = (action?: string | null) => {
  if (!action) {
    return '-';
  }
  return operationActionLabels[action] ?? action;
};

export const resolveOperationRefTypeLabel = (refType?: string | null) => {
  if (!refType) {
    return '-';
  }
  return operationRefTypeLabels[refType] ?? refType;
};

export const resolveAuditEntityTypeLabel = (entityType?: string | null) => {
  if (!entityType) {
    return '-';
  }
  return auditEntityTypeLabels[entityType] ?? entityType;
};

export const resolveActorTypeLabel = (actorType?: string | null) => {
  if (!actorType) {
    return '-';
  }
  return actorTypeLabels[actorType] ?? actorType;
};
