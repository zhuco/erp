use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(v: Uuid) -> Self {
                Self(v)
            }
        }

        impl From<$name> for Uuid {
            fn from(v: $name) -> Self {
                v.0
            }
        }
    };
}

id_newtype!(TenantId);
id_newtype!(ShopId);
id_newtype!(UserId);
id_newtype!(RoleId);
id_newtype!(OrderId);
id_newtype!(InboxEventId);
id_newtype!(OutboxId);
id_newtype!(AfterSaleCaseId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Taobao,
    Jd,
    Pdd,
    Douyin,
}

#[derive(Debug, Error)]
pub enum PlatformParseError {
    #[error("unknown platform: {0}")]
    Unknown(String),
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Platform::Taobao => "taobao",
            Platform::Jd => "jd",
            Platform::Pdd => "pdd",
            Platform::Douyin => "douyin",
        };
        write!(f, "{s}")
    }
}

impl FromStr for Platform {
    type Err = PlatformParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "taobao" => Ok(Platform::Taobao),
            "jd" | "jingdong" => Ok(Platform::Jd),
            "pdd" | "pinduoduo" => Ok(Platform::Pdd),
            "douyin" | "dy" => Ok(Platform::Douyin),
            other => Err(PlatformParseError::Unknown(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Unknown,
    /// Order created but not paid.
    PendingPayment,
    /// Paid, waiting seller to ship / prepare.
    PendingShipment,
    /// Inventory allocated and awaiting fulfillment.
    Allocated,
    /// Part of the order shipped.
    PartiallyShipped,
    /// Shipped, waiting buyer to confirm receipt / delivery.
    Shipped,
    Completed,
    Cancelled,
    /// Closed by platform or system (e.g. timeout).
    Closed,
    /// Refund is in progress.
    Refunding,
    /// Refund finished.
    Refunded,
    /// On-hold / paused / locked.
    OnHold,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderStatus::Unknown => "UNKNOWN",
            OrderStatus::PendingPayment => "PENDING_PAYMENT",
            OrderStatus::PendingShipment => "PENDING_SHIPMENT",
            OrderStatus::Allocated => "ALLOCATED",
            OrderStatus::PartiallyShipped => "PARTIALLY_SHIPPED",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Cancelled => "CANCELLED",
            OrderStatus::Closed => "CLOSED",
            OrderStatus::Refunding => "REFUNDING",
            OrderStatus::Refunded => "REFUNDED",
            OrderStatus::OnHold => "ON_HOLD",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OmsOrderStatus {
    New,
    ReviewPending,
    #[serde(alias = "REVIEWED")]
    PendingShipment,
    Allocated,
    Picking,
    Packed,
    Shipped,
    Done,
    Cancelled,
}

impl fmt::Display for OmsOrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OmsOrderStatus::New => "NEW",
            OmsOrderStatus::ReviewPending => "REVIEW_PENDING",
            OmsOrderStatus::PendingShipment => "PENDING_SHIPMENT",
            OmsOrderStatus::Allocated => "ALLOCATED",
            OmsOrderStatus::Picking => "PICKING",
            OmsOrderStatus::Packed => "PACKED",
            OmsOrderStatus::Shipped => "SHIPPED",
            OmsOrderStatus::Done => "DONE",
            OmsOrderStatus::Cancelled => "CANCELLED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for OmsOrderStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NEW" => Ok(OmsOrderStatus::New),
            "REVIEW_PENDING" => Ok(OmsOrderStatus::ReviewPending),
            "REVIEWED" | "PENDING_SHIPMENT" => Ok(OmsOrderStatus::PendingShipment),
            "ALLOCATED" => Ok(OmsOrderStatus::Allocated),
            "PICKING" => Ok(OmsOrderStatus::Picking),
            "PACKED" => Ok(OmsOrderStatus::Packed),
            "SHIPPED" => Ok(OmsOrderStatus::Shipped),
            "DONE" => Ok(OmsOrderStatus::Done),
            "CANCELLED" => Ok(OmsOrderStatus::Cancelled),
            other => Err(format!("unknown oms order status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderMainStatus {
    Draft,
    PendingReview,
    ReviewRejected,
    ExceptionHold,
    PendingAllocate,
    PendingOutbound,
    Picking,
    Shipped,
    Signed,
    Completed,
    Cancelled,
}

impl fmt::Display for OrderMainStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderMainStatus::Draft => "DRAFT",
            OrderMainStatus::PendingReview => "PENDING_REVIEW",
            OrderMainStatus::ReviewRejected => "REVIEW_REJECTED",
            OrderMainStatus::ExceptionHold => "EXCEPTION_HOLD",
            OrderMainStatus::PendingAllocate => "PENDING_ALLOCATE",
            OrderMainStatus::PendingOutbound => "PENDING_OUTBOUND",
            OrderMainStatus::Picking => "PICKING",
            OrderMainStatus::Shipped => "SHIPPED",
            OrderMainStatus::Signed => "SIGNED",
            OrderMainStatus::Completed => "COMPLETED",
            OrderMainStatus::Cancelled => "CANCELLED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for OrderMainStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DRAFT" => Ok(OrderMainStatus::Draft),
            "PENDING_REVIEW" => Ok(OrderMainStatus::PendingReview),
            "REVIEW_REJECTED" => Ok(OrderMainStatus::ReviewRejected),
            "EXCEPTION_HOLD" => Ok(OrderMainStatus::ExceptionHold),
            "PENDING_ALLOCATE" => Ok(OrderMainStatus::PendingAllocate),
            "PENDING_OUTBOUND" => Ok(OrderMainStatus::PendingOutbound),
            "PICKING" => Ok(OrderMainStatus::Picking),
            "SHIPPED" => Ok(OrderMainStatus::Shipped),
            "SIGNED" => Ok(OrderMainStatus::Signed),
            "COMPLETED" => Ok(OrderMainStatus::Completed),
            "CANCELLED" => Ok(OrderMainStatus::Cancelled),
            other => Err(format!("unknown order status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderReviewStatus {
    NotReviewed,
    AutoApproved,
    #[serde(alias = "PENDING")]
    ManualReview,
    #[serde(alias = "PASSED")]
    Approved,
    #[serde(alias = "REJECTED")]
    Rejected,
    RolledBack,
}

impl fmt::Display for OrderReviewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderReviewStatus::NotReviewed => "NOT_REVIEWED",
            OrderReviewStatus::AutoApproved => "AUTO_APPROVED",
            OrderReviewStatus::ManualReview => "MANUAL_REVIEW",
            OrderReviewStatus::Approved => "APPROVED",
            OrderReviewStatus::Rejected => "REJECTED",
            OrderReviewStatus::RolledBack => "ROLLED_BACK",
        };
        write!(f, "{s}")
    }
}

impl FromStr for OrderReviewStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_REVIEWED" => Ok(OrderReviewStatus::NotReviewed),
            "AUTO_APPROVED" => Ok(OrderReviewStatus::AutoApproved),
            "MANUAL_REVIEW" | "PENDING" => Ok(OrderReviewStatus::ManualReview),
            "APPROVED" | "PASSED" => Ok(OrderReviewStatus::Approved),
            "REJECTED" => Ok(OrderReviewStatus::Rejected),
            "ROLLED_BACK" => Ok(OrderReviewStatus::RolledBack),
            other => Err(format!("unknown review status: {other}")),
        }
    }
}

