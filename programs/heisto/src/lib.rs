//! Heisto — AI red-teaming arena on Solana.
//!
//! Defenders lock SOL in a Sentinel. Attackers pay a fee (escrowed in an
//! Attack account, together with a hash of their prompt) to try to break it.
//! The off-chain judge service runs the Sentinel model and the AI jury, then
//! signs a verdict with its Ed25519 key. `settle_attack` only moves funds after
//! verifying that signature through the native Ed25519 program.
//!
//! Safety rails for an unaudited mainnet launch:
//!   * `max_pool_lamports` caps the value any one Sentinel can hold;
//!   * `paused` stops new Sentinels, funding and attacks, but never blocks
//!     settling, refunds, fee claims or withdrawals;
//!   * an attack that is not settled within REFUND_DELAY can be refunded by
//!     the attacker, so a judge outage can never trap user funds.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked,
};
use anchor_lang::system_program;

declare_id!("FxqFvpptTRCeKCxCuxt86AJze6v16Vk9M8UZCQ8tJvbi");

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
pub const MIN_POOL_LAMPORTS: u64 = LAMPORTS_PER_SOL / 2; // 0.5 SOL
pub const LOCK_SECONDS: i64 = 7 * 24 * 60 * 60; // pool locked 7 days after last funding
pub const WITHDRAW_DELAY_SECONDS: i64 = 24 * 60 * 60; // 24h after deactivation
pub const REFUND_DELAY_SECONDS: i64 = 15 * 60; // unsettled attack refundable after 15 min
pub const WIN_SCORE: u16 = 800; // score is 0..=1000; on-chain floor for a breach
pub const MAX_FEE_MULTIPLIER: u64 = 3;
pub const VERDICT_DOMAIN: &[u8; 9] = b"HEISTO_V1";
pub const VERDICT_MESSAGE_LEN: usize = 9 + 32 + 32 + 8 + 1 + 2 + 32; // 116

/// Native Ed25519 signature-verify program id (Ed25519SigVerify111111111111111111111111111).
pub const ED25519_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    3, 125, 70, 214, 124, 147, 251, 190, 18, 249, 66, 143, 131, 141, 64, 255, 5, 112, 116, 73, 39,
    244, 138, 100, 252, 202, 112, 68, 128, 0, 0, 0,
]);

pub const STATUS_ACTIVE: u8 = 0;
pub const STATUS_BREACHED: u8 = 1;
pub const STATUS_DEACTIVATED: u8 = 2;

#[program]
pub mod heisto {
    use super::*;

