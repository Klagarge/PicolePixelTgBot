use teloxide::prelude::ChatId;

#[derive(Clone)]
pub struct User {
    chat_id_: ChatId,
    username_: String,
    hour_: u8,
}

impl User {
    pub fn new(chat_id: ChatId, username: String, hour: Option<u8>) -> User {
        User {
            chat_id_: chat_id,
            username_: username,
            hour_: hour.unwrap_or(22),
        }
    }

    pub fn get_chat_id(&self) -> ChatId {
        self.chat_id_
    }

    pub fn get_username(&self) -> String {
        self.username_.clone()
    }

    pub fn get_hour(&self) -> u8 {
        self.hour_
    }
}
