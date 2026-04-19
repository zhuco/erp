from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

BOM = b"\xef\xbb\xbf"
BINARY_EXTENSIONS = {
    ".avif",
    ".bmp",
    ".class",
    ".dll",
    ".dylib",
    ".eot",
    ".exe",
    ".gif",
    ".gz",
    ".ico",
    ".jar",
    ".jpeg",
    ".jpg",
    ".mp3",
    ".mp4",
    ".otf",
    ".pdf",
    ".png",
    ".so",
    ".svgz",
    ".tar",
    ".ttf",
    ".wasm",
    ".webm",
    ".webp",
    ".woff",
    ".woff2",
    ".zip",
}

REPO_ROOT = Path(__file__).resolve().parents[1]
CJK_RUN_RE = re.compile(r"[\u3400-\u9fff]{2,}")

# Typical mojibake glyphs produced when UTF-8 Chinese text is decoded as GBK/GB18030.
MOJIBAKE_HINT_CHARS = set("鍙鍏鍗鍒鎺鎻鏇缂搴骞鏆杩璇澶鍥皟淇瀵瑙閲摵彴潈辫棤")
MOJIBAKE_HINT_TOKENS = (
    "授权",
    "刷新",
    "更新",
    "失败",
    "发起",
    "回调",
    "暂无",
    "取消",
    "保存",
    "平台",
    "店铺",
    "编辑",
    "加载",
    "下载",
    "列表",
    "删除",
    "策略",
    "数据",
    "任务",
    "规则",
    "配置",
    "详情",
    "标记",
    "提交",
    "导入",
    "重试",
    "返回",
)
MOJIBAKE_TOKEN_MAP: dict[str, str] = {
    "授权": "授权",
    "发起": "发起",
    "回调": "回调",
    "刷新": "刷新",
    "更新": "更新",
    "删除": "删除",
    "取消": "取消",
    "保存": "保存",
    "暂无": "暂无",
    "提交": "提交",
    "请求": "请求",
    "失败": "失败",
    "成功": "成功",
    "加载": "加载",
    "下载": "下载",
    "导入": "导入",
    "返回": "返回",
    "任务": "任务",
    "编号": "编号",
    "不能为空": "不能为空",
    "平台": "平台",
    "店铺": "店铺",
    "编辑": "编辑",
    "配置": "配置",
    "规则": "规则",
    "策略": "策略",
    "数据": "数据",
    "列表": "列表",
    "详情": "详情",
    "标记": "标记",
    "客户": "客户",
    "分群": "分群",
    "积分流水": "积分流水",
    "订单": "订单",
    "货品": "货品",
    "售后": "售后",
    "网店": "网店",
    "权限": "权限",
    "修改": "修改",
    "明细": "明细",
    "单据": "单据",
    "重试": "重试",
    "重放": "重放",
    "波次": "波次",
    "质检": "质检",
    "入库": "入库",
    "异常": "异常",
    "打印": "打印",
    "模板": "模板",
    "确认": "确认",
    "高SKU": "高SKU",
}

# Common Simplified Chinese characters for simple language-likeness scoring.
COMMON_ZH_CHARS = set(
    "\u7684\u4e00\u662f\u5728\u4e0d\u4e86\u6709\u548c\u4eba\u8fd9\u4e2d"
    "\u5927\u4e3a\u4e0a\u4e2a\u56fd\u6211\u4ee5\u8981\u4ed6\u65f6\u6765"
    "\u7528\u4eec\u751f\u5230\u4f5c\u5730\u4e8e\u51fa\u5c31\u5206\u5bf9"
    "\u6210\u4f1a\u53ef\u4e3b\u53d1\u5e74\u52a8\u540c\u5de5\u4e5f\u80fd"
    "\u4e0b\u8fc7\u5b50\u8bf4\u4ea7\u79cd\u9762\u800c\u65b9\u540e\u591a"
    "\u5b9a\u884c\u5b66\u6cd5\u6240\u6c11\u5f97\u7ecf\u5341\u4e09\u4e4b"
    "\u8fdb\u7740\u7b49\u63d0\u4ea4\u5220\u9664\u8fd4\u56de\u4efb\u52a1"
    "\u5931\u8d25\u6210\u529f\u4e0b\u8f7d\u5bfc\u5165\u7f16\u8f91\u4fdd"
    "\u5b58\u5e73\u53f0\u5e97\u94fa\u8be6\u60c5\u914d\u7f6e\u89c4\u5219"
    "\u72b6\u6001\u5ba2\u6237\u6a21\u677f\u52a0\u8f7d\u5217\u8868\u8ba2"
    "\u5355\u8d27\u54c1\u552e\u540e"
)


def list_git_files(staged_only: bool) -> list[Path]:
    args = (
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z"]
        if staged_only
        else ["git", "ls-files", "-z"]
    )
    output = subprocess.check_output(args, cwd=REPO_ROOT)
    return [
        REPO_ROOT / item
        for item in output.decode("utf-8").split("\0")
        if item.strip()
    ]