impl OrderReviewStatus {
    pub fn is_pending(self) -> bool {
        matches!(
            self,
            OrderReviewStatus::NotReviewed
                | OrderReviewStatus::ManualReview
                | OrderReviewStatus::RolledBack
        )
    }

    pub fn is_approved(self) -> bool {
        matches!(
            self,
            OrderReviewStatus::AutoApproved | OrderReviewStatus::Approved
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderFulfillmentStatus {
    NotAllocated,
    Allocated,
    WaveCreated,
    Picking,
    Packing,
    ReadyToShip,
    Shipped,
    Signed,
}

impl fmt::Display for OrderFulfillmentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderFulfillmentStatus::NotAllocated => "NOT_ALLOCATED",
            OrderFulfillmentStatus::Allocated => "ALLOCATED",
            OrderFulfillmentStatus::WaveCreated => "WAVE_CREATED",
            OrderFulfillmentStatus::Picking => "PICKING",
            OrderFulfillmentStatus::Packing => "PACKING",
            OrderFulfillmentStatus::ReadyToShip => "READY_TO_SHIP",
            OrderFulfillmentStatus::Shipped => "SHIPPED",
            OrderFulfillmentStatus::Signed => "SIGNED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for OrderFulfillmentStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_ALLOCATED" => Ok(OrderFulfillmentStatus::NotAllocated),
            "ALLOCATED" => Ok(OrderFulfillmentStatus::Allocated),
            "WAVE_CREATED" => Ok(OrderFulfillmentStatus::WaveCreated),
            "PICKING" => Ok(OrderFulfillmentStatus::Picking),
            "PACKING" => Ok(OrderFulfillmentStatus::Packing),
            "READY_TO_SHIP" => Ok(OrderFulfillmentStatus::ReadyToShip),
            "SHIPPED" => Ok(OrderFulfillmentStatus::Shipped),
            "SIGNED" => Ok(OrderFulfillmentStatus::Signed),
            other => Err(format!("unknown fulfillment status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderExceptionStatus {
    None,
    Risk,
    AddressError,
    OutOfStock,
    PaymentIssue,
    CustomerBlacklist,
    ManualHold,
    Released,
    Closed,
}

impl fmt::Display for OrderExceptionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderExceptionStatus::None => "NONE",
            OrderExceptionStatus::Risk => "RISK",
            OrderExceptionStatus::AddressError => "ADDRESS_ERROR",
            OrderExceptionStatus::OutOfStock => "OUT_OF_STOCK",
            OrderExceptionStatus::PaymentIssue => "PAYMENT_ISSUE",
            OrderExceptionStatus::CustomerBlacklist => "CUSTOMER_BLACKLIST",
            OrderExceptionStatus::ManualHold => "MANUAL_HOLD",
            OrderExceptionStatus::Released => "RELEASED",
            OrderExceptionStatus::Closed => "CLOSED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for OrderExceptionStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NONE" => Ok(OrderExceptionStatus::None),
            "RISK" => Ok(OrderExceptionStatus::Risk),
            "ADDRESS_ERROR" => Ok(OrderExceptionStatus::AddressError),
            "OUT_OF_STOCK" => Ok(OrderExceptionStatus::OutOfStock),
            "PAYMENT_ISSUE" => Ok(OrderExceptionStatus::PaymentIssue),
            "CUSTOMER_BLACKLIST" => Ok(OrderExceptionStatus::CustomerBlacklist),
            "MANUAL_HOLD" => Ok(OrderExceptionStatus::ManualHold),
            "RELEASED" => Ok(OrderExceptionStatus::Released),
            "CLOSED" => Ok(OrderExceptionStatus::Closed),
            other => Err(format!("unknown exception status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderPaymentStatus {
    Unpaid,
    Paid,
    PartRefunded,
    Refunded,
    Chargeback,
}

impl fmt::Display for OrderPaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderPaymentStatus::Unpaid => "UNPAID",
            OrderPaymentStatus::Paid => "PAID",
            OrderPaymentStatus::PartRefunded => "PART_REFUNDED",
            OrderPaymentStatus::Refunded => "REFUNDED",
            OrderPaymentStatus::Chargeback => "CHARGEBACK",
        };
        write!(f, "{s}")
    }
}

impl FromStr for OrderPaymentStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "UNPAID" => Ok(OrderPaymentStatus::Unpaid),
            "PAID" => Ok(OrderPaymentStatus::Paid),
            "PART_REFUNDED" => Ok(OrderPaymentStatus::PartRefunded),
            "REFUNDED" => Ok(OrderPaymentStatus::Refunded),
            "CHARGEBACK" => Ok(OrderPaymentStatus::Chargeback),
            other => Err(format!("unknown payment status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutboundStatus {
    Draft,
    PendingReview,
    PendingAllocate,
    PendingPick,
    Picking,
    PendingVerify,
    PendingPack,
    PendingShip,
    Shipped,
    Closed,
}

impl fmt::Display for OutboundStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OutboundStatus::Draft => "DRAFT",
            OutboundStatus::PendingReview => "PENDING_REVIEW",
            OutboundStatus::PendingAllocate => "PENDING_ALLOCATE",
            OutboundStatus::PendingPick => "PENDING_PICK",
            OutboundStatus::Picking => "PICKING",
            OutboundStatus::PendingVerify => "PENDING_VERIFY",
            OutboundStatus::PendingPack => "PENDING_PACK",
            OutboundStatus::PendingShip => "PENDING_SHIP",
            OutboundStatus::Shipped => "SHIPPED",
            OutboundStatus::Closed => "CLOSED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for OutboundStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DRAFT" => Ok(OutboundStatus::Draft),
            "PENDING_REVIEW" => Ok(OutboundStatus::PendingReview),
            "PENDING_ALLOCATE" => Ok(OutboundStatus::PendingAllocate),
            "PENDING_PICK" => Ok(OutboundStatus::PendingPick),
            "PICKING" => Ok(OutboundStatus::Picking),
            "PENDING_VERIFY" => Ok(OutboundStatus::PendingVerify),
            "PENDING_PACK" => Ok(OutboundStatus::PendingPack),
            "PENDING_SHIP" => Ok(OutboundStatus::PendingShip),
            "SHIPPED" => Ok(OutboundStatus::Shipped),
            "CLOSED" => Ok(OutboundStatus::Closed),
            other => Err(format!("unknown outbound status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PickStatus {
    NotPicked,
    Picking,
    Picked,
    PickException,
}

impl fmt::Display for PickStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PickStatus::NotPicked => "NOT_PICKED",
            PickStatus::Picking => "PICKING",
            PickStatus::Picked => "PICKED",
            PickStatus::PickException => "PICK_EXCEPTION",
        };
        write!(f, "{s}")
    }
}

impl FromStr for PickStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_PICKED" => Ok(PickStatus::NotPicked),
            "PICKING" => Ok(PickStatus::Picking),
            "PICKED" => Ok(PickStatus::Picked),
            "PICK_EXCEPTION" => Ok(PickStatus::PickException),
            other => Err(format!("unknown pick status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerifyStatus {
    NotVerified,
    Verified,
    VerifyException,
}

impl fmt::Display for VerifyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            VerifyStatus::NotVerified => "NOT_VERIFIED",
            VerifyStatus::Verified => "VERIFIED",
            VerifyStatus::VerifyException => "VERIFY_EXCEPTION",
        };
        write!(f, "{s}")
    }
}

