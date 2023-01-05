#![allow(dead_code, unused_mut, unused_variables, unused_imports)]
use iced::alignment::Horizontal;
use iced::widget::{scrollable, slider, Button, Column, Container, Row, Space, Text, TextInput};
use iced::{
    keyboard, window, Alignment, Application, Color, Command, Element, Length, Settings,
    Subscription,
};

use iced_native::widget::{button, text_input};
use std::error::Error;
mod jisho;
use crate::jisho::JishoResponse;
mod example_sentences;
use crate::example_sentences::{wwwjdict_parser, ExampleSentence};
extern crate nom;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::io::prelude::*;

// use iced::{
//     button, keyboard, scrollable, slider, text_input, window, Align, Application, Button,
//     Clipboard, Color, Column, Command, Container, Element, HorizontalAlignment, Length, Row,
//     Scrollable, Settings, Space, Subscription, Text, TextInput,
// };

use iced_aw::{modal, Card, Modal};

use iced_native::{subscription, Event};

#[derive(Debug)]
enum Dict {
    Startup {},
    Waiting {
        input: text_input::State,
        input_value: String,
        button: button::State,
        example_sentences: SentenceMap,
        modal_state: modal::State<ModalState>,
    },
    Loading {
        example_sentences: SentenceMap,
    },
    Loaded {
        button: button::State,
        search_results: Vec<SearchResult>,
        example_sentences: SentenceMap,
    },
    Details {
        back_button: button::State,
        create_flashcard_button: button::State,
        show_english_button: button::State,
        word: String,
        reading: String,
        translations: Vec<String>,
        toggle_show_translation: bool,
        search_results: Vec<SearchResult>,
        example_sentences: SentenceMap,
        modal_state: modal::State<ModalState>,
        slider_state: slider::State,
        text_zoom_value: u16,
    },
}

#[derive(Debug, Default)]
struct ModalState {
    cancel_state: button::State,
    ok_state: button::State,
}

