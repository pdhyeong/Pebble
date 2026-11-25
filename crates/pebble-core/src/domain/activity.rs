use serde::Serialize;
use std::collections::VecDeque;

use super::peer::PeerId;
use super::transfer::{Direction, TransferStatus};

#[derive(Debug, Clone, Serialize)]
pub struct ActivityRecord {
    pub id: String,
    pub transfer_type: Direction,
    pub file_name: String,
    pub peer_id: PeerId,
    pub device_name: Option<String>,
    pub size: u64,
    pub timestamp: i64,
    pub status: TransferStatus,
    pub speed: Option<f64>,
}

#[derive(Debug)]
pub struct ActivityLog {
    records: VecDeque<ActivityRecord>,
    capacity: usize,
}

impl ActivityLog {
    pub const DEFAULT_CAPACITY: usize = 100;

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, record: ActivityRecord) {
        if self.records.len() == self.capacity {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    pub fn snapshot(&self) -> Vec<ActivityRecord> {
        self.records.iter().cloned().collect()
    }
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(n: u64) -> ActivityRecord {
        ActivityRecord {
            id: n.to_string(),
            transfer_type: Direction::Upload,
            file_name: format!("f{n}"),
            peer_id: PeerId::parse("12D3KooW").unwrap(),
            device_name: None,
            size: n,
            timestamp: 0,
            status: TransferStatus::Completed,
            speed: None,
        }
    }

    #[test]
    fn keeps_only_latest_records_in_order() {
        let mut log = ActivityLog::with_capacity(3);
        for n in 0..5 {
            log.push(record(n));
        }
        let ids: Vec<_> = log.snapshot().into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["2", "3", "4"]);
    }

    #[test]
    fn serializes_contract_shape() {
        let json = serde_json::to_value(record(1)).unwrap();
        assert_eq!(json["transfer_type"], "upload");
        assert_eq!(json["status"], "completed");
        assert_eq!(json["peer_id"], "12D3KooW");
    }
}
