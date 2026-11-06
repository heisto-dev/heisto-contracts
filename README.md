# Heisto Contracts

Anchor 0.31.1 / Rust source for the Heisto Solana program.

**Mainnet program:** `FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi`

[Explorer](https://explorer.solana.com/address/FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi) · [Evidence](https://github.com/heisto-dev/heisto-mainnet-proof) · Support: support@heisto.xyz

## State and instructions
- Config: administrator, judge signer, treasury, owner-held reserve, pool cap, pause and fee split.
- Sentinel: owner + u64 ID, prize pool, base fee, attack count, accrued owner fees, status, lock and instructions hash.
- Attack: Sentinel, attacker, sequence, fee escrow, prompt hash and creation time.
- Instructions: initialize_config, update_config, create_sentinel, fund_sentinel, request_attack, settle_attack, refund_attack, claim_fees, deactivate_sentinel, withdraw.

## Economic rules
Minimum pool 0.5 SOL; configured Mainnet cap currently 2 SOL. Seven-day pool lock since latest funding; withdrawal also needs 24 hours after deactivation. Fee growth is linear, capped at 3x. Eligible unsettled attacks become refundable after 15 minutes. A breach needs a valid bound judge signature plus success and score >= 800/1000.

Settled fees allocate 50% pool, 35% defender, 10% treasury and remaining 5% owner-held reserve. Despite the historical field name burn_vault, reserve transfers do not burn tokens. Allocation precedes a breach payout; the attack's pool share is included in the paid pool.

## Build/test
```sh
cargo test --locked -p heisto
# With a compatible Solana SBF toolchain and Anchor 0.31.1 installed:
anchor build
```
The existing declared Mainnet ID is public; no matching program keypair is supplied. Anchor.toml defaults to localnet to avoid accidental Mainnet use. A newly generated program identity must be configured consistently for your own local/devnet deployment. Do not treat this build command as permission or sufficient instructions to replace the official program.

Native tests include fee arithmetic, verdict serialization and Ed25519 instruction parsing with a public test-only fixture. A locally compiled binary is not automatically a reproducible match to the deployed binary; see the proof repository for its exact hash and verification scope.

## Security boundary
The contract is upgradeable and settlement trusts a configured off-chain judge. Ed25519 message bindings cover the Sentinel, Attack, sequence, outcome, score and response hash. Administrative pausing does not make historical transactions disappear. Inspect current configuration/authority rather than relying on a snapshot forever.

Detailed account layouts, transaction flows, threats and trust assumptions live in [heisto-docs](https://github.com/heisto-dev/heisto-docs). MIT. Security reports: support@heisto.xyz.
