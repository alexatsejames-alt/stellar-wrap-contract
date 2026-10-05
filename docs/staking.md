# Staking

This document describes the staking module in `contracts/staking/src/lib.rs`
(and its tests). It states what staking is for, how it is configured, and what a
staker is exposed to.

## Purpose

Staking in this system is a **commitment / eligibility mechanism**, not a
transferable financial position.

- It does **not** gate minting. Minting is handled elsewhere and is not
  conditioned on a stake.
- It does **not** weight governance. Governance weights are not derived from
  staked balances here.
- It does **not** back storage fees. Storage fees are not paid out of stake.

Instead, a stake records that an account has locked a quantity of tokens for a
period of time. The stake is held by the staking contract and is released back
to the staker only after the unbonding delay has elapsed. Because the records in
this contract are explicitly **non-transferable (soulbound)**, a stake is bound
to the staking account and cannot be sold, assigned, or moved to another party.

## Configuration: `StakeConfig`

Staking behaviour is controlled by `StakeConfig`. The relevant parameters are:

- **Unbonding delay** — the minimum time a staker must wait between requesting
  an unbond and being able to withdraw the stake. This prevents instant
  enter/exit and gives the system a window in which a stake is still committed.
- **Amount bounds** — the minimum and maximum stake amounts accepted by the
  contract. Amounts outside these bounds are rejected.
- **Pause flag** — whether staking entry points are currently paused.

### Who can change it

The configuration is administrative. It is changed through the contract's
admin-gated configuration entry point; only the configured admin (the same
admin used for the rest of the contract's privileged operations) can update
`StakeConfig`. Ordinary stakers cannot change the unbonding delay, the amount
bounds, or the pause state.

See `docs/admin-rotation.md` for how the admin itself is rotated.

## Staker exposure and risks

A staker should be aware of the following:

- **Unbonding delay.** A stake cannot be withdrawn immediately. After an unbond
  is requested, the stake remains locked until the unbonding delay has elapsed.
  During that window the staker has no access to the funds.
- **Administrative changes.** The admin can change `StakeConfig`, including the
  unbonding delay and the amount bounds. A change to the delay can affect how
  long an in-flight unbond takes to complete.
- **Pause.** The admin can pause the staking entry points. While paused, new
  stakes and unbond requests are rejected. A pause does **not** destroy a stake,
  but it can strand a staker's position: the stake remains recorded and the
  staker cannot progress it (enter, unbond, or withdraw) until the contract is
  unpaused.
- **No slashing.** This module does not implement slashing. There is no code
  path that confiscates or reduces a staker's recorded stake as a penalty.
- **No governance lock beyond the above.** Stake is not subject to governance
  seizure; the only ways a stake is held are the unbonding delay and an
  administrative pause.

## Interaction with the soulbound design

Users often expect staking to produce a transferable position (a receipt or
share they can trade). That is **not** the case here. The records in this
contract are non-transferable, so:

- A stake is tied to the staking account and cannot be transferred to another
  account.
- There is no secondary market for a staked position.
- Exiting a stake means unbonding and withdrawing back to the same account, not
  selling the position.

Any integration that assumes a staked position is transferable is incorrect for
this system.
