# Retired React UI source

The user selected the shared Rust/egui desktop and WASM UI and does not require
the old React pages to remain executable. On 2026-09-10, 975 old application files
were archived before retirement. `source.zip` contains source/configuration/tests,
not installed `node_modules`. `manifest.json` records each relative path, size and
SHA-256, plus the complete archive checksum.

`scripts/Retire-LegacyWeb.ps1` verifies the archive entries and unchanged source
hashes, rejects paths outside the exact former app and reparse-point ancestors,
and removes only files listed in that verified manifest. It does not delete a
project directory, unlisted files, databases or build caches. Empty directories
and pre-existing installed links may remain; they are not an executable app.

This archive is a recovery/reference artifact, not a runtime fallback. There is
no React package manifest in the active workspace. The package manager regenerated
the active lock without its dependencies. Do not restore the old engine into the
active application to make a missing Rust feature appear complete.

The former repository baseline checker is preserved as
`verify_repo_baseline.before.mjs`. React route/file-presence checks have been
replaced by Rust entrypoint checks. Backend integration targets are retained;
grid browser smoke is a separate real API test, not proof that all historical
order/WMS/finance UI pages have been rebuilt.
