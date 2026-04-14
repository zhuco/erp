import type { AfterSalesStatus, AfterSalesType, ReturnReceipt } from '@/api/afterSalesOps';

export type AfterSalesActionContext = {
  status: AfterSalesStatus;
  type: AfterSalesType;
  sla_status?: string | null;
  return_receipt?: ReturnReceipt | null;
  canWrite: boolean;
};

export type AfterSalesActionFlags = {
  canApprove: boolean;
  canReject: boolean;
  canReceive: boolean;
  canCreateShipment: boolean;
  canRefund: boolean;
  canCancelReceive: boolean;
  canRedoReceive: boolean;
  canPutaway: boolean;
  canEscalate: boolean;
};

const forbiddenRefundStatuses: AfterSalesStatus[] = ['REJECTED', 'REFUNDED', 'CLOSED'];

const shipmentStatuses: AfterSalesStatus[] = ['APPROVED', 'RECEIVED'];
const shipmentTypes: AfterSalesType[] = ['EXCHANGE', 'RESEND'];

export const getAfterSalesActionFlags = (context: AfterSalesActionContext): AfterSalesActionFlags => {
  const { status, type, sla_status, return_receipt, canWrite } = context;
  const returnStatus = return_receipt?.status;
  const hasReturnReceipt = Boolean(return_receipt);

  const canApprove = canWrite && status === 'OPEN';
  const canReject = canWrite && status === 'OPEN';
  const canReceive = canWrite && status === 'APPROVED';
  const canCreateShipment = canWrite && shipmentStatuses.includes(status) && shipmentTypes.includes(type);
  const canRefund = canWrite && !forbiddenRefundStatuses.includes(status);
  const canCancelReceive = canWrite && hasReturnReceipt && returnStatus !== 'CANCELLED';
  const canRedoReceive = canWrite && returnStatus === 'CANCELLED';
  const canPutaway = canWrite && returnStatus === 'RECEIVING';
  const canEscalate = canWrite && sla_status === 'OVERDUE';

  return {
    canApprove,
    canReject,
    canReceive,
    canCreateShipment,
    canRefund,
    canCancelReceive,
    canRedoReceive,
    canPutaway,
    canEscalate
  };
};
