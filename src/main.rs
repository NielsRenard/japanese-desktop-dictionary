mod jisho;
extern crate nom;
use nom::IResult;

use crate::nom::bytes::complete::{is_not, take_until};
use nom::bytes::complete::take_while;
use nom::character::complete::{char, one_of, tab, u32};
use nom::combinator::eof;
use nom::multi::many_till;
use std::collections::HashMap;

use crate::jisho::JishoResponse;
use iced::{
    button, keyboard, text_input, window, Align, Application, Button, Clipboard, Column, Command,
    Container, Element, HorizontalAlignment, Length, Row, Settings, Space, Subscription, Text,
    TextInput,
};
use std::fs::read_to_string;

use iced_native::{subscription, Event};

#[derive(Debug)]
enum Dict {
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
        result: JishoResponse,
        button: button::State,
        search_results: Vec<SearchResult>,
        example_sentences: SentenceMap,
    },
    Details {
        word: String,
        example_sentences: SentenceMap,
    },
}

#[derive(Debug, Clone)]
struct SearchResult {
    pub details_button: button::State,
    pub japanese: String,
    pub reading: String,
    pub translation: String,
}

type SentenceMap = HashMap<String, Vec<ExampleSentence>>;

impl SearchResult {
    fn new(japanese: String, reading: String, translation: String) -> Self {
        Self {
            details_button: button::State::new(),
            japanese,
            reading,
            translation,
        }
    }
    fn _to_row(&self) -> Row<Message> {
        Row::new()
            .spacing(10)
            .push(Text::new(&self.japanese).size(30).width(Length::Fill))
            .push(Text::new(&self.reading).size(30).width(Length::Fill))
            .push(
                Text::new(&self.translation)
                    .size(30)
                    .width(Length::Fill)
                    .horizontal_alignment(HorizontalAlignment::Left),
            )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExampleSentence {
    japanese_sentence_id: u32,
    english_sentence_id_or_something: u32,
    japanese_text: String,
    english_text: String,
    indices: Vec<IndexWord>,
}

#[derive(Debug, PartialEq)]
pub enum IndexElement {
    Bare,
    Reading(String),        // ()
    Sense(i32),             // []
    FormInSentence(String), // {}
    GoodAndChecked,         // {}
}

// 彼(かれ)[01]{彼の}
// The fields after the indexing headword ()[]{}~ must be in that order.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
pub struct IndexWord {
    headword: String,
    reading: Option<String>,
    sense_number: Option<i32>,
    form_in_sentence: Option<String>,
    good_and_checked: bool,
}

// Parses one index word, including all its index elements
fn parse_index_word(input: &str) -> IResult<&str, IndexWord> {
    let (input, headword) = is_not("([{~| \n")(input)?;

    let (input, (index_elements, _)) = many_till(parse_index_element, one_of(" \n"))(input)?;
    let reading_option: Option<&IndexElement> = index_elements.iter().find(|e| match e {
        IndexElement::Reading(_) => true,
        _ => false,
    });

    let reading: Option<String> = match reading_option {
        Some(IndexElement::Reading(reading)) => Some(reading.to_string()),
        _ => None,
    };

    let sense_option: Option<&IndexElement> = index_elements.iter().find(|e| match e {
        IndexElement::Sense(_) => true,
        _ => false,
    });

    let sense_number: Option<i32> = match sense_option {
        Some(IndexElement::Sense(number)) => Some(*number),
        _ => None,
    };

    let form_option: Option<&IndexElement> = index_elements.iter().find(|e| match e {
        IndexElement::FormInSentence(_) => true,
        _ => false,
    });

    let form_in_sentence: Option<String> = match form_option {
        Some(IndexElement::FormInSentence(form)) => Some(form.to_string()),
        _ => None,
    };

    let good_option: Option<&IndexElement> = index_elements.iter().find(|e| match e {
        IndexElement::GoodAndChecked => true,
        _ => false,
    });

    let good_and_checked: bool = match good_option {
        Some(IndexElement::GoodAndChecked) => true,
        _ => false,
    };

    let index_word = IndexWord {
        headword: headword.to_string(),
        reading,
        sense_number,
        form_in_sentence,
        good_and_checked,
    };
    Ok((input, index_word))
}

// Parses one of the index elements optionally present after an index headword
// delimited by (), [], {},  or ending with a ~.
fn parse_index_element(input: &str) -> IResult<&str, IndexElement> {
    let (input, delimiter) = one_of("([{~| ")(input)?;

    // early exit if char is ~
    if delimiter == '~' {
        return Ok((input, IndexElement::GoodAndChecked));
    }
    if delimiter == '|' {
        // "Some indices are followed by a "|" character and a
        // digit 1 or 2. These are an artefact from a former maintenance
        // system, and can be safely ignored. "
        let (input, _) = one_of("12")(input)?;
        // more dirty input: sometimes there are two spaces after a は|1.
        // if input.chars().take(2).all(|i| i == ' ') {
        //     let (input, _space) = tag(" ")(input)?;
        //     return Ok((input, IndexElement::Bare));
        // };
        return Ok((input, IndexElement::Bare));
    }

    let delimiter_close: char = match_delimiter(delimiter);
    let (input, value) = take_while(|c| c != delimiter_close)(input)?;
    let (input, _delimiter_end) = char(delimiter_close)(input)?;
    let index_element = match delimiter {
        '(' => IndexElement::Reading(value.to_string()),
        '[' => IndexElement::Sense(value.parse::<i32>().unwrap()),
        '{' => IndexElement::FormInSentence(value.to_string()),
        _ => IndexElement::GoodAndChecked, // TODO: make exhaustive by using enum instead of char
    };
    Ok((input, index_element))
}

fn match_delimiter(delimiter_open: char) -> char {
    return match delimiter_open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '~' => '~',
        '\n' => '\n',
        _ => '_',
    };
}

fn wwwjdict_parser(input: &str) -> IResult<&str, ExampleSentence> {
    let (input, japanese_sentence_id) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, english_sentence_id_or_something) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, japanese_text) = take_until("	")(input)?;
    let (input, _) = tab(input)?;
    let (input, english_text) = take_until("	")(input)?;
    let (input, _) = tab(input)?;
    let (input, (indices, _)) = many_till(parse_index_word, eof)(input)?;

    Ok((
        input,
        ExampleSentence {
            japanese_sentence_id,
            english_sentence_id_or_something,
            japanese_text: japanese_text.to_string(),
            english_text: english_text.to_string(),
            indices, // "愛する{愛してる}".to_string()
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    // http://www.edrdg.org/wiki/index.php/Sentence-Dictionary_Linking

    #[test]
    fn test_scheme_basic_index() {
        let mut indexes = Vec::new();
        //4851	1434	愛してる。	I love you.	愛する{愛してる}
        indexes.push(IndexWord {
            headword: "愛する".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: Some("愛してる".to_string()),
            good_and_checked: false,
        });

        let example_sentence = ExampleSentence {
            japanese_sentence_id: 4851,
            english_sentence_id_or_something: 1434,
            japanese_text: "愛してる。".to_string(),
            english_text: "I love you.".to_string(),
            indices: indexes,
        };
        assert_eq!(
            wwwjdict_parser("4851	1434	愛してる。	I love you.	愛する{愛してる}\n"),
            Ok(("", example_sentence))
        );
    }

    #[test]
    fn test_scheme_complex_index() {
        let mut indexes = Vec::new();
        indexes.push(IndexWord {
            headword: "総員".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: true,
        });
        indexes.push(IndexWord {
            headword: "脱出".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "為る".to_string(),
            reading: Some("する".to_string()),
            sense_number: None,
            form_in_sentence: Some("せよ".to_string()),
            good_and_checked: false,
        });
        let example_sentence = ExampleSentence {
            japanese_sentence_id: 75198,
            english_sentence_id_or_something: 328521,
            japanese_text: "総員、脱出せよ！".to_string(),
            english_text: "All hands, abandon ship!".to_string(),
            indices: indexes,
        };

        assert_eq!(
            wwwjdict_parser(
                // "75198	328521	総員、脱出せよ！	All hands, abandon ship!	為る(する){せよ}"
                // "75198	328521	総員、脱出せよ！	All hands, abandon ship!	総員 脱出 為る(する){せよ}"
                "75198	328521	総員、脱出せよ！	All hands, abandon ship!	総員~ 脱出 為る(する){せよ}\n"
            ),
            Ok(("", example_sentence))
        );
    }

    #[test]
    fn another_complex_test_scheme() {
        let mut indexes = Vec::new();
        //男の子(おとこのこ)
        indexes.push(IndexWord {
            headword: "男の子".to_string(),
            reading: Some("おとこのこ".to_string()),
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "は".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "結局".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "男の子".to_string(),
            reading: Some("おとこのこ".to_string()),
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "である".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "事".to_string(),
            reading: Some("こと".to_string()),
            sense_number: None,
            form_in_sentence: Some("こと".to_string()),
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "を".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "思い出す".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: Some("思いだした".to_string()),
            good_and_checked: false,
        });
        let example_sentence = ExampleSentence {
            japanese_sentence_id: 127240,
            english_sentence_id_or_something: 276849,
            japanese_text: "男の子は結局男の子であることを思いだした。".to_string(),
            english_text: "I remembered that boys will be boys.".to_string(),
            indices: indexes,
        };

        assert_eq!(
            wwwjdict_parser("127240	276849	男の子は結局男の子であることを思いだした。	I remembered that boys will be boys.	男の子(おとこのこ) は|1 結局 男の子(おとこのこ) である 事(こと){こと} を 思い出す{思いだした}\n"),
            Ok(("", example_sentence))
        );
    }

    #[test]
    fn test_scheme_complex_index_legacy_pipe_ignore() {
        let mut indexes = Vec::new();
        indexes.push(IndexWord {
            headword: "北".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "の".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "国".to_string(),
            reading: None,
            sense_number: Some(2),
            form_in_sentence: None,
            good_and_checked: true,
        });
        indexes.push(IndexWord {
            headword: "から".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "は".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "北海道".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "市".to_string(),
            reading: Some("し".to_string()),
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "を".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "舞台".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "に".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "為る".to_string(),
            reading: Some("する".to_string()),
            sense_number: None,
            form_in_sentence: Some("した".to_string()),
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "制作".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        // We don't avoid duplicates yet
        indexes.push(IndexWord {
            headword: "の".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });
        indexes.push(IndexWord {
            headword: "テレビドラマ".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });

        let example_sentence = ExampleSentence {
            japanese_sentence_id: 74031,
            english_sentence_id_or_something: 329689,
            japanese_text: "『北の国から』は、北海道富良野市を舞台にしたフジテレビジョン制作のテレビドラマ。".to_string(),
            english_text: "\"From the North Country\" is a TV drama produced by Fuji TV and set in Furano in Hokkaido.".to_string(),
            indices: indexes,
        };

        assert_eq!(
            wwwjdict_parser("74031	329689	『北の国から』は、北海道富良野市を舞台にしたフジテレビジョン制作のテレビドラマ。	\"From the North Country\" is a TV drama produced by Fuji TV and set in Furano in Hokkaido.	北 の 国[02]~ から は|1 北海道 市(し) を 舞台 に 為る(する){した} 制作 の テレビドラマ\n"),
            Ok(("", example_sentence))
        );
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

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    SearchButtonPressed,
    DetailsButtonPressed(String),
    EscapeButtonPressed,
    WordFound(Result<JishoResponse, Error>),
    SearchAgainButtonPressed,
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
        // http://www.edrdg.org/wiki/index.php/Sentence-Dictionary_Linking
        let example_sentences_raw = read_to_string("resources/wwwjdic.csv");
        let example_sentences = match example_sentences_raw {
            Ok(sentences) => {
                // a little pre-processing for dirtiness in the wwwjdict data
                let sentences = sentences.replace("	 ", "	"); // tab + space becomes just tab
                let sentences = sentences.replace(" \n", "\n"); // space + newline becomes just newline
                let sentences = sentences.replace("  ", " "); // two spaces becomes one space
                let lines = LinesWithEndings::from(&sentences);

                let parsed: Vec<ExampleSentence> = lines
                    .into_iter()
                    .map(|line| wwwjdict_parser(line).unwrap().1)
                    .collect();

                let mut words_to_sentences: HashMap<String, Vec<ExampleSentence>> = HashMap::new();
                for sentence in parsed {
                    for index_word in &sentence.indices {
                        words_to_sentences
                            .entry(index_word.headword.to_owned())
                            .or_insert(Vec::new())
                            .push(sentence.to_owned());
                    }
                }
                words_to_sentences
            }
            Err(e) => {
                println!("{:?}", e);
                SentenceMap::new()
            }
        };
        let dict = Dict::Waiting {
            input: text_input::State::new(),
            input_value: "".to_string(),
            button: button::State::new(),
            example_sentences,
        };

        (dict, Command::none())
    }

    fn title(&self) -> String {
        String::from("Dict")
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
        match self {
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
                    let old_example_sentences =
                        std::mem::replace(example_sentences, Default::default());
                    *self = Dict::Loading {
                        example_sentences: old_example_sentences,
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
                        let translation = &i.senses[0].english_definitions[0];
                        search_results.push(SearchResult::new(
                            japanese.clone(),
                            reading.clone(),
                            translation.clone(),
                        ));
                    }
                    let old_example_sentences =
                        std::mem::replace(example_sentences, Default::default());
                    *self = Dict::Loaded {
                        result: jisho_result,
                        button: button::State::new(),
                        search_results,
                        example_sentences: old_example_sentences,
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
                example_sentences, ..
            } => match message {
                Message::SearchAgainButtonPressed => {
                    let old_example_sentences =
                        std::mem::replace(example_sentences, Default::default());
                    *self = Dict::Waiting {
                        input: text_input::State::new(),
                        input_value: "".to_string(),
                        button: button::State::new(),
                        example_sentences: old_example_sentences,
                    };
                    Command::none()
                }
                Message::EscapeButtonPressed => {
                    std::process::exit(0);
                }
                Message::DetailsButtonPressed(word) => {
                    let old_example_sentences =
                        std::mem::replace(example_sentences, Default::default());
                    *self = Dict::Details {
                        word,
                        example_sentences: old_example_sentences,
                    };
                    Command::none()
                }
                _ => Command::none(),
            },
            Dict::Details {
                example_sentences, ..
            } => match message {
                Message::EscapeButtonPressed => {
                    let example_sentences =
                        std::mem::replace(example_sentences, Default::default());
                    *self = Dict::Waiting {
                        input: text_input::State::new(),
                        input_value: "".to_string(),
                        button: button::State::new(),
                        example_sentences,
                    };
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
                result,
                button,
                search_results,
                example_sentences: _,
            } => {
                let mut column = Column::new()
                    .spacing(5)
                    .align_items(Align::Start)
                    .height(Length::Fill)
                    .push(
                        Button::new(button, Text::new("Search Again").size(25))
                            .padding(10)
                            .on_press(Message::SearchAgainButtonPressed),
                    )
                    .push(
                        Text::new(format!("{} results:", &result.data.len()))
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
                            Message::DetailsButtonPressed(i.japanese.clone()),
                        ))
                        .push(Text::new(i.japanese.clone()).size(30).width(Length::Fill))
                        .push(Text::new(i.reading.clone()).size(30).width(Length::Fill))
                        .push(
                            Text::new(i.translation.clone())
                                .size(30)
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Left),
                        );

                    column = column.push(row);
                }

                column
            }
            Dict::Details {
                word,
                example_sentences,
            } => {
                let sentences = match example_sentences.get(word) {
                    Some(sentences) => sentences.to_owned(),
                    None => Vec::new(),
                };
                let mut column = Column::new()
                    .spacing(5)
                    .align_items(Align::Start)
                    .height(Length::Fill)
                    .push(Text::new(format!("{}", word)).size(50).width(Length::Fill));
                for sentence in sentences.iter().take(5) {
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
                column
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
    async fn search(query: String) -> Result<JishoResponse, Error> {
        let jisho_base_url = "https://jisho.org/api/v1/search/words?keyword=".to_string();
        let resp: JishoResponse = reqwest::get(jisho_base_url + &query[..])
            .await?
            .json()
            .await?;
        // println!("{:#?}", resp);
        Ok(resp)
    }
}

#[derive(Debug, Clone)]
enum Error {
    ApiError,
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        dbg!(error);
        Error::ApiError
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
