# ERP 统计功能开发说明

## 1. 目标与范围
- 覆盖 `/stats/*` 统计页面的核心指标、相差对比与维度明细。
- 统一统计接口，前端通过 `stats_type` 区分业务场景。
- 统计相差 = 当前周期指标 - 对比周期指标，并输出变化率。

## 2. 统计接口设计

### 2.1 统一接口
`GET /v1/tenants/{tenant_id}/stats/{stats_type}`

### 2.2 stats_type 与前端方法命名
| stats_type | 页面 | 前端方法 | 说明 |
| --- | --- | --- | --- |
| `sales` | 销售统计 | `getSalesStats` | 销售总览与退款率 |
| `shipping` | 发货统计 | `getShippingStats` | 发货时效与异常 |
| `sales-daily` | 销售日报表 | `getSalesDailyStats` | 日维度 GMV/订单/时效 |
| `sales-trends` | 销售趋势分析 | `getSalesTrendStats` | 同比、环比与趋势 |
| `slow-moving` | 滞销分析 | `getSlowMovingStats` | 滞销与周转指标 |
| `best-sellers` | 畅销分析 | `getBestSellerStats` | 畅销 SKU 指标 |
| `sales-cycle` | 销售周期 | `getSalesCycleStats` | 履约周期与达成率 |

### 2.3 与洞察域 canonical 功能映射
| stats_type | 当前页面名 | 重构后功能名 | canonical route |
| --- | --- | --- | --- |
| `sales` | 销售统计 | 交易总览 | `/insights/revenue/overview` |
| `sales-daily` | 销售日报表 | 营运日报 | `/insights/revenue/daily-brief` |
| `sales-trends` | 销售趋势分析 | 增长雷达 | `/insights/revenue/trend-radar` |
| `shipping` | 发货统计 | 履约时效 | `/insights/fulfillment/lead-time` |
| `slow-moving` | 滞销分析 | 长尾沉积 | `/insights/catalog/slow-sku` |
| `best-sellers` | 畅销分析 | 爆品热度 | `/insights/catalog/hot-sku` |
| `sales-cycle` | 销售周期 | 订单履约周期 | `/insights/fulfillment/order-cycle` |

> 更完整的功能细化规格见：`F:/erp/docs/spec/18_insights_function_details.md`

## 3. 请求参数
| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `start_at` | string(RFC3339) | 是 | 当前统计周期开始时间 |
| `end_at` | string(RFC3339) | 是 | 当前统计周期结束时间 |
| `compare_start_at` | string(RFC3339) | 否 | 对比周期开始时间 |
| `compare_end_at` | string(RFC3339) | 否 | 对比周期结束时间 |
| `dimension` | string | 否 | 维度：`channel`/`warehouse`/`category` |
| `metrics` | string | 否 | 指标列表，逗号分隔（如 `order_count,refund_rate`） |
| `timezone` | string | 否 | IANA 时区（默认使用租户时区） |

> 未传 `metrics` 时，按 `stats_type` 默认指标返回。

## 4. 响应结构
```json
{
  "stats_type": "sales",
  "current": {
    "sales_amount_cent": 1250000,
    "order_count": 980,
    "avg_order_value_cent": 1275,
    "refund_rate": 3.2
  },
  "compare": {
    "sales_amount_cent": 1180000,
    "order_count": 910,
    "avg_order_value_cent": 1290,
    "refund_rate": 2.9
  },
  "breakdown": [
    {
      "dimension": "天猫",
      "dimension_label": "天猫",
      "metrics": {
        "sales_amount_cent": 520000,
        "order_count": 360
      }
    }
  ],
  "updated_at": "2026-02-15T10:20:30Z"
}
```

## 5. 指标清单

### 5.1 销售统计 `sales`
| 指标 key | 标题 | 单位 | 说明 |
| --- | --- | --- | --- |
| `sales_amount_cent` | 销售额 | 分 | GMV 金额 |
| `order_count` | 订单量 | 单 | 成交订单数 |
| `avg_order_value_cent` | 客单价 | 分 | 平均订单金额 |
| `refund_rate` | 退款率 | % | 退款金额/GMV |