impl FromStr for VerifyStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_VERIFIED" => Ok(VerifyStatus::NotVerified),
            "VERIFIED" => Ok(VerifyStatus::Verified),
            "VERIFY_EXCEPTION" => Ok(VerifyStatus::VerifyException),
            other => Err(format!("unknown verify status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PackStatus {
    NotPacked,
    Packing,
    Packed,
}

impl fmt::Display for PackStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PackStatus::NotPacked => "NOT_PACKED",
            PackStatus::Packing => "PACKING",
            PackStatus::Packed => "PACKED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for PackStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_PACKED" => Ok(PackStatus::NotPacked),
            "PACKING" => Ok(PackStatus::Packing),
            "PACKED" => Ok(PackStatus::Packed),
            other => Err(format!("unknown pack status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShipStatus {
    NotShipped,
    Shipping,
    Shipped,
    ShipFailed,
}

impl fmt::Display for ShipStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ShipStatus::NotShipped => "NOT_SHIPPED",
            ShipStatus::Shipping => "SHIPPING",
            ShipStatus::Shipped => "SHIPPED",
            ShipStatus::ShipFailed => "SHIP_FAILED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ShipStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_SHIPPED" => Ok(ShipStatus::NotShipped),
            "SHIPPING" => Ok(ShipStatus::Shipping),
            "SHIPPED" => Ok(ShipStatus::Shipped),
            "SHIP_FAILED" => Ok(ShipStatus::ShipFailed),
            other => Err(format!("unknown ship status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InboundStatus {
    Draft,
    PendingReview,
    PendingReceipt,
    Receiving,
    QcPending,
    PendingPutaway,
    Putawaying,
    Completed,
    Closed,
}

impl fmt::Display for InboundStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InboundStatus::Draft => "DRAFT",
            InboundStatus::PendingReview => "PENDING_REVIEW",
            InboundStatus::PendingReceipt => "PENDING_RECEIPT",
            InboundStatus::Receiving => "RECEIVING",
            InboundStatus::QcPending => "QC_PENDING",
            InboundStatus::PendingPutaway => "PENDING_PUTAWAY",
            InboundStatus::Putawaying => "PUTAWAYING",
            InboundStatus::Completed => "COMPLETED",
            InboundStatus::Closed => "CLOSED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for InboundStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DRAFT" => Ok(InboundStatus::Draft),
            "PENDING_REVIEW" => Ok(InboundStatus::PendingReview),
            "PENDING_RECEIPT" => Ok(InboundStatus::PendingReceipt),
            "RECEIVING" => Ok(InboundStatus::Receiving),
            "QC_PENDING" => Ok(InboundStatus::QcPending),
            "PENDING_PUTAWAY" => Ok(InboundStatus::PendingPutaway),
            "PUTAWAYING" => Ok(InboundStatus::Putawaying),
            "COMPLETED" => Ok(InboundStatus::Completed),
            "CLOSED" => Ok(InboundStatus::Closed),
            other => Err(format!("unknown inbound status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptStatus {
    NotReceived,
    PartReceived,
    Received,
}

impl fmt::Display for ReceiptStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ReceiptStatus::NotReceived => "NOT_RECEIVED",
            ReceiptStatus::PartReceived => "PART_RECEIVED",
            ReceiptStatus::Received => "RECEIVED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ReceiptStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_RECEIVED" => Ok(ReceiptStatus::NotReceived),
            "PART_RECEIVED" => Ok(ReceiptStatus::PartReceived),
            "RECEIVED" => Ok(ReceiptStatus::Received),
            other => Err(format!("unknown receipt status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InboundQcStatus {
    NotRequired,
    Pending,
    Passed,
    Failed,
}

impl fmt::Display for InboundQcStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InboundQcStatus::NotRequired => "NOT_REQUIRED",
            InboundQcStatus::Pending => "PENDING",
            InboundQcStatus::Passed => "PASSED",
            InboundQcStatus::Failed => "FAILED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for InboundQcStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_REQUIRED" => Ok(InboundQcStatus::NotRequired),
            "PENDING" => Ok(InboundQcStatus::Pending),
            "PASSED" => Ok(InboundQcStatus::Passed),
            "FAILED" => Ok(InboundQcStatus::Failed),
            other => Err(format!("unknown inbound qc status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PutawayStatus {
    NotPutaway,
    PartPutaway,
    Putawayed,
}

impl fmt::Display for PutawayStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PutawayStatus::NotPutaway => "NOT_PUTAWAY",
            PutawayStatus::PartPutaway => "PART_PUTAWAY",
            PutawayStatus::Putawayed => "PUTAWAYED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for PutawayStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_PUTAWAY" => Ok(PutawayStatus::NotPutaway),
            "PART_PUTAWAY" => Ok(PutawayStatus::PartPutaway),
            "PUTAWAYED" => Ok(PutawayStatus::Putawayed),
            other => Err(format!("unknown putaway status: {other}")),
        }
    }
}

/// Refund state at **order-item** (sub-order) level.
///
/// Notes:
/// - This is intentionally coarse-grained and cross-platform.
/// - Platform-specific fine-grained codes should be preserved in `raw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundState {
    /// No refund / after-sale refund on this line.
    #[default]
    None,
    /// Refund/return is in progress.
    Processing,
    /// Refund finished successfully.
    Success,
    /// Refund/after-sale was closed/cancelled/rejected.
    Closed,
    /// Platform returned a state we don't recognize.
    Unknown,
}

impl fmt::Display for RefundState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RefundState::None => "NONE",
            RefundState::Processing => "PROCESSING",
            RefundState::Success => "SUCCESS",
            RefundState::Closed => "CLOSED",
            RefundState::Unknown => "UNKNOWN",
        };
        write!(f, "{s}")
    }
}

