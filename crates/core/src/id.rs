use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

/// Macro to generate strongly-typed newtype UUID wrappers.
macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema)]
        #[schema(value_type = String, format = "uuid", example = "018f3a9e-4c5b-7b3a-9e1a-2b3c4d5e6f7a")]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a new time-ordered UUIDv7 for this identifier.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wrap an existing UUID.
            pub const fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            /// Extract underlying UUID.
            pub const fn into_inner(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }
    };
}

id_type!(TenantId);
id_type!(BranchId);
id_type!(UserId);
id_type!(RoleId);
id_type!(PermissionId);
id_type!(SessionId);
id_type!(CustomerId);
id_type!(CustomerAddressId);
id_type!(CategoryId);
id_type!(GenericId);
id_type!(ProductId);
id_type!(ProductAliasId);
id_type!(SupplierId);
id_type!(BatchId);
id_type!(StockMovementId);
id_type!(BusinessIdentityId);
id_type!(ChannelId);
id_type!(ConversationId);
id_type!(MessageId);
id_type!(PrescriptionId);
id_type!(RxLineId);
id_type!(ApprovalId);
id_type!(OrderId);
id_type!(OrderItemId);
id_type!(OrderEventId);
id_type!(PaymentId);
id_type!(ProofId);
id_type!(RiderId);
id_type!(DeliveryId);
id_type!(RiderCashSessionId);
id_type!(TaxCategoryId);
id_type!(InvoiceId);
id_type!(AuditLogId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation_and_conversion() {
        let tenant_id = TenantId::new();
        assert_eq!(tenant_id.0.get_version_num(), 7);

        let uuid = tenant_id.into_inner();
        let from_uuid = TenantId::from(uuid);
        assert_eq!(tenant_id, from_uuid);
    }

    #[test]
    fn test_id_from_str() {
        let tenant_id = TenantId::new();
        let s = tenant_id.to_string();
        let parsed = TenantId::from_str(&s).unwrap();
        assert_eq!(tenant_id, parsed);
    }

    #[test]
    fn test_id_serialization_roundtrip() {
        let tenant_id = TenantId::new();
        let json = serde_json::to_string(&tenant_id).unwrap();
        let deserialized: TenantId = serde_json::from_str(&json).unwrap();
        assert_eq!(tenant_id, deserialized);
    }
}
