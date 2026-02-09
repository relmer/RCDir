// listing_totals.rs — Aggregate counts for directory listings
//
// Port of: ListingTotals.h → SListingTotals

/// Aggregates file/directory/stream counts and sizes.
/// Port of: SListingTotals
#[derive(Debug, Clone, Default)]
pub struct ListingTotals {
    pub file_count:       u32,
    pub directory_count:  u32,
    pub file_bytes:       u64,
    pub stream_count:     u32,
    pub stream_bytes:     u64,
}

impl ListingTotals {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accumulate totals from another ListingTotals.
    /// Port of: SListingTotals::Add
    pub fn add(&mut self, other: &ListingTotals) {
        self.file_count      += other.file_count;
        self.directory_count += other.directory_count;
        self.file_bytes      += other.file_bytes;
        self.stream_count    += other.stream_count;
        self.stream_bytes    += other.stream_bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_zero() {
        let t = ListingTotals::new();
        assert_eq!(t.file_count, 0);
        assert_eq!(t.directory_count, 0);
        assert_eq!(t.file_bytes, 0);
    }

    #[test]
    fn add_accumulates() {
        let mut a = ListingTotals { file_count: 3, directory_count: 1, file_bytes: 1000, stream_count: 0, stream_bytes: 0 };
        let b = ListingTotals { file_count: 5, directory_count: 2, file_bytes: 2000, stream_count: 1, stream_bytes: 100 };
        a.add(&b);
        assert_eq!(a.file_count, 8);
        assert_eq!(a.directory_count, 3);
        assert_eq!(a.file_bytes, 3000);
        assert_eq!(a.stream_count, 1);
        assert_eq!(a.stream_bytes, 100);
    }
}
