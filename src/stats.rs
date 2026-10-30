use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct GameStats {
    pub games_played: u32,
    pub games_won: u32,
    pub current_streak: u32,
    pub max_streak: u32,
    pub guess_distribution: HashMap<u32, u32>, // guess_count -> number_of_games
}

impl GameStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn win_percentage(&self) -> f32 {
        if self.games_played == 0 {
            0.0
        } else {
            (self.games_won as f32 / self.games_played as f32) * 100.0
        }
    }

    pub fn average_guesses(&self) -> f32 {
        if self.games_won == 0 {
            0.0
        } else {
            let total_guesses: u32 = self
                .guess_distribution
                .iter()
                .map(|(guesses, count)| guesses * count)
                .sum();
            total_guesses as f32 / self.games_won as f32
        }
    }

    pub fn record_game(&mut self, won: bool, guess_count: Option<usize>) {
        self.games_played += 1;

        if won {
            self.games_won += 1;
            self.current_streak += 1;
            if self.current_streak > self.max_streak {
                self.max_streak = self.current_streak;
            }

            if let Some(guesses) = guess_count {
                *self.guess_distribution.entry(guesses as u32).or_insert(0) += 1;
            }
        } else {
            self.current_streak = 0;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Default)]
pub struct StatsManager {
    stats: GameStats,
    file_path: Option<PathBuf>, // None = in-memory only (no writable data dir)
}

impl StatsManager {
    /// Loads stats from the platform data directory:
    /// - Linux: $XDG_DATA_HOME/letterlock/stats.json (~/.local/share/...)
    /// - macOS: ~/Library/Application Support/letterlock/stats.json
    /// - Windows: %APPDATA%\letterlock\stats.json
    ///
    /// Never fails: if no data directory is available, stats are kept in memory.
    pub fn new() -> Self {
        match Self::default_path() {
            Some(path) => {
                Self::migrate_legacy_file(&path);
                Self::with_path(path)
            }
            None => Self {
                stats: GameStats::default(),
                file_path: None,
            },
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        let stats = match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| {
                // Keep the unreadable file around rather than silently overwriting it
                let _ = fs::rename(&path, path.with_extension("json.bak"));
                GameStats::default()
            }),
            Err(_) => GameStats::default(),
        };

