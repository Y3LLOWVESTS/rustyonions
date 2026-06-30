//! RO:WHAT — ECON-adjacent DTOs used by ledger/rewarder (pure data).
//! RO:WHY  — Keep conservation-friendly shapes; no arithmetic logic here.

pub mod internal_roc_economics;
pub mod move_entry;

pub use internal_roc_economics::{
    InternalRocAntiFarmingConfigV1, InternalRocBpsSplitV1, InternalRocEconomicsConfigV1,
    InternalRocEconomicsConfigValidationError, InternalRocFutureFeaturePlaceholderV1,
    InternalRocPaidContentEconomicsV1, InternalRocRemainderSinkV1, InternalRocRewardCategoryCapV1,
    InternalRocRewardPoolEconomicsV1, InternalRocRoundingConfigV1, InternalRocRoundingModeV1,
    InternalRocUnitsV1, INTERNAL_ROC_BPS_DENOMINATOR, INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA,
    INTERNAL_ROC_ECONOMICS_CONFIG_VERSION,
};
pub use move_entry::MoveEntryV1;
