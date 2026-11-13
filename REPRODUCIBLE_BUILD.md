# Heisto source build verification

This procedure is read-only. It does not deploy or upgrade a program, change
authority, connect a funded wallet, sign a transaction or spend SOL.

**Verification is not an independent security audit.**

## Identities and scope

- Mainnet program: `FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi`
- [Exact source revision](https://github.com/heisto-dev/heisto-contracts/tree/358de359889de10843745d4124114a7db54b54cd)
- [Latest public build run](https://github.com/heisto-dev/heisto-contracts/actions/runs/43408695606)
- [Public registry status](https://verify.osec.io/status/FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi)

The build run, not this document, establishes whether compilation and comparison
succeeded. A workflow merely being present or started is not a verified build.
Read the final conclusion and comparison artifact before making a public claim.

## Current result: not verified

The clean build compiled successfully but **did not match** the finalized
Mainnet program. No source-build verification or audit is claimed.

| Attempt | Build parameters | Outcome |
| --- | --- | --- |
| [Initial](https://github.com/heisto-dev/heisto-contracts/actions/runs/43407536670) | Pinned image, platform-tools v1.57, SBF v3 | Compiled; comparison failed |
| [Size optimization](https://github.com/heisto-dev/heisto-contracts/actions/runs/43408218333) | Same inputs plus recorded size optimization | Compiled; comparison failed |
| [Workspace root](https://github.com/heisto-dev/heisto-contracts/actions/runs/43408695606) | Size-optimized build from workspace root | Compiled; comparison failed with the same output as the preceding attempt |

Latest rebuilt executable:

- Bytes: **312,416**
- Raw SHA-256: `75f9306c32fed398061c159cc47f9dbf7fa2c8515d7397b0d718883c328a0b7f`
- Normalized SHA-256: `b7759bc6daae90091a688570cff48a84b3e8b571fc05a3c7af41503b0a52bbc8`

Finalized Mainnet snapshot:

- Allocated program payload: **312,584 bytes**
- Normalized SHA-256: `059c48c52301b0653af169ad2b06640a0ff1ef1283478d2c6cb448f6e7ea8d8e`
- The retained release binary still matches the on-chain payload.
- Deployment slot and authority were unchanged in the post-build recheck.

[Machine-readable build comparison](evidence/source-build-comparison.json) ·
[Post-build chain recheck](evidence/source-verification-recheck.json)

The precise remaining cause has not been established. The build environment,
compiler invocation and release inputs require further investigation. A small
size difference does not establish functional equivalence, and this result alone
does not establish malicious code or an exploit.

**No contract upgrade, authority change, metadata transaction or SOL spending was
performed.** Do not hide a mismatch by silently replacing the deployed binary.

## Read-only reproduction

Use an isolated Linux host with Docker. Clone only the public contract repository.
Never mount a working directory containing environment files or wallet material
into a build container.

```sh
git clone https://github.com/heisto-dev/heisto-contracts
cd heisto-contracts
git checkout --detach 358de359889de10843745d4124114a7db54b54cd

curl --fail --location --proto '=https' --tlsv1.2 \
  https://github.com/solana-foundation/solana-verifiable-build/releases/download/v0.5.2/solana-verify-0.5.2-linux \
  --output /tmp/solana-verify
echo "b9d5ccce9634dc14269cab3e7a58649bc141b3306e7735a94478c936341f211f  /tmp/solana-verify" | sha256sum --check
chmod +x /tmp/solana-verify

/tmp/solana-verify build \
  --base-image solanafoundation/solana-verifiable-build@sha256:12fd4c0a0790f0fc41ef74b0cdb6bccc167ba6137eb1adb749bac649481c86bd \
  --arch v3 \
  --cargo-build-sbf-args="--tools-version v1.57 --optimize-size"

/tmp/solana-verify get-executable-hash target/deploy/heisto.so
/tmp/solana-verify --url https://api.mainnet-beta.solana.com \
  get-program-hash FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi
```

Compare the two normalized hashes. Public RPC endpoints can rate-limit; an RPC
failure is not evidence of a match. Re-check the deployment slot and authority
when repeating the verification because the program is upgradeable.

The image digest is the official verifier's Agave 4.3.0 image mapping at verifier
release 0.5.2; the platform-tools version and SBF architecture are supplied
explicitly. Do not substitute a floating image tag or regenerate Cargo.lock.
The pipeline checks that build inputs remain unchanged.
The retained release compiler fingerprint includes `-C opt-level=s`; reproduce
it using `--optimize-size`. A build without that option produced different bytes.
The [initial run without size optimization](https://github.com/heisto-dev/heisto-contracts/actions/runs/43407536670)
is retained as failed comparison evidence, not hidden or presented as a success.

## Snapshot comparison

The public workflow also compares the freshly built executable against
`evidence/source-verification-snapshot.json`. It uses SHA-256 after trimming
trailing zero padding, as the verifier does. Do not hash the 45-byte upgradeable
loader metadata header as program code.

A snapshot is historical evidence, not a claim that the program will remain
unchanged. A retained release binary matching a snapshot alone does not prove
that a fresh source build matches it.

## Explorer registration is a separate operation

OtterSec's public verification registry reads repository, commit and build
arguments from an Otter Verify metadata PDA. Posting arbitrary repository
fields to an old API example does not replace that requirement.

Creating the metadata PDA requires a signed on-chain transaction and account
rent plus transaction fees. That is separate from the read-only build above.
Do not upload production signing keys to CI. Obtain explicit approval for an
exact metadata transaction and spending limit before publishing it.

No contract redeployment or authority transfer should be performed merely to
make a failed build comparison pass.

## References

- [Solana verified-build documentation](https://solana.com/docs/programs/verified-builds)
- [Verifier source and release](https://github.com/solana-foundation/solana-verifiable-build/releases/tag/v0.5.2)
- [Heisto security policy](https://github.com/heisto-dev/heisto-contracts/blob/main/SECURITY.md)
- Contact: support@heisto.xyz
