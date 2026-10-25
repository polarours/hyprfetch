use std::env;
use std::time::Duration;

use serde::Serialize;

use super::ProviderError;
use super::SystemProvider;
use super::run_with_timeout;

/// How long the media tool may take to answer.
const MEDIA_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// The fields that are read from a player.
const MEDIA_FORMAT: &str = "{{playerName}}\t{{title}}\t{{artist}}\t{{album}}\t{{status}}";

/// How many fields every line of the output holds.
const MEDIA_FIELDS: usize = 5;

/// A media player that is running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MediaPlayer {
    /// The name of the player, for example "spotify".
    pub name: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// What the player is doing, for example "Playing".
    pub status: Option<String>,
}

impl MediaPlayer {
    /// The track of the player, as a single line.
    pub fn track(&self) -> Option<String> {
        let title = self.title.as_deref().unwrap_or_default();
        let artist = self.artist.as_deref().unwrap_or_default();

        match (title.is_empty(), artist.is_empty()) {
            (false, false) => Some(format!("{artist} - {title}")),
            (false, true) => Some(title.to_string()),
            (true, false) => Some(artist.to_string()),
            (true, true) => None,
        }
    }
}

/// Reads the media players that are running.
///
/// The players speak MPRIS over the session bus, which `playerctl` reads for
/// us.
fn read_media_info() -> Result<Vec<MediaPlayer>, ProviderError> {
    if !program_in_path("playerctl") {
        return Err(ProviderError::not_found(
            "playerctl not found, it reads the media players",
        ));
    }

    let output = run_with_timeout(
        "playerctl",
        &["--all-players", "metadata", "--format", MEDIA_FORMAT],
        MEDIA_PROBE_TIMEOUT,
    )
    .ok_or_else(|| {
        ProviderError::not_found("no media player is running (playerctl found no player)")
    })?;

    let players = parse_media_players(&output);
    if players.is_empty() {
        return Err(ProviderError::not_found(
            "no media player is running (playerctl found no player)",
        ));
    }

    Ok(players)
}

/// The players in the output of `playerctl`.
///
/// Every line holds the fields of one player, separated by tabs.
fn parse_media_players(output: &str) -> Vec<MediaPlayer> {
    output
        .lines()
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() < MEDIA_FIELDS {
                return None;
            }

            let name = fields[0].trim();
            if name.is_empty() {
                return None;
            }

            Some(MediaPlayer {
                name: name.to_string(),
                title: media_field(fields[1]),
                artist: media_field(fields[2]),
                album: media_field(fields[3]),
                status: media_field(fields[4]),
            })
        })
        .collect()
}

/// One field of a player, if it holds something.
fn media_field(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// Whether a program can be found in the search path.
fn program_in_path(program: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path).any(|directory| directory.join(program).is_file())
}

impl SystemProvider {
    pub fn media_info(&self) -> Result<Vec<MediaPlayer>, ProviderError> {
        read_media_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_players_from_playerctl() {
        let output = "spotify\tNever Gonna Give You Up\tRick Astley\tWhenever You Need Somebody\tPlaying\nfirefox\tA video\t\t\tPaused\n";

        let players = parse_media_players(output);
        assert_eq!(
            players,
            vec![
                MediaPlayer {
                    name: "spotify".to_string(),
                    title: Some("Never Gonna Give You Up".to_string()),
                    artist: Some("Rick Astley".to_string()),
                    album: Some("Whenever You Need Somebody".to_string()),
                    status: Some("Playing".to_string()),
                },
                MediaPlayer {
                    name: "firefox".to_string(),
                    title: Some("A video".to_string()),
                    artist: None,
                    album: None,
                    status: Some("Paused".to_string()),
                },
            ]
        );
    }

    #[test]
    fn reads_a_player_without_a_track() {
        let players = parse_media_players("mpv\t\t\t\tStopped\n");
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "mpv");
        assert_eq!(players[0].track(), None);
    }

    #[test]
    fn names_the_track_of_a_player() {
        let player = MediaPlayer {
            name: "spotify".to_string(),
            title: Some("Never Gonna Give You Up".to_string()),
            artist: Some("Rick Astley".to_string()),
            album: None,
            status: Some("Playing".to_string()),
        };
        assert_eq!(
            player.track().as_deref(),
            Some("Rick Astley - Never Gonna Give You Up")
        );

        let without_artist = MediaPlayer {
            artist: None,
            ..player.clone()
        };
        assert_eq!(
            without_artist.track().as_deref(),
            Some("Never Gonna Give You Up")
        );
    }

    #[test]
    fn has_no_players_when_playerctl_reports_none() {
        assert!(parse_media_players("").is_empty());
        assert!(parse_media_players("No players found\n").is_empty());
    }
}