def is_binary_file(file_path: Path, data: bytes) -> bool:
    if file_path.suffix.lower() in BINARY_EXTENSIONS:
        return True
    return b"\x00" in data[:4096]


def decode_surrogate_run(raw_bytes: bytes) -> str:
    try:
        return raw_bytes.decode("gb18030")
    except UnicodeDecodeError:
        try:
            return raw_bytes.decode("gbk")
        except UnicodeDecodeError:
            return raw_bytes.decode("latin1")


def repair_mixed_utf8_gbk(data: bytes) -> bytes:
    text = data.decode("utf-8", errors="surrogateescape")
    output_parts: list[str] = []
    i = 0
    while i < len(text):
        code_point = ord(text[i])
        if 0xDC80 <= code_point <= 0xDCFF:
            raw = []
            while i < len(text):
                cp = ord(text[i])
                if 0xDC80 <= cp <= 0xDCFF:
                    raw.append(cp - 0xDC00)
                    i += 1
                    continue
                break
            output_parts.append(decode_surrogate_run(bytes(raw)))
            continue
        output_parts.append(text[i])
        i += 1
    return "".join(output_parts).encode("utf-8")


def count_in_set(text: str, charset: set[str]) -> int:
    return sum(1 for ch in text if ch in charset)


def maybe_repair_mojibake_run(run: str) -> str | None:
    hint_src = count_in_set(run, MOJIBAKE_HINT_CHARS)
    has_hint_token = any(token in run for token in MOJIBAKE_HINT_TOKENS)
    if not has_hint_token and hint_src < 2:
        return None
    common_src = count_in_set(run, COMMON_ZH_CHARS)

    try:
        decoded = run.encode("gb18030").decode("utf-8")
    except UnicodeDecodeError:
        return None

    if decoded == run:
        return None
    if "\ufffd" in decoded:
        return None
    if not CJK_RUN_RE.search(decoded):
        return None

    hint_dst = count_in_set(decoded, MOJIBAKE_HINT_CHARS)
    common_dst = count_in_set(decoded, COMMON_ZH_CHARS)
    source_score = common_src * 3 - hint_src * 2
    decoded_score = common_dst * 3 - hint_dst * 2
    if decoded_score < source_score + 2:
        return None
    if hint_dst > hint_src:
        return None
    return decoded


def repair_text_mojibake(text: str) -> tuple[str, bool]:
    changed = False

    def replace_run(match: re.Match[str]) -> str:
        nonlocal changed
        run = match.group(0)
        repaired = maybe_repair_mojibake_run(run)
        if repaired and repaired != run:
            changed = True
            return repaired
        return run

    repaired_text = CJK_RUN_RE.sub(replace_run, text)
    return repaired_text, changed


def replace_mojibake_tokens(text: str) -> tuple[str, bool]:
    changed = False
    normalized = text
    for key in sorted(MOJIBAKE_TOKEN_MAP.keys(), key=len, reverse=True):
        replacement = MOJIBAKE_TOKEN_MAP[key]
        if key in normalized:
            normalized = normalized.replace(key, replacement)
            changed = True
    return normalized, changed


def normalize_file(file_path: Path, dry_run: bool) -> tuple[bool, bool, bool, bool]:
    original = file_path.read_bytes()
    if not original or is_binary_file(file_path, original):
        return False, False, False, False

    had_bom = original.startswith(BOM)
    data = original[3:] if had_bom else original

    had_invalid_utf8 = False
    had_mojibake_repair = False
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        had_invalid_utf8 = True
        text = repair_mixed_utf8_gbk(data).decode("utf-8")

    repaired_text, repaired_mojibake = repair_text_mojibake(text)
    token_fixed_text, token_repaired = replace_mojibake_tokens(repaired_text)
    had_mojibake_repair = repaired_mojibake or token_repaired
    normalized = token_fixed_text.encode("utf-8")

    changed = normalized != original
    if changed and not dry_run:
        file_path.write_bytes(normalized)

    return changed, had_bom, had_invalid_utf8, had_mojibake_repair


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Normalize tracked text files to UTF-8 without BOM."
    )
    parser.add_argument(
        "--staged",
        action="store_true",
        help="Only normalize staged files.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Report what would change without writing files.",
    )
    args = parser.parse_args()

    files = list_git_files(staged_only=args.staged)

    changed_count = 0
    bom_count = 0
    repaired_count = 0
    mojibake_repaired_count = 0

    for file_path in files:
        if not file_path.exists() or not file_path.is_file():
            continue
        changed, had_bom, had_invalid, had_mojibake = normalize_file(
            file_path=file_path,
            dry_run=args.dry_run,
        )
        if had_bom:
            bom_count += 1
        if had_invalid:
            repaired_count += 1
        if had_mojibake:
            mojibake_repaired_count += 1
        if changed:
            changed_count += 1

    scope = "staged files" if args.staged else "tracked files"
    mode = "dry-run" if args.dry_run else "write"
    print(
        f"[utf8-fix] Completed ({mode}) for {scope}: changed={changed_count}, "
        f"bom_removed={bom_count}, invalid_repaired={repaired_count}, "
        f"mojibake_repaired={mojibake_repaired_count}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