    /// One-time setup restricted to the program's upgrade authority.
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        judge: Pubkey,
        treasury: Pubkey,
        burn_vault: Pubkey,
        max_pool_lamports: u64,
    ) -> Result<()> {
        let c = &mut ctx.accounts.config;
        c.admin = ctx.accounts.admin.key();
        c.judge = judge;
        c.treasury = treasury;
        c.burn_vault = burn_vault;
        c.max_pool_lamports = max_pool_lamports;
        // Deployment and initialization do not authorize accepting user funds.
        c.paused = true;
        c.pool_bps = 5000;
        c.defender_bps = 3500;
        c.treasury_bps = 1000;
        c.burn_bps = 500;
        c.bump = ctx.bumps.config;
        Ok(())
    }

    /// Admin: rotate the judge key, move treasury/burn wallets, change the cap, pause/unpause.
    pub fn update_config(
        ctx: Context<AdminOnly>,
        judge: Option<Pubkey>,
        treasury: Option<Pubkey>,
        burn_vault: Option<Pubkey>,
        max_pool_lamports: Option<u64>,
        paused: Option<bool>,
        new_admin: Option<Pubkey>,
    ) -> Result<()> {
        let c = &mut ctx.accounts.config;
        if let Some(v) = judge { c.judge = v; }
        if let Some(v) = treasury { c.treasury = v; }
        if let Some(v) = burn_vault { c.burn_vault = v; }
        if let Some(v) = max_pool_lamports { c.max_pool_lamports = v; }
        if let Some(v) = paused { c.paused = v; }
        if let Some(v) = new_admin { c.admin = v; }
        emit!(ConfigUpdated { judge: c.judge, paused: c.paused, max_pool_lamports: c.max_pool_lamports });
        Ok(())
    }

    /// Defender creates a Sentinel and locks the initial pool.
    pub fn create_sentinel(
        ctx: Context<CreateSentinel>,
        id: u64,
        base_fee: u64,
        pool_lamports: u64,
        instructions_hash: [u8; 32],
    ) -> Result<()> {
        let c = &ctx.accounts.config;
        require!(!c.paused, HeistoError::Paused);
        require!(pool_lamports >= MIN_POOL_LAMPORTS, HeistoError::PoolTooSmall);
        require!(pool_lamports <= c.max_pool_lamports, HeistoError::PoolCapExceeded);
        require!(base_fee > 0, HeistoError::FeeInvalid);
        require!(base_fee.checked_mul(100).ok_or(HeistoError::MathOverflow)? <= pool_lamports, HeistoError::FeeInvalid);

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.owner.to_account_info(),
                    to: ctx.accounts.sentinel.to_account_info(),
                },
            ),
            pool_lamports,
        )?;

        let now = Clock::get()?.unix_timestamp;
        let s = &mut ctx.accounts.sentinel;
        s.owner = ctx.accounts.owner.key();
        s.id = id;
        s.base_fee = base_fee;
        s.attack_count = 0;
        s.pool_lamports = pool_lamports;
        s.fees_lamports = 0;
        s.status = STATUS_ACTIVE;
        s.lock_until = now + LOCK_SECONDS;
        s.deactivated_at = 0;
        s.created_at = now;
        s.instructions_hash = instructions_hash;
        s.winner = Pubkey::default();
        s.bump = ctx.bumps.sentinel;
        emit!(SentinelCreated { sentinel: s.key(), owner: s.owner, pool_lamports, base_fee });
        Ok(())
    }

    /// Anyone can add to an active pool. Resets the 7-day lock.
    pub fn fund_sentinel(ctx: Context<FundSentinel>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.config.paused, HeistoError::Paused);
        require!(amount > 0, HeistoError::FeeInvalid);
        require!(ctx.accounts.sentinel.status == STATUS_ACTIVE, HeistoError::SentinelNotActive);
        let new_pool = ctx.accounts.sentinel.pool_lamports.checked_add(amount).ok_or(HeistoError::MathOverflow)?;
        require!(new_pool <= ctx.accounts.config.max_pool_lamports, HeistoError::PoolCapExceeded);

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.funder.to_account_info(),
                    to: ctx.accounts.sentinel.to_account_info(),
                },
            ),
            amount,
        )?;
        let s = &mut ctx.accounts.sentinel;
        s.pool_lamports = new_pool;
        s.lock_until = Clock::get()?.unix_timestamp + LOCK_SECONDS;
        emit!(SentinelFunded { sentinel: s.key(), amount, pool_lamports: new_pool });
        Ok(())
    }

    /// Attacker escrows the current fee and commits to sha256(prompt).
    /// `max_fee` protects against the fee rising between quote and signature.
    pub fn request_attack(ctx: Context<RequestAttack>, prompt_hash: [u8; 32], max_fee: u64) -> Result<()> {
        require!(!ctx.accounts.config.paused, HeistoError::Paused);
        let s = &ctx.accounts.sentinel;
        require!(s.status == STATUS_ACTIVE, HeistoError::SentinelNotActive);
        let fee = current_fee(s.base_fee, s.attack_count)?;
        require!(fee <= max_fee, HeistoError::FeeAboveMax);

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.attacker.to_account_info(),
                    to: ctx.accounts.attack.to_account_info(),
                },
            ),
            fee,
        )?;

        let seq = s.attack_count;
        let a = &mut ctx.accounts.attack;
        a.sentinel = ctx.accounts.sentinel.key();
        a.attacker = ctx.accounts.attacker.key();
        a.seq = seq;
        a.fee = fee;
        a.prompt_hash = prompt_hash;
        a.created_at = Clock::get()?.unix_timestamp;
        a.bump = ctx.bumps.attack;

        let s = &mut ctx.accounts.sentinel;
        s.attack_count = s.attack_count.checked_add(1).ok_or(HeistoError::MathOverflow)?;
        emit!(AttackRequested { sentinel: s.key(), attack: a.key(), attacker: a.attacker, seq, fee });
        Ok(())
    }

    /// Settle an attack with a verdict signed by the judge key. The transaction
    /// must contain, immediately before this instruction, an Ed25519 verify
    /// instruction over the exact verdict message. Anyone may submit it.
    pub fn settle_attack(ctx: Context<SettleAttack>, success: bool, score: u16, response_hash: [u8; 32]) -> Result<()> {
        require!(score <= 1000, HeistoError::ScoreInvalid);
        let expected = verdict_message(
            &ctx.accounts.sentinel.key(),
            &ctx.accounts.attack.key(),
            ctx.accounts.attack.seq,
            success,
            score,
            &response_hash,
        );
        verify_judge_signature(&ctx.accounts.instructions.to_account_info(), &ctx.accounts.config.judge, &expected)?;

        let fee = ctx.accounts.attack.fee;
        let attack_ai = ctx.accounts.attack.to_account_info();
        let sentinel_ai = ctx.accounts.sentinel.to_account_info();
        let attacker_ai = ctx.accounts.attacker.to_account_info();

        if ctx.accounts.sentinel.status != STATUS_ACTIVE {
            // Sentinel already breached or deactivated: the fee stays in the attack
            // account and returns to the attacker when the account closes below.
            emit!(AttackRefunded { attack: ctx.accounts.attack.key(), attacker: attacker_ai.key(), fee });
            return Ok(());
        }

        let c = &ctx.accounts.config;
        let pool_part = mul_bps(fee, c.pool_bps)?;
        let defender_part = mul_bps(fee, c.defender_bps)?;
        let treasury_part = mul_bps(fee, c.treasury_bps)?;
        let burn_part = fee
            .checked_sub(pool_part + defender_part + treasury_part)
            .ok_or(HeistoError::MathOverflow)?;

        move_lamports(&attack_ai, &sentinel_ai, pool_part + defender_part)?;
        move_lamports(&attack_ai, &ctx.accounts.treasury.to_account_info(), treasury_part)?;
        move_lamports(&attack_ai, &ctx.accounts.burn_vault.to_account_info(), burn_part)?;

        let s = &mut ctx.accounts.sentinel;
        s.pool_lamports = s.pool_lamports.checked_add(pool_part).ok_or(HeistoError::MathOverflow)?;
        s.fees_lamports = s.fees_lamports.checked_add(defender_part).ok_or(HeistoError::MathOverflow)?;

        let breach = success && score >= WIN_SCORE;
        if breach {
            let payout = s.pool_lamports;
            s.pool_lamports = 0;
            s.status = STATUS_BREACHED;
            s.winner = attacker_ai.key();
            move_lamports(&sentinel_ai, &attacker_ai, payout)?;
            emit!(SentinelBreached { sentinel: s.key(), winner: s.winner, payout, score });
        }
        emit!(AttackSettled { sentinel: s.key(), attack: ctx.accounts.attack.key(), success: breach, score, fee, response_hash });
        Ok(())
    }

    /// Attacker reclaims an attack that the judge never settled.
    pub fn refund_attack(ctx: Context<RefundAttack>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(now >= ctx.accounts.attack.created_at + REFUND_DELAY_SECONDS, HeistoError::RefundTooEarly);
        emit!(AttackRefunded { attack: ctx.accounts.attack.key(), attacker: ctx.accounts.attacker.key(), fee: ctx.accounts.attack.fee });
        Ok(())
    }

    /// Defender claims accrued fees (35% of every failed attack) at any time.
    pub fn claim_fees(ctx: Context<OwnerSentinel>) -> Result<()> {
        let amount = ctx.accounts.sentinel.fees_lamports;
        require!(amount > 0, HeistoError::NothingToClaim);
        ctx.accounts.sentinel.fees_lamports = 0;
        move_lamports(&ctx.accounts.sentinel.to_account_info(), &ctx.accounts.owner.to_account_info(), amount)?;
        emit!(FeesClaimed { sentinel: ctx.accounts.sentinel.key(), amount });
        Ok(())
    }

    /// Defender stops accepting attacks. Withdrawal opens 24h later (and after the lock).
    pub fn deactivate_sentinel(ctx: Context<OwnerSentinel>) -> Result<()> {
        let s = &mut ctx.accounts.sentinel;
        require!(s.status == STATUS_ACTIVE, HeistoError::SentinelNotActive);
        s.status = STATUS_DEACTIVATED;
        s.deactivated_at = Clock::get()?.unix_timestamp;
        emit!(SentinelDeactivated { sentinel: s.key() });
        Ok(())
    }

    /// Defender withdraws the remaining pool and fees after deactivation + delays.
    pub fn withdraw(ctx: Context<OwnerSentinel>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let s = &mut ctx.accounts.sentinel;
        require!(s.status == STATUS_DEACTIVATED, HeistoError::NotDeactivated);
        require!(now >= s.lock_until, HeistoError::StillLocked);
        require!(now >= s.deactivated_at + WITHDRAW_DELAY_SECONDS, HeistoError::StillLocked);
        let amount = s.pool_lamports.checked_add(s.fees_lamports).ok_or(HeistoError::MathOverflow)?;
        require!(amount > 0, HeistoError::NothingToClaim);
        s.pool_lamports = 0;
        s.fees_lamports = 0;
        move_lamports(&ctx.accounts.sentinel.to_account_info(), &ctx.accounts.owner.to_account_info(), amount)?;
        emit!(SentinelWithdrawn { sentinel: ctx.accounts.sentinel.key(), amount });
        Ok(())
    }
}

