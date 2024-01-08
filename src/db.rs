use std::fmt::format;
use std::ops::Deref;
use crate::rank_day::RankDay;
use crate::user::User;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use sqlx::{Connection, Error, Executor, SqliteConnection, SqlitePool};
use std::sync::{Arc, Mutex};
use teloxide::types::{ChatId, MessageId};

pub struct Db {
    pool_: SqliteConnection,
}

pub async fn connect(path: String) -> Result<SqliteConnection, Error> {
    SqliteConnection::connect(path.as_str()).await
}

impl Db {
    pub async fn new(path: String) -> Db{
        Db {
            pool_: connect(path).await.unwrap()
        }
        // TODO create table users and rank_days if not exist
    }


    pub async fn add_user(&self, user: User) {
        // TODO add user on table users only if user doesn't exist
    }

    pub async fn get_user_by_chat_id(&mut self, chat_id: ChatId) -> Option<User>{
        // TODO get user form Users table with his chat_id
        let query = "SELECT * FROM users WHERE chat_id == ?";
        let mut rows = sqlx::query(query)
            .bind(chat_id)
            .fetch(&mut self.pool_.deref());

        let mut usr: Option<User> = None;
        while let Some(row) = rows.try_next().await? {
            let cid: ChatId = row.try_get("chat_id")?;
            let name : String = row.try_get("username")?;
            let hour: u8 = row.try_get("hour")?;
            usr = Option::from(User::new(cid, name, Option::from(hour)));
            println!("Find user {} for {} chat id. h = {}", name, cid, hour);
        }
        usr

    }
}

lazy_static! {
    #[derive(Clone)]
    pub static ref USER_LIST: Mutex<Vec<User>> = Mutex::new(Vec::new());
}

lazy_static! {
    #[derive(Clone)]
    static ref RANK_DAY_LIST: Arc<Mutex<Vec<RankDay>>> = Arc::new(Mutex::new(Vec::new()));
}

fn get_rank_day_list() -> RANK_DAY_LIST {
    RANK_DAY_LIST.clone()
}

fn get_user_list() -> USER_LIST {
    USER_LIST.clone()
}

pub async fn get_user_by_chat_id(chat_id: i64) -> Option<User> {
    let user_list = get_user_list();
    let user_list = user_list.lock().unwrap();
    for user in user_list.iter() {
        if user.get_chat_id().0 == chat_id {
            return Some(user.clone());
        }
    }
    None
}

pub async fn get_rank_day_by_id_msg(id_msg: i32) -> Option<RankDay> {
    let rank_day_list = get_rank_day_list();
    let rank_day_list = rank_day_list.lock().unwrap();
    for rank_day in rank_day_list.iter() {
        if rank_day.get_id_msg().0 == id_msg {
            return Some(rank_day.clone());
        }
    }
    None
}

pub async fn get_time(chat_id: ChatId, id_msg: MessageId) -> Option<DateTime<Utc>> {
    let rank_day_list = get_rank_day_list();
    let rank_day_list = rank_day_list.lock().unwrap();
    for rank_day in rank_day_list.iter() {
        if rank_day.get_chat_id() == chat_id && rank_day.get_id_msg() == id_msg {
            return Some(rank_day.get_time());
        }
    }
    None
}

pub async fn add_user(user: User) {
    let mut user_list = get_user_list();
    let mut user_list = user_list.lock().unwrap();
    user_list.push(user);
}

pub async fn set_hour(chat_id: ChatId, hour: u8) {
    let mut user_list = get_user_list();
    let mut user_list = user_list.lock().unwrap();
    for user in user_list.iter_mut() {
        if user.get_chat_id() == chat_id {
            user.set_hour(hour);
        }
    }
}

pub async fn add_rank_day(rank_day: RankDay) {
    let mut rank_day_list = get_rank_day_list();
    let mut rank_day_list = rank_day_list.lock().unwrap();
    rank_day_list.push(rank_day);
}

pub async fn update_rank_in_rank_day_list(id_chat: ChatId, id_msg: MessageId, rank: u8) {
    let mut rank_day_list = get_rank_day_list();
    let mut rank_day_list = rank_day_list.lock().unwrap();
    for rank_day in rank_day_list.iter_mut() {
        if rank_day.get_chat_id() == id_chat && rank_day.get_id_msg() == id_msg {
            rank_day.set_rank(rank);
        }
    }
}

pub async fn clear_rank_in_rank_day_list(id_chat: ChatId, id_msg: MessageId) {
    let mut rank_day_list = get_rank_day_list();
    let mut rank_day_list = rank_day_list.lock().unwrap();
    for rank_day in rank_day_list.iter_mut() {
        if rank_day.get_chat_id() == id_chat && rank_day.get_id_msg() == id_msg {
            rank_day.clear_rank();
        }
    }
}
