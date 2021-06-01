mod jisho;
use crate::jisho::JishoResponse;
use iced::{
    button, text_input, window, Align, Application, Button, Clipboard, Column, Command, Container,
    Element, HorizontalAlignment, Length, Row, Settings, Text, TextInput,
};
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
    ButtonPressed,
    WordFound(Result<JishoResponse, Error>),
    SearchAgainButtonPressed,
}

impl Application for Dict {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        // let args: Vec<String> = env::args().collect();
        // let query_string: &str = &args[1..].join(" ");

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
                Message::ButtonPressed => {
                    let query = input_value.clone();
                    *self = Dict::Loading;
                    println!("{}", query);
                    Command::perform(Dict::search(query), Message::WordFound)
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
                _ => Command::none(),
            },
        }
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
                        .on_press(Message::ButtonPressed),
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
        println!("{:#?}", resp);
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