/* ------------------------------------------------------------------------ */
/* Pure helpers (unit-tested below)                                         */
/* ------------------------------------------------------------------------ */

/// fee = base * (100 + attacks) / 100, capped at 3 × base.
pub fn current_fee(base_fee: u64, attack_count: u64) -> Result<u64> {
    let scaled = (base_fee as u128)
        .checked_mul(100u128 + attack_count as u128)
        .ok_or(HeistoError::MathOverflow)?
        / 100;
    let cap = (base_fee as u128) * MAX_FEE_MULTIPLIER as u128;
    Ok(scaled.min(cap) as u64)
}

pub fn mul_bps(amount: u64, bps: u16) -> Result<u64> {
    Ok(((amount as u128) * (bps as u128) / 10_000) as u64)
}

/// The exact bytes the judge signs:
/// "HEISTO_V1" | sentinel(32) | attack(32) | seq(u64 LE) | success(u8) | score(u16 LE) | response_hash(32)
pub fn verdict_message(sentinel: &Pubkey, attack: &Pubkey, seq: u64, success: bool, score: u16, response_hash: &[u8; 32]) -> [u8; VERDICT_MESSAGE_LEN] {
    let mut m = [0u8; VERDICT_MESSAGE_LEN];
    let mut o = 0;
    m[o..o + 9].copy_from_slice(VERDICT_DOMAIN); o += 9;
    m[o..o + 32].copy_from_slice(sentinel.as_ref()); o += 32;
    m[o..o + 32].copy_from_slice(attack.as_ref()); o += 32;
    m[o..o + 8].copy_from_slice(&seq.to_le_bytes()); o += 8;
    m[o] = success as u8; o += 1;
    m[o..o + 2].copy_from_slice(&score.to_le_bytes()); o += 2;
    m[o..o + 32].copy_from_slice(response_hash);
    m
}

