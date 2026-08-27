pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn seeded(label: &str) -> Self {
        Self { state: fnv1a(label) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, limit: u64) -> u64 {
        if limit == 0 {
            return 0;
        }
        self.next_u64() % limit
    }

    pub fn between(&mut self, low: i64, high: i64) -> i64 {
        if high <= low {
            return low;
        }
        low + self.below((high - low + 1) as u64) as i64
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    pub fn pick<'a, T>(&mut self, options: &'a [T]) -> &'a T {
        &options[self.below(options.len() as u64) as usize]
    }
}

fn fnv1a(text: &str) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_label_always_gives_the_same_stream() {
        let first: Vec<i64> = (0..40).map(|_| Rng::seeded("team_a:q1").between(0, 1000)).collect();
        let mut a = Rng::seeded("team_a:q1");
        let mut b = Rng::seeded("team_a:q1");
        assert!(first.len() == 40);
        for _ in 0..200 {
            assert_eq!(a.between(0, 1_000_000), b.between(0, 1_000_000));
        }
    }

    #[test]
    fn different_teams_get_different_streams() {
        let mut a = Rng::seeded("team_a:q1");
        let mut b = Rng::seeded("team_b:q1");
        let left: Vec<i64> = (0..40).map(|_| a.between(0, 1_000_000)).collect();
        let right: Vec<i64> = (0..40).map(|_| b.between(0, 1_000_000)).collect();
        assert_ne!(left, right);
    }

    #[test]
    fn different_questions_get_different_streams() {
        let mut a = Rng::seeded("team_a:q1");
        let mut b = Rng::seeded("team_a:q2");
        let left: Vec<i64> = (0..40).map(|_| a.between(0, 1_000_000)).collect();
        let right: Vec<i64> = (0..40).map(|_| b.between(0, 1_000_000)).collect();
        assert_ne!(left, right);
    }

    #[test]
    fn between_stays_inside_the_range() {
        let mut rng = Rng::seeded("range");
        for _ in 0..5_000 {
            let value = rng.between(-5, 5);
            assert!((-5..=5).contains(&value));
        }
        assert_eq!(rng.between(7, 7), 7);
    }
}
