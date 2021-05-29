use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
struct JishoResponse {
    meta: Status,
    data: Vec<Entry>,
}
#[derive(Deserialize, Debug)]
struct Status {
    status: u32,
}
#[derive(Deserialize, Debug)]
struct Entry {
    slug: String,
    is_common: Option<bool>,
    tags: Vec<String>,
    jlpt: Vec<String>,
    japanese: Vec<JapaneseWord>,
    senses: Vec<Sense>,
    attribution: Attribution,
}
#[derive(Deserialize, Debug)]
struct JapaneseWord {
    word: Option<String>,
    reading: Option<String>,
}
#[derive(Deserialize, Debug)]
struct Attribution {
    jmdict: serde_json::Value,
    jmnedict: serde_json::Value,
    dbpedia: serde_json::Value,
}
#[derive(Deserialize, Debug)]
struct Sense {
    english_definitions: Vec<String>,
    parts_of_speech: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let query_string : &str = &args[1..].join(" ");
    let jisho_base_url = "https://jisho.org/api/v1/search/words?keyword=".to_string();
    let resp : JishoResponse = reqwest::get(jisho_base_url + query_string)
        .await?
        .json()
        .await?;

    println!("{:#?}", resp);
    Ok(())
}
