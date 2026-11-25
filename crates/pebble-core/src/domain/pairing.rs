use rand::Rng;

pub const PIN_LEN: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin(String);

impl Pin {
    pub fn generate<R: Rng + ?Sized>(rng: &mut R) -> Self {
        let digits = (0..PIN_LEN)
            .map(|_| char::from(b'0' + rng.random_range(0..10u8)))
            .collect();
        Self(digits)
    }

    pub fn from_wire(raw: &str) -> Option<Self> {
        let valid = raw.len() == PIN_LEN && raw.bytes().all(|b| b.is_ascii_digit());
        valid.then(|| Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn generated_pin_is_six_digits() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..100 {
            let pin = Pin::generate(&mut rng);
            assert!(Pin::from_wire(pin.as_str()).is_some(), "{}", pin.as_str());
        }
    }

    #[test]
    fn every_digit_is_reachable() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut seen = [false; 10];
        for _ in 0..200 {
            for b in Pin::generate(&mut rng).as_str().bytes() {
                seen[usize::from(b - b'0')] = true;
            }
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn from_wire_rejects_bad_pins() {
        for raw in ["", "12345", "1234567", "12a456", "１２３４５６"] {
            assert!(Pin::from_wire(raw).is_none(), "{raw}");
        }
    }
}
