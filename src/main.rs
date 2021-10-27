use std::error::Error;
mod jisho;
use crate::jisho::JishoResponse;
mod example_sentences;
use crate::example_sentences::{wwwjdict_parser, ExampleSentence};
extern crate nom;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;

use iced::{
    button, keyboard, scrollable, text_input, window, Align, Application, Button, Clipboard,
    Column, Command, Container, Element, HorizontalAlignment, Length, Row, Scrollable, Settings,
    Space, Subscription, Text, TextInput,
};

use iced_native::{subscription, Event};

#[derive(Debug)]
enum Dict {
    Startup {},
    Waiting {
        input: text_input::State,
        input_value: String,
        button: button::State,
        example_sentences: SentenceMap,
    },
    Loading {
        example_sentences: SentenceMap,
    },
    Loaded {
        button: button::State,
        scroll: scrollable::State,
        search_results: Vec<SearchResult>,
        example_sentences: SentenceMap,
    },
    Details {
        create_flashcard_button: button::State,
        scroll: scrollable::State,
        word: String,
        reading: String,
        translations: Vec<String>,
        search_results: Vec<SearchResult>,
        example_sentences: SentenceMap,
    },
}

#[derive(Debug, Clone)]
enum Message {
    FoundExampleSentences(Result<String, DictError>),
    InputChanged(String),
    SearchButtonPressed,
    DetailsButtonPressed(String, String, Vec<String>),
    CreateFlashcardButtonPressed(ExampleSentence),
    EscapeButtonPressed,
    WordFound(Result<JishoResponse, DictError>),
    SearchAgainButtonPressed,
}

#[derive(Debug, Clone)]
struct SearchResult {
    pub details_button: button::State,
    pub japanese: String,
    pub reading: String,
    pub translations: Vec<String>,
}

type SentenceMap = HashMap<String, Vec<ExampleSentence>>;

impl SearchResult {
    fn new(japanese: String, reading: String, translations: Vec<String>) -> Self {
        Self {
            details_button: button::State::new(),
            japanese,
            reading,
            translations,
        }
    }
    fn _to_row(&self) -> Row<Message> {
        Row::new()
            .spacing(10)
            .push(Text::new(&self.japanese).size(30).width(Length::Fill))
            .push(Text::new(&self.reading).size(30).width(Length::Fill))
            .push(
                Text::new(&self.translations[0])
                    .size(30)
                    .width(Length::Fill)
                    .horizontal_alignment(HorizontalAlignment::Left),
            )
    }
}

pub fn main() -> iced::Result {
    Dict::run(Settings {
        default_font: Some(include_bytes!("../resources/Meiryo.ttf")),
        window: window::Settings {
            size: (800, 600),
            resizable: true,
            decorations: true,
            ..window::Settings::default()
        },
        antialiasing: true,
        ..Settings::default()
    })
}

// Iterator yielding every line in a string. The line includes newline character(s).
// https://stackoverflow.com/questions/40455997/iterate-over-lines-in-a-string-including-the-newline-characters
#[derive(Debug, Clone)]
pub struct LinesWithEndings<'a> {
    input: &'a str,
}

impl<'a> LinesWithEndings<'a> {
    pub fn from(input: &'a str) -> LinesWithEndings<'a> {
        LinesWithEndings { input }
    }
}

impl<'a> Iterator for LinesWithEndings<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.input.is_empty() {
            return None;
        }
        let split = self
            .input
            .find('\n')
            .map(|i| i + 1)
            .unwrap_or(self.input.len());
        let (line, rest) = self.input.split_at(split);
        self.input = rest;
        Some(line)
    }
}

