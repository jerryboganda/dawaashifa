use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Macro to generate strongly-typed UUID newtypes to prevent argument-order mistakes.
macro_rules! id_type {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a new UUIDv7 time-ordered identifier.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wrap an existing UUID into this strongly-typed newtype.
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Extract the inner raw UUID.
            pub const fn into_inner(self) -> Uuid {
                self.0
            }

            /// Reference the inner raw UUID.
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
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

        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

id_type!(TenantId, "Unique identifier for a tenant organization");
id_type!(BranchId, "Unique identifier for a pharmacy branch");
id_type!(UserId, "Unique identifier for a system user / employee");
id_type!(ProductId, "Unique identifier for a product / SKU");
id_type!(BatchId, "Unique identifier for an inventory batch");
id_type!(OrderId, "Unique identifier for a customer order");
id_type!(CustomerId, "Unique identifier for a retail or B2B customer");
id_type!(
    PrescriptionId,
    "Unique identifier for an uploaded prescription"
);
id_type!(
    ConversationId,
    "Unique identifier for a customer conversation thread"
);
id_type!(MessageId, "Unique identifier for a WhatsApp message");
id_type!(RiderId, "Unique identifier for a delivery rider");
id_type!(ChannelId, "Unique identifier for a communication channel");
id_type!(
    PaymentId,
    "Unique identifier for a payment attempt / record"
);
id_type!(DeliveryId, "Unique identifier for a delivery task");
id_type!(InvoiceId, "Unique identifier for an FBR tax invoice");
id_type!(TaxCategoryId, "Unique identifier for a tax category");
id_type!(SupplierId, "Unique identifier for an inventory supplier");
id_type!(RoleId, "Unique identifier for an RBAC role");
id_type!(PermissionId, "Unique identifier for an RBAC permission");
id_type!(SessionId, "Unique identifier for a user session");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation_and_conversion() {
        let tid = TenantId::new();
        let uuid_raw = tid.into_inner();
        let tid_from = TenantId::from(uuid_raw);
        assert_eq!(tid, tid_from);
        assert_eq!(tid.to_string(), uuid_raw.to_string());
    }

    #[test]
    fn test_id_serialization_roundtrip() {
        let uid = UserId::new();
        let json = serde_json::to_string(&uid).expect("serialize UserId");
        assert_eq!(json, format!("\"{}\"", uid.into_inner()));

        let deserialized: UserId = serde_json::from_str(&json).expect("deserialize UserId");
        assert_eq!(uid, deserialized);
    }

    #[test]
    fn test_id_from_str() {
        let sample = "550e8400-e29b-41d4-a716-446655440000";
        let pid = ProductId::from_str(sample).expect("parse ProductId");
        assert_eq!(pid.to_string(), sample);
    }
}
