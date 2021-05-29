mod jisho;
use crate::jisho::JishoResponse;
use std::env;
use iced::{
    button, scrollable, slider, text_input, Align, Button, Checkbox, Column,
    Container, Element, Length, ProgressBar, Radio, Row, Rule, Sandbox,
    Scrollable, Settings, Slider, Space, Text, TextInput,
};


#[derive(Default)]
struct Dict {
    input: text_input::State,
    input_value: String,
    button: button::State,        
}

pub fn main() -> iced::Result {
    Dict::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    ButtonPressed
}

impl Sandbox for Dict {
    type Message = Message;

    fn new() -> Self {
        Dict::default()
    }

    fn title(&self) -> String {
        String::from("Dict")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::InputChanged(value) => self.input_value = value,
            Message::ButtonPressed => {}
        }
    }

   fn view(&mut self) -> Element<Message> {
        let text_input = TextInput::new(
            &mut self.input,
            "Type something...",
            &self.input_value,
            Message::InputChanged,
        )
           .padding(10)
           .size(20);

        let button = Button::new(&mut self.button, Text::new("Submit"))
            .padding(10)
           .on_press(Message::ButtonPressed);

        Column::new()
           .width(Length::Fill)
           .height(Length::Fill)
           .align_items(Align::Center)
           .push(Row::new().spacing(10).push(text_input).push(button))
           .into()
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