impl FromStr for RefundState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NONE" => Ok(RefundState::None),
            "PROCESSING" => Ok(RefundState::Processing),
            "SUCCESS" => Ok(RefundState::Success),
            "CLOSED" => Ok(RefundState::Closed),
            "UNKNOWN" => Ok(RefundState::Unknown),
            other => Err(format!("unknown refund state: {other}")),
        }
    }
}

/// After-sale state at **order-item** (sub-order) level.
///
/// Covers non-refund after-sale flows too (exchange / reship / repair, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AfterSaleState {
    #[default]
    None,
    Processing,
    Success,
    Closed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AfterSalesType {
    RefundOnly,
    ReturnRefund,
    Exchange,
    Resend,
}

impl fmt::Display for AfterSalesType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AfterSalesType::RefundOnly => "REFUND_ONLY",
            AfterSalesType::ReturnRefund => "RETURN_REFUND",
            AfterSalesType::Exchange => "EXCHANGE",
            AfterSalesType::Resend => "RESEND",
        };
        write!(f, "{s}")
    }
}

impl FromStr for AfterSalesType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "REFUND_ONLY" => Ok(AfterSalesType::RefundOnly),
            "RETURN_REFUND" => Ok(AfterSalesType::ReturnRefund),
            "EXCHANGE" => Ok(AfterSalesType::Exchange),
            "RESEND" => Ok(AfterSalesType::Resend),
            other => Err(format!("unknown after-sales type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AfterSalesStatus {
    Open,
    Approved,
    Received,
    Refunded,
    Closed,
    Rejected,
}

impl fmt::Display for AfterSalesStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AfterSalesStatus::Open => "OPEN",
            AfterSalesStatus::Approved => "APPROVED",
            AfterSalesStatus::Received => "RECEIVED",
            AfterSalesStatus::Refunded => "REFUNDED",
            AfterSalesStatus::Closed => "CLOSED",
            AfterSalesStatus::Rejected => "REJECTED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for AfterSalesStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "OPEN" => Ok(AfterSalesStatus::Open),
            "APPROVED" => Ok(AfterSalesStatus::Approved),
            "RECEIVED" => Ok(AfterSalesStatus::Received),
            "REFUNDED" => Ok(AfterSalesStatus::Refunded),
            "CLOSED" => Ok(AfterSalesStatus::Closed),
            "REJECTED" => Ok(AfterSalesStatus::Rejected),
            other => Err(format!("unknown after-sales status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReturnReceiptStatus {
    Receiving,
    PutawayDone,
    Cancelled,
}

impl fmt::Display for ReturnReceiptStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ReturnReceiptStatus::Receiving => "RECEIVING",
            ReturnReceiptStatus::PutawayDone => "PUTAWAY_DONE",
            ReturnReceiptStatus::Cancelled => "CANCELLED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ReturnReceiptStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "RECEIVING" => Ok(ReturnReceiptStatus::Receiving),
            "PUTAWAY_DONE" => Ok(ReturnReceiptStatus::PutawayDone),
            "CANCELLED" => Ok(ReturnReceiptStatus::Cancelled),
            other => Err(format!("unknown return receipt status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Pending,
    Sent,
    Failed,
    Done,
}

impl fmt::Display for RefundStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RefundStatus::Pending => "PENDING",
            RefundStatus::Sent => "SENT",
            RefundStatus::Failed => "FAILED",
            RefundStatus::Done => "DONE",
        };
        write!(f, "{s}")
    }
}

