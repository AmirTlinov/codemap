pub struct LedgerEvent { pub topic: String }
pub fn persist(event: &LedgerEvent) -> bool { !event.topic.is_empty() }
