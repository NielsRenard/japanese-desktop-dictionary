use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct JishoResponse {
    pub meta: Status,
    pub data: Vec<Entry>,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Status {
    _status: u32,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Entry {
    pub slug: String,
    _is_common: Option<bool>,
    _tags: Vec<String>,
    _jlpt: Vec<String>,
    pub japanese: Vec<JapaneseWord>,
    pub senses: Vec<Sense>,
    _attribution: Attribution,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct JapaneseWord {
    _word: Option<String>,
    pub reading: Option<String>,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Attribution {
    _jmdict: serde_json::Value,
    _jmnedict: serde_json::Value,
    _dbpedia: serde_json::Value,
}
#[derive(Deserialize, Default, Clone, Debug)]
pub struct Sense {
    pub english_definitions: Vec<String>,
    _parts_of_speech: Vec<String>,
}
