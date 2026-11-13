//! Fastest completed rounds retained until reset.
#[derive(Default)]
pub struct Scores {
    pub best: [Option<u64>; 5],
}

impl Scores {
    /// Record a winning time in seconds, returning its rank if it enters the top five.
    pub fn record(&mut self, seconds: u64) -> Option<usize> {
        let rank = self
            .best
            .iter()
            .position(|entry| entry.is_none_or(|time| seconds < time))?;
        for i in (rank + 1..self.best.len()).rev() {
            self.best[i] = self.best[i - 1];
        }
        self.best[rank] = Some(seconds);
        Some(rank)
    }
}
