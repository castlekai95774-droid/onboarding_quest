# onboarding_quest

## Project Title
onboarding_quest

## Project Description
Welcoming new members into a Web3 community is usually a mess of DMs, screenshots, and trust-based checklists that nobody can audit. `onboarding_quest` turns that flow into a transparent, on-chain quest: an admin defines a sequence of steps (introduce yourself, join the Discord, attend a town hall, mint a profile NFT, ...), every new member submits a proof hash for each step, and the admin verifies the proofs. Once all required steps are verified, the member is automatically promoted to the "Onboarded" status and can claim a one-time reward recorded on Stellar — with no spreadsheets and no "did they actually do the work?" arguments.

## Project Vision
Our long-term vision is to make community onboarding as trustworthy, portable, and reusable as a smart contract. By encoding the quest on Stellar via Soroban, every community — DAOs, university clubs, hackathon crews, open-source maintainers — can run the same auditable onboarding flow without running their own backend, and members get a portable, on-chain record of the work they've completed. We want `onboarding_quest` to become the default "first contract" a community deploys, the way a guestbook or a faucet is today.

## Key Features
- **Admin-managed quest steps** — `add_step` lets a single community admin register an arbitrary number of steps, each tagged as `required` or `optional`, with the human-readable description kept off-chain and pinned by a SHA-256 hash.
- **Proof-of-completion submissions** — `complete_step` lets any member attach a `proof_hash` (screenshot, signed message, off-chain artifact hash) to the steps they have done. The contract stores who, when, and what — no central server needed.
- **Admin verification with auto-promotion** — `verify_step` confirms a member's proof, and the contract automatically flips the member to the `Onboarded` status the moment the last required step is verified.
- **One-time Onboarded reward claim** — `claim_reward` lets a fully onboarded member record their reward on chain; a `reward` event is emitted so off-chain indexers, dashboards, and bots can react in real time.
- **Transparent progress views** — `is_onboarded`, `progress`, `get_step`, `get_completion`, and `step_count` expose the full state of the quest, so members and observers can verify everything without privileged access.
- **Auth-safe by design** — every state-changing function uses Soroban `require_auth()`, admin-only operations are guarded by an on-chain `Admin` key, and no real XLM transfer is ever performed (the reward is recorded, not paid out, by the contract itself).

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** community dApp — see `contracts/onboarding_quest/src/lib.rs` for the full onboarding_quest business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CAAQZOMIZYJ2D4VXAXFCL2AITDJ4YMTH3NVCI564YDRJ44FE64DJR6WN`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/be02c8f5baed1e9464d1a35191dda90cc02770b769534513eb963e57a0097178`

## Future Scope
- **On-chain reward payouts** — wrap `claim_reward` in a token transfer (native XLM or a community-specific SAC) so the reward is actually delivered, not just recorded.
- **Multi-admin / role-based verification** — add a `Verifier` role so verification can be split across moderators instead of a single `Admin` key.
- **Step deadlines & streaks** — let admins attach `opens_at` / `closes_at` timestamps to steps, and reward members who complete the full quest within a window.
- **Off-chain proof verification hooks** — support `proof_hash` formats that resolve to verifiable attestations (e.g. signed GitHub/Discord/JumpClub messages) verified by the contract itself.
- **Frontend dashboard** — a small React/Freighter UI that lists the current quest, shows a member's `progress`, and surfaces `reward` events for community analytics.
- **Reusable quest templates** — export/import quest definitions so communities can fork and tweak onboarding flows without redeploying from scratch.
- **Soulbound Onboarded NFT** — issue a non-transferable credential token (a SAC with `auth_revocable` / clawback-friendly design) as a portable, verifiable proof of completion.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `onboarding_quest` (community)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
