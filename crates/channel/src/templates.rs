use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory or database-backed template registry tracking approval status
#[derive(Debug, Default)]
pub struct TemplateRegistry {
    templates: RwLock<HashMap<String, String>>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        // Seed default approved utility templates per Doc 02 §7
        map.insert("order_confirmed".to_string(), "APPROVED".to_string());
        map.insert("order_dispatched".to_string(), "APPROVED".to_string());
        map.insert("order_delivered".to_string(), "APPROVED".to_string());
        map.insert("rx_ready_for_review".to_string(), "APPROVED".to_string());
        map.insert("payment_reminder".to_string(), "APPROVED".to_string());

        Self {
            templates: RwLock::new(map),
        }
    }

    pub async fn get_status(&self, name: &str) -> Option<String> {
        let map = self.templates.read().unwrap();
        map.get(name).cloned()
    }

    pub fn set_status(&self, name: String, status: String) {
        let mut map = self.templates.write().unwrap();
        map.insert(name, status);
    }
}