impl FromStr for RefundStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PENDING" => Ok(RefundStatus::Pending),
            "SENT" => Ok(RefundStatus::Sent),
            "FAILED" => Ok(RefundStatus::Failed),
            "DONE" => Ok(RefundStatus::Done),
            other => Err(format!("unknown refund status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompensationStatus {
    NotRequired,
    PendingApproval,
    Approved,
    Paid,
    Rejected,
}

impl fmt::Display for CompensationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CompensationStatus::NotRequired => "NOT_REQUIRED",
            CompensationStatus::PendingApproval => "PENDING_APPROVAL",
            CompensationStatus::Approved => "APPROVED",
            CompensationStatus::Paid => "PAID",
            CompensationStatus::Rejected => "REJECTED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for CompensationStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_REQUIRED" => Ok(CompensationStatus::NotRequired),
            "PENDING_APPROVAL" => Ok(CompensationStatus::PendingApproval),
            "APPROVED" => Ok(CompensationStatus::Approved),
            "PAID" => Ok(CompensationStatus::Paid),
            "REJECTED" => Ok(CompensationStatus::Rejected),
            other => Err(format!("unknown compensation status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceStatus {
    NotRequired,
    Required,
    Submitted,
    Verified,
    Rejected,
}

impl fmt::Display for EvidenceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EvidenceStatus::NotRequired => "NOT_REQUIRED",
            EvidenceStatus::Required => "REQUIRED",
            EvidenceStatus::Submitted => "SUBMITTED",
            EvidenceStatus::Verified => "VERIFIED",
            EvidenceStatus::Rejected => "REJECTED",
        };
        write!(f, "{s}")
    }
}

impl FromStr for EvidenceStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NOT_REQUIRED" => Ok(EvidenceStatus::NotRequired),
            "REQUIRED" => Ok(EvidenceStatus::Required),
            "SUBMITTED" => Ok(EvidenceStatus::Submitted),
            "VERIFIED" => Ok(EvidenceStatus::Verified),
            "REJECTED" => Ok(EvidenceStatus::Rejected),
            other => Err(format!("unknown evidence status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundChannel {
    Platform,
    Manual,
}

impl fmt::Display for RefundChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RefundChannel::Platform => "PLATFORM",
            RefundChannel::Manual => "MANUAL",
        };
        write!(f, "{s}")
    }
}

