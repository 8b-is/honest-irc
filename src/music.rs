use serde::{Deserialize, Serialize};

/// music.vaked.dev integration.
/// Each peer shares their listening choreography — a sequence of tracks
/// that defines their current state. The choreography IS the status.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choreography {
    pub name: String,
    pub description: String,
    pub tracks: Vec<Track>,
    pub current_position: usize,
    pub peer_id: String,
}

impl Choreography {
    pub fn current_track(&self) -> Option<&Track> {
        self.tracks.get(self.current_position)
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.current_position + 1, self.tracks.len())
    }

    /// Generate a status message from the current track.
    pub fn status(&self) -> String {
        if let Some(track) = self.current_track() {
            format!(
                "{} \u{266b} {} — {} ({}/{})",
                self.peer_id,
                track.title,
                self.name,
                self.current_position + 1,
                self.tracks.len()
            )
        } else {
            format!("[{}] no track playing", self.peer_id)
        }
    }
}

/// Sample choreographies for demo purposes.
pub fn sample_choreographies() -> Vec<Choreography> {
    vec![
        Choreography {
            name: "edesapa".into(),
            description: "For my father. Metallica, Unforgiven trilogy.".into(),
            tracks: vec![
                Track { title: "The Unforgiven".into(), artist: "Metallica".into(), album: "Metallica".into(), duration_secs: 387 },
                Track { title: "The Unforgiven II".into(), artist: "Metallica".into(), album: "Reload".into(), duration_secs: 396 },
                Track { title: "The Unforgiven III".into(), artist: "Metallica".into(), album: "Death Magnetic".into(), duration_secs: 467 },
                Track { title: "Nothing Else Matters".into(), artist: "Metallica".into(), album: "Metallica".into(), duration_secs: 388 },
                Track { title: "Fade to Black".into(), artist: "Metallica".into(), album: "Ride the Lightning".into(), duration_secs: 417 },
                Track { title: "One".into(), artist: "Metallica".into(), album: "...And Justice for All".into(), duration_secs: 446 },
                Track { title: "Enter Sandman".into(), artist: "Metallica".into(), album: "Metallica".into(), duration_secs: 331 },
            ],
            current_position: 0,
            peer_id: "[The Architect of Structural Honesty]".into(),
        },
        Choreography {
            name: "vivaldi-spring".into(),
            description: "Vivaldi Spring downtempo with quant-love salt.".into(),
            tracks: vec![
                Track { title: "Spring Mvt 1 — Allegro (downtempo)".into(), artist: "Vivaldi".into(), album: "The Four Seasons".into(), duration_secs: 210 },
                Track { title: "Spring Mvt 2 — Largo".into(), artist: "Vivaldi".into(), album: "The Four Seasons".into(), duration_secs: 156 },
                Track { title: "Spring Mvt 3 — Allegro".into(), artist: "Vivaldi".into(), album: "The Four Seasons".into(), duration_secs: 245 },
            ],
            current_position: 0,
            peer_id: "boglarka".into(),
        },
    ]
}

/// API client for music.vaked.dev.
/// In production: queries the actual music.vaked.dev API.
pub struct MusicClient;

impl MusicClient {
    /// Get the currently playing track for a peer.
    pub fn now_playing(peer_id: &str) -> Option<Choreography> {
        // In production: GET https://music.vaked.dev/api/now-playing?peer={peer_id}
        // For now: return the first sample choreography
        sample_choreographies()
            .into_iter()
            .find(|c| c.peer_id == peer_id)
            .or_else(|| {
                Some(Choreography {
                    name: "unknown".into(),
                    description: "".into(),
                    tracks: vec![],
                    current_position: 0,
                    peer_id: peer_id.into(),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choreography_status() {
        let choreo = sample_choreographies().into_iter().next().unwrap();
        let status = choreo.status();
        assert!(status.contains("The Unforgiven"));
        assert!(status.contains("edesapa"));
    }

    #[test]
    fn test_choreography_progress() {
        let choreo = sample_choreographies().into_iter().next().unwrap();
        let (pos, total) = choreo.progress();
        assert_eq!(pos, 1);
        assert_eq!(total, 7);
    }

    #[test]
    fn test_now_playing() {
        let result = MusicClient::now_playing("[The Architect of Structural Honesty]");
        assert!(result.is_some());
        let choreo = result.unwrap();
        assert_eq!(choreo.name, "edesapa");
    }
}
