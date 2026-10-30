"""Builds src/answers.txt (possible answers) from src/word-list.txt (allowed guesses).

An answer must be a reasonably common English word, i.e. have a Zipf frequency
of at least 3.0 in the `wordfreq` package (roughly once per million words). Simple
plurals and past tenses ("boats", "baked") are left out, as are the words in
EXCLUDE below. Every answer is still accepted as a guess.

Usage:
    pip install wordfreq
    python src/scripts/build-answers.py
"""

from pathlib import Path

from wordfreq import zipf_frequency

MIN_ZIPF = 3.0

# Remove entirely: words whose main modern use is as a slur
SLURS = {"nigga", "gooks", "squaw", "fagot"}

# Valid guesses, but not answers: crude or sensitive words, names and proper
# nouns, slang contractions, prefixes, leftover plurals/inflections, archaic words
EXCLUDE = SLURS | {
    "bitch", "boner", "booty", "butch", "chink", "coons", "dykes", "fanny", "fecal",
    "feces", "gypsy", "horny", "idiot", "jihad", "kinky", "lynch", "moron", "paddy",
    "penis", "porno", "prick", "pubic", "pussy", "queer", "semen", "sissy", "spank",
    "sperm", "welch", "welsh", "whore", "mucus", "urine", "vomit", "bowel", "groin",
    "thong", "bible", "calif", "chile", "china", "japan", "maria", "mecca", "peter",
    "roger", "roman", "harry", "billy", "bobby", "jimmy", "terry", "sally", "homer",
    "hogan", "sears", "trump", "yahoo", "randy", "taser", "dunno", "gonna", "gotta",
    "kinda", "lemme", "sorta", "wanna", "gimme", "mamma", "momma", "multi", "inter",
    "cries", "dries", "flies", "spies", "fumes", "menus", "taxis", "semis", "tufts",
    "hicks", "warts", "upped", "ceded", "shalt", "thine", "sahib",
}

SRC = Path(__file__).resolve().parent.parent


def is_inflection(word: str) -> bool:
    """True for simple plurals and past tenses of common words."""
    common = lambda w: zipf_frequency(w, "en") >= MIN_ZIPF
    if word.endswith("s") and not word.endswith(("ss", "us", "is")):
        if common(word[:-1]) or (word.endswith("es") and common(word[:-2])):
            return True
    if word.endswith("ed") and (common(word[:-1]) or common(word[:-2])):
        return True
    return False


def main() -> None:
    guess_file = SRC / "word-list.txt"
    guesses = [w.strip() for w in guess_file.read_text().splitlines() if w.strip()]
    guesses = sorted(w for w in guesses if w not in SLURS)
    guess_file.write_text("\n".join(guesses) + "\n")

    answers = [
        w
        for w in guesses
        if zipf_frequency(w, "en") >= MIN_ZIPF and not is_inflection(w) and w not in EXCLUDE
    ]
    (SRC / "answers.txt").write_text("\n".join(answers) + "\n")
    print(f"{len(guesses)} allowed guesses, {len(answers)} answers")


if __name__ == "__main__":
    main()