impl FromStr for RefundChannel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PLATFORM" => Ok(RefundChannel::Platform),
            "MANUAL" => Ok(RefundChannel::Manual),
            other => Err(format!("unknown refund channel: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PermissionCode {
    SysPermissionRead,
    SysRoleRead,
    SysRoleWrite,
    SysUserRead,
    SysUserWrite,
    SysAuditRead,
    SysSettingsRead,
    SysSettingsWrite,
    SysDlqRead,
    SysDlqReplay,
    OmsOrderRead,
    OmsOrderWrite,
    OmsOrderRaw,
    OmsOrderReview,
    OmsShippingRead,
    OmsShippingWrite,
    OmsAfterSalesRead,
    OmsAfterSalesRaw,
    OmsAfterSalesWrite,
    ManufacturingWrite,
    QualityConcession,
    WmsInventoryRead,
    WmsInventoryWrite,
    BasicSkuRead,
    BasicSkuWrite,
    BasicKitRead,
    BasicKitWrite,
    BasicChannelRead,
    BasicChannelWrite,
    BasicCourierRead,
    BasicCourierWrite,
    BasicSupplierRead,
    BasicSupplierWrite,
    BasicCustomerRead,
    BasicCustomerWrite,
    ProcurementOrderRead,
    ProcurementOrderWrite,
    ProcurementSuggestionRead,
    ProcurementReturnRead,
    ProcurementReturnWrite,
    ProcurementRequestRead,
    ProcurementRequestWrite,
    ProcurementPlanRead,
    ProcurementPlanWrite,
    ProcurementSourcingRead,
    ProcurementSourcingWrite,
    ProcurementInspectionRead,
    ProcurementInspectionWrite,
    ProcurementReconciliationRead,
    ProcurementReconciliationWrite,
    ProcurementPerformanceRead,
    ProcurementPerformanceWrite,
    BasicMappingRead,
    BasicMappingWrite,
    BasicWarehouseRead,
    BasicWarehouseWrite,
    BasicCarrierRead,
    BasicCarrierWrite,
    PrintDeviceRead,
    PrintDeviceWrite,
    PrintTemplateRead,
    PrintTemplateWrite,
    PrintJobRead,
    PrintJobWrite,
    PrintMappingRead,
    PrintMappingWrite,
    OpsOperationLogRead,
    OpsReportRead,
    OpsExportRead,
    OpsExportWrite,
    BasicLogisticsProductRead,
    BasicLogisticsProductWrite,
    BasicLogisticsPriceRead,
    BasicLogisticsPriceWrite,
    BasicLogisticsMatchRuleRead,
    BasicLogisticsMatchRuleWrite,
    BasicOrderTagRead,
    BasicOrderTagWrite,
    BasicSkuTagRead,
    BasicSkuTagWrite,
    BasicShopProductRead,
    BasicShopProductWrite,
    MarketingPriceRead,
    MarketingPriceWrite,
    MarketingPromotionRead,
    MarketingPromotionWrite,
    MarketingGiftRead,
    MarketingGiftWrite,
    MarketingCouponRead,
    MarketingCouponWrite,
    MarketingCostRead,
    MarketingCostWrite,
    MarketingPostmortemRead,
    MarketingPostmortemWrite,
}

impl PermissionCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            PermissionCode::SysPermissionRead => "SYS.PERMISSION.READ",
            PermissionCode::SysRoleRead => "SYS.ROLE.READ",
            PermissionCode::SysRoleWrite => "SYS.ROLE.WRITE",
            PermissionCode::SysUserRead => "SYS.USER.READ",
            PermissionCode::SysUserWrite => "SYS.USER.WRITE",
            PermissionCode::SysAuditRead => "SYS.AUDIT.READ",
            PermissionCode::SysSettingsRead => "SYS.SETTINGS.READ",
            PermissionCode::SysSettingsWrite => "SYS.SETTINGS.WRITE",
            PermissionCode::SysDlqRead => "SYS.DLQ.READ",
            PermissionCode::SysDlqReplay => "SYS.DLQ.REPLAY",
            PermissionCode::OmsOrderRead => "OMS.ORDER.READ",
            PermissionCode::OmsOrderWrite => "OMS.ORDER.WRITE",
            PermissionCode::OmsOrderRaw => "OMS.ORDER.RAW",
            PermissionCode::OmsOrderReview => "OMS.ORDER.REVIEW",
            PermissionCode::OmsShippingRead => "OMS.SHIPPING.READ",
            PermissionCode::OmsShippingWrite => "OMS.SHIPPING.WRITE",
            PermissionCode::OmsAfterSalesRead => "OMS.AFTERSALES.READ",
            PermissionCode::OmsAfterSalesRaw => "OMS.AFTERSALES.RAW",
            PermissionCode::OmsAfterSalesWrite => "OMS.AFTERSALES.WRITE",
            PermissionCode::ManufacturingWrite => "MFG.PRODUCTION.WRITE",
            PermissionCode::QualityConcession => "QUALITY.CONCESSION",
            PermissionCode::WmsInventoryRead => "WMS.INVENTORY.READ",
            PermissionCode::WmsInventoryWrite => "WMS.INVENTORY.WRITE",
            PermissionCode::BasicSkuRead => "BASIC.SKU.READ",
            PermissionCode::BasicSkuWrite => "BASIC.SKU.WRITE",
            PermissionCode::BasicKitRead => "BASIC.KIT.READ",
            PermissionCode::BasicKitWrite => "BASIC.KIT.WRITE",
            PermissionCode::BasicChannelRead => "BASIC.CHANNEL.READ",
            PermissionCode::BasicChannelWrite => "BASIC.CHANNEL.WRITE",
            PermissionCode::BasicCourierRead => "BASIC.COURIER.READ",
            PermissionCode::BasicCourierWrite => "BASIC.COURIER.WRITE",
            PermissionCode::BasicSupplierRead => "BASIC.SUPPLIER.READ",
            PermissionCode::BasicSupplierWrite => "BASIC.SUPPLIER.WRITE",
            PermissionCode::BasicCustomerRead => "BASIC.CUSTOMER.READ",
            PermissionCode::BasicCustomerWrite => "BASIC.CUSTOMER.WRITE",
            PermissionCode::ProcurementOrderRead => "PROCUREMENT.ORDER.READ",
            PermissionCode::ProcurementOrderWrite => "PROCUREMENT.ORDER.WRITE",
            PermissionCode::ProcurementSuggestionRead => "PROCUREMENT.SUGGESTION.READ",
            PermissionCode::ProcurementReturnRead => "PROCUREMENT.RETURN.READ",
            PermissionCode::ProcurementReturnWrite => "PROCUREMENT.RETURN.WRITE",
            PermissionCode::ProcurementRequestRead => "PROCUREMENT.REQUEST.READ",
            PermissionCode::ProcurementRequestWrite => "PROCUREMENT.REQUEST.WRITE",
            PermissionCode::ProcurementPlanRead => "PROCUREMENT.PLAN.READ",
            PermissionCode::ProcurementPlanWrite => "PROCUREMENT.PLAN.WRITE",
            PermissionCode::ProcurementSourcingRead => "PROCUREMENT.SOURCING.READ",
            PermissionCode::ProcurementSourcingWrite => "PROCUREMENT.SOURCING.WRITE",
            PermissionCode::ProcurementInspectionRead => "PROCUREMENT.INSPECTION.READ",
            PermissionCode::ProcurementInspectionWrite => "PROCUREMENT.INSPECTION.WRITE",
            PermissionCode::ProcurementReconciliationRead => "PROCUREMENT.RECONCILIATION.READ",
            PermissionCode::ProcurementReconciliationWrite => "PROCUREMENT.RECONCILIATION.WRITE",
            PermissionCode::ProcurementPerformanceRead => "PROCUREMENT.PERFORMANCE.READ",
            PermissionCode::ProcurementPerformanceWrite => "PROCUREMENT.PERFORMANCE.WRITE",
            PermissionCode::BasicMappingRead => "BASIC.MAPPING.READ",
            PermissionCode::BasicMappingWrite => "BASIC.MAPPING.WRITE",
            PermissionCode::BasicWarehouseRead => "BASIC.WAREHOUSE.READ",
            PermissionCode::BasicWarehouseWrite => "BASIC.WAREHOUSE.WRITE",
            PermissionCode::BasicCarrierRead => "BASIC.CARRIER.READ",
            PermissionCode::BasicCarrierWrite => "BASIC.CARRIER.WRITE",
            PermissionCode::PrintDeviceRead => "PRINT.DEVICE.READ",
            PermissionCode::PrintDeviceWrite => "PRINT.DEVICE.WRITE",
            PermissionCode::PrintTemplateRead => "PRINT.TEMPLATE.READ",
            PermissionCode::PrintTemplateWrite => "PRINT.TEMPLATE.WRITE",
            PermissionCode::PrintJobRead => "PRINT.JOB.READ",
            PermissionCode::PrintJobWrite => "PRINT.JOB.WRITE",
            PermissionCode::PrintMappingRead => "PRINT.MAPPING.READ",
            PermissionCode::PrintMappingWrite => "PRINT.MAPPING.WRITE",
            PermissionCode::OpsOperationLogRead => "OPS.OPERATION_LOG.READ",
            PermissionCode::OpsReportRead => "OPS.REPORT.READ",
            PermissionCode::OpsExportRead => "OPS.EXPORT.READ",
            PermissionCode::OpsExportWrite => "OPS.EXPORT.WRITE",
            PermissionCode::BasicLogisticsProductRead => "BASIC.LOGISTICS_PRODUCT.READ",
            PermissionCode::BasicLogisticsProductWrite => "BASIC.LOGISTICS_PRODUCT.WRITE",
            PermissionCode::BasicLogisticsPriceRead => "BASIC.LOGISTICS_PRICE.READ",
            PermissionCode::BasicLogisticsPriceWrite => "BASIC.LOGISTICS_PRICE.WRITE",
            PermissionCode::BasicLogisticsMatchRuleRead => "BASIC.LOGISTICS_MATCH_RULE.READ",
            PermissionCode::BasicLogisticsMatchRuleWrite => "BASIC.LOGISTICS_MATCH_RULE.WRITE",
            PermissionCode::BasicOrderTagRead => "BASIC.ORDER_TAG.READ",
            PermissionCode::BasicOrderTagWrite => "BASIC.ORDER_TAG.WRITE",
            PermissionCode::BasicSkuTagRead => "BASIC.SKU_TAG.READ",
            PermissionCode::BasicSkuTagWrite => "BASIC.SKU_TAG.WRITE",
            PermissionCode::BasicShopProductRead => "BASIC.SHOP_PRODUCT.READ",
            PermissionCode::BasicShopProductWrite => "BASIC.SHOP_PRODUCT.WRITE",
            PermissionCode::MarketingPriceRead => "MARKETING.PRICE.READ",
            PermissionCode::MarketingPriceWrite => "MARKETING.PRICE.WRITE",
            PermissionCode::MarketingPromotionRead => "MARKETING.PROMOTION.READ",
            PermissionCode::MarketingPromotionWrite => "MARKETING.PROMOTION.WRITE",
            PermissionCode::MarketingGiftRead => "MARKETING.GIFT.READ",
            PermissionCode::MarketingGiftWrite => "MARKETING.GIFT.WRITE",
            PermissionCode::MarketingCouponRead => "MARKETING.COUPON.READ",
            PermissionCode::MarketingCouponWrite => "MARKETING.COUPON.WRITE",
            PermissionCode::MarketingCostRead => "MARKETING.COST.READ",
            PermissionCode::MarketingCostWrite => "MARKETING.COST.WRITE",
            PermissionCode::MarketingPostmortemRead => "MARKETING.POSTMORTEM.READ",
            PermissionCode::MarketingPostmortemWrite => "MARKETING.POSTMORTEM.WRITE",
        }
    }

    pub const fn all() -> &'static [PermissionCode] {
        &[
            PermissionCode::SysPermissionRead,
            PermissionCode::SysRoleRead,
            PermissionCode::SysRoleWrite,
            PermissionCode::SysUserRead,
            PermissionCode::SysUserWrite,
            PermissionCode::SysAuditRead,
            PermissionCode::SysSettingsRead,
            PermissionCode::SysSettingsWrite,
            PermissionCode::SysDlqRead,
            PermissionCode::SysDlqReplay,
            PermissionCode::OmsOrderRead,
            PermissionCode::OmsOrderWrite,
            PermissionCode::OmsOrderRaw,
            PermissionCode::OmsOrderReview,
            PermissionCode::OmsShippingRead,
            PermissionCode::OmsShippingWrite,
            PermissionCode::OmsAfterSalesRead,
            PermissionCode::OmsAfterSalesRaw,
            PermissionCode::OmsAfterSalesWrite,
            PermissionCode::ManufacturingWrite,
            PermissionCode::QualityConcession,
            PermissionCode::WmsInventoryRead,
            PermissionCode::WmsInventoryWrite,
            PermissionCode::BasicSkuRead,
            PermissionCode::BasicSkuWrite,
            PermissionCode::BasicKitRead,
            PermissionCode::BasicKitWrite,
            PermissionCode::BasicChannelRead,
            PermissionCode::BasicChannelWrite,
            PermissionCode::BasicCourierRead,
            PermissionCode::BasicCourierWrite,
            PermissionCode::BasicSupplierRead,
            PermissionCode::BasicSupplierWrite,
            PermissionCode::BasicCustomerRead,
            PermissionCode::BasicCustomerWrite,
            PermissionCode::ProcurementOrderRead,
            PermissionCode::ProcurementOrderWrite,
            PermissionCode::ProcurementSuggestionRead,
            PermissionCode::ProcurementReturnRead,
            PermissionCode::ProcurementReturnWrite,
            PermissionCode::ProcurementRequestRead,
            PermissionCode::ProcurementRequestWrite,
            PermissionCode::ProcurementPlanRead,
            PermissionCode::ProcurementPlanWrite,
            PermissionCode::ProcurementSourcingRead,
            PermissionCode::ProcurementSourcingWrite,
            PermissionCode::ProcurementInspectionRead,
            PermissionCode::ProcurementInspectionWrite,
            PermissionCode::ProcurementReconciliationRead,
            PermissionCode::ProcurementReconciliationWrite,
            PermissionCode::ProcurementPerformanceRead,
            PermissionCode::ProcurementPerformanceWrite,
            PermissionCode::BasicMappingRead,
            PermissionCode::BasicMappingWrite,
            PermissionCode::BasicWarehouseRead,
            PermissionCode::BasicWarehouseWrite,
            PermissionCode::BasicCarrierRead,
            PermissionCode::BasicCarrierWrite,
            PermissionCode::PrintDeviceRead,
            PermissionCode::PrintDeviceWrite,
            PermissionCode::PrintTemplateRead,
            PermissionCode::PrintTemplateWrite,
            PermissionCode::PrintJobRead,
            PermissionCode::PrintJobWrite,
            PermissionCode::PrintMappingRead,
            PermissionCode::PrintMappingWrite,
            PermissionCode::OpsOperationLogRead,
            PermissionCode::OpsReportRead,
            PermissionCode::OpsExportRead,
            PermissionCode::OpsExportWrite,
            PermissionCode::BasicLogisticsProductRead,
            PermissionCode::BasicLogisticsProductWrite,
            PermissionCode::BasicLogisticsPriceRead,
            PermissionCode::BasicLogisticsPriceWrite,
            PermissionCode::BasicLogisticsMatchRuleRead,
            PermissionCode::BasicLogisticsMatchRuleWrite,
            PermissionCode::BasicOrderTagRead,
            PermissionCode::BasicOrderTagWrite,
            PermissionCode::BasicSkuTagRead,
            PermissionCode::BasicSkuTagWrite,
            PermissionCode::BasicShopProductRead,
            PermissionCode::BasicShopProductWrite,
            PermissionCode::MarketingPriceRead,
            PermissionCode::MarketingPriceWrite,
            PermissionCode::MarketingPromotionRead,
            PermissionCode::MarketingPromotionWrite,
            PermissionCode::MarketingGiftRead,
            PermissionCode::MarketingGiftWrite,
            PermissionCode::MarketingCouponRead,
            PermissionCode::MarketingCouponWrite,
            PermissionCode::MarketingCostRead,
            PermissionCode::MarketingCostWrite,
            PermissionCode::MarketingPostmortemRead,
            PermissionCode::MarketingPostmortemWrite,
        ]
    }
}

impl fmt::Display for AfterSaleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AfterSaleState::None => "NONE",
            AfterSaleState::Processing => "PROCESSING",
            AfterSaleState::Success => "SUCCESS",
            AfterSaleState::Closed => "CLOSED",
            AfterSaleState::Unknown => "UNKNOWN",
        };
        write!(f, "{s}")
    }
}

impl FromStr for AfterSaleState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NONE" => Ok(AfterSaleState::None),
            "PROCESSING" => Ok(AfterSaleState::Processing),
            "SUCCESS" => Ok(AfterSaleState::Success),
            "CLOSED" => Ok(AfterSaleState::Closed),
            "UNKNOWN" => Ok(AfterSaleState::Unknown),
            other => Err(format!("unknown after_sale state: {other}")),
        }
    }
}
