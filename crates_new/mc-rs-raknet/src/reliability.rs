use crate::packet::frame::AckRecord;

/// Compress a sorted list of sequence numbers into compact ACK records.
pub fn compress_ack_records(seq_nums: &mut Vec<u32>) -> Vec<AckRecord> {
    if seq_nums.is_empty() {
        return Vec::new();
    }
    seq_nums.sort_unstable();
    seq_nums.dedup();

    let mut records = Vec::new();
    let mut start = seq_nums[0];
    let mut end = start;

    for &seq in &seq_nums[1..] {
        if seq == end + 1 {
            end = seq;
        } else {
            if start == end {
                records.push(AckRecord::Single(start));
            } else {
                records.push(AckRecord::Range {
                    min: start,
                    max: end,
                });
            }
            start = seq;
            end = seq;
        }
    }

    if start == end {
        records.push(AckRecord::Single(start));
    } else {
        records.push(AckRecord::Range {
            min: start,
            max: end,
        });
    }

    records
}

/// Expand ACK records into individual sequence numbers.
pub fn expand_ack_records(records: &[AckRecord]) -> Vec<u32> {
    let mut result = Vec::new();
    for record in records {
        match record {
            AckRecord::Single(seq) => result.push(*seq),
            AckRecord::Range { min, max } => {
                for seq in *min..=*max {
                    result.push(seq);
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_single() {
        let mut seqs = vec![5];
        let records = compress_ack_records(&mut seqs);
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0], AckRecord::Single(5)));
    }

    #[test]
    fn compress_range() {
        let mut seqs = vec![1, 2, 3, 4, 5];
        let records = compress_ack_records(&mut seqs);
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0], AckRecord::Range { min: 1, max: 5 }));
    }

    #[test]
    fn compress_mixed() {
        let mut seqs = vec![1, 2, 3, 5, 7, 8, 9];
        let records = compress_ack_records(&mut seqs);
        assert_eq!(records.len(), 3);
        assert!(matches!(records[0], AckRecord::Range { min: 1, max: 3 }));
        assert!(matches!(records[1], AckRecord::Single(5)));
        assert!(matches!(records[2], AckRecord::Range { min: 7, max: 9 }));
    }

    #[test]
    fn compress_empty() {
        let mut seqs = vec![];
        let records = compress_ack_records(&mut seqs);
        assert!(records.is_empty());
    }

    #[test]
    fn expand_roundtrip() {
        let mut seqs = vec![1, 2, 3, 5, 7, 8, 9];
        let records = compress_ack_records(&mut seqs);
        let expanded = expand_ack_records(&records);
        assert_eq!(expanded, vec![1, 2, 3, 5, 7, 8, 9]);
    }

    #[test]
    fn compress_unsorted_dedup() {
        let mut seqs = vec![3, 1, 2, 2, 5];
        let records = compress_ack_records(&mut seqs);
        let expanded = expand_ack_records(&records);
        assert_eq!(expanded, vec![1, 2, 3, 5]);
    }
}
