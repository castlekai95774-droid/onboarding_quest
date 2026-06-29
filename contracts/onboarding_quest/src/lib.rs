#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol};

/// Storage keys used by the `OnboardingQuest` contract.
///
/// All state lives in `instance` storage because the quest is a single,
/// community-wide configuration that every participant reads and writes to.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Address of the community admin who manages quest steps and verifies
    /// member submissions.
    Admin,
    /// Total number of quest steps currently registered.
    StepCount,
    /// Definition of the step with the given numeric id.
    Step(u32),
    /// Completion record for a `(member, step_id)` pair, storing the proof
    /// hash submitted by the member and the verification metadata.
    Completion(Address, u32),
    /// Set to `true` once a member has verified every required step.
    Onboarded(Address),
    /// Set to `true` once a member has claimed the Onboarded reward.
    Rewarded(Address),
}

/// A single quest step that members must complete during onboarding.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    /// SHA-256 hash of the step description (kept off-chain).
    pub description_hash: BytesN<32>,
    /// Whether this step is mandatory for the Onboarded status.
    pub required: bool,
}

/// Per-member, per-step completion record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Completion {
    /// Hash of whatever proof the member submits for the step (e.g. a signed
    /// attestation, screenshot, or task artifact hash).
    pub proof_hash: BytesN<32>,
    /// `true` once the admin has verified the proof.
    pub verified: bool,
    /// Ledger timestamp at which `complete_step` was called.
    pub completed_at: u64,
    /// Ledger timestamp at which `verify_step` confirmed the proof.
    pub verified_at: u64,
}

/// Symbol emitted when a member successfully claims the Onboarded reward.
/// Off-chain indexers can listen for this event to track community growth.
const REWARD_EVENT: Symbol = symbol_short!("reward");

/// `onboarding_quest` is a community onboarding quest: a series of steps a
/// new member completes in order to join the community. Each step is
/// defined by the admin, members submit proof of completion, the admin
/// verifies the proof, and once all *required* steps are verified the
/// member earns the "Onboarded" status and can claim a reward.
#[contract]
pub struct OnboardingQuest;

#[contractimpl]
impl OnboardingQuest {
    // -----------------------------------------------------------------
    // Admin / setup
    // -----------------------------------------------------------------

