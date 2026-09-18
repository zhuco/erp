# ERP WASM 浏览器验收

组件CI检查：先执行`./scripts/Build-ErpWeb.ps1 -Release`，再执行
`node qa/acceptance/component-smoke.mjs`。脚本启动并仅关闭自己创建的静态服务，
使用独立浏览器会话和排他锁，运行现有首绘制/合成按键入口，检查30次状态转换、
release标记和页面运行异常；不需要API或数据库。结果保存在
`output/playwright/component-smoke`。已有服务时显式加`--existing-server`，不接管该进程。
共享CI只记录耗时，不用其代替固定设备的500ms性能门槛。当前工作流已编排该检查，
远端执行成功仍须以CI实际结果为准。真实保存仍使用独立`browser-smoke.mjs`。

此入口只服务 `apps/erp-ui/index.html`、`pkg/erp_ui.js`、`pkg/erp_ui_bg.wasm`，以及生成 glue 实际导入的两个 `pkg/snippets` 浏览器适配模块。固定监听 `127.0.0.1:15440`；业务 API 应由另一进程监听 `127.0.0.1:15441`。

在仓库根目录构建，必须使用统一入口：

```powershell
.\scripts\Invoke-ErpBuild.ps1 -Action build -Platform web -Scope grid
```

`Cargo.lock` 当前锁定 `wasm-bindgen 0.2.128`。打包前必须使用同版本 CLI；版本缺失或不一致时停止，不要改用 Venue 的 `0.2.126`，也不要全局安装：

```powershell
$WasmBindgen = '.\qa\acceptance\tools\wasm-bindgen-0.2.128-x86_64-pc-windows-msvc\wasm-bindgen.exe'
if ((& $WasmBindgen --version) -ne 'wasm-bindgen 0.2.128') {
    throw 'wasm-bindgen CLI must exactly match Cargo.lock (0.2.128)'
}
& $WasmBindgen `
    .\build\cargo\wasm32-unknown-unknown\debug\erp_ui.wasm `
    --target web `
    --out-dir .\apps\erp-ui\pkg `
    --out-name erp_ui `
    --no-typescript
if ($LASTEXITCODE -ne 0) { throw 'wasm-bindgen packaging failed' }
```

启动静态服务：

```powershell
node .\qa\acceptance\serve-web.mjs
```

浏览器访问 `http://127.0.0.1:15440/`。服务启动时会核对 `erp_ui.js`、`erp_ui_bg.wasm` 以及生成胶水中的 snippets 相对模块导入；缺文件、导入路径变化或端口占用都会明确失败。`.wasm` 使用 `application/wasm`，JavaScript 使用模块 MIME。服务不提供目录列表、源码树或仓库内其他文件。

跨端口业务请求需要 API 允许此 Web 来源；当前 API 已有 CORS 层。此静态服务不代理 API，也不读取或注入凭据。验收 API 使用 `--bind 127.0.0.1` 与显式隔离数据库设置，详见 README.md。