        Self {
            stats,
            file_path: Some(path),
        }
    }

    pub fn default_path() -> Option<PathBuf> {
        let mut path = dirs::data_dir()?;
        path.push(env!("CARGO_PKG_NAME"));
        path.push("stats.json");
        Some(path)
    }

    /// Versions before 0.2 stored stats in ~/.cwordle_stats.json
    fn migrate_legacy_file(path: &Path) {
        let Some(legacy) = dirs::home_dir().map(|h| h.join(".cwordle_stats.json")) else {
            return;
        };
        if legacy.exists() && !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::copy(&legacy, path).is_ok() {
                let _ = fs::remove_file(&legacy);
            }
        }
    }

    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    pub fn get_stats(&self) -> &GameStats {
        &self.stats
    }

    pub fn record_game(
        &mut self,
        won: bool,
        guess_count: Option<usize>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.stats.record_game(won, guess_count);
        self.save()
    }

    pub fn reset_stats(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stats.reset();
        self.save()
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(path) = &self.file_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Write to a temp file then rename, so a crash can't leave a half-written file
        let content = serde_json::to_string_pretty(&self.stats)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_stats_default() {
        let stats = GameStats::default();
        assert_eq!(stats.games_played, 0);
        assert_eq!(stats.games_won, 0);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.max_streak, 0);
        assert_eq!(stats.win_percentage(), 0.0);
        assert_eq!(stats.average_guesses(), 0.0);
    }

    #[test]
    fn test_record_win() {
        let mut stats = GameStats::default();
        stats.record_game(true, Some(3));

        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.games_won, 1);
        assert_eq!(stats.current_streak, 1);
        assert_eq!(stats.max_streak, 1);
        assert_eq!(stats.guess_distribution.get(&3), Some(&1));
        assert_eq!(stats.win_percentage(), 100.0);
        assert_eq!(stats.average_guesses(), 3.0);
    }

    #[test]
    fn test_record_loss() {
        let mut stats = GameStats::default();
        stats.record_game(false, None);

        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.games_won, 0);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.max_streak, 0);
        assert_eq!(stats.win_percentage(), 0.0);
    }

    #[test]
    fn test_win_streak() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(3));
        assert_eq!(stats.current_streak, 1);
        assert_eq!(stats.max_streak, 1);

        stats.record_game(true, Some(4));
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.max_streak, 2);

        stats.record_game(true, Some(2));
        assert_eq!(stats.current_streak, 3);
        assert_eq!(stats.max_streak, 3);
    }

    #[test]
    fn test_streak_broken_by_loss() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(3));
        stats.record_game(true, Some(4));
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.max_streak, 2);

        stats.record_game(false, None);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.max_streak, 2);
    }

    #[test]
    fn test_max_streak_tracking() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(3));
        stats.record_game(true, Some(4));
        assert_eq!(stats.max_streak, 2);

        stats.record_game(false, None);

        stats.record_game(true, Some(2));
        stats.record_game(true, Some(3));
        stats.record_game(true, Some(4));

        assert_eq!(stats.current_streak, 3);
        assert_eq!(stats.max_streak, 3);
    }

    #[test]
    fn test_guess_distribution() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(1));
        stats.record_game(true, Some(2));
        stats.record_game(true, Some(2));
        stats.record_game(true, Some(3));
        stats.record_game(true, Some(3));
        stats.record_game(true, Some(3));

        assert_eq!(stats.guess_distribution.get(&1), Some(&1));
        assert_eq!(stats.guess_distribution.get(&2), Some(&2));
        assert_eq!(stats.guess_distribution.get(&3), Some(&3));
        assert_eq!(stats.guess_distribution.get(&4), None);
    }

    #[test]
    fn test_win_percentage() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(3));
        stats.record_game(true, Some(4));
        stats.record_game(false, None);
        stats.record_game(false, None);

        assert_eq!(stats.games_played, 4);
        assert_eq!(stats.games_won, 2);
        assert_eq!(stats.win_percentage(), 50.0);
    }

    #[test]
    fn test_average_guesses() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(2));
        stats.record_game(true, Some(4));
        stats.record_game(true, Some(6));

        assert_eq!(stats.average_guesses(), 4.0);
    }

    #[test]
    fn test_reset_stats() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(3));
        stats.record_game(true, Some(4));

        assert_eq!(stats.games_played, 2);

        stats.reset();

        assert_eq!(stats.games_played, 0);
        assert_eq!(stats.games_won, 0);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.max_streak, 0);
        assert!(stats.guess_distribution.is_empty());
    }

    #[test]
    fn test_complex_game_scenario() {
        let mut stats = GameStats::default();

        stats.record_game(true, Some(3));
        stats.record_game(true, Some(4));
        stats.record_game(false, None);
        stats.record_game(true, Some(2));
        stats.record_game(true, Some(5));
        stats.record_game(true, Some(3));
        stats.record_game(true, Some(4));
        stats.record_game(false, None);
        stats.record_game(true, Some(6));
        stats.record_game(true, Some(3));

        assert_eq!(stats.games_played, 10);
        assert_eq!(stats.games_won, 8);
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.max_streak, 4);
        assert_eq!(stats.win_percentage(), 80.0);

        assert_eq!(stats.guess_distribution.get(&2), Some(&1));
        assert_eq!(stats.guess_distribution.get(&3), Some(&3));
        assert_eq!(stats.guess_distribution.get(&4), Some(&2));
        assert_eq!(stats.guess_distribution.get(&5), Some(&1));
        assert_eq!(stats.guess_distribution.get(&6), Some(&1));
    }

    fn temp_stats_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("letterlock-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("nested").join("stats.json")
    }

    #[test]
    fn test_stats_manager_persists() {
        let path = temp_stats_path("persist");

        let mut manager = StatsManager::with_path(path.clone());
        manager.record_game(true, Some(3)).unwrap();
        manager.record_game(false, None).unwrap();

        let reloaded = StatsManager::with_path(path.clone());
        assert_eq!(reloaded.get_stats().games_played, 2);
        assert_eq!(reloaded.get_stats().games_won, 1);

        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn test_stats_manager_backs_up_corrupt_file() {
        let path = temp_stats_path("corrupt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();

        let manager = StatsManager::with_path(path.clone());
        assert_eq!(manager.get_stats().games_played, 0);
        assert_eq!(
            fs::read_to_string(path.with_extension("json.bak")).unwrap(),
            "not json"
        );

        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }
}
