# Heisto source build verification

This procedure is read-only. It does not deploy or upgrade a program, change
authority, connect a funded wallet, sign a transaction or spend SOL.

**Verification is not an independent security audit.**

## Identities and scope

- Mainnet program: `FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi`
- [Exact source revision](https://github.com/heisto-dev/heisto-contracts/tree/358de359889de10843745d4124114a7db54b54cd)
- [Exact release recipe: successful matching job](https://github.com/heisto-dev/heisto-contracts/actions/runs/43410617589/job/131835812374)
- [Official verifier CLI with pinned Cargo container: successful run](https://github.com/heisto-dev/heisto-contracts/actions/runs/43411854745)
- [Public registry status](https://verify.osec.io/status/FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi)

The build run, not this document, establishes whether compilation and comparison
succeeded. A workflow merely being present or started is not a verified build.
Read the final conclusion and comparison artifact before making a public claim.

## Current result: publicly verified on Mainnet

The clean build of the **exact public source commit** produces a **312,584-byte
executable identical byte-for-byte** with the binary currently deployed on
Mainnet. The independent GitHub runner fetched the original Agave release and
built using Cargo 1.88.0 with platform-tools v1.57.

The public OtterSec verification service independently rebuilt the registered
source and returned **`is_verified: true`** at **2026-11-13 11:43:10 UTC**.
Its executable hash and finalized Mainnet hash are identical:
`059c48c52301b0653af169ad2b06640a0ff1ef1283478d2c6cb448f6e7ea8d8e`.

- [Live public verification status](https://verify.osec.io/status/FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi)
- [Completed independent verifier job](https://verify.osec.io/job/db4c4fef-db04-40b6-accb-75e6be09527d)
- [Registered metadata transaction](https://solscan.io/tx/2CyXBqP9aXimrsjksCWrP37RF58Nh9t79VSp2xQ1931yPw8veLo23s8cusevF4QJBBj9PHS1VZxcbyL9Ue1q8w3f)
- [Immutable public build-image package](https://github.com/users/heisto-dev/packages/container/package/heisto-verifier-cargo188)

Explorer interfaces can cache or index verification status at different times.
The live registry above is the directly checked verification result.
The program remains upgradeable. **Verification is not an independent security
audit.** Embedded `security.txt` was not added; that is a separate binary change.

| Attempt | Build parameters | Outcome |
| --- | --- | --- |
| [Initial](https://github.com/heisto-dev/heisto-contracts/actions/runs/43407536670) | Pinned image, platform-tools v1.57, SBF v3 | Compiled; comparison failed |
| [Size optimization](https://github.com/heisto-dev/heisto-contracts/actions/runs/43408218333) | Same inputs plus recorded size optimization | Compiled; comparison failed |
| [Workspace root](https://github.com/heisto-dev/heisto-contracts/actions/runs/43408695606) | Size-optimized build from workspace root | Compiled; comparison failed with the same output as the preceding attempt |
| [Original Cargo recovered](https://github.com/heisto-dev/heisto-contracts/actions/runs/43410617589/job/131835812374) | Official Agave 4.3.2 builder, Cargo 1.88.0, SBF compiler v1.57 | **Exact raw SHA-256 match** |
| [Different Cargo control](https://github.com/heisto-dev/heisto-contracts/actions/runs/43410617589/job/131835812924) | Same builder and source, Cargo 1.90.0-nightly | Comparison fails as expected |
| [Pinned verifier container](https://github.com/heisto-dev/heisto-contracts/actions/runs/43411854745) | Official verifier CLI with pinned build image and Cargo 1.88.0 | **Exact raw SHA-256 match; entire job succeeded** |

Matching rebuilt and Mainnet executable:

- Bytes: **312,584**
- Raw SHA-256: `1b22fc202819dbf42e24f54be2b7c20725aeca3118b8c2aeb11ad38fff8ca0f8`
- Normalized SHA-256: `059c48c52301b0653af169ad2b06640a0ff1ef1283478d2c6cb448f6e7ea8d8e`

Finalized Mainnet snapshot:

- Allocated program payload: **312,584 bytes**
- Normalized SHA-256: `059c48c52301b0653af169ad2b06640a0ff1ef1283478d2c6cb448f6e7ea8d8e`
- The freshly rebuilt source executable matches the retained release binary
  byte-for-byte, which still matches the on-chain payload.
- Deployment slot and authority were unchanged in the read-only recheck.

[Machine-readable matching build comparison](evidence/matching-source-build.json) ·
[Verified Mainnet recheck](evidence/verified-mainnet-recheck.json) ·
[Public verifier response](evidence/public-verifier-status.json) ·
[Metadata receipt and exact cost](evidence/metadata-registration-receipt.json) ·
[Historical failed comparison](evidence/source-build-comparison.json)

The earlier mismatch was caused by build inputs, including Cargo version and
size optimization. Reproducing the old compiler alone was insufficient.

**No contract upgrade, authority change or prize funding was performed.**
One separately approved metadata transaction spent **0.00280916 SOL**:
0.00280416 SOL account rent and 0.000005 SOL network fee, below the approved
0.005 SOL cap. The deployed code and upgrade authority were checked unchanged.
The metadata receipt is not a prize payout or a contract deployment.

## Read-only reproduction

Use an isolated Linux host with Docker. Clone only the public contract repository.
Never mount a working directory containing environment files or wallet material
into a build container.

```sh
git clone https://github.com/heisto-dev/heisto-contracts
cd heisto-contracts
git checkout --detach 358de359889de10843745d4124114a7db54b54cd

# Fetch the verified, public Dockerfile separately. It pins the official
# Agave release, toolchain and Cargo version with release checksums.
curl --fail --location --proto '=https' --tlsv1.2 \
  https://raw.githubusercontent.com/heisto-dev/heisto-contracts/0f88a272647de5295aa2f0a164ea363daa009911/verification/Dockerfile \
  --output /tmp/heisto-verifier.Dockerfile
docker build -f /tmp/heisto-verifier.Dockerfile -t heisto-verify-cargo188:local .

curl --fail --location --proto '=https' --tlsv1.2 \
  https://github.com/solana-foundation/solana-verifiable-build/releases/download/v0.5.2/solana-verify-0.5.2-linux \
  --output /tmp/solana-verify
echo "b9d5ccce9634dc14269cab3e7a58649bc141b3306e7735a94478c936341f211f  /tmp/solana-verify" | sha256sum --check
chmod +x /tmp/solana-verify

/tmp/solana-verify build \
  --base-image heisto-verify-cargo188:local \
  --arch v3 \
  --cargo-build-sbf-args="--no-rustup-override --tools-version v1.57 --optimize-size"

/tmp/solana-verify get-executable-hash target/deploy/heisto.so
/tmp/solana-verify --url https://api.mainnet-beta.solana.com \
  get-program-hash FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi
```

Compare the two normalized hashes. Public RPC endpoints can rate-limit; an RPC
failure is not evidence of a match. Re-check the deployment slot and authority
when repeating the verification because the program is upgradeable.

The Dockerfile uses a digest-pinned official image and checksum-pinned Agave
4.3.2 archive. Cargo 1.88.0 and the platform-tools v1.57 compiler are
deliberately different versions; the old release used both. Keep SBF v3 and
`--optimize-size`. Do not substitute a floating image, a newer Cargo or a new
Cargo.lock. All failed comparisons remain publicly visible.

## Snapshot comparison

The public workflow also compares the freshly built executable against
`evidence/source-verification-recheck.json`. It uses SHA-256 after trimming
trailing zero padding, as the verifier does. Do not hash the 45-byte upgradeable
loader metadata header as program code.

A snapshot is historical evidence, not a claim that the program will remain
unchanged. Inspect a fresh finalized Mainnet slot before relying on it.

## Explorer registration is a separate operation

OtterSec's public verification registry reads repository, commit and build
arguments from an Otter Verify metadata PDA. Posting arbitrary repository
fields to an old API example does not replace that requirement.

Creating the metadata PDA requires a signed on-chain transaction and account
rent plus transaction fees. That is separate from the read-only build above.
Do not upload production signing keys to CI. Obtain explicit approval for an
exact metadata transaction and spending limit before publishing it.

No contract redeployment or authority transfer is required to match the source;
the source-to-binary match is already proven. The approved metadata transaction
has now been finalized, and the independent public registry job completed
successfully. No additional transaction is needed for this deployment.

The registered build image is pinned to:

```text
ghcr.io/heisto-dev/heisto-verifier-cargo188@sha256:23c3f9f70ae7ed175412fc817bccd44a985734f719f180ef721906663a2bd425
```

Anyone can pull it anonymously. Its public Dockerfile is pinned in the
reproduction instructions above. The remote registry used the same digest,
source commit, SBF v3 architecture and size-optimization arguments.

## References

- [Solana verified-build documentation](https://solana.com/docs/programs/verified-builds)
- [Verifier source and release](https://github.com/solana-foundation/solana-verifiable-build/releases/tag/v0.5.2)
- [Heisto security policy](https://github.com/heisto-dev/heisto-contracts/blob/main/SECURITY.md)
- Contact: support@heisto.xyz
