"""Compare a clean source build with a timestamped Mainnet bytecode hash."""
import datetime
import hashlib
import json
import pathlib
import sys

binary, snapshot_path, source_commit, output_path = sys.argv[1:]
data = pathlib.Path(binary).read_bytes()
snapshot = json.loads(pathlib.Path(snapshot_path).read_text())
assert len(source_commit) == 40 and all(c in "0123456789abcdef" for c in source_commit)
digest = hashlib.sha256(data.rstrip(b"\0")).hexdigest()
matches = digest == snapshot["normalizedProgramSha256"]
report = {
    "checkedAtUtc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "sourceRepository": "https://github.com/heisto-dev/heisto-contracts",
    "sourceCommit": source_commit,
    "programId": snapshot["programId"],
    "chainSnapshotUtc": snapshot["observedAtUtc"],
    "chainContextSlot": snapshot["contextSlot"],
    "deploymentSlot": snapshot["deploymentSlot"],
    "builtBytes": len(data),
    "builtRawSha256": hashlib.sha256(data).hexdigest(),
    "builtNormalizedSha256": digest,
    "onChainNormalizedSha256": snapshot["normalizedProgramSha256"],
    "matchesSnapshot": matches,
    "publicExplorerVerification": "not established by this build",
    "audit": "not an independent security audit",
}
pathlib.Path(output_path).write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
if not matches:
    raise SystemExit("Source build does not match the chain snapshot; do not claim verified.")