/// Parse a native Ed25519 verify instruction that carries exactly one signature
/// whose data all live inside the instruction itself (indices = u16::MAX), and
/// return (pubkey, message).
pub fn parse_ed25519_ix(data: &[u8]) -> Option<([u8; 32], Vec<u8>)> {
    if data.len() < 16 || data[0] != 1 { return None; }
    let rd = |i: usize| u16::from_le_bytes([data[i], data[i + 1]]);
    let sig_off = rd(2) as usize;
    let sig_ix = rd(4);
    let pk_off = rd(6) as usize;
    let pk_ix = rd(8);
    let msg_off = rd(10) as usize;
    let msg_len = rd(12) as usize;
    let msg_ix = rd(14);
    if sig_ix != u16::MAX || pk_ix != u16::MAX || msg_ix != u16::MAX { return None; }
    if sig_off + 64 > data.len() || pk_off + 32 > data.len() || msg_off + msg_len > data.len() { return None; }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&data[pk_off..pk_off + 32]);
    Some((pk, data[msg_off..msg_off + msg_len].to_vec()))
}

fn verify_judge_signature(ix_sysvar: &AccountInfo, judge: &Pubkey, expected: &[u8]) -> Result<()> {
    let current = load_current_index_checked(ix_sysvar)? as usize;
    require!(current > 0, HeistoError::MissingJudgeSignature);
    let ix = load_instruction_at_checked(current - 1, ix_sysvar)?;
    require_keys_eq!(ix.program_id, ED25519_PROGRAM_ID, HeistoError::MissingJudgeSignature);
    require!(ix.accounts.is_empty(), HeistoError::MissingJudgeSignature);
    let (pk, msg) = parse_ed25519_ix(&ix.data).ok_or(HeistoError::MissingJudgeSignature)?;
    require!(pk == judge.to_bytes(), HeistoError::WrongJudge);
    require!(msg.as_slice() == expected, HeistoError::VerdictMismatch);
    Ok(())
}

