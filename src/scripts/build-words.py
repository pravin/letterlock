"""Builds the word lists in src/:

  src/word-list.txt  every five-letter word accepted as a guess
  src/answers.txt    the common words an answer is picked from

Sources:
  - SCOWL (Spell Checker Oriented Word Lists) by Kevin Atkinson, the lists
    behind Debian's /usr/share/dict/words. English, American and British
    words up to size 60 are accepted as guesses. SCOWL keeps proper nouns in
    separate lists, so its word lists are lowercase dictionary words. See
    src/SCOWL-Copyright.txt.
  - The `wordfreq` package, used only to rank words by how common they are.

An answer must also be a reasonably common word: a Zipf frequency of at least
3.0 in `wordfreq` (roughly once per million words) and SCOWL size 50 or less.
Simple plurals and past tenses ("boats", "baked") are left out, as are the
words in EXCLUDE below. Every answer is still accepted as a guess.

Usage:
    pip install wordfreq
    # download and unpack https://downloads.sourceforge.net/project/wordlist/SCOWL/2020.12.07/scowl-2020.12.07.tar.gz
    python src/scripts/build-words.py path/to/scowl-2020.12.07
"""

import shutil
import sys
from pathlib import Path

from wordfreq import zipf_frequency

LENGTH = 5
MAX_LEVEL_GUESS = 60  # SCOWL size: 60 is the usual spell-checker list
MAX_LEVEL_ANSWER = 50
MIN_ZIPF = 3.0

# Never accepted: words whose main modern use is as a slur
SLURS = {"nigga", "gooks", "squaw", "fagot", "negro"}

# Real words SCOWL only has in its larger, name-heavy sizes
EXTRA_GUESSES = {
    "donut", "snuck", "pinky", "sheik", "caddy", "shorn", "eyrie", "nosey", "melee",
    "maths", "merch", "ramen", "apnea",
}

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
    "hicks", "warts", "upped", "ceded", "shalt", "thine", "sahib", "intel", "wales",
    "women", "fries", "tries", "wives", "stats", "dicks", "cocks", "sluts", "titty",
    "boobs", "farts", "turds", "slave", "nazis", "satan", "dying", "fetus", "obese",
    "milfs",
}

SRC = Path(__file__).resolve().parent.parent


def load_scowl(scowl: Path) -> dict[str, int]:
    """Lowercase five-letter SCOWL words mapped to the smallest size level that has them."""
    levels: dict[str, int] = {}
    for path in (scowl / "final").glob("*-words.*"):
        variety, level = path.name.split("-words.")
        if variety not in ("english", "american", "british") or int(level) > MAX_LEVEL_GUESS:
            continue
        for word in path.read_text(encoding="latin-1").split():
            if len(word) == LENGTH and word.isascii() and word.isalpha() and word.islower():
                levels[word] = min(levels.get(word, 99), int(level))
    return levels


def is_inflection(word: str) -> bool:
    """True for simple plurals and past tenses of common words."""
    common = lambda w: zipf_frequency(w, "en") >= MIN_ZIPF
    if word.endswith("s") and not word.endswith(("ss", "us", "is")):
        if common(word[:-1]) or (word.endswith("es") and common(word[:-2])):
            return True
        if word.endswith("ies") and common(word[:-3] + "y"):
            return True
    if word.endswith("ed") and (common(word[:-1]) or common(word[:-2])):
        return True
    return False


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    scowl = Path(sys.argv[1])
    level = load_scowl(scowl)
    for word in EXTRA_GUESSES:
        level.setdefault(word, MAX_LEVEL_GUESS)

    guesses = sorted(w for w in level if w not in SLURS)
    answers = [
        w
        for w in guesses
        if zipf_frequency(w, "en") >= MIN_ZIPF
        and level[w] <= MAX_LEVEL_ANSWER
        and not is_inflection(w)
        and w not in EXCLUDE
    ]
    (SRC / "word-list.txt").write_text("\n".join(guesses) + "\n")
    (SRC / "answers.txt").write_text("\n".join(answers) + "\n")
    # SCOWL's licence asks for its copyright notice to accompany the lists
    shutil.copy(scowl / "Copyright", SRC / "SCOWL-Copyright.txt")
    print(f"{len(guesses)} allowed guesses, {len(answers)} answers")


if __name__ == "__main__":
    main()
