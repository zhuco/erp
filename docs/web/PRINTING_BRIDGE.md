# Printing Bridge

## Overview
- Location: `apps/erp-web/src/features/printing/PrintBridge.ts`
- API:
  - `detect()` → `{ success, error?, data?: { provider, version } }`
  - `printShippingLabel(payload)` → `{ success, error? }`
  - `getPrinters?()` → `{ success, error?, data?: printers[] }`
- Payload type: `apps/erp-web/src/features/printing/types.ts`

打印任务的审计由前端创建 `print_job` 完成，随后调用 PrintBridge 执行实际打印，并将结果回写到后端。

## 接入拼多多打印组件
1. 在页面中加载拼多多官方打印组件脚本（通常由运营平台提供）。
2. 将组件挂载到 `window.PddPrint`，并实现以下方法：
   - `printShippingLabel(payload)`：接收 `PrintShippingLabelPayload`，执行打印。
   - `getPrinters()`：可选，返回打印机列表。
   - `provider` / `version`：可选，便于 `detect()` 上报。
3. `PrintBridge` 会自动检测 `window.PddPrint` 并调用对应方法。

示例（伪代码）：
```ts
window.PddPrint = {
  provider: 'pdd',
  version: '1.2.3',
  async printShippingLabel(payload) {
    await PddSdk.print(payload);
  },
  async getPrinters() {
    return await PddSdk.listPrinters();
  }
};
```

## 扩展到京东 / 抖音
1. 将新的平台打印 SDK 暴露到 `window.PlatformPrint`（或复用 `window.PddPrint` 结构）。
2. 确保实现 `printShippingLabel`，以及可选的 `getPrinters`。
3. 若需要不同 payload 结构，可在 SDK 内部自行转换，但对外保持 `PrintShippingLabelPayload` 统一。

## 审计回写
`createPrintJobAndPrint` 会：
1. 创建 `print_job`（状态为 `QUEUED`，记录模板版本、操作者、设备标识）。
2. 调用 `PrintBridge.printShippingLabel` 执行打印。
3. 将结果回写为 `SUCCESS` 或 `FAILED`，供 `/print-jobs` 列表审计追踪。
