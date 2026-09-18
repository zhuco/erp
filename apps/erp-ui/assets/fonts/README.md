# Bundled CJK font

`SourceHanSansSC-Regular.otf` is Source Han Sans SC Regular from Adobe's official
`adobe-fonts/source-han-sans` repository, pinned to release revision
`a4f7cf94edfb9d7ffbdfc4841de276358bd7e0f2`.

- Font SHA-256: `F1D8611151880C6C336AABEAC4640EF434FA13CBFBF1FFE82D0A71B2A5637256`
- License: SIL Open Font License 1.1, copied in `LICENSE-SourceHanSans.txt`
- Upstream: `https://github.com/adobe-fonts/source-han-sans`

The font is embedded into both native and WASM builds. Do not replace it with a
host-system font because the browser build must render the same glyphs offline.