/// Move lamports out of a program-owned account (must stay rent-exempt via the
/// accounting invariant lamports = rent + pool + fees).
fn move_lamports(from: &AccountInfo, to: &AccountInfo, amount: u64) -> Result<()> {
    if amount == 0 { return Ok(()); }
    **from.try_borrow_mut_lamports()? = from.lamports().checked_sub(amount).ok_or(HeistoError::MathOverflow)?;
    **to.try_borrow_mut_lamports()? = to.lamports().checked_add(amount).ok_or(HeistoError::MathOverflow)?;
    Ok(())
}

/* ------------------------------------------------------------------------ */
/* Accounts                                                                 */
/* ------------------------------------------------------------------------ */

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub judge: Pubkey,
    pub treasury: Pubkey,
    pub burn_vault: Pubkey,
    pub max_pool_lamports: u64,
    pub paused: bool,
    pub pool_bps: u16,
    pub defender_bps: u16,
    pub treasury_bps: u16,
    pub burn_bps: u16,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Sentinel {
    pub owner: Pubkey,
    pub id: u64,
    pub base_fee: u64,
    pub attack_count: u64,
    pub pool_lamports: u64,
    pub fees_lamports: u64,
    pub status: u8,
    pub lock_until: i64,
    pub deactivated_at: i64,
    pub created_at: i64,
    pub instructions_hash: [u8; 32],
    pub winner: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Attack {
    pub sentinel: Pubkey,
    pub attacker: Pubkey,
    pub seq: u64,
    pub fee: u64,
    pub prompt_hash: [u8; 32],
    pub created_at: i64,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(init, payer = admin, space = 8 + Config::INIT_SPACE, seeds = [b"config"], bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, crate::program::Heisto>,
    #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()))]
    pub program_data: Account<'info, ProgramData>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(mut, seeds = [b"config"], bump = config.bump, has_one = admin)]
    pub config: Account<'info, Config>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct CreateSentinel<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(init, payer = owner, space = 8 + Sentinel::INIT_SPACE,
        seeds = [b"sentinel", owner.key().as_ref(), &id.to_le_bytes()], bump)]
    pub sentinel: Account<'info, Sentinel>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FundSentinel<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"sentinel", sentinel.owner.as_ref(), &sentinel.id.to_le_bytes()], bump = sentinel.bump)]
    pub sentinel: Account<'info, Sentinel>,
    #[account(mut)]
    pub funder: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestAttack<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"sentinel", sentinel.owner.as_ref(), &sentinel.id.to_le_bytes()], bump = sentinel.bump)]
    pub sentinel: Account<'info, Sentinel>,
    #[account(init, payer = attacker, space = 8 + Attack::INIT_SPACE,
        seeds = [b"attack", sentinel.key().as_ref(), &sentinel.attack_count.to_le_bytes()], bump)]
    pub attack: Account<'info, Attack>,
    #[account(mut)]
    pub attacker: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SettleAttack<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"sentinel", sentinel.owner.as_ref(), &sentinel.id.to_le_bytes()], bump = sentinel.bump)]
    pub sentinel: Account<'info, Sentinel>,
    #[account(mut, close = attacker, has_one = sentinel, has_one = attacker,
        seeds = [b"attack", sentinel.key().as_ref(), &attack.seq.to_le_bytes()], bump = attack.bump)]
    pub attack: Account<'info, Attack>,
    /// CHECK: verified by `has_one = attacker` on the attack account.
    #[account(mut)]
    pub attacker: UncheckedAccount<'info>,
    /// CHECK: must equal config.treasury.
    #[account(mut, address = config.treasury)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: must equal config.burn_vault.
    #[account(mut, address = config.burn_vault)]
    pub burn_vault: UncheckedAccount<'info>,
    /// CHECK: the instructions sysvar, address-checked.
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
    pub relayer: Signer<'info>,
}

