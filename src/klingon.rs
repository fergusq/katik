use std::cmp::Ordering;
use regex::Regex;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Letter {
    letter: String,
}

impl Letter {
    fn from(letter: String) -> Letter {
        Letter {
            letter
        }
    }
}

impl ToString for Letter {
    fn to_string(&self) -> String {
        self.letter.clone()
    }
}

impl PartialOrd for Letter {
    fn partial_cmp(&self, other: &Letter) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Letter {
    fn cmp(&self, other: &Letter) -> Ordering {
        if self.letter == "q" && other.letter == "Q" {
            Ordering::Less
        } else if self.letter == "Q" && other.letter == "q" {
            Ordering::Greater
        } else {
            self.letter.to_lowercase().cmp(&other.letter.to_lowercase())
        }
    }
}

pub fn letters(mut word: &str) -> Vec<Letter> {
    lazy_static!{
        // Alfabet: a b ch D e gh H I j l m n ng o p q Q r S t tlh u v w y ʼ
        static ref RE: Regex = Regex::new(r"(ch|gh|ng|tlh|[abDeHIjlmnopqQrStuvwy'])").unwrap();
    }
    let mut ans = Vec::new();
    while let Some(i) = RE.find_at(word, 0) {
        ans.push(Letter::from(word[..i.end()].to_string()));
        word = &word[i.end()..];
    }
    ans
}