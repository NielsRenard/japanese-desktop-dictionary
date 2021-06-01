mod jisho;
use crate::jisho::JishoResponse;
use iced::{
    button, text_input, window, Align, Application, Button, Clipboard, Column, Command, Container,
    Element, Length, Row, Settings, Text, TextInput,
};
use std::env;

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
}

impl Application for Dict {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
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
                    *self = Dict::Loading;
                    return Command::perform(Self::search(), Message::WordFound);
                }
                _ => Command::none(),
            },
            Dict::Loading { .. } => match message {
                Message::WordFound(Ok(jisho_result)) => {
                    *self = Dict::Loaded {
                        result: jisho_result,
                    };
                    return Command::none();
                }
                Message::WordFound(Err(_error)) => {
                    // Do something useful here
                    Command::none()
                }
                _ => Command::none(),
            },
            Dict::Loaded { .. } => match message {
                _ => Command::none(),
            },
        }
    }

    fn view(&mut self) -> Element<Message> {
        let content = match self {
            Dict::Loading {} => Column::new()
                .width(Length::Shrink)
                .push(Text::new("Loading...").size(150)),
            Dict::Waiting {
                input,
                input_value,
                button,
            } => Column::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .align_items(Align::Center)
                .push(
                    Row::new().spacing(10).push(
                        TextInput::new(
                            input,
                            "Type something...",
                            &input_value,
                            Message::InputChanged,
                        )
                        .size(130),
                    ),
                )
                .push(
                    Button::new(button, Text::new("Submit"))
                        .padding(10)
                        .on_press(Message::ButtonPressed),
                )
                .into(),
            Dict::Loaded { result } => Column::new()
                .max_width(500)
                .spacing(20)
                .align_items(Align::End)
                .push(
                    Text::new(&result.data[0].slug)
                        .size(150)
                        .width(Length::Fill),
                ),
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

impl Dict {
    async fn search() -> Result<JishoResponse, Error> {
        // let args: Vec<String> = env::args().collect();
        // let query_string: &str = &args[1..].join(" ");
        let query_string = "家";
        let jisho_base_url = "https://jisho.org/api/v1/search/words?keyword=".to_string();
        let resp: JishoResponse = reqwest::get(jisho_base_url + query_string)
            // &self.input_value)
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

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let args: Vec<String> = env::args().collect();
//     let query_string : &str = &args[1..].join(" ");
//     let jisho_base_url = "https://jisho.org/api/v1/search/words?keyword=".to_string();
//     let resp : JishoResponse = reqwest::get(jisho_base_url + query_string)
//         .await?
//         .json()
//         .await?;

//     println!("{:#?}", resp);
//     Ok(())
// }
