use gyro_breakout::render::{self, Display, WHITE, YELLOW};
use gyro_breakout::scores::Scores;

#[test]
fn keeps_fastest_five_and_places_ties_after_existing_times() {
    let mut scores = Scores::default();
    assert_eq!(scores.record(100), Some(0));
    assert_eq!(scores.record(60), Some(0));
    assert_eq!(scores.record(80), Some(1));
    assert_eq!(scores.record(110), Some(3));
    assert_eq!(scores.record(120), Some(4));
    assert_eq!(scores.record(130), None);
    assert_eq!(scores.record(60), Some(1));
    assert_eq!(
        scores.best,
        [Some(60), Some(60), Some(80), Some(100), Some(110)]
    );
    assert_eq!(scores.record(60), Some(2));
    assert_eq!(scores.record(60), Some(3));
    assert_eq!(scores.record(60), Some(4));
    assert_eq!(scores.record(60), None);
}

#[test]
fn result_highlights_current_time_and_its_rank() {
    #[derive(Default)]
    struct DisplayLog(Vec<(i32, u16)>);
    impl Display for DisplayLog {
        fn rect(&mut self, _: i32, y: i32, _: i32, _: i32, color: u16) {
            self.0.push((y, color));
        }
    }
    let mut scores = Scores::default();
    scores.record(30);
    let rank = scores.record(40);
    let mut display = DisplayLog::default();
    render::result(&mut display, true, 40, &scores, rank);
    assert!(display
        .0
        .iter()
        .any(|&(y, c)| (57..71).contains(&y) && c == YELLOW));
    assert!(display
        .0
        .iter()
        .any(|&(y, c)| (120..134).contains(&y) && c == WHITE));
    assert!(display
        .0
        .iter()
        .any(|&(y, c)| (144..158).contains(&y) && c == YELLOW));
}
