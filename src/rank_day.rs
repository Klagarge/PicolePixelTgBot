use crate::user::User;
use chrono::{DateTime, Utc};
use teloxide::types::MessageId;

#[derive(Clone)]
pub struct RankDay {
    user_: User,
    time_: DateTime<Utc>,
    id_msg_: MessageId,
    rank_: Option<u8>,
}

impl RankDay {
    pub fn new(user: User, time: DateTime<Utc>, id_msg: MessageId) -> RankDay {
        RankDay {
            user_: user,
            time_: time,
            id_msg_: id_msg,
            rank_: None,
        }
    }

    pub fn get_rank(&self) -> Option<u8> {
        self.rank_
    }

    pub fn get_user(&self) -> User {
        self.user_.clone()
    }

    pub fn get_time(&self) -> DateTime<Utc> {
        self.time_
    }

    pub fn get_id_msg(&self) -> MessageId {
        self.id_msg_
    }

}
