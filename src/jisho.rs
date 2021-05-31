use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct JishoResponse {
    pub meta: Status,
    pub data: Vec<Entry>,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Status {
    status: u32,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Entry {
    pub slug: String,
    is_common: Option<bool>,
    tags: Vec<String>,
    jlpt: Vec<String>,
    japanese: Vec<JapaneseWord>,
    senses: Vec<Sense>,
    attribution: Attribution,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct JapaneseWord {
    word: Option<String>,
    reading: Option<String>,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Attribution {
    jmdict: serde_json::Value,
    jmnedict: serde_json::Value,
    dbpedia: serde_json::Value,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Sense {
    english_definitions: Vec<String>,
    parts_of_speech: Vec<String>,
}
