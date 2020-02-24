#![feature(proc_macro_hygiene, decl_macro)]

mod zrajm;
use zrajm::*;

mod klingon;

extern crate regex;
#[macro_use] extern crate lazy_static;

use std::cmp::Ordering;
use std::collections::{HashSet, BTreeSet};
use std::io::Cursor;

use cursive::Cursive;
use cursive::traits::*;
use cursive::views::{Dialog, LinearLayout, TextContent, TextView, EditView};
use cursive::utils::markup::StyledString;
use cursive::theme::{Style, Effect};

use enumset::EnumSet;

use clap::{App, Arg};

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;

use rocket::State;
use rocket::response::{Responder, Response};
use rocket::http::{ContentType, Status};
use rocket_contrib::json::Json;

fn main() {
    let matches = App::new("Katik")
        .version("0.0.1")
        .author("Iikka Hauhio <iikka.hauhio@helsinki.fi>")
        .about("Smart Klingon dictionary")
        .arg(Arg::with_name("dictionary")
            .help("Zrajm Dictionary file path")
            .required(true)
        )
        .arg(Arg::with_name("server")
            .help("Create a HTTP server instead of TUI interface")
            .short("-s")
            .long("server")
        )
        .get_matches();
    let dict = read_dictionary(matches.value_of("dictionary").unwrap());
    if let Ok(dict) = dict {
        println!("Loaded {} words.", dict.words.len());
        if matches.is_present("server") {
            server(dict);
        } else {
            tui(dict);
        }
    } else {
        eprintln!("Failed to load dictionary.");
    }
}

fn server(dict: ZrajmDictionary) {
    rocket::ignite()
        .manage(dict)
        .mount("/", routes![server_root, server_complete]).launch();
}

#[get("/")]
fn server_root() -> impl Responder<'static> {
    Response::build()
        .status(Status::Ok)
        .header(ContentType::HTML)
        .sized_body(Cursor::new(include_bytes!("../static/index.html") as &[u8]))
        .finalize()
}

#[get("/complete/<word>")]
fn server_complete(word: String, dict: State<ZrajmDictionary>) -> Json<CompleteResponse> {
    Json(completion_json(&*dict, word.as_str()))
}

#[derive(Serialize)]
struct CompleteResponse {
    parsed: HashSet<Vec<BTreeSet<ZrajmWord>>>,
    suggestions: Vec<ZrajmWord>,
}

fn completion_json(dict: &ZrajmDictionary, word: &str) -> CompleteResponse {
    let grammar = grammar();
    let mut parsed_words = HashSet::new();
    let mut suggestion_words = HashSet::new();
    for mut g in grammar {
        let (ending, parsed, suggestions) = complete(&dict, &mut g.1, word);
        //println!(" {} {}", ending, suggestions.len());
        if !parsed.is_empty() && (ending.is_empty() || !suggestions.is_empty()) {
            parsed_words.insert(parsed);
        }
        if ending.is_empty() {
            continue
        }
        suggestion_words.extend(suggestions)
    }
    let mut suggestions = suggestion_words.into_iter().collect::<Vec<_>>();
    suggestions.sort_by(|a, b| {
        let asw = a.tlh.starts_with(word);
        let bsw = b.tlh.starts_with(word);
        if asw && !bsw {
            Ordering::Less
        } else if !asw && bsw {
            Ordering::Greater
        } else {
            a.cmp(b)
        }
    });
    CompleteResponse {
        parsed: parsed_words,
        suggestions,
    }
}

fn tui(dict: ZrajmDictionary) {
    let mut siv = Cursive::default();

    let compl = TextContent::new("");
    let compl_view = TextView::new_with_content(compl.clone()).fixed_width(50);
    let input_view = EditView::new()
        .on_edit(move|_, text: &str, _| {
            let text = text.to_string();
            if text.contains("!") {
                compl.set_content(translation_string(&dict, &text[text.match_indices("!").last().unwrap().0+1..]))
            } else {
                let words: Vec<&str> = text.split(" ").collect();
                let last_word = words.last().unwrap_or(&"");
                compl.set_content(completion_string(&dict, last_word))
            }
        })
        .fixed_width(50);

    siv.add_layer(Dialog::around(LinearLayout::vertical().child(input_view).child(compl_view)));

    siv.run();
}

fn translation_string(dict: &ZrajmDictionary, word: &str) -> StyledString {
    let mut ans = StyledString::new();
    let mut suggestions = dict.en_index.get(&word.to_string()).map_or(Vec::new(), |a| a.iter().collect());
    suggestions.sort_by(|a, b| a.tlh.cmp(&b.tlh));
    for s in suggestions.iter().take(20) {
        ans.append_plain(format!("{} - {:?}: {}\n", s.tlh, s.pos, s.en.join(", ")).as_str());
    }
    ans
}

fn completion_string(dict: &ZrajmDictionary, word: &str) -> StyledString {
    let grammar = grammar();
    let mut ans = StyledString::new();
    let mut already_listed = HashSet::new();
    for mut g in grammar {
        let (ending, _parsed, suggestions) = complete(&dict, &mut g.1, word);
        //println!(" {} {}", ending, suggestions.len());
        if ending.is_empty() {
            continue
        }
        let suggestions = suggestions.difference(&already_listed).cloned().collect::<HashSet<_>>();
        already_listed.extend(suggestions.clone());
        let mut suggestions = suggestions.iter().collect::<Vec<_>>();
        suggestions.sort_by(|a, b| a.tlh.cmp(&b.tlh));
        if !suggestions.is_empty() {
            ans.append_styled(format!("{}:\n", g.0).as_str(), Style { effects: EnumSet::from(Effect::Bold), color: Option::None });
        }
        for s in suggestions.iter().take(10) {
            ans.append_plain(format!("{} - {:?}: {}\n", s.tlh, s.pos, s.en.join(", ")).as_str())
        }
    }
    ans
}

