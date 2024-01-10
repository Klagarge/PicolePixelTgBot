use std::cell::RefCell;
use crate::rank_day::RankDay;
use crate::user::User;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use sqlx::sqlite::{SqliteQueryResult, SqliteRow};
use sqlx::{Connection, Error, Executor, Row, SqliteConnection, SqlitePool, Statement};
use std::fmt::format;
use std::ops::Deref;
use std::ptr::null;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use teloxide::types::{ChatId, MessageId};
use tokio::runtime::Runtime;


pub struct Database {
    path_: String,
}

impl Database {
    pub fn new(path: String) -> Database {
        Database {
            path_: path
        }
    }

    pub async fn create_table(&self) {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS User (\
                        id INTEGER CONSTRAINT user_pk PRIMARY KEY AUTOINCREMENT,\
                        chat_id INTEGER(8) NOT NULL CONSTRAINT user_chat_id UNIQUE,\
                        username TEXT NOT NULL,\
                        hour INTEGER(1) NOT NULL DEFAULT 22)"
        ).await.unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS Rank_day (\
                        id INTEGER CONSTRAINT rank_day_pk PRIMARY KEY AUTOINCREMENT,\
                        user_id INTEGER NOT NULL CONSTRAINT User_id_fk REFERENCES User (id),\
                        time INTEGER(8) NOT NULL,\
                        id_msg INTEGER(4) NOT NULL,\
                        rank INTEGER(1))"
        ).await.unwrap();
        conn.close();
    }

    pub async fn add_user(&self, user: User) {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT id, username FROM User WHERE chat_id = ?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(user.get_chat_id().0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

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
                    .execute(&mut conn)
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
                    .execute(&mut conn)
                    .await
                    .expect("Error when updating user");
            }
        }

        conn.close();
    }

    pub async fn add_rank_day(&self, rank_day: RankDay) {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("INSERT INTO Rank_day (user_id, time, id_msg, rank) VALUES (?, ?, ?, ?)")
            .await
            .unwrap();

        let user = rank_day.get_user();
        let user_id = self
            .get_user_id_by_chat_id(user.get_chat_id())
            .await
            .expect("404 User not found");

        let query = stmt
            .query()
            .bind(user_id)
            .bind(rank_day.get_time().timestamp())
            .bind(rank_day.get_id_msg().0)
            .bind(rank_day.get_rank());

        query
            .execute(&mut conn)
            .await
            .expect("Error when inserting new rank_day");

        conn.close();
    }

    async fn get_user_id_by_chat_id(&self, chat_id:ChatId) -> Option<i64> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT id FROM User WHERE chat_id = ?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(chat_id.0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

        conn.close();

        match result {
            None => { None }
            Some(row) => {
                let id: i64 = row.try_get("id").unwrap();
                Some(id)
            }
        }
    }

    pub async fn get_user_by_chat_id(&mut self, chat_id: ChatId) -> Option<User> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT chat_id, username, hour FROM User WHERE chat_id = ?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(chat_id.0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

        conn.close();

        match result {
            None => { None }
            Some(row) => {
                let chat_id: i64 = row.try_get("chat_id").unwrap();
                let username: String = row.try_get("username").unwrap();
                let hour: u8 = row.try_get("hour").unwrap();
                let user = User::new(ChatId(chat_id), username, Option::from(hour));
                Some(user)
            }
        }
    }

    pub async fn get_time(&self, chat_id: ChatId, id_msg: MessageId) -> Option<DateTime<Utc>> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT time
                            FROM Rank_day
                            join User on User.id = Rank_day.user_id
                            WHERE User.chat_id=? AND Rank_day.id_msg=?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(chat_id.0)
            .bind(id_msg.0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

        conn.close();

        match result {
            None => { None }
            Some(row) => {
                let tst: i64 = row.try_get("time").unwrap();
                let time = DateTime::from_timestamp(tst, 0);
                time
            }
        }
    }
}


pub async fn update_rank_in_rank_day_list(id_chat: ChatId, id_msg: MessageId, rank: u8) {
    /*
    let mut rank_day_list = get_rank_day_list();
    let mut rank_day_list = rank_day_list.lock().unwrap();
    for rank_day in rank_day_list.iter_mut() {
        if rank_day.get_chat_id() == id_chat && rank_day.get_id_msg() == id_msg {
            rank_day.set_rank(rank);
        }
    }
    */
}

pub async fn clear_rank_in_rank_day_list(id_chat: ChatId, id_msg: MessageId) {
    /*
    let mut rank_day_list = get_rank_day_list();
    let mut rank_day_list = rank_day_list.lock().unwrap();
    for rank_day in rank_day_list.iter_mut() {
        if rank_day.get_chat_id() == id_chat && rank_day.get_id_msg() == id_msg {
            rank_day.clear_rank();
        }
    }
    */
}


pub async fn set_hour(chat_id: ChatId, hour: u8) {
    /*
    let mut user_list = get_user_list();
    let mut user_list = user_list.lock().unwrap();
    for user in user_list.iter_mut() {
        if user.get_chat_id() == chat_id {
            user.set_hour(hour);
        }
    }
    */
}