    /// Initialize the contract and set the community admin. Must be called
    /// exactly once, after which the admin is the only address allowed to
    /// add quest steps and verify member submissions.
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::StepCount, &0u32);
    }

    /// Register a new quest step. The admin supplies a numeric `step_id`
    /// (caller-chosen, must be unique), a `description_hash` (SHA-256 of
    /// the step description kept off-chain), and a `required` flag. Returns
    /// the new total number of steps.
    pub fn add_step(
        env: Env,
        admin: Address,
        step_id: u32,
        description_hash: BytesN<32>,
        required: bool,
    ) -> u32 {
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        let key = DataKey::Step(step_id);
        if env.storage().instance().has(&key) {
            panic!("step already exists");
        }

        let step = Step {
            description_hash,
            required,
        };
        env.storage().instance().set(&key, &step);

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::StepCount)
            .unwrap_or(0);
        let new_count = count + 1;
        env.storage().instance().set(&DataKey::StepCount, &new_count);
        new_count
    }

    // -----------------------------------------------------------------
    // Member actions
    // -----------------------------------------------------------------

    /// Record that `user` has completed `step_id` and submit a `proof_hash`
    /// as evidence. The completion is stored as *unverified* until the
    /// admin calls `verify_step`. A member may resubmit a new proof for
    /// the same step; doing so overwrites the previous record.
    pub fn complete_step(env: Env, user: Address, step_id: u32, proof_hash: BytesN<32>) {
        user.require_auth();

        if !env.storage().instance().has(&DataKey::Step(step_id)) {
            panic!("unknown step");
        }

        let completion = Completion {
            proof_hash,
            verified: false,
            completed_at: env.ledger().timestamp(),
            verified_at: 0,
        };
        env
            .storage()
            .instance()
            .set(&DataKey::Completion(user, step_id), &completion);
    }

    /// Claim the Onboarded reward. The caller must have verified every
    /// required step (see `is_onboarded`) and must not have claimed
    /// before. Marks the user as rewarded and emits a `reward` event
    /// with the ledger timestamp; no real XLM transfer is performed.
    pub fn claim_reward(env: Env, user: Address) -> bool {
        user.require_auth();

        if !Self::is_onboarded(env.clone(), user.clone()) {
            panic!("not fully onboarded");
        }
        let rewarded_key = DataKey::Rewarded(user.clone());
        if env
            .storage()
            .instance()
            .get::<DataKey, bool>(&rewarded_key)
            .unwrap_or(false)
        {
            panic!("already rewarded");
        }

        env.storage().instance().set(&rewarded_key, &true);
        env.events()
            .publish((REWARD_EVENT, user), env.ledger().timestamp());
        true
    }

    // -----------------------------------------------------------------
    // Admin verification
    // -----------------------------------------------------------------

    /// Admin verifies a member's completion of a step. After verification
    /// the contract automatically promotes the member to "Onboarded" if
    /// every required step is now verified. Subsequent calls for the same
    /// `(user, step_id)` pair panic to avoid double-counting.
    pub fn verify_step(env: Env, admin: Address, user: Address, step_id: u32) {
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        let key = DataKey::Completion(user.clone(), step_id);
        let mut completion: Completion = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic!("step not completed by user"));
        if completion.verified {
            panic!("already verified");
        }

        completion.verified = true;
        completion.verified_at = env.ledger().timestamp();
        env.storage().instance().set(&key, &completion);

        // Promote to "Onboarded" as soon as every required step is verified.
        if !env
            .storage()
            .instance()
            .has(&DataKey::Onboarded(user.clone()))
            && Self::is_fully_complete(&env, &user)
        {
            env.storage()
                .instance()
                .set(&DataKey::Onboarded(user), &true);
        }
    }

    // -----------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------

    /// Returns `true` if the user has reached the "Onboarded" status,
    /// meaning every required quest step has been verified.
    pub fn is_onboarded(env: Env, user: Address) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Onboarded(user))
            .unwrap_or(false)
    }

    /// Returns the number of steps the given user has had *verified* by
    /// the admin so far (across all steps, required or optional). This is
    /// the user's onboarding progress in absolute terms.
    pub fn progress(env: Env, user: Address) -> u32 {
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::StepCount)
            .unwrap_or(0);
        let mut done: u32 = 0;
        let mut i: u32 = 0;
        while i < count {
            let key = DataKey::Completion(user.clone(), i);
            if let Some(c) = env.storage().instance().get::<DataKey, Completion>(&key) {
                if c.verified {
                    done += 1;
                }
            }
            i += 1;
        }
        done
    }

    /// Returns the stored completion record for a `(user, step_id)` pair,
    /// or `None` if the user has not submitted proof for that step.
    pub fn get_completion(env: Env, user: Address, step_id: u32) -> Option<Completion> {
        env.storage()
            .instance()
            .get(&DataKey::Completion(user, step_id))
    }

    /// Returns the stored definition for the given step id, or `None` if
    /// no such step has been registered.
    pub fn get_step(env: Env, step_id: u32) -> Option<Step> {
        env.storage().instance().get(&DataKey::Step(step_id))
    }

    /// Returns the total number of quest steps currently registered.
    pub fn step_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::StepCount)
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------

    /// Verify that `admin` is the stored admin; panic otherwise.
    fn assert_admin(env: &Env, admin: &Address) {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("contract not initialized"));
        if stored != *admin {
            panic!("caller is not admin");
        }
    }

    /// Check that every step flagged `required` has a verified completion
    /// record for the given user. Optional steps are ignored.
    fn is_fully_complete(env: &Env, user: &Address) -> bool {
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::StepCount)
            .unwrap_or(0);
        let mut i: u32 = 0;
        while i < count {
            let step: Step = env
                .storage()
                .instance()
                .get(&DataKey::Step(i))
                .unwrap();
            if step.required {
                let key = DataKey::Completion(user.clone(), i);
                let verified = env
                    .storage()
                    .instance()
                    .get::<DataKey, Completion>(&key)
                    .map(|c| c.verified)
                    .unwrap_or(false);
                if !verified {
                    return false;
                }
            }
            i += 1;
        }
        true
    }
}