fn complete(dict: &ZrajmDictionary, grammar: &mut Vec<(ZrajmPOS, bool)>, word: &str) -> (String, Vec<BTreeSet<ZrajmWord>>, HashSet<ZrajmWord>) {
    let (ending, parsed, poses) = complete_pos(dict, Vec::new(), grammar, word);
    //println!("{:?} {:?}", ending, poses.len());
    let mut ans_parsed = Vec::new();
    for (word, pos) in parsed {
        ans_parsed.push(dict.pos_index.get(&pos).unwrap_or(&HashSet::new()).iter().filter(|w| w.tlh == word).cloned().collect());
    }
    let mut ans_words = HashSet::new();
    for pos in poses {
        for dword in dict.pos_index.get(&pos.0).map_or(Vec::new(), |a| a.iter().collect()) {
            if dword.tlh.starts_with(ending.as_str()) {
                ans_words.insert(dword.clone());
            }
        }
        if pos.1 {
            break
        }
    }
    (ending, ans_parsed, ans_words)
}

fn complete_pos(dict: &ZrajmDictionary, mut parsed: Vec<(String, ZrajmPOS)>, grammar: &mut Vec<(ZrajmPOS, bool)>, word: &str) -> (String, Vec<(String, ZrajmPOS)>, Vec<(ZrajmPOS, bool)>) {
    if grammar.is_empty() {
        return (String::from(word), parsed, grammar.clone())
    }
    let mut dwords = Vec::new();
    if let Some(index_words) = dict.pos_index.get(&grammar[0].0) {
        dwords.extend(index_words);
    }
    dwords.sort_by(|a, b| b.tlh.len().cmp(&a.tlh.len()));
    for dword in dwords {
        let dword_ending = dword.tlh.trim_end_matches("-");
        // Valitaan ahneesti
        if word == dword_ending {
            return (String::from(word), parsed, grammar.clone())
        }
        if word.starts_with(dword_ending) {
            let t = if dword.tlh.ends_with("-") {
                ""
            } else {
                "-"
            };
            let ending = format!("{}{}", t, &word[dword_ending.len()..]);
            let pos = grammar.remove(0);
            parsed.push((dword.tlh.clone(), pos.0));
            return complete_pos(dict, parsed, grammar, &ending)
        }
    }
    if grammar[0].1 {
        return (String::from(word), parsed, grammar.clone())
    }
    let mut new_grammar = grammar.clone();
    new_grammar.remove(0);
    let (word2, parsed2, grammar2) = complete_pos(dict, parsed.clone(), &mut new_grammar, word);
    if word == word2 {
        (String::from(word), parsed, grammar.clone())
    } else {
        (word2, parsed2, grammar2)
    }
}

fn grammar() -> Vec<(&'static str, Vec<(ZrajmPOS, bool)>)> {
    vec![
        ("Noun track", vec![
            (ZrajmPOS::Noun, true),
            (ZrajmPOS::NounSuffix1, false),
            (ZrajmPOS::NounSuffix2, false),
            (ZrajmPOS::NounSuffix3, false),
            (ZrajmPOS::NounSuffix4, false),
            (ZrajmPOS::NounSuffix5, false),
        ]),
        ("Verb track", vec![
            (ZrajmPOS::VerbPrefix, false),
            (ZrajmPOS::Verb, true),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix1, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix2, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix3, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix4, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix5, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix6, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix7, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix8, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix9, false),
            (ZrajmPOS::VerbSuffixRover, false),
        ]),
        ("Nominalized verb track", vec![
            (ZrajmPOS::Verb, true),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix1, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix2, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix3, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix4, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix5, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix6, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix7, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix8, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix9, true),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::NounSuffix1, false),
            (ZrajmPOS::NounSuffix2, false),
            (ZrajmPOS::NounSuffix3, false),
            (ZrajmPOS::NounSuffix4, false),
            (ZrajmPOS::NounSuffix5, false),
        ]),
        ("Adjective track", vec![
            (ZrajmPOS::Verb, true),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::NounSuffix5, false),
        ]),
        ("Pronoun track (verb)", vec![
            (ZrajmPOS::Pronoun, true),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix1, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix2, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix3, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix4, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix5, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix6, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix7, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix8, false),
            (ZrajmPOS::VerbSuffixRover, false),
            (ZrajmPOS::VerbSuffix9, true),
            (ZrajmPOS::VerbSuffixRover, false),
        ]),
        ("Pronoun track (noun)", vec![
            (ZrajmPOS::Pronoun, true),
            (ZrajmPOS::NounSuffix5, false),
        ]),
        ("Numerals", vec![(ZrajmPOS::Numeral, true)]),
        ("Adverbials", vec![(ZrajmPOS::Adverbial, true)]),
        ("Conjunctions", vec![(ZrajmPOS::Conjunction, true)]),
        ("Question words", vec![(ZrajmPOS::QuestionWord, true)]),
    ]
}