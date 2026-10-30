// Responsible for managing game state

use super::config::{Config, ALPHABET_SIZE, ASCII_LOWERCASE_A, WORD_LENGTH};
use super::words;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GameStatus {
    PLAYING = 0,
    WON = 1,
    LOST = 2,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyBoardHelperState {
    NONE = 0,
    GRAY = 1,
    AMBER = 2,
    GREEN = 3,
}
pub struct GameData {
    winning_word: String,
    guess_count: usize,
    max_guesses: usize,
    words_guessed: Vec<String>,
    scores: Vec<[KeyBoardHelperState; WORD_LENGTH]>,
    game_state: GameStatus,
    keyboard_helper: [KeyBoardHelperState; ALPHABET_SIZE],
    hard_mode: bool,
    revealed_letters: Vec<char>, // Letters that must be used in hard mode
}

/// Scores a guess against the answer, tile by tile.
///
/// Repeated letters are handled the standard way: exact matches are claimed
/// first, then each remaining answer letter can mark at most one guess letter
/// as AMBER. e.g. guessing "speed" for "abide" gives one AMBER 'e', not two.
pub fn score_guess(guess: &str, answer: &str) -> [KeyBoardHelperState; WORD_LENGTH] {
    let guess: Vec<char> = guess.chars().collect();
    let mut remaining: Vec<Option<char>> = answer.chars().map(Some).collect();
    let mut scores = [KeyBoardHelperState::GRAY; WORD_LENGTH];

    // First, letters in the correct position
    for i in 0..WORD_LENGTH {
        if remaining[i] == Some(guess[i]) {
            scores[i] = KeyBoardHelperState::GREEN;
            remaining[i] = None;
        }
    }
    // Next, letters present in the word but in the wrong position
    for i in 0..WORD_LENGTH {
        if scores[i] == KeyBoardHelperState::GREEN {
            continue;
        }
        if let Some(idx) = remaining.iter().position(|&c| c == Some(guess[i])) {
            scores[i] = KeyBoardHelperState::AMBER;
            remaining[idx] = None;
        }
    }
    scores
}

fn is_valid_shape(guess: &str) -> bool {
    guess.len() == WORD_LENGTH && guess.chars().all(|c| c.is_ascii_lowercase())
}

impl GameData {
    pub fn new(config: &Config) -> GameData {
        let winning_word = if let Some(ref custom_word) = config.word {
            custom_word.to_ascii_lowercase()
        } else {
            words::choose_word().to_string()
        };

        GameData {
            winning_word,
            guess_count: 0,
            max_guesses: config.max_guesses,
            words_guessed: Vec::new(),
            scores: Vec::new(),
            game_state: GameStatus::PLAYING,
            keyboard_helper: [KeyBoardHelperState::NONE; ALPHABET_SIZE],
            hard_mode: config.hard_mode,
            revealed_letters: Vec::new(),
        }
    }

    /// Checks hard mode rules: every GREEN letter must stay in its position
    /// and every revealed letter must appear somewhere in the guess.
    fn check_hard_mode(&self, guess: &str) -> Result<(), String> {
        let guess_chars: Vec<char> = guess.chars().collect();
        for (word, scores) in self.words_guessed.iter().zip(&self.scores) {
            for (i, ch) in word.chars().enumerate() {
                if scores[i] == KeyBoardHelperState::GREEN && guess_chars[i] != ch {
                    return Err(format!(
                        "Hard mode: must use revealed letter '{}' in position {}",
                        ch.to_ascii_uppercase(),
                        i + 1
                    ));
                }
            }
        }
        for &letter in &self.revealed_letters {
            if !guess.contains(letter) {
                return Err(format!(
                    "Hard mode: must use revealed letter '{}'",
                    letter.to_ascii_uppercase()
                ));
            }
        }
        Ok(())
    }

    pub fn new_guess(&mut self, guess: &str) -> Result<GameStatus, String> {
        if self.game_state != GameStatus::PLAYING || self.guess_count >= self.max_guesses {
            return Err("Maximum guesses exceeded".to_string());
        }
        if !is_valid_shape(guess) {
            return Err(format!("Guess must be {} letters", WORD_LENGTH));
        }

        if self.hard_mode {
            self.check_hard_mode(guess)?;
        }

        let scores = score_guess(guess, &self.winning_word);
        self.words_guessed.push(guess.to_string());
        self.scores.push(scores);
        self.guess_count += 1;

        if guess == self.winning_word {
            self.game_state = GameStatus::WON;
        } else if self.guess_count >= self.max_guesses {
            self.game_state = GameStatus::LOST;
        }

        // Update keyboard helper (a key only ever upgrades: NONE < GRAY < AMBER < GREEN)
        // and revealed letters for hard mode
        for (ch, score) in guess.chars().zip(scores) {
            let char_index = ch as usize - ASCII_LOWERCASE_A;
            if score as u8 > self.keyboard_helper[char_index] as u8 {
                self.keyboard_helper[char_index] = score;
            }
            if self.hard_mode
                && score != KeyBoardHelperState::GRAY
                && !self.revealed_letters.contains(&ch)
            {
                self.revealed_letters.push(ch);
            }
        }

        Ok(self.game_state)
    }

    pub fn status(&self) -> GameStatus {
        self.game_state
    }

    pub fn winning_word(&self) -> &str {
        &self.winning_word
    }

    pub fn max_guesses(&self) -> usize {
        self.max_guesses
    }

    pub fn is_hard_mode(&self) -> bool {
        self.hard_mode
    }

    pub fn revealed_letters(&self) -> &[char] {
        &self.revealed_letters
    }

    pub fn guess_count(&self) -> usize {
        self.guess_count
    }

    /// Previous guesses paired with their per-tile scores, oldest first
    pub fn guesses(&self) -> impl Iterator<Item = (&str, &[KeyBoardHelperState; WORD_LENGTH])> {
        self.words_guessed
            .iter()
            .map(String::as_str)
            .zip(&self.scores)
    }

    pub fn keyboard_helper(&self) -> [KeyBoardHelperState; ALPHABET_SIZE] {
        self.keyboard_helper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config(hard_mode: bool, max_guesses: usize, word: Option<String>) -> Config {
        Config {
            hard_mode,
            max_guesses,
            stats: false,
            reset_stats: false,
            word,
            no_color: false,
        }
    }

    #[test]
    fn test_new_game_initialization() {
        let config = create_test_config(false, 6, Some("tests".to_string()));
        let game = GameData::new(&config);

        assert_eq!(game.status(), GameStatus::PLAYING);
        assert_eq!(game.guess_count(), 0);
        assert_eq!(game.winning_word(), "tests");
        assert_eq!(game.max_guesses(), 6);
        assert!(!game.is_hard_mode());
        assert_eq!(game.revealed_letters().len(), 0);
    }

    #[test]
    fn test_winning_guess() {
        let config = create_test_config(false, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        let result = game.new_guess("apple");
        assert!(result.is_ok());
        assert_eq!(game.status(), GameStatus::WON);
        assert_eq!(game.guess_count(), 1);
    }

    #[test]
    fn test_losing_game() {
        let config = create_test_config(false, 3, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("tests").unwrap();
        assert_eq!(game.status(), GameStatus::PLAYING);

        game.new_guess("bread").unwrap();
        assert_eq!(game.status(), GameStatus::PLAYING);

        game.new_guess("lemon").unwrap();
        assert_eq!(game.status(), GameStatus::LOST);
        assert_eq!(game.guess_count(), 3);
    }

    #[test]
    fn test_keyboard_helper_correct_position() {
        let config = create_test_config(false, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("ankle").unwrap();

        let keyboard = game.keyboard_helper();
        let a_index = 'a' as usize - ASCII_LOWERCASE_A;
        assert_eq!(keyboard[a_index], KeyBoardHelperState::GREEN); // 'a' is correct
    }

    #[test]
    fn test_keyboard_helper_wrong_position() {
        let config = create_test_config(false, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("plumb").unwrap();

        let keyboard = game.keyboard_helper();
        let p_index = 'p' as usize - ASCII_LOWERCASE_A;
        assert_eq!(keyboard[p_index], KeyBoardHelperState::AMBER); // 'p' exists but wrong position
    }

    #[test]
    fn test_keyboard_helper_incorrect_letter() {
        let config = create_test_config(false, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("tests").unwrap();

        let keyboard = game.keyboard_helper();
        let t_index = 't' as usize - ASCII_LOWERCASE_A;
        assert_eq!(keyboard[t_index], KeyBoardHelperState::GRAY); // 't' not in word
    }

    #[test]
    fn test_hard_mode_revealed_letters() {
        let config = create_test_config(true, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("ankle").unwrap();

        assert!(game.revealed_letters().contains(&'a'));
        assert!(game.revealed_letters().contains(&'l'));
        assert!(game.revealed_letters().contains(&'e'));
    }

    #[test]
    fn test_hard_mode_constraint_violation() {
        let config = create_test_config(true, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("ankle").unwrap();

        // Try to guess a word without revealed letters
        let result = game.new_guess("birds");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must use revealed letter"));
    }

    #[test]
    fn test_hard_mode_constraint_satisfied() {
        let config = create_test_config(true, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("ankle").unwrap();

        // Keeps greens 'a', 'l', 'e' in place
        let result = game.new_guess("addle");
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_guesses() {
        let config = create_test_config(false, 6, Some("table".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("tests").unwrap();
        assert_eq!(game.guess_count(), 1);
        assert_eq!(game.status(), GameStatus::PLAYING);

        game.new_guess("train").unwrap();
        assert_eq!(game.guess_count(), 2);
        assert_eq!(game.status(), GameStatus::PLAYING);

        game.new_guess("table").unwrap();
        assert_eq!(game.guess_count(), 3);
        assert_eq!(game.status(), GameStatus::WON);
    }

    #[test]
    fn test_max_guesses_customization() {
        let config = create_test_config(false, 2, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("tests").unwrap();
        assert_eq!(game.status(), GameStatus::PLAYING);

        game.new_guess("bread").unwrap();
        assert_eq!(game.status(), GameStatus::LOST);
        assert_eq!(game.guess_count(), 2);
    }

    #[test]
    fn test_keyboard_helper_priority() {
        // Test that GREEN takes priority over AMBER
        let config = create_test_config(false, 6, Some("aback".to_string()));
        let mut game = GameData::new(&config);

        // First guess: 'a' is in wrong position (should be AMBER)
        game.new_guess("beans").unwrap();

        // Second guess: 'a' is in correct position (should be GREEN)
        game.new_guess("abler").unwrap();

        let keyboard = game.keyboard_helper();
        let a_index = 'a' as usize - ASCII_LOWERCASE_A;
        assert_eq!(keyboard[a_index], KeyBoardHelperState::GREEN);
    }

    #[test]
    fn test_hard_mode_green_must_stay_in_place() {
        let config = create_test_config(true, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("ankle").unwrap();

        // Contains 'a', 'l', 'e' but moves the green 'a'
        let result = game.new_guess("table");
        assert!(result.unwrap_err().contains("position 1"));
        assert_eq!(game.guess_count(), 1);
    }

    #[test]
    fn test_score_guess_repeated_letters() {
        use KeyBoardHelperState::*;
        // Only one 'e' in the answer, so only one 'e' is scored
        assert_eq!(
            score_guess("speed", "abide"),
            [GRAY, GRAY, AMBER, GRAY, AMBER]
        );
        // Exact match claims the letter before any AMBER
        assert_eq!(
            score_guess("lolly", "hello"),
            [GRAY, AMBER, GREEN, GREEN, GRAY]
        );
        assert_eq!(score_guess("apple", "apple"), [GREEN; WORD_LENGTH]);
    }

    #[test]
    fn test_invalid_guess_shape_rejected() {
        let config = create_test_config(false, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        assert!(game.new_guess("app").is_err());
        assert!(game.new_guess("APPLE").is_err());
        assert_eq!(game.guess_count(), 0);
    }

    #[test]
    fn test_no_guesses_after_game_over() {
        let config = create_test_config(false, 6, Some("apple".to_string()));
        let mut game = GameData::new(&config);

        game.new_guess("apple").unwrap();
        assert!(game.new_guess("apple").is_err());
        assert_eq!(game.guess_count(), 1);
    }
}
