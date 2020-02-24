#![feature(proc_macro_hygiene, decl_macro)]

extern crate klingon_utils;
use klingon_utils::zrajm::{ZrajmDictionary, read_dictionary};
use klingon_utils::morpho::{Completions, completions};

use std::io::Cursor;

use clap::{App, Arg};

#[macro_use] extern crate rocket;

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
        .get_matches();
    let dict = read_dictionary(matches.value_of("dictionary").unwrap());
    if let Ok(dict) = dict {
        println!("Loaded {} words.", dict.words.len());
        server(dict);
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
fn server_complete(word: String, dict: State<ZrajmDictionary>) -> Json<Completions> {
    Json(completions(&*dict, word.as_str()))
}

