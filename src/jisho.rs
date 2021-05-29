use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct JishoResponse {
    meta: Status,
    data: Vec<Entry>,
}
#[derive(Deserialize, Debug)]
pub struct Status {
    status: u32,
}
#[derive(Deserialize, Debug)]
pub struct Entry {
    slug: String,
    is_common: Option<bool>,
    tags: Vec<String>,
    jlpt: Vec<String>,
    japanese: Vec<JapaneseWord>,
    senses: Vec<Sense>,
    attribution: Attribution,
}
#[derive(Deserialize, Debug)]
pub struct JapaneseWord {
    word: Option<String>,
    reading: Option<String>,
}
#[derive(Deserialize, Debug)]
pub struct Attribution {
    jmdict: serde_json::Value,
    jmnedict: serde_json::Value,
    dbpedia: serde_json::Value,
}
#[derive(Deserialize, Debug)]
pub struct Sense {
    english_definitions: Vec<String>,
    parts_of_speech: Vec<String>,
}
