mod jisho;
extern crate nom;
use nom::{IResult};

use nom::character::is_digit;
use nom::branch::alt;
use nom::multi::many0;
use nom::character::complete::{char, digit1, tab, u32, newline, anychar};
use nom::sequence::delimited;
use crate::nom::bytes::complete::{take_until, is_not};

use crate::jisho::JishoResponse;
use std::fs::read_to_string;
use iced::{
    keyboard, button, text_input, window, Align, Application, Button, Clipboard, Column, Command, Container,
    Element, HorizontalAlignment, Length, Row, Settings, Text, TextInput, Subscription
};

use iced_native::{subscription, Event};

//use std::env;

#[derive(Debug)]
enum Dict {
    Waiting {
        input: text_input::State,
        input_value: String,
        button: button::State,
    },
    Loading,
    Loaded {
        result: JishoResponse,
        button: button::State,
    },
}

// 4707    1282    ムーリエルは２０歳になりました。        Muiriel is 20 now.      は|1 二十歳(はたち){２０歳} になる[01]{になりました}
// 4851    1434    愛してる。      I love you.     愛する{愛してる}
// 4858    1442    ログアウトするんじゃなかったよ。        I shouldn't have logged off.    ログアウト~ 為る(する){する} ん[03] だ{じゃなかった} よ[01]

#[derive(Debug,PartialEq)]
pub struct ExampleSentence {
    japanese_sentence_id: u32,
    english_sentence_id_or_something: u32,
    japanese_text: String,
    english_text: String,
    indices: Vec<Index>
}

fn wwwjdict_parser(input: &str) ->IResult<&str, ExampleSentence> {
    let (input, japanese_sentence_id) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, english_sentence_id_or_something) = u32(input)?;
    let (input, _) = tab(input)?;
    let (input, japanese_text) = take_until("	")(input)?;
    let (input, _) = tab(input)?;
    let (input, english_text) = take_until("	")(input)?;
    let (input, _) = tab(input)?;
    let (input, indices_string) = take_until("\n")(input)?;
    let indices: Vec<&str> = indices_string.split(" ").collect();

    fn is_not_brace(s: &str) -> IResult<&str, &str>  {
        is_not("({[")(s)
    }
    let mut indices_vector = Vec::new();

    for index in &indices {
        let (input, headword) = is_not_brace(index)?;
        let (input, brace) = alt((delimited(char('('),
                                               many0(anychar),
                                               char(')')),
                                 delimited(char('['),
                                           many0(anychar),
                                           char(']')),
                                 delimited(char('{'),
                                           many0(anychar),
                                           char('}'))))(input)?;
        indices_vector.push(Index {
            headword: headword.to_string(),
            form_in_sentence: None,
            sense_number: None,
            good_and_checked: false,
            reading: None,
        });
    }

    
//    println!("INDICES: {:?}\n", indices);
    // index parser:
    // take_until ( or [ or { -> headword
    // if (, take until )     -> reading
    // if [, take until ]     -> sense
    // if {, take until }     -> form_in_sentence
    // if ~                   -> good_and_checked

    
    Ok((input, ExampleSentence {
        japanese_sentence_id,
        english_sentence_id_or_something,
        japanese_text: japanese_text.to_string(),
        english_text: english_text.to_string(),
        indices: indices_vector // "愛する{愛してる}".to_string()
    }))
}

// 彼(かれ)[01]{彼の}
// The fields after the indexing headword ()[]{}~ must be in that order.
#[derive(Debug,PartialEq)]
pub struct Index {
    headword: String,
    reading: Option<String>,
    sense_number: Option<u32>,
    form_in_sentence: Option<String>,
    good_and_checked: bool
}


#[cfg(test)]
mod tests {
    use super::*;
    use nom::{
        error::{ErrorKind, VerboseError, VerboseErrorKind},
        Err as NomErr,
    };

    #[test]
    fn test_scheme() {
        // assert_eq!(wwwjdict_parser("01234    "), Ok(("    ", "01234")));
        // assert_eq!(wwwjdict_parser("01a"), Ok(("a", "01")));
        let mut indexes = Vec::new();
        indexes.push(Index {
            headword: "愛する".to_string(),
            form_in_sentence: Some("愛してる".to_string()),
            reading: None,
            sense_number: None,
            good_and_checked: false
        });
        let example_sentence = ExampleSentence {
            japanese_sentence_id: 4851,
            english_sentence_id_or_something: 1434,
            japanese_text: "愛してる。".to_string(),
            english_text: "I love you.".to_string(),
            // indices: "愛する{愛してる}".to_string(),
            indices: indexes
        };
        assert_eq!(wwwjdict_parser("4851	1434	愛してる。	I love you.	愛する{愛してる}"), Ok(("", example_sentence)));
    }
}


// http://www.edrdg.org/wiki/index.php/Sentence-Dictionary_Linking

// fn identification_code(input: &str) -> Res<&str, (&str, Option<&str>)> {
//     context(
//         "identification code",
//         terminated(
//             digit1,
//             tag("\t"),
//         ),
//     )(input)
// }



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
    EscapeButtonPressed,
    WordFound(Result<JishoResponse, Error>),
    SearchAgainButtonPressed,
}

impl Application for Dict {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {

        // http://www.edrdg.org/wiki/index.php/Sentence-Dictionary_Linking
        let example_sentences = read_to_string("resources/wwwjdic.csv");
        match example_sentences {
            Ok(sentences) => {
                let lines: Vec<&str> =  sentences.lines().collect();
                let first_sentence : Vec<&str> = lines[0].split("\t").collect();
                print!("{:?}", first_sentence );
            }
            Err(e) => { println!("{:?}", e); }
        }


        let dict = Dict::Waiting {
            input: text_input::State::new(),
            input_value: "".to_string(),
            button: button::State::new(),
        };
        (dict, Command::none())
    }


    fn title(&self) -> String {
        String::from("Dict")
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
        match self {
            Dict::Waiting { input_value, .. } => match message {
                Message::InputChanged(value) => {
                    *input_value = value;
                    Command::none()
                }
                Message::SearchButtonPressed => {
                    let query = input_value.clone();
                    *self = Dict::Loading;
                    println!("{}", query);
                    Command::perform(Dict::search(query), Message::WordFound)
                }
                Message::EscapeButtonPressed => {
                    std::process::exit(0);
                }
                _ => Command::none(),
            },
            Dict::Loading { .. } => match message {
                Message::WordFound(Ok(jisho_result)) => {
                    *self = Dict::Loaded {
                        result: jisho_result,
                        button: button::State::new(),
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
            Dict::Loaded { .. } => match message {
                Message::SearchAgainButtonPressed => {
                    *self = Dict::Waiting {
                        input: text_input::State::new(),
                        input_value: "".to_string(),
                        button: button::State::new(),
                    };
                    Command::none()
                }
                Message::EscapeButtonPressed => {
                    std::process::exit(0);
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
            Dict::Loading {} => Column::new()
                .width(Length::Shrink)
                .push(Text::new("Loading...").size(40)),
            Dict::Waiting {
                input,
                input_value,
                button,
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
                            &input_value,
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

            Dict::Loaded { result, button } => {
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

                for i in &result.data {
                    let reading = i.japanese[0].reading.clone();
                    let row = Row::new()
                        .spacing(10)
                        .push(Text::new(&i.slug).size(30).width(Length::Fill))
                        .push(
                            Text::new(reading.unwrap_or_default())
                                .size(30)
                                .width(Length::Fill),
                        )
                        .push(
                            Text::new(&i.senses[0].english_definitions[0])
                                .size(30)
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Left),
                        );
                    column = column.push(row);
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
