use std::collections::HashMap;

use crate::ai_gateway::input::Input;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Message {
    FunctionReturn {
        role: MessageRole,
        name: String,
        content: String,
    },
    FunctionCall {
        role: MessageRole,
        function_call: FunctionCall,
        content: (),
    },
    // NB: This has to be the last variant as this enum is marked `#[serde(untagged)]`, so
    // deserialization will always try this variant last. Otherwise, it is possible to
    // accidentally deserialize a `FunctionReturn` value as `PlainText`.
    PlainText {
        role: MessageRole,
        content: MessageContent,
    },
}

impl Message {
    pub fn new(input: &Input) -> Self {
        Self {
            role: MessageRole::User,
            content: input.to_message_content(),
            function_call: None,
        }
    }

    pub fn new_text(role: &str, content: &str) -> Self {
        Self::PlainText {
            role: role.to_owned(),
            content: content.to_owned(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self::new_text("system", content)
    }

    pub fn user(content: &str) -> Self {
        Self::new_text("user", content)
    }

    pub fn assistant(content: &str) -> Self {
        Self::new_text("assistant", content)
    }

    pub fn function_call(call: &FunctionCall) -> Self {
        Self::FunctionCall {
            role: MessageRole::Assistant,
            function_call: call.clone(),
            content: (),
        }
    }

    pub fn function_return(name: &str, content: &str) -> Self {
        Self::FunctionReturn {
            role: MessageRole::Function,
            name: name.to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    Assistant,
    User,
    Function,
}

#[allow(dead_code)]
impl MessageRole {
    pub fn is_system(&self) -> bool {
        matches!(self, MessageRole::System)
    }

    pub fn is_user(&self) -> bool {
        matches!(self, MessageRole::User)
    }

    pub fn is_assistant(&self) -> bool {
        matches!(self, MessageRole::Assistant)
    }

    pub fn is_function(&self) -> bool {
        matches!(self, MessageRole::Function)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Array(Vec<MessageContentPart>),
}

impl MessageContent {
    pub fn render_input(&self, resolve_url_fn: impl Fn(&str) -> String) -> String {
        match self {
            MessageContent::Text(text) => text.to_string(),
            MessageContent::Array(list) => {
                let (mut concated_text, mut files) = (String::new(), vec![]);
                for item in list {
                    match item {
                        MessageContentPart::Text { text } => {
                            concated_text = format!("{concated_text} {text}")
                        }
                        MessageContentPart::ImageUrl { image_url } => {
                            files.push(resolve_url_fn(&image_url.url))
                        }
                    }
                }
                if !concated_text.is_empty() {
                    concated_text = format!(" -- {concated_text}")
                }
                format!(".file {}{}", files.join(" "), concated_text)
            }
        }
    }

    pub fn merge_prompt(&mut self, replace_fn: impl Fn(&str) -> String) {
        match self {
            MessageContent::Text(text) => *text = replace_fn(text),
            MessageContent::Array(list) => {
                if list.is_empty() {
                    list.push(MessageContentPart::Text {
                        text: replace_fn(""),
                    })
                } else if let Some(MessageContentPart::Text { text }) = list.get_mut(0) {
                    *text = replace_fn(text)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Choice {
    pub index: usize,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletion {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    // Include other fields you need here
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    pub name: Option<String>,
    pub arguments: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: Parameters,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Parameters {
    #[serde(rename = "type")]
    pub _type: String,
    pub properties: HashMap<String, Parameter>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Parameter {
    #[serde(rename = "type")]
    pub _type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Parameter>>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ExMessage {
    FunctionReturn {
        role: String,
        name: String,
        content: String,
    },
    FunctionCall {
        role: String,
        function_call: FunctionCall,
        content: (),
    },
    // NB: This has to be the last variant as this enum is marked `#[serde(untagged)]`, so
    // deserialization will always try this variant last. Otherwise, it is possible to
    // accidentally deserialize a `FunctionReturn` value as `PlainText`.
    PlainText {
        role: String,
        content: String,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Functions {
    pub functions: Vec<Function>,
}
