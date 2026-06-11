//! Opaque identifier types. Each domain concept has its own ID type so
//! they cannot be accidentally interchanged.

use nutype::nutype;
use uuid::Uuid;

/// Newtype wrapping a UUID so the compiler distinguishes each ID family.
macro_rules! uuid_id {
    ($name:ident) => {
        #[nutype(
            validate(predicate = |_| true),
            derive(
                Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, Deserialize
            )
        )]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self::try_new(Uuid::new_v4()).expect("valid UUID")
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

uuid_id!(ProjectId);
uuid_id!(WorkItemId);
uuid_id!(StepId);
uuid_id!(LeaseId);
uuid_id!(GateId);
uuid_id!(EscalationId);
uuid_id!(SliceId);
uuid_id!(WorkflowId);
