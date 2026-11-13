//! Debounced user-button handling.
use crate::game::CYCLES_PER_SECOND;

/// Detect a stable release, press, and release, even across timer wraparound.
#[derive(Default)]
pub struct PressCycle {
    candidate: Option<bool>,
    changed: u32,
    phase: u8,
}

impl PressCycle {
    pub fn update(&mut self, pressed: bool, now: u32) -> bool {
        if self.candidate != Some(pressed) {
            self.candidate = Some(pressed);
            self.changed = now;
            return false;
        }
        if now.wrapping_sub(self.changed) < CYCLES_PER_SECOND / 50 {
            return false;
        }
        match (self.phase, pressed) {
            (0, false) => self.phase = 1,
            (1, true) => self.phase = 2,
            (2, false) => self.phase = 3,
            _ => {}
        }
        self.phase == 3
    }
}

#[cfg(test)]
mod tests {
    use super::PressCycle;
    use crate::game::CYCLES_PER_SECOND;

    #[test]
    fn requires_debounced_release_press_and_release_across_wrap() {
        let mut cycle = PressCycle::default();
        let mut now = u32::MAX - CYCLES_PER_SECOND / 100;
        let tick = CYCLES_PER_SECOND / 100;
        for raw in [
            true, true, true, false, false, false, true, false, true, true, true, false, false,
            false,
        ] {
            let done = cycle.update(raw, now);
            now = now.wrapping_add(tick);
            if done {
                assert!(!raw);
                assert_eq!(cycle.phase, 3);
                return;
            }
        }
        panic!("a stable button cycle should complete");
    }
}
