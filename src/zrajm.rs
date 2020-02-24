use std::hash::{Hash, Hasher};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, prelude::*, BufReader};
use std::cmp::Ordering;
use regex::Regex;

use crate::klingon::letters;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ZrajmWord {
    pub homonym: i8,
    pub sense: i8,
    pub subsense: i8,
    pub tlh: String,
    pub en: Vec<String>,
    pub sv: Vec<String>,
    pub pos: ZrajmPOS,
    pub fields: HashMap<String, String>,
    pub tag: Vec<String>,
    pub data: Vec<String>,
    pub id: String,
}

impl ZrajmWord {
    pub fn new() -> ZrajmWord {
        ZrajmWord {
            homonym: 1,
            sense: 1,
            subsense: 1,
            tlh: String::new(),
            en: Vec::new(),
            sv: Vec::new(),
            pos: ZrajmPOS::Unknown,
            fields: HashMap::new(),
            tag: Vec::new(),
            data: Vec::new(),
            id: String::new(),
        }
    }

    pub fn en_index(&self) -> Vec<String> {
        get_index_words(&self.en)
    }

    pub fn sv_index(&self) -> Vec<String> {
        get_index_words(&self.sv)
    }
}

impl Hash for ZrajmWord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for ZrajmWord {
    fn partial_cmp(&self, other: &ZrajmWord) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ZrajmWord {
    fn cmp(&self, other: &ZrajmWord) -> Ordering {
        let a = (letters(self.tlh.as_str()), self.homonym, self.sense, self.subsense);
        let b = (letters(other.tlh.as_str()), other.homonym, other.sense, other.subsense);
        a.cmp(&b)
    }
}

fn get_index_words(translations: &Vec<String>) -> Vec<String> {
    let mut ans = Vec::new();
    for translation in translations {
        let mut stack: Vec<String> = Vec::new();
        stack.push(String::new());
        for ch in translation.chars() {
            match ch {
                '<' | '«' => {
                    stack.push(String::new());
                }
                '>' | '»' => {
                    if let Some(word) = stack.pop() {
                        ans.push(word);
                    }
                }
                _ => {
                    for s in &mut stack {
                        s.push(ch);
                    }
                }
            }
        }
        ans.push(stack.pop().unwrap());
    }
    ans
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum ZrajmPOS {
    Adverbial,
    Conjunction,
    Exclamation,
    Name,
    Noun,
    NounSuffix1,
    NounSuffix2,
    NounSuffix3,
    NounSuffix4,
    NounSuffix5,
    Numeral,
    Pronoun,
    QuestionWord,
    Verb,
    VerbPrefix,
    VerbSuffix1,
    VerbSuffix2,
    VerbSuffix3,
    VerbSuffix4,
    VerbSuffix5,
    VerbSuffix6,
    VerbSuffix7,
    VerbSuffix8,
    VerbSuffix9,
    VerbSuffixRover,
    Unknown,
}

impl ZrajmPOS {
    pub fn new(text: &str) -> ZrajmPOS {
        match text {
            "adverbial" => ZrajmPOS::Adverbial,
            "conjunction" => ZrajmPOS::Conjunction,
            "exclamation" => ZrajmPOS::Exclamation,
            "name" => ZrajmPOS::Name,
            "noun" => ZrajmPOS::Noun,
            "noun suffix type 1" => ZrajmPOS::NounSuffix1,
            "noun suffix type 2" => ZrajmPOS::NounSuffix2,
            "noun suffix type 3" => ZrajmPOS::NounSuffix3,
            "noun suffix type 4" => ZrajmPOS::NounSuffix4,
            "noun suffix type 5" => ZrajmPOS::NounSuffix5,
            "numeral" => ZrajmPOS::Numeral,
            "pronoun" => ZrajmPOS::Pronoun,
            "question word" => ZrajmPOS::QuestionWord,
            "verb" => ZrajmPOS::Verb,
            "verb prefix" => ZrajmPOS::VerbPrefix,
            "verb suffix type 1" => ZrajmPOS::VerbSuffix1,
            "verb suffix type 2" => ZrajmPOS::VerbSuffix2,
            "verb suffix type 3" => ZrajmPOS::VerbSuffix3,
            "verb suffix type 4" => ZrajmPOS::VerbSuffix4,
            "verb suffix type 5" => ZrajmPOS::VerbSuffix5,
            "verb suffix type 6" => ZrajmPOS::VerbSuffix6,
            "verb suffix type 7" => ZrajmPOS::VerbSuffix7,
            "verb suffix type 8" => ZrajmPOS::VerbSuffix8,
            "verb suffix type 9" => ZrajmPOS::VerbSuffix9,
            "verb suffix type rover" => ZrajmPOS::VerbSuffixRover,
            _ => ZrajmPOS::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct ZrajmDictionary {
    pub words: Vec<ZrajmWord>,
    pub tlh_index: HashMap<String, HashSet<ZrajmWord>>,
    pub pos_index: HashMap<ZrajmPOS, HashSet<ZrajmWord>>,
    pub en_index: HashMap<String, HashSet<ZrajmWord>>,
    pub sv_index: HashMap<String, HashSet<ZrajmWord>>,
}

impl ZrajmDictionary {
    pub fn new() -> ZrajmDictionary {
        ZrajmDictionary {
            words: Vec::new(),
            tlh_index: HashMap::new(),
            pos_index: HashMap::new(),
            en_index: HashMap::new(),
            sv_index: HashMap::new(),
        }
    }

    fn push_word(&mut self, word: &ZrajmWord) {
        self.words.push(word.clone());

        self.tlh_index.entry(word.tlh.clone()).or_insert(HashSet::new()).insert(word.clone());
        self.pos_index.entry(word.pos).or_insert(HashSet::new()).insert(word.clone());

        for index_word in word.en_index() {
            self.en_index.entry(index_word).or_insert(HashSet::new()).insert(word.clone());
        }

        for index_word in word.sv_index() {
            self.sv_index.entry(index_word).or_insert(HashSet::new()).insert(word.clone());
        }
    }
}

pub fn read_dictionary(filename: &str) -> io::Result<ZrajmDictionary> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    let mut dict = ZrajmDictionary::new();
    let mut data = false;
    let mut word = ZrajmWord::new();
    let mut prev_field = String::new();

    for maybe_line in reader.lines() {
        let line: String = maybe_line?;
        if line == "== start-of-data ==" {
            data = true
        }
        if line == "== end-of-data ==" {
            break
        }
        else if !data || line.starts_with("==") {
            continue
        }
        if line.is_empty() {
            if word.id != "" {
                dict.push_word(&word)
            }
            word = ZrajmWord::new()
        }

        let fields: Vec<_> = line.split("\t").collect();

        if fields.len() < 2 {
            continue
        }

        match fields[0] {
            "tlh:" => parse_tlh(&mut word, &fields),
            "pos:" => word.pos = ZrajmPOS::new(fields[1]),
            "en:" => word.en = fields[1].split(", ").map(|s| s.to_string()).collect(),
            "sv:" => word.sv = fields[1].split(", ").map(|s| s.to_string()).collect(),
            "tag:" => word.tag = fields[1].split("; ").map(|s| s.to_string()).collect(),
            "data:" => word.data = fields[1].split("; ").map(|s| s.to_string()).collect(),
            "id:" => word.id = String::from(fields[1]),
            "" => word.fields.get_mut(&prev_field).unwrap_or(&mut String::new()).push_str(line.as_str()),
            _ => {
                let name = String::from(fields[0]);
                let value = String::from(fields[1]);
                word.fields.insert(name.clone(), value);
                prev_field = name
            }
        }
    }

    Ok(dict)
}

fn parse_tlh(word: &mut ZrajmWord, fields: &Vec<&str>) {
    lazy_static!{
        static ref RE: Regex = Regex::new(r"(?x)
        (\[(?P<homonym>\d+)\]\s)?
        \{(?P<word>.*)\}
        (\s\[(?P<sense>\d+)?(\.(?P<subsense>\d+))?\])?").unwrap();
    }
    if !RE.is_match(fields[1]) {
        eprintln!("{} is not tlh", fields[1]);
        return
    }
    let caps = RE.captures(fields[1]).unwrap();
    
    word.tlh = String::from(&caps["word"]);
    word.homonym = caps.name("homonym").map_or(1, |m| m.as_str().parse::<i8>().unwrap());
    word.sense = caps.name("sense").map_or(1, |m| m.as_str().parse::<i8>().unwrap());
    word.subsense = caps.name("subsense").map_or(1, |m| m.as_str().parse::<i8>().unwrap());
}