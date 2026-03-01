use crate::CanId;

/// Filter for CAN frame acceptance.
#[derive(Debug, Clone)]
pub enum CanFilter {
    /// Accept all frames.
    AcceptAll,
    /// Accept only frames with this exact ID.
    Exact(CanId),
    /// Accept frames with IDs in [low, high] inclusive.
    Range { low: CanId, high: CanId },
    /// Accept frames where (id & mask) == filter.
    Mask { filter: u16, mask: u16 },
    /// Accept frames matching any of these filters.
    Any(Vec<CanFilter>),
}

impl CanFilter {
    pub fn matches(&self, id: CanId) -> bool {
        match self {
            Self::AcceptAll => true,
            Self::Exact(target) => id == *target,
            Self::Range { low, high } => id >= *low && id <= *high,
            Self::Mask { filter, mask } => (id.raw() & mask) == *filter,
            Self::Any(filters) => filters.iter().any(|f| f.matches(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_accept_all() {
        let filter = CanFilter::AcceptAll;
        assert!(filter.matches(CanId::new(0x100).unwrap()));
        assert!(filter.matches(CanId::new(0x7FF).unwrap()));
    }

    #[test]
    fn filter_exact() {
        let filter = CanFilter::Exact(CanId::new(0x100).unwrap());
        assert!(filter.matches(CanId::new(0x100).unwrap()));
        assert!(!filter.matches(CanId::new(0x101).unwrap()));
    }

    #[test]
    fn filter_range() {
        let filter = CanFilter::Range {
            low: CanId::new(0x100).unwrap(),
            high: CanId::new(0x110).unwrap(),
        };
        assert!(filter.matches(CanId::new(0x100).unwrap()));
        assert!(filter.matches(CanId::new(0x108).unwrap()));
        assert!(filter.matches(CanId::new(0x110).unwrap()));
        assert!(!filter.matches(CanId::new(0x111).unwrap()));
    }

    #[test]
    fn filter_mask() {
        // Match all IDs where bits 8-10 are 0x1 (0x100-0x1FF)
        let filter = CanFilter::Mask {
            filter: 0x100,
            mask: 0x700,
        };
        assert!(filter.matches(CanId::new(0x100).unwrap()));
        assert!(filter.matches(CanId::new(0x1FF).unwrap()));
        assert!(!filter.matches(CanId::new(0x200).unwrap()));
    }

    #[test]
    fn filter_any() {
        let filter = CanFilter::Any(vec![
            CanFilter::Exact(CanId::new(0x100).unwrap()),
            CanFilter::Exact(CanId::new(0x200).unwrap()),
        ]);
        assert!(filter.matches(CanId::new(0x100).unwrap()));
        assert!(filter.matches(CanId::new(0x200).unwrap()));
        assert!(!filter.matches(CanId::new(0x300).unwrap()));
    }
}