### 5.2 发货统计 `shipping`
| 指标 key | 标题 | 单位 | 说明 |
| --- | --- | --- | --- |
| `shipment_count` | 发货单量 | 单 | 发货单数量 |
| `on_time_rate` | 准时发货率 | % | 准时发货占比 |
| `exception_count` | 异常发货数 | 单 | 超时/异常发货 |
| `avg_outbound_hours` | 平均出库时长 | 小时 | 出库到发货耗时 |

### 5.3 销售日报表 `sales-daily`
| 指标 key | 标题 | 单位 | 说明 |
| --- | --- | --- | --- |
| `daily_sales_cent` | 日GMV | 分 | 当日成交金额 |
| `daily_order_count` | 日订单数 | 单 | 当日订单量 |
| `daily_refund_rate` | 日退款率 | % | 当日退款占比 |
| `daily_on_time_rate` | 日发货及时率 | % | 当日履约及时率 |

### 5.4 销售趋势分析 `sales-trends`
| 指标 key | 标题 | 单位 | 说明 |
| --- | --- | --- | --- |
| `yoy_growth_rate` | 同比增长 | % | 与去年同期对比 |
| `mom_growth_rate` | 环比增长 | % | 与上一周期对比 |
| `moving_avg_7d_cent` | 7日移动均值 | 分 | 7 日均值 |
| `season_peak_cent` | 旺季峰值 | 分 | 峰值 GMV |
| `channel_trend_index` | 渠道趋势指数 | 分 | 趋势综合评分 |

### 5.5 滞销分析 `slow-moving`
| 指标 key | 标题 | 单位 | 说明 |
| --- | --- | --- | --- |
| `slow_sku_count` | 滞销SKU数 | 个 | 滞销 SKU 数量 |
| `slow_stock_amount_cent` | 滞销库存额 | 分 | 滞销库存金额 |
| `avg_turnover_days` | 平均周转天数 | 天 | 周转平均值 |
| `dead_stock_ratio` | 呆滞占比 | % | 呆滞库存占比 |

### 5.6 畅销分析 `best-sellers`
| 指标 key | 标题 | 单位 | 说明 |
| --- | --- | --- | --- |
| `top_sku_sales_cent` | 畅销SKU销售额 | 分 | TOP SKU GMV |
| `top_sku_order_count` | 畅销SKU订单量 | 单 | TOP SKU 订单量 |
| `top_sku_share` | 畅销SKU贡献率 | % | 销售贡献占比 |
| `repeat_purchase_rate` | 复购率 | % | 复购占比 |

### 5.7 销售周期 `sales-cycle`
| 指标 key | 标题 | 单位 | 说明 |
| --- | --- | --- | --- |
| `avg_order_cycle_hours` | 下单到发货 | 小时 | 下单到发货耗时 |
| `avg_ship_cycle_hours` | 发货到签收 | 小时 | 发货到签收耗时 |
| `avg_return_cycle_days` | 退货周期 | 天 | 退货处理周期 |
| `on_time_delivery_rate` | 按时履约率 | % | 按时履约占比 |

## 6. 相差计算规则
- 相差值：`diff = current - compare`
- 变化率：`diff_rate = diff / compare`（`compare = 0` 时返回 `null`）
- 前端展示：
  - 正值显示绿色，上升箭头。
  - 负值显示红色，下降箭头。
  - 无对比周期时仅显示当前值。

## 7. 前端实现位置
- API：`apps/erp-web/src/api/stats.ts`
- 相差计算工具：`apps/erp-web/src/utils/statsDiff.ts`
- 统计看板：`apps/erp-web/src/pages/stats/StatsPlaceholder.tsx`
- React Query Key：`['stats', stats_type, query]`

## 8. 备注
- `metrics` 传空时返回默认指标，保持页面可渲染。
- `dimension` 为空时不返回 `breakdown`，前端提示“选择统计维度后展示明细”。
- 金额类统一使用 **分** 存储，前端展示时换算为 **元**。
