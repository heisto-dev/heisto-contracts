# Heisto verified program and embedded security contact

Program: [`FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi`](https://solscan.io/account/FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi)

This release embeds a public security-reporting contact in the actual Mainnet
program. It does not add new instructions or change existing instruction
handlers, account layouts, fees, prize rules or upgrade authority.

## Public evidence

- [Live independent verifier status](https://verify.osec.io/status/FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi)
- [Independent rebuild job](https://verify.osec.io/job/33835d6f-c893-4698-83c7-bf7dbc186ff0)
- [Exact verified source commit](https://github.com/heisto-dev/heisto-contracts/tree/b562a214241e16ef10ba383b4d6d8779e7578f3e)
- [Public build and nine passing native tests](https://github.com/heisto-dev/heisto-contracts/actions/runs/43420939055)
- [Approved Mainnet upgrade](https://solscan.io/tx/7qu2kLKpsTzSniQZQzDPjjUjP94FLe258CvKugWbqtzHBmLv2rPqvn9H8U8iBAMEEQUxo3BxBDLWx2juKF1J5MF)
- [Updated verifier metadata](https://solscan.io/tx/2JftZogMMxLfYbXX17ohmZSwEv8Fnv3LuA12PQ8myarzAiaKx8bXsC4CnQKL8m2FYpLE58mR4QjFEdHTbwr7Uv9E)
- [Deposit and unused-budget refund](https://solscan.io/tx/1eEJ22tSS6FfcARDfmRdBBmc8YS7KYtDbUfwcSXGJqcqvEXnbnTdLj1jtfmaQPyBG3gzoAY74WeUbNerc7jScxb)

Machine-readable observations are in
[`evidence/security-contact-upgrade-receipt.json`](evidence/security-contact-upgrade-receipt.json)
and [`evidence/security-contact-verifier-status.json`](evidence/security-contact-verifier-status.json).

## Exact release identity

- Deployment slot: **468930601**
- ELF size: **312,912 bytes**
- ELF raw SHA-256: `8069f3085b6c796a956fdb2eb287b977e6d628161da49934dc05fa46b7f72940`
- Normalized on-chain and rebuilt SHA-256:
  `95162cd3dbdf5a2061c7f5da84bfa3dc8b68003f6f3621d8cf3ea503386a4907`
- Build image:
  `ghcr.io/heisto-dev/heisto-verifier-cargo188@sha256:23c3f9f70ae7ed175412fc817bccd44a985734f719f180ef721906663a2bd425`
- Cargo 1.88.0, platform-tools v1.57, SBF v3, explicit size optimization.

The loader account includes its metadata header and zero-filled capacity.
Compare the executable hash after excluding the header and trimming trailing
zero padding, not a raw hash of the complete ProgramData account.

## Embedded contact fields

| Field | Value |
| --- | --- |
| name | Heisto |
| project_url | https://heisto.xyz |
| contacts | email:support@heisto.xyz |
| policy | https://github.com/heisto-dev/heisto-contracts/blob/main/SECURITY.md |
| preferred_languages | en |
| source_code | https://github.com/heisto-dev/heisto-contracts |

The block uses the Solana security.txt V1 null-delimited format. Its presence
in the built executable was checked before deployment. This is separate from a
website's `/.well-known/security.txt` endpoint.

## Upgrade scope and actual cost

Nine native tests passed. The build pipeline checked that all instruction
handler source, Cargo manifests and Cargo.lock were unchanged; only the
metadata module and its import were added. No new dependency was introduced.

The owner approved a **0.06 SOL net cap**. The observed net wallet debit was
**0.054200197 SOL**. The upload buffer was closed on successful upgrade, and
**1.596215962 SOL** was returned to the authority wallet, including the
temporary deposit and unused budget. No prize accounts were included.

The ELF grew by 328 bytes, but the active Mainnet loader feature required a
10 KiB ProgramData extension. Added on-chain storage rent and transaction fees
are distinct from the refunded temporary upload deposit.

## Read-only reproduction

Use an isolated checkout with no credentials or wallet files.

```sh
git clone https://github.com/heisto-dev/heisto-contracts
cd heisto-contracts
git checkout --detach b562a214241e16ef10ba383b4d6d8779e7578f3e
curl --fail --location --proto '=https' --tlsv1.2 \
  https://github.com/solana-foundation/solana-verifiable-build/releases/download/v0.5.2/solana-verify-0.5.2-linux \
  --output /tmp/solana-verify
echo "b9d5ccce9634dc14269cab3e7a58649bc141b3306e7735a94478c936341f211f  /tmp/solana-verify" | sha256sum --check
chmod +x /tmp/solana-verify
/tmp/solana-verify build \
  --base-image ghcr.io/heisto-dev/heisto-verifier-cargo188@sha256:23c3f9f70ae7ed175412fc817bccd44a985734f719f180ef721906663a2bd425 \
  --arch v3 \
  --cargo-build-sbf-args="--no-rustup-override --tools-version v1.57 --optimize-size"
/tmp/solana-verify get-executable-hash target/deploy/heisto.so
/tmp/solana-verify --url https://api.mainnet-beta.solana.com \
  get-program-hash FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi
```

The two normalized hashes must match. RPC failure is not evidence of a match.
Re-check the deployment slot after any future upgrade.

## Limits

**Embedded security.txt is contact metadata, not an audit or a safety rating.**
Verified source establishes bytecode identity, not absence of vulnerabilities.
The program is still upgradeable and has not been independently audited.

Earlier failed comparisons and the first successful release remain recorded in
[the initial build history](REPRODUCIBLE_BUILD.md); they are not the current hash.
