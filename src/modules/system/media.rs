use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::system::MediaPlayer;

pub struct MediaModule;

impl Module for MediaModule {
    fn id(&self) -> ModuleId {
        ModuleId::Media
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.media_info() {
            Ok(players) => {
                let compact = config.option_bool("compact", true);
                let mut rows = if compact {
                    vec![OutputRow::new("Media", summarize_media(&players))]
                } else {
                    media_rows(&players)
                };

                // Every player on its own row, so a rice screenshot shows
                // what plays and where it comes from.
                if config.option_bool("list", false) {
                    for player in listed_players(&players) {
                        rows.push(player);
                    }
                }

                ModuleOutput::ok(self.id(), rows, json!({ "players": players }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_media(players: &[MediaPlayer]) -> String {
    let mut parts = Vec::new();

    for player in players {
        let mut part = player.track().unwrap_or_else(|| player.name.clone());
        if let Some(status) = player.status.as_deref() {
            part.push_str(&format!(" ({})", status.to_lowercase()));
        }
        parts.push(part);
    }

    parts.join(", ")
}

fn media_rows(players: &[MediaPlayer]) -> Vec<OutputRow> {
    let mut rows = Vec::new();

    for player in players {
        if let Some(track) = player.track() {
            rows.push(OutputRow::new(player.name.clone(), track));
        }
        if let Some(status) = player.status.as_deref() {
            rows.push(OutputRow::new("Status", status.to_lowercase()));
        }
        if let Some(album) = player.album.as_deref() {
            rows.push(OutputRow::new("Album", album.to_string()));
        }
    }

    rows
}

/// One row per player, with the track it plays.
fn listed_players(players: &[MediaPlayer]) -> Vec<OutputRow> {
    players
        .iter()
        .map(|player| {
            let mut value = player.track().unwrap_or_else(|| player.name.clone());
            if let Some(status) = player.status.as_deref() {
                value.push_str(&format!(", {}", status.to_lowercase()));
            }
            OutputRow::new(player.name.clone(), value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(name: &str, title: &str, artist: &str, status: &str) -> MediaPlayer {
        MediaPlayer {
            name: name.to_string(),
            title: (!title.is_empty()).then(|| title.to_string()),
            artist: (!artist.is_empty()).then(|| artist.to_string()),
            album: None,
            status: (!status.is_empty()).then(|| status.to_string()),
        }
    }

    #[test]
    fn summarizes_the_media_players() {
        let players = vec![
            player(
                "spotify",
                "Never Gonna Give You Up",
                "Rick Astley",
                "Playing",
            ),
            player("firefox", "A video", "", "Paused"),
        ];

        assert_eq!(
            summarize_media(&players),
            "Rick Astley - Never Gonna Give You Up (playing), A video (paused)"
        );
    }

    #[test]
    fn lists_every_media_player() {
        let players = vec![
            player(
                "spotify",
                "Never Gonna Give You Up",
                "Rick Astley",
                "Playing",
            ),
            player("mpv", "", "", ""),
        ];

        let rows = listed_players(&players);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key, "spotify");
        assert_eq!(
            rows[0].value,
            "Rick Astley - Never Gonna Give You Up, playing"
        );
        assert_eq!(rows[1].key, "mpv");
        assert_eq!(rows[1].value, "mpv");
    }
}