impl Application for Dict {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Dict::Startup {},
            (Command::perform(
                Dict::load_example_sentences(),
                Message::FoundExampleSentences,
            )),
        )
    }

    fn title(&self) -> String {
        String::from("Dict")
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
        match self {
            Dict::Startup {} => match message {
                Message::FoundExampleSentences(result) => match result {
                    Ok(sentences) => {
                        let sentence_map: SentenceMap = Self::parse_example_sentences(sentences);

                        println!("startup: finished loading sentences!");
                        *self = Dict::Waiting {
                            input: text_input::State::new(),
                            input_value: "".to_string(),
                            button: button::State::new(),
                            example_sentences: sentence_map,
                        };
                        Command::none()
                    }
                    Err(_error) => {
                        // loading/parsing sentences somehow failed
                        // TODO: do something
                        println!("startup: wtf error!");
                        Command::none()
                    }
                },
                _ => {
                    println!("startup: sentences not loaded yet!");
                    Command::none()
                }
            },
            Dict::Waiting {
                input_value,
                example_sentences,
                ..
            } => match message {
                Message::InputChanged(value) => {
                    *input_value = value;
                    Command::none()
                }
                Message::SearchButtonPressed => {
                    let query = input_value.clone();
                    let state_swap_example_sentences = std::mem::take(example_sentences);
                    *self = Dict::Loading {
                        example_sentences: state_swap_example_sentences,
                    };
                    println!("{}", query);
                    Command::perform(Dict::search(query), Message::WordFound)
                }
                Message::EscapeButtonPressed => {
                    std::process::exit(0);
                }
                _ => Command::none(),
            },
            Dict::Loading {
                example_sentences, ..
            } => match message {
                Message::WordFound(Ok(jisho_result)) => {
                    let mut search_results: Vec<SearchResult> = vec![];
                    for i in &jisho_result.data {
                        let japanese = &i.slug;
                        let reading = &i.japanese[0].reading.clone().unwrap_or_default();
                        let translation = &i.senses[0].english_definitions;
                        search_results.push(SearchResult::new(
                            japanese.clone(),
                            reading.clone(),
                            translation.clone(),
                        ));
                    }
                    let state_swap_example_sentences = std::mem::take(example_sentences);
                    *self = Dict::Loaded {
                        button: button::State::new(),
                        scroll: scrollable::State::new(),
                        search_results,
                        example_sentences: state_swap_example_sentences,
                    };
                    Command::none()
                }
                Message::WordFound(Err(_error)) => {
                    // Do something useful here
                    Command::none()
                }
                Message::EscapeButtonPressed => {
                    std::process::exit(0);
                }
                _ => Command::none(),
            },
            Dict::Loaded {
                example_sentences,
                search_results,
                ..
            } => match message {
                Message::SearchAgainButtonPressed => {
                    let state_swap_example_sentences = std::mem::take(example_sentences);
                    *self = Dict::Waiting {
                        input: text_input::State::new(),
                        input_value: "".to_string(),
                        button: button::State::new(),
                        example_sentences: state_swap_example_sentences,
                    };
                    Command::none()
                }
                Message::EscapeButtonPressed => {
                    std::process::exit(0);
                }
                Message::DetailsButtonPressed(word, reading, translations) => {
                    *self = Dict::Details {
                        scroll: scrollable::State::new(),
                        create_flashcard_button: button::State::new(),
                        word,
                        reading,
                        translations,
                        search_results: std::mem::take(search_results),
                        example_sentences: std::mem::take(example_sentences),
                    };
                    Command::none()
                }
                _ => Command::none(),
            },
            Dict::Details {
                example_sentences,
                search_results,
                word,
                reading,
                translations,
                ..
            } => match message {
                Message::EscapeButtonPressed => {
                    *self = Dict::Loaded {
                        button: button::State::new(),
                        scroll: scrollable::State::new(),
                        example_sentences: std::mem::take(example_sentences),
                        search_results: std::mem::take(search_results),
                    };
                    Command::none()
                }
                Message::CreateFlashcardButtonPressed(example_sentence) => {
                    let card = BasicJapaneseFlashcard {
                        vocab: word,
                        vocab_kana: reading,
                        vocab_translation: &translations.join(" / "),
                        part_of_speech: "TODO",
                        sentence: &example_sentence.japanese_text,
                        sentence_translation: &example_sentence.english_text,
                    };
                    let _ = store_word_to_csv(&card);
                    Command::none()
                }
                _ => Command::none(),
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events_with(|event, _status| {
            // this can be used to not handle the event when cursor is inside an input box
            // if let event::Status::Captured = status {
            //     return None;
            // }

            match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    modifiers: _,
                    key_code,
                }) => handle_hotkey(key_code),
                _ => None,
            }
        })
    }

    fn view(&mut self) -> Element<Message> {
        let content = match self {
            Dict::Startup {} => Column::new()
                .width(Length::Shrink)
                .push(Text::new("Loading example sentences, just a sec.").size(40)),
            Dict::Loading {
                example_sentences: _,
            } => Column::new()
                .width(Length::Shrink)
                .push(Text::new("Loading...").size(40)),
            Dict::Waiting {
                input,
                input_value,
                button,
                example_sentences: _,
            } => Column::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .align_items(Align::Start)
                .padding(10)
                .push(Text::new("Search the dictionary:").size(40))
                .push(
                    Row::new().spacing(10).push(
                        TextInput::new(
                            input,
                            "Type something...",
                            input_value,
                            Message::InputChanged,
                        )
                        .padding(10)
                        .size(25),
                    ),
                )
                .push(
                    Button::new(button, Text::new("Search").size(20))
                        .padding(10)
                        .on_press(Message::SearchButtonPressed),
                ),

            Dict::Loaded {
                button,
                scroll,
                search_results,
                example_sentences: _,
            } => {
                let mut content = Column::new()
                    .spacing(5)
                    .align_items(Align::Start)
                    .height(Length::Fill)
                    .push(
                        Button::new(button, Text::new("Search Again").size(25))
                            .padding(10)
                            .on_press(Message::SearchAgainButtonPressed),
                    )
                    .push(
                        Text::new(format!("{} results:", &search_results.len()))
                            .size(30)
                            .width(Length::Fill),
                    );

                for i in search_results.iter_mut() {
                    let button = |state, label: String, message: Message| {
                        Button::new(
                            state,
                            Text::new(label)
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .size(16),
                        )
                        .width(Length::FillPortion(1))
                        .on_press(message)
                        .padding(4)
                    };

                    let row = Row::new()
                        .spacing(10)
                        .push(button(
                            &mut i.details_button,
                            "details".to_string(),
                            Message::DetailsButtonPressed(
                                i.japanese.clone(),
                                i.reading.clone(),
                                i.translations.clone(),
                            ),
                        ))
                        .push(Text::new(i.japanese.clone()).size(30).width(Length::Fill))
                        .push(Text::new(i.reading.clone()).size(30).width(Length::Fill))
                        .push(
                            Text::new(i.translations.clone().join(" / "))
                                .size(30)
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Left),
                        );

                    content = content.push(row);
                }

                let scrollable = Scrollable::new(scroll)
                    .push(Container::new(content).width(Length::Fill).center_x());

                Column::new()
                    .spacing(5)
                    .align_items(Align::Start)
                    .height(Length::Fill)
                    .push(scrollable)
            }
            Dict::Details {
                word,
                reading,
                translations,
                example_sentences,
                create_flashcard_button,
                scroll,
                ..
            } => {
                let sentences = match example_sentences.get(word) {
                    Some(sentences) => sentences.to_owned(),
                    None => Vec::new(),
                };

                let maybe_shortest_sentence: Option<_> = sentences
                    .iter()
                    .min_by(|s1, s2| (s1.english_text.len().cmp(&s2.english_text.len())));

                let shortest_sentence: ExampleSentence =
                    if let Some(sentence) = maybe_shortest_sentence {
                        sentence.clone()
                    } else {
                        ExampleSentence::default()
                    };

                let mut column = Column::new()
                    .align_items(Align::Start)
                    .height(Length::Fill)
                    .push(
                        Row::new().push(
                            Text::new(reading.to_string())
                                .size(35)
                                .width(Length::FillPortion(4)),
                        ),
                    )
                    .push(
                        Row::new().push(Text::new(word.to_string()).size(50).width(Length::Shrink)),
                    )
                    .push(
                        Row::new().push(
                            Text::new(translations.clone().join(" / "))
                                .size(35)
                                .width(Length::FillPortion(1))
                                .horizontal_alignment(HorizontalAlignment::Left),
                        ),
                    )
                    .push(
                        Button::new(
                            create_flashcard_button,
                            Text::new("Save to Anki flash card")
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .size(16),
                        )
                        .on_press(Message::CreateFlashcardButtonPressed(shortest_sentence)),
                    )
                    .push(Row::new().push(Space::new(Length::Fill, Length::Units(20))))
                    .push(
                        Text::new(format!(
                            "{} sentences:",
                            std::cmp::min(sentences.len(), 20 as usize)
                        ))
                        .size(30)
                        .width(Length::Fill),
                    );

                for sentence in sentences.iter().take(20) {
                    let japanese_row = Row::new().spacing(20).push(
                        Text::new(&sentence.japanese_text)
                            .size(30)
                            .width(Length::Fill),
                    );
                    let english_row = Row::new().spacing(20).push(
                        Text::new(&sentence.english_text)
                            .size(30)
                            .width(Length::Fill),
                    );
                    let spacing_row = Row::new().push(Space::new(Length::Fill, Length::Units(20)));

                    column = column.push(japanese_row);
                    column = column.push(english_row);
                    column = column.push(spacing_row);
                }
                let scrollable = Scrollable::new(scroll)
                    .push(Container::new(column).width(Length::Fill).center_x());

                Column::new()
                    .spacing(5)
                    .align_items(Align::Start)
                    .height(Length::Fill)
                    .push(scrollable)
            }
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(30)
            .into()
    }
}

