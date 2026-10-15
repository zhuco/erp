"""Preserve earlier OMS evidence; fork a verifier for this thirteen-fault batch."""
from pathlib import Path

base = Path(__file__).resolve().parent
source = (base / 'free-erp-fulfillment-verify.py').read_text(encoding='utf8')
source = source.replace("if args.stage == 'original':", "if args.stage == 'atomic-original':")
source = source.replace("summary['failed'] == 5", "summary['failed'] == 13")
source = source.replace('five original failures reproduced', 'thirteen original authority/record failures reproduced')
source = source.replace("original_failures_expected=args.stage == 'original'", "original_failures_expected=args.stage == 'atomic-original'")
# These fixtures have no corrupted stock identity: independently reconcile all
# physical facts in the original interval as well as the repaired intervals.
start = source.index("    if args.stage != 'original':\n")
end = source.index('\nartifacts = {}', start)
block = source[start:end]
first = block.index('\n') + 1
else_at = block.index('\n    else:\n')
block = block[first:else_at]
block = '\n'.join(line[4:] if line.startswith('    ') else line for line in block.split('\n'))
source = source[:start] + block + '\n' + source[end:]
destination = base / 'free-erp-fulfillment-atomic-verify.py'
if destination.exists():
    raise SystemExit('refusing to replace independent verifier')
destination.write_text(source, encoding='utf8')
print('forked bounded OMS authority/records verifier')