#[derive(Debug, Clone)]
enum Message {
    FoundExampleSentences(Result<String, DictError>),
    InputChanged(String),
    SearchButtonPressed,
    BackButtonPressed,
    DetailsButtonPressed(String, String, Vec<String>),
    CreateFlashcardButtonPressed(ExampleSentence),
    ToggleShowTranslationButtonPressed,
    EscapeButtonPressed,
    QButtonPressed,
    TButtonPressed,
    WordFound(Result<JishoResponse, DictError>),
    SearchAgainButtonPressed,
    TextSizeSliderChanged(u16),
    OpenModal,
    CloseModal,
    CancelButtonPressed,
    OkButtonPressed,
    UndoButtonPressed,
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
                    .horizontal_alignment(Horizontal::Left),
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
    type Theme = iced::Theme;
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

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            Dict::Startup {} => match message {
                Message::FoundExampleSentences(result) => match result {
                    Ok(sentences) => {
                        let sentence_map: SentenceMap = Self::parse_example_sentences(sentences);

                        println!("startup: finished loading sentences!");
                        *self = Dict::Waiting {
                            input: text_input::State::focused(),
                            input_value: "".to_string(),
                            button: button::State::new(),
                            example_sentences: sentence_map,
                            modal_state: modal::State::new(ModalState {
                                cancel_state: button::State::new(),
                                ok_state: button::State::new(),
                            }),
                        };
                        Command::none()
                    }
                    Err(_error) => {
                        // loading/parsing sentences somehow failed
                        // TODO: do something
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
                modal_state,
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
                Message::EscapeButtonPressed => self.update(Message::OpenModal),
                Message::OpenModal => {
                    modal_state.show(true);
                    Command::none()
                }
                Message::CancelButtonPressed | Message::CloseModal => {
                    modal_state.show(false);
                    Command::none()
                }
                Message::OkButtonPressed => {
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
                Message::SearchAgainButtonPressed | Message::EscapeButtonPressed => {
                    let state_swap_example_sentences = std::mem::take(example_sentences);
                    *self = Dict::Waiting {
                        input: text_input::State::focused(),
                        input_value: "".to_string(),
                        button: button::State::new(),
                        example_sentences: state_swap_example_sentences,
                        modal_state: modal::State::new(ModalState {
                            cancel_state: button::State::new(),
                            ok_state: button::State::new(),
                        }),
                    };
                    Command::none()
                }
                Message::DetailsButtonPressed(word, reading, translations) => {
                    *self = Dict::Details {
                        back_button: button::State::new(),
                        show_english_button: button::State::new(),
                        create_flashcard_button: button::State::new(),
                        word,
                        reading,
                        translations,
                        toggle_show_translation: false,
                        search_results: std::mem::take(search_results),
                        example_sentences: std::mem::take(example_sentences),
                        modal_state: modal::State::new(ModalState {
                            cancel_state: button::State::new(),
                            ok_state: button::State::new(),
                        }),
                        slider_state: slider::State::new(),
                        text_zoom_value: 18,
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
                toggle_show_translation,
                modal_state,
                text_zoom_value,
                ..
            } => match message {
                Message::BackButtonPressed | Message::EscapeButtonPressed => {
                    *self = Dict::Loaded {
                        button: button::State::new(),
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
                    self.update(Message::OpenModal)
                }
                Message::ToggleShowTranslationButtonPressed | Message::TButtonPressed => {
                    // let current_state = *toggle_show_translation;
                    *toggle_show_translation = !(*toggle_show_translation);
                    Command::none()
                }
                Message::TextSizeSliderChanged(new_size) => {
                    *text_zoom_value = new_size;
                    Command::none()
                }
                Message::OpenModal => {
                    modal_state.show(true);
                    Command::none()
                }
                Message::CancelButtonPressed | Message::CloseModal => {
                    modal_state.show(false);
                    Command::none()
                }
                Message::OkButtonPressed => self.update(Message::CloseModal),
                Message::UndoButtonPressed => {
                    let _ = delete_last_line_of_csv();
                    modal_state.show(false);
                    Command::none()
                }
                _ => Command::none(),
            },
        }
    }

    fn view(&self) -> Element<Message> {
        return match self {
            Dict::Startup {} => {
                let column = Column::new()
                    .width(Length::Shrink)
                    .push(Text::new("Loading example sentences, just a sec.").size(40));
                Container::new(column)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(30)
                    .into()
            }
            Dict::Loading {
                example_sentences: _,
            } => {
                let column = Column::new()
                    .width(Length::Shrink)
                    .push(Text::new("Loading...").size(40));
                Container::new(column)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(30)
                    .into()
            }
            Dict::Waiting {
                input,
                input_value,
                button,
                example_sentences: _,
                modal_state,
                ..
            } => {
                let column = Column::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Alignment::Start)
                    .padding(10)
                    .spacing(10)
                    .push(Text::new("Search the dictionary:").size(40))
                    .push(
                        Row::new().spacing(10).push(
                            TextInput::new(
                                // input,
                                "Type something...",
                                input_value,
                                Message::InputChanged,
                            )
                            .padding(10)
                            .size(25),
                        ),
                    )
                    .push(
                        Button::new(Text::new("Search").size(20))
                            .padding(10)
                            .on_press(Message::SearchButtonPressed), // .style(style::Button::Primary),
                    );

                // let modal = Modal::new(modal_state, column, |state| {
                let modal = Modal::new(false, column, || {
                    Card::new(
                        Text::new("Exit"),
                        Text::new("Are you sure you want to quit?"),
                    )
                    .foot(
                        Row::new()
                            .spacing(10)
                            .padding(5)
                            .width(Length::Fill)
                            .push(
                                Button::new(
                                    Text::new("Quit").horizontal_alignment(Horizontal::Center),
                                )
                                // .style(style::Button::Primary)
                                .width(Length::Fill)
                                .on_press(Message::OkButtonPressed),
                            )
                            .push(
                                Button::new(
                                    Text::new("Cancel").horizontal_alignment(Horizontal::Center),
                                )
                                // .style(style::Button::Secondary)
                                .width(Length::Fill)
                                .on_press(Message::CancelButtonPressed),
                            ),
                    )
                    .max_width(300)
                    .on_close(Message::CloseModal)
                    .into()
                })
                .backdrop(Message::CloseModal)
                .on_esc(Message::CancelButtonPressed);

                Container::new(modal)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(30)
                    .into()
            }

            Dict::Loaded {
                button,
                search_results,
                example_sentences: _,
            } => {
                let mut content = Column::new()
                    .spacing(5)
                    .align_items(Alignment::Start)
                    .height(Length::Fill)
                    .push(
                        Button::new(Text::new("Search Again").size(25))
                            .padding(10)
                            .on_press(Message::SearchAgainButtonPressed), // .style(style::Button::Secondary),
                    )
                    .push(
                        Text::new(format!("{} results:", &search_results.len()))
                            .size(30)
                            .width(Length::Fill),
                    );

                let row = Row::new()
                    .spacing(10)
                    .push(Space::new(Length::FillPortion(1), Length::Units(1)))
                    .push(
                        Text::new("Word")
                            .size(30)
                            .width(Length::Fill)
                            .style(Color::new(0.67, 0.61, 0.60, 1.0)),
                    )
                    .push(
                        Text::new("Reading")
                            .size(30)
                            .width(Length::Fill)
                            .style(Color::new(0.67, 0.61, 0.60, 1.0)),
                    )
                    .push(
                        Text::new("Translations")
                            .size(30)
                            .width(Length::Fill)
                            .style(Color::new(0.67, 0.61, 0.60, 1.0)),
                    );
                content = content.push(row);

                for i in search_results.iter() {
                    let button = |label: String, message: Message| {
                        Button::new(
                            Text::new(label)
                                .width(Length::FillPortion(1))
                                .horizontal_alignment(Horizontal::Center)
                                .size(16),
                        )
                        .width(Length::FillPortion(1))
                        // .style(style::Button::Primary)
                        .on_press(message)
                        .padding(4)
                    };

                    let row = Row::new()
                        .spacing(10)
                        .push(button(
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
                                .horizontal_alignment(Horizontal::Left),
                        );
                    content = content.push(row);
                }

                let scrollable = scrollable(Container::new(content).width(Length::Fill).center_x());

                Container::new(scrollable)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(30)
                    .into()
            }
            Dict::Details {
                word,
                reading,
                translations,
                example_sentences,
                create_flashcard_button,
                back_button,
                show_english_button,
                toggle_show_translation,
                modal_state,
                slider_state,
                text_zoom_value,
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
                    .align_items(Alignment::Start)
                    .height(Length::Fill)
                    .spacing(10)
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(
                                Button::new(Text::new("Back").size(20))
                                    .padding(10)
                                    .on_press(Message::BackButtonPressed), // .style(style::Button::Secondary),
                            )
                            .push(
                                Button::new(
                                    Text::new("Save to Anki flash card")
                                        .width(Length::Fill)
                                        .horizontal_alignment(Horizontal::Center)
                                        .size(16),
                                )
                                .on_press(Message::CreateFlashcardButtonPressed(shortest_sentence))
                                // .style(style::Button::Primary)
                                .padding(10),
                            )
                            .push(
                                Button::new(
                                    Text::new(if *toggle_show_translation {
                                        "Hide translation"
                                    } else {
                                        "Show translation"
                                    })
                                    .width(Length::Fill)
                                    .horizontal_alignment(Horizontal::Center)
                                    .size(16),
                                )
                                .on_press(Message::ToggleShowTranslationButtonPressed)
                                // .style(if *toggle_show_translation  { style::Button::Secondary } else { style::Button::Primary } )
                                .padding(10),
                            )
                            .push(Text::new("Font size").size(30).width(Length::Shrink))
                            .push(
                                slider(0..=40, *text_zoom_value, Message::TextSizeSliderChanged)
                                    .width(Length::Units(150)),
                            ),
                    )
                    .push(
                        Column::new()
                            .align_items(Alignment::Start)
                            .height(Length::Shrink)
                            .spacing(0)
                            .push(
                                Row::new().push(
                                    Text::new(reading.to_string())
                                        .size(35)
                                        .width(Length::FillPortion(4)),
                                ),
                            )
                            .push(
                                Row::new().push(
                                    Text::new(word.to_string()).size(50).width(Length::Shrink),
                                ),
                            ),
                    )
                    .push(
                        Row::new().push(
                            Text::new(translations.clone().join(" / "))
                                .size(35)
                                .width(Length::FillPortion(1))
                                .horizontal_alignment(Horizontal::Left),
                        ),
                    )
                    .push(Row::new().push(Space::new(Length::Fill, Length::Units(20))))
                    .push(
                        Text::new(format!(
                            "{} sentence(s):",
                            std::cmp::min(sentences.len(), 20_usize)
                        ))
                        .size(30)
                        .width(Length::Fill),
                    );

                for (n, sentence) in sentences.iter().take(20).enumerate() {
                    let japanese_row = Row::new()
                        .spacing(20)
                        .push(
                            Text::new(format!("{}.", n))
                                .size(20 + *text_zoom_value)
                                .width(Length::Shrink),
                        )
                        .push(
                            Text::new(sentence.japanese_text.clone())
                                .size(30 + *text_zoom_value)
                                .width(Length::Fill),
                        );
                    let english_row = Row::new().spacing(20).push(
                        Text::new(sentence.english_text.clone())
                            .size(30)
                            .width(Length::Fill),
                    );
                    let spacing_row = Row::new().push(Space::new(Length::Fill, Length::Units(20)));
                    column = column.push(japanese_row);
                    if *toggle_show_translation {
                        column = column.push(english_row);
                    }
                    column = column.push(spacing_row);
                }

                let scrollable = scrollable(Container::new(column).width(Length::Fill).center_x());

                // let modal = Modal::new(modal_state, scrollable, |state| {
                let modal = Modal::new(false, scrollable, || {
                    Card::new(Text::new("Save Anki flash card"), Text::new("Saved!"))
                        .foot(
                            Row::new()
                                .spacing(10)
                                .padding(5)
                                .width(Length::Fill)
                                .push(
                                    Button::new(
                                        Text::new("Ok").horizontal_alignment(Horizontal::Center),
                                    )
                                    // .style(style::Button::Primary)
                                    .width(Length::Fill)
                                    .on_press(Message::OkButtonPressed),
                                )
                                .push(
                                    Button::new(
                                        Text::new("Undo").horizontal_alignment(Horizontal::Center),
                                    )
                                    // .style(style::Button::Secondary)
                                    .width(Length::Fill)
                                    .on_press(Message::UndoButtonPressed),
                                ),
                        )
                        .max_width(300)
                        .on_close(Message::CloseModal)
                        .into()
                })
                .backdrop(Message::CloseModal)
                .on_esc(Message::CancelButtonPressed);

                Container::new(modal)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(30)
                    .into()
            }
        };
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
        KeyCode::Enter | KeyCode::NumpadEnter => Some(Message::SearchButtonPressed),
        KeyCode::Escape => Some(Message::EscapeButtonPressed),
        KeyCode::Q => Some(Message::QButtonPressed),
        KeyCode::T => Some(Message::TButtonPressed),
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

fn delete_last_line_of_csv() -> Result<(), Box<dyn Error>> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .open("japanese_words_anki_import.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let lines = contents.lines();
    let mut all_but_last: Vec<&str> = lines.rev().skip(1).collect();
    all_but_last.reverse();

    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("japanese_words_anki_import.txt")?;
    let mut buf_writer = std::io::BufWriter::new(file);
    for line in all_but_last {
        writeln!(buf_writer, "{}", line)?;
    }
    Ok(())
}

// mod style {
//     use iced::{Background, Color, Vector, widget::button};

//     pub enum Button {
//         Primary,
//         Secondary,
//     }

//     impl button::StyleSheet for Button {
//         fn active(&self) -> button::Style {
//             button::Style {
//                 background: Some(Background::Color(match self {
//                     Button::Primary => Color::from_rgb(0.11, 0.42, 0.87),
//                     Button::Secondary => Color::from_rgb(0.5, 0.5, 0.5),
//                 })),
//                 border_radius: 12.0,
//                 shadow_offset: Vector::new(1.0, 1.0),
//                 text_color: Color::from_rgb8(0xEE, 0xEE, 0xEE),
//                 ..button::Style::default()
//             }
//         }

//         fn hovered(&self) -> button::Style {
//             button::Style {
//                 text_color: Color::WHITE,
//                 shadow_offset: Vector::new(1.0, 2.0),
//                 ..self.active()
//             }
//         }
//     }
// }
