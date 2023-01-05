use crate::nom::bytes::complete::{is_not, take_until};
use nom::bytes::complete::take_while;
use nom::character::complete::{char, one_of, tab, u32};
use nom::combinator::eof;
use nom::multi::many_till;
use nom::IResult;

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct ExampleSentence {
    pub japanese_sentence_id: u32,
    pub english_sentence_id: u32,
    pub japanese_text: String,
    pub english_text: String,
    pub indices: Vec<IndexWord>,
}

#[derive(Debug, Eq, PartialEq)]
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
    pub headword: String,
    pub reading: Option<String>,
    pub sense_number: Option<i32>,
    pub form_in_sentence: Option<String>,
    pub good_and_checked: bool,
}

// Parses one index word, including all its index elements
fn parse_index_word(input: &str) -> IResult<&str, IndexWord> {
    let (input, headword) = is_not("([{~| \n")(input)?;

    let (input, (index_elements, _)) = many_till(parse_index_element, one_of(" \n"))(input)?;
    let reading_option: Option<&IndexElement> = index_elements
        .iter()
        .find(|e| matches!(e, IndexElement::Reading(_)));
    let reading: Option<String> = match reading_option {
        Some(IndexElement::Reading(reading)) => Some(reading.to_string()),
        _ => None,
    };

    let sense_option: Option<&IndexElement> = index_elements
        .iter()
        .find(|e| matches!(e, IndexElement::Sense(_)));
    let sense_number: Option<i32> = match sense_option {
        Some(IndexElement::Sense(number)) => Some(*number),
        _ => None,
    };

    let form_option: Option<&IndexElement> = index_elements
        .iter()
        .find(|e| matches!(e, IndexElement::FormInSentence(_)));
    let form_in_sentence: Option<String> = match form_option {
        Some(IndexElement::FormInSentence(form)) => Some(form.to_string()),
        _ => None,
    };

    let good_option: Option<&IndexElement> = index_elements
        .iter()
        .find(|e| matches!(e, IndexElement::GoodAndChecked));
    let good_and_checked: bool = matches!(good_option, Some(IndexElement::GoodAndChecked));

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
    match delimiter_open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '~' => '~',
        '\n' => '\n',
        _ => '_',
    }
}

pub fn wwwjdict_parser(input: &str) -> IResult<&str, ExampleSentence> {
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
            english_sentence_id: english_sentence_id_or_something,
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
        //4851	1434	愛してる。	I love you.	愛する{愛してる}
        let indexes = vec!(IndexWord {
            headword: "愛する".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: Some("愛してる".to_string()),
            good_and_checked: false,
        });

        let example_sentence = ExampleSentence {
            japanese_sentence_id: 4851,
            english_sentence_id: 1434,
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
        let indexes = vec!(IndexWord {
            headword: "総員".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: true,
        }, IndexWord {
            headword: "脱出".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "為る".to_string(),
            reading: Some("する".to_string()),
            sense_number: None,
            form_in_sentence: Some("せよ".to_string()),
            good_and_checked: false,
        });
        let example_sentence = ExampleSentence {
            japanese_sentence_id: 75198,
            english_sentence_id: 328521,
            japanese_text: "総員、脱出せよ！".to_string(),
            english_text: "All hands, abandon ship!".to_string(),
            indices: indexes,
        };

        assert_eq!(
            wwwjdict_parser(
                "75198	328521	総員、脱出せよ！	All hands, abandon ship!	総員~ 脱出 為る(する){せよ}\n"
            ),
            Ok(("", example_sentence))
        );
    }

    #[test]
    fn another_complex_test_scheme() {
        //男の子(おとこのこ)
        let indexes = vec!(IndexWord {
            headword: "男の子".to_string(),
            reading: Some("おとこのこ".to_string()),
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "は".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "結局".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "男の子".to_string(),
            reading: Some("おとこのこ".to_string()),
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "である".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "事".to_string(),
            reading: Some("こと".to_string()),
            sense_number: None,
            form_in_sentence: Some("こと".to_string()),
            good_and_checked: false,
        }, IndexWord {
            headword: "を".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "思い出す".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: Some("思いだした".to_string()),
            good_and_checked: false,
        });
        let example_sentence = ExampleSentence {
            japanese_sentence_id: 127240,
            english_sentence_id: 276849,
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
        let indexes = vec!(IndexWord {
            headword: "北".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "の".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "国".to_string(),
            reading: None,
            sense_number: Some(2),
            form_in_sentence: None,
            good_and_checked: true,
        }, IndexWord {
            headword: "から".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "は".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "北海道".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "市".to_string(),
            reading: Some("し".to_string()),
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "を".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "舞台".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "に".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "為る".to_string(),
            reading: Some("する".to_string()),
            sense_number: None,
            form_in_sentence: Some("した".to_string()),
            good_and_checked: false,
        }, IndexWord {
            headword: "制作".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, 
        // We don't avoid duplicates yet
        IndexWord {
            headword: "の".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        }, IndexWord {
            headword: "テレビドラマ".to_string(),
            reading: None,
            sense_number: None,
            form_in_sentence: None,
            good_and_checked: false,
        });

        let example_sentence = ExampleSentence {
            japanese_sentence_id: 74031,
            english_sentence_id: 329689,
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
