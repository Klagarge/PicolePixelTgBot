use std::cell::RefCell;
use crate::rank_day::RankDay;
use crate::user::User;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use sqlx::sqlite::{SqliteQueryResult, SqliteRow};
use sqlx::{Connection, Error, Executor, Row, SqliteConnection, SqlitePool, Statement};
use std::fmt::format;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use teloxide::types::{ChatId, MessageId};
use tokio::runtime::Runtime;


pub struct Database {
    pub(crate) connection_: Rc<RefCell<SqliteConnection>>,
}

impl Database {
    pub fn new(path: String) -> Database {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut conn = SqliteConnection::connect(path.as_str()).await.unwrap();
            conn.execute(
                "CREATE TABLE IF NOT EXISTS User (\
                        id INTEGER CONSTRAINT user_pk PRIMARY KEY AUTOINCREMENT,\
                        chat_id INTEGER(8) NOT NULL CONSTRAINT user_chat_id UNIQUE,\
                        username TEXT NOT NULL,\
                        hour INTEGER(1) NOT NULL DEFAULT 22)"
            ).await.unwrap();
            Database {
                connection_: Rc::new(RefCell::new(conn)),
            }
        })
    }

    pub async fn add_user(&self, user: User) {
        let mut conn = self.connection_.borrow_mut();
        let conn = &mut *conn;

        let stmt = conn
            .prepare("SELECT id, username FROM User WHERE chat_id = ?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(user.get_chat_id().0);

        let result = query.fetch_optional(&mut *conn).await.unwrap();

        match result {
            None => {
                // add user
                let stmt = conn
                    .prepare("INSERT INTO User (chat_id, username, hour) VALUES (?, ?, ?)")
                    .await
                    .unwrap();

                let query = stmt
                    .query()
                    .bind(user.get_chat_id().0)
                    .bind(user.get_username())
                    .bind(user.get_hour());

                query
                    .execute(&mut *conn)
                    .await
                    .expect("Error when inserting new user");
            }
            Some(row) => {
                // modify username
                let stmt = conn
                    .prepare("UPDATE User SET username=? WHERE id=?")
                    .await
                    .unwrap();
                let id: i64 = row.try_get("id").unwrap();
                let query = stmt.query().bind(user.get_username()).bind(id);
                query
                    .execute(&mut *conn)
                    .await
                    .expect("Error when updating user");
            }
        }

        // TODO add user on table users only if user doesn't exist
    }

    pub async fn get_user_by_chat_id(&mut self, chat_id: ChatId) -> Option<User> {
        // TODO get user form Users table with his chat_id
        /*
        let query = "SELECT * FROM users WHERE chat_id == ?";
        let mut rows = sqlx::query(query)
            .bind(chat_id)
            .fetch(&mut self.pool_.deref());

        let mut usr: Option<User> = None;
        while let Some(row) = rows.try_next().await? {
            let cid: ChatId = row.try_get("chat_id")?;
            let name: String = row.try_get("username")?;
            let hour: u8 = row.try_get("hour")?;
            usr = Option::from(User::new(cid, name, Option::from(hour)));
            println!("Find user {} for {} chat id. h = {}", name, cid, hour);
        }
        usr
        */
        Option::from(User::new(chat_id, "toto".to_string(), None))
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