#[derive(Accounts)]
pub struct RefundAttack<'info> {
    #[account(mut, close = attacker, has_one = attacker,
        seeds = [b"attack", attack.sentinel.as_ref(), &attack.seq.to_le_bytes()], bump = attack.bump)]
    pub attack: Account<'info, Attack>,
    #[account(mut)]
    pub attacker: Signer<'info>,
}

#[derive(Accounts)]
pub struct OwnerSentinel<'info> {
    #[account(mut, has_one = owner, seeds = [b"sentinel", owner.key().as_ref(), &sentinel.id.to_le_bytes()], bump = sentinel.bump)]
    pub sentinel: Account<'info, Sentinel>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

/* ------------------------------------------------------------------------ */
/* Events & errors                                                          */
/* ------------------------------------------------------------------------ */

#[event] pub struct ConfigUpdated { pub judge: Pubkey, pub paused: bool, pub max_pool_lamports: u64 }
#[event] pub struct SentinelCreated { pub sentinel: Pubkey, pub owner: Pubkey, pub pool_lamports: u64, pub base_fee: u64 }
#[event] pub struct SentinelFunded { pub sentinel: Pubkey, pub amount: u64, pub pool_lamports: u64 }
#[event] pub struct AttackRequested { pub sentinel: Pubkey, pub attack: Pubkey, pub attacker: Pubkey, pub seq: u64, pub fee: u64 }
#[event] pub struct AttackSettled { pub sentinel: Pubkey, pub attack: Pubkey, pub success: bool, pub score: u16, pub fee: u64, pub response_hash: [u8; 32] }
#[event] pub struct AttackRefunded { pub attack: Pubkey, pub attacker: Pubkey, pub fee: u64 }
#[event] pub struct SentinelBreached { pub sentinel: Pubkey, pub winner: Pubkey, pub payout: u64, pub score: u16 }
#[event] pub struct FeesClaimed { pub sentinel: Pubkey, pub amount: u64 }
#[event] pub struct SentinelDeactivated { pub sentinel: Pubkey }
#[event] pub struct SentinelWithdrawn { pub sentinel: Pubkey, pub amount: u64 }

#[error_code]
pub enum HeistoError {
    #[msg("Heisto is paused")] Paused,
    #[msg("Pool is below the 0.5 SOL minimum")] PoolTooSmall,
    #[msg("Pool exceeds the current cap")] PoolCapExceeded,
    #[msg("Fee must be above zero and at most 1% of the pool")] FeeInvalid,
    #[msg("Current fee is above your max_fee")] FeeAboveMax,
    #[msg("Sentinel is not active")] SentinelNotActive,
    #[msg("Score must be 0..=1000")] ScoreInvalid,
    #[msg("Missing or malformed judge signature instruction")] MissingJudgeSignature,
    #[msg("Verdict was not signed by the judge key")] WrongJudge,
    #[msg("Signed verdict does not match this settlement")] VerdictMismatch,
    #[msg("Refund is available 15 minutes after the attack")] RefundTooEarly,
    #[msg("Nothing to claim")] NothingToClaim,
    #[msg("Sentinel must be deactivated first")] NotDeactivated,
    #[msg("Funds are still locked")] StillLocked,
    #[msg("Math overflow")] MathOverflow,
}

