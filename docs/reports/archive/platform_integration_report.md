# 平台联调验收报告（M3）

更新日期：2026-01-17  
范围：OAuth/验签/拉单/回传/库存回写（真实订单 E2E）

## 环境与前提
- 代码适配器：`crates/adapter-taobao`、`crates/adapter-jd`、`crates/adapter-pdd`、`crates/adapter-douyin`
- 联调指导：`docs/codex/21_platform_adapter_signing_and_mapping.md`
- 当前缺失：真实平台账号/应用密钥/回调地址，无法触发真实订单全链路验收

## 本次执行记录
- 完成适配器目录与接口入口梳理，确认各平台 adapter 已接入仓库。
- 未执行真实平台 OAuth 授权、Webhook 验签、拉单/回传/库存回写链路（凭证缺失）。
- 未生成真实订单 E2E 验收截图/日志。

## 阻塞项与问题清单
1. **缺少平台凭证**：需要真实平台 AppKey/Secret、回调域名及白名单。
2. **签名向量未全量固化**：PDD/抖店官方示例向量需补充单测（见联调指导 A1）。
3. **Webhook 严格模式校验**：需落地 `shop.credentials.webhook_verification` 配置与验签策略（见联调指导 A2）。
4. **字段映射核对**：需按 `docs/spec/03_order_field_mapping.md` 逐平台复核关键字段。

## 下一步计划
1. 申请并配置真实平台账号/密钥（OAuth 回调、Webhook URL）。
2. 按 `docs/ops/LOCAL_RUN_WINDOWS.md` 模式 B 拉起全链路环境。
3. 完成真实订单拉单与回传，输出日志与验收截图。
4. 在本报告补充 E2E 结果与问题闭环。
