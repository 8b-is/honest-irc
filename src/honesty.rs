use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use zeroize::Zeroize;

/// Honesty Auth — identity through personality, not passwords.
/// The honesty vector is a set of deeply personal answers that only
/// the real person can answer consistently over time.
#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct HonestyVector {
    pub version: u32,
    pub fields: HonestyFields,
    pub sig: Option<String>, // Dilithium signature over hash of fields
}

#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct HonestyFields {
    /// Favorite computer game
    pub game: String,
    /// Favorite color (hex)
    pub color: String,
    /// Favorite poet
    pub poet: String,
    /// Favorite poem by that poet
    pub poem: String,
    /// Favorite band
    pub band: String,
    /// Favorite song by that band
    pub song: String,
    /// Date of birth
    pub birth_date: String,
    /// Place of birth
    pub birth_place: String,
    /// Relationship with mother: "good", "bad", "complicated"
    pub mother: String,
    /// Astrological main constellation
    pub constellation: String,
    /// Believes in random? "yes", "no", "sometimes"
    pub belief: String,
    /// First names (multiple, order matters)
    pub names: Vec<String>,
    /// Favorite country in the world
    pub country: String,
    /// Has pets now?
    pub pets: bool,
    /// Last time had sex: date and outcome
    pub last_sex_date: String,
    pub last_sex_outcome: String,
    /// Random nonce to prevent replay
    pub nonce: String,
}

impl HonestyVector {
    /// Compute the core identity hash — the first 5 fields define the self.
    /// These are considered immutable: game, color, poet, poem, band.
    pub fn core_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.fields.game.as_bytes());
        hasher.update(self.fields.color.as_bytes());
        hasher.update(self.fields.poet.as_bytes());
        hasher.update(self.fields.poem.as_bytes());
        hasher.update(self.fields.band.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Compute full hash
    pub fn full_hash(&self) -> String {
        let json = serde_json::to_string(&self.fields).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Compare two vectors. Returns (match_percentage, stable_fields_match, volatile_fields_match).
    pub fn compare(&self, other: &HonestyVector) -> (f64, bool, f64) {
        let mut total: f64 = 0.0;
        let mut matches: f64 = 0.0;

        // Stable fields (should NEVER change) — exact match required
        let stable_checks = [
            self.fields.birth_date == other.fields.birth_date,
            self.fields.birth_place == other.fields.birth_place,
            self.fields.names == other.fields.names,
            self.fields.constellation == other.fields.constellation,
        ];
        let stable_match = stable_checks.iter().all(|&x| x);

        // Semi-stable (rarely changes but can) — close match accepted
        let semi_checks = [
            (self.fields.game == other.fields.game, 1.0),
            (self.fields.color == other.fields.color, 0.5),
            (self.fields.country == other.fields.country, 0.5),
        ];
        for (check, weight) in semi_checks.iter() {
            total += weight;
            if *check { matches += weight; }
        }

        // Volatile fields (can change over time) — any answer acceptable
        let volatile_checks = [
            (self.fields.poet == other.fields.poet, 0.5),
            (self.fields.poem == other.fields.poem, 0.5),
            (self.fields.band == other.fields.band, 0.5),
            (self.fields.song == other.fields.song, 0.5),
            (self.fields.mother == other.fields.mother, 0.5),
            (self.fields.belief == other.fields.belief, 0.5),
            (self.fields.pets == other.fields.pets, 0.5),
            (self.fields.last_sex_outcome == other.fields.last_sex_outcome, 0.5),
        ];
        for (check, weight) in volatile_checks.iter() {
            total += weight;
            if *check { matches += weight; }
        }

        let vol_match = matches / total.max(0.01_f64);
        (vol_match * 100.0, stable_match, vol_match * 100.0)
    }

    /// Check if another vector likely belongs to the same identity.
    /// Requires stable fields match and overall match ≥ 80%.
    pub fn verify(&self, other: &HonestyVector) -> bool {
        let (overall, stable, _) = self.compare(other);
        stable && overall >= 80.0
    }

    /// Generate a challenge question based on a field that should be consistent.
    pub fn challenge(&self) -> String {
        format!(
            "Verify: what is your favorite {} by {}?",
            self.fields.poem, self.fields.poet
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vector() -> HonestyVector {
        HonestyVector {
            version: 1,
            fields: HonestyFields {
                game: "commander_keen_4".into(),
                color: "#070b16".into(),
                poet: "petofi_sandor".into(),
                poem: "szabadsag_szerelem".into(),
                band: "metallica".into(),
                song: "unforgiven_2".into(),
                birth_date: "1988-05-14".into(),
                birth_place: "budapest".into(),
                mother: "complicated".into(),
                constellation: "scorpio".into(),
                belief: "yes".into(),
                names: vec!["istvan".into(), "jozsef".into(), "peter".into()],
                country: "hungary".into(),
                pets: true,
                last_sex_date: "2026-07".into(),
                last_sex_outcome: "good_feelings".into(),
                nonce: "random_nonce".into(),
            },
            sig: None,
        }
    }

    #[test]
    fn test_same_identity() {
        let v1 = sample_vector();
        let mut v2 = sample_vector();
        assert!(v1.verify(&v2));

        // Change volatile field — still should verify
        v2.fields.song = "nothing_else_matters".into();
        assert!(v1.verify(&v2));
    }

    #[test]
    fn test_different_identity() {
        let v1 = sample_vector();
        let mut v2 = sample_vector();
        v2.fields.birth_date = "1970-01-01".into();
        assert!(!v1.verify(&v2));
    }

    #[test]
    fn test_core_hash_deterministic() {
        let v1 = sample_vector();
        let v2 = sample_vector();
        assert_eq!(v1.core_hash(), v2.core_hash());
    }
}