/* ------------------------------------------------------------------------ */
/* Tests                                                                    */
/* ------------------------------------------------------------------------ */

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn ed25519_program_id_matches_native() {
        assert_eq!(ED25519_PROGRAM_ID, Pubkey::from_str("Ed25519SigVerify111111111111111111111111111").unwrap());
    }

    #[test]
    fn fee_curve_rises_one_percent_and_caps_at_3x() {
        assert_eq!(current_fee(1_000_000, 0).unwrap(), 1_000_000);
        assert_eq!(current_fee(1_000_000, 10).unwrap(), 1_100_000);
        assert_eq!(current_fee(1_000_000, 50).unwrap(), 1_500_000);
        assert_eq!(current_fee(1_000_000, 200).unwrap(), 3_000_000);
        assert_eq!(current_fee(1_000_000, 10_000).unwrap(), 3_000_000);
    }

    #[test]
    fn split_sums_to_fee() {
        for fee in [1u64, 7, 999, 1_000_000, 123_456_789] {
            let p = mul_bps(fee, 5000).unwrap();
            let d = mul_bps(fee, 3500).unwrap();
            let t = mul_bps(fee, 1000).unwrap();
            let b = fee - p - d - t;
            assert_eq!(p + d + t + b, fee);
            assert!(b <= fee / 20 + 3);
        }
    }

    #[test]
    fn verdict_message_layout() {
        let s = Pubkey::new_from_array([1; 32]);
        let a = Pubkey::new_from_array([2; 32]);
        let m = verdict_message(&s, &a, 0x0102, true, 850, &[9; 32]);
        assert_eq!(&m[0..9], b"HEISTO_V1");
        assert_eq!(&m[9..41], &[1; 32]);
        assert_eq!(&m[41..73], &[2; 32]);
        assert_eq!(&m[73..81], &0x0102u64.to_le_bytes());
        assert_eq!(m[81], 1);
        assert_eq!(&m[82..84], &850u16.to_le_bytes());
        assert_eq!(&m[84..116], &[9; 32]);
    }

    /// Mirrors @solana/web3.js Ed25519Program.createInstructionWithPublicKey layout.
    fn build_ed25519_data(pk: &[u8; 32], sig: &[u8; 64], msg: &[u8]) -> Vec<u8> {
        let header = 16u16;
        let pk_off = header;
        let sig_off = pk_off + 32;
        let msg_off = sig_off + 64;
        let mut d = vec![1u8, 0u8];
        for v in [sig_off, u16::MAX, pk_off, u16::MAX, msg_off, msg.len() as u16, u16::MAX] { d.extend_from_slice(&v.to_le_bytes()); }
        d.extend_from_slice(pk);
        d.extend_from_slice(sig);
        d.extend_from_slice(msg);
        d
    }

    #[test]
    fn parses_web3js_ed25519_layout() {
        let pk = [7u8; 32];
        let msg = verdict_message(&Pubkey::new_from_array([1; 32]), &Pubkey::new_from_array([2; 32]), 3, false, 120, &[5; 32]);
        let data = build_ed25519_data(&pk, &[0u8; 64], &msg);
        let (got_pk, got_msg) = parse_ed25519_ix(&data).unwrap();
        assert_eq!(got_pk, pk);
        assert_eq!(got_msg, msg.to_vec());
    }

    /// Fixture written by server/test/sdk.test.ts from a real @solana/web3.js Ed25519 instruction.
    #[test]
    fn parses_fixture_from_web3js() {
        let f = include_str!("../tests/fixtures/ed25519_ix.hex");
        let mut lines = f.lines();
        let hex = |s: &str| (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let data = hex(lines.next().unwrap());
        let pk = hex(lines.next().unwrap());
        let msg = hex(lines.next().unwrap());
        let (got_pk, got_msg) = parse_ed25519_ix(&data).unwrap();
        assert_eq!(got_pk.to_vec(), pk);
        assert_eq!(got_msg, msg);
    }

    #[test]
    fn rejects_cross_instruction_offsets() {
        let pk = [7u8; 32];
        let mut data = build_ed25519_data(&pk, &[0u8; 64], b"hello");
        data[8] = 0; data[9] = 0; // public_key_instruction_index = 0 (points elsewhere)
        assert!(parse_ed25519_ix(&data).is_none());
    }
}
