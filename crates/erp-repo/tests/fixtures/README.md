# After-sale fixtures

All fixtures use fake/masked values and contain no real PII.

## PDD
- `pdd_aftersale_refund_processing.json`: Refund in progress (after_sales_status=2, refund_status=2).
- `pdd_aftersale_refund_success.json`: Refund completed (after_sales_status=10, refund_status=4).
- `pdd_aftersale_exchange.json`: Exchange/reship style after-sale (after_sales_status=14, non-refund).

## Douyin
- `douyin_aftersale_refund.json`: Sub-order refund with `after_sale_info` present.
- `douyin_aftersale_only_refund.json`: Only-refund flow (`after_sale_type_desc=仅退款`).
- `douyin_aftersale_return_refund.json`: Return+refund flow (`after_sale_type_desc=退货退款`).