impl Dict {
    async fn search(query: String) -> Result<JishoResponse, DictError> {
        let jisho_base_url = "https://jisho.org/api/v1/search/words?keyword=".to_string();
        let resp: JishoResponse = reqwest::get(jisho_base_url + &query[..])
            .await?
            .json()
            .await?;
        // println!("{:#?}", resp);
        Ok(resp)
    }

    async fn load_example_sentences() -> Result<String, DictError> {
        // let mut file = File::open("resources/wwwjdic.csv").await?;
        // let mut buffer = String::new();
        // file.read_to_string(&mut buffer).await?;
        // Ok(buffer)
        use async_std::prelude::*;
        let mut contents = String::new();

        let mut file = async_std::fs::File::open("resources/wwwjdic.csv")
            .await
            .map_err(|_| DictError::FileNotFound)?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|_| DictError::ReadFile)?;

        Ok(contents)
    }

    fn parse_example_sentences(sentences: String) -> SentenceMap {
        // http://www.edrdg.org/wiki/index.php/Sentence-Dictionary_Linking
        // a little pre-processing for dirtiness in the wwwjdict data
        let sentences = sentences.replace("	 ", "	"); // tab + space becomes just tab
        let sentences = sentences.replace(" \n", "\n"); // space + newline becomes just newline
        let sentences = sentences.replace("  ", " "); // two spaces becomes one space
        let sentences = sentences + &"\n".to_string(); // add newline to keep parser simple (kind of hacky)
        let lines: Vec<_> = LinesWithEndings::from(&sentences).collect();
        println!("start parsing wwwjdict example sentences...");
        let start_parsing = std::time::SystemTime::now();
        let parsed: Vec<ExampleSentence> = lines
            .into_par_iter()
            .map(|line| wwwjdict_parser(line).unwrap().1)
            .collect();
        println!(
            "parsed {} example sentences in: {} milliseconds",
            parsed.len(),
            start_parsing.elapsed().unwrap().as_millis()
        );
        let start_indexing = std::time::SystemTime::now();
        let mut words_to_sentences: HashMap<String, Vec<ExampleSentence>> = HashMap::new();
        for sentence in parsed {
            for index_word in &sentence.indices {
                words_to_sentences
                    .entry(index_word.headword.to_owned())
                    .or_insert_with(Vec::new)
                    .push(sentence.to_owned());
            }
        }
        println!(
            "indexing example sentences took: {} milliseconds",
            start_indexing.elapsed().unwrap().as_millis()
        );
        words_to_sentences
    }
}

#[derive(Debug, Clone)]
enum DictError {
    SearchApi,
    FileNotFound,
    ReadFile,
}

impl From<reqwest::Error> for DictError {
    fn from(error: reqwest::Error) -> DictError {
        dbg!(error);
        DictError::SearchApi
    }
}

fn handle_hotkey(key_code: keyboard::KeyCode) -> Option<Message> {
    use keyboard::KeyCode;

    match key_code {
        KeyCode::Enter => Some(Message::SearchButtonPressed),
        KeyCode::Escape => Some(Message::EscapeButtonPressed),
        _ => None,
    }
}

#[derive(Serialize)]
struct BasicJapaneseFlashcard<'a> {
    vocab: &'a str,
    vocab_kana: &'a str,
    vocab_translation: &'a str,
    part_of_speech: &'a str,
    sentence: &'a str,
    sentence_translation: &'a str,
}

fn store_word_to_csv<T: Serialize>(card: &T) -> Result<(), Box<dyn Error>> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("japanese_words_anki_import.txt")
        .unwrap();
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);
    wtr.serialize(card)?;
    Ok(())
}
